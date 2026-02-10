use aave_v3_tracker::db::models::{NewReserve, Reserve};
use aave_v3_tracker::db::schema::reserves;
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use aave_v3_tracker::db::models::{NewReserveState, ReserveState};
use aave_v3_tracker::db::schema::reserve_state;
use alloy::primitives::Address;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

static GLOBAL_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn unique_asset() -> String {
    let id = GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst);
    let hex = format!("0x{:040x}", id);
    Address::from_str(&hex).unwrap().to_string()
}

pub fn unique_tx_hash() -> String {
    let id = GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("0x{:064x}", id)
}

pub struct ReserveStateBuilder {
    asset_address: String,
    liquidity_index: BigDecimal,
    variable_borrow_index: BigDecimal,
    current_liquidity_rate: BigDecimal,
    current_variable_borrow_rate: BigDecimal,
    current_stable_borrow_rate: BigDecimal,
    last_updated_block: i64,
    last_updated_log_index: i64,
}

impl Default for ReserveStateBuilder {
    fn default() -> Self {
        Self {
            asset_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            liquidity_index: BigDecimal::from(1),
            variable_borrow_index: BigDecimal::from(1),
            current_liquidity_rate: BigDecimal::from(0),
            current_variable_borrow_rate: BigDecimal::from(0),
            current_stable_borrow_rate: BigDecimal::from(0),
            last_updated_block: 0,
            last_updated_log_index: 0,
        }
    }
}

impl ReserveStateBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn asset_address(mut self, addr: &str) -> Self {
        self.asset_address = addr.to_string();
        self
    }

    pub fn at_block(mut self, block: i64, log_index: i64) -> Self {
        self.last_updated_block = block;
        self.last_updated_log_index = log_index;
        self
    }

    pub fn liquidity_index(mut self, val: BigDecimal) -> Self {
        self.liquidity_index = val;
        self
    }

    pub fn variable_borrow_index(mut self, val: BigDecimal) -> Self {
        self.variable_borrow_index = val;
        self
    }

    pub fn current_liquidity_rate(mut self, val: BigDecimal) -> Self {
        self.current_liquidity_rate = val;
        self
    }

    pub fn current_variable_borrow_rate(mut self, val: BigDecimal) -> Self {
        self.current_variable_borrow_rate = val;
        self
    }

    pub fn current_stable_borrow_rate(mut self, val: BigDecimal) -> Self {
        self.current_stable_borrow_rate = val;
        self
    }

    pub async fn insert(self, conn: &mut AsyncPgConnection) -> String {
        let asset = self.asset_address.clone();

        let state = NewReserveState {
            asset_address: self.asset_address,
            liquidity_index: self.liquidity_index,
            variable_borrow_index: self.variable_borrow_index,
            current_liquidity_rate: self.current_liquidity_rate,
            current_variable_borrow_rate: self.current_variable_borrow_rate,
            current_stable_borrow_rate: self.current_stable_borrow_rate,
            total_liquidity: BigDecimal::from(0),
            total_variable_debt: BigDecimal::from(0),
            total_stable_debt: BigDecimal::from(0),
            accrued_to_treasury: BigDecimal::from(0),
            unbacked: BigDecimal::from(0),
            isolation_mode_total_debt: BigDecimal::from(0),
            last_updated_block: self.last_updated_block,
            last_updated_log_index: self.last_updated_log_index,
        };

        diesel::insert_into(reserve_state::table)
            .values(&state)
            .execute(conn)
            .await
            .expect("Failed to insert test reserve state");

        asset
    }
}

pub async fn get_reserve_state(conn: &mut AsyncPgConnection, asset: &str) -> Option<ReserveState> {
    use aave_v3_tracker::db::schema::reserve_state::dsl::*;

    reserve_state
        .filter(asset_address.eq(asset))
        .first(conn)
        .await
        .ok()
}

pub struct ReserveBuilder {
    asset_address: String,
    is_frozen: bool,
    is_paused: bool,
    is_active: bool,
    is_borrowing_enabled: bool,
    is_dropped: bool,
    ltv: i64,
    liquidation_threshold: i64,
    liquidation_bonus: i64,
    supply_cap: BigDecimal,
    borrow_cap: BigDecimal,
    reserve_factor: i64,
    is_flash_loan_enabled: bool,
    is_stable_borrow_enabled: bool,
    is_siloed_borrowing: bool,
    emode_category_id: i32,
    debt_ceiling: BigDecimal,
    liquidation_protocol_fee: i64,
    unbacked_mint_cap: BigDecimal,
    last_updated_block: i64,
    last_updated_log_index: i64,
}

impl Default for ReserveBuilder {
    fn default() -> Self {
        Self {
            asset_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            is_frozen: false,
            is_paused: false,
            is_active: true,
            is_borrowing_enabled: true,
            is_dropped: false,
            ltv: 8000,
            liquidation_threshold: 8500,
            liquidation_bonus: 10500,
            supply_cap: BigDecimal::from(1_000_000),
            borrow_cap: BigDecimal::from(500_000),
            reserve_factor: 1000,
            is_flash_loan_enabled: true,
            is_stable_borrow_enabled: false,
            is_siloed_borrowing: false,
            emode_category_id: 0,
            debt_ceiling: BigDecimal::from(0),
            liquidation_protocol_fee: 1000,
            unbacked_mint_cap: BigDecimal::from(0),
            last_updated_block: 0,
            last_updated_log_index: 0,
        }
    }
}

