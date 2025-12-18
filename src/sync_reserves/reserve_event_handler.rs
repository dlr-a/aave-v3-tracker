use crate::db::connection::DbPool;
use crate::db::repositories::{reserve_state_repository, reserves_repository};
use crate::sync_reserves::fetch_reserves::process_reserve;
use alloy::primitives::Uint;
use alloy::primitives::address;
use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::eth::Filter,
    rpc::types::eth::Log,
    sol,
    sol_types::SolEvent,
};
use bigdecimal::BigDecimal;
use eyre::eyre;
use eyre::{Context, Result};
use futures_util::StreamExt;
use std::str::FromStr;
use tracing::{error, info};

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

        _ => None,
    }
}

fn to_bigdecimal<const BITS: usize, const LIMBS: usize>(
    val: Uint<BITS, LIMBS>,
) -> Result<BigDecimal> {
    BigDecimal::from_str(&val.to_string()).map_err(|e| eyre!("BigDecimal conversion error: {}", e))
}

sol! {
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

    #[sol(rpc)]
    interface IProtocolDataProvider {
        #[derive(Debug)]
        struct TokenData { string symbol; address tokenAddress; }
        function getAllReservesTokens() external view returns (TokenData[] memory);
        function getReserveTokensAddresses(address asset) external view override returns (address aTokenAddress, address stableDebtTokenAddress, address variableDebtTokenAddress);
        function getReserveConfigurationData(address asset) external view override returns (uint256 decimals, uint256 ltv, uint256 liquidationThreshold, uint256 liquidationBonus, uint256 reserveFactor, bool usageAsCollateralEnabled, bool borrowingEnabled, bool stableBorrowRateEnabled, bool isActive, bool isFrozen);
    }

}

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
        ]);

    let sub_events = provider.subscribe_logs(&filter).await?;
    let mut stream = sub_events.into_stream();

    info!("Reserve Handler Started...");

    while let Some(log) = stream.next().await {
        let block_number = log.block_number.unwrap_or(0) as i64;
        let log_index = log.log_index.unwrap_or(0) as i64;

        let log_data = match decode_log_type(&log) {
            Some(data) => data,
            None => continue,
        };

        let data_provider_addr = address!("0x0a16f2FCC0D44FaE41cc54e079281D84A363bECD");
        let pool_addr = address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2");

        let data_provider = IProtocolDataProvider::new(data_provider_addr, provider.clone());

        match log_data {
            ProcessedLog::ReserveData(e) => {
                let asset = e.reserve.to_string();
                let result = reserve_state_repository::update_financials(
                    pool,
                    asset.clone(),
                    to_bigdecimal(e.liquidityIndex)?,
                    to_bigdecimal(e.variableBorrowIndex)?,
                    to_bigdecimal(e.liquidityRate)?,
                    to_bigdecimal(e.variableBorrowRate)?,
                    to_bigdecimal(e.stableBorrowRate)?,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = result {
                    error!("DB Error (Rates): {}", err);
                }
            }

            ProcessedLog::ReserveInitialized(e) => {
                let asset_address = e.asset;

                process_reserve(
                    &pool.clone(),
                    &provider.clone(),
                    asset_address,
                    data_provider_addr,
                    pool_addr,
                )
                .await?;
            }

            ProcessedLog::CollateralConfig(e) => {
                let asset = e.asset.to_string();
                let result = reserves_repository::update_risk_config(
                    pool,
                    asset.clone(),
                    e.ltv.to::<u64>() as i64,
                    e.liquidationThreshold.to::<u64>() as i64,
                    e.liquidationBonus.to::<u64>() as i64,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = result {
                    error!("DB Error (Config): {}", err);
                }
            }

            ProcessedLog::ReserveFrozen(e) => {
                let asset = e.asset.to_string();
                let res = reserves_repository::set_frozen_status(
                    pool,
                    asset.clone(),
                    true,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Frozen): {}", err);
                } else {
                    info!("Frozen: {}", asset);
                }
            }

            ProcessedLog::ReserveUnfrozen(e) => {
                let asset = e.asset.to_string();
                let res = reserves_repository::set_frozen_status(
                    pool,
                    asset.clone(),
                    false,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Unfrozen): {}", err);
                } else {
                    info!("Unfrozen: {}", asset);
                }
            }

            ProcessedLog::ReservePaused(e) => {
                let asset = e.asset.to_string();
                let res = reserves_repository::set_paused_status(
                    pool,
                    asset.clone(),
                    true,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Paused): {}", err);
                } else {
                    info!("⏸Paused: {}", asset);
                }
            }

            ProcessedLog::ReserveBorrowing(e) => {
                let asset = e.asset.to_string();
                let res = reserves_repository::set_borrowing_status(
                    pool,
                    asset.clone(),
                    e.enabled,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Borrowing): {}", err);
                }
            }

            ProcessedLog::ReserveActive(e) => {
                let asset = e.asset.to_string();
                let res = reserves_repository::set_active_status(
                    pool,
                    asset.clone(),
                    true,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Active): {}", err);
                }
            }

            ProcessedLog::ReserveDropped(e) => {
                let asset = e.asset.to_string();
                let res = reserves_repository::set_dropped_status(
                    pool,
                    asset.clone(),
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Dropped): {}", err);
                } else {
                    info!("Dropped: {}", asset);
                }
            }

            ProcessedLog::InterestRateStrategy(e) => {
                let asset = e.asset.to_string();
                let new_strategy = e.newStrategy.to_string();
                let res = reserves_repository::update_strategy_address(
                    pool,
                    asset.clone(),
                    new_strategy,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Strategy): {}", err);
                }
            }

            ProcessedLog::ReserveStableRateBorrowing(e) => {
                let asset = e.asset;
                let token_addresses = data_provider
                    .getReserveTokensAddresses(asset)
                    .call()
                    .await?;

                let stable_borrow_addr = token_addresses.stableDebtTokenAddress;

                let res = reserves_repository::update_stable_borrow_address(
                    pool,
                    asset.to_string().clone(),
                    stable_borrow_addr.to_string(),
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Stable Borrowing): {}", err);
                }
            }

            ProcessedLog::SupplyCapChanged(e) => {
                let asset = e.asset;

                let res = reserves_repository::update_supply_cap(
                    pool,
                    asset.to_string().clone(),
                    to_bigdecimal(e.newSupplyCap)?,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Supply Cap): {}", err);
                }
            }

            ProcessedLog::BorrowCapChanged(e) => {
                let asset = e.asset;

                let res = reserves_repository::update_borrow_cap(
                    pool,
                    asset.to_string().clone(),
                    to_bigdecimal(e.newBorrowCap)?,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Borrow Cap): {}", err);
                }
            }

            ProcessedLog::ReserveFactorChanged(e) => {
                let asset = e.asset;

                let res = reserves_repository::update_reserve_factor(
                    pool,
                    asset.to_string().clone(),
                    e.newReserveFactor.to::<u64>() as i64,
                    block_number,
                    log_index,
                )
                .await;
                if let Err(err) = res {
                    error!("DB Error (Reserve Factor): {}", err);
                }
            }
        }
    }

    Ok(())
}
