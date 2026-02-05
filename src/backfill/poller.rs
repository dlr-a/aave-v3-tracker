use crate::backfill::runner::backfill;
use crate::db::connection::DbPool;
use crate::db::repositories::sync_status_repository;
use alloy::providers::Provider;
use backoff::{ExponentialBackoff, future::retry};
use eyre::Result;
use std::time::Duration;

pub async fn backfill_loop(pool: &DbPool, provider: impl Provider + Clone + 'static) -> Result<()> {
    const CONFIRMATIONS: i64 = 20;
    const STEP: i64 = 1_000;
    const MIN_GAP: i64 = 10;
    const INTERVAL_SECS: u64 = 12;

    let mut consecutive_failures = 0u32;

    loop {
        let result: eyre::Result<()> = async {
            let last = retry(db_backoff(), || async {
                let mut conn = pool.get().await.map_err(|e| {
                    tracing::warn!("DB connection failed: {}", e);
                    backoff::Error::transient(eyre::eyre!(e))
                })?;
                sync_status_repository::get_last_block(&mut conn)
                    .await
                    .map_err(|e| {
                        tracing::warn!("get_last_block failed: {}", e);
                        backoff::Error::transient(eyre::eyre!(e))
                    })
            })
            .await?;

            let head = retry(provider_backoff(), || {
                let provider = provider.clone();
                async move {
                    provider
                        .get_block_number()
                        .await
                        .map(|n| n as i64)
                        .map_err(|e| {
                            tracing::warn!("Failed to get block number: {}", e);
                            backoff::Error::transient(e)
                        })
                }
            })
            .await?;

            let target = head - CONFIRMATIONS;

            if last + MIN_GAP < target {
                let to = (last + STEP).min(target);
                tracing::info!("Backfill gap detected: {} → {} (head: {})", last, to, head);

                backfill(pool, provider.clone(), last + 1, to).await?;
            } else {
                tracing::debug!(
                    "Backfill up to date: last={}, target={}, gap={}",
                    last,
                    target,
                    target - last
                );
            }

            Ok(())
        }
        .await;

        const ALERT_THRESHOLD: u32 = 10;

        match result {
            Ok(()) => {
                consecutive_failures = 0;
            }
            Err(err) => {
                consecutive_failures += 1;
                tracing::error!(error = ?err, consecutive_failures, "Backfill iteration failed");

                if consecutive_failures == ALERT_THRESHOLD {
                    tracing::error!(
                        "🚨 ALERT: {} consecutive failures - needs investigation",
                        consecutive_failures
                    );
                }
            }
        }

        let wait = (INTERVAL_SECS * 2u64.pow(consecutive_failures.min(4))).min(300);
        tokio::time::sleep(Duration::from_secs(wait)).await;
    }
}

fn db_backoff() -> ExponentialBackoff {
    ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(30)),
        initial_interval: Duration::from_millis(100),
        max_interval: Duration::from_secs(5),
        multiplier: 2.0,
        ..Default::default()
    }
}

fn provider_backoff() -> ExponentialBackoff {
    ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(60)),
        initial_interval: Duration::from_millis(100),
        max_interval: Duration::from_secs(5),
        multiplier: 2.0,
        ..Default::default()
    }
}