impl ReserveBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn asset_address(mut self, addr: &str) -> Self {
        self.asset_address = addr.to_string();
        self
    }

    pub fn frozen(mut self, val: bool) -> Self {
        self.is_frozen = val;
        self
    }

    pub fn paused(mut self, val: bool) -> Self {
        self.is_paused = val;
        self
    }

    pub fn active(mut self, val: bool) -> Self {
        self.is_active = val;
        self
    }

    pub fn borrowing_enabled(mut self, val: bool) -> Self {
        self.is_borrowing_enabled = val;
        self
    }

    pub fn dropped(mut self, val: bool) -> Self {
        self.is_dropped = val;
        self
    }

    pub fn at_block(mut self, block: i64, log_index: i64) -> Self {
        self.last_updated_block = block;
        self.last_updated_log_index = log_index;
        self
    }

    pub fn ltv(mut self, val: i64) -> Self {
        self.ltv = val;
        self
    }

    pub fn liquidation_threshold(mut self, val: i64) -> Self {
        self.liquidation_threshold = val;
        self
    }

    pub fn liquidation_bonus(mut self, val: i64) -> Self {
        self.liquidation_bonus = val;
        self
    }

    pub fn supply_cap(mut self, val: i64) -> Self {
        self.supply_cap = BigDecimal::from(val);
        self
    }

    pub fn borrow_cap(mut self, val: i64) -> Self {
        self.borrow_cap = BigDecimal::from(val);
        self
    }

    pub fn reserve_factor(mut self, val: i64) -> Self {
        self.reserve_factor = val;
        self
    }

    pub fn flash_loan_enabled(mut self, val: bool) -> Self {
        self.is_flash_loan_enabled = val;
        self
    }

    pub fn stable_borrow_enabled(mut self, val: bool) -> Self {
        self.is_stable_borrow_enabled = val;
        self
    }

    pub fn siloed_borrowing(mut self, val: bool) -> Self {
        self.is_siloed_borrowing = val;
        self
    }

    pub fn emode_category_id(mut self, val: i32) -> Self {
        self.emode_category_id = val;
        self
    }

    pub fn debt_ceiling(mut self, val: i64) -> Self {
        self.debt_ceiling = BigDecimal::from(val);
        self
    }

    pub fn liquidation_protocol_fee(mut self, val: i64) -> Self {
        self.liquidation_protocol_fee = val;
        self
    }

    pub fn unbacked_mint_cap(mut self, val: i64) -> Self {
        self.unbacked_mint_cap = BigDecimal::from(val);
        self
    }

    pub async fn insert(self, conn: &mut AsyncPgConnection) -> String {
        let asset = self.asset_address.clone();

        let reserve = NewReserve {
            asset_address: self.asset_address,
            symbol: "TEST".to_string(),
            decimals: 6,
            reserve_id: 1,
            ltv: self.ltv,
            liquidation_threshold: self.liquidation_threshold,
            liquidation_bonus: self.liquidation_bonus,
            is_active: self.is_active,
            is_frozen: self.is_frozen,
            is_paused: self.is_paused,
            is_borrowing_enabled: self.is_borrowing_enabled,
            is_dropped: self.is_dropped,
            supply_cap: self.supply_cap,
            borrow_cap: self.borrow_cap,
            reserve_factor: self.reserve_factor,
            is_collateral_enabled: true,
            is_stable_borrow_enabled: self.is_stable_borrow_enabled,
            is_flash_loan_enabled: self.is_flash_loan_enabled,
            emode_category_id: self.emode_category_id,
            debt_ceiling: self.debt_ceiling,
            liquidation_protocol_fee: self.liquidation_protocol_fee,
            is_siloed_borrowing: self.is_siloed_borrowing,
            unbacked_mint_cap: self.unbacked_mint_cap,
            atoken_address: "0x0000000000000000000000000000000000000001".to_string(),
            v_debt_token_address: "0x0000000000000000000000000000000000000002".to_string(),
            s_debt_token_address: "0x0000000000000000000000000000000000000003".to_string(),
            interest_rate_strategy_address: "0x0000000000000000000000000000000000000004"
                .to_string(),
            last_updated_block: self.last_updated_block,
            last_updated_log_index: self.last_updated_log_index,
        };

        diesel::insert_into(reserves::table)
            .values(&reserve)
            .execute(conn)
            .await
            .expect("Failed to insert test reserve");

        asset
    }
}

pub async fn get_reserve(conn: &mut AsyncPgConnection, asset: &str) -> Option<Reserve> {
    use aave_v3_tracker::db::schema::reserves::dsl::*;

    reserves
        .filter(asset_address.eq(asset))
        .first(conn)
        .await
        .ok()
}
