mod common;

use aave_v3_tracker::abi::{BalanceTransfer, Burn, IScaledBalanceToken, Mint};
use aave_v3_tracker::backfill::dispatcher::handle_log_logic;
use aave_v3_tracker::db::models::Reserve;
use aave_v3_tracker::db::repositories::reserves_repository;
use aave_v3_tracker::db::repositories::reserves_repository::TokenType;
use aave_v3_tracker::db::repositories::user_positions_repository;
use aave_v3_tracker::db::schema::{reserves as reserves_schema, user_positions};
use aave_v3_tracker::provider::MultiProvider;
use aave_v3_tracker::user_tracking::position_event_handler::{
    ScaledDelta, compute_burn_scaled_delta, compute_mint_scaled_delta,
};
use alloy::eips::BlockId;
use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use alloy::rpc::types::eth::Filter;
use alloy_sol_types::SolEvent;
use backoff::{ExponentialBackoff, future::retry};
use bigdecimal::BigDecimal;
use common::db::TestDb;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use dotenvy::dotenv;
use std::collections::{HashMap, HashSet};
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

fn u256_to_bd(val: U256) -> BigDecimal {
    BigDecimal::from_str(&val.to_string()).unwrap()
}

fn abs_diff(a: &BigDecimal, b: &BigDecimal) -> BigDecimal {
    if a > b { a - b } else { b - a }
}

