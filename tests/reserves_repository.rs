mod common;
use aave_v3_tracker::db::repositories::reserves_repository;
use bigdecimal::BigDecimal;
use common::db::TestDb;
use common::fixtures::{ReserveBuilder, get_reserve, unique_asset};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn test_set_paused_status() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .paused(false)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_paused_status(&mut conn, asset.clone(), true, 101, 0)
        .await
        .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_paused, true);
}

#[tokio::test]
async fn test_set_active_status() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .active(true)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_active_status(&mut conn, asset.clone(), false, 101, 0)
        .await
        .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_active, false);
}

#[tokio::test]
async fn test_set_borrowing_status() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .borrowing_enabled(true)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated =
        reserves_repository::set_borrowing_status(&mut conn, asset.clone(), false, 101, 0)
            .await
            .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_borrowing_enabled, false);
}

#[tokio::test]
async fn test_set_dropped_also_deactivates() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .active(true)
        .dropped(false)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_dropped_status(&mut conn, asset.clone(), 101, 0)
        .await
        .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert!(reserve.is_dropped);
    assert!(!reserve.is_active, "Dropped reserve should be inactive");
}

#[tokio::test]
async fn test_update_supply_cap() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .supply_cap(1_000_000)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::update_supply_cap(
        &mut conn,
        asset.clone(),
        BigDecimal::from(2_000_000),
        101,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.supply_cap, BigDecimal::from(2_000_000));
}

#[tokio::test]
async fn test_update_borrow_cap() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .borrow_cap(500_000)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::update_borrow_cap(
        &mut conn,
        asset.clone(),
        BigDecimal::from(750_000),
        101,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.borrow_cap, BigDecimal::from(750_000));
}

#[tokio::test]
async fn test_update_risk_config() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .ltv(8000)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::update_risk_config(
        &mut conn,
        asset.clone(),
        7500,
        8000,
        10500,
        101,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.ltv, 7500);
    assert_eq!(reserve.liquidation_threshold, 8000);
    assert_eq!(reserve.liquidation_bonus, 10500);
}

#[tokio::test]
async fn test_update_reserve_factor() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated =
        reserves_repository::update_reserve_factor(&mut conn, asset.clone(), 2000, 101, 0)
            .await
            .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.reserve_factor, 2000);
}

#[tokio::test]
async fn test_update_strategy_address() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let new_strategy = "0xNewStrategyAddress1234567890123456789012".to_string();

    let updated = reserves_repository::update_strategy_address(
        &mut conn,
        asset.clone(),
        new_strategy.clone(),
        101,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.interest_rate_strategy_address, Some(new_strategy));
}

#[tokio::test]
async fn test_set_flash_loan_status() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated =
        reserves_repository::set_flash_loan_status(&mut conn, asset.clone(), false, 101, 0)
            .await
            .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_flash_loan_enabled, false);
}

#[tokio::test]
async fn test_update_emode_category() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::update_emode_category(&mut conn, asset.clone(), 1, 101, 0)
        .await
        .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.emode_category_id, 1);
}

#[tokio::test]
async fn test_update_stable_borrow_address() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let new_address = "0x1234567890123456789012345678901234567890".to_string();

    let updated = reserves_repository::update_stable_borrow_address(
        &mut conn,
        asset.clone(),
        new_address.clone(),
        101,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.s_debt_token_address, new_address);
}

#[tokio::test]
async fn test_update_debt_ceiling() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::update_debt_ceiling(
        &mut conn,
        asset.clone(),
        BigDecimal::from(1_000_000),
        101,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.debt_ceiling, BigDecimal::from(1_000_000));
}

#[tokio::test]
async fn test_update_liquidation_protocol_fee() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated =
        reserves_repository::update_liquidation_protocol_fee(&mut conn, asset.clone(), 500, 101, 0)
            .await
            .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.liquidation_protocol_fee, 500);
}

#[tokio::test]
async fn test_set_siloed_borrowing_status() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated =
        reserves_repository::set_siloed_borrowing_status(&mut conn, asset.clone(), true, 101, 0)
            .await
            .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_siloed_borrowing, true);
}

#[tokio::test]
async fn test_update_unbacked_mint_cap() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::update_unbacked_mint_cap(
        &mut conn,
        asset.clone(),
        BigDecimal::from(500_000),
        101,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.unbacked_mint_cap, BigDecimal::from(500_000));
}

#[tokio::test]
async fn test_set_stable_borrow_status() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    reserves_repository::set_stable_borrow_status(&mut conn, asset.clone(), true, 101, 0)
        .await
        .unwrap();

    let new_stable_addr = "0x1234567890AbCdEf1234567890AbCdEf12345678".to_string();

    reserves_repository::update_stable_borrow_address(
        &mut conn,
        asset.clone(),
        new_stable_addr.clone(),
        101,
        1,
    )
    .await
    .unwrap();

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_stable_borrow_enabled, true);
    assert_eq!(reserve.s_debt_token_address, new_stable_addr);
}

#[tokio::test]
async fn test_set_frozen_status_true() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(false)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 101, 0)
        .await
        .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_frozen, true);
}

#[tokio::test]
async fn test_set_frozen_status_false() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(true)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_frozen_status(&mut conn, asset.clone(), false, 101, 0)
        .await
        .unwrap();

    assert_eq!(updated, 1);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_frozen, false);
}

#[tokio::test]
async fn test_set_frozen_status_older_block_no_update() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(false)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 99, 0)
        .await
        .unwrap();

    assert_eq!(updated, 0);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_frozen, false);
}

#[tokio::test]
async fn test_update_nonexistent_asset_returns_zero() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;

    let updated = reserves_repository::set_paused_status(
        &mut conn,
        "0xNONEXISTENT".to_string(),
        true,
        100,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 0);
}

#[tokio::test]
async fn test_multiple_fields_updated_atomically() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .ltv(8000)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    reserves_repository::update_risk_config(&mut conn, asset.clone(), 7000, 7500, 10800, 101, 0)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.ltv, 7000);
    assert_eq!(reserve.liquidation_threshold, 7500);
    assert_eq!(reserve.liquidation_bonus, 10800);
    assert_eq!(reserve.last_updated_block, 101);
}

#[tokio::test]
async fn test_same_block_same_log_index_no_update() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .paused(false)
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_paused_status(&mut conn, asset.clone(), true, 100, 5)
        .await
        .unwrap();

    assert_eq!(updated, 0);

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_paused, false);
}
