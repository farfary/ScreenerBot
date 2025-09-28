use crate::arguments::is_debug_security_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::security_db::{ parse_rugcheck_response, SecurityDatabase, SecurityInfo };
use once_cell::sync::Lazy;
use reqwest::{ Client, StatusCode };
use std::collections::HashMap;
use std::sync::atomic::{ AtomicU64, Ordering };
use std::sync::{ Arc, Mutex };
use tokio::sync::{ Notify, RwLock };
use tokio::time::{ sleep, Duration, Instant };

const RUGCHECK_API_BASE: &str = "https://api.rugcheck.xyz/v1/tokens";
const MAX_CACHE_AGE_HOURS: i64 = 24;

#[derive(Debug, Default)]
pub struct SecurityMetrics {
    pub api_calls_total: AtomicU64,
    pub api_calls_success: AtomicU64,
    pub api_calls_failed: AtomicU64,
    pub last_api_call: Arc<RwLock<Option<Instant>>>,
}

impl SecurityMetrics {
    pub fn new() -> Self {
        Self {
            api_calls_total: AtomicU64::new(0),
            api_calls_success: AtomicU64::new(0),
            api_calls_failed: AtomicU64::new(0),
            last_api_call: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn record_api_call(&self, success: bool) {
        self.api_calls_total.fetch_add(1, Ordering::Relaxed);
        if success {
            self.api_calls_success.fetch_add(1, Ordering::Relaxed);
        } else {
            self.api_calls_failed.fetch_add(1, Ordering::Relaxed);
        }
        *self.last_api_call.write().await = Some(Instant::now());
    }
}

pub struct SecurityAnalyzer {
    db_path: String,
    client: Client,
    cache: Arc<RwLock<HashMap<String, SecurityInfo>>>, // In-memory cache for fast access
    metrics: Arc<SecurityMetrics>,
}

#[derive(Debug, Clone)]
pub struct SecuritySummary {
    pub api_calls_total: u64,
    pub api_calls_success: u64,
    pub api_calls_failed: u64,
    pub last_api_call: Option<Instant>,
    pub db_safe_tokens: u64,
    pub db_warning_tokens: u64,
    pub db_danger_tokens: u64,
    pub db_missing_tokens: u64,
    pub db_pump_fun_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct SecurityAnalysis {
    pub score: i32,
    pub score_normalized: i32,
    pub risk_level: RiskLevel,
    pub authorities_safe: bool,
    pub lp_safe: bool,
    pub holders_safe: bool,
    pub liquidity_adequate: bool,
    pub pump_fun_token: bool,
    pub risks: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Safe, // score >= 70
    Warning, // 40-69
    Danger, // < 40
    Unknown, // No data
}

impl SecurityAnalyzer {
    pub fn new(db_path: &str) -> Result<Self, String> {
        // Test database connection to ensure it works
        let _test_db = SecurityDatabase::new(db_path).map_err(|e|
            format!("Failed to initialize security database: {}", e)
        )?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(SecurityAnalyzer {
            db_path: db_path.to_string(),
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(SecurityMetrics::new()),
        })
    }

    fn get_db(&self) -> Result<SecurityDatabase, String> {
        SecurityDatabase::new(&self.db_path).map_err(|e| format!("Failed to open database: {}", e))
    }

    // Compatibility helper: expose DB security info retrieval
    pub fn get_security_info(&self, mint: &str) -> Result<Option<SecurityInfo>, String> {
        self.get_db()?
            .get_security_info(mint)
            .map_err(|e| format!("Database error: {}", e))
    }

    // Compatibility helper: check in-memory cache presence
    pub async fn has_cached_info(&self, mint: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_key(mint)
    }

    // Public methods for database operations used by test binaries
    pub fn count_tokens_without_security(&self) -> Result<i64, String> {
        self.get_db()?
            .count_tokens_without_security()
            .map_err(|e| format!("Database error: {}", e))
    }

    pub fn get_tokens_without_security(&self) -> Result<Vec<String>, String> {
        self.get_db()?
            .get_tokens_without_security()
            .map_err(|e| format!("Database error: {}", e))
    }

    pub async fn analyze_token(&self, mint: &str) -> SecurityAnalysis {
        // Only log a compact start line in debug mode to avoid noise
        if is_debug_security_enabled() {
            log(
                LogTag::Security,
                "ANALYZE",
                &format!("Starting security analysis for mint={}", mint)
            );
        }

        // Try to get from cache first
        {
            let cache = self.cache.read().await;
            if let Some(info) = cache.get(mint) {
                if is_debug_security_enabled() {
                    log(
                        LogTag::Security,
                        "CACHE_HIT",
                        &format!("Using cached security data for mint={}", mint)
                    );
                }
                let analysis = self.calculate_security_analysis(info);
                log(
                    LogTag::Security,
                    "ANALYSIS",
                    &format!(
                        "mint={} risk_level={:?} score={} risks={} pump_fun={} source=cache",
                        mint,
                        analysis.risk_level,
                        analysis.score_normalized,
                        analysis.risks.len(),
                        analysis.pump_fun_token
                    )
                );
                return analysis;
            }
        }

        if let Ok(db) = self.get_db() {
            match db.get_security_info(mint) {
                Ok(Some(info)) => {
                    match db.is_stale(mint, MAX_CACHE_AGE_HOURS) {
                        Ok(false) => {
                            if is_debug_security_enabled() {
                                log(
                                    LogTag::Security,
                                    "DB_HIT",
                                    &format!("Using fresh database security data for mint={}", mint)
                                );
                            }
                            // Add to cache
                            {
                                let mut cache = self.cache.write().await;
                                cache.insert(mint.to_string(), info.clone());
                            }
                            let analysis = self.calculate_security_analysis(&info);
                            log(
                                LogTag::Security,
                                "ANALYSIS",
                                &format!(
                                    "mint={} risk_level={:?} score={} risks={} pump_fun={} source=db",
                                    mint,
                                    analysis.risk_level,
                                    analysis.score_normalized,
                                    analysis.risks.len(),
                                    analysis.pump_fun_token
                                )
                            );
                            return analysis;
                        }
                        Ok(true) => {
                            if is_debug_security_enabled() {
                                log(
                                    LogTag::Security,
                                    "DB_STALE",
                                    &format!("Database security data is stale for mint={}, refreshing", mint)
                                );
                            }
                        }
                        Err(e) => {
                            if is_debug_security_enabled() {
                                log(
                                    LogTag::Security,
                                    "DB_ERROR",
                                    &format!("Error checking staleness for mint={}: {}", mint, e)
                                );
                            }
                        }
                    }
                }
                Ok(None) => {
                    if is_debug_security_enabled() {
                        log(
                            LogTag::Security,
                            "DB_MISS",
                            &format!("No security data in database for mint={}", mint)
                        );
                    }
                }
                Err(e) => {
                    if is_debug_security_enabled() {
                        log(
                            LogTag::Security,
                            "DB_ERROR",
                            &format!("Database error for mint={}: {}", mint, e)
                        );
                    }
                }
            }
        }

        // Fetch fresh data from Rugcheck API
        match self.fetch_rugcheck_data(mint).await {
            Ok(info) => {
                self.metrics.record_api_call(true).await;
                // Store in database
                if
                    let Err(e) = self
                        .get_db()
                        .and_then(|db| db.store_security_info(&info).map_err(|e| e.to_string()))
                {
                    if is_debug_security_enabled() {
                        log(
                            LogTag::Security,
                            "DB_STORE_ERROR",
                            &format!("Failed to store security data for mint={}: {}", mint, e)
                        );
                    }
                }

                // Add to cache
                {
                    let mut cache = self.cache.write().await;
                    cache.insert(mint.to_string(), info.clone());
                }

                let analysis = self.calculate_security_analysis(&info);
                log(
                    LogTag::Security,
                    "ANALYSIS",
                    &format!(
                        "mint={} risk_level={:?} score={} risks={} pump_fun={} source=api",
                        mint,
                        analysis.risk_level,
                        analysis.score_normalized,
                        analysis.risks.len(),
                        analysis.pump_fun_token
                    )
                );
                analysis
            }
            Err(e) => {
                self.metrics.record_api_call(false).await;
                if is_debug_security_enabled() {
                    log(
                        LogTag::Security,
                        "API_ERROR",
                        &format!("Failed to fetch security data for mint={}: {}", mint, e)
                    );
                }
                // Return conservative analysis for unknown tokens
                let analysis = SecurityAnalysis {
                    score: 0,
                    score_normalized: 0,
                    risk_level: RiskLevel::Unknown,
                    authorities_safe: false,
                    lp_safe: false,
                    holders_safe: false,
                    liquidity_adequate: false,
                    pump_fun_token: false,
                    risks: vec!["Failed to fetch security data".to_string()],
                    summary: "Unable to analyze token security".to_string(),
                };
                log(
                    LogTag::Security,
                    "ANALYSIS",
                    &format!(
                        "mint={} risk_level={:?} score={} risks={} pump_fun={} source=error",
                        mint,
                        analysis.risk_level,
                        analysis.score_normalized,
                        analysis.risks.len(),
                        analysis.pump_fun_token
                    )
                );
                analysis
            }
        }
    }

    async fn fetch_rugcheck_data(&self, mint: &str) -> Result<SecurityInfo, String> {
        // Add a base delay between all API calls to prevent rate limiting
        static LAST_API_CALL: std::sync::OnceLock<std::sync::Mutex<Option<Instant>>> = std::sync::OnceLock::new();
        let last_call_mutex = LAST_API_CALL.get_or_init(|| std::sync::Mutex::new(None));

        // Rate limit: minimum 500ms between API calls to respect Rugcheck limits
        let delay_needed = {
            if let Ok(mut last_call) = last_call_mutex.lock() {
                let delay_needed = if let Some(last_instant) = *last_call {
                    let elapsed = last_instant.elapsed();
                    let min_interval = Duration::from_millis(500);
                    if elapsed < min_interval {
                        Some(min_interval - elapsed)
                    } else {
                        None
                    }
                } else {
                    None
                };
                *last_call = Some(Instant::now());
                delay_needed
            } else {
                None
            }
        };

        if let Some(delay) = delay_needed {
            if is_debug_security_enabled() {
                log(
                    LogTag::Security,
                    "RATE_LIMIT",
                    &format!(
                        "Rate limiting: waiting {}ms before API call for mint={}",
                        delay.as_millis(),
                        mint
                    )
                );
            }
            sleep(delay).await;
        }

        if is_debug_security_enabled() {
            log(
                LogTag::Security,
                "API_FETCH",
                &format!("Fetching Rugcheck data for mint={}", mint)
            );
        }

        let url = format!("{}/{}/report", RUGCHECK_API_BASE, mint);

        // Improved exponential backoff for rate limits - longer delays for 429 errors
        let mut attempt: u32 = 0;
        let max_attempts: u32 = 4; // 1 initial + 3 retries
        let mut last_err: Option<String> = None;

        while attempt < max_attempts {
            attempt += 1;
            let req = self.client.get(&url);
            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let raw_json = resp
                            .text().await
                            .map_err(|e| format!("Failed to read response body: {}", e))?;
                        return parse_rugcheck_response(&raw_json).map_err(|e|
                            format!("Failed to parse Rugcheck response: {}", e)
                        );
                    }

                    // For rate limit errors, use longer delays specifically
                    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                        let delay = if status == StatusCode::TOO_MANY_REQUESTS {
                            // For 429 errors, use much longer delays: 1s, 3s, 8s, 20s
                            let backoff_seconds = match attempt {
                                1 => 1,
                                2 => 3,
                                3 => 8,
                                _ => 20,
                            };
                            Duration::from_secs(backoff_seconds)
                        } else {
                            // For server errors, use normal exponential backoff
                            let base_delay_ms = 250u64 * (1u64 << (attempt - 1));
                            Duration::from_millis(base_delay_ms)
                        };

                        if is_debug_security_enabled() {
                            log(
                                LogTag::Security,
                                "API_RETRY",
                                &format!(
                                    "Rugcheck status {} {} for mint={}, retrying in {}ms (attempt {}/{})",
                                    status.as_u16(),
                                    status.canonical_reason().unwrap_or("Unknown"),
                                    mint,
                                    delay.as_millis(),
                                    attempt,
                                    max_attempts
                                )
                            );
                        }
                        sleep(delay).await;
                        continue;
                    } else {
                        return Err(format!("Rugcheck API returned status: {}", status));
                    }
                }
                Err(e) => {
                    last_err = Some(format!("HTTP request failed: {}", e));
                    // Retry network errors with backoff
                    if attempt < max_attempts {
                        let base_delay_ms = 250u64 * (1u64 << (attempt - 1));
                        let delay = Duration::from_millis(base_delay_ms);
                        if is_debug_security_enabled() {
                            log(
                                LogTag::Security,
                                "API_RETRY",
                                &format!(
                                    "HTTP error for mint={}, retrying in {}ms (attempt {}/{})",
                                    mint,
                                    delay.as_millis(),
                                    attempt,
                                    max_attempts
                                )
                            );
                        }
                        sleep(delay).await;
                        continue;
                    }
                }
            }
            break;
        }

