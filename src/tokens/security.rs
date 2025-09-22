use crate::logger::{ log, LogTag };
use crate::arguments::is_debug_security_enabled;
use crate::tokens::security_db::{ SecurityDatabase, SecurityInfo, parse_rugcheck_response };
use once_cell::sync::Lazy;
use reqwest::{ Client, StatusCode };
use std::collections::{ HashMap, HashSet };
use std::sync::{ Arc, Mutex };
use std::sync::atomic::{ AtomicU64, Ordering };
use tokio::sync::{ RwLock, Notify };
use tokio::time::{ sleep, Duration, Instant };

const RUGCHECK_API_BASE: &str = "https://api.rugcheck.xyz/v1/tokens";
const MAX_CACHE_AGE_HOURS: i64 = 24; // Cache security data for 24 hours

// Normalize raw risk strings into concise categories for aggregation
fn normalize_reason(reason: &str) -> String {
    let r = reason.to_lowercase();
    if
        r.contains("authorit") ||
        r.contains("not revoked") ||
        r.contains("mint authority") ||
        r.contains("freeze authority")
    {
        return "Authorities not revoked".to_string();
    }
    if
        r.contains("lp") &&
        (r.contains("not") ||
            r.contains("unlock") ||
            r.contains("lock") ||
            r.contains("locked") ||
            r.contains("burn"))
    {
        return "LP not locked/burned".to_string();
    }
    if
        r.contains("holder") ||
        r.contains("concentration") ||
        r.contains("top10") ||
        r.contains("top1")
    {
        return "High holder concentration".to_string();
    }
    if r.contains("liquidity") || r.contains("low liquidity") {
        return "Low liquidity".to_string();
    }
    if r.contains("pump") {
        return "Pump.fun risk".to_string();
    }
    if r.contains("blacklist") {
        return "Blacklisted".to_string();
    }
    if r.contains("danger") || r.contains("risk") {
        return "Rugcheck risk: danger".to_string();
    }
    // Compact long strings
    reason.chars().take(64).collect()
}

#[derive(Debug, Default)]
pub struct SecurityMetrics {
    pub api_calls_total: AtomicU64,
    pub api_calls_success: AtomicU64,
    pub api_calls_failed: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub db_hits: AtomicU64,
    pub db_misses: AtomicU64,
    pub tokens_analyzed: AtomicU64,
    pub tokens_safe: AtomicU64,
    pub tokens_unsafe: AtomicU64,
    pub tokens_unknown: AtomicU64,
    pub pump_fun_tokens: AtomicU64,
    pub last_api_call: Arc<RwLock<Option<Instant>>>,
    // Aggregated rejection reasons for unsafe classifications
    pub rejection_reasons: Arc<Mutex<HashMap<String, u64>>>,
}

