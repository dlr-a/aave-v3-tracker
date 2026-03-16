use crate::abi::{IERC20, IPool, IProtocolDataProvider, IUiPoolDataProviderV3};
use crate::db::connection::DbPool;
use crate::db::models::{NewEmodeCategory, NewReserve, NewReserveState};
use crate::db::repositories::{emode_categories_repository, reserve_state_repository, reserves_repository};
use alloy::primitives::{Address, U256, Uint, address};
use alloy::providers::{Provider, ProviderBuilder};
use bigdecimal::BigDecimal;
use eyre::{Result, eyre};
use std::str::FromStr;
use tracing::{error, info};

fn to_bigdecimal<const BITS: usize, const LIMBS: usize>(
    val: Uint<BITS, LIMBS>,
) -> Result<BigDecimal> {
    BigDecimal::from_str(&val.to_string()).map_err(|e| eyre!("BigDecimal conversion error: {}", e))
}

fn to_i64<const BITS: usize, const LIMBS: usize>(
    val: Uint<BITS, LIMBS>,
    field_name: &str,
) -> Result<i64> {
    let as_u64 = val.to::<u64>();
    i64::try_from(as_u64).map_err(|_| eyre!("{} overflow: {} exceeds i64::MAX", field_name, as_u64))
}

pub async fn fetch_reserves(pool: &DbPool, rpc_url: String) -> Result<()> {
    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);

    let data_provider_addr = address!("0x0a16f2FCC0D44FaE41cc54e079281D84A363bECD");
    let pool_addr = address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2");
    let ui_pool_data_provider_addr = address!("0x56b7A1012765C285afAC8b8F25C69Bf10ccfE978");
    let addresses_provider_addr = address!("0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e");

    let protocol_data_provider = IProtocolDataProvider::new(data_provider_addr, provider.clone());

    let tokens = protocol_data_provider.getAllReservesTokens().call().await?;

    for token in tokens {
        match process_reserve(
            pool,
            &provider,
            token.tokenAddress,
            data_provider_addr,
            pool_addr,
        )
        .await
        {
            Ok(_) => info!("Synced: {}", token.symbol),
            Err(e) => {
                error!(
                    symbol = %token.symbol,
                    address = %token.tokenAddress,
                    error = %e,
                    "Failed to sync reserve"
                );
                return Err(e);
            }
        }
    }

    fetch_emode_categories(pool, &provider, ui_pool_data_provider_addr, addresses_provider_addr).await?;

    Ok(())
}

