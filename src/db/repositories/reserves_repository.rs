use crate::db::connection::DbPool;
use crate::db::models::NewReserve;
use crate::db::schema::reserves::dsl::*;
use crate::errors::TrackerError;
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
            atoken_address.eq(excluded(atoken_address)),
            v_debt_token_address.eq(excluded(v_debt_token_address)),
            s_debt_token_address.eq(excluded(s_debt_token_address)),
            interest_rate_strategy_address.eq(excluded(interest_rate_strategy_address)),
        ))
        .execute(&mut conn)
        .await?;

    Ok(result)
}
