CREATE TABLE reserves (
    asset_address CHAR(42) PRIMARY KEY,
    symbol VARCHAR(255) NOT NULL,
    
    decimals BIGINT NOT NULL,
    reserve_id INTEGER,
    liquidation_threshold BIGINT NOT NULL DEFAULT 0,
    liquidation_bonus BIGINT NOT NULL DEFAULT 0,
    ltv BIGINT NOT NULL DEFAULT 0,
    
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_frozen BOOLEAN NOT NULL DEFAULT FALSE,
    is_paused BOOLEAN NOT NULL DEFAULT FALSE,
    is_borrowing_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    is_dropped BOOLEAN NOT NULL DEFAULT FALSE,
    supply_cap NUMERIC(78, 0) NOT NULL DEFAULT 0,
    borrow_cap NUMERIC(78, 0) NOT NULL DEFAULT 0,
    reserve_factor BIGINT NOT NULL DEFAULT 0,
    
    is_collateral_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    is_stable_borrow_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    is_flash_loan_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    debt_ceiling NUMERIC(78, 0) NOT NULL DEFAULT 0,
    liquidation_protocol_fee BIGINT NOT NULL DEFAULT 0,
    is_siloed_borrowing BOOLEAN NOT NULL DEFAULT FALSE,
    unbacked_mint_cap NUMERIC(78, 0) NOT NULL DEFAULT 0,
    
    atoken_address CHAR(42) NOT NULL,
    v_debt_token_address CHAR(42) NOT NULL,
    s_debt_token_address CHAR(42) NOT NULL,
    interest_rate_strategy_address CHAR(42),
    last_updated_block BIGINT NOT NULL DEFAULT 0,
    last_updated_log_index BIGINT NOT NULL DEFAULT 0
);


CREATE TABLE reserve_state (
    asset_address CHAR(42) PRIMARY KEY REFERENCES reserves(asset_address),
    liquidity_index NUMERIC(78, 0) NOT NULL DEFAULT 0,
    variable_borrow_index NUMERIC(78, 0) NOT NULL DEFAULT 0,
    current_liquidity_rate NUMERIC(78, 0) NOT NULL DEFAULT 0,
    current_variable_borrow_rate NUMERIC(78, 0) NOT NULL DEFAULT 0,
    current_stable_borrow_rate NUMERIC(78, 0) NOT NULL DEFAULT 0,
    total_liquidity NUMERIC(78, 0) NOT NULL DEFAULT 0, 
    total_variable_debt NUMERIC(78, 0) NOT NULL DEFAULT 0,
    total_stable_debt NUMERIC(78, 0) NOT NULL DEFAULT 0,
    accrued_to_treasury NUMERIC(78, 0) NOT NULL DEFAULT 0,
    unbacked NUMERIC(78,0) NOT NULL DEFAULT 0,
    isolation_mode_total_debt NUMERIC(78,0) NOT NULL DEFAULT 0,
    last_updated_block BIGINT NOT NULL DEFAULT 0,
    last_updated_log_index BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE user_positions (
    user_address              CHAR(42) NOT NULL,
    asset_address             CHAR(42) NOT NULL REFERENCES reserves(asset_address),
    scaled_atoken_balance     NUMERIC(78, 0) NOT NULL DEFAULT 0,
    scaled_variable_debt      NUMERIC(78, 0) NOT NULL DEFAULT 0,
    use_as_collateral         BOOLEAN NOT NULL DEFAULT FALSE,
    atoken_last_index         NUMERIC(78, 0) NOT NULL DEFAULT 0,
    debt_last_index           NUMERIC(78, 0) NOT NULL DEFAULT 0,
    last_updated_block        BIGINT NOT NULL DEFAULT 0,
    last_updated_log_index    BIGINT NOT NULL DEFAULT 0,
    is_active                 BOOLEAN NOT NULL DEFAULT FALSE,
    created_at_block          BIGINT NOT NULL,
    PRIMARY KEY (user_address, asset_address)
);


CREATE INDEX idx_user_positions_user ON user_positions(user_address);
CREATE INDEX idx_user_positions_asset ON user_positions(asset_address);

CREATE TABLE bootstrap_state (
    id INTEGER PRIMARY KEY DEFAULT 1,
    last_cursor TEXT NOT NULL DEFAULT '',
    meta_block BIGINT NOT NULL DEFAULT 0,
    completed BOOLEAN NOT NULL DEFAULT FALSE
);


CREATE TABLE user_emode (
    user_address      CHAR(42) PRIMARY KEY,
    emode_category_id INTEGER NOT NULL DEFAULT 0,
    last_updated_block     BIGINT NOT NULL,
    last_updated_log_index BIGINT NOT NULL
);

CREATE TABLE emode_categories (
    category_id   INTEGER PRIMARY KEY,
    ltv           BIGINT NOT NULL DEFAULT 0,
    liquidation_threshold BIGINT NOT NULL DEFAULT 0,
    liquidation_bonus     BIGINT NOT NULL DEFAULT 0,
    collateral_bitmap NUMERIC(39, 0) NOT NULL DEFAULT 0,
    borrowable_bitmap NUMERIC(39, 0) NOT NULL DEFAULT 0,
    ltvzero_bitmap    NUMERIC(39, 0) NOT NULL DEFAULT 0,
    label         VARCHAR(64) NOT NULL DEFAULT '',
    last_updated_block     BIGINT NOT NULL DEFAULT 0,
    last_updated_log_index BIGINT NOT NULL DEFAULT 0
);



CREATE TABLE processed_events (
    tx_hash CHAR(66) NOT NULL,
    log_index BIGINT NOT NULL,
    block_number BIGINT NOT NULL,
    processed_at TIMESTAMP DEFAULT now(),

    PRIMARY KEY (tx_hash, log_index)
);

CREATE INDEX idx_processed_events_block
ON processed_events(block_number);



CREATE TABLE sync_status (
    id INTEGER PRIMARY KEY DEFAULT 1,
    last_processed_block BIGINT NOT NULL
);

INSERT INTO sync_status (id, last_processed_block)
VALUES (1, 0)
ON CONFLICT (id) DO NOTHING;