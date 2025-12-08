use crate::db::connection::DbPool;
use crate::db::models::NewReserveState;
use crate::db::schema::reserve_state::dsl::*;
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
