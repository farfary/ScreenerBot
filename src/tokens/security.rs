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
use std::sync::Mutex as StdMutex;
use tokio::sync::{ Notify, RwLock };

// =============================================================================
// SECURITY SCANNING CONFIGURATION
// =============================================================================

/// Number of tokens to analyze per security scan cycle (similar to monitor service)
const SECURITY_TOKENS_PER_CYCLE: usize = 50;

/// Batch size for API calls within a cycle (RugCheck rate limiting)
const SECURITY_BATCH_SIZE: usize = 10;

/// Delay between batches within a cycle (milliseconds)
const SECURITY_BATCH_DELAY_MS: u64 = 2000;

/// Delay between individual requests within a batch (milliseconds)
const SECURITY_REQUEST_DELAY_MS: u64 = 300;

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
            self.mint,
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
    #[serde(rename = "totalHolders", default)]
    pub total_holders: u32,
    #[serde(default)]
    pub markets: Option<Vec<MarketInfo>>,
    #[serde(default)]
    pub risks: Option<Vec<RiskInfo>>,
    // knownAccounts is a mapping of address -> { name, type }
    // We don't need to model it fully for our current logic.
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
    // Pool pubkey (owner PDA for liquidity accounts typically equals this)
    #[serde(default)]
    pub pubkey: Option<String>,
    // AMM program type (e.g., "raydium", "orca", "meteoraDlmm")
    #[serde(rename = "marketType")]
    #[serde(default)]
    pub market_type: Option<String>,
    // Token mints involved
    #[serde(rename = "mintA")]
    #[serde(default)]
    pub mint_a: Option<String>,
    #[serde(rename = "mintB")]
    #[serde(default)]
    pub mint_b: Option<String>,
    #[serde(rename = "mintLP")]
    #[serde(default)]
    pub mint_lp: Option<String>,
    // Liquidity token accounts (owners should be pool PDA for non-LP pools)
    #[serde(rename = "liquidityAAccount")]
    #[serde(default)]
    pub liquidity_a: Option<TokenAccountInfo>,
    #[serde(rename = "liquidityBAccount")]
    #[serde(default)]
    pub liquidity_b: Option<TokenAccountInfo>,
    // LP info summary
    #[serde(default)]
    pub lp: Option<LpInfo>,
}

