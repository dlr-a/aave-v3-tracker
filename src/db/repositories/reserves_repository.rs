use crate::db::connection::DbPool;
use crate::db::models::NewReserve;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("Failed to connect DB")]
    DBConnectionFailed,
}

#[derive(Debug, Error)]
pub enum InsertError {
    #[error("Failed to insert reserve")]
    InsertReserveFailed,
}

pub async fn sync_reserve(
    pool: &DbPool,
    new_reserve: NewReserve,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    use crate::db::schema::reserves::dsl::*;

    let mut conn = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                "Error: Failed to get database connection from pool: {:?}",
                e
            );
            return Err(ConnectionError::DBConnectionFailed.into());
        }
    };

    let result = match diesel::insert_into(reserves)
        .values(&new_reserve)
        .on_conflict(asset_address)
        .do_update()
        .set((
            symbol.eq(excluded(symbol)),
            decimals.eq(excluded(decimals)),
            liquidation_threshold.eq(excluded(liquidation_threshold)),
            ltv.eq(excluded(ltv)),
            liquidation_bonus.eq(excluded(liquidation_bonus)),
            is_active.eq(excluded(is_active)),
            is_frozen.eq(excluded(is_frozen)),
            atoken_address.eq(excluded(atoken_address)),
            v_debt_token_address.eq(excluded(v_debt_token_address)),
            s_debt_token_address.eq(excluded(s_debt_token_address)),
        ))
        .execute(&mut conn)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Error: Failed to get insert reserve: {:?}", e);
            return Err(InsertError::InsertReserveFailed.into());
        }
    };

    Ok(result)
}
