mod abi;
mod db;
mod errors;
mod indexer;
mod sync_reserves;
use crate::db::repositories::sync_status_repository;
use crate::indexer::backfill_loop::backfill_loop;
use crate::sync_reserves::fetch_reserves::fetch_reserves;
// use crate::sync_reserves::reserve_event_handler::reserve_event_handler;
use alloy_provider::Provider;
use alloy_provider::ProviderBuilder;
use dotenvy::dotenv;
use std::env;
use tracing::{error, info /*warn*/};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenv().ok();

    // let ws_rpc_url = env::var("WS_RPC_URL").expect("Couldn't find WS_RPC_URL");
    let http_rpc_url = env::var("HTTP_RPC_URL").expect("Couldn't find HTTP_RPC_URL");
    let http_provider = ProviderBuilder::new().connect_http(http_rpc_url.parse()?);

    let pool = db::connection::init_pool().await;
    let mut conn = pool.get().await?;

    let current_head = http_provider.get_block_number().await? as i64;
    let last_block = sync_status_repository::get_last_block(&mut conn).await?;

    if last_block == 0 {
        info!("No snapshot found. Running initial fetch_reserves…");
        info!("Starting reserves synchronization...");

        fetch_reserves(&pool, http_rpc_url.clone()).await?;

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
