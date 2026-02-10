mod common;

use aave_v3_tracker::backfill::dispatcher::handle_log_logic;
use alloy::primitives::{Address, B256};
use bigdecimal::BigDecimal;
use common::db::TestDb;
use common::fixtures::{
    ReserveBuilder, ReserveStateBuilder, get_reserve, get_reserve_state, unique_tx_hash,
};
use common::log_builder::LogBuilder;
use pretty_assertions::assert_eq;
use std::str::FromStr;

fn dummy_provider() -> impl alloy::providers::Provider + Clone + 'static {
    alloy::providers::ProviderBuilder::new().connect_http("http://localhost:1".parse().unwrap())
}

fn addr_from_hex(hex: &str) -> Address {
    Address::from_str(hex).unwrap()
}

// Frozen / Unfrozen
#[tokio::test]
async fn test_pipeline_reserve_frozen_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);
    let asset_str = asset_addr.to_string();

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .frozen(false)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .reserve_frozen(asset_addr);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_frozen, true);
    assert_eq!(reserve.last_updated_block, 100);
}

#[tokio::test]
async fn test_pipeline_reserve_unfrozen_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .frozen(true)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .reserve_unfrozen(asset_addr);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_frozen, false);
    assert_eq!(reserve.last_updated_block, 100);
}

// Paused
#[tokio::test]
async fn test_pipeline_reserve_paused_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .paused(false)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .reserve_paused(asset_addr, true);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_paused, true);
}

// Borrowing
#[tokio::test]
async fn test_pipeline_reserve_borrowing_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .borrowing_enabled(true)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .reserve_borrowing(asset_addr, false);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_borrowing_enabled, false);
}

// Active
#[tokio::test]
async fn test_pipeline_reserve_active_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .active(false)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .reserve_active(asset_addr, true);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_active, true);
}

#[tokio::test]
async fn test_pipeline_reserve_deactivated_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .active(true)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .reserve_active(asset_addr, false);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_active, false);
}

// Dropped
#[tokio::test]
async fn test_pipeline_reserve_dropped_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .reserve_dropped(asset_addr);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_dropped, true);
    assert_eq!(reserve.is_active, false);
}

// Supply Cap

#[tokio::test]
async fn test_pipeline_supply_cap_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .supply_cap(1_000_000)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .supply_cap_changed(asset_addr, 1_000_000, 2_000_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.supply_cap, BigDecimal::from(2_000_000));
}

// Borrow Cap
#[tokio::test]
async fn test_pipeline_borrow_cap_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .borrow_cap(500_000)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .borrow_cap_changed(asset_addr, 500_000, 750_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.borrow_cap, BigDecimal::from(750_000));
}

// Reserve Factor
#[tokio::test]
async fn test_pipeline_reserve_factor_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .reserve_factor(1000)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .reserve_factor_changed(asset_addr, 1000, 1500);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.reserve_factor, 1500);
}

// Collateral Config
#[tokio::test]
async fn test_pipeline_collateral_config_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .ltv(8000)
        .liquidation_threshold(8500)
        .liquidation_bonus(10500)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .collateral_config_changed(asset_addr, 7500, 8000, 11000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.ltv, 7500);
    assert_eq!(reserve.liquidation_threshold, 8000);
    assert_eq!(reserve.liquidation_bonus, 11000);
}

// Interest Rate Strategy
#[tokio::test]
async fn test_pipeline_strategy_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let new_strategy = Address::repeat_byte(0xAA);
    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .interest_rate_strategy_changed(asset_addr, Address::ZERO, new_strategy);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(
        reserve.interest_rate_strategy_address.unwrap(),
        new_strategy.to_string()
    );
}

// Flash Loan
#[tokio::test]
async fn test_pipeline_flash_loan_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .flash_loan_enabled(true)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .flash_loan_changed(asset_addr, false);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_flash_loan_enabled, false);
}

// EMode Category
#[tokio::test]
async fn test_pipeline_emode_category_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .emode_category_id(0)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .emode_category_changed(asset_addr, 0, 1);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.emode_category_id, 1);
}

// Debt Ceiling

#[tokio::test]
async fn test_pipeline_debt_ceiling_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .debt_ceiling(0)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .debt_ceiling_changed(asset_addr, 0, 1_000_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.debt_ceiling, BigDecimal::from(1_000_000));
}

// Liquidation Protocol Fee
#[tokio::test]
async fn test_pipeline_liquidation_protocol_fee_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .liquidation_protocol_fee(1000)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .liquidation_protocol_fee_changed(asset_addr, 1000, 2000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.liquidation_protocol_fee, 2000);
}

