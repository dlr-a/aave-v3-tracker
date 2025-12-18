use crate::db::connection::DbPool;
use crate::db::models::{NewReserve, NewReserveState};
use crate::db::repositories::{reserve_state_repository, reserves_repository};
use alloy::primitives::{Address, U256, Uint, address};
use alloy::providers::Provider;
use alloy::{
    providers::{ProviderBuilder, WsConnect},
    sol,
};
use backoff::ExponentialBackoff;
use backoff::future::retry;
use bigdecimal::BigDecimal;
use eyre::{Context, Result, eyre};
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info};

fn to_bigdecimal<const BITS: usize, const LIMBS: usize>(
    val: Uint<BITS, LIMBS>,
) -> Result<BigDecimal> {
    BigDecimal::from_str(&val.to_string()).map_err(|e| eyre!("BigDecimal conversion error: {}", e))
}

sol! {
    #[sol(rpc)]
    interface IProtocolDataProvider {
        #[derive(Debug)]
        struct TokenData { string symbol; address tokenAddress; }
        function getAllReservesTokens() external view returns (TokenData[] memory);
        function getReserveTokensAddresses(address asset) external view override returns (address aTokenAddress, address stableDebtTokenAddress, address variableDebtTokenAddress);
        function getReserveConfigurationData(address asset) external view override returns (uint256 decimals, uint256 ltv, uint256 liquidationThreshold, uint256 liquidationBonus, uint256 reserveFactor, bool usageAsCollateralEnabled, bool borrowingEnabled, bool stableBorrowRateEnabled, bool isActive, bool isFrozen);

        function getReserveCaps(address asset) external view override returns (uint256 borrowCap, uint256 supplyCap);

    }
    #[sol(rpc)]
    interface IERC20 {
        function symbol() external view returns (string);

        function totalSupply() external view returns (uint256);
    }

    #[sol(rpc)]
    interface IPool {
        struct ReserveConfigurationMap {
            uint256 data;
        }

        struct ReserveData {
            ReserveConfigurationMap configuration;
            uint128 liquidityIndex;
            uint128 currentLiquidityRate;
            uint128 variableBorrowIndex;
            uint128 currentVariableBorrowRate;
            uint128 currentStableBorrowRate;
            uint40 lastUpdateTimestamp;
            uint16 id;
            address aTokenAddress;
            address stableDebtTokenAddress;
            address variableDebtTokenAddress;
            address interestRateStrategyAddress;
            uint128 accruedToTreasury;
            uint128 unbacked;
            uint128 isolationModeTotalDebt;
        }

        function getReserveData(address asset) external view returns (ReserveData memory);
    }
}

pub async fn fetch_reserves(pool: &DbPool, rpc_url: String) -> Result<()> {
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new()
        .connect_ws(ws)
        .await
        .wrap_err("Couldn't connect to the WS")?;

    let data_provider_addr = address!("0x0a16f2FCC0D44FaE41cc54e079281D84A363bECD");
    let pool_addr = address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2");

    let protocol_data_provider = IProtocolDataProvider::new(data_provider_addr, provider.clone());

    let tokens = protocol_data_provider
        .getAllReservesTokens()
        .call()
        .await
        .wrap_err("Failed to fetch all reserve tokens from ProtocolDataProvider")?;

    for token in tokens {
        let backoff_strategy = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(60)),
            ..Default::default()
        };

        let op = || async {
            let p_clone = provider.clone();
            let pool_clone = pool.clone();

            process_reserve(
                &pool_clone,
                &p_clone,
                token.tokenAddress,
                data_provider_addr,
                pool_addr,
            )
            .await
            .map_err(|e| {
                info!("Error fetching {}: {}. Retrying...", token.symbol, e);
                backoff::Error::transient(e)
            })
        };

        match retry(backoff_strategy, op).await {
            Ok(_) => info!("Synced: {}", token.symbol),
            Err(e) => {
                error!(
                    "FAILED PERMANENTLY: Could not sync {:?}. Error: {:?}",
                    token.symbol, e
                );
            }
        }
    }
    Ok(())
}

