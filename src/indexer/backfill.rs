use crate::abi::{
    BorrowCapChanged, CollateralConfigurationChanged, DebtCeilingChanged,
    EModeAssetCategoryChanged, LiquidationProtocolFeeChanged, ReserveActive, ReserveBorrowing,
    ReserveDataUpdated, ReserveDropped, ReserveFactorChanged, ReserveFlashLoaning, ReserveFrozen,
    ReserveInitialized, ReserveInterestRateStrategyChanged, ReservePaused,
    ReserveStableRateBorrowing, ReserveUnfrozen, SiloedBorrowingChanged, SupplyCapChanged,
    UnbackedMintCapChanged,
};
use crate::db::connection::DbPool;
use crate::db::repositories::sync_status_repository;
use crate::indexer::dispatcher::handle_log_logic;
use alloy::primitives::Address;
use alloy::{providers::Provider, rpc::types::eth::Filter};
use alloy_primitives::address;
use alloy_sol_types::SolEvent;
use backoff::{ExponentialBackoff, future::retry};
use diesel_async::AsyncConnection;
use eyre::{Context, Result};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct BackfillConfig {
    pub initial_chunk_size: i64,
    pub min_chunk_size: i64,
    pub max_chunk_size: i64,
    pub max_logs_per_chunk: usize,
    pub backoff_max_elapsed: Duration,
}

impl Default for BackfillConfig {
    fn default() -> Self {
        Self {
            initial_chunk_size: 10,
            min_chunk_size: 1,
            max_chunk_size: 10,
            max_logs_per_chunk: 1000,
            backoff_max_elapsed: Duration::from_secs(300),
        }
    }
}

pub async fn backfill(
    pool: &DbPool,
    provider: impl Provider + Clone + 'static,
    from_block: i64,
    to_block: i64,
) -> Result<()> {
    let config = BackfillConfig::default();
    let addresses: Vec<Address> = vec![
        address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2"),
        address!("0x64b761D848206f447Fe2dd461b0c635Ec39EbB27"),
    ];

    let events = vec![
        // Transfer::SIGNATURE_HASH,
        ReserveInitialized::SIGNATURE_HASH,
        ReserveDataUpdated::SIGNATURE_HASH,
        ReserveStableRateBorrowing::SIGNATURE_HASH,
        ReserveDropped::SIGNATURE_HASH,
        ReserveFactorChanged::SIGNATURE_HASH,
        ReserveInterestRateStrategyChanged::SIGNATURE_HASH,
        CollateralConfigurationChanged::SIGNATURE_HASH,
        ReserveFrozen::SIGNATURE_HASH,
        ReserveUnfrozen::SIGNATURE_HASH,
        ReservePaused::SIGNATURE_HASH,
        ReserveBorrowing::SIGNATURE_HASH,
        ReserveActive::SIGNATURE_HASH,
        BorrowCapChanged::SIGNATURE_HASH,
        SupplyCapChanged::SIGNATURE_HASH,
        ReserveFlashLoaning::SIGNATURE_HASH,
        EModeAssetCategoryChanged::SIGNATURE_HASH,
        DebtCeilingChanged::SIGNATURE_HASH,
        LiquidationProtocolFeeChanged::SIGNATURE_HASH,
        SiloedBorrowingChanged::SIGNATURE_HASH,
        UnbackedMintCapChanged::SIGNATURE_HASH,
    ];

    let mut conn = pool.get().await.wrap_err("Failed to get DB connection")?;
    let start_block = match sync_status_repository::get_last_block(&mut conn).await {
        Ok(last) if last >= from_block => {
            tracing::info!(checkpoint = last, "Resuming from checkpoint");
            last + 1
        }
        _ => from_block,
    };
    drop(conn);

    if start_block > to_block {
        tracing::info!("Backfill already complete");
        return Ok(());
    }

    let mut current = start_block;
    let mut chunk_size = config.initial_chunk_size;

    while current <= to_block {
        let end = (current + chunk_size - 1).min(to_block);

        match process_chunk_with_retry(
            pool,
            provider.clone(),
            current,
            end,
            &addresses,
            &events,
            &config,
        )
        .await
        {
            Ok(log_count) => {
                if log_count > config.max_logs_per_chunk {
                    chunk_size = (chunk_size / 2).max(config.min_chunk_size);
                } else if log_count < config.max_logs_per_chunk / 4 {
                    chunk_size = (chunk_size * 2).min(config.max_chunk_size);
                }
                current = end + 1;
            }
            Err(e) => {
                return Err(e).wrap_err(format!("Failed to process chunk {}-{}", current, end));
            }
        }
    }

    tracing::info!("Backfill completed: blocks {} → {}", from_block, to_block);
    Ok(())
}

