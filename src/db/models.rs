use diesel::prelude::*;

#[derive(Insertable, Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::db::schema::reserves)]
pub struct Reserve {
    pub asset_address: String,
    pub symbol: String,

    pub decimals: i64,
    pub liquidation_threshold: i64,
    pub liquidation_bonus: i64,
    pub ltv: i64,

    pub is_active: bool,
    pub is_frozen: bool,
    pub atoken_address: String,
    pub v_debt_token_address: String,
    pub s_debt_token_address: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::reserves)]
pub struct NewReserve {
    pub asset_address: String,
    pub symbol: String,

    pub decimals: i64,
    pub liquidation_threshold: i64,
    pub liquidation_bonus: i64,
    pub ltv: i64,

    pub is_active: bool,
    pub is_frozen: bool,
    pub atoken_address: String,
    pub v_debt_token_address: String,
    pub s_debt_token_address: String,
}