        Err(last_err.unwrap_or_else(|| "Unknown error during Rugcheck fetch".to_string()))
    }

    // Security data extraction - no filtering decisions
    fn calculate_security_analysis(&self, info: &SecurityInfo) -> SecurityAnalysis {
        let mut risks = Vec::new();

        // Check if it's a Pump.Fun token
        let pump_fun_token = info.markets
            .iter()
            .any(|m| {
                m.market_type.to_lowercase().contains("pump_fun") ||
                    m.market_type.to_lowercase().contains("pump.fun")
            });

        // Analyze authorities
        let mint_authority_safe =
            info.mint_authority.is_none() ||
            info.mint_authority.as_deref() == Some("11111111111111111111111111111111");
        let freeze_authority_safe =
            info.freeze_authority.is_none() ||
            info.freeze_authority.as_deref() == Some("11111111111111111111111111111111");
        let authorities_safe = mint_authority_safe && freeze_authority_safe;

        // Get LP lock percentage (raw data, no thresholds)
        let max_lp_locked = info.markets
            .iter()
            .map(|m| m.lp_locked_pct)
            .fold(0.0, f64::max);
        let lp_safe = max_lp_locked > 0.0; // Just check if any LP is locked

        // Get holder concentration data (raw percentages, no thresholds)
        let top_holder_pct = info.top_holders
            .first()
            .map(|h| h.pct)
            .unwrap_or(0.0);
        let top_3_pct: f64 = info.top_holders
            .iter()
            .take(3)
            .map(|h| h.pct)
            .sum();
        let top_10_pct: f64 = info.top_holders
            .iter()
            .take(10)
            .map(|h| h.pct)
            .sum();
        let holders_safe = !info.top_holders.is_empty(); // Just check if we have holder data

        // Basic liquidity check (just verify data exists)
        let liquidity_adequate = info.total_market_liquidity > 0.0;

        // Collect danger-level risks from Rugcheck
        for risk in &info.risks {
            if risk.level == "danger" {
                risks.push(format!("{}: {}", risk.name, risk.description));
            }
        }

        // Determine risk level based on score only
        let risk_level = match info.score_normalised {
            70..=100 => RiskLevel::Safe,
            40..=69 => RiskLevel::Warning,
            0..=39 => RiskLevel::Danger,
            _ => RiskLevel::Unknown,
        };

        // Create summary
        let summary = format!(
            "Token analysis: score={}/100, LP={:.1}%, top_holder={:.1}%, liquidity=${:.0}",
            info.score_normalised,
            max_lp_locked,
            top_holder_pct,
            info.total_market_liquidity
        );

        SecurityAnalysis {
            score: info.score,
            score_normalized: info.score_normalised,
            risk_level,
            authorities_safe,
            lp_safe,
            holders_safe,
            liquidity_adequate,
            pump_fun_token,
            risks,
            summary,
        }
    }

    pub async fn get_security_stats(&self) -> Result<HashMap<String, i64>, String> {
        self.get_db()?
            .get_security_stats()
            .map_err(|e| format!("Failed to get security stats: {}", e))
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        if is_debug_security_enabled() {
            log(LogTag::Security, "CACHE_CLEAR", "Security cache cleared");
        }
    }

    pub async fn get_cached_holder_count(&self, mint: &str) -> Option<u32> {
        // First try in-memory cache
        {
            let cache = self.cache.read().await;
            if let Some(info) = cache.get(mint) {
                return Some(info.total_holders as u32);
            }
        }

        // Fall back to DB read (non-blocking to callers)
        if let Ok(db) = self.get_db() {
            if let Ok(Some(info)) = db.get_security_info(mint) {
                return Some(info.total_holders as u32);
            }
        }
        None
    }

    /// Lightweight cache-only security check for filtering: no API calls
    /// Returns Some(SecurityAnalysis) if cache or non-stale DB data exists, else None
    pub async fn analyze_token_cached_only(&self, mint: &str) -> Option<SecurityAnalysis> {
        // 1) In-memory cache fast path
        {
            let cache = self.cache.read().await;
            if let Some(info) = cache.get(mint) {
                // Record cache hit and analysis metrics for summary accuracy
                let analysis = self.calculate_security_analysis(info);
                return Some(analysis);
            }
        }

        // Record a cache miss when not found in memory

        // 2) Database non-stale path
        if let Ok(db) = self.get_db() {
            match db.get_security_info(mint) {
                Ok(Some(info)) => {
                    match db.is_stale(mint, MAX_CACHE_AGE_HOURS) {
                        Ok(false) => {
                            // Count DB hit
                            // Put into cache for next time
                            {
                                let mut cache = self.cache.write().await;
                                cache.insert(mint.to_string(), info.clone());
                            }
                            let analysis = self.calculate_security_analysis(&info);
                            return Some(analysis);
                        }
                        Ok(true) => {
                            // Stale counts as a miss for hit-rate visibility
                        }
                        Err(_) => {
                            // Error also treated as miss
                        }
                    }
                }
                Ok(None) => {
                    // Not found counts as miss
                }
                Err(_) => {
                    // Error counts as miss
                }
            }
        }

        None
    }

    /// Analyze token using ANY available security data (including stale) for filtering
    /// This is more inclusive than cached_only - uses any DB data we have, even if old
    /// Still avoids API calls for performance in filtering context
    /// Returns full SecurityAnalysis for authority checking
    pub async fn analyze_token_any_cached(&self, mint: &str) -> Option<SecurityAnalysis> {
        // First try the standard cache/fresh DB path
        if let Some(analysis) = self.analyze_token_cached_only(mint).await {
            return Some(analysis);
        }

        // If that fails, try ANY security data in DB, even if stale
        if let Ok(db) = self.get_db() {
            if let Ok(Some(info)) = db.get_security_info(mint) {
                // Use any available security data, regardless of age
                let analysis = self.calculate_security_analysis(&info);
                return Some(analysis);
            }
        }

        None
    }

    pub async fn get_security_summary(&self) -> SecuritySummary {
        let last_api_call = *self.metrics.last_api_call.read().await;

        let (safe, warning, danger, pump_fun, missing) = match self.get_db() {
            Ok(db) => {
                let safe = db.count_safe_tokens().unwrap_or(0) as u64;
                let warning = db.count_warning_tokens().unwrap_or(0) as u64;
                let danger = db.count_danger_tokens().unwrap_or(0) as u64;
                let pump_fun = db.count_pump_fun_tokens().unwrap_or(0) as u64;
                let missing = db.count_tokens_without_security().unwrap_or(0) as u64;
                (safe, warning, danger, pump_fun, missing)
            }
            Err(_) => (0, 0, 0, 0, 0),
        };

        SecuritySummary {
            api_calls_total: self.metrics.api_calls_total.load(Ordering::Relaxed),
            api_calls_success: self.metrics.api_calls_success.load(Ordering::Relaxed),
            api_calls_failed: self.metrics.api_calls_failed.load(Ordering::Relaxed),
            last_api_call,
            db_safe_tokens: safe,
            db_warning_tokens: warning,
            db_danger_tokens: danger,
            db_pump_fun_tokens: pump_fun,
            db_missing_tokens: missing,
        }
    }

    pub fn get_metrics(&self) -> Arc<SecurityMetrics> {
        self.metrics.clone()
    }
}

