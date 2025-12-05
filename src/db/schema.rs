// @generated automatically by Diesel CLI.

diesel::table! {
    reserve_state (asset_address) {
        #[max_length = 42]
        asset_address -> Bpchar,
        liquidity_index -> Numeric,
        variable_borrow_index -> Numeric,
        current_liquidity_rate -> Nullable<Numeric>,
        current_variable_borrow_rate -> Nullable<Numeric>,
    }
}

diesel::table! {
    reserves (asset_address) {
        #[max_length = 42]
        asset_address -> Bpchar,
        #[max_length = 255]
        symbol -> Varchar,
        decimals -> Int8,
        liquidation_threshold -> Int8,
        liquidation_bonus -> Int8,
        ltv -> Int8,
        is_active -> Bool,
        is_frozen -> Bool,
        #[max_length = 42]
        atoken_address -> Bpchar,
        #[max_length = 42]
        v_debt_token_address -> Bpchar,
        #[max_length = 42]
        s_debt_token_address -> Bpchar,
    }
}

diesel::joinable!(reserve_state -> reserves (asset_address));

diesel::allow_tables_to_appear_in_same_query!(reserve_state, reserves,);