#[derive(Debug, Deserialize)]
struct TokenAccountInfo {
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LpInfo {
    #[serde(rename = "lpLocked")]
    #[serde(default)]
    pub lp_locked: f64,
    #[serde(rename = "lpUnlocked")]
    #[serde(default)]
    pub lp_unlocked: f64,
    #[serde(rename = "lpLockedPct")]
    #[serde(default)]
    pub lp_locked_pct: f64,
    // Extended fields used for pool classification/selection
    #[serde(rename = "lpMint")]
    #[serde(default)]
    pub lp_mint: Option<String>,
    #[serde(rename = "lpTotalSupply")]
    #[serde(default)]
    pub lp_total_supply: Option<f64>,
    #[serde(rename = "baseMint")]
    #[serde(default)]
    pub base_mint: Option<String>,
    #[serde(rename = "quoteMint")]
    #[serde(default)]
    pub quote_mint: Option<String>,
    #[serde(rename = "baseUSD")]
    #[serde(default)]
    pub base_usd: Option<f64>,
    #[serde(rename = "quoteUSD")]
    #[serde(default)]
    pub quote_usd: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RiskInfo {
    pub name: String,
    pub level: String,
}

/// HTTP client for API calls
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static SECURITY_SCAN_MUTEX: OnceLock<Arc<StdMutex<()>>> = OnceLock::new();

/// Initialize HTTP client
fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        match
            reqwest::Client
                ::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ScreenerBot/1.0")
                .build()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "[SECURITY][HTTP_INIT_ERROR] Failed to build configured client: {e} - using default client"
                );
                reqwest::Client::new()
            }
        }
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
        &format!("Starting security analysis for mint: {}", mint)
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
                            "API request failed (attempt {}), retrying in {}ms (mint={})",
                            attempt,
                            delay_ms,
                            mint
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
                        "API returned {} (attempt {}), retrying in {}ms (mint={})",
                        status_code,
                        attempt,
                        delay_ms,
                        mint
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
                            "JSON parse failed (attempt {}), retrying in {}ms (mint={})",
                            attempt,
                            delay_ms,
                            mint
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
                mint,
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
fn check_lp_safety(markets: &Option<Vec<MarketInfo>>, risks: &Option<Vec<RiskInfo>>) -> bool {
    // LP SAFETY CONTRACT (no USD thresholds, deterministic):
    // 1) We select the canonical SOL pool (highest-liquidity SOL pair) only for decision.
    // 2) If pool has an LP token (lpMint != burn address): LP is safe ONLY if fully locked/burned
    //    (lpUnlocked==0 OR lpLockedPct==100). Partial locks are unsafe.
    // 3) If pool is position-based (no LP token): We accept as "not removable by creator"
    //    when liquidity token accounts are owned by the pool PDA and the marketType matches
    //    a known non-custodial AMM family (meteora, clmm, orca). Missing ownership info is unsafe.
    // 4) If Rugcheck flags "LP unlocked" as DANGER, it's unsafe regardless.

    // If Rugcheck explicitly flags LP unlocked as danger, fail fast
    if let Some(risks_vec) = risks {
        let has_lp_unlock_risk = risks_vec.iter().any(|risk| {
            let name_l = risk.name.to_ascii_lowercase();
            name_l.contains("lp unlocked") && risk.level.eq_ignore_ascii_case("danger")
        });
        if has_lp_unlock_risk {
            return false;
        }
    }

    let Some(list) = markets else {
        return false;
    };
    if list.is_empty() {
        return false;
    }

    // Choose the canonical SOL pool (highest-liquidity SOL pair) without using thresholds.
    // We only use USD values for sorting (allowed by project rules), not as a decision threshold.
    const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

    // Score markets: prefer where quote/base equals SOL; compute liquidity score as baseUSD+quoteUSD if present
    let mut best_idx: Option<usize> = None;
    let mut best_score: f64 = -1.0;
    for (idx, m) in list.iter().enumerate() {
        // Determine if this is a SOL pair
        let is_sol_pair = m.lp
            .as_ref()
            .and_then(|lp| lp.quote_mint.clone())
            .map(|qm| qm == SOL_MINT)
            .unwrap_or_else(|| {
                // Fallback: check mintA/mintB
                m.mint_a.as_deref() == Some(SOL_MINT) || m.mint_b.as_deref() == Some(SOL_MINT)
            });
        if !is_sol_pair {
            continue;
        }

        // Score by USD sum if available, else 0 (still allows selection if nothing else has USD)
        let score = m.lp
            .as_ref()
            .map(|lp| lp.base_usd.unwrap_or(0.0) + lp.quote_usd.unwrap_or(0.0))
            .unwrap_or(0.0);
        if score > best_score {
            best_score = score;
            best_idx = Some(idx);
        }
    }

    let Some(canonical_idx) = best_idx else {
        // No SOL pair found → treat as unsafe per single-pool SOL policy
        return false;
    };
    let canonical = &list[canonical_idx];

    // Helper to check if a string equals the burn/"no mint" address
    fn is_burn_or_none(s: &str) -> bool {
        s == "11111111111111111111111111111111"
    }

    // If LP info exists, classify pool archetype
    if let Some(lp) = &canonical.lp {
        // LP-token pool when lp_mint is Some and not burn address
        let is_lp_token_pool = lp.lp_mint
            .as_ref()
            .map(|mint| !is_burn_or_none(mint))
            .unwrap_or(false);

        if is_lp_token_pool {
            // Deterministic: LP immovable only if fully locked/burned
            let locked_pct = lp.lp_locked_pct;
            let unlocked = lp.lp_unlocked;
            let fully_locked = locked_pct.is_finite() && locked_pct == 100.0;
            let no_unlocked = unlocked.is_finite() && unlocked == 0.0;

            if fully_locked || no_unlocked {
                return true;
            } else {
                log(
                    LogTag::Security,
                    "LP_CHECK",
                    &format!(
                        "LP-token pool not fully locked (mint={:?}, locked_pct={:.4}, unlocked={:.6})",
                        lp.lp_mint,
                        locked_pct,
                        unlocked
                    )
                );
                return false;
            }
        } else {
            // Position-based pool (no LP token). Verify non-custodial ownership by PDA.
            // Accept if liquidityA/B owners equal the pool pubkey (typical PDA custody) and market type is known.
            let pool_pubkey = canonical.pubkey.as_deref().unwrap_or("");
            let owner_a_ok = canonical.liquidity_a
                .as_ref()
                .and_then(|acc| acc.owner.as_deref())
                .map(|o| !o.is_empty() && o == pool_pubkey)
                .unwrap_or(false);
            let owner_b_ok = canonical.liquidity_b
                .as_ref()
                .and_then(|acc| acc.owner.as_deref())
                .map(|o| !o.is_empty() && o == pool_pubkey)
                .unwrap_or(false);

            // Allowlist known non-custodial AMM families
            let mt = canonical.market_type.as_deref().unwrap_or("").to_ascii_lowercase();
            let known = mt.contains("meteora") || mt.contains("clmm") || mt.contains("orca");

            if known && owner_a_ok && owner_b_ok {
                return true;
            } else {
                log(
                    LogTag::Security,
                    "LP_CHECK",
                    &format!(
                        "Position-based pool ownership not verified (pool={}, mt={}, ownerAok={}, ownerBok={})",
                        pool_pubkey,
                        mt,
                        owner_a_ok,
                        owner_b_ok
                    )
                );
                return false;
            }
        }
    }

    // If we cannot classify due to missing LP info, be conservative
    log(LogTag::Security, "LP_CHECK", "Missing LP info for canonical SOL pool; marking unsafe");
    false
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
                    &format!("Failed to analyze token {}: {}", mint, e)
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
    /// Create new security database instance (uses dedicated data/security.db)
    pub fn new() -> Result<Self, ScreenerBotError> {
        use rusqlite::Connection;

        // Create/open dedicated security database
        let conn = Connection::open("data/security.db").map_err(|e|
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to open security.db: {}", e),
            })
        )?;

        // Configure connection similarly to other DBs
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

        // Create security table if not exists
        conn
            .execute(
                "CREATE TABLE IF NOT EXISTS security (
                mint TEXT PRIMARY KEY,
                mint_authority_disabled INTEGER NOT NULL,
                freeze_authority_disabled INTEGER NOT NULL,
                lp_is_safe INTEGER NOT NULL,
                holder_count INTEGER NOT NULL,
                is_safe INTEGER NOT NULL,
                analyzed_at TEXT NOT NULL,
                risk_level TEXT NOT NULL
            )",
                []
            )
            .map_err(|e|
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to create security table: {}", e),
                })
            )?;

        // Optional migration: copy existing security_* columns from tokens.db if present
        // This is read-only for tokens.db and idempotent (upserts into security table)
        match rusqlite::Connection::open("data/tokens.db") {
            Ok(tokens_conn) => {
                // Best-effort busy timeout
                let _ = tokens_conn.busy_timeout(std::time::Duration::from_secs(5));

                // Try to prepare a statement that will only work if columns exist
                let prep = tokens_conn.prepare(
                    "SELECT mint,
                            security_mint_authority_disabled,
                            security_freeze_authority_disabled,
                            security_lp_is_safe,
                            security_holder_count,
                            security_is_safe,
                            security_analyzed_at,
                            security_risk_level
                     FROM tokens
                     WHERE security_analyzed_at IS NOT NULL"
                );

                if let Ok(mut stmt) = prep {
                    let mut migrated = 0usize;
                    let rows = stmt.query_map([], |row| {
                        // Extract row values with defaults
                        let mint: String = row.get(0)?;
                        let ma: Option<i32> = row.get(1).ok();
                        let fa: Option<i32> = row.get(2).ok();
                        let lp: Option<i32> = row.get(3).ok();
                        let hc: Option<i64> = row.get(4).ok();
                        let is: Option<i32> = row.get(5).ok();
                        let ts: String = row.get(6)?;
                        let rl: Option<String> = row.get(7).ok();

                        Ok((mint, ma, fa, lp, hc, is, ts, rl))
                    });

                    if let Ok(iter) = rows {
                        for r in iter.flatten() {
                            let (mint, ma, fa, lp, hc, is, ts, rl) = r;
                            // Parse timestamp; skip row if invalid
                            let parsed_ts = chrono::DateTime
                                ::parse_from_rfc3339(&ts)
                                .map(|dt| dt.with_timezone(&chrono::Utc));
                            if parsed_ts.is_err() {
                                continue;
                            }

                            let _ = conn.execute(
                                "INSERT INTO security (
                                    mint, mint_authority_disabled, freeze_authority_disabled, lp_is_safe,
                                    holder_count, is_safe, analyzed_at, risk_level
                                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                                ON CONFLICT(mint) DO UPDATE SET
                                    mint_authority_disabled = excluded.mint_authority_disabled,
                                    freeze_authority_disabled = excluded.freeze_authority_disabled,
                                    lp_is_safe = excluded.lp_is_safe,
                                    holder_count = excluded.holder_count,
                                    is_safe = excluded.is_safe,
                                    analyzed_at = excluded.analyzed_at,
                                    risk_level = excluded.risk_level",
                                rusqlite::params![
                                    mint,
                                    ma.unwrap_or(0),
                                    fa.unwrap_or(0),
                                    lp.unwrap_or(0),
                                    hc.unwrap_or(0) as i64,
                                    is.unwrap_or(0),
                                    parsed_ts.unwrap().to_rfc3339(),
                                    rl.unwrap_or_else(|| "UNKNOWN".to_string())
                                ]
                            );
                            migrated += 1;
                        }

                        if migrated > 0 {
                            log(
                                LogTag::Security,
                                "MIGRATE",
                                &format!("Migrated {} security rows from tokens.db to security.db", migrated)
                            );
                        }
                    }
                }
            }
            Err(_) => {
                // tokens.db not present or cannot be opened; skip migration silently
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
                "SELECT mint_authority_disabled, freeze_authority_disabled, 
                        lp_is_safe, holder_count, is_safe, analyzed_at, risk_level
                 FROM security WHERE mint = ?1"
            )
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to prepare select statement (security): {}", e),
                })
            })?;

        let result = stmt.query_row([mint], |row| {
            let analyzed_at_str: String = row.get(5)?;
            let analyzed_at = chrono::DateTime
                ::parse_from_rfc3339(&analyzed_at_str)
                .map_err(|_|
                    rusqlite::Error::InvalidColumnType(
                        5,
                        "analyzed_at".to_string(),
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

        // Upsert into dedicated security table
        conn
            .execute(
                "INSERT INTO security (
                mint, mint_authority_disabled, freeze_authority_disabled, lp_is_safe,
                holder_count, is_safe, analyzed_at, risk_level
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(mint) DO UPDATE SET
                mint_authority_disabled = excluded.mint_authority_disabled,
                freeze_authority_disabled = excluded.freeze_authority_disabled,
                lp_is_safe = excluded.lp_is_safe,
                holder_count = excluded.holder_count,
                is_safe = excluded.is_safe,
                analyzed_at = excluded.analyzed_at,
                risk_level = excluded.risk_level",
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
                    message: format!("Failed to upsert security info: {}", e),
                })
            })?;

        log(LogTag::Security, "STORE", &format!("Stored security info for {}", info.mint));

        Ok(())
    }

    /// Get all tokens that don't have security info cached
    pub fn get_tokens_without_security(&self) -> Result<Vec<String>, ScreenerBotError> {
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

        // Read all token mints from tokens.db and return those not in security.db
        let tokens_conn = rusqlite::Connection::open("data/tokens.db").map_err(|e|
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to open tokens.db for reading mints: {}", e),
            })
        )?;
        tokens_conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set busy timeout (tokens.db): {}", e),
            })
        })?;

        let mut stmt_tokens = tokens_conn
            .prepare("SELECT mint FROM tokens ORDER BY mint")
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to prepare tokens query: {}", e),
                })
            })?;

        let token_rows = stmt_tokens
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to execute tokens query: {}", e),
                })
            })?;

        let mut missing: Vec<String> = Vec::new();
        let mut exists_stmt = conn
            .prepare("SELECT 1 FROM security WHERE mint = ?1 LIMIT 1")
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to prepare existence check: {}", e),
                })
            })?;

        for r in token_rows {
            if let Ok(mint) = r {
                let mut has = false;
                let mut q = exists_stmt.query([&mint]).map_err(|e|
                    ScreenerBotError::Data(crate::errors::DataError::Generic {
                        message: format!("Failed to query security existence: {}", e),
                    })
                )?;
                if let Ok(Some(_row)) = q.next() {
                    has = true;
                }
                if !has {
                    missing.push(mint);
                }
            }
        }

        log(
            LogTag::Security,
            "UNCACHED_QUERY",
            &format!("Found {} tokens without security info", missing.len())
        );

        Ok(missing)
    }

    /// Get limited number of tokens for security scan cycle (similar to monitor service)
    pub fn get_tokens_for_security_scan(
        &self,
        limit: usize
    ) -> Result<Vec<String>, ScreenerBotError> {
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

        // Read token mints from tokens.db and return those not in security.db, limited by count
        let tokens_conn = rusqlite::Connection::open("data/tokens.db").map_err(|e|
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to open tokens.db for reading mints: {}", e),
            })
        )?;
        tokens_conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set busy timeout (tokens.db): {}", e),
            })
        })?;

        // Order by creation time or mint to ensure consistent priority
        let mut stmt_tokens = tokens_conn
            .prepare("SELECT mint FROM tokens ORDER BY mint LIMIT ?1")
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to prepare tokens query: {}", e),
                })
            })?;

        // We query more than the limit to account for tokens that already have security info
        let query_limit = (limit * 3).max(100); // Query 3x the limit or at least 100
        let token_rows = stmt_tokens
            .query_map([query_limit], |row| row.get::<_, String>(0))
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to execute tokens query: {}", e),
                })
            })?;

        let mut missing: Vec<String> = Vec::new();
        let mut exists_stmt = conn
            .prepare("SELECT 1 FROM security WHERE mint = ?1 LIMIT 1")
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to prepare existence check: {}", e),
                })
            })?;

        for r in token_rows {
            if let Ok(mint) = r {
                // Stop if we've reached our limit
                if missing.len() >= limit {
                    break;
                }

                let mut has = false;
                let mut q = exists_stmt.query([&mint]).map_err(|e|
                    ScreenerBotError::Data(crate::errors::DataError::Generic {
                        message: format!("Failed to query security existence: {}", e),
                    })
                )?;
                if let Ok(Some(_row)) = q.next() {
                    has = true;
                }
                if !has {
                    missing.push(mint);
                }
            }
        }

        log(
            LogTag::Security,
            "CYCLE_TOKENS",
            &format!("Selected {} tokens for security scan cycle (limit: {})", missing.len(), limit)
        );

        Ok(missing)
    }

    /// Get count of tokens without security info
    pub fn count_tokens_without_security(&self) -> Result<i64, ScreenerBotError> {
        // Don't hold the lock while calling get_tokens_without_security to avoid deadlock
        let list = self.get_tokens_without_security()?;
        Ok(list.len() as i64)
    }

    /// Get count of tokens with security info
    pub fn count_tokens_with_security(&self) -> Result<i64, ScreenerBotError> {
        let conn = self.connection.lock().map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to lock database: {}", e),
            })
        })?;

        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set busy timeout: {}", e),
            })
        })?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM security", [], |row| row.get(0))
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to count tokens with security: {}", e),
                })
            })?;

        Ok(count)
    }

    /// Get count of safe tokens
    pub fn count_safe_tokens(&self) -> Result<i64, ScreenerBotError> {
        let conn = self.connection.lock().map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to lock database: {}", e),
            })
        })?;

        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set busy timeout: {}", e),
            })
        })?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM security WHERE is_safe = 1", [], |row| row.get(0))
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to count safe tokens: {}", e),
                })
            })?;

        Ok(count)
    }

    /// Get count of unsafe tokens
    pub fn count_unsafe_tokens(&self) -> Result<i64, ScreenerBotError> {
        let conn = self.connection.lock().map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to lock database: {}", e),
            })
        })?;

        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set busy timeout: {}", e),
            })
        })?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM security WHERE is_safe = 0", [], |row| row.get(0))
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to count unsafe tokens: {}", e),
                })
            })?;

        Ok(count)
    }

    /// Get counts by risk level
    pub fn get_risk_level_counts(&self) -> Result<SecurityRiskCounts, ScreenerBotError> {
        let conn = self.connection.lock().map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to lock database: {}", e),
            })
        })?;

        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to set busy timeout: {}", e),
            })
        })?;

        let mut stmt = conn
            .prepare("SELECT risk_level, COUNT(*) FROM security GROUP BY risk_level")
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to prepare risk level query: {}", e),
                })
            })?;

        let rows = stmt
            .query_map([], |row| {
                let risk_level: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((risk_level, count as usize))
            })
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to execute risk level query: {}", e),
                })
            })?;

        let mut counts = SecurityRiskCounts::default();
        for row in rows {
            if let Ok((level, count)) = row {
                match level.as_str() {
                    "SAFE" => {
                        counts.safe = count;
                    }
                    "LOW" => {
                        counts.low = count;
                    }
                    "MEDIUM" => {
                        counts.medium = count;
                    }
                    "HIGH" => {
                        counts.high = count;
                    }
                    "CRITICAL" => {
                        counts.critical = count;
                    }
                    _ => {
                        counts.unknown = count;
                    }
                }
            }
        }

        Ok(counts)
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
                        &format!("Using cached security info for {} (age: {}h)", mint, age_hours)
                    );
                    return Ok(cached_info);
                }
            }
        }

        // Cache miss or stale data - fetch from API
        log(LogTag::Security, "CACHE_MISS", &format!("Fetching fresh security data for {}", mint));

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
                        &format!("Failed to analyze token {}: {}", mint, e)
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

