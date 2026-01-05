use crate::db::connection::DbPool;
use crate::db::repositories::sync_status_repository;
use crate::indexer::backfill::backfill;
use alloy::providers::Provider;
use backoff::{ExponentialBackoff, future::retry};
use eyre::Result;
use std::time::Duration;

pub async fn backfill_loop(pool: &DbPool, provider: impl Provider + Clone) -> Result<()> {
    const CONFIRMATIONS: i64 = 10;
    const STEP: i64 = 1_000;
    const MIN_GAP: i64 = 100;
    const INTERVAL_SECS: u64 = 12;

    let backoff = ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(300)),
        ..Default::default()
    };

    loop {
        let result: eyre::Result<()> = async {
            let last = {
                let mut conn = pool.get().await?;
                sync_status_repository::get_last_block(&mut conn).await?
            };

            let head = retry(backoff.clone(), || {
                let provider = provider.clone();
                async move {
                    provider
                        .get_block_number()
                        .await
                        .map(|n| n as i64)
                        .map_err(backoff::Error::transient)
                }
            })
            .await?;

            let target = head - CONFIRMATIONS;

            if last + MIN_GAP < target {
                tracing::info!("Starting backfill for: {} → {}", last, target);
                let to = (last + STEP).min(target);

                retry(backoff.clone(), || {
                    let provider = provider.clone();
                    async move {
                        backfill(pool, provider, last + 1, to)
                            .await
                            .map_err(backoff::Error::transient)
                    }
                })
                .await?;
            }

            Ok(())
        }
        .await;

        if let Err(err) = result {
            tracing::error!(error = ?err, "Backfill iteration failed");
        }

        tokio::time::sleep(Duration::from_secs(INTERVAL_SECS)).await;
    }
}
