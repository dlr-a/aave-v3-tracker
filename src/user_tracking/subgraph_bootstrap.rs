use crate::db::connection::DbPool;
use crate::db::models::NewUserPosition;
use crate::db::repositories::{bootstrap_state_repository, user_positions_repository};
use alloy::primitives::Address;
use bigdecimal::BigDecimal;
use eyre::{Context, Result};
use serde::Deserialize;
use std::str::FromStr;
use tracing::info;

const SUBGRAPH_URL: &str = "https://gateway.thegraph.com/api/{API_KEY}/subgraphs/id/Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL87g";
const PAGE_SIZE: usize = 1000;

#[derive(Debug, Deserialize)]
struct GraphResponse {
    data: Option<GraphData>,
    errors: Option<Vec<GraphError>>,
}

#[derive(Debug, Deserialize)]
struct GraphError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphData {
    #[serde(rename = "userReserves")]
    user_reserves: Vec<SubgraphUserReserve>,
}

#[derive(Debug, Deserialize)]
struct SubgraphUserReserve {
    id: String,
    user: SubgraphUser,
    reserve: SubgraphReserve,
    #[serde(rename = "scaledATokenBalance")]
    scaled_atoken_balance: String,
    #[serde(rename = "scaledVariableDebt")]
    scaled_variable_debt: String,
    #[serde(rename = "usageAsCollateralEnabledOnUser")]
    usage_as_collateral: bool,
}

#[derive(Debug, Deserialize)]
struct SubgraphUser {
    id: String,
}

#[derive(Debug, Deserialize)]
struct SubgraphReserve {
    #[serde(rename = "underlyingAsset")]
    underlying_asset: String,
    #[serde(rename = "liquidityIndex")]
    liquidity_index: String,
    #[serde(rename = "variableBorrowIndex")]
    variable_borrow_index: String,
}

fn to_checksum_address(addr: &str) -> Result<String> {
    let parsed: Address = addr
        .parse()
        .map_err(|e| eyre::eyre!("Invalid address {}: {}", addr, e))?;
    Ok(parsed.to_string())
}

fn parse_bigdecimal(s: &str) -> Result<BigDecimal> {
    BigDecimal::from_str(s).map_err(|e| eyre::eyre!("BigDecimal parse error for '{}': {}", s, e))
}

fn build_query_first_page(at_block: i64) -> String {
    format!(
        r#"{{
  userReserves(
    block: {{ number: {} }}
    first: {}
    where: {{ or: [
      {{ scaledATokenBalance_gt: "0" }}
      {{ scaledVariableDebt_gt: "0" }}
    ]}}
    orderBy: id
    orderDirection: asc
  ) {{
    id
    user {{ id }}
    reserve {{ underlyingAsset liquidityIndex variableBorrowIndex }}
    scaledATokenBalance
    scaledVariableDebt
    usageAsCollateralEnabledOnUser
  }}

}}"#,
        at_block, PAGE_SIZE
    )
}

fn build_query_with_cursor(cursor: &str, at_block: i64) -> String {
    format!(
        r#"{{
  userReserves(
    block: {{ number: {} }}
    first: {}
    where: {{
      and: [
        {{ id_gt: "{}" }}
        {{ or: [
          {{ scaledATokenBalance_gt: "0" }}
          {{ scaledVariableDebt_gt: "0" }}
        ]}}
      ]
    }}
    orderBy: id
    orderDirection: asc
  ) {{
    id
    user {{ id }}
    reserve {{ underlyingAsset liquidityIndex variableBorrowIndex }}
    scaledATokenBalance
    scaledVariableDebt
    usageAsCollateralEnabledOnUser
  }}

}}"#,
        at_block, PAGE_SIZE, cursor
    )
}