// =============================================================================
// SECURITY STATS & SUMMARY (similar to monitor.rs)
// =============================================================================

#[derive(Debug, Clone, Default)]
struct SecurityStats {
    total_cycles: u64,
    total_analyzed: u64,
    total_safe: u64,
    total_unsafe: u64,
    last_cycle_started: Option<DateTime<Utc>>,
    last_cycle_completed: Option<DateTime<Utc>>,
    last_cycle_analyzed: usize,
    last_cycle_safe: usize,
    last_cycle_unsafe: usize,
    last_cycle_errors: usize,
    last_error: Option<String>,

    // API health tracking
    api_status: String,
    last_api_check: Option<DateTime<Utc>>,

    // Database counts (snapshot)
    tokens_with_security: usize,
    tokens_without_security: usize,
    safe_tokens_count: usize,
    unsafe_tokens_count: usize,

    // 30-second interval aggregation
    interval_started: Option<DateTime<Utc>>,
    interval_cycles: u64,
    interval_analyzed: usize,
    interval_safe: usize,
    interval_unsafe: usize,
    interval_errors: usize,
    interval_duration_ms_sum: u128,
}

#[derive(Debug, Clone, Default)]
struct SecurityRiskCounts {
    safe: usize,
    low: usize,
    medium: usize,
    high: usize,
    critical: usize,
    unknown: usize,
}

