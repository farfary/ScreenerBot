use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::time::{ Duration, Instant };
use reqwest::Client;
use serde_json::{ json, Value };
use tokio::time::sleep;
use crate::global::{ is_shutdown, get_config, update_rpc_stats };
use crate::logger::{ log, LogLevel };

#[derive(Debug)]
pub struct RpcManager {
    client: Client,
    main_rpc_url: String,
    fallback_urls: Vec<String>,
    current_fallback_index: usize,
    rate_limiter: Arc<Mutex<RateLimiter>>,
    method_stats: Arc<Mutex<HashMap<String, MethodStats>>>,
    max_retries: usize,
    retry_delay: Duration,
}

#[derive(Debug)]
pub struct RateLimiter {
    requests_per_second: u32,
    last_request_times: Vec<Instant>,
    max_requests: usize,
}

#[derive(Debug, Clone)]
pub struct MethodStats {
    pub calls: u64,
    pub successes: u64,
    pub failures: u64,
    pub total_time: Duration,
    pub last_call: Option<Instant>,
}

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("Rate limited")]
    RateLimited,
    #[error("All endpoints failed")]
    AllEndpointsFailed,
    #[error("Invalid response")]
    InvalidResponse,
    #[error("Network error: {0}")] NetworkError(String),
}

impl RpcManager {
    pub fn new() -> anyhow::Result<Self> {
        let config = get_config().ok_or_else(|| anyhow::anyhow!("Config not available"))?;
        let config_guard = config.lock().unwrap();

        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(RpcManager {
            client,
            main_rpc_url: config_guard.rpc_url.clone(),
            fallback_urls: config_guard.rpc_fallbacks.clone(),
            current_fallback_index: 0,
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(10))), // 10 requests per second
            method_stats: Arc::new(Mutex::new(HashMap::new())),
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
        })
    }

    pub async fn call_method(&self, method: &str, params: Vec<Value>) -> anyhow::Result<Value> {
        let start_time = Instant::now();

        // Check rate limit
        if !self.check_rate_limit().await {
            log("RPC", LogLevel::Warn, &format!("Rate limited for method: {}", method));
            update_rpc_stats(|stats| {
                stats.rate_limited_calls += 1;
            });
            return Err(anyhow::anyhow!("Rate limited"));
        }

        // Update method stats
        self.update_method_stats(method, true, Duration::default()).await;

        let mut last_error = None;

        // Try main RPC first
        match self.try_rpc_call(&self.main_rpc_url, method, &params).await {
            Ok(response) => {
                let elapsed = start_time.elapsed();
                self.update_method_stats(method, true, elapsed).await;
                update_rpc_stats(|stats| {
                    stats.main_rpc_calls += 1;
                    stats.total_calls += 1;
                });
                log(
                    "RPC",
                    LogLevel::Info,
                    &format!("Method {} succeeded on main RPC in {:?}", method, elapsed)
                );
                return Ok(response);
            }
            Err(e) => {
                log("RPC", LogLevel::Warn, &format!("Main RPC failed for {}: {}", method, e));
                last_error = Some(e);
            }
        }

        // Try fallback RPCs
        for (attempt, url) in self.fallback_urls.iter().enumerate() {
            if is_shutdown() {
                break;
            }

            match self.try_rpc_call(url, method, &params).await {
                Ok(response) => {
                    let elapsed = start_time.elapsed();
                    self.update_method_stats(method, true, elapsed).await;
                    update_rpc_stats(|stats| {
                        stats.fallback_rpc_calls += 1;
                        stats.total_calls += 1;
                    });
                    log(
                        "RPC",
                        LogLevel::Info,
                        &format!(
                            "Method {} succeeded on fallback RPC {} in {:?}",
                            method,
                            attempt + 1,
                            elapsed
                        )
                    );
                    return Ok(response);
                }
                Err(e) => {
                    log(
                        "RPC",
                        LogLevel::Warn,
                        &format!("Fallback RPC {} failed for {}: {}", attempt + 1, method, e)
                    );
                    last_error = Some(e);

                    // Wait before trying next fallback
                    if attempt < self.fallback_urls.len() - 1 {
                        sleep(self.retry_delay).await;
                    }
                }
            }
        }

        // All endpoints failed
        let elapsed = start_time.elapsed();
        self.update_method_stats(method, false, elapsed).await;
        update_rpc_stats(|stats| {
            stats.failed_calls += 1;
            stats.total_calls += 1;
        });

        log("RPC", LogLevel::Error, &format!("All RPC endpoints failed for method: {}", method));
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All endpoints failed")))
    }

    async fn try_rpc_call(
        &self,
        url: &str,
        method: &str,
        params: &[Value]
    ) -> anyhow::Result<Value> {
        let payload =
            json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let response = self.client.post(url).json(&payload).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }

        let json_response: Value = response.json().await?;

        if let Some(error) = json_response.get("error") {
            return Err(anyhow::anyhow!("RPC error: {}", error));
        }

        json_response
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Invalid response"))
    }

    async fn check_rate_limit(&self) -> bool {
        let mut limiter = self.rate_limiter.lock().unwrap();
        limiter.check_and_update()
    }

    async fn update_method_stats(&self, method: &str, success: bool, duration: Duration) {
        let mut stats = self.method_stats.lock().unwrap();
        let entry = stats.entry(method.to_string()).or_insert_with(|| MethodStats {
            calls: 0,
            successes: 0,
            failures: 0,
            total_time: Duration::default(),
            last_call: None,
        });

        entry.calls += 1;
        if success {
            entry.successes += 1;
        } else {
            entry.failures += 1;
        }
        entry.total_time += duration;
        entry.last_call = Some(Instant::now());
    }

    pub async fn get_method_stats(&self) -> HashMap<String, MethodStats> {
        self.method_stats.lock().unwrap().clone()
    }

    // Common Solana RPC methods
    pub async fn get_balance(&self, pubkey: &str) -> anyhow::Result<u64> {
        let params = vec![json!(pubkey), json!({"commitment": "confirmed"})];
        let result = self.call_method("getBalance", params).await?;

        result
            .get("value")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Invalid balance response"))
    }

    pub async fn get_account_info(&self, pubkey: &str) -> anyhow::Result<Value> {
        let params = vec![json!(pubkey), json!({"commitment": "confirmed", "encoding": "base64"})];
        self.call_method("getAccountInfo", params).await
    }

    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &str,
        mint: Option<&str>
    ) -> anyhow::Result<Value> {
        let filter = if let Some(mint) = mint {
            json!({"mint": mint})
        } else {
            json!({"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"})
        };

        let params = vec![
            json!(owner),
            filter,
            json!({"commitment": "confirmed", "encoding": "base64"})
        ];
        self.call_method("getTokenAccountsByOwner", params).await
    }

    pub async fn get_signature_statuses(&self, signatures: Vec<&str>) -> anyhow::Result<Value> {
        let params = vec![json!(signatures), json!({"searchTransactionHistory": true})];
        self.call_method("getSignatureStatuses", params).await
    }

    pub async fn simulate_transaction(&self, transaction: &str) -> anyhow::Result<Value> {
        let params = vec![
            json!(transaction),
            json!({"commitment": "confirmed", "encoding": "base64"})
        ];
        self.call_method("simulateTransaction", params).await
    }

    pub async fn send_transaction(&self, transaction: &str) -> anyhow::Result<String> {
        let params = vec![
            json!(transaction),
            json!({"commitment": "confirmed", "encoding": "base64"})
        ];
        let result = self.call_method("sendTransaction", params).await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Invalid transaction response"))
    }

    pub async fn get_recent_blockhash(&self) -> anyhow::Result<Value> {
        let params = vec![json!({"commitment": "confirmed"})];
        self.call_method("getRecentBlockhash", params).await
    }

    pub async fn get_slot(&self) -> anyhow::Result<u64> {
        let result = self.call_method("getSlot", vec![]).await?;
        result.as_u64().ok_or_else(|| anyhow::anyhow!("Invalid slot response"))
    }
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            requests_per_second,
            last_request_times: Vec::new(),
            max_requests: requests_per_second as usize,
        }
    }

    pub fn check_and_update(&mut self) -> bool {
        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);

        // Remove old requests
        self.last_request_times.retain(|&time| time > one_second_ago);

        // Check if we can make a new request
        if self.last_request_times.len() < self.max_requests {
            self.last_request_times.push(now);
            true
        } else {
            false
        }
    }
}

