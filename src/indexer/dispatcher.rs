use crate::db::connection::DbPool;
use alloy::{providers::Provider, rpc::types::eth::Log, sol, sol_types::SolEvent};
use eyre::Result;

pub async fn handle_log_logic(
    pool: &DbPool,
    provider: impl Provider + Clone,
    log: &Log,
) -> Result<()> {
    //TODO
    Ok(())
}
