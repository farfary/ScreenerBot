use crate::logger::{ log, LogTag };
use reqwest::{ Client, Response };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::time::{ Duration, Instant, SystemTime, UNIX_EPOCH };
use tokio::time::sleep;

/// Solscan API configuration and rate limiter
pub struct SolscanClient {
    client: Client,
    base_url: String,
    api_token: String,
    rate_limiter: Arc<Mutex<RateLimiter>>,
    usage_stats: Arc<Mutex<UsageStats>>,
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimiter {
    requests_per_second: u32,
    requests_per_minute: u32,
    requests_per_hour: u32,
    last_request: Option<Instant>,
    requests_this_second: u32,
    requests_this_minute: u32,
    requests_this_hour: u32,
    second_reset: Instant,
    minute_reset: Instant,
    hour_reset: Instant,
}

/// Usage statistics tracking
#[derive(Debug, Clone)]
pub struct UsageStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub requests_by_endpoint: HashMap<String, u64>,
    pub compute_units_used: u64,
    pub last_usage_check: Option<SystemTime>,
}

/// API usage response from Solscan monitor endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUsageResponse {
    pub success: bool,
    pub data: ApiUsageData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUsageData {
    pub remaining_cus: u64,
    pub usage_cus: u64,
    pub total_requests_24h: u64,
    pub success_rate_24h: f64,
    pub total_cu_24h: u64,
}

/// Top tokens response
#[derive(Debug, Serialize, Deserialize)]
pub struct TopTokensResponse {
    pub success: bool,
    pub data: TopTokensData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopTokensData {
    pub total: u32,
    pub items: Vec<TokenInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
    pub market_cap: u64,
    pub price: f64,
    pub price_24h_change: f64,
    pub holder: u32,
    pub created_time: u64,
}

/// Solscan API endpoints
#[derive(Debug, Clone)]
pub enum SolscanEndpoint {
    MonitorUsage,
    TopTokens,
    Transaction(String),
    TokenInfo(String),
    AccountInfo(String),
}

impl SolscanEndpoint {
    pub fn path(&self) -> String {
        match self {
            SolscanEndpoint::MonitorUsage => "/v2.0/monitor/usage".to_string(),
            SolscanEndpoint::TopTokens => "/v2.0/token/top".to_string(),
            SolscanEndpoint::Transaction(sig) => format!("/v2.0/transaction/{}", sig),
            SolscanEndpoint::TokenInfo(mint) => format!("/v2.0/token/{}", mint),
            SolscanEndpoint::AccountInfo(account) => format!("/v2.0/account/{}", account),
        }
    }

    pub fn name(&self) -> String {
        match self {
            SolscanEndpoint::MonitorUsage => "monitor_usage".to_string(),
            SolscanEndpoint::TopTokens => "top_tokens".to_string(),
            SolscanEndpoint::Transaction(_) => "transaction".to_string(),
            SolscanEndpoint::TokenInfo(_) => "token_info".to_string(),
            SolscanEndpoint::AccountInfo(_) => "account_info".to_string(),
        }
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            requests_per_second: 10, // Conservative limits
            requests_per_minute: 300,
            requests_per_hour: 5000,
            last_request: None,
            requests_this_second: 0,
            requests_this_minute: 0,
            requests_this_hour: 0,
            second_reset: now,
            minute_reset: now,
            hour_reset: now,
        }
    }

    pub fn can_make_request(&mut self) -> bool {
        let now = Instant::now();

        // Reset counters if time periods have elapsed
        if now.duration_since(self.second_reset) >= Duration::from_secs(1) {
            self.requests_this_second = 0;
            self.second_reset = now;
        }
        if now.duration_since(self.minute_reset) >= Duration::from_secs(60) {
            self.requests_this_minute = 0;
            self.minute_reset = now;
        }
        if now.duration_since(self.hour_reset) >= Duration::from_secs(3600) {
            self.requests_this_hour = 0;
            self.hour_reset = now;
        }

        // Check if we're within limits
        self.requests_this_second < self.requests_per_second &&
            self.requests_this_minute < self.requests_per_minute &&
            self.requests_this_hour < self.requests_per_hour
    }

