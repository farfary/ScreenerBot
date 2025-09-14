/// Simplified Token Security Analysis Module using Rugcheck API
/// Extended with backward compatibility for existing codebase
///
/// This module provides essential security analysis for Solana tokens by using
/// the Rugcheck API to check:
/// - Mint authority (must be None for safety)
/// - Freeze authority (must be None for safety)
/// - LP lock status (must be locked for safety)
/// - Holder count (for basic distribution analysis)
///
/// Includes backward compatibility for existing interfaces.

use crate::{ errors::ScreenerBotError, logger::{ log, LogTag }, utils::safe_truncate };

use chrono::{ DateTime, Utc };
use reqwest;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::time::Duration;
use std::sync::{ Arc, OnceLock };
use tokio::sync::Notify;

/// Security risk levels (backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityRiskLevel {
    /// Very safe token - all security checks passed
    Safe,
    /// Low risk - minor concerns
    Low,
    /// Medium risk - some security concerns
    Medium,
    /// High risk - significant security issues
    High,
    /// Critical risk - major red flags
    Critical,
    /// Unable to analyze properly
    Unknown,
}

impl SecurityRiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityRiskLevel::Safe => "SAFE",
            SecurityRiskLevel::Low => "LOW",
            SecurityRiskLevel::Medium => "MEDIUM",
            SecurityRiskLevel::High => "HIGH",
            SecurityRiskLevel::Critical => "CRITICAL",
            SecurityRiskLevel::Unknown => "UNKNOWN",
        }
    }

    pub fn color_emoji(&self) -> &'static str {
        match self {
            SecurityRiskLevel::Safe => "🟢",
            SecurityRiskLevel::Low => "🟡",
            SecurityRiskLevel::Medium => "🟠",
            SecurityRiskLevel::High => "🔴",
            SecurityRiskLevel::Critical => "💀",
            SecurityRiskLevel::Unknown => "❓",
        }
    }
}

/// Security flags (backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFlags {
    /// Can mint new tokens (opposite of mint_authority_disabled)
    pub can_mint: bool,
    /// Can freeze accounts (opposite of freeze_authority_disabled)
    pub can_freeze: bool,
    /// Has update authority
    pub has_update_authority: bool,
    /// LP is locked/burned (same as lp_is_safe)
    pub lp_locked: bool,
    /// High holder concentration
    pub high_concentration: bool,
    /// Very few holders
    pub few_holders: bool,
    /// Potential whale manipulation
    pub whale_risk: bool,
    /// Unknown or failed to analyze
    pub analysis_incomplete: bool,
}

/// Holder security info (backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolderSecurityInfo {
    pub total_holders: u32,
    pub top_10_concentration: f64,
    pub largest_holder_percentage: f64,
    pub whale_count: u32,
    pub distribution_score: u8,
}

/// Security timestamps (backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityTimestamps {
    pub first_analyzed: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub authority_last_checked: DateTime<Utc>,
    pub holder_last_checked: Option<DateTime<Utc>>,
    pub lp_lock_last_checked: Option<DateTime<Utc>>,
}

/// Essential security information for a token (with backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSecurityInfo {
    /// Token mint address
    pub mint: String,
    /// Whether mint authority is disabled (None = safe)
    pub mint_authority_disabled: bool,
    /// Whether freeze authority is disabled (None = safe)
    pub freeze_authority_disabled: bool,
    /// Whether LP is considered locked/safe
    pub lp_is_safe: bool,
    /// Total holder count
    pub holder_count: u32,
    /// Overall safety status
    pub is_safe: bool,
    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
    /// Source API status
    pub api_status: String,

    // Backward compatibility fields
    /// Overall security score (0-100, higher is safer) - computed from safety
    pub security_score: u8,
    /// Risk level based on combined analysis
    pub risk_level: SecurityRiskLevel,
    /// Security flags and warnings
    pub security_flags: SecurityFlags,

    // Additional backward compatibility fields
    /// Holder security information
    pub holder_info: Option<HolderSecurityInfo>,
    /// Analysis timestamps
    pub timestamps: SecurityTimestamps,
}

