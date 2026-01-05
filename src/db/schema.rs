// @generated automatically by Diesel CLI.

diesel::table! {
    processed_events (tx_hash, log_index) {
        #[max_length = 66]
        tx_hash -> Bpchar,
        log_index -> BigInt,
        block_number -> Int8,
        processed_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    reserve_state (asset_address) {
        #[max_length = 42]
        asset_address -> Bpchar,
        liquidity_index -> Numeric,
        variable_borrow_index -> Numeric,
        current_liquidity_rate -> Numeric,
        current_variable_borrow_rate -> Numeric,
        current_stable_borrow_rate -> Numeric,
        total_liquidity -> Numeric,
        total_variable_debt -> Numeric,
        total_stable_debt -> Numeric,
        accrued_to_treasury -> Numeric,
        unbacked -> Numeric,
        isolation_mode_total_debt -> Numeric,
        last_updated_block -> Int8,
        last_updated_log_index -> Int8,
    }
}

diesel::table! {
    reserves (asset_address) {
        #[max_length = 42]
        asset_address -> Bpchar,
        #[max_length = 255]
        symbol -> Varchar,
        decimals -> Int8,
        reserve_id -> Nullable<Int4>,
        liquidation_threshold -> Int8,
        liquidation_bonus -> Int8,
        ltv -> Int8,
        is_active -> Bool,
        is_frozen -> Bool,
        is_paused -> Bool,
        is_borrowing_enabled -> Bool,
        is_dropped -> Bool,
        supply_cap -> Numeric,
        borrow_cap -> Numeric,
        reserve_factor -> Int8,
        #[max_length = 42]
        atoken_address -> Bpchar,
        #[max_length = 42]
        v_debt_token_address -> Bpchar,
        #[max_length = 42]
        s_debt_token_address -> Bpchar,
        #[max_length = 42]
        interest_rate_strategy_address -> Nullable<Bpchar>,
        last_updated_block -> Int8,
        last_updated_log_index -> Int8,
    }
}

diesel::table! {
    sync_status (id) {
        id -> Int4,
        last_processed_block -> Int8,
    }
}

diesel::joinable!(reserve_state -> reserves (asset_address));

diesel::allow_tables_to_appear_in_same_query!(
    processed_events,
    reserve_state,
    reserves,
    sync_status,
);
