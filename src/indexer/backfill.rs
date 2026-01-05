use crate::db::connection::DbPool;
use crate::db::repositories::sync_status_repository;
use crate::indexer::dispatcher::handle_log_logic;
use alloy::primitives::Address;
use alloy::{providers::Provider, rpc::types::eth::Filter};
use alloy_primitives::address;
use alloy_sol_types::SolEvent;
use alloy_sol_types::sol;
use eyre::Result;

sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);

    event ReserveInitialized(
        address indexed asset,
        address indexed aToken,
        address stableDebtToken,
        address variableDebtToken,
        address interestRateStrategyAddress
    );

    event ReserveDataUpdated(
        address indexed reserve,
        uint256 liquidityRate,
        uint256 stableBorrowRate,
        uint256 variableBorrowRate,
        uint256 liquidityIndex,
        uint256 variableBorrowIndex
    );

    event ReserveStableRateBorrowing(
        address indexed asset,
        bool enabled
    );

    event ReserveDropped(
        address indexed asset
    );

    event ReserveFactorChanged(
        address indexed asset,
        uint256 oldReserveFactor,
        uint256 newReserveFactor
    );

    event ReserveInterestRateStrategyChanged(
        address indexed asset,
        address oldStrategy,
        address newStrategy
    );

    event CollateralConfigurationChanged(
        address indexed asset,
        uint256 ltv,
        uint256 liquidationThreshold,
        uint256 liquidationBonus
    );

    event ReserveFrozen(
        address indexed asset
    );

    event ReserveUnfrozen(
        address indexed asset
    );

    event ReservePaused(
        address indexed asset,
        bool paused
    );

    event ReserveBorrowing(
        address indexed asset,
        bool enabled
    );

    event ReserveActive(
        address indexed asset
    );

    event MarketIdSet(
        string indexed oldMarketId,
        string indexed newMarketId
    );

    event BorrowCapChanged(
        address indexed asset,
        uint256 oldBorrowCap,
        uint256 newBorrowCap
    );

    event SupplyCapChanged(
        address indexed asset,
        uint256 oldSupplyCap,
        uint256 newSupplyCap
    );

    event LiquidationProtocolFeeChanged(
        address indexed asset,
        uint256 oldFee,
        uint256 newFee
    );

    event DebtCeilingChanged(
        address indexed asset,
        uint256 oldDebtCeiling,
        uint256 newDebtCeiling
    );

}

pub async fn backfill(
    pool: &DbPool,
    provider: impl Provider + Clone + 'static,
    from_block: i64,
    to_block: i64,
) -> Result<()> {
    let addresses: Vec<Address> = vec![
        address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2"),
        address!("0x64b761D848206f447Fe2dd461b0c635Ec39EbB27"),
    ];

    const CHUNK_SIZE: i64 = 10;

    let mut current = from_block;

    let events = vec![
        Transfer::SIGNATURE_HASH,
        ReserveInitialized::SIGNATURE_HASH,
        ReserveDataUpdated::SIGNATURE_HASH,
        ReserveStableRateBorrowing::SIGNATURE_HASH,
        ReserveDropped::SIGNATURE_HASH,
        ReserveFactorChanged::SIGNATURE_HASH,
        ReserveInterestRateStrategyChanged::SIGNATURE_HASH,
        CollateralConfigurationChanged::SIGNATURE_HASH,
        ReserveFrozen::SIGNATURE_HASH,
        ReserveUnfrozen::SIGNATURE_HASH,
        ReservePaused::SIGNATURE_HASH,
        ReserveBorrowing::SIGNATURE_HASH,
        ReserveActive::SIGNATURE_HASH,
        BorrowCapChanged::SIGNATURE_HASH,
        SupplyCapChanged::SIGNATURE_HASH,
    ];

    while current <= to_block {
        let end = (current + CHUNK_SIZE - 1).min(to_block);

        let filter = Filter::new()
            .from_block(current as u64)
            .to_block(end as u64)
            .address(addresses.clone())
            .event_signature(events.clone());

        tracing::info!("Backfill chunk: {} → {}", current, end);

        let logs = provider.get_logs(&filter).await?;

        let mut logs = logs;
        logs.sort_by_key(|l| (l.block_number, l.log_index));

        for log in logs {
            handle_log_logic(pool, provider.clone(), &log).await?;
        }

        let mut conn = pool.get().await?;

        sync_status_repository::update_last_block(&mut conn, end).await?;

        current = end + 1;
    }

    Ok(())
}
