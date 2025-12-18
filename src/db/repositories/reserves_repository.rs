use crate::db::connection::DbPool;
use crate::db::models::NewReserve;
use crate::db::schema::reserves::dsl::*;
use crate::errors::TrackerError;
use bigdecimal::BigDecimal;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use eyre::Result;

pub async fn sync_reserve(pool: &DbPool, new_reserve: NewReserve) -> Result<usize, TrackerError> {
    let mut conn = pool.get().await?;

    let result = diesel::insert_into(reserves)
        .values(&new_reserve)
        .on_conflict(asset_address)
        .do_update()
        .set((
            symbol.eq(excluded(symbol)),
            decimals.eq(excluded(decimals)),
            reserve_id.eq(excluded(reserve_id)),
            liquidation_threshold.eq(excluded(liquidation_threshold)),
            ltv.eq(excluded(ltv)),
            liquidation_bonus.eq(excluded(liquidation_bonus)),
            is_active.eq(excluded(is_active)),
            is_frozen.eq(excluded(is_frozen)),
            is_paused.eq(excluded(is_paused)),
            is_borrowing_enabled.eq(excluded(is_borrowing_enabled)),
            is_dropped.eq(excluded(is_dropped)),
            supply_cap.eq(excluded(supply_cap)),
            borrow_cap.eq(excluded(borrow_cap)),
            reserve_factor.eq(excluded(reserve_factor)),
            atoken_address.eq(excluded(atoken_address)),
            v_debt_token_address.eq(excluded(v_debt_token_address)),
            s_debt_token_address.eq(excluded(s_debt_token_address)),
            interest_rate_strategy_address.eq(excluded(interest_rate_strategy_address)),
        ))
        .execute(&mut conn)
        .await?;

    Ok(result)
}

pub async fn update_reserve_factor(
    pool: &DbPool,
    asset: String,
    rsrv_factor: i64,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        reserve_factor.eq(rsrv_factor),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn update_supply_cap(
    pool: &DbPool,
    asset: String,
    sply_cap: BigDecimal,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        supply_cap.eq(sply_cap),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn update_borrow_cap(
    pool: &DbPool,
    asset: String,
    brw_cap: BigDecimal,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        borrow_cap.eq(brw_cap),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn update_stable_borrow_address(
    pool: &DbPool,
    asset: String,
    stable_borrow_address: String,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        s_debt_token_address.eq(stable_borrow_address),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn update_risk_config(
    pool: &DbPool,
    asset: String,
    ltv_val: i64,
    threshold_val: i64,
    bonus_val: i64,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        ltv.eq(ltv_val),
        liquidation_threshold.eq(threshold_val),
        liquidation_bonus.eq(bonus_val),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn set_frozen_status(
    pool: &DbPool,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        is_frozen.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn set_paused_status(
    pool: &DbPool,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        is_paused.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn set_borrowing_status(
    pool: &DbPool,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        is_borrowing_enabled.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn set_active_status(
    pool: &DbPool,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        is_active.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn set_dropped_status(
    pool: &DbPool,
    asset: String,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        is_dropped.eq(true),
        is_active.eq(false),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}

pub async fn update_strategy_address(
    pool: &DbPool,
    asset: String,
    new_strategy: String,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserves
            .filter(asset_address.eq(asset))
            .filter(last_updated_block.lt(block_number))
            .filter(last_updated_log_index.lt(log_index)),
    )
    .set((
        interest_rate_strategy_address.eq(new_strategy),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;

    Ok(result)
}
