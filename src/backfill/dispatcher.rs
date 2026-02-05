// use crate::abi::Transfer;
use crate::db::connection::DbPool;
use crate::sync_reserves::reserve_event_handler::process_reserve_event;
use alloy::{providers::Provider, rpc::types::eth::Log};
use diesel_async::AsyncPgConnection;
use eyre::Result;

pub async fn handle_log_logic(
    conn: &mut AsyncPgConnection,
    pool: &DbPool,
    provider: impl Provider + Clone + 'static,
    log: &Log,
) -> Result<()> {
    // if let Ok(decoded) = log.log_decode::<Transfer>() {
    //     //TODO
    //     return Ok(());
    // }

    process_reserve_event(conn, pool, provider.clone(), &log).await?;

    Ok(())
}