static SECURITY_STATS: OnceLock<Arc<RwLock<SecurityStats>>> = OnceLock::new();

fn get_security_stats_handle() -> Arc<RwLock<SecurityStats>> {
    SECURITY_STATS.get_or_init(|| Arc::new(RwLock::new(SecurityStats::default()))).clone()
}

async fn get_security_stats() -> SecurityStats {
    let stats_handle = get_security_stats_handle();
    let stats = stats_handle.read().await.clone();
    stats
}

/// Single comprehensive summary log per ~30s interval
async fn print_security_interval_summary() {
    let stats = get_security_stats().await;

    // Emoji based on effectiveness and safety
    let emoji = if stats.interval_analyzed > 0 {
        if stats.interval_safe > stats.interval_unsafe { "🟢" } else { "🟡" }
    } else {
        "⏸️"
    };

    // Average duration per cycle
    let avg_ms = if stats.interval_cycles > 0 {
        stats.interval_duration_ms_sum / (stats.interval_cycles as u64 as u128)
    } else {
        0
    };

    // Calculate percentages
    let total_tokens = stats.tokens_with_security + stats.tokens_without_security;
    let coverage_pct = if total_tokens > 0 {
        ((stats.tokens_with_security as f64) / (total_tokens as f64)) * 100.0
    } else {
        0.0
    };

    let safety_pct = if stats.tokens_with_security > 0 {
        ((stats.safe_tokens_count as f64) / (stats.tokens_with_security as f64)) * 100.0
    } else {
        0.0
    };

    let header_line = "════════════════════════════════════════════════════════════";
    let title = format!("{} SECURITY SUMMARY (last ~30s)", emoji);
    let cycles_line = format!("  • Cycles    🔁  {}", stats.interval_cycles);
    let analyzed_line = format!(
        "  • Analyzed  🔍  {}  |  Safe 🟢  {}  |  Unsafe 🔴  {}  |  Errors ❌  {}",
        stats.interval_analyzed,
        stats.interval_safe,
        stats.interval_unsafe,
        stats.interval_errors
    );
    let timing_line = format!("  • Avg cycle 🕒  {} ms", avg_ms);

    let database_line = format!(
        "  • Database  📊  Total: {}  |  With security: {} ({:.1}%)  |  Without: {}",
        total_tokens,
        stats.tokens_with_security,
        coverage_pct,
        stats.tokens_without_security
    );

    let safety_line = format!(
        "  • Safety    🛡️  Safe: {} ({:.1}%)  |  Unsafe: {}  |  API: {}",
        stats.safe_tokens_count,
        safety_pct,
        stats.unsafe_tokens_count,
        stats.api_status
    );

    let api_info = if let Some(last_check) = stats.last_api_check {
        let minutes_ago = (Utc::now() - last_check).num_minutes();
        format!("\n  • API Check ✅  {} minutes ago", minutes_ago)
    } else {
        "\n  • API Check ❓  Never checked".to_string()
    };

    let body = format!(
        "\n{header}\n{title}\n{header}\n{cycles}\n{analyzed}\n{timing}\n{database}\n{safety}{api}\n{header}",
        header = header_line,
        title = title,
        cycles = cycles_line,
        analyzed = analyzed_line,
        timing = timing_line,
        database = database_line,
        safety = safety_line,
        api = api_info
    );

    log(LogTag::Security, "SUMMARY", &body);
}

