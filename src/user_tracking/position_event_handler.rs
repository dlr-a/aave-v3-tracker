use crate::abi::{
    BalanceTransfer, Burn, Mint, ReserveUsedAsCollateralDisabled, ReserveUsedAsCollateralEnabled,
};
use crate::db::repositories::reserves_repository;
use crate::db::repositories::reserves_repository::TokenType;
use crate::db::repositories::{processed_events_repository, user_positions_repository};
use alloy::primitives::{U256, uint};
use alloy::rpc::types::eth::Log;
use alloy_sol_types::SolEvent;
use bigdecimal::BigDecimal;
use diesel_async::AsyncPgConnection;
use eyre::{Context, Result, anyhow};
use std::str::FromStr;
use tracing::info;

// 10^27
const RAY: U256 = uint!(1000000000000000000000000000_U256);

fn u256_to_bigdecimal(val: U256) -> BigDecimal {
    BigDecimal::from_str(&val.to_string()).expect("U256 always produces a valid decimal string")
}

/// Solidity WadRayMath.rayDiv: (amount * RAY + index/2) / index
fn ray_div(amount: U256, index: U256) -> U256 {
    let half_index = index / U256::from(2);
    (amount * RAY + half_index) / index
}

// Mint event result: normal mint increases, burn-triggered mint decreases.
pub enum ScaledDelta {
    Increase(U256),
    Decrease(U256),
}

/// Notes from ScaledBalanceTokenBase.sol: https://github.com/aave/aave-v3-core/blob/master/contracts/protocol/tokenization/base/ScaledBalanceTokenBase.sol
///
/// Mint:
/// - amount is converted to scaled via amount.rayDiv(index)
/// - Event emits underlying value = amount + balanceIncrease
/// - Real scaled delta comes from (value - balanceIncrease).rayDiv(index)
///
/// Burn:
/// - amount is converted to scaled via amount.rayDiv(index)
/// - If interest (balanceIncrease) > amount, a Mint event is emitted (net decrease)
/// - Otherwise a Burn event is emitted
/// - Real scaled decrease comes from (value + balanceIncrease).rayDiv(index)
///
/// Transfer:
/// - Transfer works directly with scaled amount (amount.rayDiv(index))
/// - BalanceTransfer.value is already scaled

pub fn compute_mint_scaled_delta(value: U256, balance_increase: U256, index: U256) -> ScaledDelta {
    if value >= balance_increase {
        // Normal mint: net underlying = value - balanceIncrease
        ScaledDelta::Increase(ray_div(value - balance_increase, index))
    } else {
        // Burn-triggered mint: actual burned underlying = balanceIncrease - value
        ScaledDelta::Decrease(ray_div(balance_increase - value, index))
    }
}

pub fn compute_burn_scaled_delta(value: U256, balance_increase: U256, index: U256) -> U256 {
    // Burn.value = amount - balanceIncrease → actual underlying = value + balanceIncrease
    ray_div(value + balance_increase, index)
}

pub async fn process_collateral_event(conn: &mut AsyncPgConnection, log: &Log) -> Result<()> {
    let block_number = log
        .block_number
        .ok_or_else(|| anyhow!("missing block_number in log"))? as i64;

    let log_index = log
        .log_index
        .ok_or_else(|| anyhow!("missing log_index in log"))? as i64;

    let tx_hash = log
        .transaction_hash
        .ok_or_else(|| anyhow!("missing tx_hash in log"))?
        .to_string();

    let inserted =
        processed_events_repository::try_insert_event(conn, tx_hash, log_index, block_number)
            .await?;
    if !inserted {
        return Ok(());
    }

    let topic0 = match log.topics().first() {
        Some(t) => *t,
        None => return Ok(()),
    };

    let enabled = topic0 == ReserveUsedAsCollateralEnabled::SIGNATURE_HASH;

    let (reserve, user) = if enabled {
        let decoded = log
            .log_decode::<ReserveUsedAsCollateralEnabled>()
            .wrap_err("Failed to decode ReserveUsedAsCollateralEnabled")?;
        (
            decoded.data().reserve.to_string(),
            decoded.data().user.to_string(),
        )
    } else {
        let decoded = log
            .log_decode::<ReserveUsedAsCollateralDisabled>()
            .wrap_err("Failed to decode ReserveUsedAsCollateralDisabled")?;
        (
            decoded.data().reserve.to_string(),
            decoded.data().user.to_string(),
        )
    };

    user_positions_repository::set_collateral(
        conn,
        &user,
        &reserve,
        enabled,
        block_number,
        log_index,
    )
    .await
    .wrap_err("Failed to set collateral flag")?;

    info!(user = %user, asset = %reserve, enabled, "Collateral toggle processed");
    Ok(())
}

