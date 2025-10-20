/// Cleanup logic - Blacklist management and database maintenance
///
/// Automatically blacklists tokens based on permanent token characteristics:
/// - mint_authority present (can mint unlimited tokens)
/// - freeze_authority present (can freeze user accounts)
///
/// NOTE: Liquidity and security scores are FILTERING criteria, not blacklist criteria.
/// Blacklist is for tokens that should NEVER be traded due to fundamental risks.
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::{TokenError, TokenResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::sleep;

// ============================================================================
// BLACKLIST CONDITIONS
// ============================================================================

/// Check if a token should be blacklisted based on authorities
///
/// Blacklist ONLY if:
/// - mint_authority IS NOT NULL (can mint unlimited tokens)
/// - freeze_authority IS NOT NULL (can freeze user accounts)
///
/// These are permanent token characteristics that make tokens fundamentally unsafe.
/// Liquidity and security scores belong in filtering, not blacklisting.
pub async fn should_blacklist_for_authorities(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<String>> {
    if let Some(rugcheck_data) = db.get_rugcheck_data(mint)? {
        // Check mint authority
        if let Some(mint_auth) = &rugcheck_data.mint_authority {
            return Ok(Some(format!(
                "Mint authority present: {}",
                mint_auth
            )));
        }

        // Check freeze authority
        if let Some(freeze_auth) = &rugcheck_data.freeze_authority {
            return Ok(Some(format!(
                "Freeze authority present: {}",
                freeze_auth
            )));
        }
    }

    Ok(None)
}

/// Evaluate all blacklist conditions for a token
///
/// Only checks for permanent token characteristics (authorities).
/// Filtering criteria (liquidity, security scores) are handled by the filtering system.
pub async fn evaluate_blacklist(mint: &str, db: &TokenDatabase) -> TokenResult<Option<String>> {
    // Check if already blacklisted
    if db.is_blacklisted(mint)? {
        return Ok(None);
    }

    // ONLY check authorities - this is the correct blacklist criteria
    should_blacklist_for_authorities(mint, db).await
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
        db.get_blacklist_reason(mint)
            .map(|opt| opt.map(|(reason, _)| reason))
    } else {
        Ok(None)
    }
}

/// Blacklist summary for dashboard
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlacklistSummary {
    pub total_count: usize,
    pub authority_mint_count: usize,
    pub authority_freeze_count: usize,
    pub manual_count: usize,
    pub non_authority_auto_count: usize,
    pub non_authority_breakdown: std::collections::HashMap<String, usize>,
}

/// Get blacklist summary
pub fn get_blacklist_summary(db: &TokenDatabase) -> TokenResult<BlacklistSummary> {
    let conn = db.connection();
    let conn = conn
        .lock()
        .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

    let total_count: usize = conn
        .query_row("SELECT COUNT(*) FROM blacklist", [], |row| row.get(0))
        .unwrap_or(0);

    let mut authority_mint_count = 0;
    let mut authority_freeze_count = 0;
    let mut manual_count = 0;
    let mut non_authority_auto_count = 0;
    let mut non_authority_breakdown: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    let mut stmt = conn
        .prepare("SELECT reason, source FROM blacklist")
        .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

    let reasons = stmt
        .query_map([], |row| {
            let reason: String = row.get(0)?;
            let source: String = row.get(1)?;
            Ok((reason, source))
        })
        .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

    for reason_result in reasons {
        if let Ok((reason, source)) = reason_result {
            if source.eq_ignore_ascii_case("manual") {
                manual_count += 1;
                continue;
            }

            let reason_lower = reason.to_ascii_lowercase();

            if reason_lower.starts_with("mint authority") {
                authority_mint_count += 1;
            } else if reason_lower.starts_with("freeze authority") {
                authority_freeze_count += 1;
            } else {
                non_authority_auto_count += 1;
                *non_authority_breakdown.entry(reason).or_insert(0) += 1;
            }
        }
    }

    Ok(BlacklistSummary {
        total_count,
        authority_mint_count,
        authority_freeze_count,
        manual_count,
        non_authority_auto_count,
        non_authority_breakdown,
    })
}
