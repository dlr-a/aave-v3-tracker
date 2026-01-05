use crate::db::connection::DbPool;
use crate::sync_reserves::reserve_event_handler::process_reserve_event;
use alloy::{providers::Provider, rpc::types::eth::Log, sol};
use eyre::Result;

sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
}

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
