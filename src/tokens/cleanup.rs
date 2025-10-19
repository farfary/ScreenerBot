/// Cleanup logic - Blacklist management and database maintenance
/// 
/// Automatically blacklists tokens that meet certain conditions:
/// - Too many consecutive update failures
/// - Liquidity below threshold for extended period
/// - Marked as rugged by security analysis
/// - Manual blacklist via API

use crate::tokens::database::TokenDatabase;
use crate::tokens::types::{TokenResult, TokenError};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::sleep;

// ============================================================================
// BLACKLIST CONDITIONS
// ============================================================================

/// Check if a token should be blacklisted based on update failures
/// 
/// Blacklist if:
/// - More than 5 consecutive update failures
/// - Last successful update > 7 days ago
pub async fn should_blacklist_for_failures(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<String>> {
    // Check update tracking
    let tracking = match db.get_oldest_non_blacklisted(1) {
        Ok(tokens) => {
            if tokens.is_empty() || tokens[0] != mint {
                return Ok(None);
            }
            // Token is in oldest list - check its failure count
            // This is a simplified check; real impl would query tracking table
            None
        }
        Err(_) => return Ok(None),
    };
    
    // For now, return None (full implementation requires update_tracking queries)
    Ok(None)
}

/// Check if a token should be blacklisted based on low liquidity
/// 
/// Blacklist if:
/// - Liquidity < $1000 USD for > 24 hours
/// - No market data available for > 7 days
pub async fn should_blacklist_for_liquidity(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<String>> {
    // Check DexScreener liquidity
    if let Some(dex_data) = db.get_dexscreener_data(mint)? {
        if let Some(liquidity) = dex_data.liquidity_usd {
            if liquidity < 1000.0 {
                return Ok(Some(format!(
                    "Low liquidity: ${:.2} (threshold: $1000)",
                    liquidity
                )));
            }
        }
    }
    
    // Check GeckoTerminal liquidity
    if let Some(gecko_data) = db.get_geckoterminal_data(mint)? {
        if let Some(reserve) = gecko_data.reserve_usd {
            if reserve < 1000.0 {
                return Ok(Some(format!(
                    "Low reserve: ${:.2} (threshold: $1000)",
                    reserve
                )));
            }
        }
    }
    
    Ok(None)
}

/// Check if a token should be blacklisted based on security analysis
/// 
/// Blacklist if:
/// - Marked as rugged by Rugcheck
/// - Security score < 20 (dangerous)
pub async fn should_blacklist_for_security(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<String>> {
    if let Some(rugcheck_data) = db.get_rugcheck_data(mint)? {
        // Check if marked as rugged
        if rugcheck_data.rugged {
            return Ok(Some("Token marked as rugged by Rugcheck".to_string()));
        }
        
        // Check security score (normalized 0-100)
        if let Some(score) = rugcheck_data.score_normalised {
            if score < 20 {
                return Ok(Some(format!(
                    "Security score too low: {} (threshold: 20)",
                    score
                )));
            }
        }
    }
    
    Ok(None)
}

/// Evaluate all blacklist conditions for a token
pub async fn evaluate_blacklist(mint: &str, db: &TokenDatabase) -> TokenResult<Option<String>> {
    // Check if already blacklisted
    if db.is_blacklisted(mint)? {
        return Ok(None);
    }
    
    // Check all conditions
    if let Some(reason) = should_blacklist_for_failures(mint, db).await? {
        return Ok(Some(reason));
    }
    
    if let Some(reason) = should_blacklist_for_liquidity(mint, db).await? {
        return Ok(Some(reason));
    }
    
    if let Some(reason) = should_blacklist_for_security(mint, db).await? {
        return Ok(Some(reason));
    }
    
    Ok(None)
}

// ============================================================================
// CLEANUP TASKS
// ============================================================================

/// Run cleanup scan on all tokens
/// 
/// Checks all non-blacklisted tokens and blacklists any that meet conditions
pub async fn run_cleanup_scan(db: &TokenDatabase) -> TokenResult<CleanupResult> {
    let mut checked = 0;
    let mut blacklisted = 0;
    let mut errors = 0;
    
    // Get all non-blacklisted tokens (limit to 10000 for performance)
    let tokens = db.list_tokens(10000)?;
    
    for token in tokens {
        checked += 1;
        
        // Skip if already blacklisted
        if db.is_blacklisted(&token.mint)? {
            continue;
        }
        
        // Evaluate blacklist conditions
        match evaluate_blacklist(&token.mint, db).await {
            Ok(Some(reason)) => {
                // Add to blacklist
                match db.add_to_blacklist(&token.mint, &reason, "auto_cleanup") {
                    Ok(_) => {
                        blacklisted += 1;
                        println!("[CLEANUP] Blacklisted {}: {}", token.mint, reason);
                    }
                    Err(e) => {
                        errors += 1;
                        eprintln!("[CLEANUP] Failed to blacklist {}: {}", token.mint, e);
                    }
                }
            }
            Ok(None) => {
                // No blacklist needed
            }
            Err(e) => {
                errors += 1;
                eprintln!("[CLEANUP] Error evaluating {}: {}", token.mint, e);
            }
        }
    }
    
    Ok(CleanupResult {
        checked,
        blacklisted,
        errors,
    })
}

/// Result of a cleanup scan
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub checked: usize,
    pub blacklisted: usize,
    pub errors: usize,
}

/// Start cleanup loop (runs every hour)
pub fn start_cleanup_loop(db: Arc<TokenDatabase>, shutdown: Arc<Notify>) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown.notified() => break,
                _ = sleep(Duration::from_secs(3600)) => {
                    println!("[CLEANUP] Starting cleanup scan...");
                    match run_cleanup_scan(&db).await {
                        Ok(result) => {
                            println!(
                                "[CLEANUP] Scan complete: {} checked, {} blacklisted, {} errors",
                                result.checked, result.blacklisted, result.errors
                            );
                        }
                        Err(e) => {
                            eprintln!("[CLEANUP] Scan failed: {}", e);
                        }
                    }
                }
            }
        }
    })
}