pub async fn process_token_event(conn: &mut AsyncPgConnection, log: &Log) -> Result<()> {
    let block_number = log
        .block_number
        .ok_or_else(|| anyhow!("missing block_number in log"))? as i64;

    let log_index = log
        .log_index
        .ok_or_else(|| anyhow!("missing log_index in log"))? as i64;

    let tx_hash = log
        .transaction_hash
        .ok_or_else(|| anyhow!("missing tx_hash in log"))?
        .to_string();

    let inserted =
        processed_events_repository::try_insert_event(conn, tx_hash, log_index, block_number)
            .await?;
    if !inserted {
        return Ok(());
    }

    let token_map = reserves_repository::get_token_address_map(conn).await?;

    let emitter = log.address();
    let (asset_addr, token_type) = match token_map.get(&emitter) {
        Some(v) => v,
        None => return Ok(()),
    };

    let topic0 = match log.topics().first() {
        Some(t) => *t,
        None => return Ok(()),
    };

    match token_type {
        TokenType::AToken => {
            if topic0 == Mint::SIGNATURE_HASH {
                handle_atoken_mint(conn, log, asset_addr, block_number, log_index).await?;
            } else if topic0 == Burn::SIGNATURE_HASH {
                handle_atoken_burn(conn, log, asset_addr, block_number, log_index).await?;
            } else if topic0 == BalanceTransfer::SIGNATURE_HASH {
                handle_balance_transfer(conn, log, asset_addr, block_number, log_index).await?;
            }
        }
        TokenType::VariableDebtToken => {
            if topic0 == Mint::SIGNATURE_HASH {
                handle_debt_mint(conn, log, asset_addr, block_number, log_index).await?;
            } else if topic0 == Burn::SIGNATURE_HASH {
                handle_debt_burn(conn, log, asset_addr, block_number, log_index).await?;
            }
        }
    }

    Ok(())
}

async fn handle_atoken_mint(
    conn: &mut AsyncPgConnection,
    log: &Log,
    asset_addr: &str,
    block: i64,
    log_idx: i64,
) -> Result<()> {
    let decoded = log
        .log_decode::<Mint>()
        .wrap_err("Failed to decode aToken Mint")?;
    let e = decoded.data();

    let user = e.onBehalfOf.to_string();

    match compute_mint_scaled_delta(e.value, e.balanceIncrease, e.index) {
        ScaledDelta::Increase(delta) => {
            user_positions_repository::upsert_supply(
                conn,
                &user,
                asset_addr,
                u256_to_bigdecimal(delta),
                u256_to_bigdecimal(e.index),
                block,
                log_idx,
            )
            .await
            .wrap_err("Failed to upsert supply on aToken Mint")?;
        }
        ScaledDelta::Decrease(delta) => {
            user_positions_repository::decrease_supply(
                conn,
                &user,
                asset_addr,
                u256_to_bigdecimal(delta),
                u256_to_bigdecimal(e.index),
                block,
                log_idx,
            )
            .await
            .wrap_err("Failed to decrease supply on burn-triggered aToken Mint")?;
        }
    }

    info!(user = %user, asset = %asset_addr, "aToken Mint processed");
    Ok(())
}

async fn handle_atoken_burn(
    conn: &mut AsyncPgConnection,
    log: &Log,
    asset_addr: &str,
    block: i64,
    log_idx: i64,
) -> Result<()> {
    let decoded = log
        .log_decode::<Burn>()
        .wrap_err("Failed to decode aToken Burn")?;
    let e = decoded.data();

    let user = e.from.to_string();
    let scaled_delta = compute_burn_scaled_delta(e.value, e.balanceIncrease, e.index);

    user_positions_repository::decrease_supply(
        conn,
        &user,
        asset_addr,
        u256_to_bigdecimal(scaled_delta),
        u256_to_bigdecimal(e.index),
        block,
        log_idx,
    )
    .await
    .wrap_err("Failed to decrease supply on aToken Burn")?;

    user_positions_repository::clear_collateral_if_zero(conn, &user, asset_addr)
        .await
        .wrap_err("Failed to clear collateral after aToken Burn")?;

    info!(user = %user, asset = %asset_addr, "aToken Burn processed");
    Ok(())
}

