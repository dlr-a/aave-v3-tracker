use crate::db::connection::DbPool;
use crate::db::repositories::sync_status_repository;
use crate::indexer::dispatcher::handle_log_logic;
use alloy::primitives::Address;
use alloy::{providers::Provider, rpc::types::eth::Filter};
use alloy_primitives::address;
use eyre::Result;

pub async fn backfill(
    pool: &DbPool,
    provider: impl Provider + Clone,
    from_block: i64,
    to_block: i64,
) -> Result<()> {
    let addresses: Vec<Address> = vec![
        address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2"),
        address!("0x64b761D848206f447Fe2dd461b0c635Ec39EbB27"),
    ];

    const CHUNK_SIZE: i64 = 10;

    let mut current = from_block;

    while current <= to_block {
        let end = (current + CHUNK_SIZE - 1).min(to_block);

        let filter = Filter::new()
            .from_block(current as u64)
            .to_block(end as u64)
            .address(addresses.clone());

        tracing::info!("Backfill chunk: {} → {}", current, end);

        let logs = provider.get_logs(&filter).await?;

        let mut logs = logs;
        logs.sort_by_key(|l| (l.block_number, l.log_index));

        for log in logs {
            handle_log_logic(pool, provider.clone(), &log).await?;
        }

        let mut conn = pool.get().await?;

        sync_status_repository::update_last_block(&mut conn, end).await?;

        current = end + 1;
    }

    Ok(())
}
