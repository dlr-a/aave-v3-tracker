use diesel::ExpressionMethods;
use diesel::query_dsl::methods::SelectDsl;
use diesel::result::Error as DieselError;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn get_last_block(conn: &mut AsyncPgConnection) -> Result<i64, DieselError> {
    use crate::db::schema::sync_status::dsl::*;

    let val = sync_status
        .select(last_processed_block)
        .first::<i64>(conn)
        .await?;

    Ok(val)
}

pub async fn update_last_block(
    conn: &mut AsyncPgConnection,
    block: i64,
) -> Result<(), DieselError> {
    use crate::db::schema::sync_status::dsl::*;

    diesel::update(sync_status)
        .set(last_processed_block.eq(block))
        .execute(conn)
        .await?;

    Ok(())
}