async fn process_chunk_with_retry(
    pool: &DbPool,
    provider: impl Provider + Clone + 'static,
    from_block: i64,
    to_block: i64,
    addresses: &[Address],
    events: &[alloy_primitives::B256],
    config: &BackfillConfig,
) -> Result<usize> {
    let start_time = Instant::now();

    let backoff = ExponentialBackoff {
        max_elapsed_time: Some(config.backoff_max_elapsed),
        initial_interval: Duration::from_millis(100),
        max_interval: Duration::from_secs(10),
        multiplier: 2.0,
        randomization_factor: 0.3,
        ..Default::default()
    };

    let log_count = retry(backoff, || {
        let pool = pool.clone();
        let provider = provider.clone();
        let addresses = addresses.to_vec();
        let events = events.to_vec();

        async move {
            process_chunk_once(&pool, provider, from_block, to_block, &addresses, &events)
                .await
                .map_err(|e| {
                    if is_retryable_error(&e) {
                        tracing::warn!(error = ?e, "Transient error, will retry");
                        backoff::Error::transient(e)
                    } else {
                        tracing::error!(error = ?e, "Permanent error");
                        backoff::Error::permanent(e)
                    }
                })
        }
    })
    .await?;

    tracing::info!(
        from = from_block,
        to = to_block,
        logs = log_count,
        duration_ms = start_time.elapsed().as_millis(),
        "Chunk committed"
    );

    Ok(log_count)
}

async fn process_chunk_once(
    pool: &DbPool,
    provider: impl Provider + Clone + 'static,
    from_block: i64,
    to_block: i64,
    addresses: &[Address],
    events: &[alloy_primitives::B256],
) -> Result<usize> {
    let filter = Filter::new()
        .from_block(from_block as u64)
        .to_block(to_block as u64)
        .address(addresses.to_vec())
        .event_signature(events.to_vec());

    let logs = tokio::time::timeout(Duration::from_secs(30), provider.get_logs(&filter))
        .await
        .wrap_err("Timeout fetching logs")?
        .wrap_err("Failed to fetch logs")?;

    let log_count = logs.len();

    let mut conn = pool.get().await.wrap_err("Failed to get DB connection")?;

    conn.transaction::<_, eyre::Report, _>(|conn| {
        let provider = provider.clone();
        let logs = logs.clone();

        Box::pin(async move {
            if log_count == 0 {
                sync_status_repository::update_last_block(conn, to_block).await?;
                return Ok(0);
            }

            let mut logs = logs;
            logs.sort_by_key(|l| (l.block_number, l.log_index));

            for log in &logs {
                handle_log_logic(conn, pool, provider.clone(), log)
                    .await
                    .wrap_err_with(|| {
                        format!(
                            "Failed to handle log at block {} index {}",
                            log.block_number.unwrap_or_default(),
                            log.log_index.unwrap_or_default()
                        )
                    })?;
            }

            sync_status_repository::update_last_block(conn, to_block).await?;

            Ok(log_count)
        })
    })
    .await
}

fn is_retryable_error(error: &eyre::Report) -> bool {
    let error_string = format!("{:?}", error).to_lowercase();

    let retryable_patterns = [
        "timeout",
        "connection",
        "rate limit",
        "too many requests",
        "503",
        "502",
        "504",
        "temporary",
        "transient",
        "unavailable",
        "try again",
        "backend",
    ];

    retryable_patterns
        .iter()
        .any(|pattern| error_string.contains(pattern))
}
