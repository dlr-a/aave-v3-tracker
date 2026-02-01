use crate::abi::{
    BorrowCapChanged, CollateralConfigurationChanged, DebtCeilingChanged,
    EModeAssetCategoryChanged, IProtocolDataProvider, LiquidationProtocolFeeChanged, ReserveActive,
    ReserveBorrowing, ReserveDataUpdated, ReserveDropped, ReserveFactorChanged,
    ReserveFlashLoaning, ReserveFrozen, ReserveInitialized, ReserveInterestRateStrategyChanged,
    ReservePaused, ReserveStableRateBorrowing, ReserveUnfrozen, SiloedBorrowingChanged,
    SupplyCapChanged, UnbackedMintCapChanged,
};
use crate::db::connection::DbPool;
use crate::db::repositories::{
    processed_events_repository, reserve_state_repository, reserves_repository,
};
use crate::sync_reserves::fetch_reserves::process_reserve;
use alloy::primitives::Uint;
use alloy::primitives::address;
use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::eth::Filter,
    rpc::types::eth::Log,
    sol_types::SolEvent,
};
use backoff::{ExponentialBackoff, future::retry};
use bigdecimal::BigDecimal;
use diesel_async::AsyncPgConnection;
use eyre::eyre;
use eyre::{Context, Result};
use futures_util::StreamExt;
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info, warn};

pub enum ProcessedLog {
    ReserveData(ReserveDataUpdated),
    ReserveInitialized(ReserveInitialized),
    CollateralConfig(CollateralConfigurationChanged),
    ReserveFrozen(ReserveFrozen),
    ReserveUnfrozen(ReserveUnfrozen),
    ReservePaused(ReservePaused),
    ReserveBorrowing(ReserveBorrowing),
    ReserveActive(ReserveActive),
    ReserveDropped(ReserveDropped),
    InterestRateStrategy(ReserveInterestRateStrategyChanged),
    ReserveStableRateBorrowing(ReserveStableRateBorrowing),
    SupplyCapChanged(SupplyCapChanged),
    BorrowCapChanged(BorrowCapChanged),
    ReserveFactorChanged(ReserveFactorChanged),
    ReserveFlashLoaning(ReserveFlashLoaning),
    EModeAssetCategoryChanged(EModeAssetCategoryChanged),
    DebtCeilingChanged(DebtCeilingChanged),
    LiquidationProtocolFeeChanged(LiquidationProtocolFeeChanged),
    SiloedBorrowingChanged(SiloedBorrowingChanged),
    UnbackedMintCapChanged(UnbackedMintCapChanged),
}

