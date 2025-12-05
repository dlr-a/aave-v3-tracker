pub mod db;
pub mod sync_reserves;
use crate::sync_reserves::fetch_reserves::fetch_reserves;
use dotenvy::dotenv;
use std::env;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenv().ok();

    let rpc_url = env::var("RPC_URL").expect("Couldn't find RPC_URL");
    let pool = db::connection::init_pool().await;

    info!("Starting reserves synchronization...");
    fetch_reserves(&pool, rpc_url.clone()).await?;
    info!("Reserves synced!");

    Ok(())
}
