/// Async Rugcheck Integration for ScreenerBot Filtering
///
/// This module provides async-compatible rugcheck validation that can be
/// integrated with the main trading loop without blocking operations.

use crate::logger::{ log, LogTag };
use crate::tokens::{
    get_token_rugcheck_risk_assessment,
    RugcheckRiskLevel,
    RugcheckRiskAssessment,
    Token,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::{ DateTime, Utc, Duration };

// ===== RUGCHECK CACHE FOR SYNC ACCESS =====

/// Cached rugcheck result for sync access
#[derive(Debug, Clone)]
pub struct CachedRugcheckResult {
    pub should_filter: bool,
    pub risk_level: Option<RugcheckRiskLevel>,
    pub risk_reasons: Vec<String>,
    pub cached_at: DateTime<Utc>,
}

/// Global rugcheck cache for fast sync access
static RUGCHECK_CACHE: once_cell::sync::Lazy<
    Arc<RwLock<HashMap<String, CachedRugcheckResult>>>
> = once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Cache expiration time (10 minutes)
const CACHE_EXPIRATION_MINUTES: i64 = 10;

// ===== SYNC RUGCHECK FUNCTIONS FOR FILTERING =====

/// Get cached rugcheck result for sync filtering (non-blocking)
pub fn get_cached_rugcheck_result(mint: &str) -> Option<CachedRugcheckResult> {
    // Try to get from cache without blocking
    if let Ok(cache) = RUGCHECK_CACHE.try_read() {
        if let Some(result) = cache.get(mint) {
            // Check if cache is still valid
            let age = Utc::now() - result.cached_at;
            if age < chrono::Duration::minutes(CACHE_EXPIRATION_MINUTES) {
                return Some(result.clone());
            }
        }
    }
    None
}

/// Check if token should be filtered based on cached rugcheck data
pub fn should_filter_token_cached_rugcheck(mint: &str) -> Option<bool> {
    get_cached_rugcheck_result(mint).map(|result| result.should_filter)
}

/// Update rugcheck cache with new result
pub async fn update_rugcheck_cache(mint: &str, result: CachedRugcheckResult) {
    let mut cache = RUGCHECK_CACHE.write().await;
    cache.insert(mint.to_string(), result);

    // Clean up expired entries
    let now = Utc::now();
    cache.retain(|_, result| {
        let age = now - result.cached_at;
        age < chrono::Duration::minutes(CACHE_EXPIRATION_MINUTES * 2)
    });
}

// ===== ASYNC RUGCHECK BACKGROUND SERVICE =====

/// Background service to populate rugcheck cache
pub struct RugcheckCacheService;

impl RugcheckCacheService {
    /// Update rugcheck cache for a list of tokens
    pub async fn update_cache_for_tokens(mints: Vec<String>) -> Result<(), String> {
        log(
            LogTag::Rugcheck,
            "CACHE",
            &format!("Updating rugcheck cache for {} tokens", mints.len())
        );

        let mut success_count = 0;
        let mut error_count = 0;

        for mint in mints {
            match Self::update_single_token_cache(&mint).await {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    error_count += 1;
                    if error_count <= 5 {
                        // Only log first 5 errors to avoid spam
                        log(
                            LogTag::Rugcheck,
                            "ERROR",
                            &format!("Failed to update cache for {}: {}", mint, e)
                        );
                    }
                }
            }
        }

        log(
            LogTag::Rugcheck,
            "CACHE",
            &format!("Cache update complete: {} success, {} errors", success_count, error_count)
        );

        Ok(())
    }

    /// Update cache for a single token
    async fn update_single_token_cache(mint: &str) -> Result<(), String> {
        // Get rugcheck assessment
        let assessment_result = get_token_rugcheck_risk_assessment(mint).await;

        let cached_result = match assessment_result {
            Ok(Some(assessment)) => {
                let should_filter = matches!(
                    assessment.risk_level,
                    RugcheckRiskLevel::Dangerous | RugcheckRiskLevel::Critical
                );

                CachedRugcheckResult {
                    should_filter,
                    risk_level: Some(assessment.risk_level),
                    risk_reasons: assessment.risk_reasons,
                    cached_at: Utc::now(),
                }
            }
            Ok(None) => {
                // No rugcheck data available
                CachedRugcheckResult {
                    should_filter: false, // Allow trading if no data
                    risk_level: None,
                    risk_reasons: vec!["No rugcheck data available".to_string()],
                    cached_at: Utc::now(),
                }
            }
            Err(e) => {
                // Error getting data
                CachedRugcheckResult {
                    should_filter: false, // Allow trading on error
                    risk_level: None,
                    risk_reasons: vec![format!("Error fetching rugcheck data: {}", e)],
                    cached_at: Utc::now(),
                }
            }
        };

        // Update cache
        update_rugcheck_cache(mint, cached_result).await;
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats() -> (usize, usize) {
        let cache = RUGCHECK_CACHE.read().await;
        let total = cache.len();
        let now = Utc::now();
        let valid = cache
            .values()
            .filter(|result| {
                let age = now - result.cached_at;
                age < chrono::Duration::minutes(CACHE_EXPIRATION_MINUTES)
            })
            .count();

        (total, valid)
    }

    /// Clear expired cache entries
    pub async fn cleanup_cache() {
        let mut cache = RUGCHECK_CACHE.write().await;
        let now = Utc::now();
        let before_count = cache.len();

        cache.retain(|_, result| {
            let age = now - result.cached_at;
            age < chrono::Duration::minutes(CACHE_EXPIRATION_MINUTES * 2)
        });

        let after_count = cache.len();
        if before_count != after_count {
            log(
                LogTag::Rugcheck,
                "CLEANUP",
                &format!("Cleaned up {} expired cache entries", before_count - after_count)
            );
        }
    }
}

// ===== INTEGRATION FUNCTIONS =====

/// Enhanced filtering check that includes cached rugcheck data
pub fn enhanced_filter_token_rugcheck(token: &Token) -> Option<String> {
    // First check cached result
    if let Some(cached) = get_cached_rugcheck_result(&token.mint) {
        if cached.should_filter {
            let risk_level = cached.risk_level
                .map(|r| format!("{:?}", r))
                .unwrap_or_else(|| "UNKNOWN".to_string());

            let reasons = if cached.risk_reasons.is_empty() {
                "Security risk detected".to_string()
            } else {
                cached.risk_reasons.join(", ")
            };

            return Some(format!("RUGCHECK-{}: {}", risk_level, reasons));
        }
    }

    // If no cached data, do basic checks (same as before)
    if !token.labels.is_empty() {
        for label in &token.labels {
            if
                label.to_lowercase().contains("freeze") &&
                !label.to_lowercase().contains("no freeze")
            {
                return Some(
                    "FREEZE-AUTHORITY: Token may have freeze authority enabled".to_string()
                );
            }
        }
    }

    // Check for obvious scam indicators
    let symbol_lower = token.symbol.to_lowercase();
    let name_lower = token.name.to_lowercase();

    let rug_indicators = ["scam", "rug", "fake", "test", "honeypot"];
    for indicator in &rug_indicators {
        if symbol_lower.contains(indicator) || name_lower.contains(indicator) {
            return Some(format!("SCAM-INDICATOR: Token name/symbol contains '{}'", indicator));
        }
    }

    None
}

/// Start background rugcheck cache update service
pub async fn start_rugcheck_cache_service(
    shutdown: Arc<tokio::sync::Notify>
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        log(LogTag::Rugcheck, "START", "Background cache service started");

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Rugcheck, "STOP", "Background cache service stopping");
                    break;
                }
                
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
                    // Get current priority tokens and update their cache
                    let priority_tokens = crate::tokens::get_priority_tokens_safe().await;
                    
                    if !priority_tokens.is_empty() {
                        if let Err(e) = RugcheckCacheService::update_cache_for_tokens(priority_tokens).await {
                            log(LogTag::Rugcheck, "ERROR", 
                                &format!("Rugcheck cache update failed: {}", e));
                        }
                    }
                    
                    // Cleanup expired entries
                    RugcheckCacheService::cleanup_cache().await;
                }
            }
        }

        log(LogTag::Rugcheck, "STOP", "Background cache service stopped");
    })
}