pub fn decode_log_type(log: &Log) -> Option<ProcessedLog> {
    let topic0 = log.topics().get(0)?;

    match *topic0 {
        ReserveDataUpdated::SIGNATURE_HASH => log
            .log_decode::<ReserveDataUpdated>()
            .ok()
            .map(|e| ProcessedLog::ReserveData(e.data().clone())),

        ReserveInitialized::SIGNATURE_HASH => log
            .log_decode::<ReserveInitialized>()
            .ok()
            .map(|e| ProcessedLog::ReserveInitialized(e.data().clone())),

        CollateralConfigurationChanged::SIGNATURE_HASH => log
            .log_decode::<CollateralConfigurationChanged>()
            .ok()
            .map(|e| ProcessedLog::CollateralConfig(e.data().clone())),

        ReserveFrozen::SIGNATURE_HASH => log
            .log_decode::<ReserveFrozen>()
            .ok()
            .map(|e| ProcessedLog::ReserveFrozen(e.data().clone())),

        ReserveUnfrozen::SIGNATURE_HASH => log
            .log_decode::<ReserveUnfrozen>()
            .ok()
            .map(|e| ProcessedLog::ReserveUnfrozen(e.data().clone())),

        ReservePaused::SIGNATURE_HASH => log
            .log_decode::<ReservePaused>()
            .ok()
            .map(|e| ProcessedLog::ReservePaused(e.data().clone())),

        ReserveBorrowing::SIGNATURE_HASH => log
            .log_decode::<ReserveBorrowing>()
            .ok()
            .map(|e| ProcessedLog::ReserveBorrowing(e.data().clone())),

        ReserveActive::SIGNATURE_HASH => log
            .log_decode::<ReserveActive>()
            .ok()
            .map(|e| ProcessedLog::ReserveActive(e.data().clone())),

        ReserveDropped::SIGNATURE_HASH => log
            .log_decode::<ReserveDropped>()
            .ok()
            .map(|e| ProcessedLog::ReserveDropped(e.data().clone())),

        ReserveInterestRateStrategyChanged::SIGNATURE_HASH => log
            .log_decode::<ReserveInterestRateStrategyChanged>()
            .ok()
            .map(|e| ProcessedLog::InterestRateStrategy(e.data().clone())),

        ReserveStableRateBorrowing::SIGNATURE_HASH => log
            .log_decode::<ReserveStableRateBorrowing>()
            .ok()
            .map(|e| ProcessedLog::ReserveStableRateBorrowing(e.data().clone())),

        SupplyCapChanged::SIGNATURE_HASH => log
            .log_decode::<SupplyCapChanged>()
            .ok()
            .map(|e| ProcessedLog::SupplyCapChanged(e.data().clone())),

        BorrowCapChanged::SIGNATURE_HASH => log
            .log_decode::<BorrowCapChanged>()
            .ok()
            .map(|e| ProcessedLog::BorrowCapChanged(e.data().clone())),

        ReserveFactorChanged::SIGNATURE_HASH => log
            .log_decode::<ReserveFactorChanged>()
            .ok()
            .map(|e| ProcessedLog::ReserveFactorChanged(e.data().clone())),
        ReserveFlashLoaning::SIGNATURE_HASH => log
            .log_decode::<ReserveFlashLoaning>()
            .ok()
            .map(|e| ProcessedLog::ReserveFlashLoaning(e.data().clone())),

        EModeAssetCategoryChanged::SIGNATURE_HASH => log
            .log_decode::<EModeAssetCategoryChanged>()
            .ok()
            .map(|e| ProcessedLog::EModeAssetCategoryChanged(e.data().clone())),

        DebtCeilingChanged::SIGNATURE_HASH => log
            .log_decode::<DebtCeilingChanged>()
            .ok()
            .map(|e| ProcessedLog::DebtCeilingChanged(e.data().clone())),

        LiquidationProtocolFeeChanged::SIGNATURE_HASH => log
            .log_decode::<LiquidationProtocolFeeChanged>()
            .ok()
            .map(|e| ProcessedLog::LiquidationProtocolFeeChanged(e.data().clone())),

        SiloedBorrowingChanged::SIGNATURE_HASH => log
            .log_decode::<SiloedBorrowingChanged>()
            .ok()
            .map(|e| ProcessedLog::SiloedBorrowingChanged(e.data().clone())),

        UnbackedMintCapChanged::SIGNATURE_HASH => log
            .log_decode::<UnbackedMintCapChanged>()
            .ok()
            .map(|e| ProcessedLog::UnbackedMintCapChanged(e.data().clone())),

        _ => None,
    }
}

fn to_bigdecimal<const BITS: usize, const LIMBS: usize>(
    val: Uint<BITS, LIMBS>,
) -> Result<BigDecimal> {
    BigDecimal::from_str(&val.to_string()).map_err(|e| eyre!("BigDecimal conversion error: {}", e))
}

#[allow(dead_code)]
pub async fn reserve_event_handler(pool: &DbPool, rpc_url: String) -> Result<()> {
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new()
        .connect_ws(ws)
        .await
        .wrap_err("Couldn't connect to the WS")?;

    let pool_address = address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2");
    let pool_configurator = address!("0x64b761D848206f447Fe2dd461b0c635Ec39EbB27");

    let filter = Filter::new()
        .address(vec![pool_address, pool_configurator])
        .event_signature(vec![
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
            ReserveFlashLoaning::SIGNATURE_HASH,
            EModeAssetCategoryChanged::SIGNATURE_HASH,
            DebtCeilingChanged::SIGNATURE_HASH,
            LiquidationProtocolFeeChanged::SIGNATURE_HASH,
            SiloedBorrowingChanged::SIGNATURE_HASH,
            UnbackedMintCapChanged::SIGNATURE_HASH,
        ]);

    let sub_events = provider.subscribe_logs(&filter).await?;
    let mut stream = sub_events.into_stream();

    info!("Reserve Handler Started...");

    while let Some(log) = stream.next().await {
        let mut conn = pool.get().await?;
        if let Err(e) = process_reserve_event(&mut conn, pool, provider.clone(), &log).await {
            error!(error = ?e, "Failed to process reserve event, continuing...");
        }
    }

    Ok(())
}

