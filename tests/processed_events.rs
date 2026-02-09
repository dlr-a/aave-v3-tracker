mod common;

use aave_v3_tracker::db::repositories::processed_events_repository;
use common::db::TestDb;
use common::fixtures::unique_tx_hash;

#[tokio::test]
async fn test_first_insert_succeeds() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let tx_hash = unique_tx_hash();

    let inserted = processed_events_repository::try_insert_event(&mut conn, tx_hash, 5, 100)
        .await
        .unwrap();

    assert!(inserted, "First insert should succeed");
}

#[tokio::test]
async fn test_duplicate_event_rejected() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let tx_hash = unique_tx_hash();

    let first = processed_events_repository::try_insert_event(&mut conn, tx_hash.clone(), 5, 100)
        .await
        .unwrap();
    assert!(first);

    let second = processed_events_repository::try_insert_event(&mut conn, tx_hash, 5, 100)
        .await
        .unwrap();

    assert!(!second, "Duplicate should be rejected");
}

#[tokio::test]
async fn test_same_tx_different_log_index_allowed() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;
    let tx_hash = unique_tx_hash();

    let first = processed_events_repository::try_insert_event(&mut conn, tx_hash.clone(), 0, 100)
        .await
        .unwrap();

    let second = processed_events_repository::try_insert_event(&mut conn, tx_hash, 1, 100)
        .await
        .unwrap();

    assert!(first);
    assert!(second, "Different log_index should be allowed");
}

#[tokio::test]
async fn test_different_tx_same_log_index_allowed() {
    let ctx = TestDb::new().await;
    let mut conn = ctx.conn().await;

    let first = processed_events_repository::try_insert_event(&mut conn, unique_tx_hash(), 5, 100)
        .await
        .unwrap();

    let second = processed_events_repository::try_insert_event(&mut conn, unique_tx_hash(), 5, 200)
        .await
        .unwrap();

    assert!(first);
    assert!(second, "Different tx_hash should be allowed");
}
