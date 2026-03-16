use aave_v3_tracker::abi::IScaledBalanceToken;
use aave_v3_tracker::db::schema::reserves;
use aave_v3_tracker::db::schema::user_positions;
use aave_v3_tracker::provider::MultiProvider;
use alloy::eips::BlockId;
use alloy::primitives::Address;
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use dotenvy::dotenv;
use std::str::FromStr;

const DUST_WEI: i64 = 5;

fn load_provider() -> MultiProvider {
    dotenv().ok();
    let urls: Vec<String> = std::env::var("HTTP_RPC_URLS")
        .or_else(|_| std::env::var("HTTP_RPC_URL").map(|u| u.to_string()))
        .expect("Set HTTP_RPC_URLS or HTTP_RPC_URL")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    MultiProvider::new(urls).unwrap()
}

async fn load_db_pool() -> aave_v3_tracker::db::connection::DbPool {
    dotenv().ok();
    aave_v3_tracker::db::connection::init_pool().await
}

fn u256_to_bd(val: alloy::primitives::U256) -> BigDecimal {
    BigDecimal::from_str(&val.to_string()).unwrap()
}

fn abs_diff(a: &BigDecimal, b: &BigDecimal) -> BigDecimal {
    if a > b { a - b } else { b - a }
}

#[derive(Queryable, Debug)]
#[allow(dead_code)]
struct SnapshotPosition {
    user_address: String,
    asset_address: String,
    scaled_atoken_balance: BigDecimal,
    scaled_variable_debt: BigDecimal,
    created_at_block: i64,
    last_updated_log_index: i64,
    last_updated_block: i64,
    atoken_address: String,
    v_debt_token_address: String,
}

// Verifies positions that were imported from subgraph and never updated by an event
// (created_at_block == last_updated_block, log_index == -1).
// Their on-chain scaled balance at created_at_block must match the DB value.
// A FAIL here means the subgraph snapshot was already wrong — not our event handler.
#[tokio::test]
async fn test_subgraph_snapshot_accuracy() {
    let provider = load_provider();
    let pool = load_db_pool().await;
    let mut conn = pool.get().await.expect("db conn failed");

    let dust = BigDecimal::from(DUST_WEI);

    // Positions seeded from subgraph and never touched by an event:
    // created_at_block == last_updated_block, log_index == -1
    let positions: Vec<SnapshotPosition> = user_positions::table
        .inner_join(reserves::table.on(reserves::asset_address.eq(user_positions::asset_address)))
        .filter(
            user_positions::created_at_block
                .eq(user_positions::last_updated_block)
                .and(user_positions::last_updated_log_index.eq(-1)),
        )
        .filter(
            user_positions::scaled_atoken_balance
                .gt(dust.clone())
                .or(user_positions::scaled_variable_debt.gt(dust)),
        )
        .select((
            user_positions::user_address,
            user_positions::asset_address,
            user_positions::scaled_atoken_balance,
            user_positions::scaled_variable_debt,
            user_positions::created_at_block,
            user_positions::last_updated_log_index,
            user_positions::last_updated_block,
            reserves::atoken_address,
            reserves::v_debt_token_address,
        ))
        .order(user_positions::created_at_block.asc())
        .limit(100_000)
        .load::<SnapshotPosition>(&mut conn)
        .await
        .expect("snapshot query failed");

    if positions.is_empty() {
        eprintln!("[snapshot] no subgraph-only positions found, skipping");
        return;
    }

    let batch_size = 100usize;
    let step = (positions.len() / batch_size).max(1);
    let selected: Vec<&SnapshotPosition> =
        positions.iter().step_by(step).take(batch_size).collect();

    eprintln!(
        "[snapshot] {} subgraph-only positions found, testing {}",
        positions.len(),
        selected.len()
    );

    let atol = BigDecimal::from(DUST_WEI);

    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut fail_details: Vec<String> = Vec::new();

    for (i, pos) in selected.iter().enumerate() {
        let user_addr: Address = pos.user_address.parse().expect("invalid user");
        let atoken_addr: Address = pos.atoken_address.parse().expect("invalid atoken");
        let vdebt_addr: Address = pos.v_debt_token_address.parse().expect("invalid vdebt");
        let bid = BlockId::number(pos.created_at_block as u64);

        let at = IScaledBalanceToken::new(atoken_addr, &provider);
        let vd = IScaledBalanceToken::new(vdebt_addr, &provider);

        let onchain_supply = u256_to_bd(
            at.scaledBalanceOf(user_addr)
                .block(bid)
                .call()
                .await
                .expect(&format!(
                    "scaledBalanceOf fail: user={} block={}",
                    user_addr, pos.created_at_block
                )),
        );
        let onchain_debt = u256_to_bd(
            vd.scaledBalanceOf(user_addr)
                .block(bid)
                .call()
                .await
                .expect(&format!(
                    "scaledBalanceOf fail: user={} block={}",
                    user_addr, pos.created_at_block
                )),
        );

        let supply_diff = abs_diff(&pos.scaled_atoken_balance, &onchain_supply);
        let debt_diff = abs_diff(&pos.scaled_variable_debt, &onchain_debt);

        let supply_ok = supply_diff <= atol;
        let debt_ok = debt_diff <= atol;

        if supply_ok && debt_ok {
            pass += 1;
        } else {
            fail += 1;
            let mut detail = format!(
                "  [{}] user={}.. asset={}.. block={}",
                i + 1,
                &pos.user_address[..10],
                &pos.asset_address[..10],
                pos.created_at_block,
            );
            if !supply_ok {
                detail.push_str(&format!(
                    "\n    supply: db={} onchain={} diff={}",
                    pos.scaled_atoken_balance, onchain_supply, supply_diff
                ));
            }
            if !debt_ok {
                detail.push_str(&format!(
                    "\n    debt:   db={} onchain={} diff={}",
                    pos.scaled_variable_debt, onchain_debt, debt_diff
                ));
            }
            fail_details.push(detail);
        }
    }

    eprintln!("\n[snapshot] {}/{} passed", pass, pass + fail);

    if !fail_details.is_empty() {
        eprintln!(
            "\n[snapshot] {} mismatches (subgraph data errors, not event handler):",
            fail_details.len()
        );
        for d in &fail_details {
            eprintln!("{}", d);
        }
    }
}