// Public interface for getting security analysis data
pub async fn get_token_risk_level(analyzer: &SecurityAnalyzer, mint: &str) -> RiskLevel {
    let analysis = analyzer.analyze_token(mint).await;
    analysis.risk_level
}

pub async fn get_token_security_analysis(
    analyzer: &SecurityAnalyzer,
    mint: &str
) -> SecurityAnalysis {
    analyzer.analyze_token(mint).await
}

// Global security analyzer singleton
static GLOBAL_SECURITY_ANALYZER: Lazy<Mutex<Option<Arc<SecurityAnalyzer>>>> = Lazy::new(||
    Mutex::new(None)
);

pub fn initialize_security_analyzer() -> Result<(), String> {
    let analyzer = Arc::new(SecurityAnalyzer::new("data/security.db")?);
    let mut global_analyzer = GLOBAL_SECURITY_ANALYZER.lock().map_err(|e|
        format!("Failed to lock global analyzer: {}", e)
    )?;
    *global_analyzer = Some(analyzer);
    log(LogTag::Security, "INIT", "Global security analyzer initialized");
    Ok(())
}

pub fn get_security_analyzer() -> Option<Arc<SecurityAnalyzer>> {
    let global_analyzer = GLOBAL_SECURITY_ANALYZER.lock().ok()?;
    global_analyzer.clone()
}

