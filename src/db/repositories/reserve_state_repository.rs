use crate::db::connection::DbPool;
use crate::db::models::NewReserveState;
use crate::db::schema::reserve_state::dsl::*;
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use eyre::Result;

pub async fn sync_state(pool: &DbPool, new_state: NewReserveState) -> Result<()> {
    let mut conn = pool.get().await?;

    diesel::insert_into(reserve_state)
        .values(&new_state)
        .on_conflict(asset_address)
        .do_update()
        .set((
            liquidity_index.eq(excluded(liquidity_index)),
            variable_borrow_index.eq(excluded(variable_borrow_index)),
            current_liquidity_rate.eq(excluded(current_liquidity_rate)),
            current_variable_borrow_rate.eq(excluded(current_variable_borrow_rate)),
            current_stable_borrow_rate.eq(excluded(current_stable_borrow_rate)),
            total_liquidity.eq(excluded(total_liquidity)),
            total_variable_debt.eq(excluded(total_variable_debt)),
            total_stable_debt.eq(excluded(total_stable_debt)),
            accrued_to_treasury.eq(excluded(accrued_to_treasury)),
            unbacked.eq(excluded(unbacked)),
            isolation_mode_total_debt.eq(excluded(isolation_mode_total_debt)),
        ))
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub async fn update_financials(
    pool: &DbPool,
    asset: String,
    liq_index_val: BigDecimal,
    var_borrow_index_val: BigDecimal,
    liq_rate_val: BigDecimal,
    var_borrow_rate_val: BigDecimal,
    stable_borrow_rate_val: BigDecimal,
    block_number: i64,
    log_index: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = pool.get().await?;

    let result = diesel::update(
        reserve_state.filter(asset_address.eq(asset)).filter(
            last_updated_block.lt(block_number).or(last_updated_block
                .eq(block_number)
                .and(last_updated_log_index.lt(log_index))),
        ),
    )
    .set((
        liquidity_index.eq(liq_index_val),
        variable_borrow_index.eq(var_borrow_index_val),
        current_liquidity_rate.eq(liq_rate_val),
        current_variable_borrow_rate.eq(var_borrow_rate_val),
        current_stable_borrow_rate.eq(stable_borrow_rate_val),
        last_updated_block.eq(block_number),
        last_updated_log_index.eq(log_index),
    ))
    .execute(&mut conn)
    .await?;
    Ok(result)
}