// Siloed Borrowing
#[tokio::test]
async fn test_pipeline_siloed_borrowing_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .siloed_borrowing(false)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .siloed_borrowing_changed(asset_addr, false, true);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_siloed_borrowing, true);
}

// Unbacked Mint Cap
#[tokio::test]
async fn test_pipeline_unbacked_mint_cap_changed_updates_db() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .unbacked_mint_cap(0)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .unbacked_mint_cap_changed(asset_addr, 0, 500_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.unbacked_mint_cap, BigDecimal::from(500_000));
}

// ReserveDataUpdated

#[tokio::test]
async fn test_pipeline_reserve_data_updated_updates_state() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    // reserve_state has FK to reserves, so insert reserve first
    ReserveBuilder::new()
        .asset_address(&asset_str)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    ReserveStateBuilder::new()
        .asset_address(&asset_str)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx = unique_tx_hash();
    let log = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx).unwrap())
        .reserve_data_updated(asset_addr, 5000, 3000, 7000, 1_100_000, 1_050_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log)
        .await
        .unwrap();

    let state = get_reserve_state(&mut conn, &asset_str).await.unwrap();
    assert_eq!(state.current_liquidity_rate, BigDecimal::from(5000));
    assert_eq!(state.current_stable_borrow_rate, BigDecimal::from(3000));
    assert_eq!(state.current_variable_borrow_rate, BigDecimal::from(7000));
    assert_eq!(state.liquidity_index, BigDecimal::from(1_100_000));
    assert_eq!(state.variable_borrow_index, BigDecimal::from(1_050_000));
    assert_eq!(state.last_updated_block, 100);
}

// DEDUP + ORDERING (end-to-end)
#[tokio::test]
async fn test_pipeline_duplicate_event_is_idempotent() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .supply_cap(1_000_000)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx1 = unique_tx_hash();
    let log1 = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx1).unwrap())
        .supply_cap_changed(asset_addr, 1_000_000, 2_000_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log1)
        .await
        .unwrap();

    let log2 = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx1).unwrap())
        .supply_cap_changed(asset_addr, 2_000_000, 5_000_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log2)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.supply_cap, BigDecimal::from(2_000_000));
}

#[tokio::test]
async fn test_pipeline_older_block_does_not_overwrite_newer() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .supply_cap(1_000_000)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx1 = unique_tx_hash();
    let log1 = LogBuilder::new()
        .at_block(200)
        .log_index(0)
        .tx_hash(B256::from_str(&tx1).unwrap())
        .supply_cap_changed(asset_addr, 1_000_000, 3_000_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log1)
        .await
        .unwrap();

    let tx2 = unique_tx_hash();
    let log2 = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx2).unwrap())
        .supply_cap_changed(asset_addr, 1_000_000, 2_000_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log2)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.supply_cap, BigDecimal::from(3_000_000));
    assert_eq!(reserve.last_updated_block, 200);
}

#[tokio::test]
async fn test_pipeline_multiple_events_same_block_ordered_by_log_index() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .frozen(false)
        .paused(false)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    // log_index 0: freeze
    let tx1 = unique_tx_hash();
    let log1 = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx1).unwrap())
        .reserve_frozen(asset_addr);

    // log_index 1: pause
    let tx2 = unique_tx_hash();
    let log2 = LogBuilder::new()
        .at_block(100)
        .log_index(1)
        .tx_hash(B256::from_str(&tx2).unwrap())
        .reserve_paused(asset_addr, true);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log1)
        .await
        .unwrap();
    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log2)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.is_frozen, true);
    assert_eq!(reserve.is_paused, true);
    assert_eq!(reserve.last_updated_block, 100);
    assert_eq!(reserve.last_updated_log_index, 1);
}