/// Start security monitoring background task (proactive security checking)
pub async fn start_security_monitoring(
    shutdown: Arc<Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    log(
        LogTag::Security,
        "MONITOR_START",
        "Security monitoring service started - proactive token security analysis enabled"
    );

    // Get security analyzer instance
    let analyzer = get_security_analyzer();

    let handle = tokio::spawn(async move {
        let mut check_interval = tokio::time::interval(std::time::Duration::from_secs(1800)); // 30 minutes
        check_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        // Initialize stats tracking
        {
            let stats_handle = get_security_stats_handle();
            let mut stats = stats_handle.write().await;
            stats.interval_started = Some(Utc::now());
            stats.api_status = "UNKNOWN".to_string();
        }

        // Initial startup scan
        log(LogTag::Security, "STARTUP_SCAN", "Starting initial security scan for uncached tokens");

        let cycle_start = std::time::Instant::now();
        match run_security_scan(&analyzer).await {
            Ok(scan_stats) => {
                let cycle_duration_ms = cycle_start.elapsed().as_millis();

                // Update stats
                {
                    let stats_handle = get_security_stats_handle();
                    let mut stats = stats_handle.write().await;
                    stats.total_cycles = stats.total_cycles.saturating_add(1);
                    stats.total_analyzed = stats.total_analyzed.saturating_add(
                        scan_stats.successful as u64
                    );
                    stats.total_safe = stats.total_safe.saturating_add(
                        scan_stats.safe_count as u64
                    );
                    stats.total_unsafe = stats.total_unsafe.saturating_add(
                        scan_stats.unsafe_count as u64
                    );
                    stats.last_cycle_started = Some(Utc::now());
                    stats.last_cycle_completed = Some(Utc::now());
                    stats.last_cycle_analyzed = scan_stats.successful;
                    stats.last_cycle_safe = scan_stats.safe_count;
                    stats.last_cycle_unsafe = scan_stats.unsafe_count;
                    stats.last_cycle_errors = scan_stats.failed;

                    // Interval tracking
                    stats.interval_cycles = stats.interval_cycles.saturating_add(1);
                    stats.interval_analyzed = stats.interval_analyzed.saturating_add(
                        scan_stats.successful
                    );
                    stats.interval_safe = stats.interval_safe.saturating_add(scan_stats.safe_count);
                    stats.interval_unsafe = stats.interval_unsafe.saturating_add(
                        scan_stats.unsafe_count
                    );
                    stats.interval_errors = stats.interval_errors.saturating_add(scan_stats.failed);
                    stats.interval_duration_ms_sum =
                        stats.interval_duration_ms_sum.saturating_add(cycle_duration_ms);

                    // Update database counts
                    if let Ok(with_security) = analyzer.database.count_tokens_with_security() {
                        stats.tokens_with_security = with_security as usize;
                    }
                    if let Ok(without_security) = analyzer.database.count_tokens_without_security() {
                        stats.tokens_without_security = without_security as usize;
                    }
                    if let Ok(safe_count) = analyzer.database.count_safe_tokens() {
                        stats.safe_tokens_count = safe_count as usize;
                    }
                    if let Ok(unsafe_count) = analyzer.database.count_unsafe_tokens() {
                        stats.unsafe_tokens_count = unsafe_count as usize;
                    }
                }

                log(
                    LogTag::Security,
                    "STARTUP_COMPLETE",
                    &format!(
                        "Initial scan complete: {} processed, {} successful, {} failed",
                        scan_stats.processed,
                        scan_stats.successful,
                        scan_stats.failed
                    )
                );
            }
            Err(e) => {
                // Update error in stats
                {
                    let stats_handle = get_security_stats_handle();
                    let mut stats = stats_handle.write().await;
                    stats.last_error = Some(format!("Startup scan failed: {}", e));
                    stats.interval_errors = stats.interval_errors.saturating_add(1);
                }

                log(
                    LogTag::Security,
                    "STARTUP_ERROR",
                    &format!("Initial security scan failed: {}", e)
                );
            }
        }

        // Periodic scanning and summary loop
        let mut summary_interval = tokio::time::interval(std::time::Duration::from_secs(30));
        summary_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Security, "MONITOR_STOP", "Security monitoring service stopping");
                    break;
                }
                _ = check_interval.tick() => {
                    let cycle_start = std::time::Instant::now();
                    
                    log(LogTag::Security, "PERIODIC_SCAN", "Starting periodic security scan");
                    
                    match run_security_scan(&analyzer).await {
                        Ok(scan_stats) => {
                            let cycle_duration_ms = cycle_start.elapsed().as_millis();
                            
                            // Update stats
                            {
                                let stats_handle = get_security_stats_handle();
                                let mut stats = stats_handle.write().await;
                                stats.total_cycles = stats.total_cycles.saturating_add(1);
                                stats.total_analyzed = stats.total_analyzed.saturating_add(scan_stats.successful as u64);
                                stats.total_safe = stats.total_safe.saturating_add(scan_stats.safe_count as u64);
                                stats.total_unsafe = stats.total_unsafe.saturating_add(scan_stats.unsafe_count as u64);
                                stats.last_cycle_started = Some(Utc::now());
                                stats.last_cycle_completed = Some(Utc::now());
                                stats.last_cycle_analyzed = scan_stats.successful;
                                stats.last_cycle_safe = scan_stats.safe_count;
                                stats.last_cycle_unsafe = scan_stats.unsafe_count;
                                stats.last_cycle_errors = scan_stats.failed;
                                
                                // Interval tracking
                                stats.interval_cycles = stats.interval_cycles.saturating_add(1);
                                stats.interval_analyzed = stats.interval_analyzed.saturating_add(scan_stats.successful);
                                stats.interval_safe = stats.interval_safe.saturating_add(scan_stats.safe_count);
                                stats.interval_unsafe = stats.interval_unsafe.saturating_add(scan_stats.unsafe_count);
                                stats.interval_errors = stats.interval_errors.saturating_add(scan_stats.failed);
                                stats.interval_duration_ms_sum = stats.interval_duration_ms_sum.saturating_add(cycle_duration_ms);
                                
                                // Update database counts periodically
                                if let Ok(with_security) = analyzer.database.count_tokens_with_security() {
                                    stats.tokens_with_security = with_security as usize;
                                }
                                if let Ok(without_security) = analyzer.database.count_tokens_without_security() {
                                    stats.tokens_without_security = without_security as usize;
                                }
                                if let Ok(safe_count) = analyzer.database.count_safe_tokens() {
                                    stats.safe_tokens_count = safe_count as usize;
                                }
                                if let Ok(unsafe_count) = analyzer.database.count_unsafe_tokens() {
                                    stats.unsafe_tokens_count = unsafe_count as usize;
                                }
                            }
                            
                            if scan_stats.processed > 0 {
                                log(
                                    LogTag::Security,
                                    "PERIODIC_COMPLETE",
                                    &format!("Periodic scan complete: {} processed, {} successful, {} failed", 
                                            scan_stats.processed, scan_stats.successful, scan_stats.failed)
                                );
                            } else {
                                log(LogTag::Security, "PERIODIC_SKIP", "No new tokens to analyze");
                            }
                        }
                        Err(e) => {
                            // Update error in stats
                            {
                                let stats_handle = get_security_stats_handle();
                                let mut stats = stats_handle.write().await;
                                stats.last_error = Some(format!("Periodic scan failed: {}", e));
                                stats.interval_errors = stats.interval_errors.saturating_add(1);
                            }
                            
                            log(LogTag::Security, "PERIODIC_ERROR", &format!("Periodic security scan failed: {}", e));
                        }
                    }
                }
                _ = summary_interval.tick() => {
                    // Check API status
                    match check_api_status().await {
                        Ok(is_ok) => {
                            let status = if is_ok { "OK" } else { "ERROR" };
                            {
                                let stats_handle = get_security_stats_handle();
                                let mut stats = stats_handle.write().await;
                                stats.api_status = status.to_string();
                                stats.last_api_check = Some(Utc::now());
                            }
                        }
                        Err(_) => {
                            {
                                let stats_handle = get_security_stats_handle();
                                let mut stats = stats_handle.write().await;
                                stats.api_status = "UNREACHABLE".to_string();
                                stats.last_api_check = Some(Utc::now());
                            }
                        }
                    }

                    print_security_interval_summary().await;

                    // Reset interval
                    {
                        let stats_handle = get_security_stats_handle();
                        let mut stats = stats_handle.write().await;
                        stats.interval_started = Some(Utc::now());
                        stats.interval_cycles = 0;
                        stats.interval_analyzed = 0;
                        stats.interval_safe = 0;
                        stats.interval_unsafe = 0;
                        stats.interval_errors = 0;
                        stats.interval_duration_ms_sum = 0;
                    }
                }
            }
        }

        log(LogTag::Security, "MONITOR_STOP", "Security monitoring service stopped");
    });

    Ok(handle)
}