impl TokenSecurityInfo {
    /// Create a new TokenSecurityInfo with computed compatibility fields
    pub fn new(
        mint: String,
        mint_authority_disabled: bool,
        freeze_authority_disabled: bool,
        lp_is_safe: bool,
        holder_count: u32,
        is_safe: bool,
        analyzed_at: DateTime<Utc>,
        api_status: String
    ) -> Self {
        let mut info = TokenSecurityInfo {
            mint,
            mint_authority_disabled,
            freeze_authority_disabled,
            lp_is_safe,
            holder_count,
            is_safe,
            analyzed_at: analyzed_at.clone(),
            api_status,
            // Initialize computed fields
            security_score: 0,
            risk_level: SecurityRiskLevel::Unknown,
            security_flags: SecurityFlags {
                can_mint: false,
                can_freeze: false,
                has_update_authority: false,
                lp_locked: false,
                high_concentration: false,
                few_holders: false,
                whale_risk: false,
                analysis_incomplete: false,
            },
            // Backward compatibility fields
            holder_info: None, // Will be populated if needed
            timestamps: SecurityTimestamps {
                first_analyzed: analyzed_at.clone(),
                last_updated: analyzed_at.clone(),
                authority_last_checked: analyzed_at.clone(),
                holder_last_checked: Some(analyzed_at.clone()),
                lp_lock_last_checked: Some(analyzed_at),
            },
        };

        // Update computed fields
        info.update_computed_fields();
        info
    }

    /// Update computed fields based on core security data
    fn update_computed_fields(&mut self) {
        // Calculate security score (0-100)
        let mut score = 0u8;

        if self.mint_authority_disabled {
            score += 30; // Most important
        }
        if self.freeze_authority_disabled {
            score += 30; // Most important
        }
        if self.lp_is_safe {
            score += 25; // Very important
        }
        if self.holder_count >= 100 {
            score += 15; // Good distribution
        } else if self.holder_count >= 50 {
            score += 10;
        } else if self.holder_count >= 20 {
            score += 5;
        }

        self.security_score = score.min(100);

        // Base risk level from score
        let mut derived = if self.is_safe {
            SecurityRiskLevel::Safe
        } else if self.security_score >= 70 {
            SecurityRiskLevel::Low
        } else if self.security_score >= 50 {
            SecurityRiskLevel::Medium
        } else if self.security_score >= 25 {
            SecurityRiskLevel::High
        } else {
            SecurityRiskLevel::Critical
        };

        // Clamp: any core unsafe dimension promotes minimum severity
        if !self.lp_is_safe || !self.mint_authority_disabled || !self.freeze_authority_disabled {
            // If base was Safe -> downgrade to Medium (can't be safe with any core vector active)
            derived = match derived {
                SecurityRiskLevel::Safe => SecurityRiskLevel::Medium,
                SecurityRiskLevel::Low => SecurityRiskLevel::Medium,
                other => other,
            };
            // If mint + freeze both enabled OR LP unsafe + an authority enabled -> at least High
            if
                (!self.mint_authority_disabled && !self.freeze_authority_disabled) ||
                (!self.lp_is_safe && !self.mint_authority_disabled)
            {
                if matches!(derived, SecurityRiskLevel::Medium | SecurityRiskLevel::Low) {
                    derived = SecurityRiskLevel::High;
                }
            }
            // If all three unsafe -> Critical
            if !self.lp_is_safe && !self.mint_authority_disabled && !self.freeze_authority_disabled {
                derived = SecurityRiskLevel::Critical;
            }
        }

        self.risk_level = derived;

        // Set security flags
        self.security_flags = SecurityFlags {
            can_mint: !self.mint_authority_disabled,
            can_freeze: !self.freeze_authority_disabled,
            has_update_authority: false, // Not tracked in simplified version
            lp_locked: self.lp_is_safe,
            high_concentration: self.holder_count < 50,
            few_holders: self.holder_count < 20,
            whale_risk: self.holder_count < 10,
            analysis_incomplete: false,
        };

        // Set holder info for backward compatibility
        self.holder_info = Some(HolderSecurityInfo {
            total_holders: self.holder_count,
            top_10_concentration: if self.holder_count < 10 {
                95.0
            } else {
                50.0
            }, // Estimated
            largest_holder_percentage: if self.holder_count < 5 {
                80.0
            } else {
                20.0
            }, // Estimated
            whale_count: if self.holder_count < 10 {
                self.holder_count
            } else {
                5
            }, // Estimated
            distribution_score: if self.holder_count >= 100 {
                85
            } else if self.holder_count >= 50 {
                70
            } else {
                40
            },
        });
    }

