mod common;

use aave_v3_tracker::db::models::NewEmodeCategory;
use aave_v3_tracker::db::repositories::{emode_categories_repository, user_emode_repository};
use aave_v3_tracker::user_tracking::position_event_handler::process_user_emode_event;
use alloy::primitives::{Address, B256};
use bigdecimal::BigDecimal;
use common::db::TestDb;
use common::fixtures::unique_asset;
use common::log_builder::LogBuilder;

// helpers
fn unique_tx() -> B256 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(1);
    let mut b = [0u8; 32];
    b[24..].copy_from_slice(&CTR.fetch_add(1, Ordering::Relaxed).to_be_bytes());
    B256::from(b)
}

fn new_category(id: i32, ltv: i64, lt: i64, lb: i64) -> NewEmodeCategory {
    NewEmodeCategory {
        category_id: id,
        ltv,
        liquidation_threshold: lt,
        liquidation_bonus: lb,
        collateral_bitmap: BigDecimal::from(0u8),
        borrowable_bitmap: BigDecimal::from(0u8),
        ltvzero_bitmap: BigDecimal::from(0u8),
        label: format!("Category {}", id),
        last_updated_block: 0,
        last_updated_log_index: -1,
    }
}

// eMode category Repository Tests
#[tokio::test]
async fn test_emode_category_upsert_and_get() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;

    // use a high ID unlikely to conflict with other tests
    let id = 201i32;
    emode_categories_repository::upsert(&mut conn, new_category(id, 9700, 9750, 10100)).await?;

    let cat = emode_categories_repository::get(&mut conn, id).await?;
    assert!(cat.is_some(), "category should exist after upsert");
    let cat = cat.unwrap();
    assert_eq!(cat.category_id, id);
    assert_eq!(cat.ltv, 9700);
    assert_eq!(cat.liquidation_threshold, 9750);
    assert_eq!(cat.liquidation_bonus, 10100);

    Ok(())
}

#[tokio::test]
async fn test_emode_category_update_on_conflict() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;

    let id = 202i32;
    emode_categories_repository::upsert(&mut conn, new_category(id, 7000, 7500, 10500)).await?;

    emode_categories_repository::upsert(&mut conn, new_category(id, 7500, 8000, 10800)).await?;

    let cat = emode_categories_repository::get(&mut conn, id)
        .await?
        .unwrap();
    assert_eq!(cat.ltv, 7500, "ltv should reflect second upsert");
    assert_eq!(cat.liquidation_threshold, 8000);
    assert_eq!(cat.liquidation_bonus, 10800);

    Ok(())
}

#[tokio::test]
async fn test_emode_category_get_nonexistent() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;

    let result = emode_categories_repository::get(&mut conn, 255).await?;
    assert!(result.is_none(), "non-existent category should return None");

    Ok(())
}

#[tokio::test]
async fn test_get_all_as_map_returns_all() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;

    let id_a = 211i32;
    let id_b = 212i32;
    emode_categories_repository::upsert(&mut conn, new_category(id_a, 9000, 9200, 10100)).await?;
    emode_categories_repository::upsert(&mut conn, new_category(id_b, 8000, 8500, 10500)).await?;

    let map = emode_categories_repository::get_all_as_map(&mut conn).await?;
    assert!(
        map.contains_key(&id_a),
        "map should contain category {}",
        id_a
    );
    assert!(
        map.contains_key(&id_b),
        "map should contain category {}",
        id_b
    );
    assert_eq!(map[&id_a].ltv, 9000);
    assert_eq!(map[&id_b].ltv, 8000);

    Ok(())
}

// User eMode Repository Tests
#[tokio::test]
async fn test_user_emode_upsert_and_get() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user = unique_asset();

    user_emode_repository::upsert(&mut conn, &user, 1, 100, 0).await?;

    let row = user_emode_repository::get(&mut conn, &user).await?;
    assert!(row.is_some(), "user_emode row should exist");
    let row = row.unwrap();
    assert_eq!(row.emode_category_id, 1);
    assert_eq!(row.last_updated_block, 100);

    Ok(())
}

#[tokio::test]
async fn test_user_emode_newer_block_overwrites() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user = unique_asset();

    user_emode_repository::upsert(&mut conn, &user, 1, 100, 0).await?;
    user_emode_repository::upsert(&mut conn, &user, 2, 200, 0).await?;

    let row = user_emode_repository::get(&mut conn, &user).await?.unwrap();
    assert_eq!(
        row.emode_category_id, 2,
        "newer block should overwrite category"
    );
    assert_eq!(row.last_updated_block, 200);

    Ok(())
}