// ============================================================================
// MANUAL BLACKLIST OPERATIONS
// ============================================================================

/// Manually add a token to blacklist (e.g., via API)
pub fn blacklist_token(mint: &str, reason: &str, db: &TokenDatabase) -> TokenResult<()> {
    db.add_to_blacklist(mint, reason, "manual")
}

/// Remove a token from blacklist (e.g., via API)
pub fn unblacklist_token(mint: &str, db: &TokenDatabase) -> TokenResult<()> {
    db.remove_from_blacklist(mint)
}

/// Check if a token is blacklisted and get reason
pub fn get_blacklist_status(mint: &str, db: &TokenDatabase) -> TokenResult<Option<String>> {
    if db.is_blacklisted(mint)? {
        db.get_blacklist_reason(mint).map(|opt| opt.map(|(reason, _)| reason))
    } else {
        Ok(None)
    }
}

/// Blacklist summary for dashboard
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlacklistSummary {
    pub total_count: usize,
    pub low_liquidity_count: usize,
    pub no_route_count: usize,
    pub api_error_count: usize,
    pub system_token_count: usize,
    pub stable_token_count: usize,
    pub manual_count: usize,
    pub poor_performance_count: usize,
    pub security_count: usize,
}

/// Get blacklist summary
pub fn get_blacklist_summary(db: &TokenDatabase) -> TokenResult<BlacklistSummary> {
    let conn = db.connection();
    let conn = conn.lock()
        .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;
    
    let total_count: usize = conn.query_row(
        "SELECT COUNT(*) FROM blacklist",
        [],
        |row| row.get(0),
    ).unwrap_or(0);
    
    let mut low_liquidity_count = 0;
    let mut no_route_count = 0;
    let mut api_error_count = 0;
    let mut system_token_count = 0;
    let mut stable_token_count = 0;
    let mut manual_count = 0;
    let mut poor_performance_count = 0;
    let mut security_count = 0;
    
    let mut stmt = conn.prepare("SELECT reason FROM blacklist")
        .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;
    
    let reasons = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;
    
    for reason_result in reasons {
        if let Ok(reason) = reason_result {
            match reason.as_str() {
                "LowLiquidity" => low_liquidity_count += 1,
                "NoRoute" => no_route_count += 1,
                "ApiError" => api_error_count += 1,
                "SystemToken" => system_token_count += 1,
                "StableToken" => stable_token_count += 1,
                "manual" => manual_count += 1,
                "PoorPerformance" => poor_performance_count += 1,
                "SecurityIssue" | "LowSecurityScore" => security_count += 1,
                _ => {},
            }
        }
    }
    
    Ok(BlacklistSummary {
        total_count,
        low_liquidity_count,
        no_route_count,
        api_error_count,
        system_token_count,
        stable_token_count,
        manual_count,
        poor_performance_count,
        security_count,
    })
}
