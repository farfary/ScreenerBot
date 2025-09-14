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
use rusqlite;
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

        // Calculate risk level - downgrade if any core safety vector fails
        self.risk_level = if
            !self.mint_authority_disabled ||
            !self.freeze_authority_disabled ||
            !self.lp_is_safe
        {
            // Any core failure means at least Medium risk
            if self.security_score >= 50 {
                SecurityRiskLevel::Medium
            } else if self.security_score >= 25 {
                SecurityRiskLevel::High
            } else {
                SecurityRiskLevel::Critical
            }
        } else if self.is_safe {
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

        // Set holder info for backward compatibility (NOTE: Contains estimated/synthetic data)
        self.holder_info = Some(HolderSecurityInfo {
            total_holders: self.holder_count,
            // SYNTHETIC: Estimates based on holder count - not real distribution analysis
            top_10_concentration: if self.holder_count < 10 {
                95.0 // Estimate: very concentrated if few holders
            } else {
                50.0 // Estimate: moderate concentration otherwise
            },
            largest_holder_percentage: if self.holder_count < 5 {
                80.0 // Estimate: high concentration with very few holders
            } else {
                20.0 // Estimate: lower concentration with more holders
            },
            whale_count: if self.holder_count < 10 {
                self.holder_count // Estimate: all holders might be whales if very few
            } else {
                5 // Estimate: assume some whales exist
            },
            distribution_score: if self.holder_count >= 100 {
                85 // Good distribution
            } else if self.holder_count >= 50 {
                70 // Moderate distribution
            } else {
                40 // Poor distribution
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

/// Initialize HTTP client
fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client
            ::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ScreenerBot/1.0")
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
            log(LogTag::Security, "API_ERROR", &format!("Failed to check API status: {}", e));
            Err(
                ScreenerBotError::Network(crate::errors::NetworkError::Generic {
                    message: format!("API check failed: {}", e),
                })
            )
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

    // Retry with exponential backoff (max 3 attempts)
    let mut last_error = None;
    for attempt in 1..=3 {
        // Make API request
        let response = match client.get(&url).send().await {
            Ok(response) => response,
            Err(e) => {
                last_error = Some(e);
                if attempt < 3 {
                    let delay_ms = match attempt {
                        1 => 150,
                        2 => 400,
                        _ => 1000,
                    };
                    log(
                        LogTag::Security,
                        "API_RETRY",
                        &format!(
                            "API request failed (attempt {}), retrying in {}ms",
                            attempt,
                            delay_ms
                        )
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }
                let error_msg = format!(
                    "Failed to fetch token report after {} attempts: {}",
                    attempt,
                    last_error.as_ref().unwrap()
                );
                log(LogTag::Security, "API_ERROR", &error_msg);
                return Err(
                    ScreenerBotError::Network(crate::errors::NetworkError::Generic {
                        message: error_msg,
                    })
                );
            }
        };

        // Check response status
        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            if attempt < 3 && (status_code >= 500 || status_code == 429) {
                let delay_ms = match attempt {
                    1 => 150,
                    2 => 400,
                    _ => 1000,
                };
                log(
                    LogTag::Security,
                    "API_RETRY",
                    &format!(
                        "API returned {} (attempt {}), retrying in {}ms",
                        status_code,
                        attempt,
                        delay_ms
                    )
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                continue;
            }

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
                if attempt < 3 {
                    let delay_ms = match attempt {
                        1 => 150,
                        2 => 400,
                        _ => 1000,
                    };
                    log(
                        LogTag::Security,
                        "PARSE_RETRY",
                        &format!(
                            "JSON parse failed (attempt {}), retrying in {}ms",
                            attempt,
                            delay_ms
                        )
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }
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

        // Success - extract security information and return
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

        return Ok(security_info);
    }

    // This should never be reached due to loop logic, but return error instead of panic
    Err(
        ScreenerBotError::Data(crate::errors::DataError::Generic {
            message: "Retry loop completed without returning result".to_string(),
        })
    )
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
            "API".to_string() // Mark as fresh API data
        )
    )
}

/// Check if LP is considered safe based on markets and risks
fn check_lp_safety(markets: &[MarketInfo], risks: &[RiskInfo]) -> bool {
    // Check if there are any high-risk LP-related issues
    let has_lp_unlock_risk = risks
        .iter()
        .any(|risk| risk.name.to_lowercase().contains("lp unlocked") && risk.level == "danger");

    if has_lp_unlock_risk {
        return false;
    }

    // If no markets, consider unsafe
    if markets.is_empty() {
        return false;
    }

    // Check LP lock percentage across all markets - use minimum (worst case)
    let min_lp_locked_pct = markets
        .iter()
        .map(|market| market.lp.lp_locked_pct)
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    // Consider LP safe if at least 90% is locked in ALL pools
    min_lp_locked_pct >= 90.0
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

/// Security database for persistent storage
#[derive(Clone)]
pub struct SecurityDatabase {
    connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
}

impl SecurityDatabase {
    /// Create new security database instance (uses tokens.db)
    pub fn new() -> Result<Self, ScreenerBotError> {
        use rusqlite::Connection;

        let conn = Connection::open("data/tokens.db").map_err(|e|
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to open tokens.db: {}", e),
            })
        )?;

        // Configure connection for optimal performance (same as token cache)
        conn.pragma_update(None, "journal_mode", "WAL").map_err(|e|
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set journal mode: {}", e),
            })
        )?;
        conn.pragma_update(None, "synchronous", "NORMAL").map_err(|e|
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set synchronous mode: {}", e),
            })
        )?;
        conn.pragma_update(None, "temp_store", "memory").map_err(|e|
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set temp store: {}", e),
            })
        )?;
        conn.pragma_update(None, "cache_size", 10000).map_err(|e|
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set cache size: {}", e),
            })
        )?;
        conn.busy_timeout(std::time::Duration::from_millis(30_000)).map_err(|e|
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set busy timeout: {}", e),
            })
        )?;

        // Add security columns if they don't exist (ignore "column already exists" errors)
        let alter_statements = vec![
            "ALTER TABLE tokens ADD COLUMN security_mint_authority_disabled INTEGER",
            "ALTER TABLE tokens ADD COLUMN security_freeze_authority_disabled INTEGER",
            "ALTER TABLE tokens ADD COLUMN security_lp_is_safe INTEGER",
            "ALTER TABLE tokens ADD COLUMN security_holder_count INTEGER",
            "ALTER TABLE tokens ADD COLUMN security_is_safe INTEGER",
            "ALTER TABLE tokens ADD COLUMN security_analyzed_at TEXT",
            "ALTER TABLE tokens ADD COLUMN security_risk_level TEXT"
        ];

        for stmt in alter_statements {
            match conn.execute(stmt, []) {
                Ok(_) => {} // Column added successfully
                Err(rusqlite::Error::SqliteFailure(code, Some(msg))) if
                    msg.contains("duplicate column name")
                => {
                    // Column already exists, this is fine
                }
                Err(e) => {
                    log(LogTag::Security, "DB_WARNING", &format!("Failed to add column: {}", e));
                    // Don't fail initialization for column issues
                }
            }
        }

        Ok(SecurityDatabase {
            connection: std::sync::Arc::new(std::sync::Mutex::new(conn)),
        })
    }

    /// Get security info from database
    pub fn get_security_info(
        &self,
        mint: &str
    ) -> Result<Option<TokenSecurityInfo>, ScreenerBotError> {
        let conn = self.connection.lock().map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to lock database: {}", e),
            })
        })?;

        // Set a busy timeout to avoid blocking indefinitely
        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set busy timeout: {}", e),
            })
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT security_mint_authority_disabled, security_freeze_authority_disabled, 
                    security_lp_is_safe, security_holder_count, security_is_safe, 
                    security_analyzed_at, security_risk_level 
             FROM tokens WHERE mint = ?1 AND security_analyzed_at IS NOT NULL"
            )
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to prepare select statement: {}", e),
                })
            })?;

        let result = stmt.query_row([mint], |row| {
            let analyzed_at_str: String = row.get(5)?;
            let analyzed_at = chrono::DateTime
                ::parse_from_rfc3339(&analyzed_at_str)
                .map_err(|_|
                    rusqlite::Error::InvalidColumnType(
                        5,
                        "security_analyzed_at".to_string(),
                        rusqlite::types::Type::Text
                    )
                )?
                .with_timezone(&chrono::Utc);

            Ok(
                TokenSecurityInfo::new(
                    mint.to_string(),
                    row.get::<_, i32>(0)? == 1, // mint_authority_disabled
                    row.get::<_, i32>(1)? == 1, // freeze_authority_disabled
                    row.get::<_, i32>(2)? == 1, // lp_is_safe
                    row.get::<_, u32>(3)?, // holder_count
                    row.get::<_, i32>(4)? == 1, // is_safe
                    analyzed_at,
                    "CACHED".to_string()
                )
            )
        });

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) =>
                Err(
                    ScreenerBotError::Data(crate::errors::DataError::Generic {
                        message: format!("Failed to query security info: {}", e),
                    })
                ),
        }
    }

    /// Store security info in database
    pub fn store_security_info(&self, info: &TokenSecurityInfo) -> Result<(), ScreenerBotError> {
        let conn = self.connection.lock().map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to lock database: {}", e),
            })
        })?;

        // Set a busy timeout to avoid blocking indefinitely
        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set busy timeout: {}", e),
            })
        })?;

        conn
            .execute(
                "UPDATE tokens SET 
                security_mint_authority_disabled = ?2,
                security_freeze_authority_disabled = ?3,
                security_lp_is_safe = ?4,
                security_holder_count = ?5,
                security_is_safe = ?6,
                security_analyzed_at = ?7,
                security_risk_level = ?8
             WHERE mint = ?1",
                rusqlite::params![
                    info.mint,
                    if info.mint_authority_disabled {
                        1
                    } else {
                        0
                    },
                    if info.freeze_authority_disabled {
                        1
                    } else {
                        0
                    },
                    if info.lp_is_safe {
                        1
                    } else {
                        0
                    },
                    info.holder_count,
                    if info.is_safe {
                        1
                    } else {
                        0
                    },
                    info.analyzed_at.to_rfc3339(),
                    info.risk_level.as_str()
                ]
            )
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to update security info: {}", e),
                })
            })?;

        log(
            LogTag::Security,
            "STORE",
            &format!("Stored security info for {}", safe_truncate(&info.mint, 12))
        );

        Ok(())
    }
}

