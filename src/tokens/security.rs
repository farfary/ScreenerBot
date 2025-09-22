use crate::logger::{ log, LogTag };
use crate::tokens::security_db::{ SecurityDatabase, SecurityInfo, parse_rugcheck_response };
use once_cell::sync::Lazy;
use reqwest::{ Client, StatusCode };
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::sync::atomic::{ AtomicU64, Ordering };
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration, Instant };

const RUGCHECK_API_BASE: &str = "https://api.rugcheck.xyz/v1/tokens";
const MAX_CACHE_AGE_HOURS: i64 = 24; // Cache security data for 24 hours

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
}

impl SecurityMetrics {
    pub fn new() -> Self {
        Self::default()
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

    pub async fn analyze_token(&self, mint: &str) -> SecurityAnalysis {
        log(LogTag::Security, "ANALYZE", &format!("Starting security analysis for mint={}", mint));

        // Try to get from cache first
        {
            let cache = self.cache.read().await;
            if let Some(info) = cache.get(mint) {
                self.metrics.record_cache_hit();
                log(
                    LogTag::Security,
                    "CACHE_HIT",
                    &format!("Using cached security data for mint={}", mint)
                );
                let analysis = self.calculate_security_analysis(info);
                self.metrics.record_analysis(&analysis);
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
                            log(
                                LogTag::Security,
                                "DB_HIT",
                                &format!("Using fresh database security data for mint={}", mint)
                            );
                            // Add to cache
                            {
                                let mut cache = self.cache.write().await;
                                cache.insert(mint.to_string(), info.clone());
                            }
                            let analysis = self.calculate_security_analysis(&info);
                            self.metrics.record_analysis(&analysis);
                            return analysis;
                        }
                        Ok(true) => {
                            log(
                                LogTag::Security,
                                "DB_STALE",
                                &format!("Database security data is stale for mint={}, refreshing", mint)
                            );
                        }
                        Err(e) => {
                            log(
                                LogTag::Security,
                                "DB_ERROR",
                                &format!("Error checking staleness for mint={}: {}", mint, e)
                            );
                        }
                    }
                }
                Ok(None) => {
                    self.metrics.record_db_miss();
                    log(
                        LogTag::Security,
                        "DB_MISS",
                        &format!("No security data in database for mint={}", mint)
                    );
                }
                Err(e) => {
                    log(
                        LogTag::Security,
                        "DB_ERROR",
                        &format!("Database error for mint={}: {}", mint, e)
                    );
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
                    log(
                        LogTag::Security,
                        "DB_STORE_ERROR",
                        &format!("Failed to store security data for mint={}: {}", mint, e)
                    );
                }

                // Add to cache
                {
                    let mut cache = self.cache.write().await;
                    cache.insert(mint.to_string(), info.clone());
                }

                let analysis = self.calculate_security_analysis(&info);
                self.metrics.record_analysis(&analysis);
                analysis
            }
            Err(e) => {
                self.metrics.record_api_call(false).await;
                log(
                    LogTag::Security,
                    "API_ERROR",
                    &format!("Failed to fetch security data for mint={}: {}", mint, e)
                );
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
                analysis
            }
        }
    }

    async fn fetch_rugcheck_data(&self, mint: &str) -> Result<SecurityInfo, String> {
        log(LogTag::Security, "API_FETCH", &format!("Fetching Rugcheck data for mint={}", mint));

        let url = format!("{}/{}/report", RUGCHECK_API_BASE, mint);

        // Simple exponential backoff with jitter for transient failures and rate limits
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

                    // For rate limit or server errors, retry with backoff
                    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                        let base_delay_ms = 250u64 * (1u64 << (attempt - 1));
                        let delay = Duration::from_millis(base_delay_ms);
                        log(
                            LogTag::Security,
                            "API_RETRY",
                            &format!(
                                "Rugcheck status {} for mint={}, retrying in {}ms (attempt {}/{})",
                                status,
                                mint,
                                delay.as_millis(),
                                attempt,
                                max_attempts
                            )
                        );
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

        log(
            LogTag::Security,
            "ANALYSIS",
            &format!(
                "Security analysis complete for mint={}: safe={}, score={}, risks={}, pump_fun={}",
                info.mint,
                is_safe,
                info.score_normalised,
                risks.len(),
                pump_fun_token
            )
        );

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
                    log(
                        LogTag::Security,
                        "LP_PUMP_SAFE",
                        &format!(
                            "Pump.Fun LP verified as safe: locked_pct={:.2}%, mint={}",
                            market.lp_locked_pct,
                            info.mint
                        )
                    );
                    return true;
                }
            }
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
        log(LogTag::Security, "CACHE_CLEAR", "Security cache cleared");
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

    pub async fn get_security_summary(&self) -> SecuritySummary {
        let cache_size = self.cache.read().await.len();
        let last_api_call = *self.metrics.last_api_call.read().await;

        let db_stats = match self.get_security_stats().await {
            Ok(stats) => stats,
            Err(_) => HashMap::new(),
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

    // Create status indicators with emojis
    let api_status_icon = if api_success_rate >= 90.0 {
        "🟢"
    } else if api_success_rate >= 70.0 {
        "🟡"
    } else {
        "🔴"
    };

    let cache_status_icon = if cache_hit_rate >= 80.0 {
        "⚡"
    } else if cache_hit_rate >= 50.0 {
        "🟡"
    } else {
        "🔴"
    };

    let db_status_icon = if db_hit_rate >= 70.0 {
        "💾"
    } else if db_hit_rate >= 40.0 {
        "🟡"
    } else {
        "🔴"
    };

    let safety_status_icon = if safe_percentage >= 50.0 {
        "🛡️"
    } else if safe_percentage >= 20.0 {
        "⚠️"
    } else {
        "⛔"
    };

    // Create formatted sections without table structure
    log(
        LogTag::Security,
        "MONITOR",
        &format!(
            "\n🔐 ════════════════════════════════════════════════════════════════════════\n\
             🔐 SECURITY ANALYZER STATUS - 30 Second Summary Report\n\
             🔐 ════════════════════════════════════════════════════════════════════════\n\
             \n\
             {} API PERFORMANCE:\n\
             📞 Total Calls: {} | ✅ Success: {} ({:.1}%) | ❌ Failed: {} | ⏰ Last: {}\n\
             \n\
             {} CACHE PERFORMANCE:\n\
             ⚡ Hits: {}/{} ({:.1}% hit rate) | � Cache Size: {} tokens\n\
             \n\
             {} DATABASE PERFORMANCE:\n\
             💾 Hits: {}/{} ({:.1}% hit rate) | 📊 Total: {} tokens | 🟢 Safe: {} tokens\n\
             \n\
             {} TOKEN SAFETY OVERVIEW:\n\
             🛡️ Safe: {} tokens ({:.1}%) | ⛔ Unsafe: {} tokens | ❓ Unknown: {} tokens\n\
             🔄 Pump.Fun: {} tokens | 📈 High Score: {} tokens\n\
             \n\
             � SESSION SUMMARY:\n\
             ├─ Total Analyzed: {} tokens\n\
             ├─ Safety Rate: {:.1}% classified as safe\n\
             ├─ API Success Rate: {:.1}% ({} successful calls)\n\
             ├─ Cache Efficiency: {:.1}% hit rate ({})\n\
             └─ Database Coverage: {}/{} tokens stored\n\
             \n\
             🔐 ════════════════════════════════════════════════════════════════════════",
            api_status_icon,
            summary.api_calls_total,
            summary.api_calls_success,
            api_success_rate,
            summary.api_calls_failed,
            last_api,
            cache_status_icon,
            summary.cache_hits,
            summary.cache_hits + summary.cache_misses,
            cache_hit_rate,
            summary.cache_size,
            db_status_icon,
            summary.db_hits,
            summary.db_hits + summary.db_misses,
            db_hit_rate,
            summary.db_total_tokens,
            summary.db_safe_tokens,
            safety_status_icon,
            summary.tokens_safe,
            safe_percentage,
            summary.tokens_unsafe,
            summary.tokens_unknown,
            summary.pump_fun_tokens,
            summary.db_high_score_tokens,
            summary.tokens_analyzed,
            safe_percentage,
            api_success_rate,
            summary.api_calls_success,
            cache_hit_rate,
            if cache_hit_rate >= 80.0 {
                "EXCELLENT"
            } else if cache_hit_rate >= 50.0 {
                "GOOD"
            } else {
                "NEEDS IMPROVEMENT"
            },
            summary.db_safe_tokens,
            summary.db_total_tokens
        )
    );
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
