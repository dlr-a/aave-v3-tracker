use crate::db::models::NewUserPosition;
use crate::db::schema::user_positions::dsl::*;
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use eyre::Result;

pub async fn upsert_supply(
    conn: &mut AsyncPgConnection,
    user: &str,
    asset: &str,
    delta: BigDecimal,
    index: BigDecimal,
    block: i64,
    log_idx: i64,
) -> Result<usize> {
    let delta_for_update = delta.clone();
    let index_for_update = index.clone();

    let new_pos = NewUserPosition {
        user_address: user.to_string(),
        asset_address: asset.to_string(),
        scaled_atoken_balance: delta,
        scaled_variable_debt: BigDecimal::from(0),
        use_as_collateral: false,
        atoken_last_index: index,
        debt_last_index: BigDecimal::from(0),
        last_updated_block: block,
        last_updated_log_index: log_idx,
        is_active: true,
        created_at_block: block,
    };

    let inserted = diesel::insert_into(user_positions)
        .values(&new_pos)
        .on_conflict((user_address, asset_address))
        .do_nothing()
        .execute(conn)
        .await?;

    if inserted > 0 {
        return Ok(inserted);
    }

    let updated = diesel::update(
        user_positions
            .filter(user_address.eq(user))
            .filter(asset_address.eq(asset))
            .filter(
                last_updated_block.lt(block).or(last_updated_block
                    .eq(block)
                    .and(last_updated_log_index.lt(log_idx))),
            ),
    )
    .set((
        scaled_atoken_balance.eq(scaled_atoken_balance + delta_for_update),
        atoken_last_index.eq(index_for_update),
        is_active.eq(true),
        last_updated_block.eq(block),
        last_updated_log_index.eq(log_idx),
    ))
    .execute(conn)
    .await?;

    Ok(updated)
}