impl TokenSecurityAnalyzer {
    /// Analyze token security (compatibility wrapper with caching)
    pub async fn analyze_token_security(
        &self,
        mint: &str
    ) -> Result<TokenSecurityInfo, ScreenerBotError> {
        self.analyze_token_security_with_cache(mint, false).await
    }

    /// Analyze token security with optional force refresh
    pub async fn analyze_token_security_with_cache(
        &self,
        mint: &str,
        force_refresh: bool
    ) -> Result<TokenSecurityInfo, ScreenerBotError> {
        // Check database cache first (unless forcing refresh)
        if !force_refresh {
            if let Ok(Some(cached_info)) = self.database.get_security_info(mint) {
                // Check if cached data is fresh (less than 24 hours old)
                let age_hours = (chrono::Utc::now() - cached_info.analyzed_at).num_hours();
                if age_hours < 24 {
                    log(
                        LogTag::Security,
                        "CACHE_HIT",
                        &format!(
                            "Using cached security info for {} (age: {}h)",
                            safe_truncate(mint, 12),
                            age_hours
                        )
                    );
                    return Ok(cached_info);
                }
            }
        }

        // Cache miss or stale data - fetch from API
        log(
            LogTag::Security,
            "CACHE_MISS",
            &format!("Fetching fresh security data for {}", safe_truncate(mint, 12))
        );

        let security_info = analyze_token_security(mint).await?;

        // Store in database for future use
        if let Err(e) = self.database.store_security_info(&security_info) {
            log(LogTag::Security, "STORE_ERROR", &format!("Failed to cache security info: {}", e));
            // Continue anyway - don't fail the whole operation
        }

        Ok(security_info)
    }

