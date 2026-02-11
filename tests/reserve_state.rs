mod common;

use aave_v3_tracker::db::repositories::reserve_state_repository;
use bigdecimal::BigDecimal;
use common::db::TestDb;
use common::fixtures::{ReserveBuilder, ReserveStateBuilder, get_reserve_state, unique_asset};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn test_update_financials_basic() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    // foreign key constraint
    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    ReserveStateBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    let updated = reserve_state_repository::update_financials(
        &mut conn,
        asset.clone(),
        BigDecimal::from(2),
        BigDecimal::from(3),
        BigDecimal::from(100),
        BigDecimal::from(200),
        BigDecimal::from(50),
        101,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1);

    let state = get_reserve_state(&mut conn, &asset).await.unwrap();
    assert_eq!(state.liquidity_index, BigDecimal::from(2));
    assert_eq!(state.variable_borrow_index, BigDecimal::from(3));
    assert_eq!(state.current_liquidity_rate, BigDecimal::from(100));
    assert_eq!(state.current_variable_borrow_rate, BigDecimal::from(200));
    assert_eq!(state.current_stable_borrow_rate, BigDecimal::from(50));
}

#[tokio::test]
async fn test_update_financials_older_block_no_update() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    ReserveStateBuilder::new()
        .asset_address(&asset)
        .liquidity_index(BigDecimal::from(1))
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserve_state_repository::update_financials(
        &mut conn,
        asset.clone(),
        BigDecimal::from(999),
        BigDecimal::from(999),
        BigDecimal::from(999),
        BigDecimal::from(999),
        BigDecimal::from(999),
        99,
        10,
    )
    .await
    .unwrap();

    assert_eq!(updated, 0, "Older block should not update");

    let state = get_reserve_state(&mut conn, &asset).await.unwrap();
    assert_eq!(state.liquidity_index, BigDecimal::from(1));
}

#[tokio::test]
async fn test_update_financials_same_block_lower_log_index_no_update() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    ReserveStateBuilder::new()
        .asset_address(&asset)
        .liquidity_index(BigDecimal::from(1))
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserve_state_repository::update_financials(
        &mut conn,
        asset.clone(),
        BigDecimal::from(999),
        BigDecimal::from(999),
        BigDecimal::from(999),
        BigDecimal::from(999),
        BigDecimal::from(999),
        100,
        3,
    )
    .await
    .unwrap();

    assert_eq!(updated, 0, "Lower log_index should not update");
    let state = get_reserve_state(&mut conn, &asset).await.unwrap();
    assert_eq!(state.liquidity_index, BigDecimal::from(1));
}

#[tokio::test]
async fn test_update_financials_newer_block_updates() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    ReserveStateBuilder::new()
        .asset_address(&asset)
        .liquidity_index(BigDecimal::from(1))
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserve_state_repository::update_financials(
        &mut conn,
        asset.clone(),
        BigDecimal::from(5),
        BigDecimal::from(5),
        BigDecimal::from(5),
        BigDecimal::from(5),
        BigDecimal::from(5),
        101,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1, "Newer block should update");

    let state = get_reserve_state(&mut conn, &asset).await.unwrap();
    assert_eq!(state.liquidity_index, BigDecimal::from(5));
    assert_eq!(state.last_updated_block, 101);
}

#[tokio::test]
async fn test_update_financials_same_block_higher_log_index_updates() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    ReserveStateBuilder::new()
        .asset_address(&asset)
        .liquidity_index(BigDecimal::from(1))
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserve_state_repository::update_financials(
        &mut conn,
        asset.clone(),
        BigDecimal::from(10),
        BigDecimal::from(10),
        BigDecimal::from(10),
        BigDecimal::from(10),
        BigDecimal::from(10),
        100,
        10,
    )
    .await
    .unwrap();

    assert_eq!(updated, 1, "Higher log_index should update");

    let state = get_reserve_state(&mut conn, &asset).await.unwrap();
    assert_eq!(state.liquidity_index, BigDecimal::from(10));
    assert_eq!(state.last_updated_log_index, 10);
}

#[tokio::test]
async fn test_update_financials_nonexistent_asset_returns_zero() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;

    let updated = reserve_state_repository::update_financials(
        &mut conn,
        "0xNONEXISTENT".to_string(),
        BigDecimal::from(1),
        BigDecimal::from(1),
        BigDecimal::from(1),
        BigDecimal::from(1),
        BigDecimal::from(1),
        100,
        0,
    )
    .await
    .unwrap();

    assert_eq!(updated, 0);
}
