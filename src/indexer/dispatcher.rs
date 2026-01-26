// use crate::abi::Transfer;
use crate::db::connection::DbPool;
use crate::sync_reserves::reserve_event_handler::process_reserve_event;
use alloy::{providers::Provider, rpc::types::eth::Log};
use eyre::Result;

pub async fn handle_log_logic(
    pool: &DbPool,
    provider: impl Provider + Clone + 'static,
    log: &Log,
) -> Result<()> {
    // if let Ok(decoded) = log.log_decode::<Transfer>() {
    //     //TODO
    //     return Ok(());
    // }

    process_reserve_event(pool, provider.clone(), &log).await?;

    Ok(())
}