// Global RPC manager instance
use once_cell::sync::Lazy;
pub static RPC_MANAGER: Lazy<Arc<Mutex<Option<RpcManager>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

pub async fn initialize_rpc_manager() -> anyhow::Result<()> {
    let manager = RpcManager::new()?;
    let mut global_manager = RPC_MANAGER.lock().unwrap();
    *global_manager = Some(manager);
    Ok(())
}

pub async fn get_rpc_manager() -> anyhow::Result<Arc<Mutex<Option<RpcManager>>>> {
    Ok(RPC_MANAGER.clone())
}

pub fn start_rpc_manager() {
    tokio::task::spawn(async move {
        if let Err(e) = initialize_rpc_manager().await {
            log("RPC", LogLevel::Error, &format!("Failed to initialize RPC manager: {}", e));
            return;
        }

        log("RPC", LogLevel::Info, "RPC Manager initialized successfully");

        let delays = crate::global::get_task_delays();

        loop {
            if is_shutdown() {
                log("RPC", LogLevel::Info, "RPC Manager shutting down...");
                break;
            }

            // Periodic RPC health check and stats logging
            let stats_count = {
                let manager_guard = RPC_MANAGER.lock().unwrap();
                if let Some(_manager) = manager_guard.as_ref() {
                    // For now, just return a simple count
                    1
                } else {
                    0
                }
            };

            if stats_count > 0 {
                log("RPC", LogLevel::Info, "RPC Manager is active");
            }

            tokio::time::sleep(Duration::from_secs(delays.rpc_delay)).await;
        }
    });
}
