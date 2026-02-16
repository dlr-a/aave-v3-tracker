use aave_v3_tracker::backfill::poller::backfill_loop;
use aave_v3_tracker::db;
use aave_v3_tracker::db::repositories::sync_status_repository;
use aave_v3_tracker::sync_reserves::fetch_reserves::fetch_reserves;
// use crate::sync_reserves::reserve_event_handler::reserve_event_handler;
use aave_v3_tracker::provider::MultiProvider;
use alloy_provider::Provider;
use dotenvy::dotenv;
use std::env;
use tracing::{error, info /*warn*/};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenv().ok();

    let http_rpc_urls: Vec<String> = env::var("HTTP_RPC_URLS")
        .or_else(|_| env::var("HTTP_RPC_URL").map(|u| u.to_string()))
        .expect("Set HTTP_RPC_URLS or HTTP_RPC_URL")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // let ws_rpc_url = env::var("WS_RPC_URL").expect("Couldn't find WS_RPC_URL");
    let first_url = http_rpc_urls[0].clone();
    let http_provider = MultiProvider::new(http_rpc_urls)?;

    let pool = db::connection::init_pool().await;
    let mut conn = pool.get().await?;

    let current_head = http_provider.get_block_number().await? as i64;
    let last_block = sync_status_repository::get_last_block(&mut conn).await?;

    if last_block == 0 {
        info!("No snapshot found. Running initial fetch_reserves…");
        info!("Starting reserves synchronization...");

        fetch_reserves(&pool, first_url).await?;

        sync_status_repository::update_last_block(&mut conn, current_head).await?;
        info!("Reserves synced successfully!");
    }

    // let ws_pool = pool.clone();
    // let ws_url = ws_rpc_url.clone();
    // tokio::spawn(async move {
    //     loop {
    //         info!("Starting WS event handler...");
    //         match reserve_event_handler(&ws_pool, ws_url.clone()).await {
    //             Ok(_) => {
    //                 warn!("WS handler exited normally, reconnecting...");
    //             }
    //             Err(e) => {
    //                 error!("WS handler error: {:?}, reconnecting in 5s...", e);
    //             }
    //         }
    //         tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    //     }
    // });

    tokio::spawn(async move {
        info!("Starting backfill loop...");
        if let Err(e) = backfill_loop(&pool, http_provider.clone()).await {
            error!("Backfill loop error: {:?}", e);
        }
    });

    futures::future::pending::<()>().await;
    Ok(())
}
