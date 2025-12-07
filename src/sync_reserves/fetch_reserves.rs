use crate::db::connection::DbPool;
use crate::db::models::NewReserve;
use crate::db::repositories::reserves_repository;
use alloy::primitives::address;
use alloy::{
    providers::{ProviderBuilder, WsConnect},
    sol,
};
use eyre::{Context, Result};

sol! {
    #[sol(rpc)]
    interface IProtocolDataProvider {
        #[derive(Debug)]
        struct TokenData { string symbol; address tokenAddress; }
        function getAllReservesTokens() external view returns (TokenData[] memory);
        function getReserveTokensAddresses(address asset) external view override returns (address aTokenAddress, address stableDebtTokenAddress, address variableDebtTokenAddress);
        function getReserveConfigurationData(address asset) external view override returns (uint256 decimals, uint256 ltv, uint256 liquidationThreshold, uint256 liquidationBonus, uint256 reserveFactor, bool usageAsCollateralEnabled, bool borrowingEnabled, bool stableBorrowRateEnabled, bool isActive, bool isFrozen);
    }
    #[sol(rpc)]
    interface IERC20 {
        function symbol() external view returns (string);
    }
}

pub async fn fetch_reserves(pool: &DbPool, rpc_url: String) -> Result<()> {
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new()
        .connect_ws(ws)
        .await
        .wrap_err("Couldn't connect to the WS")?;

    let data_provider_addr = address!("0x0a16f2FCC0D44FaE41cc54e079281D84A363bECD");
    let protocol_data_provider = IProtocolDataProvider::new(data_provider_addr, provider.clone());

    let tokens = protocol_data_provider
        .getAllReservesTokens()
        .call()
        .await
        .wrap_err("Failed to fetch all reserve tokens from ProtocolDataProvider")?;

    for token in tokens {
        let token_addresses = protocol_data_provider
            .getReserveTokensAddresses(token.tokenAddress)
            .call()
            .await
            .wrap_err_with(|| {
                format!(
                    "RPC Error: Failed to fetch token addresses for assset: {:?}",
                    token.tokenAddress
                )
            })?;

        let reserve = protocol_data_provider
            .getReserveConfigurationData(token.tokenAddress)
            .call()
            .await
            .wrap_err_with(|| {
                format!(
                    "RPC Error: Failed to fetch reserve configuration data. Reserve: {:?}",
                    token.tokenAddress
                )
            })?;

        let contract = IERC20::new(token.tokenAddress, &provider);
        let symbol = match contract.symbol().call().await {
            Ok(s) => s,
            Err(_) => "UNKNOWN".to_string(),
        };

        let ltv = reserve.ltv.to::<u64>() as i64;
        let liquidation_threshold = reserve.liquidationThreshold.to::<u64>() as i64;
        let liquidation_bonus = reserve.liquidationBonus.to::<u64>() as i64;

        let reserve_data = NewReserve {
            asset_address: token.tokenAddress.to_string(),
            symbol: symbol.clone(),
            decimals: reserve.decimals.to::<u64>() as i64,
            liquidation_threshold,
            ltv,
            liquidation_bonus: liquidation_bonus,
            is_active: reserve.isActive,
            is_frozen: reserve.isFrozen,
            atoken_address: token_addresses.aTokenAddress.to_string(),
            v_debt_token_address: token_addresses.variableDebtTokenAddress.to_string(),
            s_debt_token_address: token_addresses.stableDebtTokenAddress.to_string(),
        };

        reserves_repository::sync_reserve(pool, reserve_data)
            .await
            .wrap_err_with(|| format!("Failed to sync reserve. Asset: {}", symbol))?;
    }
    Ok(())
}