// Start background security monitoring task that fetches security info for unprocessed tokens
pub async fn start_security_monitoring(
    shutdown: Arc<Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    let analyzer = get_security_analyzer().ok_or_else(|| "Security analyzer not initialized")?;

    let handle = tokio::spawn(async move {
        log(LogTag::Security, "MONITOR_START", "Starting background security monitoring task");

        let mut interval = tokio::time::interval(Duration::from_secs(10)); // Check every 10 seconds
        let mut last_fetch = Instant::now() - Duration::from_secs(600); // Start immediately

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Security, "MONITOR_SHUTDOWN", "Security monitoring task shutting down");
                    break;
                }
                _ = interval.tick() => {
                    // Only fetch if enough time has passed (rate limiting)
                    if last_fetch.elapsed() >= Duration::from_millis(500) {
                        // Get tokens that need security analysis
                        match analyzer.get_db() {
                            Ok(db) => {
                                match db.get_tokens_without_security() {
                                    Ok(tokens) => {
                                        if !tokens.is_empty() {
                                            log(LogTag::Security, "MONITOR_BATCH",
                                                &format!("Found {} tokens without security info, processing batch", tokens.len()));

                                            // Process tokens one by one to respect rate limits
                                            for (i, mint) in tokens.iter().enumerate() {
                                                tokio::select! {
                                                    _ = shutdown.notified() => {
                                                        log(LogTag::Security, "MONITOR_SHUTDOWN",
                                                            &format!("Shutdown during batch processing at token {}/{}", i+1, tokens.len()));
                                                        return;
                                                    }
                                                    _ = async {
                                                        // Fetch security info for this token (uses full analyze_token with API calls)
                                                        let analysis = analyzer.analyze_token(mint).await;

                                                        if is_debug_security_enabled() {
                                                            log(LogTag::Security, "MONITOR_TOKEN",
                                                                &format!("Processed {} → risk_level: {:?}, score: {}",
                                                                mint, analysis.risk_level, analysis.score));
                                                        }

                                                        // Rate limit: wait 500ms between requests
                                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                                    } => {}
                                                }
                                            }

                                            last_fetch = Instant::now();
                                            log(LogTag::Security, "MONITOR_COMPLETE",
                                                &format!("Completed processing batch of {} tokens", tokens.len()));
                                        } else {
                                            // No tokens to process - less frequent logging
                                            if last_fetch.elapsed() >= Duration::from_secs(300) { // Log every 5 minutes if nothing to do
                                                log(LogTag::Security, "MONITOR_IDLE", "All tokens have security info - monitoring idle");
                                                last_fetch = Instant::now();
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log(LogTag::Security, "MONITOR_ERROR",
                                            &format!("Failed to get tokens without security: {}", e));
                                        // Back off on error
                                        tokio::time::sleep(Duration::from_secs(30)).await;
                                    }
                                }
                            }
                            Err(e) => {
                                log(LogTag::Security, "MONITOR_ERROR",
                                    &format!("Failed to open security database: {}", e));
                                // Back off on error
                                tokio::time::sleep(Duration::from_secs(30)).await;
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(handle)
}

// Start periodic security summary reporting
pub fn start_security_summary_task() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            if let Some(analyzer) = get_security_analyzer() {
                let summary = analyzer.get_security_summary().await;
                log_security_summary(&summary);
            }
        }
    });
    log(LogTag::Security, "SUMMARY_TASK", "Started 30-second security summary task");
}

fn log_security_summary(summary: &SecuritySummary) {
    let api_success_rate = if summary.api_calls_total > 0 {
        ((summary.api_calls_success as f64) / (summary.api_calls_total as f64)) * 100.0
    } else {
        0.0
    };

    let last_api = if let Some(instant) = summary.last_api_call {
        format!("{:.1}s ago", instant.elapsed().as_secs_f64())
    } else {
        "never".to_string()
    };

    let total_with_data =
        summary.db_safe_tokens + summary.db_warning_tokens + summary.db_danger_tokens;

    let header_line = "════════════════════════════════════════════════════════════";
    let title = "🔐 SECURITY DATABASE STATE";

    let api_line = format!(
        "  • API       📡  calls={}, ok={} ({:.1}%), fail={}, last={}",
        summary.api_calls_total,
        summary.api_calls_success,
        api_success_rate,
        summary.api_calls_failed,
        last_api
    );

    let tokens_line = format!(
        "  • Tokens     safe={}, warning={}, danger={}, missing={}, pump.fun={}",
        summary.db_safe_tokens,
        summary.db_warning_tokens,
        summary.db_danger_tokens,
        summary.db_missing_tokens,
        summary.db_pump_fun_tokens
    );

    let body = format!(
        "\n{header}\n{title}\n{header}\n{api}\n{tokens}\n{header}",
        header = header_line,
        title = title,
        api = api_line,
        tokens = tokens_line
    );

    log(LogTag::Security, "MONITOR", &body);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_analyzer() {
        let analyzer = SecurityAnalyzer::new(":memory:").unwrap();

        // Test with a known token
        let analysis = analyzer.analyze_token("So11111111111111111111111111111111111111112").await;
        println!("SOL analysis: {:?}", analysis);
    }
}
