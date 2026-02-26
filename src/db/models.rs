use crate::db::schema::processed_events;
use crate::db::schema::reserve_state;
use crate::db::schema::sync_status;
use crate::db::schema::user_emode;
use crate::db::schema::user_positions;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Insertable, Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::db::schema::reserves)]
pub struct Reserve {
    pub asset_address: String,
    pub symbol: String,

    pub decimals: i64,
    pub reserve_id: Option<i32>,
    pub liquidation_threshold: i64,
    pub liquidation_bonus: i64,
    pub ltv: i64,

    pub is_active: bool,
    pub is_frozen: bool,
    pub is_paused: bool,
    pub is_borrowing_enabled: bool,
    pub is_dropped: bool,
    pub supply_cap: BigDecimal,
    pub borrow_cap: BigDecimal,
    pub reserve_factor: i64,

    pub is_collateral_enabled: bool,
    pub is_stable_borrow_enabled: bool,
    pub is_flash_loan_enabled: bool,
    pub emode_category_id: i32,
    pub debt_ceiling: BigDecimal,
    pub liquidation_protocol_fee: i64,
    pub is_siloed_borrowing: bool,
    pub unbacked_mint_cap: BigDecimal,

    pub atoken_address: String,
    pub v_debt_token_address: String,
    pub s_debt_token_address: String,
    pub interest_rate_strategy_address: Option<String>, // Nullable -> Option

    pub last_updated_block: i64,
    pub last_updated_log_index: i64,
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
    pub supply_cap: BigDecimal,
    pub borrow_cap: BigDecimal,
    pub reserve_factor: i64,

    pub is_collateral_enabled: bool,
    pub is_stable_borrow_enabled: bool,
    pub is_flash_loan_enabled: bool,
    pub emode_category_id: i32,
    pub debt_ceiling: BigDecimal,
    pub liquidation_protocol_fee: i64,
    pub is_siloed_borrowing: bool,
    pub unbacked_mint_cap: BigDecimal,

    pub atoken_address: String,
    pub v_debt_token_address: String,
    pub s_debt_token_address: String,
    pub interest_rate_strategy_address: String,

    pub last_updated_block: i64,
    pub last_updated_log_index: i64,
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
    pub last_updated_block: i64,
    pub last_updated_log_index: i64,
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
    pub last_updated_block: i64,
    pub last_updated_log_index: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = processed_events)]
pub struct NewProcessedEvent<'a> {
    pub tx_hash: &'a str,
    pub log_index: i64,
    pub block_number: i64,
}

#[derive(Debug, Queryable, Identifiable)]
#[diesel(table_name = processed_events)]
#[diesel(primary_key(tx_hash, log_index))]
pub struct ProcessedEvent {
    pub tx_hash: String,
    pub log_index: i64,
    pub block_number: i64,
    pub processed_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = sync_status)]
pub struct NewSyncStatus {
    pub id: i32,
    pub last_processed_block: i64,
}

#[derive(Debug, Queryable, Identifiable)]
#[diesel(table_name = sync_status)]
pub struct SyncStatus {
    pub id: i32,
    pub last_processed_block: i64,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = user_positions)]
#[diesel(primary_key(user_address, asset_address))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserPosition {
    pub user_address: String,
    pub asset_address: String,
    pub scaled_atoken_balance: BigDecimal,
    pub scaled_variable_debt: BigDecimal,
    pub use_as_collateral: bool,
    pub atoken_last_index: BigDecimal,
    pub debt_last_index: BigDecimal,
    pub last_updated_block: i64,
    pub last_updated_log_index: i64,
    pub is_active: bool,
    pub created_at_block: i64,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = user_positions)]
pub struct NewUserPosition {
    pub user_address: String,
    pub asset_address: String,
    pub scaled_atoken_balance: BigDecimal,
    pub scaled_variable_debt: BigDecimal,
    pub use_as_collateral: bool,
    pub atoken_last_index: BigDecimal,
    pub debt_last_index: BigDecimal,
    pub last_updated_block: i64,
    pub last_updated_log_index: i64,
    pub is_active: bool,
    pub created_at_block: i64,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = user_emode)]
#[diesel(primary_key(user_address))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserEmode {
    pub user_address: String,
    pub emode_category_id: i32,
    pub last_updated_block: i64,
    pub last_updated_log_index: i64,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = user_emode)]
pub struct NewUserEmode {
    pub user_address: String,
    pub emode_category_id: i32,
    pub last_updated_block: i64,
    pub last_updated_log_index: i64,
}

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = crate::db::schema::bootstrap_state)]
pub struct BootstrapState {
    pub id: i32,
    pub last_cursor: String,
    pub meta_block: i64,
    pub completed: bool,
}

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(table_name = crate::db::schema::bootstrap_state)]
pub struct NewBootstrapState {
    pub id: i32,
    pub last_cursor: String,
    pub meta_block: i64,
    pub completed: bool,
}
