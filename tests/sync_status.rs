mod common;

use aave_v3_tracker::db::repositories::sync_status_repository;
use common::db::TestDb;
use diesel_async::AsyncConnection;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn test_get_last_block_returns_seed_value() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    conn.begin_test_transaction().await.unwrap();

    let block = sync_status_repository::get_last_block(&mut conn)
        .await
        .unwrap();

    assert_eq!(block, 0);
}

#[tokio::test]
async fn test_update_last_block() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    conn.begin_test_transaction().await.unwrap();

    sync_status_repository::update_last_block(&mut conn, 12345)
        .await
        .unwrap();

    let block = sync_status_repository::get_last_block(&mut conn)
        .await
        .unwrap();

    assert_eq!(block, 12345);
}

#[tokio::test]
async fn test_update_last_block_overwrites_previous() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    conn.begin_test_transaction().await.unwrap();

    sync_status_repository::update_last_block(&mut conn, 100)
        .await
        .unwrap();
    sync_status_repository::update_last_block(&mut conn, 200)
        .await
        .unwrap();

    let block = sync_status_repository::get_last_block(&mut conn)
        .await
        .unwrap();

    assert_eq!(block, 200);
}