#[tokio::test]
async fn test_pipeline_unknown_event_is_silently_skipped() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;

    let unknown_log = alloy::rpc::types::Log {
        inner: alloy::primitives::Log {
            address: Address::ZERO,
            data: alloy::primitives::LogData::new(vec![B256::repeat_byte(0xFF)], vec![].into())
                .unwrap(),
        },
        block_hash: None,
        block_number: Some(100),
        block_timestamp: None,
        transaction_hash: Some(B256::repeat_byte(0x99)),
        transaction_index: Some(0),
        log_index: Some(0),
        removed: false,
    };

    let result = handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &unknown_log).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_pipeline_same_block_reverse_log_index_keeps_latest() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .supply_cap(1_000_000)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx1 = unique_tx_hash();
    let log_hi = LogBuilder::new()
        .at_block(100)
        .log_index(5)
        .tx_hash(B256::from_str(&tx1).unwrap())
        .supply_cap_changed(asset_addr, 1_000_000, 3_000_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log_hi)
        .await
        .unwrap();

    let tx2 = unique_tx_hash();
    let log_lo = LogBuilder::new()
        .at_block(100)
        .log_index(2)
        .tx_hash(B256::from_str(&tx2).unwrap())
        .supply_cap_changed(asset_addr, 1_000_000, 2_000_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log_lo)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.supply_cap, BigDecimal::from(3_000_000));
    assert_eq!(reserve.last_updated_block, 100);
    assert_eq!(reserve.last_updated_log_index, 5);
}

#[tokio::test]
async fn test_pipeline_reserve_data_updated_older_block_no_overwrite() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    ReserveStateBuilder::new()
        .asset_address(&asset_str)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let tx1 = unique_tx_hash();
    let log_new = LogBuilder::new()
        .at_block(200)
        .log_index(0)
        .tx_hash(B256::from_str(&tx1).unwrap())
        .reserve_data_updated(asset_addr, 9000, 4000, 12000, 1_200_000, 1_100_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log_new)
        .await
        .unwrap();

    let tx2 = unique_tx_hash();
    let log_old = LogBuilder::new()
        .at_block(100)
        .log_index(0)
        .tx_hash(B256::from_str(&tx2).unwrap())
        .reserve_data_updated(asset_addr, 1000, 500, 2000, 1_010_000, 1_005_000);

    handle_log_logic(&mut conn, &db.pool(), dummy_provider(), &log_old)
        .await
        .unwrap();

    let state = get_reserve_state(&mut conn, &asset_str).await.unwrap();
    assert_eq!(state.current_liquidity_rate, BigDecimal::from(9000));
    assert_eq!(state.liquidity_index, BigDecimal::from(1_200_000));
    assert_eq!(state.last_updated_block, 200);
}

// MULTI-EVENT SCENARIO
#[tokio::test]
async fn test_pipeline_multi_event_scenario() {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let asset_str = common::fixtures::unique_asset();
    let asset_addr = addr_from_hex(&asset_str);

    ReserveBuilder::new()
        .asset_address(&asset_str)
        .supply_cap(1_000_000)
        .borrow_cap(500_000)
        .frozen(false)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    ReserveStateBuilder::new()
        .asset_address(&asset_str)
        .at_block(50, 0)
        .insert(&mut conn)
        .await;

    let events: Vec<alloy::rpc::types::Log> = vec![
        // Block 100: supply cap changes
        LogBuilder::new()
            .at_block(100)
            .log_index(0)
            .tx_hash(B256::from_str(&unique_tx_hash()).unwrap())
            .supply_cap_changed(asset_addr, 1_000_000, 2_000_000),
        // Block 100: borrow cap also changes
        LogBuilder::new()
            .at_block(100)
            .log_index(1)
            .tx_hash(B256::from_str(&unique_tx_hash()).unwrap())
            .borrow_cap_changed(asset_addr, 500_000, 750_000),
        // Block 101: reserve gets frozen
        LogBuilder::new()
            .at_block(101)
            .log_index(0)
            .tx_hash(B256::from_str(&unique_tx_hash()).unwrap())
            .reserve_frozen(asset_addr),
        // Block 102: rate data updated
        LogBuilder::new()
            .at_block(102)
            .log_index(0)
            .tx_hash(B256::from_str(&unique_tx_hash()).unwrap())
            .reserve_data_updated(asset_addr, 9000, 4000, 12000, 1_200_000, 1_100_000),
    ];

    for log in &events {
        handle_log_logic(&mut conn, &db.pool(), dummy_provider(), log)
            .await
            .unwrap();
    }

    let reserve = get_reserve(&mut conn, &asset_str).await.unwrap();
    assert_eq!(reserve.supply_cap, BigDecimal::from(2_000_000));
    assert_eq!(reserve.borrow_cap, BigDecimal::from(750_000));
    assert_eq!(reserve.is_frozen, true);

    let state = get_reserve_state(&mut conn, &asset_str).await.unwrap();
    assert_eq!(state.current_liquidity_rate, BigDecimal::from(9000));
    assert_eq!(state.liquidity_index, BigDecimal::from(1_200_000));
    assert_eq!(state.last_updated_block, 102);
}