    pub fn record_request(&mut self) {
        let now = Instant::now();
        self.last_request = Some(now);
        self.requests_this_second += 1;
        self.requests_this_minute += 1;
        self.requests_this_hour += 1;
    }

    pub fn get_wait_time(&self) -> Option<Duration> {
        let now = Instant::now();

        if self.requests_this_second >= self.requests_per_second {
            Some(Duration::from_millis(100)) // Wait 100ms if hitting per-second limit
        } else if let Some(last_req) = self.last_request {
            // Minimum 50ms between requests to be polite
            let elapsed = now.duration_since(last_req);
            if elapsed < Duration::from_millis(50) {
                Some(Duration::from_millis(50) - elapsed)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl UsageStats {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            requests_by_endpoint: HashMap::new(),
            compute_units_used: 0,
            last_usage_check: None,
        }
    }

    pub fn record_request(&mut self, endpoint: &str, success: bool) {
        self.total_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }

        *self.requests_by_endpoint.entry(endpoint.to_string()).or_insert(0) += 1;
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            ((self.successful_requests as f64) / (self.total_requests as f64)) * 100.0
        }
    }
}

impl SolscanClient {
    /// Create a new Solscan client with rate limiting
    pub fn new(api_token: String) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ScreenerBot/1.0")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            base_url: "https://pro-api.solscan.io".to_string(),
            api_token,
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new())),
            usage_stats: Arc::new(Mutex::new(UsageStats::new())),
        })
    }

    /// Make a rate-limited request to Solscan API
    async fn make_request(&self, endpoint: SolscanEndpoint) -> Result<Response, String> {
        // Check rate limits and wait if necessary
        let wait_time = {
            let mut limiter = self.rate_limiter.lock().map_err(|e| format!("Lock error: {}", e))?;
            if limiter.can_make_request() {
                limiter.record_request();
                None // No wait needed
            } else {
                limiter.get_wait_time()
            }
        };

        if let Some(wait) = wait_time {
            log(
                LogTag::Api,
                "DEBUG",
                &format!("Rate limiting: waiting {:?} before Solscan request", wait)
            );
            sleep(wait).await;

            // Try again after waiting
            let mut limiter = self.rate_limiter.lock().map_err(|e| format!("Lock error: {}", e))?;
            limiter.record_request();
        }

        let url = format!("{}{}", self.base_url, endpoint.path());
        let endpoint_name = endpoint.name();

        log(LogTag::Api, "DEBUG", &format!("Making Solscan API request to: {}", endpoint_name));

        let response = self.client
            .get(&url)
            .header("token", &self.api_token)
            .header("accept", "application/json")
            .send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let success = response.status().is_success();

        // Record usage statistics
        {
            let mut stats = self.usage_stats.lock().map_err(|e| format!("Lock error: {}", e))?;
            stats.record_request(&endpoint_name, success);
        }

        if !success {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            log(LogTag::Api, "ERROR", &format!("Solscan API error {}: {}", status, error_text));
            return Err(format!("Solscan API error: {} {}", status, error_text));
        }

        log(
            LogTag::Api,
            "DEBUG",
            &format!("Solscan API request to {} completed successfully", endpoint_name)
        );
        Ok(response)
    }

    /// Check API usage and remaining credits
    pub async fn check_api_usage(&self) -> Result<ApiUsageData, String> {
        log(LogTag::Api, "INFO", "Checking Solscan API usage");

        let response = self.make_request(SolscanEndpoint::MonitorUsage).await?;
        let usage_response: ApiUsageResponse = response
            .json().await
            .map_err(|e| format!("Failed to parse usage response: {}", e))?;

        if !usage_response.success {
            return Err("Solscan API returned success=false for usage check".to_string());
        }

        // Update our local usage stats
        {
            let mut stats = self.usage_stats.lock().map_err(|e| format!("Lock error: {}", e))?;
            stats.compute_units_used = usage_response.data.usage_cus;
            stats.last_usage_check = Some(SystemTime::now());
        }

        log(
            LogTag::Api,
            "INFO",
            &format!(
                "Solscan API Usage - Remaining: {} CUs, Used: {} CUs, 24h Requests: {}, Success Rate: {:.2}%",
                usage_response.data.remaining_cus,
                usage_response.data.usage_cus,
                usage_response.data.total_requests_24h,
                usage_response.data.success_rate_24h
            )
        );

        Ok(usage_response.data)
    }

    /// Get top tokens from Solscan
    pub async fn get_top_tokens(&self) -> Result<Vec<TokenInfo>, String> {
        log(LogTag::Api, "INFO", "Fetching top tokens from Solscan");

        let response = self.make_request(SolscanEndpoint::TopTokens).await?;
        let tokens_response: TopTokensResponse = response
            .json().await
            .map_err(|e| format!("Failed to parse top tokens response: {}", e))?;

        if !tokens_response.success {
            return Err("Solscan API returned success=false for top tokens".to_string());
        }

        log(
            LogTag::Api,
            "INFO",
            &format!("Retrieved {} top tokens from Solscan", tokens_response.data.items.len())
        );
        Ok(tokens_response.data.items)
    }

    /// Get transaction details from Solscan
    pub async fn get_transaction(&self, signature: &str) -> Result<serde_json::Value, String> {
        log(LogTag::Api, "INFO", &format!("Fetching transaction {} from Solscan", &signature[..8]));

        let response = self.make_request(
            SolscanEndpoint::Transaction(signature.to_string())
        ).await?;
        let transaction_data: serde_json::Value = response
            .json().await
            .map_err(|e| format!("Failed to parse transaction response: {}", e))?;

        log(
            LogTag::Api,
            "INFO",
            &format!("Retrieved transaction {} from Solscan", &signature[..8])
        );
        Ok(transaction_data)
    }

    /// Get token information from Solscan
    pub async fn get_token_info(&self, mint: &str) -> Result<serde_json::Value, String> {
        log(LogTag::Api, "INFO", &format!("Fetching token info for {} from Solscan", &mint[..8]));

        let response = self.make_request(SolscanEndpoint::TokenInfo(mint.to_string())).await?;
        let token_data: serde_json::Value = response
            .json().await
            .map_err(|e| format!("Failed to parse token info response: {}", e))?;

        log(LogTag::Api, "INFO", &format!("Retrieved token info for {} from Solscan", &mint[..8]));
        Ok(token_data)
    }

    /// Get account information from Solscan
    pub async fn get_account_info(&self, account: &str) -> Result<serde_json::Value, String> {
        log(
            LogTag::Api,
            "INFO",
            &format!("Fetching account info for {} from Solscan", &account[..8])
        );

        let response = self.make_request(SolscanEndpoint::AccountInfo(account.to_string())).await?;
        let account_data: serde_json::Value = response
            .json().await
            .map_err(|e| format!("Failed to parse account info response: {}", e))?;

        log(
            LogTag::Api,
            "INFO",
            &format!("Retrieved account info for {} from Solscan", &account[..8])
        );
        Ok(account_data)
    }

    /// Get current usage statistics
    pub fn get_usage_stats(&self) -> Result<UsageStats, String> {
        let stats = self.usage_stats.lock().map_err(|e| format!("Lock error: {}", e))?;
        Ok(stats.clone())
    }

    /// Check if API is healthy and within usage limits
    pub async fn health_check(&self) -> Result<bool, String> {
        match self.check_api_usage().await {
            Ok(usage) => {
                let healthy = usage.remaining_cus > 1000 && usage.success_rate_24h > 90.0;
                if healthy {
                    log(LogTag::Api, "INFO", "Solscan API health check: HEALTHY");
                } else {
                    log(
                        LogTag::Api,
                        "ERROR",
                        &format!(
                            "Solscan API health check: UNHEALTHY - Remaining CUs: {}, Success Rate: {:.2}%",
                            usage.remaining_cus,
                            usage.success_rate_24h
                        )
                    );
                }
                Ok(healthy)
            }
            Err(e) => {
                log(LogTag::Api, "ERROR", &format!("Solscan API health check failed: {}", e));
                Ok(false)
            }
        }
    }

    /// Log current rate limiter status
    pub fn log_rate_limiter_status(&self) {
        if let Ok(limiter) = self.rate_limiter.lock() {
            log(
                LogTag::Api,
                "INFO",
                &format!(
                    "Solscan Rate Limiter - This Second: {}/{}, This Minute: {}/{}, This Hour: {}/{}",
                    limiter.requests_this_second,
                    limiter.requests_per_second,
                    limiter.requests_this_minute,
                    limiter.requests_per_minute,
                    limiter.requests_this_hour,
                    limiter.requests_per_hour
                )
            );
        }
    }

    /// Log detailed usage statistics
    pub fn log_usage_stats(&self) {
        if let Ok(stats) = self.usage_stats.lock() {
            log(
                LogTag::Api,
                "INFO",
                &format!(
                    "Solscan Usage Stats - Total: {}, Success: {}, Failed: {}, Success Rate: {:.2}%",
                    stats.total_requests,
                    stats.successful_requests,
                    stats.failed_requests,
                    stats.get_success_rate()
                )
            );

            if !stats.requests_by_endpoint.is_empty() {
                log(LogTag::Api, "INFO", "Requests by endpoint:");
                for (endpoint, count) in &stats.requests_by_endpoint {
                    log(LogTag::Api, "INFO", &format!("  {}: {}", endpoint, count));
                }
            }
        }
    }
}