    /// Get a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Security for {}: Safe={} (Mint:{}, Freeze:{}, LP:{}, Holders:{})",
            safe_truncate(&self.mint, 12),
            self.is_safe,
            if self.mint_authority_disabled {
                "DISABLED"
            } else {
                "ENABLED"
            },
            if self.freeze_authority_disabled {
                "DISABLED"
            } else {
                "ENABLED"
            },
            if self.lp_is_safe {
                "SAFE"
            } else {
                "UNSAFE"
            },
            self.holder_count
        )
    }

    /// Check if token meets minimum safety requirements
    pub fn meets_safety_requirements(&self) -> bool {
        self.mint_authority_disabled && self.freeze_authority_disabled && self.lp_is_safe
    }
}

/// Rugcheck API maintenance response
#[derive(Debug, Deserialize)]
struct MaintenanceResponse {
    // API returns empty response or status when operational
}

/// Rugcheck API token report response (simplified - only fields we need)
#[derive(Debug, Deserialize)]
struct RugcheckTokenReport {
    pub mint: String,
    pub token: TokenInfo,
    #[serde(rename = "totalHolders")]
    pub total_holders: u32,
    pub markets: Vec<MarketInfo>,
    pub risks: Vec<RiskInfo>,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    #[serde(rename = "mintAuthority")]
    pub mint_authority: Option<String>,
    #[serde(rename = "freezeAuthority")]
    pub freeze_authority: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MarketInfo {
    pub lp: LpInfo,
}

#[derive(Debug, Deserialize)]
struct LpInfo {
    #[serde(rename = "lpLocked")]
    pub lp_locked: f64,
    #[serde(rename = "lpUnlocked")]
    pub lp_unlocked: f64,
    #[serde(rename = "lpLockedPct")]
    pub lp_locked_pct: f64,
}

#[derive(Debug, Deserialize)]
struct RiskInfo {
    pub name: String,
    pub level: String,
}

/// HTTP client for API calls
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Initialize and return a reference to the shared HTTP client (thread-safe)
fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client
            ::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ScreenerBot/1.0 (+github.com/farfary/ScreenerBot)")
            .build()
            .expect("Failed to create HTTP client")
    })
}

/// Check if Rugcheck API is operational
pub async fn check_api_status() -> Result<bool, ScreenerBotError> {
    let client = get_http_client();
    let url = "https://api.rugcheck.xyz/v1/maintenance";

    log(LogTag::Security, "API_CHECK", "Checking Rugcheck API status");

    match client.get(url).send().await {
        Ok(response) => {
            let status_ok = response.status().is_success();
            let status_code = response.status().as_u16();

            log(
                LogTag::Security,
                "API_STATUS",
                &format!(
                    "Rugcheck API status: {} ({})",
                    if status_ok {
                        "OK"
                    } else {
                        "ERROR"
                    },
                    status_code
                )
            );

            Ok(status_ok)
        }
        Err(e) => {
            let msg = format!("Failed to check API status: {}", e);
            log(LogTag::Security, "API_ERROR", &msg);
            Err(ScreenerBotError::Network(crate::errors::NetworkError::Generic { message: msg }))
        }
    }
}