    /// Analyze multiple tokens (compatibility wrapper)
    pub async fn analyze_multiple_tokens(
        &self,
        mints: &[String]
    ) -> Result<HashMap<String, TokenSecurityInfo>, ScreenerBotError> {
        if mints.is_empty() {
            return Ok(HashMap::new());
        }

        log(
            LogTag::Security,
            "BATCH_START",
            &format!("Starting cached batch security analysis for {} tokens", mints.len())
        );

        let mut results = HashMap::new();
        let mut cache_hits = 0;
        let mut api_calls = 0;
        let mut errors = 0;

        // Process tokens with caching
        for (i, mint) in mints.iter().enumerate() {
            match self.analyze_token_security_with_cache(mint, false).await {
                Ok(security_info) => {
                    if security_info.api_status == "CACHED" {
                        cache_hits += 1;
                    } else {
                        api_calls += 1;
                    }
                    results.insert(mint.clone(), security_info);
                }
                Err(e) => {
                    errors += 1;
                    log(
                        LogTag::Security,
                        "BATCH_ERROR",
                        &format!("Failed to analyze token {}: {}", safe_truncate(mint, 12), e)
                    );
                }
            }

            // Add delay only for API calls (not cached results)
            if
                i < mints.len() - 1 &&
                api_calls > 0 &&
                results
                    .get(mint)
                    .map(|info| info.api_status != "CACHED")
                    .unwrap_or(false)
            {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }

        log(
            LogTag::Security,
            "BATCH_COMPLETE",
            &format!(
                "Batch complete: {} success ({} cached, {} API calls), {} errors",
                results.len(),
                cache_hits,
                api_calls,
                errors
            )
        );

        Ok(results)
    }
}

/// Global security database instance
static SECURITY_DATABASE: OnceLock<SecurityDatabase> = OnceLock::new();

/// Get global security analyzer (backward compatibility)
pub fn get_security_analyzer() -> TokenSecurityAnalyzer {
    let database = SECURITY_DATABASE.get_or_init(|| {
        SecurityDatabase::new().unwrap_or_else(|e| {
            log(
                LogTag::Security,
                "DB_ERROR",
                &format!("Failed to create security database: {}", e)
            );
            // Fallback to dummy database for compatibility
            SecurityDatabase {
                connection: std::sync::Arc::new(
                    std::sync::Mutex::new(
                        rusqlite::Connection
                            ::open_in_memory()
                            .expect("Failed to create fallback DB")
                    )
                ),
            }
        })
    });

    TokenSecurityAnalyzer {
        cache: SecurityCache,
        database: database.clone(),
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
