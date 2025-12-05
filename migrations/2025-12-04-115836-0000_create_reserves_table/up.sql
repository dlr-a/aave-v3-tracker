CREATE TABLE reserves (
    asset_address CHAR(42) PRIMARY KEY,
    symbol VARCHAR(255) NOT NULL,
    
    decimals BIGINT NOT NULL,
    liquidation_threshold BIGINT NOT NULL DEFAULT 0,
    liquidation_bonus BIGINT NOT NULL DEFAULT 0,
    ltv BIGINT NOT NULL DEFAULT 0,
    
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_frozen BOOLEAN NOT NULL DEFAULT FALSE,
    
    atoken_address CHAR(42) NOT NULL,
    v_debt_token_address CHAR(42) NOT NULL,
    s_debt_token_address CHAR(42) NOT NULL
);


CREATE TABLE reserve_state (
    asset_address CHAR(42) PRIMARY KEY REFERENCES reserves(asset_address),
    liquidity_index NUMERIC(78, 0) NOT NULL DEFAULT 0,
    variable_borrow_index NUMERIC(78, 0) NOT NULL DEFAULT 0,
    current_liquidity_rate NUMERIC(78, 0) DEFAULT 0,
    current_variable_borrow_rate NUMERIC(78, 0) DEFAULT 0
);