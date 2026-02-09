mod common;

use aave_v3_tracker::db::repositories::reserves_repository;
use common::db::TestDb;
use common::fixtures::{ReserveBuilder, get_reserve, unique_asset};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn test_older_block_does_not_overwrite_newer() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(false)
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 99, 10)
        .await
        .unwrap();

    assert_eq!(updated, 0, "Older block should not update");

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_frozen, false);
    assert_eq!(reserve.last_updated_block, 100);
}

#[tokio::test]
async fn test_same_block_lower_log_index_no_update() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(false)
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 100, 3)
        .await
        .unwrap();

    assert_eq!(updated, 0, "Lower log_index should not update");
    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_frozen, false);
    assert_eq!(reserve.last_updated_log_index, 5);
}

#[tokio::test]
async fn test_same_block_same_log_index_is_idempotent() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(false)
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 100, 5)
        .await
        .unwrap();

    assert_eq!(updated, 0, "Same event should be idempotent");
}

#[tokio::test]
async fn test_newer_block_updates() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(false)
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 101, 1)
        .await
        .unwrap();

    assert_eq!(updated, 1, "Newer block should update");

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_frozen, true);
    assert_eq!(reserve.last_updated_block, 101);
    assert_eq!(reserve.last_updated_log_index, 1);
}

#[tokio::test]
async fn test_same_block_higher_log_index_updates() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(false)
        .at_block(100, 5)
        .insert(&mut conn)
        .await;

    let updated = reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 100, 10)
        .await
        .unwrap();

    assert_eq!(updated, 1, "Higher log_index should update");

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_frozen, true);
    assert_eq!(reserve.last_updated_block, 100);
    assert_eq!(reserve.last_updated_log_index, 10);
}

#[tokio::test]
async fn test_sequential_events_reach_correct_state() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(false)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    // freeze -> unfreeze -> freeze
    reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 101, 0)
        .await
        .unwrap();
    reserves_repository::set_frozen_status(&mut conn, asset.clone(), false, 102, 0)
        .await
        .unwrap();
    reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 103, 0)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_frozen, true);
    assert_eq!(reserve.last_updated_block, 103);
}

#[tokio::test]
async fn test_out_of_order_events_reach_correct_state() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let asset = unique_asset();

    ReserveBuilder::new()
        .asset_address(&asset)
        .frozen(false)
        .at_block(100, 0)
        .insert(&mut conn)
        .await;

    reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 103, 0)
        .await
        .unwrap();
    reserves_repository::set_frozen_status(&mut conn, asset.clone(), true, 101, 0)
        .await
        .unwrap();
    reserves_repository::set_frozen_status(&mut conn, asset.clone(), false, 102, 0)
        .await
        .unwrap();

    let reserve = get_reserve(&mut conn, &asset).await.unwrap();
    assert_eq!(reserve.is_frozen, true);
    assert_eq!(reserve.last_updated_block, 103);
}

#[tokio::test]
async fn test_nonexistent_asset_returns_zero() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;

    let updated = reserves_repository::set_frozen_status(
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
