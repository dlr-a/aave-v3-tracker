use alloy::providers::{Provider, RootProvider};
use alloy_network::Ethereum;
use eyre::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
pub struct MultiProvider {
    inner: Arc<MultiProviderInner>,
}

struct MultiProviderInner {
    providers: Vec<RootProvider<Ethereum>>,
    current: AtomicUsize,
}

impl MultiProvider {
    pub fn new(urls: Vec<String>) -> Result<Self> {
        if urls.is_empty() {
            return Err(eyre::eyre!("At least one RPC URL required"));
        }

        let providers = urls
            .iter()
            .map(|url| {
                let parsed = url
                    .parse()
                    .map_err(|e| eyre::eyre!("Invalid URL {}: {}", url, e))?;
                Ok(RootProvider::new_http(parsed))
            })
            .collect::<Result<Vec<_>>>()?;

        tracing::info!(count = providers.len(), "Initialized MultiProvider");

        Ok(Self {
            inner: Arc::new(MultiProviderInner {
                providers,
                current: AtomicUsize::new(0),
            }),
        })
    }

    pub fn rotate(&self) {
        let len = self.inner.providers.len();
        if len <= 1 {
            return;
        }
        let prev = self.inner.current.fetch_add(1, Ordering::Relaxed);
        let next = (prev + 1) % len;
        tracing::warn!(from = prev % len, to = next, "Rotated RPC provider");
    }

    pub fn provider_count(&self) -> usize {
        self.inner.providers.len()
    }

    pub fn current_index(&self) -> usize {
        self.inner.current.load(Ordering::Relaxed) % self.inner.providers.len()
    }
}

#[async_trait::async_trait]
impl Provider for MultiProvider {
    fn root(&self) -> &RootProvider<Ethereum> {
        let idx = self.inner.current.load(Ordering::Relaxed) % self.inner.providers.len();
        &self.inner.providers[idx]
    }
}

pub fn is_provider_error(error: &eyre::Report) -> bool {
    let error_string = format!("{:?}", error).to_lowercase();
    let patterns = [
        "rate limit",
        "too many requests",
        "429",
        "timeout",
        "connection",
        "502",
        "503",
        "504",
        "unavailable",
        "backend",
    ];
    patterns.iter().any(|p| error_string.contains(p))
}
