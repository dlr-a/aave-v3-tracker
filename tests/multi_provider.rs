use aave_v3_tracker::provider::{MultiProvider, is_provider_error};
use alloy_provider::Provider;
use dotenvy::dotenv;
use std::env;

fn load_urls() -> Vec<String> {
    dotenv().ok();
    env::var("HTTP_RPC_URLS")
        .or_else(|_| env::var("HTTP_RPC_URL").map(|u| u.to_string()))
        .expect("Set HTTP_RPC_URLS or HTTP_RPC_URL")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[tokio::test]
async fn rotate_and_query_still_works() {
    let urls = load_urls();
    if urls.len() < 2 {
        eprintln!("Skipping: need at least 2 URLs in HTTP_RPC_URLS");
        return;
    }
    let provider = MultiProvider::new(urls).unwrap();

    let block1 = provider.get_block_number().await.unwrap();
    assert_eq!(provider.current_index(), 0);

    provider.rotate();
    assert_eq!(provider.current_index(), 1);

    let block2 = provider.get_block_number().await.unwrap();
    assert!(block1.abs_diff(block2) < 5);
}

#[tokio::test]
async fn fallback_after_bad_provider() {
    let urls = load_urls();
    let mut mixed = vec!["http://localhost:1".to_string()];
    mixed.extend(urls);

    let provider = MultiProvider::new(mixed).unwrap();

    let result = provider.get_block_number().await;
    assert!(result.is_err());

    provider.rotate();
    let block = provider.get_block_number().await.unwrap();
    assert!(block > 0);
}

#[test]
fn rotate_cycles_through_providers() {
    let urls = vec![
        "http://localhost:8545".to_string(),
        "http://localhost:8546".to_string(),
        "http://localhost:8547".to_string(),
    ];
    let provider = MultiProvider::new(urls).unwrap();

    assert_eq!(provider.current_index(), 0);
    provider.rotate();
    assert_eq!(provider.current_index(), 1);
    provider.rotate();
    assert_eq!(provider.current_index(), 2);
    provider.rotate();
    assert_eq!(provider.current_index(), 0);
}

#[test]
fn rotate_noop_with_single_provider() {
    let urls = vec!["http://localhost:8545".to_string()];
    let provider = MultiProvider::new(urls).unwrap();

    provider.rotate();
    assert_eq!(provider.current_index(), 0);
}

#[test]
fn clone_shares_state() {
    let urls = vec![
        "http://localhost:8545".to_string(),
        "http://localhost:8546".to_string(),
    ];
    let provider = MultiProvider::new(urls).unwrap();
    let cloned = provider.clone();

    provider.rotate();
    assert_eq!(cloned.current_index(), 1);
}

#[test]
fn empty_urls_returns_error() {
    assert!(MultiProvider::new(vec![]).is_err());
}

#[test]
fn detects_provider_errors() {
    assert!(is_provider_error(&eyre::eyre!(
        "HTTP 429 Too Many Requests"
    )));
    assert!(is_provider_error(&eyre::eyre!("rate limit exceeded")));
    assert!(is_provider_error(&eyre::eyre!("too many requests")));
    assert!(is_provider_error(&eyre::eyre!("connection timeout")));
    assert!(is_provider_error(&eyre::eyre!("502 Bad Gateway")));
    assert!(is_provider_error(&eyre::eyre!("503 Service Unavailable")));
    assert!(!is_provider_error(&eyre::eyre!("invalid block range")));
    assert!(!is_provider_error(&eyre::eyre!(
        "diesel error: unique violation"
    )));
}
