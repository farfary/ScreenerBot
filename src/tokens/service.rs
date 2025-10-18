// Tokens orchestrator logic – owned by tokens module; invoked by a centralized service in services module

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::logger::{log, LogTag};
use crate::tokens::blacklist as bl;
use crate::tokens::provider::TokenDataProvider;
use crate::tokens::store;
use crate::tokens::{decimals, discovery};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio_metrics::TaskMonitor;

/// Public orchestrator that owns provider and spawns all token-related background tasks.
pub struct TokensOrchestrator {
    provider: Arc<TokenDataProvider>,
}

impl TokensOrchestrator {
    pub async fn new() -> Result<Self, String> {
        let provider = Arc::new(TokenDataProvider::new().await?);
        Ok(Self { provider })
    }

    /// Initialize caches/state owned by tokens module (e.g., blacklist hydration)
    pub fn initialize(&self) -> Result<(), String> {
        let db = self.provider.database();
        match bl::hydrate_from_db(&db) {
            Ok(count) => log(LogTag::Tokens, "INFO", &format!("Blacklist hydrated: {} entries", count)),
            Err(e) => log(LogTag::Tokens, "WARN", &format!("Blacklist hydrate failed: {}", e)),
        }
        Ok(())
    }

    /// Start all background tasks; return join handles to the caller service.
    pub async fn start(
        &self,
        shutdown: Arc<Notify>,
        monitor: TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let mut handles = Vec::new();

        // Discovery loop (every 20s)
        {
            let provider = self.provider.clone();
            let shutdown_c = shutdown.clone();
            let fut = async move {
                loop {
                    tokio::select! {
                        _ = shutdown_c.notified() => break,
                        _ = tokio::time::sleep(Duration::from_secs(20)) => {
                            match discovery::discover_from_sources(&provider).await {
                                Ok(entries) => {
                                    if !entries.is_empty() {
                                        log(LogTag::Tokens, "INFO", &format!("Discovery dispatching {} mints", entries.len()));
                                        discovery::process_new_mints(&provider, entries).await;
                                    }
                                }
                                Err(e) => log(LogTag::Tokens, "WARN", &format!("Discovery error: {}", e)),
                            }
                        }
                    }
                }
            };
            handles.push(tokio::spawn(monitor.instrument(fut)));
        }

        // Decimals ensure loop (lazy fill for known mints)
        {
            let provider = self.provider.clone();
            let shutdown_c = shutdown.clone();
            let fut = async move {
                let mut completed: HashSet<String> = HashSet::new();
                let mut retry_state: HashMap<String, RetryState> = HashMap::new();
                loop {
                    tokio::select! {
                        _ = shutdown_c.notified() => break,
                        _ = tokio::time::sleep(Duration::from_secs(5)) => {
                            let mints = store::list_mints();
                            for mint in mints {
                                if bl::is(&mint) { continue; }
                                if completed.contains(&mint) {
                                    continue;
                                }

                                if let Some(snapshot) = store::get_snapshot(&mint) {
                                    if snapshot.decimals.is_some() {
                                        completed.insert(mint.clone());
                                        retry_state.remove(&mint);
                                        continue;
                                    }
                                }

                                let now = Instant::now();
                                let state = retry_state
                                    .entry(mint.clone())
                                    .or_insert_with(|| RetryState::new(now));

                                if now < state.next_attempt {
                                    continue;
                                }

                                match decimals::ensure(&provider, &mint).await {
                                    Ok(_) => {
                                        completed.insert(mint.clone());
                                        retry_state.remove(&mint);
                                    }
                                    Err(e) => {
                                        state.attempts += 1;
                                        match determine_retry_backoff(state.attempts, &e) {
                                            RetryDisposition::RetryAfter(delay) => {
                                                state.next_attempt = now + delay;
                                                log(
                                                    LogTag::Tokens,
                                                    "WARN",
                                                    &format!(
                                                        "Decimals ensure failed: mint={} attempts={} next_retry_in={}s err={}",
                                                        mint,
                                                        state.attempts,
                                                        delay.as_secs(),
                                                        e
                                                    )
                                                );
                                            }
                                            RetryDisposition::GiveUp => {
                                                log(
                                                    LogTag::Tokens,
                                                    "WARN",
                                                    &format!(
                                                        "Decimals ensure giving up: mint={} attempts={} err={}",
                                                        mint,
                                                        state.attempts,
                                                        e
                                                    )
                                                );
                                                completed.insert(mint.clone());
                                                retry_state.remove(&mint);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            };
            handles.push(tokio::spawn(monitor.instrument(fut)));
        }

        Ok(handles)
    }
}

#[derive(Debug, Clone)]
struct RetryState {
    attempts: u32,
    next_attempt: Instant,
}

impl RetryState {
    fn new(now: Instant) -> Self {
        Self {
            attempts: 0,
            next_attempt: now,
        }
    }
}

enum RetryDisposition {
    RetryAfter(Duration),
    GiveUp,
}

fn determine_retry_backoff(attempts: u32, err: &str) -> RetryDisposition {
    let error_text = err.to_ascii_lowercase();

    if error_text.contains("invalid pubkey")
        || error_text.contains("not a valid public key")
        || error_text.contains("account not found")
        || error_text.contains("could not find account")
        || error_text.contains("no account data")
    {
        return RetryDisposition::GiveUp;
    }

    if error_text.contains("rate limit") || error_text.contains("429") {
        return RetryDisposition::RetryAfter(Duration::from_secs(120));
    }

    if error_text.contains("timeout")
        || error_text.contains("timed out")
        || error_text.contains("connection reset")
        || error_text.contains("connection refused")
    {
        return RetryDisposition::RetryAfter(Duration::from_secs(45));
    }

    let base_delay = match attempts {
        0 | 1 => Duration::from_secs(15),
        2 => Duration::from_secs(60),
        3 => Duration::from_secs(300),
        _ => Duration::from_secs(900),
    };

    RetryDisposition::RetryAfter(base_delay)
}