#[tokio::test]
async fn test_user_emode_stale_block_ignored() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user = unique_asset();

    // user set eMode at block 500
    user_emode_repository::upsert(&mut conn, &user, 2, 500, 0).await?;
    // stale event from block 100 must not overwrite
    user_emode_repository::upsert(&mut conn, &user, 0, 100, 0).await?;

    let row = user_emode_repository::get(&mut conn, &user).await?.unwrap();
    assert_eq!(
        row.emode_category_id, 2,
        "stale block event must not change category"
    );
    assert_eq!(row.last_updated_block, 500);

    Ok(())
}

#[tokio::test]
async fn test_user_emode_same_block_lower_log_ignored() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user = unique_asset();

    // set at block 300, log_index 5
    user_emode_repository::upsert(&mut conn, &user, 1, 300, 5).await?;
    // same block, lower log_index → must be ignored
    user_emode_repository::upsert(&mut conn, &user, 0, 300, 3).await?;

    let row = user_emode_repository::get(&mut conn, &user).await?.unwrap();
    assert_eq!(
        row.emode_category_id, 1,
        "lower log_index at same block must not overwrite"
    );

    Ok(())
}

#[tokio::test]
async fn test_user_emode_same_block_higher_log_overwrites() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user = unique_asset();

    // set at block 300, log_index 5
    user_emode_repository::upsert(&mut conn, &user, 1, 300, 5).await?;
    // same block, higher log_index → must overwrite
    user_emode_repository::upsert(&mut conn, &user, 2, 300, 10).await?;

    let row = user_emode_repository::get(&mut conn, &user).await?.unwrap();
    assert_eq!(
        row.emode_category_id, 2,
        "higher log_index at same block should overwrite"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_all_with_emode_filters_zero() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user_on = unique_asset();
    let user_off = unique_asset();

    user_emode_repository::upsert(&mut conn, &user_on, 1, 100, 0).await?;
    user_emode_repository::upsert(&mut conn, &user_off, 0, 100, 0).await?;

    let active = user_emode_repository::get_all_with_emode(&mut conn).await?;
    let addresses: Vec<String> = active.iter().map(|r| r.user_address.clone()).collect();

    assert!(
        addresses.contains(&user_on),
        "user with eMode=1 should appear"
    );
    assert!(
        !addresses.contains(&user_off),
        "user with eMode=0 should be excluded"
    );

    Ok(())
}

// user eMode Event Handler Tests
#[tokio::test]
async fn test_process_user_emode_event_sets_category() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user_addr: Address = unique_asset().parse().unwrap();

    let log = LogBuilder::new()
        .at_block(500)
        .log_index(3)
        .tx_hash(unique_tx())
        .user_emode_set(user_addr, 1);

    process_user_emode_event(&mut conn, &log).await?;

    let row = user_emode_repository::get(&mut conn, &user_addr.to_string())
        .await?
        .unwrap();
    assert_eq!(row.emode_category_id, 1);
    assert_eq!(row.last_updated_block, 500);
    assert_eq!(row.last_updated_log_index, 3);

    Ok(())
}

#[tokio::test]
async fn test_process_user_emode_event_newer_block_wins() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user_addr: Address = unique_asset().parse().unwrap();

    let log1 = LogBuilder::new()
        .at_block(100)
        .tx_hash(unique_tx())
        .user_emode_set(user_addr, 1);
    let log2 = LogBuilder::new()
        .at_block(200)
        .tx_hash(unique_tx())
        .user_emode_set(user_addr, 2);

    process_user_emode_event(&mut conn, &log1).await?;
    process_user_emode_event(&mut conn, &log2).await?;

    let row = user_emode_repository::get(&mut conn, &user_addr.to_string())
        .await?
        .unwrap();
    assert_eq!(row.emode_category_id, 2, "newer block should win");

    Ok(())
}

