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
