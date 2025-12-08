use crate::db::schema::reserve_state;
use bigdecimal::BigDecimal;
use diesel::prelude::*;

#[derive(Insertable, Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::db::schema::reserves)]
pub struct Reserve {
    pub asset_address: String,
    pub symbol: String,

    pub decimals: i64,
    pub reserve_id: i32,
    pub liquidation_threshold: i64,
    pub liquidation_bonus: i64,
    pub ltv: i64,

    pub is_active: bool,
    pub is_frozen: bool,
    pub is_paused: bool,
    pub is_borrowing_enabled: bool,
    pub is_dropped: bool,

    pub atoken_address: String,
    pub v_debt_token_address: String,
    pub s_debt_token_address: String,
    pub interest_rate_strategy_address: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::reserves)]
pub struct NewReserve {
    pub asset_address: String,
    pub symbol: String,

    pub decimals: i64,
    pub reserve_id: i32,
    pub liquidation_threshold: i64,
    pub liquidation_bonus: i64,
    pub ltv: i64,

    pub is_active: bool,
    pub is_frozen: bool,
    pub is_paused: bool,
    pub is_borrowing_enabled: bool,
    pub is_dropped: bool,

    pub atoken_address: String,
    pub v_debt_token_address: String,
    pub s_debt_token_address: String,
    pub interest_rate_strategy_address: String,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = reserve_state)]
#[diesel(primary_key(asset_address))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ReserveState {
    pub asset_address: String,

    pub liquidity_index: BigDecimal,
    pub variable_borrow_index: BigDecimal,
    pub current_liquidity_rate: BigDecimal,
    pub current_variable_borrow_rate: BigDecimal,
    pub current_stable_borrow_rate: BigDecimal,

    pub total_liquidity: BigDecimal,
    pub total_variable_debt: BigDecimal,
    pub total_stable_debt: BigDecimal,

    pub accrued_to_treasury: BigDecimal,
    pub unbacked: BigDecimal,
    pub isolation_mode_total_debt: BigDecimal,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = reserve_state)]
pub struct NewReserveState {
    pub asset_address: String,

    pub liquidity_index: BigDecimal,
    pub variable_borrow_index: BigDecimal,
    pub current_liquidity_rate: BigDecimal,
    pub current_variable_borrow_rate: BigDecimal,
    pub current_stable_borrow_rate: BigDecimal,

    pub total_liquidity: BigDecimal,
    pub total_variable_debt: BigDecimal,
    pub total_stable_debt: BigDecimal,

    pub accrued_to_treasury: BigDecimal,
    pub unbacked: BigDecimal,
    pub isolation_mode_total_debt: BigDecimal,
}
