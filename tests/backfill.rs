use aave_v3_tracker::backfill::runner::{BackfillConfig, is_retryable_error};
use eyre::eyre;
use std::time::Duration;

#[test]
fn test_default_config() {
    let config = BackfillConfig::default();

    assert_eq!(config.initial_chunk_size, 10);
    assert_eq!(config.min_chunk_size, 1);
    assert_eq!(config.max_chunk_size, 10);
    assert_eq!(config.max_logs_per_chunk, 1000);
    assert_eq!(config.backoff_max_elapsed, Duration::from_secs(300));
}

#[test]
fn test_timeout_is_retryable() {
    let error = eyre!("connection timeout after 30s");
    assert!(is_retryable_error(&error));
}

#[test]
fn test_connection_error_is_retryable() {
    let error = eyre!("connection refused");
    assert!(is_retryable_error(&error));
}

#[test]
fn test_rate_limit_is_retryable() {
    let error = eyre!("rate limit exceeded");
    assert!(is_retryable_error(&error));
}

#[test]
fn test_503_is_retryable() {
    let error = eyre!("HTTP 503 Service Unavailable");
    assert!(is_retryable_error(&error));
}

#[test]
fn test_502_is_retryable() {
    let error = eyre!("502 Bad Gateway");
    assert!(is_retryable_error(&error));
}

#[test]
fn test_504_is_retryable() {
    let error = eyre!("504 Gateway Timeout");
    assert!(is_retryable_error(&error));
}

#[test]
fn test_too_many_requests_is_retryable() {
    let error = eyre!("429 Too Many Requests");
    assert!(is_retryable_error(&error));
}

#[test]
fn test_invalid_address_is_not_retryable() {
    let error = eyre!("invalid ethereum address format");
    assert!(!is_retryable_error(&error));
}

#[test]
fn test_parse_error_is_not_retryable() {
    let error = eyre!("failed to parse response JSON");
    assert!(!is_retryable_error(&error));
}

#[test]
fn test_db_constraint_is_not_retryable() {
    let error = eyre!("unique constraint violation");
    assert!(!is_retryable_error(&error));
}

#[test]
fn test_case_insensitive_matching() {
    let error = eyre!("CONNECTION TIMEOUT");
    assert!(is_retryable_error(&error));
}

#[test]
fn test_nested_error_is_checked() {
    let inner = eyre!("connection refused");
    let outer = inner.wrap_err("Failed to fetch logs");
    assert!(is_retryable_error(&outer));
}
