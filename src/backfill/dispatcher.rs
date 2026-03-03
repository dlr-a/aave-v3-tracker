use crate::abi::{
    BalanceTransfer, Burn, Mint, ReserveUsedAsCollateralDisabled, ReserveUsedAsCollateralEnabled,
};
use crate::db::connection::DbPool;
use crate::sync_reserves::reserve_event_handler::process_reserve_event;
use crate::user_tracking::position_event_handler::{process_collateral_event, process_token_event};
use alloy::providers::Provider;
use alloy::rpc::types::eth::Log;
use alloy_sol_types::SolEvent;
use diesel_async::AsyncPgConnection;
use eyre::Result;

pub async fn handle_log_logic(
    conn: &mut AsyncPgConnection,
    pool: &DbPool,
    provider: impl Provider + Clone + 'static,
    log: &Log,
) -> Result<()> {
    let topic0 = match log.topics().first() {
        Some(t) => *t,
        None => return Ok(()),
    };

    if topic0 == Mint::SIGNATURE_HASH
        || topic0 == Burn::SIGNATURE_HASH
        || topic0 == BalanceTransfer::SIGNATURE_HASH
    {
        process_token_event(conn, log).await
    } else if topic0 == ReserveUsedAsCollateralEnabled::SIGNATURE_HASH
        || topic0 == ReserveUsedAsCollateralDisabled::SIGNATURE_HASH
    {
        process_collateral_event(conn, log).await
    } else {
        process_reserve_event(conn, pool, provider, log).await
    }
}