/// Global Solscan client instance
static mut SOLSCAN_CLIENT: Option<Arc<SolscanClient>> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Initialize the global Solscan client
pub fn initialize_solscan_client(api_token: String) -> Result<(), String> {
    INIT.call_once(|| {
        match SolscanClient::new(api_token) {
            Ok(client) => {
                unsafe {
                    SOLSCAN_CLIENT = Some(Arc::new(client));
                }
                log(LogTag::Api, "INFO", "✅ Solscan API client initialized successfully");
            }
            Err(e) => {
                log(
                    LogTag::Api,
                    "ERROR",
                    &format!("❌ Failed to initialize Solscan client: {}", e)
                );
            }
        }
    });

    Ok(())
}

/// Get the global Solscan client instance
pub fn get_solscan_client() -> Option<Arc<SolscanClient>> {
    unsafe { SOLSCAN_CLIENT.clone() }
}

/// Utility function to safely use Solscan client
pub async fn with_solscan_client<F, R>(f: F) -> Result<R, String>
    where F: FnOnce(Arc<SolscanClient>) -> R
{
    match get_solscan_client() {
        Some(client) => Ok(f(client)),
        None => Err("Solscan client not initialized".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new();

        // Should allow first request
        assert!(limiter.can_make_request());
        limiter.record_request();

        // Test that we track requests correctly
        assert_eq!(limiter.requests_this_second, 1);
        assert_eq!(limiter.requests_this_minute, 1);
        assert_eq!(limiter.requests_this_hour, 1);
    }

    #[test]
    fn test_usage_stats() {
        let mut stats = UsageStats::new();

        stats.record_request("test_endpoint", true);
        stats.record_request("test_endpoint", false);

        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.get_success_rate(), 50.0);
    }

    #[test]
    fn test_endpoint_paths() {
        assert_eq!(SolscanEndpoint::MonitorUsage.path(), "/v2.0/monitor/usage");
        assert_eq!(SolscanEndpoint::TopTokens.path(), "/v2.0/token/top");
        assert_eq!(
            SolscanEndpoint::Transaction("test".to_string()).path(),
            "/v2.0/transaction/test"
        );
    }
}