pub async fn process_reserve<P>(
    pool: &DbPool,
    provider: &P,
    asset_address: Address,
    dp_addr: Address,
    pool_addr: Address,
) -> Result<()>
where
    P: Provider + Clone + 'static,
{
    let data_provider = IProtocolDataProvider::new(dp_addr, provider.clone());
    let pool_contract = IPool::new(pool_addr, provider.clone());

    let current_block = provider
        .get_block_number()
        .await
        .wrap_err("Failed to fetch current block number")?;

    let token_addresses = data_provider
        .getReserveTokensAddresses(asset_address)
        .call()
        .await?;
    let reserve_config = data_provider
        .getReserveConfigurationData(asset_address)
        .call()
        .await?;
    let pool_data = pool_contract.getReserveData(asset_address).call().await?;
    let caps = data_provider.getReserveCaps(asset_address).call().await?;

    let erc20 = IERC20::new(asset_address, provider.clone());
    let symbol = erc20
        .symbol()
        .call()
        .await
        .map(|s| s)
        .unwrap_or("UNKNOWN".to_string());

    let config_data: U256 = pool_data.configuration.data;
    let reserve_factor = reserve_config.reserveFactor.to::<u64>() as i64;
    let is_paused = !((config_data >> 60usize).bitand(U256::from(1)).is_zero());
    let reserve_id = pool_data.id as i32;

    let ltv = reserve_config.ltv.to::<u64>() as i64;
    let threshold = reserve_config.liquidationThreshold.to::<u64>() as i64;
    let bonus = reserve_config.liquidationBonus.to::<u64>() as i64;

    let atoken = IERC20::new(token_addresses.aTokenAddress, provider.clone());
    let total_liquidity_raw = atoken.totalSupply().call().await?;

    let vtoken = IERC20::new(token_addresses.variableDebtTokenAddress, provider.clone());
    let total_variable_raw = vtoken.totalSupply().call().await?;

    let total_stable_raw = if token_addresses.stableDebtTokenAddress != Address::ZERO {
        let stoken = IERC20::new(token_addresses.stableDebtTokenAddress, provider.clone());
        stoken.totalSupply().call().await?
    } else {
        U256::ZERO
    };

    let reserve_data = NewReserve {
        asset_address: asset_address.to_string(),
        symbol: symbol.clone(),
        decimals: reserve_config.decimals.to::<u64>() as i64,
        reserve_id,
        ltv,
        liquidation_threshold: threshold,
        liquidation_bonus: bonus,
        is_active: reserve_config.isActive,
        is_frozen: reserve_config.isFrozen,
        is_paused,
        supply_cap: to_bigdecimal(caps.supplyCap)?,
        borrow_cap: to_bigdecimal(caps.borrowCap)?,
        reserve_factor,
        is_borrowing_enabled: reserve_config.borrowingEnabled,
        is_dropped: false,
        atoken_address: token_addresses.aTokenAddress.to_string(),
        v_debt_token_address: token_addresses.variableDebtTokenAddress.to_string(),
        s_debt_token_address: token_addresses.stableDebtTokenAddress.to_string(),
        interest_rate_strategy_address: pool_data.interestRateStrategyAddress.to_string(),
        last_updated_block: current_block as i64,
    };

    let reserve_state = NewReserveState {
        asset_address: asset_address.to_string(),
        liquidity_index: to_bigdecimal(U256::from(pool_data.liquidityIndex))?,
        variable_borrow_index: to_bigdecimal(U256::from(pool_data.variableBorrowIndex))?,
        current_liquidity_rate: to_bigdecimal(U256::from(pool_data.currentLiquidityRate))?,
        current_variable_borrow_rate: to_bigdecimal(U256::from(
            pool_data.currentVariableBorrowRate,
        ))?,
        current_stable_borrow_rate: to_bigdecimal(U256::from(pool_data.currentStableBorrowRate))?,

        total_liquidity: to_bigdecimal(total_liquidity_raw)?,
        total_variable_debt: to_bigdecimal(total_variable_raw)?,
        total_stable_debt: to_bigdecimal(total_stable_raw)?,

        accrued_to_treasury: to_bigdecimal(U256::from(pool_data.accruedToTreasury))?,
        unbacked: to_bigdecimal(U256::from(pool_data.unbacked))?,
        isolation_mode_total_debt: to_bigdecimal(U256::from(pool_data.isolationModeTotalDebt))?,
        last_updated_block: current_block as i64,
    };

    reserves_repository::sync_reserve(pool, reserve_data).await?;
    reserve_state_repository::sync_state(pool, reserve_state).await?;

    Ok(())
}
