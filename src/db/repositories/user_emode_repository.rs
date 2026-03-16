use crate::db::models::{NewUserEmode, UserEmode};
use crate::db::schema::user_emode::dsl::*;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use eyre::Result;

pub async fn upsert(
    conn: &mut AsyncPgConnection,
    user: &str,
    category: i32,
    block: i64,
    log_idx: i64,
) -> Result<()> {
    let new_row = NewUserEmode {
        user_address: user.to_string(),
        emode_category_id: category,
        last_updated_block: block,
        last_updated_log_index: log_idx,
    };

    let inserted = diesel::insert_into(user_emode)
        .values(&new_row)
        .on_conflict(user_address)
        .do_nothing()
        .execute(conn)
        .await?;

    if inserted > 0 {
        return Ok(());
    }

    diesel::update(
        user_emode
            .filter(user_address.eq(user))
            .filter(
                last_updated_block.lt(block).or(last_updated_block
                    .eq(block)
                    .and(last_updated_log_index.lt(log_idx))),
            ),
    )
    .set((
        emode_category_id.eq(category),
        last_updated_block.eq(block),
        last_updated_log_index.eq(log_idx),
    ))
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn get(
    conn: &mut AsyncPgConnection,
    user: &str,
) -> Result<Option<UserEmode>> {
    let result = user_emode
        .filter(user_address.eq(user))
        .first::<UserEmode>(conn)
        .await
        .optional()?;
    Ok(result)
}

pub async fn get_all_with_emode(conn: &mut AsyncPgConnection) -> Result<Vec<UserEmode>> {
    let result = user_emode
        .filter(emode_category_id.gt(0))
        .load::<UserEmode>(conn)
        .await?;
    Ok(result)
}