async fn fetch_emode_categories<P>(
    pool: &DbPool,
    provider: &P,
    ui_pool_data_provider_addr: Address,
    addresses_provider_addr: Address,
) -> Result<()>
where
    P: Provider + Clone + 'static,
{
    let ui_data_provider = IUiPoolDataProviderV3::new(ui_pool_data_provider_addr, provider.clone());

    let emodes = ui_data_provider
        .getEModes(addresses_provider_addr)
        .call()
        .await?;

    let count = emodes.len();
    let mut conn = pool.get().await?;
    for emode in emodes {
        emode_categories_repository::upsert(
            &mut conn,
            NewEmodeCategory {
                category_id: emode.id as i32,
                ltv: emode.eMode.ltv as i64,
                liquidation_threshold: emode.eMode.liquidationThreshold as i64,
                liquidation_bonus: emode.eMode.liquidationBonus as i64,
                collateral_bitmap: to_bigdecimal(U256::from(emode.eMode.collateralBitmap))?,
                borrowable_bitmap: to_bigdecimal(U256::from(emode.eMode.borrowableBitmap))?,
                ltvzero_bitmap: to_bigdecimal(U256::from(emode.eMode.ltvzeroBitmap))?,
                label: emode.eMode.label,
                last_updated_block: 0,
                last_updated_log_index: -1,
            },
        )
        .await?;
    }

    info!("Synced {} eMode categories", count);
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
    let erc20 = IERC20::new(asset_address, provider.clone());

    let (token_addresses, reserve_config, pool_data, caps, current_block) = provider
        .multicall()
        .add(data_provider.getReserveTokensAddresses(asset_address))
        .add(data_provider.getReserveConfigurationData(asset_address))
        .add(pool_contract.getReserveData(asset_address))
        .add(data_provider.getReserveCaps(asset_address))
        .get_block_number()
        .aggregate()
        .await?;

    let symbol = erc20.symbol().call().await.unwrap_or("UNKNOWN".to_string());

    let optional_result = provider
        .multicall()
        .add(data_provider.getFlashLoanEnabled(asset_address))
        .add(data_provider.getDebtCeiling(asset_address))
        .add(data_provider.getLiquidationProtocolFee(asset_address))
        .add(data_provider.getSiloedBorrowing(asset_address))
        .add(data_provider.getUnbackedMintCap(asset_address))
        .aggregate()
        .await;

    let (
        flash_loan_enabled,
        debt_ceiling,
        liquidation_protocol_fee,
        siloed_borrowing,
        unbacked_mint_cap,
    ) = match optional_result {
        Ok(result) => result,
        Err(_) => (true, U256::ZERO, U256::ZERO, false, U256::ZERO),
    };

    let atoken = IERC20::new(token_addresses.aTokenAddress, provider.clone());
    let vtoken = IERC20::new(token_addresses.variableDebtTokenAddress, provider.clone());

    let (total_liquidity_raw, total_variable_raw) = provider
        .multicall()
        .add(atoken.totalSupply())
        .add(vtoken.totalSupply())
        .aggregate()
        .await?;

    let total_stable_raw = if token_addresses.stableDebtTokenAddress != Address::ZERO {
        let stoken = IERC20::new(token_addresses.stableDebtTokenAddress, provider.clone());
        stoken.totalSupply().call().await.unwrap_or(U256::ZERO)
    } else {
        U256::ZERO
    };

    let config_data: U256 = pool_data.configuration.data;
    let is_paused = !((config_data >> 60usize).bitand(U256::from(1)).is_zero());

    let reserve_data = NewReserve {
        asset_address: asset_address.to_string(),
        symbol: symbol.clone(),
        decimals: to_i64(reserve_config.decimals, "decimals")?,
        reserve_id: pool_data.id as i32,
        ltv: to_i64(reserve_config.ltv, "ltv")?,
        liquidation_threshold: to_i64(
            reserve_config.liquidationThreshold,
            "liquidation_threshold",
        )?,
        liquidation_bonus: to_i64(reserve_config.liquidationBonus, "liquidation_bonus")?,
        is_active: reserve_config.isActive,
        is_frozen: reserve_config.isFrozen,
        is_paused,
        is_borrowing_enabled: reserve_config.borrowingEnabled,
        is_dropped: false,
        supply_cap: to_bigdecimal(caps.supplyCap)?,
        borrow_cap: to_bigdecimal(caps.borrowCap)?,
        reserve_factor: to_i64(reserve_config.reserveFactor, "reserve_factor")?,
        is_collateral_enabled: reserve_config.usageAsCollateralEnabled,
        is_stable_borrow_enabled: reserve_config.stableBorrowRateEnabled,
        is_flash_loan_enabled: flash_loan_enabled,
        debt_ceiling: to_bigdecimal(debt_ceiling)?,
        liquidation_protocol_fee: liquidation_protocol_fee.to::<u64>() as i64,
        is_siloed_borrowing: siloed_borrowing,
        unbacked_mint_cap: to_bigdecimal(unbacked_mint_cap)?,
        atoken_address: token_addresses.aTokenAddress.to_string(),
        v_debt_token_address: token_addresses.variableDebtTokenAddress.to_string(),
        s_debt_token_address: token_addresses.stableDebtTokenAddress.to_string(),
        interest_rate_strategy_address: pool_data.interestRateStrategyAddress.to_string(),
        last_updated_block: i64::try_from(current_block)
            .map_err(|_| eyre!("block number overflow: {}", current_block))?,
        last_updated_log_index: -1,
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
        last_updated_block: i64::try_from(current_block)
            .map_err(|_| eyre!("block number overflow: {}", current_block))?,
        last_updated_log_index: -1,
    };

    reserves_repository::sync_reserve(pool, reserve_data)
        .await
        .map_err(|e| {
            error!(
                asset = %asset_address,
                error = %e,
                "Failed to sync reserve"
            );
            e
        })?;

    reserve_state_repository::sync_state(pool, reserve_state)
        .await
        .map_err(|e| {
            error!(
                asset = %asset_address,
                error = %e,
                "Failed to sync reserve state"
            );
            e
        })?;

    Ok(())
}