#[derive(Debug)]
struct ScanStats {
    processed: usize,
    successful: usize,
    failed: usize,
    safe_count: usize,
    unsafe_count: usize,
}

/// Run a security scan cycle for a limited number of tokens (similar to monitor service)
async fn run_security_scan(
    analyzer: &TokenSecurityAnalyzer
) -> Result<ScanStats, ScreenerBotError> {
    // Acquire global scan mutex to prevent overlapping scans (drop before awaits)
    {
        let scan_mutex = SECURITY_SCAN_MUTEX.get_or_init(|| Arc::new(StdMutex::new(()))).clone();
        if scan_mutex.try_lock().is_err() {
            log(
                LogTag::Security,
                "SCAN_SKIP_LOCK",
                "Another security scan is already running; skipping this invocation"
            );
            return Ok(ScanStats {
                processed: 0,
                successful: 0,
                failed: 0,
                safe_count: 0,
                unsafe_count: 0,
            });
        }
        // guard dropped here at end of block
    }

    // Check API status first
    if !check_api_status().await? {
        return Err(
            ScreenerBotError::Network(crate::errors::NetworkError::Generic {
                message: "Rugcheck API is not operational".to_string(),
            })
        );
    }

    // Get limited number of tokens for this cycle (not all tokens)
    let tokens_to_check =
        analyzer.database.get_tokens_for_security_scan(SECURITY_TOKENS_PER_CYCLE)?;

    if tokens_to_check.is_empty() {
        // Also log total backlog for context
        if let Ok(total_uncached) = analyzer.database.count_tokens_without_security() {
            if total_uncached > 0 {
                log(
                    LogTag::Security,
                    "CYCLE_SKIP",
                    &format!("No tokens selected for this cycle (total backlog: {})", total_uncached)
                );
            } else {
                log(LogTag::Security, "CYCLE_SKIP", "All tokens have security info cached");
            }
        }
        return Ok(ScanStats {
            processed: 0,
            successful: 0,
            failed: 0,
            safe_count: 0,
            unsafe_count: 0,
        });
    }

    // Log cycle start with backlog context
    let total_uncached = analyzer.database.count_tokens_without_security().unwrap_or(0);
    log(
        LogTag::Security,
        "CYCLE_START",
        &format!(
            "Starting security scan cycle: {} tokens selected (total backlog: {})",
            tokens_to_check.len(),
            total_uncached
        )
    );

    let mut stats = ScanStats {
        processed: 0,
        successful: 0,
        failed: 0,
        safe_count: 0,
        unsafe_count: 0,
    };

    // Process tokens in batches to respect rate limits
    for (batch_idx, batch) in tokens_to_check.chunks(SECURITY_BATCH_SIZE).enumerate() {
        log(
            LogTag::Security,
            "BATCH_START",
            &format!(
                "Processing batch {}/{} ({} tokens in this cycle)",
                batch_idx + 1,
                (tokens_to_check.len() + SECURITY_BATCH_SIZE - 1) / SECURITY_BATCH_SIZE,
                batch.len()
            )
        );

        for (token_idx, mint) in batch.iter().enumerate() {
            // Add delay between requests to be respectful to API
            if token_idx > 0 {
                tokio::time::sleep(
                    std::time::Duration::from_millis(SECURITY_REQUEST_DELAY_MS)
                ).await;
            }

            match analyzer.analyze_token_security_with_cache(mint, false).await {
                Ok(security_info) => {
                    stats.successful += 1;
                    if security_info.is_safe {
                        stats.safe_count += 1;
                    } else {
                        stats.unsafe_count += 1;
                    }
                    log(
                        LogTag::Security,
                        "PROCESS_SUCCESS",
                        &format!("✅ {} - {}", mint, security_info.summary())
                    );
                }
                Err(e) => {
                    stats.failed += 1;
                    log(LogTag::Security, "PROCESS_FAILED", &format!("❌ {} - Error: {}", mint, e));
                }
            }

            stats.processed += 1;
        }

        // Add delay between batches (except for last batch)
        if batch_idx < (tokens_to_check.len() + SECURITY_BATCH_SIZE - 1) / SECURITY_BATCH_SIZE - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(SECURITY_BATCH_DELAY_MS)).await;
        }
    }

    log(
        LogTag::Security,
        "CYCLE_COMPLETE",
        &format!(
            "Security scan cycle finished: {}/{} successful ({:.1}% success rate) | Safe: {} | Unsafe: {}",
            stats.successful,
            stats.processed,
            if stats.processed > 0 {
                ((stats.successful as f64) / (stats.processed as f64)) * 100.0
            } else {
                0.0
            },
            stats.safe_count,
            stats.unsafe_count
        )
    );

    Ok(stats)
}