impl SecurityMetrics {
    pub fn new() -> Self {
        Self {
            api_calls_total: AtomicU64::new(0),
            api_calls_success: AtomicU64::new(0),
            api_calls_failed: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            db_hits: AtomicU64::new(0),
            db_misses: AtomicU64::new(0),
            tokens_analyzed: AtomicU64::new(0),
            tokens_safe: AtomicU64::new(0),
            tokens_unsafe: AtomicU64::new(0),
            tokens_unknown: AtomicU64::new(0),
            pump_fun_tokens: AtomicU64::new(0),
            last_api_call: Arc::new(RwLock::new(None)),
            rejection_reasons: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_db_hit(&self) {
        self.db_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_db_miss(&self) {
        self.db_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_analysis(&self, analysis: &SecurityAnalysis) {
        self.tokens_analyzed.fetch_add(1, Ordering::Relaxed);
        if analysis.is_safe {
            self.tokens_safe.fetch_add(1, Ordering::Relaxed);
        } else if analysis.risk_level == RiskLevel::Unknown {
            self.tokens_unknown.fetch_add(1, Ordering::Relaxed);
        } else {
            self.tokens_unsafe.fetch_add(1, Ordering::Relaxed);
            // Aggregate rejection reasons per token (deduplicated categories per token)
            let mut categories: HashSet<String> = HashSet::new();
            for r in &analysis.risks {
                categories.insert(normalize_reason(r));
            }
            // Consider Pump.fun nature as a reason category
            if analysis.pump_fun_token {
                categories.insert("Pump.fun risk".to_string());
            }
            // If still empty but token is unsafe, attribute to low score
            if categories.is_empty() {
                categories.insert("Low Rugcheck score".to_string());
            }

            let mut map = self.rejection_reasons.lock().unwrap_or_else(|e| e.into_inner());
            for cat in categories {
                *map.entry(cat).or_insert(0) += 1;
            }
        }
        if analysis.pump_fun_token {
            self.pump_fun_tokens.fetch_add(1, Ordering::Relaxed);
        }
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
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub db_hits: u64,
    pub db_misses: u64,
    pub tokens_analyzed: u64,
    pub tokens_safe: u64,
    pub tokens_unsafe: u64,
    pub tokens_unknown: u64,
    pub pump_fun_tokens: u64,
    pub cache_size: u64,
    pub db_total_tokens: u64,
    pub db_safe_tokens: u64,
    pub db_high_score_tokens: u64,
    pub last_api_call: Option<Instant>,
    pub top_rejection_reasons: Vec<(String, u64)>,
    pub db_unprocessed_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct SecurityAnalysis {
    pub is_safe: bool,
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
                self.metrics.record_cache_hit();
                if is_debug_security_enabled() {
                    log(
                        LogTag::Security,
                        "CACHE_HIT",
                        &format!("Using cached security data for mint={}", mint)
                    );
                }
                let analysis = self.calculate_security_analysis(info);
                self.metrics.record_analysis(&analysis);
                // One compact per-token log for full analysis paths (cache hit)
                log(
                    LogTag::Security,
                    "ANALYSIS",
                    &format!(
                        "mint={} safe={} score={} risks={} pump_fun={} source=cache",
                        mint,
                        analysis.is_safe,
                        analysis.score_normalized,
                        analysis.risks.len(),
                        analysis.pump_fun_token
                    )
                );
                return analysis;
            }
        }
        self.metrics.record_cache_miss();

        // Try to get from database (single connection for read + staleness check)
        if let Ok(db) = self.get_db() {
            match db.get_security_info(mint) {
                Ok(Some(info)) => {
                    match db.is_stale(mint, MAX_CACHE_AGE_HOURS) {
                        Ok(false) => {
                            self.metrics.record_db_hit();
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
                            self.metrics.record_analysis(&analysis);
                            log(
                                LogTag::Security,
                                "ANALYSIS",
                                &format!(
                                    "mint={} safe={} score={} risks={} pump_fun={} source=db",
                                    mint,
                                    analysis.is_safe,
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
                    self.metrics.record_db_miss();
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
                self.metrics.record_analysis(&analysis);
                log(
                    LogTag::Security,
                    "ANALYSIS",
                    &format!(
                        "mint={} safe={} score={} risks={} pump_fun={} source=api",
                        mint,
                        analysis.is_safe,
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
                    is_safe: false,
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
                self.metrics.record_analysis(&analysis);
                log(
                    LogTag::Security,
                    "ANALYSIS",
                    &format!(
                        "mint={} safe={} score={} risks={} pump_fun={} source=error",
                        mint,
                        analysis.is_safe,
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

    // Pure calculation function - no database/API calls, just runtime analysis
    fn calculate_security_analysis(&self, info: &SecurityInfo) -> SecurityAnalysis {
        let mut risks = Vec::new();
        let mut is_safe = true;

        // Check if it's a Pump.Fun token
        let pump_fun_token = info.markets
            .iter()
            .any(
                |m|
                    m.market_type.to_lowercase().contains("pump_fun") ||
                    m.market_type.to_lowercase().contains("pump.fun")
            );

        // Analyze authorities
        let mint_authority_safe =
            info.mint_authority.is_none() ||
            info.mint_authority.as_deref() == Some("11111111111111111111111111111111");
        let freeze_authority_safe =
            info.freeze_authority.is_none() ||
            info.freeze_authority.as_deref() == Some("11111111111111111111111111111111");
        let authorities_safe = mint_authority_safe && freeze_authority_safe;

        if !authorities_safe {
            risks.push("Token authorities not revoked".to_string());
            is_safe = false;
        }

        // Analyze LP safety
        let lp_safe = self.analyze_lp_safety(info, pump_fun_token);
        if !lp_safe {
            risks.push("LP not adequately locked".to_string());
            is_safe = false;
        }

        // Analyze holder distribution
        let holders_safe = self.analyze_holder_safety(info);
        if !holders_safe {
            risks.push("Concerning holder concentration".to_string());
            is_safe = false;
        }

        // Check liquidity - lowered threshold for more inclusion
        let liquidity_adequate = info.total_market_liquidity >= 500.0; // $500 minimum (was $1000)
        if !liquidity_adequate {
            risks.push(format!("Low liquidity: ${:.2}", info.total_market_liquidity));
            is_safe = false;
        }

        // Add specific risks from Rugcheck
        for risk in &info.risks {
            if risk.level == "danger" {
                risks.push(format!("{}: {}", risk.name, risk.description));
                is_safe = false;
            }
        }

        // Determine risk level - more realistic thresholds
        let risk_level = match info.score_normalised {
            60..=100 => RiskLevel::Safe, // Lowered from 70 to 60
            35..=59 => RiskLevel::Warning, // Lowered from 40 to 35
            0..=34 => RiskLevel::Danger, // Lowered from 39 to 34
            _ => RiskLevel::Unknown,
        };

        // Override safety based on risk level - only fail on Danger, not Warning
        if risk_level == RiskLevel::Danger {
            is_safe = false;
        }

        // Create summary
        let summary = if is_safe {
            format!("Safe token (score: {}/100)", info.score_normalised)
        } else {
            format!("Risky token (score: {}/100, {} risks)", info.score_normalised, risks.len())
        };

        SecurityAnalysis {
            is_safe,
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

    fn analyze_lp_safety(&self, info: &SecurityInfo, is_pump_fun: bool) -> bool {
        if info.markets.is_empty() {
            return false;
        }

        // For Pump.Fun tokens, check LP locked percentage directly
        if is_pump_fun {
            for market in &info.markets {
                if market.lp_locked_pct >= 100.0 {
                    if is_debug_security_enabled() {
                        log(
                            LogTag::Security,
                            "LP_PUMP_SAFE",
                            &format!(
                                "Pump.Fun LP verified as safe: locked_pct={:.2}%, mint={}",
                                market.lp_locked_pct,
                                info.mint
                            )
                        );
                    }
                    return true;
                }
            }
            if is_debug_security_enabled() {
                log(
                    LogTag::Security,
                    "LP_PUMP_UNSAFE",
                    &format!(
                        "Pump.Fun LP not fully locked: max_locked={:.2}%, mint={}",
                        info.markets
                            .iter()
                            .map(|m| m.lp_locked_pct)
                            .fold(0.0, f64::max),
                        info.mint
                    )
                );
            }
            return false;
        }

        // For established tokens with large liquidity, be more lenient
        let max_lp_locked = info.markets
            .iter()
            .map(|m| m.lp_locked_pct)
            .fold(0.0, f64::max);

        // More realistic thresholds based on liquidity and market maturity
        let is_safe = if info.total_market_liquidity >= 50_000_000.0 {
            // Large established tokens (>$50M liquidity) - very lenient
            max_lp_locked >= 10.0 || info.score_normalised >= 60
        } else if info.total_market_liquidity >= 5_000_000.0 {
            // Medium tokens ($5-50M liquidity) - moderate requirement
            max_lp_locked >= 25.0 || info.score_normalised >= 65
        } else {
            // Smaller tokens (<$5M liquidity) - stricter requirement
            max_lp_locked >= 50.0
        };

        if is_debug_security_enabled() {
            log(
                LogTag::Security,
                "LP_CHECK",
                &format!(
                    "LP safety check: max_locked={:.2}%, liquidity=${:.0}, safe={}, mint={}",
                    max_lp_locked,
                    info.total_market_liquidity,
                    is_safe,
                    info.mint
                )
            );
        }

        is_safe
    }

    fn analyze_holder_safety(&self, info: &SecurityInfo) -> bool {
        if info.top_holders.is_empty() {
            // For tokens with good scores but no holder data, be lenient
            return info.score_normalised >= 60;
        }

        // Check top holder concentration
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

        // More realistic thresholds based on market maturity
        let is_safe = if info.total_market_liquidity >= 10_000_000.0 {
            // Large tokens - more lenient concentration limits
            top_holder_pct < 70.0 && top_3_pct < 85.0 && top_10_pct < 95.0
        } else {
            // Smaller tokens - standard concentration limits
            top_holder_pct < 60.0 && top_3_pct < 80.0 && top_10_pct < 92.0
        };

        if is_debug_security_enabled() {
            log(
                LogTag::Security,
                "HOLDER_CHECK",
                &format!(
                    "Holder distribution: top1={:.2}%, top3={:.2}%, top10={:.2}%, liquidity=${:.0}, safe={}, mint={}",
                    top_holder_pct,
                    top_3_pct,
                    top_10_pct,
                    info.total_market_liquidity,
                    is_safe,
                    info.mint
                )
            );
        }

        is_safe
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
    /// Returns Some(is_safe) if cache or non-stale DB data exists, else None
    pub async fn analyze_token_cached_only(&self, mint: &str) -> Option<bool> {
        // 1) In-memory cache fast path
        {
            let cache = self.cache.read().await;
            if let Some(info) = cache.get(mint) {
                // Record cache hit and analysis metrics for summary accuracy
                self.metrics.record_cache_hit();
                let analysis = self.calculate_security_analysis(info);
                self.metrics.record_analysis(&analysis);
                return Some(analysis.is_safe);
            }
        }

        // Record a cache miss when not found in memory
        self.metrics.record_cache_miss();

        // 2) Database non-stale path
        if let Ok(db) = self.get_db() {
            match db.get_security_info(mint) {
                Ok(Some(info)) => {
                    match db.is_stale(mint, MAX_CACHE_AGE_HOURS) {
                        Ok(false) => {
                            // Count DB hit
                            self.metrics.record_db_hit();
                            // Put into cache for next time
                            {
                                let mut cache = self.cache.write().await;
                                cache.insert(mint.to_string(), info.clone());
                            }
                            let analysis = self.calculate_security_analysis(&info);
                            self.metrics.record_analysis(&analysis);
                            return Some(analysis.is_safe);
                        }
                        Ok(true) => {
                            // Stale counts as a miss for hit-rate visibility
                            self.metrics.record_db_miss();
                        }
                        Err(_) => {
                            // Error also treated as miss
                            self.metrics.record_db_miss();
                        }
                    }
                }
                Ok(None) => {
                    // Not found counts as miss
                    self.metrics.record_db_miss();
                }
                Err(_) => {
                    // Error counts as miss
                    self.metrics.record_db_miss();
                }
            }
        }

        None
    }

    /// Analyze token using ANY available security data (including stale) for filtering
    /// This is more inclusive than cached_only - uses any DB data we have, even if old
    /// Still avoids API calls for performance in filtering context
    pub async fn analyze_token_any_cached(&self, mint: &str) -> Option<bool> {
        // First try the standard cache/fresh DB path
        if let Some(is_safe) = self.analyze_token_cached_only(mint).await {
            return Some(is_safe);
        }

        // If that fails, try ANY security data in DB, even if stale
        if let Ok(db) = self.get_db() {
            if let Ok(Some(info)) = db.get_security_info(mint) {
                // Use any available security data, regardless of age
                let analysis = self.calculate_security_analysis(&info);
                self.metrics.record_analysis(&analysis);
                return Some(analysis.is_safe);
            }
        }

        None
    }

    pub async fn get_security_summary(&self) -> SecuritySummary {
        let cache_size = self.cache.read().await.len();
        let last_api_call = *self.metrics.last_api_call.read().await;

        let db_stats = match self.get_security_stats().await {
            Ok(stats) => stats,
            Err(_) => HashMap::new(),
        };

        // Count tokens present in tokens.db missing security_info (unprocessed)
        let db_unprocessed_tokens: u64 = self
            .get_db()
            .ok()
            .and_then(|db| db.count_tokens_without_security().ok())
            .unwrap_or(0) as u64;

        // Prepare top rejection reasons snapshot (top 5)
        let top_rejection_reasons = {
            let mut pairs: Vec<(String, u64)> = self.metrics.rejection_reasons
                .lock()
                .map(|m| m.clone())
                .unwrap_or_default()
                .into_iter()
                .collect();
            pairs.sort_by(|a, b| b.1.cmp(&a.1));
            pairs.truncate(5);
            pairs
        };

        SecuritySummary {
            api_calls_total: self.metrics.api_calls_total.load(Ordering::Relaxed),
            api_calls_success: self.metrics.api_calls_success.load(Ordering::Relaxed),
            api_calls_failed: self.metrics.api_calls_failed.load(Ordering::Relaxed),
            cache_hits: self.metrics.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.metrics.cache_misses.load(Ordering::Relaxed),
            db_hits: self.metrics.db_hits.load(Ordering::Relaxed),
            db_misses: self.metrics.db_misses.load(Ordering::Relaxed),
            tokens_analyzed: self.metrics.tokens_analyzed.load(Ordering::Relaxed),
            tokens_safe: self.metrics.tokens_safe.load(Ordering::Relaxed),
            tokens_unsafe: self.metrics.tokens_unsafe.load(Ordering::Relaxed),
            tokens_unknown: self.metrics.tokens_unknown.load(Ordering::Relaxed),
            pump_fun_tokens: self.metrics.pump_fun_tokens.load(Ordering::Relaxed),
            cache_size: cache_size as u64,
            db_total_tokens: db_stats.get("total").copied().unwrap_or(0) as u64,
            db_safe_tokens: db_stats.get("safe").copied().unwrap_or(0) as u64,
            db_high_score_tokens: db_stats.get("high_score").copied().unwrap_or(0) as u64,
            last_api_call,
            top_rejection_reasons,
            db_unprocessed_tokens,
        }
    }

    pub fn get_metrics(&self) -> Arc<SecurityMetrics> {
        self.metrics.clone()
    }
}

// Simplified public interface for token filtering
pub async fn is_token_safe(analyzer: &SecurityAnalyzer, mint: &str) -> bool {
    let analysis = analyzer.analyze_token(mint).await;
    analysis.is_safe
}

pub async fn get_token_risk_level(analyzer: &SecurityAnalyzer, mint: &str) -> RiskLevel {
    let analysis = analyzer.analyze_token(mint).await;
    analysis.risk_level
}

// Global security analyzer singleton
static GLOBAL_SECURITY_ANALYZER: Lazy<Mutex<Option<Arc<SecurityAnalyzer>>>> = Lazy::new(|| {
    Mutex::new(None)
});

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
                                                                &format!("Processed {} → safe: {}, score: {}", 
                                                                mint, analysis.is_safe, analysis.score));
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
    let cache_hit_rate = if summary.cache_hits + summary.cache_misses > 0 {
        ((summary.cache_hits as f64) / ((summary.cache_hits + summary.cache_misses) as f64)) * 100.0
    } else {
        0.0
    };

    let db_hit_rate = if summary.db_hits + summary.db_misses > 0 {
        ((summary.db_hits as f64) / ((summary.db_hits + summary.db_misses) as f64)) * 100.0
    } else {
        0.0
    };

    let api_success_rate = if summary.api_calls_total > 0 {
        ((summary.api_calls_success as f64) / (summary.api_calls_total as f64)) * 100.0
    } else {
        0.0
    };

    let safe_percentage = if summary.tokens_analyzed > 0 {
        ((summary.tokens_safe as f64) / (summary.tokens_analyzed as f64)) * 100.0
    } else {
        0.0
    };

    let last_api = if let Some(instant) = summary.last_api_call {
        format!("{:.1}s ago", instant.elapsed().as_secs_f64())
    } else {
        "never".to_string()
    };

    // Compact summary: minimal emojis (header/footer + safety), concise single-line sections.
    let reasons_inline = {
        if summary.top_rejection_reasons.is_empty() {
            "-".to_string()
        } else {
            let mut parts: Vec<String> = Vec::new();
            for (idx, (reason, count)) in summary.top_rejection_reasons.iter().take(3).enumerate() {
                let pct = if summary.tokens_unsafe > 0 {
                    ((*count as f64) / (summary.tokens_unsafe as f64)) * 100.0
                } else {
                    0.0
                };
                parts.push(format!("{}. {} — {} ({:.1}%)", idx + 1, reason, count, pct));
            }
            parts.join(" | ")
        }
    };

    log(
        LogTag::Security,
        "MONITOR",
        &format!(
            "\n🔐 Security (30s)\n\
             SAFETY: 🛡️ {safe_cnt} ({safe_pct:.1}%) | ⛔ {unsafe_cnt} | ❓ {unknown_cnt}\n\
             API: calls={api_total}, ok={api_ok} ({api_pct:.1}%), fail={api_fail}, last={last_api}\n\
             CACHE: {cache_hits}/{cache_total} ({cache_rate:.1}%), size={cache_size} | DB: {db_hits}/{db_total} ({db_rate:.1}%), stored={db_tokens_safe}/{db_tokens_total}, unproc={db_unprocessed}\n\
             MARKET: pump.fun={pump_cnt}, high_score={high_score_cnt}\n\
             REASONS: {reasons}\n\
             🔐",
            safe_cnt = summary.tokens_safe,
            safe_pct = safe_percentage,
            unsafe_cnt = summary.tokens_unsafe,
            unknown_cnt = summary.tokens_unknown,
            api_total = summary.api_calls_total,
            api_ok = summary.api_calls_success,
            api_pct = api_success_rate,
            api_fail = summary.api_calls_failed,
            last_api = last_api,
            cache_hits = summary.cache_hits,
            cache_total = summary.cache_hits + summary.cache_misses,
            cache_rate = cache_hit_rate,
            cache_size = summary.cache_size,
            db_hits = summary.db_hits,
            db_total = summary.db_hits + summary.db_misses,
            db_rate = db_hit_rate,
            db_tokens_total = summary.db_total_tokens,
            db_tokens_safe = summary.db_safe_tokens,
            db_unprocessed = summary.db_unprocessed_tokens,
            pump_cnt = summary.pump_fun_tokens,
            high_score_cnt = summary.db_high_score_tokens,
            reasons = reasons_inline
        )
    );
}

fn format_top_reasons(top: &[(String, u64)], total_unsafe: u64) -> String {
    if top.is_empty() {
        return "(no unsafe tokens recorded)".to_string();
    }
    let mut out = String::new();
    for (i, (reason, count)) in top.iter().enumerate() {
        let pct = if total_unsafe > 0 {
            ((*count as f64) / (total_unsafe as f64)) * 100.0
        } else {
            0.0
        };
        // Indented bullet points
        out.push_str(&format!("   {}. {} — {} ({:.1}%)\n", i + 1, reason, count, pct));
    }
    out.trim_end().to_string()
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
