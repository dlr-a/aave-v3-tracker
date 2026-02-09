mod common;

use aave_v3_tracker::db::models::NewReserve;
use aave_v3_tracker::db::repositories::reserves_repository;
use bigdecimal::BigDecimal;
use common::db::TestDb;
use common::fixtures::{get_reserve, unique_asset};
use pretty_assertions::assert_eq;

fn create_new_reserve(asset: &str) -> NewReserve {
    NewReserve {
        asset_address: asset.to_string(),
        symbol: "TEST".to_string(),
        decimals: 18,
        reserve_id: 1,
        ltv: 8000,
        liquidation_threshold: 8500,
        liquidation_bonus: 10500,
        is_active: true,
        is_frozen: false,
        is_paused: false,
        is_borrowing_enabled: true,
        is_dropped: false,
        supply_cap: BigDecimal::from(1_000_000),
        borrow_cap: BigDecimal::from(500_000),
        reserve_factor: 1000,
        is_collateral_enabled: true,
        is_stable_borrow_enabled: false,
        is_flash_loan_enabled: true,
        emode_category_id: 0,
        debt_ceiling: BigDecimal::from(0),
        liquidation_protocol_fee: 1000,
        is_siloed_borrowing: false,
        unbacked_mint_cap: BigDecimal::from(0),
        atoken_address: "0x0000000000000000000000000000000000000001".to_string(),
        v_debt_token_address: "0x0000000000000000000000000000000000000002".to_string(),
        s_debt_token_address: "0x0000000000000000000000000000000000000003".to_string(),
        interest_rate_strategy_address: "0x0000000000000000000000000000000000000004".to_string(),
        last_updated_block: 100,
        last_updated_log_index: 0,
    }
}

#[tokio::test]
async fn test_sync_reserve_inserts_new() {
    let ctx = TestDb::new().await;
    let pool = ctx.pool();
    let asset = unique_asset();

    let new_reserve = create_new_reserve(&asset);

    let result = reserves_repository::sync_reserve(&pool, new_reserve)
        .await
        .unwrap();

    assert_eq!(result, 1);

    let mut conn = ctx.conn().await;
    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.symbol, "TEST");
    assert_eq!(reserve.decimals, 18);
    assert_eq!(reserve.ltv, 8000);
    assert_eq!(reserve.is_active, true);
}

#[tokio::test]
async fn test_sync_reserve_upsert_updates_existing() {
    let ctx = TestDb::new().await;
    let pool = ctx.pool();
    let asset = unique_asset();

    let initial = create_new_reserve(&asset);
    reserves_repository::sync_reserve(&pool, initial)
        .await
        .unwrap();

    let mut updated = create_new_reserve(&asset);
    updated.symbol = "UPDATED".to_string();
    updated.ltv = 7500;
    updated.is_frozen = true;
    updated.supply_cap = BigDecimal::from(2_000_000);

    let result = reserves_repository::sync_reserve(&pool, updated)
        .await
        .unwrap();

    assert_eq!(result, 1);

    let mut conn = ctx.conn().await;
    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.symbol, "UPDATED");
    assert_eq!(reserve.ltv, 7500);
    assert_eq!(reserve.is_frozen, true);
    assert_eq!(reserve.supply_cap, BigDecimal::from(2_000_000));
}

#[tokio::test]
async fn test_sync_reserve_preserves_all_fields() {
    let ctx = TestDb::new().await;
    let pool = ctx.pool();
    let asset = unique_asset();

    let mut new_reserve = create_new_reserve(&asset);
    new_reserve.decimals = 6;
    new_reserve.reserve_id = 42;
    new_reserve.liquidation_threshold = 9000;
    new_reserve.liquidation_bonus = 10800;
    new_reserve.is_stable_borrow_enabled = true;
    new_reserve.emode_category_id = 2;
    new_reserve.debt_ceiling = BigDecimal::from(500_000);
    new_reserve.liquidation_protocol_fee = 500;
    new_reserve.is_siloed_borrowing = true;
    new_reserve.unbacked_mint_cap = BigDecimal::from(100_000);

    reserves_repository::sync_reserve(&pool, new_reserve)
        .await
        .unwrap();

    let mut conn = ctx.conn().await;
    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.decimals, 6);
    assert_eq!(reserve.reserve_id, Some(42));
    assert_eq!(reserve.liquidation_threshold, 9000);
    assert_eq!(reserve.liquidation_bonus, 10800);
    assert_eq!(reserve.is_stable_borrow_enabled, true);
    assert_eq!(reserve.emode_category_id, 2);
    assert_eq!(reserve.debt_ceiling, BigDecimal::from(500_000));
    assert_eq!(reserve.liquidation_protocol_fee, 500);
    assert_eq!(reserve.is_siloed_borrowing, true);
    assert_eq!(reserve.unbacked_mint_cap, BigDecimal::from(100_000));
}

#[tokio::test]
async fn test_sync_reserve_updates_token_addresses() {
    let ctx = TestDb::new().await;
    let pool = ctx.pool();
    let asset = unique_asset();

    let initial = create_new_reserve(&asset);
    reserves_repository::sync_reserve(&pool, initial)
        .await
        .unwrap();

    let mut updated = create_new_reserve(&asset);
    updated.atoken_address = "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
    updated.v_debt_token_address = "0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string();
    updated.s_debt_token_address = "0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC".to_string();
    updated.interest_rate_strategy_address =
        "0xDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD".to_string();

    reserves_repository::sync_reserve(&pool, updated)
        .await
        .unwrap();

    let mut conn = ctx.conn().await;
    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(
        reserve.atoken_address,
        "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
    assert_eq!(
        reserve.v_debt_token_address,
        "0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"
    );
    assert_eq!(
        reserve.s_debt_token_address,
        "0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"
    );
    assert_eq!(
        reserve.interest_rate_strategy_address,
        Some("0xDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD".to_string())
    );
}
