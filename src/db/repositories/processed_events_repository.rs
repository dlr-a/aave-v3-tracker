use diesel::ExpressionMethods;
use diesel::result::Error as DieselError;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;

pub async fn try_insert_event(
    conn: &mut AsyncPgConnection,
    tx_hash_val: String,
    log_index_val: i64,
    block_number_val: i64,
) -> Result<bool, DieselError> {
    use crate::db::schema::processed_events::dsl::*;

    let inserted = diesel::insert_into(processed_events)
        .values((
            tx_hash.eq(tx_hash_val),
            log_index.eq(log_index_val),
            block_number.eq(block_number_val),
        ))
        .on_conflict_do_nothing()
        .execute(conn)
        .await?;

    Ok(inserted == 1)
}