/// Analyze token security using Rugcheck API
pub async fn analyze_token_security(mint: &str) -> Result<TokenSecurityInfo, ScreenerBotError> {
    log(
        LogTag::Security,
        "ANALYZE_START",
        &format!("Starting security analysis for mint: {}", safe_truncate(mint, 12))
    );

    let client = get_http_client();
    let url = format!("https://api.rugcheck.xyz/v1/tokens/{}/report", mint);

    // Make API request with simple retry & backoff
    let mut attempt: u8 = 0;
    let max_attempts = 3u8;
    let mut last_err: Option<reqwest::Error> = None;
    let mut response_opt: Option<reqwest::Response> = None;
    while attempt < max_attempts {
        attempt += 1;
        match client.get(&url).send().await {
            Ok(resp) => {
                response_opt = Some(resp);
                break;
            }
            Err(e) => {
                last_err = Some(e);
                if attempt < max_attempts {
                    let backoff_ms = if attempt == 1 { 150 } else { 400 };
                    log(
                        LogTag::Security,
                        "API_RETRY",
                        &format!(
                            "Attempt {} failed for {} – backing off {}ms",
                            attempt,
                            safe_truncate(mint, 12),
                            backoff_ms
                        )
                    );
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }
    let response = if let Some(r) = response_opt {
        r
    } else {
        let error_msg = format!(
            "Failed to fetch token report after {} attempts: {}",
            attempt,
            last_err.map(|e| e.to_string()).unwrap_or_else(|| "unknown error".to_string())
        );
        log(LogTag::Security, "API_ERROR", &error_msg);
        return Err(
            ScreenerBotError::Network(crate::errors::NetworkError::Generic { message: error_msg })
        );
    };

    // Check response status
    if !response.status().is_success() {
        let status_code = response.status().as_u16();
        let error_msg = format!("API returned error status: {}", status_code);
        log(LogTag::Security, "API_ERROR", &error_msg);
        return Err(
            ScreenerBotError::Network(crate::errors::NetworkError::HttpStatusError {
                endpoint: url.clone(),
                status: status_code,
                body: None,
            })
        );
    }

    // Parse response
    let report: RugcheckTokenReport = match response.json().await {
        Ok(report) => report,
        Err(e) => {
            let error_msg = format!("Failed to parse API response: {}", e);
            log(LogTag::Security, "PARSE_ERROR", &error_msg);
            return Err(
                ScreenerBotError::Data(crate::errors::DataError::ParseError {
                    data_type: "rugcheck_report".to_string(),
                    error: error_msg,
                })
            );
        }
    };

    // Extract security information
    let security_info = extract_security_info(report)?;

    log(
        LogTag::Security,
        "ANALYZE_COMPLETE",
        &format!(
            "Security analysis complete for {}: Safe={}, MintAuth={}, FreezeAuth={}, LP={}, Holders={}",
            safe_truncate(mint, 12),
            security_info.is_safe,
            security_info.mint_authority_disabled,
            security_info.freeze_authority_disabled,
            security_info.lp_is_safe,
            security_info.holder_count
        )
    );

    Ok(security_info)
}

/// Extract essential security information from Rugcheck report
fn extract_security_info(
    report: RugcheckTokenReport
) -> Result<TokenSecurityInfo, ScreenerBotError> {
    // Check mint authority (None = safe, Some = unsafe)
    let mint_authority_disabled = report.token.mint_authority.is_none();

    // Check freeze authority (None = safe, Some = unsafe)
    let freeze_authority_disabled = report.token.freeze_authority.is_none();

    // Check LP lock status
    let lp_is_safe = check_lp_safety(&report.markets, &report.risks);

    // Get holder count
    let holder_count = report.total_holders;

    // Determine overall safety
    let is_safe = mint_authority_disabled && freeze_authority_disabled && lp_is_safe;

    Ok(
        TokenSecurityInfo::new(
            report.mint,
            mint_authority_disabled,
            freeze_authority_disabled,
            lp_is_safe,
            holder_count,
            is_safe,
            Utc::now(),
            "OK".to_string()
        )
    )
}

/// Check if LP is considered safe based on markets and risks
fn check_lp_safety(markets: &[MarketInfo], risks: &[RiskInfo]) -> bool {
    if markets.is_empty() {
        log(LogTag::Security, "LP_HEURISTIC", "No markets present -> LP unsafe");
        return false;
    }

    // Normalize and look for any unlock danger patterns
    let has_unlock = risks.iter().any(|risk| {
        let name_lc = risk.name.to_lowercase();
        let level_lc = risk.level.to_lowercase();
        name_lc.contains("lp") &&
            name_lc.contains("unlock") &&
            (level_lc == "danger" || level_lc == "high")
    });
    if has_unlock {
        return false;
    }

    // Use WORST (minimum) locked pct to avoid picking a tiny safe pool
    let mut min_locked_pct = f64::INFINITY;
    for market in markets.iter() {
        let pct = market.lp.lp_locked_pct;
        if pct < min_locked_pct {
            min_locked_pct = pct;
        }
    }
    if !min_locked_pct.is_finite() {
        return false;
    }

    // Require >= 90% locked across worst pool
    min_locked_pct >= 90.0
}

/// Batch analyze multiple tokens
pub async fn analyze_multiple_tokens(
    mints: &[String]
) -> Result<HashMap<String, TokenSecurityInfo>, ScreenerBotError> {
    if mints.is_empty() {
        return Ok(HashMap::new());
    }

    log(
        LogTag::Security,
        "BATCH_START",
        &format!("Starting batch security analysis for {} tokens", mints.len())
    );

    let mut results = HashMap::new();
    let mut successful = 0;
    let mut failed = 0;

    // Process tokens with small delays to be respectful to API
    for (i, mint) in mints.iter().enumerate() {
        match analyze_token_security(mint).await {
            Ok(security_info) => {
                results.insert(mint.clone(), security_info);
                successful += 1;
            }
            Err(e) => {
                failed += 1;
                log(
                    LogTag::Security,
                    "BATCH_ERROR",
                    &format!("Failed to analyze token {}: {}", safe_truncate(mint, 12), e)
                );
            }
        }

        // Add small delay between requests to avoid rate limiting
        if i < mints.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    }

    log(
        LogTag::Security,
        "BATCH_COMPLETE",
        &format!("Batch analysis complete: {}/{} successful", successful, successful + failed)
    );

    Ok(results)
}

/// Quick safety check - returns just boolean
pub async fn is_token_safe(mint: &str) -> Result<bool, ScreenerBotError> {
    let security_info = analyze_token_security(mint).await?;
    Ok(security_info.is_safe)
}

/// Get security summary string for logging
pub async fn get_security_summary(mint: &str) -> Result<String, ScreenerBotError> {
    let security_info = analyze_token_security(mint).await?;

    let status_emoji = if security_info.is_safe { "🟢" } else { "🔴" };
    let mint_auth = if security_info.mint_authority_disabled { "✅" } else { "❌" };
    let freeze_auth = if security_info.freeze_authority_disabled { "✅" } else { "❌" };
    let lp_status = if security_info.lp_is_safe { "🔒" } else { "🔓" };

    Ok(
        format!(
            "{} {} | Mint:{} Freeze:{} LP:{} Holders:{}",
            status_emoji,
            if security_info.is_safe {
                "SAFE"
            } else {
                "UNSAFE"
            },
            mint_auth,
            freeze_auth,
            lp_status,
            security_info.holder_count
        )
    )
}

// =============================================================================
// BACKWARD COMPATIBILITY FUNCTIONS
// =============================================================================

/// Security analyzer struct (backward compatibility)
pub struct TokenSecurityAnalyzer {
    pub cache: SecurityCache,
    pub database: SecurityDatabase,
}

/// Dummy cache for backward compatibility
pub struct SecurityCache;

impl SecurityCache {
    pub fn get(&self, _mint: &str) -> Option<TokenSecurityInfo> {
        // No caching in simplified version
        None
    }

    pub fn set(&self, _info: TokenSecurityInfo) {
        // No caching in simplified version
    }
}

/// Dummy database for backward compatibility
pub struct SecurityDatabase;

impl SecurityDatabase {
    pub fn get_security_info(
        &self,
        _mint: &str
    ) -> Result<Option<TokenSecurityInfo>, ScreenerBotError> {
        // No database caching in simplified version
        Ok(None)
    }
}

impl TokenSecurityAnalyzer {
    /// Analyze token security (compatibility wrapper)
    pub async fn analyze_token_security(
        &self,
        mint: &str
    ) -> Result<TokenSecurityInfo, ScreenerBotError> {
        analyze_token_security(mint).await
    }

    /// Analyze multiple tokens (compatibility wrapper)
    pub async fn analyze_multiple_tokens(
        &self,
        mints: &[String]
    ) -> Result<HashMap<String, TokenSecurityInfo>, ScreenerBotError> {
        analyze_multiple_tokens(mints).await
    }
}

/// Get global security analyzer (backward compatibility)
pub fn get_security_analyzer() -> TokenSecurityAnalyzer {
    TokenSecurityAnalyzer {
        cache: SecurityCache,
        database: SecurityDatabase,
    }
}

/// Start security monitoring background task (stub for backward compatibility)
pub async fn start_security_monitoring(
    shutdown: Arc<Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    log(
        LogTag::Security,
        "MONITOR_STUB",
        "Security monitoring started as stub - background monitoring disabled in simplified version"
    );

    // Return a dummy task that just waits for shutdown
    let handle = tokio::spawn(async move {
        shutdown.notified().await;
        log(LogTag::Security, "MONITOR_STOP", "Security monitoring stopped");
    });

    Ok(handle)
}
