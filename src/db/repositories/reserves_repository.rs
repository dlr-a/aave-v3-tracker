use crate::db::connection::DbPool;
use crate::db::models::NewReserve;
use crate::db::schema::reserves::dsl::*;
use crate::errors::TrackerError;
use bigdecimal::BigDecimal;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
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
            is_collateral_enabled.eq(excluded(is_collateral_enabled)),
            is_stable_borrow_enabled.eq(excluded(is_stable_borrow_enabled)),
            is_flash_loan_enabled.eq(excluded(is_flash_loan_enabled)),
            emode_category_id.eq(excluded(emode_category_id)),
            debt_ceiling.eq(excluded(debt_ceiling)),
            liquidation_protocol_fee.eq(excluded(liquidation_protocol_fee)),
            is_siloed_borrowing.eq(excluded(is_siloed_borrowing)),
            unbacked_mint_cap.eq(excluded(unbacked_mint_cap)),
        ))
        .execute(&mut conn)
        .await?;

    Ok(result)
}

pub async fn update_reserve_factor(
    conn: &mut AsyncPgConnection,
    asset: String,
    rsrv_factor: i64,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        reserve_factor.eq(rsrv_factor),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn update_supply_cap(
    conn: &mut AsyncPgConnection,
    asset: String,
    sply_cap: BigDecimal,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        supply_cap.eq(sply_cap),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn update_borrow_cap(
    conn: &mut AsyncPgConnection,
    asset: String,
    brw_cap: BigDecimal,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        borrow_cap.eq(brw_cap),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn update_stable_borrow_address(
    conn: &mut AsyncPgConnection,
    asset: String,
    stable_borrow_address: String,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        s_debt_token_address.eq(stable_borrow_address),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn update_risk_config(
    conn: &mut AsyncPgConnection,
    asset: String,
    ltv_val: i64,
    threshold_val: i64,
    bonus_val: i64,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        ltv.eq(ltv_val),
        liquidation_threshold.eq(threshold_val),
        liquidation_bonus.eq(bonus_val),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn set_frozen_status(
    conn: &mut AsyncPgConnection,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        is_frozen.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn set_paused_status(
    conn: &mut AsyncPgConnection,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        is_paused.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn set_borrowing_status(
    conn: &mut AsyncPgConnection,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        is_borrowing_enabled.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn set_active_status(
    conn: &mut AsyncPgConnection,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        is_active.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn set_dropped_status(
    conn: &mut AsyncPgConnection,
    asset: String,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        is_dropped.eq(true),
        is_active.eq(false),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn update_strategy_address(
    conn: &mut AsyncPgConnection,
    asset: String,
    new_strategy: String,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        interest_rate_strategy_address.eq(new_strategy),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn set_flash_loan_status(
    conn: &mut AsyncPgConnection,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        is_flash_loan_enabled.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn update_emode_category(
    conn: &mut AsyncPgConnection,
    asset: String,
    category_id: i32,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        emode_category_id.eq(category_id),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn update_debt_ceiling(
    conn: &mut AsyncPgConnection,
    asset: String,
    ceiling: BigDecimal,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        debt_ceiling.eq(ceiling),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn update_liquidation_protocol_fee(
    conn: &mut AsyncPgConnection,
    asset: String,
    fee: i64,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        liquidation_protocol_fee.eq(fee),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn set_siloed_borrowing_status(
    conn: &mut AsyncPgConnection,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        is_siloed_borrowing.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn update_unbacked_mint_cap(
    conn: &mut AsyncPgConnection,
    asset: String,
    cap: BigDecimal,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        unbacked_mint_cap.eq(cap),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn set_stable_borrow_status(
    conn: &mut AsyncPgConnection,
    asset: String,
    status: bool,
    block_number: i64,
    log_index: i64,
) -> Result<usize> {
    let result = diesel::update(
        reserves.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        is_stable_borrow_enabled.eq(status),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn reserve_exists(conn: &mut AsyncPgConnection, asset: String) -> Result<bool> {
    use diesel::dsl::count;

    let result: i64 = reserves
        .filter(asset_address.eq(asset))
        .select(count(asset_address))
        .first(conn)
        .await?;

    Ok(result > 0)
}