#[tokio::test]
async fn test_process_user_emode_event_stale_ignored() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user_addr: Address = unique_asset().parse().unwrap();

    // current state at block 800
    let log_current = LogBuilder::new()
        .at_block(800)
        .tx_hash(unique_tx())
        .user_emode_set(user_addr, 2);
    // replayed stale event from block 400
    let log_stale = LogBuilder::new()
        .at_block(400)
        .tx_hash(unique_tx())
        .user_emode_set(user_addr, 0);

    process_user_emode_event(&mut conn, &log_current).await?;
    process_user_emode_event(&mut conn, &log_stale).await?;

    let row = user_emode_repository::get(&mut conn, &user_addr.to_string())
        .await?
        .unwrap();
    assert_eq!(
        row.emode_category_id, 2,
        "stale event must not overwrite current state"
    );
    assert_eq!(row.last_updated_block, 800);

    Ok(())
}

#[tokio::test]
async fn test_process_user_emode_disable_event() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;
    let user_addr: Address = unique_asset().parse().unwrap();

    let log_enable = LogBuilder::new()
        .at_block(100)
        .tx_hash(unique_tx())
        .user_emode_set(user_addr, 1);
    let log_disable = LogBuilder::new()
        .at_block(200)
        .tx_hash(unique_tx())
        .user_emode_set(user_addr, 0);

    process_user_emode_event(&mut conn, &log_enable).await?;
    process_user_emode_event(&mut conn, &log_disable).await?;

    let row = user_emode_repository::get(&mut conn, &user_addr.to_string())
        .await?
        .unwrap();
    assert_eq!(
        row.emode_category_id, 0,
        "disabling eMode via event should set category to 0"
    );

    Ok(())
}

// eMode Bitmap Repository Tests
// Handler-level tests for EModeCategoryAdded and asset bitmap events require
// a real RPC node (bitmap values are fetched on-chain). These tests cover
// the repository layer directly.
#[tokio::test]
async fn test_update_collateral_bitmap() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;

    let id = 220i32;
    emode_categories_repository::upsert(&mut conn, new_category(id, 9000, 9200, 10100)).await?;

    // bits 1 and 2 set → value 6
    let bitmap = bigdecimal::BigDecimal::from(6u32);
    emode_categories_repository::update_collateral_bitmap(&mut conn, id, bitmap).await?;

    let cat = emode_categories_repository::get(&mut conn, id)
        .await?
        .unwrap();
    assert_eq!(cat.collateral_bitmap.to_string(), "6");

    Ok(())
}

#[tokio::test]
async fn test_update_borrowable_bitmap() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;

    let id = 221i32;
    emode_categories_repository::upsert(&mut conn, new_category(id, 9000, 9200, 10100)).await?;

    let bitmap = bigdecimal::BigDecimal::from(3u32);
    emode_categories_repository::update_borrowable_bitmap(&mut conn, id, bitmap).await?;

    let cat = emode_categories_repository::get(&mut conn, id)
        .await?
        .unwrap();
    assert_eq!(cat.borrowable_bitmap.to_string(), "3");

    Ok(())
}

#[tokio::test]
async fn test_update_ltvzero_bitmap() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;

    let id = 222i32;
    emode_categories_repository::upsert(&mut conn, new_category(id, 9000, 9200, 10100)).await?;

    let bitmap = bigdecimal::BigDecimal::from(12u32);
    emode_categories_repository::update_ltvzero_bitmap(&mut conn, id, bitmap).await?;

    let cat = emode_categories_repository::get(&mut conn, id)
        .await?
        .unwrap();
    assert_eq!(cat.ltvzero_bitmap.to_string(), "12");

    Ok(())
}

#[tokio::test]
async fn test_bitmap_update_overwrites_previous() -> eyre::Result<()> {
    let db = TestDb::new().await;
    let mut conn = db.conn().await;

    let id = 223i32;
    emode_categories_repository::upsert(&mut conn, new_category(id, 9000, 9200, 10100)).await?;

    emode_categories_repository::update_collateral_bitmap(
        &mut conn,
        id,
        bigdecimal::BigDecimal::from(7u32),
    )
    .await?;
    // second update (e.g. asset removed from eMode) overwrites entirely
    emode_categories_repository::update_collateral_bitmap(
        &mut conn,
        id,
        bigdecimal::BigDecimal::from(5u32),
    )
    .await?;

    let cat = emode_categories_repository::get(&mut conn, id)
        .await?
        .unwrap();
    assert_eq!(
        cat.collateral_bitmap.to_string(),
        "5",
        "second update should overwrite bitmap"
    );

    Ok(())
}