async fn handle_balance_transfer(
    conn: &mut AsyncPgConnection,
    log: &Log,
    asset_addr: &str,
    block: i64,
    log_idx: i64,
) -> Result<()> {
    let decoded = log
        .log_decode::<BalanceTransfer>()
        .wrap_err("Failed to decode BalanceTransfer")?;
    let e = decoded.data();

    let sender = e.from.to_string();
    let receiver = e.to.to_string();
    // BalanceTransfer.value is already scaled
    let scaled_delta = u256_to_bigdecimal(e.value);

    user_positions_repository::decrease_supply_no_index(
        conn,
        &sender,
        asset_addr,
        scaled_delta.clone(),
        block,
        log_idx,
    )
    .await
    .wrap_err("Failed to decrease sender on BalanceTransfer")?;

    user_positions_repository::clear_collateral_if_zero(conn, &sender, asset_addr)
        .await
        .wrap_err("Failed to clear collateral for sender")?;

    user_positions_repository::upsert_supply(
        conn,
        &receiver,
        asset_addr,
        scaled_delta,
        u256_to_bigdecimal(e.index),
        block,
        log_idx,
    )
    .await
    .wrap_err("Failed to upsert receiver on BalanceTransfer")?;

    info!(sender = %sender, receiver = %receiver, asset = %asset_addr, "BalanceTransfer processed");
    Ok(())
}

async fn handle_debt_mint(
    conn: &mut AsyncPgConnection,
    log: &Log,
    asset_addr: &str,
    block: i64,
    log_idx: i64,
) -> Result<()> {
    let decoded = log
        .log_decode::<Mint>()
        .wrap_err("Failed to decode vDebtToken Mint")?;
    let e = decoded.data();

    let user = e.onBehalfOf.to_string();

    match compute_mint_scaled_delta(e.value, e.balanceIncrease, e.index) {
        ScaledDelta::Increase(delta) => {
            user_positions_repository::upsert_debt(
                conn,
                &user,
                asset_addr,
                u256_to_bigdecimal(delta),
                u256_to_bigdecimal(e.index),
                block,
                log_idx,
            )
            .await
            .wrap_err("Failed to upsert debt on vDebtToken Mint")?;
        }
        ScaledDelta::Decrease(delta) => {
            user_positions_repository::decrease_debt(
                conn,
                &user,
                asset_addr,
                u256_to_bigdecimal(delta),
                u256_to_bigdecimal(e.index),
                block,
                log_idx,
            )
            .await
            .wrap_err("Failed to decrease debt on burn-triggered vDebtToken Mint")?;
        }
    }

    info!(user = %user, asset = %asset_addr, "vDebtToken Mint processed");
    Ok(())
}

async fn handle_debt_burn(
    conn: &mut AsyncPgConnection,
    log: &Log,
    asset_addr: &str,
    block: i64,
    log_idx: i64,
) -> Result<()> {
    let decoded = log
        .log_decode::<Burn>()
        .wrap_err("Failed to decode vDebtToken Burn")?;
    let e = decoded.data();

    let user = e.from.to_string();
    let scaled_delta = compute_burn_scaled_delta(e.value, e.balanceIncrease, e.index);

    user_positions_repository::decrease_debt(
        conn,
        &user,
        asset_addr,
        u256_to_bigdecimal(scaled_delta),
        u256_to_bigdecimal(e.index),
        block,
        log_idx,
    )
    .await
    .wrap_err("Failed to decrease debt on vDebtToken Burn")?;

    user_positions_repository::clear_inactive_if_zero(conn, &user, asset_addr)
        .await
        .wrap_err("Failed to clear inactive after vDebtToken Burn")?;

    info!(user = %user, asset = %asset_addr, "vDebtToken Burn processed");
    Ok(())
}
