use crate::db::models::{EmodeCategory, NewEmodeCategory};
use crate::db::schema::emode_categories::dsl::*;
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use eyre::Result;
use std::collections::HashMap;

pub async fn upsert(conn: &mut AsyncPgConnection, category: NewEmodeCategory) -> Result<()> {
    diesel::insert_into(emode_categories)
        .values(&category)
        .on_conflict(category_id)
        .do_update()
        .set(&category)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get(
    conn: &mut AsyncPgConnection,
    id: i32,
) -> Result<Option<EmodeCategory>> {
    let result = emode_categories
        .filter(category_id.eq(id))
        .first::<EmodeCategory>(conn)
        .await
        .optional()?;
    Ok(result)
}

pub async fn get_all_as_map(
    conn: &mut AsyncPgConnection,
) -> Result<HashMap<i32, EmodeCategory>> {
    let rows = emode_categories.load::<EmodeCategory>(conn).await?;
    Ok(rows.into_iter().map(|c| (c.category_id, c)).collect())
}

pub async fn update_collateral_bitmap(
    conn: &mut AsyncPgConnection,
    cat_id: i32,
    bitmap: BigDecimal,
) -> Result<()> {
    diesel::update(emode_categories.filter(category_id.eq(cat_id)))
        .set(collateral_bitmap.eq(bitmap))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn update_borrowable_bitmap(
    conn: &mut AsyncPgConnection,
    cat_id: i32,
    bitmap: BigDecimal,
) -> Result<()> {
    diesel::update(emode_categories.filter(category_id.eq(cat_id)))
        .set(borrowable_bitmap.eq(bitmap))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn update_ltvzero_bitmap(
    conn: &mut AsyncPgConnection,
    cat_id: i32,
    bitmap: BigDecimal,
) -> Result<()> {
    diesel::update(emode_categories.filter(category_id.eq(cat_id)))
        .set(ltvzero_bitmap.eq(bitmap))
        .execute(conn)
        .await?;
    Ok(())
}
