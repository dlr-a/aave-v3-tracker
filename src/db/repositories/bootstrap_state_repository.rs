use crate::db::models::{BootstrapState, NewBootstrapState};
use crate::db::schema::bootstrap_state::dsl::*;
use diesel::prelude::*;
use diesel::result::OptionalExtension;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use eyre::Result;

const BOOTSTRAP_ID: i32 = 1;

pub async fn get(conn: &mut AsyncPgConnection) -> Result<Option<BootstrapState>> {
    let result = bootstrap_state
        .filter(id.eq(BOOTSTRAP_ID))
        .first::<BootstrapState>(conn)
        .await
        .optional()?;

    Ok(result)
}

pub async fn upsert(conn: &mut AsyncPgConnection, cursor: &str, block: i64) -> Result<()> {
    let new = NewBootstrapState {
        id: BOOTSTRAP_ID,
        last_cursor: cursor.to_string(),
        meta_block: block,
        completed: false,
    };

    diesel::insert_into(bootstrap_state)
        .values(&new)
        .on_conflict(id)
        .do_update()
        .set((last_cursor.eq(cursor), meta_block.eq(block)))
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn mark_completed(conn: &mut AsyncPgConnection) -> Result<()> {
    diesel::update(bootstrap_state.filter(id.eq(BOOTSTRAP_ID)))
        .set(completed.eq(true))
        .execute(conn)
        .await?;
    Ok(())
}