fn convert_reserves(
    reserves: &[SubgraphUserReserve],
    meta_block: i64,
) -> Result<Vec<NewUserPosition>> {
    let mut positions = Vec::with_capacity(reserves.len());

    for ur in reserves {
        let user_addr = to_checksum_address(&ur.user.id)?;
        let asset_addr = to_checksum_address(&ur.reserve.underlying_asset)?;
        let scaled_supply = parse_bigdecimal(&ur.scaled_atoken_balance)?;
        let scaled_debt = parse_bigdecimal(&ur.scaled_variable_debt)?;
        let liquidity_index = parse_bigdecimal(&ur.reserve.liquidity_index)?;
        let variable_borrow_index = parse_bigdecimal(&ur.reserve.variable_borrow_index)?;

        let is_active = scaled_supply > BigDecimal::from(0) || scaled_debt > BigDecimal::from(0);

        positions.push(NewUserPosition {
            user_address: user_addr,
            asset_address: asset_addr,
            scaled_atoken_balance: scaled_supply,
            scaled_variable_debt: scaled_debt,
            use_as_collateral: ur.usage_as_collateral,
            atoken_last_index: liquidity_index,
            debt_last_index: variable_borrow_index,
            last_updated_block: meta_block,
            last_updated_log_index: -1,
            is_active,
            created_at_block: meta_block,
        });
    }

    Ok(positions)
}

async fn fetch_page(client: &reqwest::Client, url: &str, query: &str) -> Result<GraphData> {
    let response = client
        .post(url)
        .json(&serde_json::json!({ "query": query }))
        .send()
        .await
        .wrap_err("Failed to query subgraph")?;

    let graph_response: GraphResponse = response
        .json()
        .await
        .wrap_err("Failed to parse subgraph response")?;

    if let Some(errors) = &graph_response.errors {
        let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
        return Err(eyre::eyre!("Subgraph errors: {:?}", msgs));
    }

    graph_response
        .data
        .ok_or_else(|| eyre::eyre!("No data in subgraph response"))
}

pub async fn bootstrap_from_subgraph(pool: &DbPool, api_key: &str, at_block: i64) -> Result<()> {
    let mut conn = pool.get().await?;

    let state = bootstrap_state_repository::get(&mut conn).await?;
    let mut last_cursor = match state {
        Some(s) if s.completed => {
            info!("Subgraph bootstrap already completed, skipping");
            drop(conn);
            return Ok(());
        }
        Some(s) => s.last_cursor.clone(),
        None => String::new(),
    };
    drop(conn);

    if last_cursor.is_empty() {
        info!(at_block, "Starting subgraph bootstrap...");
    } else {
        info!(cursor = %last_cursor, at_block, "Resuming subgraph bootstrap from saved cursor");
    }

    let url = SUBGRAPH_URL.replace("{API_KEY}", api_key);
    let client = reqwest::Client::new();
    let mut total_inserted: usize = 0;
    let mut page_num: usize = 0;

    loop {
        let query = if last_cursor.is_empty() {
            build_query_first_page(at_block)
        } else {
            build_query_with_cursor(&last_cursor, at_block)
        };

        let data = fetch_page(&client, &url, &query).await?;
        let page_count = data.user_reserves.len();

        if page_count == 0 {
            break;
        }

        let positions = convert_reserves(&data.user_reserves, at_block)?;
        let mut conn = pool.get().await?;
        let inserted = user_positions_repository::batch_insert(&mut conn, &positions)
            .await
            .wrap_err("Failed to batch insert positions")?;
        total_inserted += inserted;

        last_cursor = data
            .user_reserves
            .last()
            .ok_or_else(|| eyre::eyre!("user_reserves unexpectedly empty"))?
            .id
            .clone();
        bootstrap_state_repository::upsert(&mut conn, &last_cursor, at_block).await?;
        drop(conn);

        page_num += 1;
        info!(
            page = page_num,
            fetched = page_count,
            inserted,
            total_inserted,
            at_block,
            "Subgraph page committed"
        );

        if page_count < PAGE_SIZE {
            break;
        }
    }

    let mut conn = pool.get().await?;
    bootstrap_state_repository::mark_completed(&mut conn).await?;

    info!(
        total_inserted,
        at_block,
        "Subgraph bootstrap complete. Backfill will continue from block {}",
        at_block + 1
    );

    Ok(())
}