fn get_chunk_size() -> u64 {
    std::env::var("HTTP_RPC_CHUNK_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
}

fn rpc_backoff() -> ExponentialBackoff {
    ExponentialBackoff {
        max_elapsed_time: Some(std::time::Duration::from_secs(60)),
        initial_interval: std::time::Duration::from_secs(2),
        max_interval: std::time::Duration::from_secs(15),
        multiplier: 2.0,
        ..Default::default()
    }
}

fn is_rate_limited(msg: &str) -> bool {
    let s = msg.to_lowercase();
    s.contains("429") || s.contains("1015") || s.contains("rate limit") || s.contains("too many")
}

fn is_capability_error(msg: &str) -> bool {
    let s = msg.to_lowercase();
    s.contains("block range")
        || s.contains("whitelisted")
        || s.contains("free tier")
        || s.contains("not supported")
        || s.contains("403")
}

async fn rpc_get_block_number(provider: &MultiProvider) -> eyre::Result<u64> {
    let p = provider.clone();
    retry(rpc_backoff(), move || {
        let p = p.clone();
        async move {
            p.get_block_number().await.map_err(|e| {
                let msg = format!("{e}");
                if is_rate_limited(&msg) {
                    backoff::Error::transient(eyre::eyre!("{msg}"))
                } else if is_capability_error(&msg) {
                    p.rotate();
                    backoff::Error::transient(eyre::eyre!("{msg}"))
                } else {
                    backoff::Error::permanent(eyre::eyre!("{msg}"))
                }
            })
        }
    })
    .await
}

async fn rpc_get_logs(
    provider: &MultiProvider,
    filter: Filter,
) -> eyre::Result<Vec<alloy::rpc::types::eth::Log>> {
    let p = provider.clone();
    retry(rpc_backoff(), move || {
        let p = p.clone();
        let f = filter.clone();
        async move {
            p.get_logs(&f).await.map_err(|e| {
                let msg = format!("{e}");
                if is_rate_limited(&msg) {
                    backoff::Error::transient(eyre::eyre!("{msg}"))
                } else if is_capability_error(&msg) {
                    p.rotate();
                    backoff::Error::transient(eyre::eyre!("{msg}"))
                } else {
                    backoff::Error::permanent(eyre::eyre!("{msg}"))
                }
            })
        }
    })
    .await
}

async fn rpc_scaled_balances(
    provider: &MultiProvider,
    atoken_addr: Address,
    vdebt_addr: Address,
    user: Address,
    block: BlockId,
) -> eyre::Result<(U256, U256)> {
    let p = provider.clone();
    retry(rpc_backoff(), move || {
        let p = p.clone();
        async move {
            let at = IScaledBalanceToken::new(atoken_addr, &p);
            let vd = IScaledBalanceToken::new(vdebt_addr, &p);
            p.multicall()
                .add(at.scaledBalanceOf(user))
                .add(vd.scaledBalanceOf(user))
                .block(block)
                .aggregate()
                .await
                .map_err(|e| {
                    let msg = format!("{e}");
                    if is_rate_limited(&msg) {
                        backoff::Error::transient(eyre::eyre!("{msg}"))
                    } else if is_capability_error(&msg) {
                        p.rotate();
                        backoff::Error::transient(eyre::eyre!("{msg}"))
                    } else {
                        backoff::Error::permanent(eyre::eyre!("{msg}"))
                    }
                })
        }
    })
    .await
}

async fn fetch_sorted_logs(
    provider: &MultiProvider,
    token_addresses: &[Address],
    event_sigs: &[alloy::primitives::B256],
    start: u64,
    end: u64,
) -> eyre::Result<Vec<alloy::rpc::types::eth::Log>> {
    let chunk_size = get_chunk_size();
    let address_chunk_size = 5;
    let mut logs = Vec::new();

    for addr_chunk in token_addresses.chunks(address_chunk_size) {
        let mut cs = start;
        while cs <= end {
            let ce = (cs + chunk_size - 1).min(end);
            let filter = Filter::new()
                .from_block(cs)
                .to_block(ce)
                .address(addr_chunk.to_vec())
                .event_signature(event_sigs.to_vec());
            logs.extend(rpc_get_logs(provider, filter).await?);
            cs = ce + 1;
        }
    }

    logs.sort_by(|a, b| {
        let ba = a.block_number.unwrap_or(0);
        let bb = b.block_number.unwrap_or(0);
        ba.cmp(&bb)
            .then_with(|| a.log_index.unwrap_or(0).cmp(&b.log_index.unwrap_or(0)))
    });
    Ok(logs)
}

fn build_asset_to_tokens(
    token_map: &HashMap<Address, (String, TokenType)>,
) -> HashMap<String, (Address, Address)> {
    let mut m: HashMap<String, (Address, Address)> = HashMap::new();
    for (token_addr, (asset, token_type)) in token_map {
        let entry = m
            .entry(asset.clone())
            .or_insert((Address::ZERO, Address::ZERO));
        match token_type {
            TokenType::AToken => entry.0 = *token_addr,
            TokenType::VariableDebtToken => entry.1 = *token_addr,
        }
    }
    m
}

#[derive(Queryable, Debug)]
struct PosWithTokens {
    user_address: String,
    asset_address: String,
    scaled_atoken_balance: BigDecimal,
    scaled_variable_debt: BigDecimal,
    last_updated_block: i64,
    atoken_address: String,
    v_debt_token_address: String,
}

async fn fetch_db_positions(
    conn: &mut diesel_async::AsyncPgConnection,
    min_block: i64,
    limit: i64,
) -> Vec<PosWithTokens> {
    use aave_v3_tracker::db::schema::reserves;
    use aave_v3_tracker::db::schema::user_positions;
    let dust = BigDecimal::from(DUST_WEI);

    user_positions::table
        .inner_join(reserves::table.on(reserves::asset_address.eq(user_positions::asset_address)))
        .filter(
            user_positions::scaled_atoken_balance
                .gt(dust.clone())
                .or(user_positions::scaled_variable_debt.gt(dust)),
        )
        .filter(user_positions::last_updated_block.gt(min_block))
        .filter(user_positions::last_updated_block.gt(user_positions::created_at_block))
        .order(user_positions::last_updated_block.asc())
        .select((
            user_positions::user_address,
            user_positions::asset_address,
            user_positions::scaled_atoken_balance,
            user_positions::scaled_variable_debt,
            user_positions::last_updated_block,
            reserves::atoken_address,
            reserves::v_debt_token_address,
        ))
        .limit(limit)
        .load::<PosWithTokens>(conn)
        .await
        .expect("fetch_db_positions failed")
}

async fn test_diagnose_known_failures_origin(
    user_str: Address,
    asset_str: Address,
    diff: BigDecimal,
    is_debt: bool,
) -> Result<bool, eyre::Error> {
    let provider = load_provider();
    let pool = load_db_pool().await;
    let mut conn = pool.get().await?;

    use aave_v3_tracker::db::schema::reserves;
    use aave_v3_tracker::db::schema::user_positions;

    let pos: Option<(BigDecimal, BigDecimal, i64, i64)> = user_positions::table
        .filter(user_positions::user_address.eq(user_str.to_string()))
        .filter(user_positions::asset_address.eq(asset_str.to_string()))
        .select((
            user_positions::scaled_atoken_balance,
            user_positions::scaled_variable_debt,
            user_positions::created_at_block,
            user_positions::last_updated_block,
        ))
        .first(&mut conn)
        .await
        .optional()
        .expect("query failed");

    let (db_supply, db_debt, created_block, updated_block) = match pos {
        Some(p) => p,
        None => {
            eprintln!("  position not found in DB");
            return Ok(false);
        }
    };

    eprintln!(
        "  DB: supply={} debt={} created_block={} last_updated_block={}",
        db_supply, db_debt, created_block, updated_block
    );

    let token_info: Option<(String, String)> = reserves::table
        .filter(reserves::asset_address.eq(asset_str.to_string()))
        .select((reserves::atoken_address, reserves::v_debt_token_address))
        .first(&mut conn)
        .await
        .optional()
        .expect("query failed");

    let (atoken_str, vdebt_str) = match token_info {
        Some(t) => t,
        None => {
            eprintln!("  reserve not found");
            return Ok(false);
        }
    };

    let user_addr: Address = user_str;
    let atoken_addr: Address = atoken_str.parse()?;
    let vdebt_addr: Address = vdebt_str.parse()?;

    let created_bid = BlockId::number(created_block as u64);
    let (onchain_at_created_supply_raw, onchain_at_created_debt_raw) =
        rpc_scaled_balances(&provider, atoken_addr, vdebt_addr, user_addr, created_bid).await?;

    let onchain_at_created_supply = u256_to_bd(onchain_at_created_supply_raw);
    let onchain_at_created_debt = u256_to_bd(onchain_at_created_debt_raw);

    eprintln!(
        "  On-chain at created_block({}): supply={} debt={}",
        created_block, onchain_at_created_supply, onchain_at_created_debt
    );

    let updated_bid = BlockId::number(updated_block as u64);
    let (onchain_at_updated_supply_raw, onchain_at_updated_debt_raw) =
        rpc_scaled_balances(&provider, atoken_addr, vdebt_addr, user_addr, updated_bid).await?;

    let onchain_at_updated_supply = u256_to_bd(onchain_at_updated_supply_raw);
    let onchain_at_updated_debt = u256_to_bd(onchain_at_updated_debt_raw);

    eprintln!(
        "  On-chain at last_updated_block({}): supply={} debt={}",
        updated_block, onchain_at_updated_supply, onchain_at_updated_debt
    );

    let supply_diff_at_created = abs_diff(&db_supply, &onchain_at_created_supply);
    let supply_diff_at_updated = abs_diff(&db_supply, &onchain_at_updated_supply);

    let debt_diff_at_created = abs_diff(&db_debt, &onchain_at_created_debt);
    let debt_diff_at_updated = abs_diff(&db_debt, &onchain_at_updated_debt);

    let is_subgraph_issue = if !is_debt {
        if supply_diff_at_created == diff {
            eprintln!(
                "  → SUBGRAPH (supply): mismatch already at created_block (diff={})",
                supply_diff_at_created
            );
            true
        } else if supply_diff_at_updated != diff {
            eprintln!(
                "  → EVENT HANDLER (supply): was correct at created_block, drifted after (diff={})",
                supply_diff_at_updated
            );
            false
        } else {
            eprintln!("  → supply within tolerance");
            true
        }
    } else {
        if debt_diff_at_created == diff {
            eprintln!(
                "  → SUBGRAPH (debt): mismatch already at created_block (diff={})",
                debt_diff_at_created
            );
            true
        } else if debt_diff_at_updated != diff {
            eprintln!(
                "  → EVENT HANDLER (debt): was correct at created_block, drifted after (diff={})",
                debt_diff_at_updated
            );
            false
        } else {
            eprintln!("  → debt within tolerance");
            true
        }
    };
    Ok(is_subgraph_issue)
}

#[tokio::test]
async fn test_event_replay_all_events() -> eyre::Result<()> {
    const WINDOW: u64 = 200;
    const TARGET_USERS: usize = 30;

    let provider = load_provider();
    let pool = load_db_pool().await;
    let mut conn = pool.get().await?;

    let latest = rpc_get_block_number(&provider).await?;
    let end_block = latest.saturating_sub(50);
    let start_block = end_block.saturating_sub(WINDOW);
    let anchor_block = start_block.saturating_sub(1);

    eprintln!("[event_replay] anchor={anchor_block} events=[{start_block}, {end_block}]");

    let token_map = reserves_repository::get_token_address_map(&mut conn).await?;
    let token_addresses: Vec<Address> = token_map.keys().cloned().collect();
    let asset_to_tokens = build_asset_to_tokens(&token_map);
    drop(conn);

    let event_sigs = vec![
        Mint::SIGNATURE_HASH,
        Burn::SIGNATURE_HASH,
        BalanceTransfer::SIGNATURE_HASH,
    ];
    let logs = fetch_sorted_logs(
        &provider,
        &token_addresses,
        &event_sigs,
        start_block,
        end_block,
    )
    .await?;

    eprintln!("[event_replay] {} logs fetched", logs.len());

    let mut pairs: HashSet<(Address, String)> = HashSet::new();
    for log in &logs {
        let topic0 = match log.topics().first() {
            Some(t) => *t,
            None => continue,
        };
        let (asset, _) = match token_map.get(&log.address()) {
            Some(v) => v,
            None => continue,
        };
        if topic0 == Mint::SIGNATURE_HASH {
            if let Ok(d) = log.log_decode::<Mint>() {
                if d.data().onBehalfOf != Address::ZERO {
                    pairs.insert((d.data().onBehalfOf, asset.clone()));
                }
            }
        } else if topic0 == Burn::SIGNATURE_HASH {
            if let Ok(d) = log.log_decode::<Burn>() {
                if d.data().from != Address::ZERO {
                    pairs.insert((d.data().from, asset.clone()));
                }
            }
        } else if topic0 == BalanceTransfer::SIGNATURE_HASH {
            if let Ok(d) = log.log_decode::<BalanceTransfer>() {
                let e = d.data();
                if e.from != Address::ZERO {
                    pairs.insert((e.from, asset.clone()));
                }
                if e.to != Address::ZERO {
                    pairs.insert((e.to, asset.clone()));
                }
            }
        }
    }

    assert!(
        pairs.len() >= TARGET_USERS,
        "not enough user-asset pairs: {} < {}",
        pairs.len(),
        TARGET_USERS
    );

    let mut sorted: Vec<_> = pairs.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let step = (sorted.len() / TARGET_USERS).max(1);
    let selected: Vec<_> = sorted
        .iter()
        .step_by(step)
        .take(TARGET_USERS)
        .cloned()
        .collect();

    eprintln!("[event_replay] {} pairs selected", selected.len());

    let anchor_bid = BlockId::number(anchor_block);
    let end_bid = BlockId::number(end_block);
    let tolerance = BigDecimal::from(5i64);

    let mut passed_count = 0usize;
    let mut failed_count = 0usize;

    for (i, (user_addr, asset_str)) in selected.iter().enumerate() {
        let label = i + 1;
        let (atoken_addr, vdebt_addr) = match asset_to_tokens.get(asset_str) {
            Some(t) => *t,
            None => continue,
        };

        let (anchor_supply_raw, anchor_debt_raw) =
            rpc_scaled_balances(&provider, atoken_addr, vdebt_addr, *user_addr, anchor_bid).await?;

        let mut supply = u256_to_bd(U256::from(anchor_supply_raw));
        let mut debt = u256_to_bd(U256::from(anchor_debt_raw));


        for log in &logs {
            let topic0 = match log.topics().first() {
                Some(t) => *t,
                None => continue,
            };
            let emitter = log.address();

            if emitter == atoken_addr {
                if topic0 == Mint::SIGNATURE_HASH {
                    if let Ok(d) = log.log_decode::<Mint>() {
                        let e = d.data();
                        if e.onBehalfOf != *user_addr {
                            continue;
                        }
                        match compute_mint_scaled_delta(e.value, e.balanceIncrease, e.index) {
                            ScaledDelta::Increase(d) => supply = supply + u256_to_bd(d),
                            ScaledDelta::Decrease(d) => supply = supply - u256_to_bd(d),
                        }
                    }
                } else if topic0 == Burn::SIGNATURE_HASH {
                    if let Ok(d) = log.log_decode::<Burn>() {
                        let e = d.data();
                        if e.from != *user_addr {
                            continue;
                        }
                        let delta = compute_burn_scaled_delta(e.value, e.balanceIncrease, e.index);
                        supply = supply - u256_to_bd(delta);
                    }
                } else if topic0 == BalanceTransfer::SIGNATURE_HASH {
                    if let Ok(d) = log.log_decode::<BalanceTransfer>() {
                        let e = d.data();
                        let transfer_amount = u256_to_bd(e.value);
                        if e.from == *user_addr {
                            supply = supply - transfer_amount.clone();
                        }
                        if e.to == *user_addr {
                            supply = supply + transfer_amount;
                        }
                    }
                }
            } else if emitter == vdebt_addr {
                if topic0 == Mint::SIGNATURE_HASH {
                    if let Ok(d) = log.log_decode::<Mint>() {
                        let e = d.data();
                        if e.onBehalfOf != *user_addr {
                            continue;
                        }
                        match compute_mint_scaled_delta(e.value, e.balanceIncrease, e.index) {
                            ScaledDelta::Increase(d) => debt = debt + u256_to_bd(d),
                            ScaledDelta::Decrease(d) => debt = debt - u256_to_bd(d),
                        }
                    }
                } else if topic0 == Burn::SIGNATURE_HASH {
                    if let Ok(d) = log.log_decode::<Burn>() {
                        let e = d.data();
                        if e.from != *user_addr {
                            continue;
                        }
                        let delta = compute_burn_scaled_delta(e.value, e.balanceIncrease, e.index);
                        debt = debt - u256_to_bd(delta);
                    }
                }
            }
        }

        let (onchain_supply_raw, onchain_debt_raw) =
            rpc_scaled_balances(&provider, atoken_addr, vdebt_addr, *user_addr, end_bid).await?;

        let onchain_supply = u256_to_bd(onchain_supply_raw);
        let onchain_debt = u256_to_bd(onchain_debt_raw);

        let supply_diff = abs_diff(&supply, &onchain_supply);
        let debt_diff = abs_diff(&debt, &onchain_debt);
        let within_tolerance = supply_diff <= tolerance && debt_diff <= tolerance;

        if within_tolerance {
            passed_count += 1;
        } else {
            failed_count += 1;
            eprintln!(
                "  [{}] FAIL  user={}  asset={}",
                label, user_addr, asset_str
            );
            eprintln!(
                "    anchor_balance={} end_balance={}",
                u256_to_bd(anchor_supply_raw),
                u256_to_bd(onchain_supply_raw)
            );
            eprintln!("    replay_result={} (anchor + events)", supply);
            if supply_diff > tolerance {
                eprintln!(
                    "    supply: computed={} expected={} diff={}",
                    supply, onchain_supply, supply_diff
                );
            }
            if debt_diff > tolerance {
                eprintln!(
                    "    debt:   computed={} expected={} diff={}",
                    debt, onchain_debt, debt_diff
                );
            }
        }
    }

    eprintln!(
        "\n[event_replay] {}/{} passed (tol=±5 wei, window={})",
        passed_count,
        passed_count + failed_count,
        WINDOW
    );

    assert_eq!(failed_count, 0, "{} pairs failed event replay", failed_count);
    Ok(())
}

#[tokio::test]
async fn test_db_scaled_balance_snapshot() -> Result<(), eyre::Error> {
    let provider = load_provider();
    let pool = load_db_pool().await;
    let mut conn = pool.get().await.expect("db conn failed");

    let latest = rpc_get_block_number(&provider).await? as i64;
    let min_block: i64 = std::env::var("VERIFY_MIN_BLOCK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| latest.saturating_sub(500));
    let batch_size: i64 = 25;

    let all_positions = fetch_db_positions(&mut conn, min_block, 500_000).await;
    if all_positions.is_empty() {
        eprintln!("[db_snapshot] no positions found, skipping");
        return Ok(());
    }

    let step = (all_positions.len() as i64 / batch_size).max(1) as usize;
    let selected: Vec<&PosWithTokens> = all_positions
        .iter()
        .step_by(step)
        .take(batch_size as usize)
        .collect();

    eprintln!(
        "[db_snapshot] testing {} of {} positions",
        selected.len(),
        all_positions.len()
    );

    let atol = BigDecimal::from(DUST_WEI);
    let mut pass = 0usize;

    for pos in selected.iter() {
        let user_addr: Address = pos.user_address.parse().expect("invalid user");
        let atoken_addr: Address = pos.atoken_address.parse().expect("invalid atoken");
        let vdebt_addr: Address = pos.v_debt_token_address.parse().expect("invalid vdebt");
        let bid = BlockId::number(pos.last_updated_block as u64);

        let (onchain_supply_raw, onchain_debt_raw) =
            rpc_scaled_balances(&provider, atoken_addr, vdebt_addr, user_addr, bid).await?;

        let onchain_supply = u256_to_bd(onchain_supply_raw);
        let onchain_debt = u256_to_bd(onchain_debt_raw);

        let supply_diff = abs_diff(&pos.scaled_atoken_balance, &onchain_supply);
        let debt_diff = abs_diff(&pos.scaled_variable_debt, &onchain_debt);

        let supply_ok = supply_diff <= atol;
        let debt_ok = debt_diff <= atol;

        if supply_ok && debt_ok {
            pass += 1;
        }

        let asset: Address = pos.asset_address.parse().expect("invalid asset");
        if !supply_ok {
            let result =
                test_diagnose_known_failures_origin(user_addr, asset, supply_diff, false).await?;
            assert!(result);
        }

        if !debt_ok {
            let result =
                test_diagnose_known_failures_origin(user_addr, asset, debt_diff, true).await?;
            assert!(result);
        }
    }

    println!("PASSED: {}", pass);
    Ok(())
}

async fn seed_reserves_from_prod(conn: &mut diesel_async::AsyncPgConnection) -> eyre::Result<()> {
    dotenv().ok();
    let prod_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let all_reserves: Vec<Reserve> = tokio::task::spawn_blocking(move || {
        use diesel::Connection;
        use diesel::pg::PgConnection;
        let mut c = PgConnection::establish(&prod_url).expect("failed to connect to prod DB");
        diesel::RunQueryDsl::load(reserves_schema::table, &mut c).expect("failed to load reserves")
    })
    .await?;

    diesel::insert_into(reserves_schema::table)
        .values(&all_reserves)
        .on_conflict(reserves_schema::asset_address)
        .do_nothing()
        .execute(conn)
        .await?;

    eprintln!("[e2e] {} reserve seeded", all_reserves.len());
    Ok(())
}

async fn get_db_position(
    conn: &mut diesel_async::AsyncPgConnection,
    user: &str,
    asset: &str,
) -> Option<(BigDecimal, BigDecimal)> {
    user_positions::table
        .filter(user_positions::user_address.eq(user))
        .filter(user_positions::asset_address.eq(asset))
        .select((
            user_positions::scaled_atoken_balance,
            user_positions::scaled_variable_debt,
        ))
        .first::<(BigDecimal, BigDecimal)>(conn)
        .await
        .optional()
        .ok()
        .flatten()
}

#[tokio::test]
async fn test_dispatcher_e2e_accuracy() -> eyre::Result<()> {
    const WINDOW: u64 = 200;
    const TARGET_USERS: usize = 30;

    let db = TestDb::new().await;
    let provider = load_provider();

    {
        let mut conn = db.conn().await;
        seed_reserves_from_prod(&mut conn).await?;
    }

    let token_map = {
        let mut conn = db.conn().await;
        reserves_repository::get_token_address_map(&mut conn).await?
    };
    let asset_to_tokens = build_asset_to_tokens(&token_map);
    let token_addresses: Vec<Address> = token_map.keys().cloned().collect();

    let end_block = rpc_get_block_number(&provider).await? - 50;
    let start_block = end_block.saturating_sub(WINDOW);
    let anchor_block = start_block.saturating_sub(1);

    eprintln!("[e2e] anchor={anchor_block} events=[{start_block}, {end_block}]");

    let event_sigs = vec![
        Mint::SIGNATURE_HASH,
        Burn::SIGNATURE_HASH,
        BalanceTransfer::SIGNATURE_HASH,
    ];
    let logs = fetch_sorted_logs(
        &provider,
        &token_addresses,
        &event_sigs,
        start_block,
        end_block,
    )
    .await?;
    eprintln!("[e2e] {} log", logs.len());

    let mut pairs: HashSet<(Address, String)> = HashSet::new();
    for log in &logs {
        let topic0 = match log.topics().first() {
            Some(t) => *t,
            None => continue,
        };
        let (asset, _) = match token_map.get(&log.address()) {
            Some(v) => v,
            None => continue,
        };
        if topic0 == Mint::SIGNATURE_HASH {
            if let Ok(d) = log.log_decode::<Mint>() {
                if d.data().onBehalfOf != Address::ZERO {
                    pairs.insert((d.data().onBehalfOf, asset.clone()));
                }
            }
        } else if topic0 == Burn::SIGNATURE_HASH {
            if let Ok(d) = log.log_decode::<Burn>() {
                if d.data().from != Address::ZERO {
                    pairs.insert((d.data().from, asset.clone()));
                }
            }
        } else if topic0 == BalanceTransfer::SIGNATURE_HASH {
            if let Ok(d) = log.log_decode::<BalanceTransfer>() {
                let e = d.data();
                if e.from != Address::ZERO {
                    pairs.insert((e.from, asset.clone()));
                }
                if e.to != Address::ZERO {
                    pairs.insert((e.to, asset.clone()));
                }
            }
        }
    }

    let mut sorted: Vec<_> = pairs.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let step = (sorted.len() / TARGET_USERS).max(1);
    let selected: Vec<_> = sorted
        .iter()
        .step_by(step)
        .take(TARGET_USERS)
        .cloned()
        .collect();

    eprintln!(
        "[e2e] {} pairs selected, seeding anchor balances...",
        selected.len()
    );

    let anchor_bid = BlockId::number(anchor_block);
    let dummy_index = BigDecimal::from(1);

    for (user_addr, asset_str) in &selected {
        let (atoken_addr, vdebt_addr) = match asset_to_tokens.get(asset_str) {
            Some(t) => *t,
            None => continue,
        };

        let (supply_raw, debt_raw) =
            rpc_scaled_balances(&provider, atoken_addr, vdebt_addr, *user_addr, anchor_bid).await?;

        let supply = u256_to_bd(U256::from(supply_raw));
        let debt = u256_to_bd(U256::from(debt_raw));
        let user_str = user_addr.to_string();

        let mut conn = db.conn().await;
        if supply > BigDecimal::from(0) {
            user_positions_repository::upsert_supply(
                &mut conn,
                &user_str,
                asset_str,
                supply,
                dummy_index.clone(),
                anchor_block as i64,
                0,
            )
            .await?;
        }
        if debt > BigDecimal::from(0) {
            // log_idx=1 ensures the UPDATE filter passes when both supply and debt exist
            user_positions_repository::upsert_debt(
                &mut conn,
                &user_str,
                asset_str,
                debt,
                dummy_index.clone(),
                anchor_block as i64,
                1,
            )
            .await?;
        }
    }

    eprintln!("[e2e] anchor seed done, running dispatcher...");

    for log in &logs {
        let mut conn = db.conn().await;
        let mut user = String::from(" ");
        let topic0 = match log.topics().first() {
            Some(t) => *t,
            None => continue,
        };
        if token_map.get(&log.address()).is_none() {
            continue;
        }
        if topic0 == Mint::SIGNATURE_HASH {
            if let Ok(d) = log.log_decode::<Mint>() {
                user = d.data().onBehalfOf.to_string();
            }
        } else if topic0 == Burn::SIGNATURE_HASH {
            if let Ok(d) = log.log_decode::<Burn>() {
                user = d.data().from.to_string();
            }
        } else if topic0 == BalanceTransfer::SIGNATURE_HASH {
            if let Ok(d) = log.log_decode::<BalanceTransfer>() {
                user = d.data().from.to_string();
            }
        }

        if let Err(e) = handle_log_logic(&mut conn, &db.pool(), provider.clone(), log).await {
            eprintln!("[e2e] handle_log_logic error: {e}");
        }

        let (_db_supply, _db_debt) = get_db_position(&mut conn, &user, &log.address().to_string())
            .await
            .unwrap_or((BigDecimal::from(0), BigDecimal::from(0)));
    }

    eprintln!("[e2e] dispatcher done, verifying...");

    let end_bid = BlockId::number(end_block);
    let tol = BigDecimal::from(5i64);
    let mut pass = 0usize;
    let mut fail = 0usize;

    for (user_addr, asset_str) in &selected {
        let (atoken_addr, vdebt_addr) = match asset_to_tokens.get(asset_str) {
            Some(t) => *t,
            None => continue,
        };

        let (exp_supply_raw, exp_debt_raw) =
            rpc_scaled_balances(&provider, atoken_addr, vdebt_addr, *user_addr, end_bid).await?;

        let exp_supply = u256_to_bd(exp_supply_raw);
        let exp_debt = u256_to_bd(exp_debt_raw);

        let mut conn = db.conn().await;
        let (db_supply, db_debt) = get_db_position(&mut conn, &user_addr.to_string(), asset_str)
            .await
            .unwrap_or((BigDecimal::from(0), BigDecimal::from(0)));

        let sd = abs_diff(&db_supply, &exp_supply);
        let dd = abs_diff(&db_debt, &exp_debt);
        let ok = sd <= tol && dd <= tol;

        if ok {
            pass += 1;
        } else {
            fail += 1;
            eprintln!("  FAIL user={user_addr} asset={asset_str}");
            if sd > tol {
                eprintln!("    supply: db={db_supply} expected={exp_supply} diff={sd}");
            }
            if dd > tol {
                eprintln!("    debt:   db={db_debt} expected={exp_debt} diff={dd}");
            }
        }
    }

    eprintln!(
        "\n[e2e] {pass}/{} passed (tol=±5 wei, window={WINDOW})",
        pass + fail
    );
    assert_eq!(fail, 0, "{fail} pairs failed e2e test");

    Ok(())
}
