mod common;

use aave_v3_tracker::abi::{ReserveUsedAsCollateralDisabled, ReserveUsedAsCollateralEnabled};
use aave_v3_tracker::db::repositories::user_positions_repository;
use aave_v3_tracker::db::schema::user_positions;
use aave_v3_tracker::user_tracking::position_event_handler::process_collateral_event;
use alloy::primitives::{Address, B256, Bytes};
use alloy_sol_types::SolEvent;
use bigdecimal::BigDecimal;
use common::db::TestDb;
use common::fixtures::{ReserveBuilder, unique_asset, unique_tx_hash};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::str::FromStr;

const RAY: &str = "1000000000000000000000000000";

fn ray() -> BigDecimal {
    BigDecimal::from_str(RAY).unwrap()
}

fn bd(n: i64) -> BigDecimal {
    BigDecimal::from(n)
}

fn addr_topic(a: Address) -> B256 {
    let mut bytes = [0u8; 32];
    bytes[12..].copy_from_slice(a.as_slice());
    B256::from(bytes)
}

fn make_rpc_log(
    address: Address,
    topics: Vec<B256>,
    data: Bytes,
    block_number: u64,
    log_index: u64,
    tx_hash: B256,
) -> alloy::rpc::types::Log {
    alloy::rpc::types::Log {
        inner: alloy::primitives::Log {
            address,
            data: alloy::primitives::LogData::new_unchecked(topics, data),
        },
        block_hash: None,
        block_number: Some(block_number),
        block_timestamp: None,
        transaction_hash: Some(tx_hash),
        transaction_index: Some(0),
        log_index: Some(log_index),
        removed: false,
    }
}

fn collateral_log(
    reserve: Address,
    user: Address,
    enabled: bool,
    block: u64,
    log_idx: u64,
    tx_hash: B256,
) -> alloy::rpc::types::Log {
    let topic0 = if enabled {
        ReserveUsedAsCollateralEnabled::SIGNATURE_HASH
    } else {
        ReserveUsedAsCollateralDisabled::SIGNATURE_HASH
    };
    make_rpc_log(
        Address::ZERO,
        vec![topic0, addr_topic(reserve), addr_topic(user)],
        Bytes::new(),
        block,
        log_idx,
        tx_hash,
    )
}

async fn get_pos(
    conn: &mut diesel_async::AsyncPgConnection,
    user: &str,
    asset: &str,
) -> Option<(BigDecimal, BigDecimal, bool, bool)> {
    user_positions::table
        .filter(user_positions::user_address.eq(user))
        .filter(user_positions::asset_address.eq(asset))
        .select((
            user_positions::scaled_atoken_balance,
            user_positions::scaled_variable_debt,
            user_positions::use_as_collateral,
            user_positions::is_active,
        ))
        .first::<(BigDecimal, BigDecimal, bool, bool)>(conn)
        .await
        .optional()
        .expect("DB query failed")
}


// ReserveUsedAsCollateralEnabled → true, then Disabled → false
#[tokio::test]
async fn test_collateral_disabled_clears_flag() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let a = unique_asset();
    let u = unique_asset();
    let asset: Address = a.parse().unwrap();
    let user: Address = u.parse().unwrap();
    let tx1 = B256::from_str(&unique_tx_hash()).unwrap();
    let tx2 = B256::from_str(&unique_tx_hash()).unwrap();

    ReserveBuilder::new()
        .asset_address(&a)
        .insert(&mut conn)
        .await;
    user_positions_repository::upsert_supply(&mut conn, &u, &a, bd(1000), ray(), 100, 0).await?;

    let (_, _, collateral, active) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert!(!collateral, "expected use_as_collateral=false initially");
    assert!(active, "expected is_active=true initially");

    // Enable collateral first
    process_collateral_event(&mut conn, &collateral_log(asset, user, true, 200, 0, tx1)).await?;
    let (_, _, collateral, _) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert!(collateral, "expected use_as_collateral=true after Enabled");

    // Now disable it
    process_collateral_event(&mut conn, &collateral_log(asset, user, false, 300, 0, tx2)).await?;
    let (_, _, collateral, active) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert!(!collateral, "expected use_as_collateral=false after Disabled");
    assert!(active, "supply exists, expected is_active=true");

    Ok(())
}