pub async fn decrease_supply(
    conn: &mut AsyncPgConnection,
    user: &str,
    asset: &str,
    amount: BigDecimal,
    index: BigDecimal,
    block: i64,
    log_idx: i64,
) -> Result<usize> {
    let result = diesel::update(
        user_positions
            .filter(user_address.eq(user))
            .filter(asset_address.eq(asset))
            .filter(
                last_updated_block.lt(block).or(last_updated_block
                    .eq(block)
                    .and(last_updated_log_index.lt(log_idx))),
            ),
    )
    .set((
        scaled_atoken_balance.eq(scaled_atoken_balance - amount),
        atoken_last_index.eq(index),
        last_updated_block.eq(block),
        last_updated_log_index.eq(log_idx),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn decrease_supply_no_index(
    conn: &mut AsyncPgConnection,
    user: &str,
    asset: &str,
    amount: BigDecimal,
    block: i64,
    log_idx: i64,
) -> Result<usize> {
    let result = diesel::update(
        user_positions
            .filter(user_address.eq(user))
            .filter(asset_address.eq(asset))
            .filter(
                last_updated_block.lt(block).or(last_updated_block
                    .eq(block)
                    .and(last_updated_log_index.lt(log_idx))),
            ),
    )
    .set((
        scaled_atoken_balance.eq(scaled_atoken_balance - amount),
        last_updated_block.eq(block),
        last_updated_log_index.eq(log_idx),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn clear_collateral_if_zero(
    conn: &mut AsyncPgConnection,
    user: &str,
    asset: &str,
) -> Result<usize> {
    diesel::update(
        user_positions
            .filter(user_address.eq(user))
            .filter(asset_address.eq(asset))
            .filter(scaled_atoken_balance.le(BigDecimal::from(0))),
    )
    .set(use_as_collateral.eq(false))
    .execute(conn)
    .await?;

    let result = diesel::update(
        user_positions
            .filter(user_address.eq(user))
            .filter(asset_address.eq(asset))
            .filter(scaled_atoken_balance.le(BigDecimal::from(0)))
            .filter(scaled_variable_debt.le(BigDecimal::from(0))),
    )
    .set(is_active.eq(false))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn upsert_debt(
    conn: &mut AsyncPgConnection,
    user: &str,
    asset: &str,
    delta: BigDecimal,
    index: BigDecimal,
    block: i64,
    log_idx: i64,
) -> Result<usize> {
    let delta_for_update = delta.clone();
    let index_for_update = index.clone();

    let new_pos = NewUserPosition {
        user_address: user.to_string(),
        asset_address: asset.to_string(),
        scaled_atoken_balance: BigDecimal::from(0),
        scaled_variable_debt: delta,
        use_as_collateral: false,
        atoken_last_index: BigDecimal::from(0),
        debt_last_index: index,
        last_updated_block: block,
        last_updated_log_index: log_idx,
        is_active: true,
        created_at_block: block,
    };

    let inserted = diesel::insert_into(user_positions)
        .values(&new_pos)
        .on_conflict((user_address, asset_address))
        .do_nothing()
        .execute(conn)
        .await?;

    if inserted > 0 {
        return Ok(inserted);
    }

    let updated = diesel::update(
        user_positions
            .filter(user_address.eq(user))
            .filter(asset_address.eq(asset))
            .filter(
                last_updated_block.lt(block).or(last_updated_block
                    .eq(block)
                    .and(last_updated_log_index.lt(log_idx))),
            ),
    )
    .set((
        scaled_variable_debt.eq(scaled_variable_debt + delta_for_update),
        debt_last_index.eq(index_for_update),
        is_active.eq(true),
        last_updated_block.eq(block),
        last_updated_log_index.eq(log_idx),
    ))
    .execute(conn)
    .await?;

    Ok(updated)
}

pub async fn batch_insert(
    conn: &mut AsyncPgConnection,
    positions: &[NewUserPosition],
) -> Result<usize> {
    let result = diesel::insert_into(user_positions)
        .values(positions)
        .on_conflict((user_address, asset_address))
        .do_nothing()
        .execute(conn)
        .await?;

    Ok(result)
}

pub async fn decrease_debt(
    conn: &mut AsyncPgConnection,
    user: &str,
    asset: &str,
    amount: BigDecimal,
    index: BigDecimal,
    block: i64,
    log_idx: i64,
) -> Result<usize> {
    let result = diesel::update(
        user_positions
            .filter(user_address.eq(user))
            .filter(asset_address.eq(asset))
            .filter(
                last_updated_block.lt(block).or(last_updated_block
                    .eq(block)
                    .and(last_updated_log_index.lt(log_idx))),
            ),
    )
    .set((
        scaled_variable_debt.eq(scaled_variable_debt - amount),
        debt_last_index.eq(index),
        last_updated_block.eq(block),
        last_updated_log_index.eq(log_idx),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn set_collateral(
    conn: &mut AsyncPgConnection,
    user: &str,
    asset: &str,
    enabled: bool,
    block: i64,
    log_idx: i64,
) -> Result<usize> {
    let result = diesel::update(
        user_positions
            .filter(user_address.eq(user))
            .filter(asset_address.eq(asset))
            .filter(
                last_updated_block.lt(block).or(last_updated_block
                    .eq(block)
                    .and(last_updated_log_index.lt(log_idx))),
            ),
    )
    .set((
        use_as_collateral.eq(enabled),
        last_updated_block.eq(block),
        last_updated_log_index.eq(log_idx),
    ))
    .execute(conn)
    .await?;

    Ok(result)
}

pub async fn clear_inactive_if_zero(
    conn: &mut AsyncPgConnection,
    user: &str,
    asset: &str,
) -> Result<usize> {
    let result = diesel::update(
        user_positions
            .filter(user_address.eq(user))
            .filter(asset_address.eq(asset))
            .filter(scaled_atoken_balance.le(BigDecimal::from(0)))
            .filter(scaled_variable_debt.le(BigDecimal::from(0))),
    )
    .set(is_active.eq(false))
    .execute(conn)
    .await?;

    Ok(result)
}