pub async fn process_reserve_event(
    conn: &mut AsyncPgConnection,
    pool: &DbPool,
    provider: impl Provider + Clone + 'static,
    log: &Log,
) -> Result<()> {
    let block_number = log.block_number.unwrap_or(0) as i64;
    let log_index = log.log_index.unwrap_or(0) as i64;
    let tx_hash = log.transaction_hash.unwrap().to_string();

    let log_data = match decode_log_type(&log) {
        Some(data) => data,
        None => return Ok(()),
    };

    let inserted =
        processed_events_repository::try_insert_event(conn, tx_hash, log_index, block_number)
            .await?;

    if !inserted {
        return Ok(());
    }

    let data_provider_addr = address!("0x0a16f2FCC0D44FaE41cc54e079281D84A363bECD");
    let pool_addr = address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2");

    let data_provider = IProtocolDataProvider::new(data_provider_addr, provider.clone());

    let rpc_backoff = ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(30)),
        initial_interval: Duration::from_millis(100),
        max_interval: Duration::from_secs(5),
        multiplier: 2.0,
        ..Default::default()
    };

    match log_data {
        ProcessedLog::ReserveData(e) => {
            let asset = e.reserve.to_string();
            reserve_state_repository::update_financials(
                conn,
                asset.clone(),
                to_bigdecimal(e.liquidityIndex)?,
                to_bigdecimal(e.variableBorrowIndex)?,
                to_bigdecimal(e.liquidityRate)?,
                to_bigdecimal(e.variableBorrowRate)?,
                to_bigdecimal(e.stableBorrowRate)?,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update financials for asset {}", asset))?;
        }

        ProcessedLog::ReserveInitialized(e) => {
            let asset_address = e.asset;

            process_reserve(
                pool,
                &provider,
                asset_address,
                data_provider_addr,
                pool_addr,
            )
            .await
            .wrap_err_with(|| {
                format!(
                    "Failed to process reserve initialization for {}",
                    asset_address
                )
            })?;
        }

        ProcessedLog::CollateralConfig(e) => {
            let asset = e.asset.to_string();
            reserves_repository::update_risk_config(
                conn,
                asset.clone(),
                e.ltv.to::<u64>() as i64,
                e.liquidationThreshold.to::<u64>() as i64,
                e.liquidationBonus.to::<u64>() as i64,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update risk config for {}", asset))?;
        }

        ProcessedLog::ReserveFrozen(e) => {
            let asset = e.asset.to_string();
            reserves_repository::set_frozen_status(
                conn,
                asset.clone(),
                true,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to set frozen status for {}", asset))?;
            info!("Frozen: {}", asset);
        }

        ProcessedLog::ReserveUnfrozen(e) => {
            let asset = e.asset.to_string();
            reserves_repository::set_frozen_status(
                conn,
                asset.clone(),
                false,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to unfreeze {}", asset))?;
            info!("Unfrozen: {}", asset);
        }

        ProcessedLog::ReservePaused(e) => {
            let asset = e.asset.to_string();
            reserves_repository::set_paused_status(
                conn,
                asset.clone(),
                true,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to pause {}", asset))?;
            info!("⏸ Paused: {}", asset);
        }

        ProcessedLog::ReserveBorrowing(e) => {
            let asset = e.asset.to_string();
            reserves_repository::set_borrowing_status(
                conn,
                asset.clone(),
                e.enabled,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update borrowing status for {}", asset))?;
        }

        ProcessedLog::ReserveActive(e) => {
            let asset = e.asset.to_string();
            reserves_repository::set_active_status(
                conn,
                asset.clone(),
                true,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to activate {}", asset))?;
        }

        ProcessedLog::ReserveDropped(e) => {
            let asset = e.asset.to_string();
            reserves_repository::set_dropped_status(conn, asset.clone(), block_number, log_index)
                .await
                .wrap_err_with(|| format!("Failed to mark {} as dropped", asset))?;
            info!("Dropped: {}", asset);
        }

        ProcessedLog::InterestRateStrategy(e) => {
            let asset = e.asset.to_string();
            let new_strategy = e.newStrategy.to_string();
            reserves_repository::update_strategy_address(
                conn,
                asset.clone(),
                new_strategy,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update strategy for {}", asset))?;
        }
        ProcessedLog::ReserveStableRateBorrowing(e) => {
            let asset = e.asset;

            reserves_repository::set_stable_borrow_status(
                conn,
                asset.to_string(),
                e.enabled,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update stable borrow status for {}", asset))?;

            let token_addresses = retry(rpc_backoff, || {
                let dp = data_provider.clone();
                let asset_clone = asset.clone();
                async move {
                    dp.getReserveTokensAddresses(asset_clone)
                        .call()
                        .await
                        .map_err(|e| {
                            warn!("RPC error fetching token addresses: {}", e);
                            backoff::Error::transient(e)
                        })
                }
            })
            .await
            .wrap_err_with(|| format!("Failed to fetch token addresses for {}", asset))?;

            let stable_borrow_addr = token_addresses.stableDebtTokenAddress;

            reserves_repository::update_stable_borrow_address(
                conn,
                asset.to_string(),
                stable_borrow_addr.to_string(),
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update stable borrow address for {}", asset))?;

            info!("Stable rate borrowing changed for {}: {}", asset, e.enabled);
        }

        ProcessedLog::SupplyCapChanged(e) => {
            let asset = e.asset.to_string();
            reserves_repository::update_supply_cap(
                conn,
                asset.clone(),
                to_bigdecimal(e.newSupplyCap)?,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update supply cap for {}", asset))?;
        }

        ProcessedLog::BorrowCapChanged(e) => {
            let asset = e.asset.to_string();
            reserves_repository::update_borrow_cap(
                conn,
                asset.clone(),
                to_bigdecimal(e.newBorrowCap)?,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update borrow cap for {}", asset))?;
        }

        ProcessedLog::ReserveFactorChanged(e) => {
            let asset = e.asset.to_string();
            reserves_repository::update_reserve_factor(
                conn,
                asset.clone(),
                e.newReserveFactor.to::<u64>() as i64,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update reserve factor for {}", asset))?;
        }

        ProcessedLog::ReserveFlashLoaning(e) => {
            let asset = e.asset.to_string();
            reserves_repository::set_flash_loan_status(
                conn,
                asset.clone(),
                e.enabled,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update flash loan status for {}", asset))?;
            info!("Flash loan status changed for {}: {}", asset, e.enabled);
        }

        ProcessedLog::EModeAssetCategoryChanged(e) => {
            let asset = e.asset.to_string();
            reserves_repository::update_emode_category(
                conn,
                asset.clone(),
                e.newCategoryId as i32,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update eMode category for {}", asset))?;
            info!(
                "eMode category changed for {}: {} -> {}",
                asset, e.oldCategoryId, e.newCategoryId
            );
        }

        ProcessedLog::DebtCeilingChanged(e) => {
            let asset = e.asset.to_string();
            reserves_repository::update_debt_ceiling(
                conn,
                asset.clone(),
                to_bigdecimal(e.newDebtCeiling)?,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update debt ceiling for {}", asset))?;
            info!("Debt ceiling changed for {}", asset);
        }

        ProcessedLog::LiquidationProtocolFeeChanged(e) => {
            let asset = e.asset.to_string();
            reserves_repository::update_liquidation_protocol_fee(
                conn,
                asset.clone(),
                e.newFee.to::<u64>() as i64,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update liquidation protocol fee for {}", asset))?;
            info!("Liquidation protocol fee changed for {}", asset);
        }

        ProcessedLog::SiloedBorrowingChanged(e) => {
            let asset = e.asset.to_string();
            reserves_repository::set_siloed_borrowing_status(
                conn,
                asset.clone(),
                e.newState,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update siloed borrowing for {}", asset))?;
            info!("Siloed borrowing changed for {}: {}", asset, e.newState);
        }

        ProcessedLog::UnbackedMintCapChanged(e) => {
            let asset = e.asset.to_string();
            reserves_repository::update_unbacked_mint_cap(
                conn,
                asset.clone(),
                to_bigdecimal(e.newUnbackedMintCap)?,
                block_number,
                log_index,
            )
            .await
            .wrap_err_with(|| format!("Failed to update unbacked mint cap for {}", asset))?;
            info!("Unbacked mint cap changed for {}", asset);
        }
    }

    Ok(())
}