// Enabled → Disabled → Enabled round-trip
#[tokio::test]
async fn test_collateral_toggle_roundtrip() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let a = unique_asset();
    let u = unique_asset();
    let asset: Address = a.parse().unwrap();
    let user: Address = u.parse().unwrap();
    let tx1 = B256::from_str(&unique_tx_hash()).unwrap();
    let tx2 = B256::from_str(&unique_tx_hash()).unwrap();
    let tx3 = B256::from_str(&unique_tx_hash()).unwrap();

    ReserveBuilder::new()
        .asset_address(&a)
        .insert(&mut conn)
        .await;
    user_positions_repository::upsert_supply(&mut conn, &u, &a, bd(1000), ray(), 100, 0).await?;

    // Enable
    process_collateral_event(&mut conn, &collateral_log(asset, user, true, 200, 0, tx1)).await?;
    let (_, _, collateral, _) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert!(collateral, "expected true after Enabled");

    // Disable
    process_collateral_event(&mut conn, &collateral_log(asset, user, false, 300, 0, tx2)).await?;
    let (_, _, collateral, _) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert!(!collateral, "expected false after Disabled");

    // Enable again
    process_collateral_event(&mut conn, &collateral_log(asset, user, true, 400, 0, tx3)).await?;
    let (_, _, collateral, _) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert!(collateral, "expected true after re-Enabled");

    Ok(())
}


// supply=0 but debt>0 → is_active=true, use_as_collateral=false
#[tokio::test]
async fn test_burn_supply_to_zero_debt_remains_active() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let a = unique_asset();
    let u = unique_asset();

    ReserveBuilder::new()
        .asset_address(&a)
        .insert(&mut conn)
        .await;
    user_positions_repository::upsert_supply(&mut conn, &u, &a, bd(1000), ray(), 100, 0).await?;
    user_positions_repository::upsert_debt(&mut conn, &u, &a, bd(500), ray(), 100, 1).await?;

    // mirrors handle_atoken_burn: decrease_supply → clear_collateral_if_zero
    user_positions_repository::decrease_supply(&mut conn, &u, &a, bd(1000), ray(), 200, 0).await?;
    user_positions_repository::clear_collateral_if_zero(&mut conn, &u, &a).await?;

    let (supply, debt, collateral, active) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert!(supply <= bd(0), "expected supply=0");
    assert!(debt > bd(0), "expected debt still > 0");
    assert!(!collateral, "expected use_as_collateral=false (supply=0)");
    assert!(active, "expected is_active=true (debt>0)");

    Ok(())
}


// debt=0, supply>0 → is_active=true
#[tokio::test]
async fn test_repay_all_debt_supply_remains_active() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let a = unique_asset();
    let u = unique_asset();

    ReserveBuilder::new()
        .asset_address(&a)
        .insert(&mut conn)
        .await;
    user_positions_repository::upsert_supply(&mut conn, &u, &a, bd(1000), ray(), 100, 0).await?;
    user_positions_repository::upsert_debt(&mut conn, &u, &a, bd(500), ray(), 100, 1).await?;

    // mirrors handle_debt_burn: decrease_debt → clear_inactive_if_zero
    user_positions_repository::decrease_debt(&mut conn, &u, &a, bd(500), ray(), 200, 0).await?;
    user_positions_repository::clear_inactive_if_zero(&mut conn, &u, &a).await?;

    let (supply, debt, _, active) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert!(supply > bd(0), "expected supply still > 0");
    assert!(debt <= bd(0), "expected debt=0");
    assert!(active, "expected is_active=true (supply>0)");

    Ok(())
}


// supply=0 ve debt=0 → is_active=false
#[tokio::test]
async fn test_both_zero_marks_inactive() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let a = unique_asset();
    let u = unique_asset();

    ReserveBuilder::new()
        .asset_address(&a)
        .insert(&mut conn)
        .await;
    user_positions_repository::upsert_supply(&mut conn, &u, &a, bd(1000), ray(), 100, 0).await?;
    user_positions_repository::upsert_debt(&mut conn, &u, &a, bd(500), ray(), 100, 1).await?;

    user_positions_repository::decrease_supply(&mut conn, &u, &a, bd(1000), ray(), 200, 0).await?;
    user_positions_repository::decrease_debt(&mut conn, &u, &a, bd(500), ray(), 200, 1).await?;
    // clear_collateral_if_zero: supply=0 → collateral=false; both zero → is_active=false
    user_positions_repository::clear_collateral_if_zero(&mut conn, &u, &a).await?;

    let (_, _, collateral, active) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert!(!active, "expected is_active=false (both zero)");
    assert!(!collateral, "expected use_as_collateral=false");

    Ok(())
}


// events from old blocks must not mutate state
#[tokio::test]
async fn test_old_block_event_ignored() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let a = unique_asset();
    let u = unique_asset();

    ReserveBuilder::new()
        .asset_address(&a)
        .insert(&mut conn)
        .await;
    // seed supply=1000 at block 200
    user_positions_repository::upsert_supply(&mut conn, &u, &a, bd(1000), ray(), 200, 5).await?;

    let (before, _, _, _) = get_pos(&mut conn, &u, &a).await.unwrap();

    // try to decrease at block 50 (200 > 50 → UPDATE filter skips it)
    user_positions_repository::decrease_supply(&mut conn, &u, &a, bd(500), ray(), 50, 0).await?;

    let (after, _, _, _) = get_pos(&mut conn, &u, &a).await.unwrap();
    assert_eq!(before, after, "stale block event must not change balance");

    Ok(())
}
