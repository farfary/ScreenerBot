/// Token Security Analysis Module
///
/// This module provides comprehensive security analysis for Solana tokens by combining
/// multiple security checks including authority analysis, holder distribution, and LP lock status.
///
/// Features:
/// - Batch processing with get_multiple_accounts for efficiency
/// - Database and memory caching for performance
/// - Background service integration
/// - Static vs dynamic security property tracking
/// - Comprehensive security scoring

use crate::{
    errors::ScreenerBotError,
    logger::{ log, LogTag },
    rpc::get_rpc_client,
    tokens::{
        authority::{
            get_token_authorities,
            get_multiple_token_authorities,
            TokenAuthorities,
            TokenRiskLevel,
        },
        holders::{
            get_holder_stats,
            get_top_holders_analysis,
            should_skip_holder_analysis,
            get_token_account_count_estimate,
            HolderStats,
            TopHoldersAnalysis,
        },
        lp_lock::{ check_lp_lock_status, check_multiple_lp_locks, LpLockAnalysis, LpLockStatus },
    },
    utils::safe_truncate,
};

use chrono::{ DateTime, Utc, Duration as ChronoDuration };
use rusqlite::{ params, Connection, Result as SqliteResult };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::path::Path;
use std::sync::{ Arc, RwLock };
use std::time::Instant;
use tokio::sync::RwLock as AsyncRwLock;

/// Comprehensive security analysis result for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSecurityInfo {
    /// Token mint address
    pub mint: String,
    /// Overall security score (0-100, higher is safer)
    pub security_score: u8,
    /// Risk level based on combined analysis
    pub risk_level: SecurityRiskLevel,
    /// Authority analysis (static - rarely changes)
    pub authority_info: TokenAuthorities,
    /// Holder distribution analysis (dynamic - changes frequently)
    pub holder_info: Option<HolderSecurityInfo>,
    /// LP lock analysis (static - rarely changes)
    pub lp_lock_info: Option<LpLockAnalysis>,
    /// Analysis timestamps
    pub timestamps: SecurityTimestamps,
    /// Security flags and warnings
    pub security_flags: SecurityFlags,
    /// Last update strategy used
    pub update_strategy: UpdateStrategy,
}

/// Security-focused holder information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolderSecurityInfo {
    /// Total number of holders
    pub total_holders: u32,
    /// Top 10 concentration percentage
    pub top_10_concentration: f64,
    /// Top 5 concentration percentage
    pub top_5_concentration: f64,
    /// Single largest holder percentage
    pub largest_holder_percentage: f64,
    /// Number of holders with >5% supply
    pub whale_count: u32,
    /// Average holder balance
    pub average_balance: f64,
    /// Holder distribution score (0-100)
    pub distribution_score: u8,
}

/// Security risk levels
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
            SecurityRiskLevel::Critical => "🚨",
            SecurityRiskLevel::Unknown => "⚪",
        }
    }

    fn from_score(score: u8) -> Self {
        match score {
            90..=100 => SecurityRiskLevel::Safe,
            75..=89 => SecurityRiskLevel::Low,
            50..=74 => SecurityRiskLevel::Medium,
            25..=49 => SecurityRiskLevel::High,
            1..=24 => SecurityRiskLevel::Critical,
            0 => SecurityRiskLevel::Unknown,
            _ => SecurityRiskLevel::Unknown, // Handle any other values
        }
    }
}

/// Security analysis timestamps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityTimestamps {
    /// When the full analysis was first performed
    pub first_analyzed: DateTime<Utc>,
    /// When the analysis was last updated
    pub last_updated: DateTime<Utc>,
    /// When authority info was last checked (static)
    pub authority_last_checked: DateTime<Utc>,
    /// When holder info was last checked (dynamic)
    pub holder_last_checked: Option<DateTime<Utc>>,
    /// When LP lock info was last checked (static)
    pub lp_lock_last_checked: Option<DateTime<Utc>>,
}

/// Security flags and warnings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFlags {
    /// Can mint new tokens
    pub can_mint: bool,
    /// Can freeze accounts
    pub can_freeze: bool,
    /// Has update authority
    pub has_update_authority: bool,
    /// LP is locked/burned
    pub lp_locked: bool,
    /// High holder concentration (>50% in top 10)
    pub high_concentration: bool,
    /// Very few holders (<50)
    pub few_holders: bool,
    /// Potential whale manipulation (single holder >20%)
    pub whale_risk: bool,
    /// Unknown or failed to analyze
    pub analysis_incomplete: bool,
}

/// Update strategy for security info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateStrategy {
    /// Full analysis with all checks
    Full,
    /// Partial update (some properties updated)
    Partial,
    /// Cached data, no updates needed
    Cached,
}

/// Security database manager
pub struct SecurityDatabase {
    db_path: String,
}

impl SecurityDatabase {
    /// Create new security database manager
    pub fn new(db_path: &str) -> Result<Self, ScreenerBotError> {
        let db = Self {
            db_path: db_path.to_string(),
        };
        db.init_database()?;
        Ok(db)
    }

    /// Initialize database schema
    fn init_database(&self) -> Result<(), ScreenerBotError> {
        let conn = Connection::open(&self.db_path).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to open security database: {}", e),
            })
        })?;

        conn
            .execute(
                r#"
            CREATE TABLE IF NOT EXISTS token_security (
                mint TEXT PRIMARY KEY,
                security_score INTEGER NOT NULL,
                risk_level TEXT NOT NULL,
                authority_info TEXT NOT NULL,
                holder_info TEXT,
                lp_lock_info TEXT,
                timestamps TEXT NOT NULL,
                security_flags TEXT NOT NULL,
                update_strategy TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
                []
            )
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to create security table: {}", e),
                })
            })?;

        // Create index for efficient queries
        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_security_score ON token_security(security_score)",
                []
            )
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to create security index: {}", e),
                })
            })?;

        conn
            .execute("CREATE INDEX IF NOT EXISTS idx_risk_level ON token_security(risk_level)", [])
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to create risk index: {}", e),
                })
            })?;

        log(LogTag::Security, "INIT", "Security database initialized successfully");
        Ok(())
    }

    /// Save security info to database
    pub fn save_security_info(&self, info: &TokenSecurityInfo) -> Result<(), ScreenerBotError> {
        let conn = Connection::open(&self.db_path).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to open security database: {}", e),
            })
        })?;

        let authority_json = serde_json::to_string(&info.authority_info).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::ParseError {
                data_type: "authority_info".to_string(),
                error: e.to_string(),
            })
        })?;

        let holder_json = if let Some(ref holder_info) = info.holder_info {
            Some(
                serde_json::to_string(holder_info).map_err(|e| {
                    ScreenerBotError::Data(crate::errors::DataError::ParseError {
                        data_type: "holder_info".to_string(),
                        error: e.to_string(),
                    })
                })?
            )
        } else {
            None
        };

        let lp_lock_json = if let Some(ref lp_info) = info.lp_lock_info {
            Some(
                serde_json::to_string(lp_info).map_err(|e| {
                    ScreenerBotError::Data(crate::errors::DataError::ParseError {
                        data_type: "lp_lock_info".to_string(),
                        error: e.to_string(),
                    })
                })?
            )
        } else {
            None
        };

        let timestamps_json = serde_json::to_string(&info.timestamps).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::ParseError {
                data_type: "timestamps".to_string(),
                error: e.to_string(),
            })
        })?;

        let flags_json = serde_json::to_string(&info.security_flags).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::ParseError {
                data_type: "security_flags".to_string(),
                error: e.to_string(),
            })
        })?;

        let strategy_json = serde_json::to_string(&info.update_strategy).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::ParseError {
                data_type: "update_strategy".to_string(),
                error: e.to_string(),
            })
        })?;

        conn
            .execute(
                r#"
            INSERT OR REPLACE INTO token_security 
            (mint, security_score, risk_level, authority_info, holder_info, lp_lock_info, 
             timestamps, security_flags, update_strategy, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP)
            "#,
                params![
                    info.mint,
                    info.security_score,
                    info.risk_level.as_str(),
                    authority_json,
                    holder_json,
                    lp_lock_json,
                    timestamps_json,
                    flags_json,
                    strategy_json
                ]
            )
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to save security info: {}", e),
                })
            })?;

        Ok(())
    }

    /// Get security info from database
    pub fn get_security_info(
        &self,
        mint: &str
    ) -> Result<Option<TokenSecurityInfo>, ScreenerBotError> {
        let conn = Connection::open(&self.db_path).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to open security database: {}", e),
            })
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, security_score, risk_level, authority_info, holder_info, lp_lock_info, 
             timestamps, security_flags, update_strategy FROM token_security WHERE mint = ?1"
            )
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to prepare get security query: {}", e),
                })
            })?;

        let result = stmt.query_row(params![mint], |row| {
            let authority_json: String = row.get(3)?;
            let holder_json: Option<String> = row.get(4)?;
            let lp_lock_json: Option<String> = row.get(5)?;
            let timestamps_json: String = row.get(6)?;
            let flags_json: String = row.get(7)?;
            let strategy_json: String = row.get(8)?;

            Ok((
                row.get::<_, String>(0)?, // mint
                row.get::<_, u8>(1)?, // security_score
                row.get::<_, String>(2)?, // risk_level
                authority_json,
                holder_json,
                lp_lock_json,
                timestamps_json,
                flags_json,
                strategy_json,
            ))
        });

        match result {
            Ok(
                (
                    mint,
                    security_score,
                    risk_level_str,
                    authority_json,
                    holder_json,
                    lp_lock_json,
                    timestamps_json,
                    flags_json,
                    strategy_json,
                ),
            ) => {
                let authority_info: TokenAuthorities = serde_json
                    ::from_str(&authority_json)
                    .map_err(|e| {
                        ScreenerBotError::Data(crate::errors::DataError::ParseError {
                            data_type: "authority_info".to_string(),
                            error: e.to_string(),
                        })
                    })?;

                let holder_info: Option<HolderSecurityInfo> = if let Some(json) = holder_json {
                    Some(
                        serde_json::from_str(&json).map_err(|e| {
                            ScreenerBotError::Data(crate::errors::DataError::ParseError {
                                data_type: "holder_info".to_string(),
                                error: e.to_string(),
                            })
                        })?
                    )
                } else {
                    None
                };

                let lp_lock_info: Option<LpLockAnalysis> = if let Some(json) = lp_lock_json {
                    Some(
                        serde_json::from_str(&json).map_err(|e| {
                            ScreenerBotError::Data(crate::errors::DataError::ParseError {
                                data_type: "lp_lock_info".to_string(),
                                error: e.to_string(),
                            })
                        })?
                    )
                } else {
                    None
                };

                let timestamps: SecurityTimestamps = serde_json
                    ::from_str(&timestamps_json)
                    .map_err(|e| {
                        ScreenerBotError::Data(crate::errors::DataError::ParseError {
                            data_type: "timestamps".to_string(),
                            error: e.to_string(),
                        })
                    })?;

                let security_flags: SecurityFlags = serde_json::from_str(&flags_json).map_err(|e| {
                    ScreenerBotError::Data(crate::errors::DataError::ParseError {
                        data_type: "security_flags".to_string(),
                        error: e.to_string(),
                    })
                })?;

                let update_strategy: UpdateStrategy = serde_json
                    ::from_str(&strategy_json)
                    .map_err(|e| {
                        ScreenerBotError::Data(crate::errors::DataError::ParseError {
                            data_type: "update_strategy".to_string(),
                            error: e.to_string(),
                        })
                    })?;

                let risk_level = match risk_level_str.as_str() {
                    "SAFE" => SecurityRiskLevel::Safe,
                    "LOW" => SecurityRiskLevel::Low,
                    "MEDIUM" => SecurityRiskLevel::Medium,
                    "HIGH" => SecurityRiskLevel::High,
                    "CRITICAL" => SecurityRiskLevel::Critical,
                    _ => SecurityRiskLevel::Unknown,
                };

                Ok(
                    Some(TokenSecurityInfo {
                        mint,
                        security_score,
                        risk_level,
                        authority_info,
                        holder_info,
                        lp_lock_info,
                        timestamps,
                        security_flags,
                        update_strategy,
                    })
                )
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) =>
                Err(
                    ScreenerBotError::Data(crate::errors::DataError::Generic {
                        message: format!("Failed to get security info: {}", e),
                    })
                ),
        }
    }

    /// Get multiple security infos efficiently
    pub fn get_multiple_security_infos(
        &self,
        mints: &[String]
    ) -> Result<HashMap<String, TokenSecurityInfo>, ScreenerBotError> {
        if mints.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = Connection::open(&self.db_path).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to open security database: {}", e),
            })
        })?;

        let placeholders = mints
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query =
            format!("SELECT mint, security_score, risk_level, authority_info, holder_info, lp_lock_info, 
             timestamps, security_flags, update_strategy FROM token_security WHERE mint IN ({})", placeholders);

        let mut stmt = conn.prepare(&query).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to prepare get multiple security query: {}", e),
            })
        })?;

        let params: Vec<&dyn rusqlite::ToSql> = mints
            .iter()
            .map(|m| m as &dyn rusqlite::ToSql)
            .collect();
        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?, // mint
                    row.get::<_, u8>(1)?, // security_score
                    row.get::<_, String>(2)?, // risk_level
                    row.get::<_, String>(3)?, // authority_info
                    row.get::<_, Option<String>>(4)?, // holder_info
                    row.get::<_, Option<String>>(5)?, // lp_lock_info
                    row.get::<_, String>(6)?, // timestamps
                    row.get::<_, String>(7)?, // security_flags
                    row.get::<_, String>(8)?, // update_strategy
                ))
            })
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to query multiple security: {}", e),
                })
            })?;

        let mut result = HashMap::new();
        for row in rows {
            let (
                mint,
                security_score,
                risk_level_str,
                authority_json,
                holder_json,
                lp_lock_json,
                timestamps_json,
                flags_json,
                strategy_json,
            ) = row.map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to process security row: {}", e),
                })
            })?;

            // Parse JSON fields (with error handling for individual entries)
            if let Ok(authority_info) = serde_json::from_str::<TokenAuthorities>(&authority_json) {
                if
                    let Ok(timestamps) = serde_json::from_str::<SecurityTimestamps>(
                        &timestamps_json
                    )
                {
                    if let Ok(security_flags) = serde_json::from_str::<SecurityFlags>(&flags_json) {
                        if
                            let Ok(update_strategy) = serde_json::from_str::<UpdateStrategy>(
                                &strategy_json
                            )
                        {
                            let holder_info = holder_json.and_then(|json|
                                serde_json::from_str::<HolderSecurityInfo>(&json).ok()
                            );

                            let lp_lock_info = lp_lock_json.and_then(|json|
                                serde_json::from_str::<LpLockAnalysis>(&json).ok()
                            );

                            let risk_level = match risk_level_str.as_str() {
                                "SAFE" => SecurityRiskLevel::Safe,
                                "LOW" => SecurityRiskLevel::Low,
                                "MEDIUM" => SecurityRiskLevel::Medium,
                                "HIGH" => SecurityRiskLevel::High,
                                "CRITICAL" => SecurityRiskLevel::Critical,
                                _ => SecurityRiskLevel::Unknown,
                            };

                            result.insert(mint.clone(), TokenSecurityInfo {
                                mint,
                                security_score,
                                risk_level,
                                authority_info,
                                holder_info,
                                lp_lock_info,
                                timestamps,
                                security_flags,
                                update_strategy,
                            });
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Clean old security data (older than specified days)
    pub fn cleanup_old_data(&self, days: i64) -> Result<usize, ScreenerBotError> {
        let conn = Connection::open(&self.db_path).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Failed to open security database: {}", e),
            })
        })?;

        let cutoff_date = Utc::now() - ChronoDuration::days(days);
        let cutoff_str = cutoff_date.format("%Y-%m-%d %H:%M:%S").to_string();

        let deleted = conn
            .execute("DELETE FROM token_security WHERE updated_at < ?1", params![cutoff_str])
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::Generic {
                    message: format!("Failed to cleanup old security data: {}", e),
                })
            })?;

        log(LogTag::Security, "CLEANUP", &format!("Cleaned {} old security records", deleted));
        Ok(deleted)
    }
}

/// In-memory cache for security information
pub struct SecurityCache {
    cache: Arc<RwLock<HashMap<String, (TokenSecurityInfo, Instant)>>>,
    /// Cache TTL for different types of data
    static_ttl: std::time::Duration, // Authority, LP lock (rarely changes)
    dynamic_ttl: std::time::Duration, // Holder info (changes frequently)
}

impl SecurityCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            static_ttl: std::time::Duration::from_secs(3600 * 6), // 6 hours
            dynamic_ttl: std::time::Duration::from_secs(300), // 5 minutes
        }
    }

    /// Get security info from cache if valid
    pub fn get(&self, mint: &str) -> Option<TokenSecurityInfo> {
        let cache = self.cache.read().ok()?;
        if let Some((info, cached_at)) = cache.get(mint) {
            let age = cached_at.elapsed();

            // Use different TTL based on what data we have
            let ttl = if info.holder_info.is_some() { self.dynamic_ttl } else { self.static_ttl };

            if age < ttl {
                return Some(info.clone());
            }
        }
        None
    }

    /// Store security info in cache
    pub fn set(&self, info: TokenSecurityInfo) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(info.mint.clone(), (info, std::time::Instant::now()));
        }
    }

    /// Remove expired entries from cache
    pub fn cleanup_expired(&self) {
        if let Ok(mut cache) = self.cache.write() {
            let now = std::time::Instant::now();
            cache.retain(|_, (info, cached_at)| {
                let ttl = if info.holder_info.is_some() {
                    self.dynamic_ttl
                } else {
                    self.static_ttl
                };
                now.duration_since(*cached_at) < ttl
            });
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        if let Ok(cache) = self.cache.read() {
            let total = cache.len();
            let now = std::time::Instant::now();
            let (static_count, dynamic_count) = cache
                .values()
                .fold((0, 0), |(s, d), (info, cached_at)| {
                    let ttl = if info.holder_info.is_some() {
                        self.dynamic_ttl
                    } else {
                        self.static_ttl
                    };
                    if now.duration_since(*cached_at) < ttl {
                        if info.holder_info.is_some() { (s, d + 1) } else { (s + 1, d) }
                    } else {
                        (s, d)
                    }
                });
            (total, static_count, dynamic_count)
        } else {
            (0, 0, 0)
        }
    }
}

/// Main security analyzer
pub struct TokenSecurityAnalyzer {
    pub database: SecurityDatabase,
    pub cache: SecurityCache,
}

impl TokenSecurityAnalyzer {
    /// Create new security analyzer
    pub fn new(db_path: &str) -> Result<Self, ScreenerBotError> {
        let database = SecurityDatabase::new(db_path)?;
        let cache = SecurityCache::new();

        Ok(Self {
            database,
            cache,
        })
    }

    /// Analyze token security with caching and intelligent update strategy
    pub async fn analyze_token_security(
        &self,
        mint: &str
    ) -> Result<TokenSecurityInfo, ScreenerBotError> {
        self.analyze_token_security_with_options(mint, false).await
    }

    /// Analyze token security with force refresh option
    pub async fn analyze_token_security_with_options(
        &self,
        mint: &str,
        force_refresh: bool
    ) -> Result<TokenSecurityInfo, ScreenerBotError> {
        log(
            LogTag::Security,
            "DEBUG",
            &format!(
                "Starting security analysis for {} (force_refresh: {})",
                safe_truncate(mint, 8),
                force_refresh
            )
        );

        // Skip cache and database checks if force refresh is requested
        if !force_refresh {
            // Check cache first
            if let Some(cached_info) = self.cache.get(mint) {
                log(
                    LogTag::Security,
                    "DEBUG",
                    &format!("Using cached security info for {}", safe_truncate(mint, 8))
                );
                return Ok(cached_info);
            }

            // Check database
            if let Some(db_info) = self.database.get_security_info(mint)? {
                // Always try to update existing info - let update_security_info decide if update is needed
                log(
                    LogTag::Security,
                    "DEBUG",
                    &format!("Checking for updates needed for {}", safe_truncate(mint, 8))
                );
                return self.update_security_info(db_info).await;
            }
        } else {
            log(
                LogTag::Security,
                "DEBUG",
                &format!(
                    "Force refresh requested - bypassing cache and database for {}",
                    safe_truncate(mint, 8)
                )
            );
        }

        // No cached data or force refresh requested, perform full analysis
        log(
            LogTag::Security,
            "DEBUG",
            &format!("Performing full security analysis for {}", safe_truncate(mint, 8))
        );
        self.perform_full_security_analysis(mint).await
    }

    /// Batch analyze multiple tokens efficiently
    pub async fn analyze_multiple_tokens(
        &self,
        mints: &[String]
    ) -> Result<HashMap<String, TokenSecurityInfo>, ScreenerBotError> {
        if mints.is_empty() {
            return Ok(HashMap::new());
        }

        log(
            LogTag::Security,
            "BATCH_ANALYZE",
            &format!("Analyzing security for {} tokens", mints.len())
        );

        let mut results = HashMap::new();
        let mut needs_analysis = Vec::new();
        let mut needs_update = Vec::new();

        // Check cache and database for existing data
        for mint in mints {
            if let Some(cached_info) = self.cache.get(mint) {
                results.insert(mint.clone(), cached_info);
            } else if let Some(db_info) = self.database.get_security_info(mint)? {
                // Always try to update existing info - let update_security_info decide if update is needed
                needs_update.push((mint.clone(), db_info));
            } else {
                needs_analysis.push(mint.clone());
            }
        }

        // Batch process tokens that need full analysis
        if !needs_analysis.is_empty() {
            log(
                LogTag::Security,
                "BATCH_FULL",
                &format!("Performing full analysis for {} tokens", needs_analysis.len())
            );
            let full_analysis_results = self.batch_full_security_analysis(&needs_analysis).await?;
            results.extend(full_analysis_results);
        }

        // Update tokens that need refresh
        if !needs_update.is_empty() {
            log(
                LogTag::Security,
                "BATCH_UPDATE",
                &format!("Updating {} tokens", needs_update.len())
            );
            for (mint, old_info) in needs_update {
                if let Ok(updated_info) = self.update_security_info(old_info).await {
                    results.insert(mint, updated_info);
                }
            }
        }

        log(
            LogTag::Security,
            "BATCH_COMPLETE",
            &format!("Completed security analysis for {} tokens", results.len())
        );
        Ok(results)
    }

    /// Update existing security info
    async fn update_security_info(
        &self,
        mut old_info: TokenSecurityInfo
    ) -> Result<TokenSecurityInfo, ScreenerBotError> {
        let now = Utc::now();
        let mint = old_info.mint.clone();

        // Check what needs updating
        let static_age = now.signed_duration_since(old_info.timestamps.authority_last_checked);
        let static_needs_update = static_age > ChronoDuration::hours(6);

        let dynamic_needs_update = if
            let Some(holder_last_checked) = old_info.timestamps.holder_last_checked
        {
            let dynamic_age = now.signed_duration_since(holder_last_checked);
            dynamic_age > ChronoDuration::minutes(5)
        } else {
            true // No holder data, we should get it
        };

        // If no update is needed, cache and return existing info
        if !static_needs_update && !dynamic_needs_update {
            log(
                LogTag::Security,
                "DEBUG",
                &format!("No update needed for {}", safe_truncate(&mint, 8))
            );
            self.cache.set(old_info.clone());
            return Ok(old_info);
        }

        log(
            LogTag::Security,
            "UPDATE",
            &format!("Updating security info for {}", safe_truncate(&mint, 8))
        );

        // Update static properties if needed
        if static_needs_update {
            log(
                LogTag::Security,
                "UPDATE_STATIC",
                &format!("Updating static properties for {}", safe_truncate(&mint, 8))
            );

            // Update authority info
            if let Ok(new_authority) = get_token_authorities(&mint).await {
                old_info.authority_info = new_authority;
                old_info.timestamps.authority_last_checked = now;
            }

            // Update LP lock info
            if let Ok(new_lp_lock) = check_lp_lock_status(&mint).await {
                old_info.lp_lock_info = Some(new_lp_lock);
                old_info.timestamps.lp_lock_last_checked = Some(now);
            }
        }

        // Update dynamic properties if needed
        if dynamic_needs_update {
            log(
                LogTag::Security,
                "UPDATE_DYNAMIC",
                &format!("Updating dynamic properties for {}", safe_truncate(&mint, 8))
            );

            // Update holder info
            if let Ok(holder_stats) = get_holder_stats(&mint).await {
                if let Ok(top_holders) = get_top_holders_analysis(&mint, Some(10)).await {
                    old_info.holder_info = Some(
                        self.create_holder_security_info(&holder_stats, &top_holders)
                    );
                    old_info.timestamps.holder_last_checked = Some(now);
                }
            }
        }

        // Recalculate security score and flags
        self.calculate_security_metrics(&mut old_info);

        // Update timestamps
        old_info.timestamps.last_updated = now;
        old_info.update_strategy = if static_needs_update || dynamic_needs_update {
            UpdateStrategy::Partial
        } else {
            UpdateStrategy::Cached
        };

        // Save to database and cache
        self.database.save_security_info(&old_info)?;
        self.cache.set(old_info.clone());

        log(
            LogTag::Security,
            "UPDATE_COMPLETE",
            &format!(
                "Updated security info for {} (score: {})",
                safe_truncate(&mint, 8),
                old_info.security_score
            )
        );

        Ok(old_info)
    }

    /// Perform full security analysis for a single token
    async fn perform_full_security_analysis(
        &self,
        mint: &str
    ) -> Result<TokenSecurityInfo, ScreenerBotError> {
        let now = Utc::now();

        log(
            LogTag::Security,
            "FULL_START",
            &format!("Starting full analysis for {}", safe_truncate(mint, 8))
        );

        // Get authority information
        let authority_info = get_token_authorities(mint).await?;

        // Get LP lock information (optional)
        let lp_lock_info = match check_lp_lock_status(mint).await {
            Ok(info) => Some(info),
            Err(e) => {
                log(
                    LogTag::Security,
                    "LP_LOCK_FAIL",
                    &format!("Failed to get LP lock info for {}: {}", safe_truncate(mint, 8), e)
                );
                None
            }
        };

        // Get holder information (optional)
        let holder_info = match self.get_holder_security_info(mint).await {
            Ok(info) => Some(info),
            Err(e) => {
                log(
                    LogTag::Security,
                    "HOLDER_FAIL",
                    &format!("Failed to get holder info for {}: {}", safe_truncate(mint, 8), e)
                );
                None
            }
        };

        let holder_last_checked = holder_info.is_some();
        let lp_lock_last_checked = lp_lock_info.is_some();

        // Create security info
        let mut security_info = TokenSecurityInfo {
            mint: mint.to_string(),
            security_score: 0,
            risk_level: SecurityRiskLevel::Unknown,
            authority_info,
            holder_info,
            lp_lock_info,
            timestamps: SecurityTimestamps {
                first_analyzed: now,
                last_updated: now,
                authority_last_checked: now,
                holder_last_checked: if holder_last_checked {
                    Some(now)
                } else {
                    None
                },
                lp_lock_last_checked: if lp_lock_last_checked {
                    Some(now)
                } else {
                    None
                },
            },
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
            update_strategy: UpdateStrategy::Full,
        };

        // Calculate security metrics
        self.calculate_security_metrics(&mut security_info);

        // Save to database and cache
        self.database.save_security_info(&security_info)?;
        self.cache.set(security_info.clone());

        log(
            LogTag::Security,
            "FULL_COMPLETE",
            &format!(
                "Completed full analysis for {} (score: {})",
                safe_truncate(mint, 8),
                security_info.security_score
            )
        );

        Ok(security_info)
    }

    /// Batch full security analysis with efficient RPC usage
    async fn batch_full_security_analysis(
        &self,
        mints: &[String]
    ) -> Result<HashMap<String, TokenSecurityInfo>, ScreenerBotError> {
        if mints.is_empty() {
            return Ok(HashMap::new());
        }

        let now = Utc::now();
        let mut results = HashMap::new();

        log(
            LogTag::Security,
            "BATCH_START",
            &format!("Starting batch analysis for {} tokens", mints.len())
        );

        // Batch get authorities (most efficient)
        let authorities_result = get_multiple_token_authorities(mints).await;
        let mut authorities_map = HashMap::new();

        if let Ok(authorities) = authorities_result {
            for auth in authorities {
                authorities_map.insert(auth.mint.clone(), auth);
            }
        } else {
            log(
                LogTag::Security,
                "BATCH_AUTH_FAIL",
                "Failed to get batch authorities, falling back to individual calls"
            );
            // Fallback to individual calls
            for mint in mints {
                if let Ok(auth) = get_token_authorities(mint).await {
                    authorities_map.insert(mint.clone(), auth);
                }
            }
        }

        // Batch get LP lock status (if needed)
        let lp_locks_result = check_multiple_lp_locks(mints).await;
        let mut lp_locks_map = HashMap::new();

        if let Ok(lp_locks) = lp_locks_result {
            for lp_lock in lp_locks {
                lp_locks_map.insert(lp_lock.token_mint.clone(), lp_lock);
            }
        }

        // Process each token
        for mint in mints {
            if let Some(authority_info) = authorities_map.get(mint) {
                // Get holder info individually (most expensive operation)
                let holder_info = match self.get_holder_security_info(mint).await {
                    Ok(info) => Some(info),
                    Err(_) => None,
                };

                let lp_lock_info = lp_locks_map.get(mint).cloned();

                let holder_last_checked = holder_info.is_some();
                let lp_lock_last_checked = lp_lock_info.is_some();

                let mut security_info = TokenSecurityInfo {
                    mint: mint.clone(),
                    security_score: 0,
                    risk_level: SecurityRiskLevel::Unknown,
                    authority_info: authority_info.clone(),
                    holder_info,
                    lp_lock_info,
                    timestamps: SecurityTimestamps {
                        first_analyzed: now,
                        last_updated: now,
                        authority_last_checked: now,
                        holder_last_checked: if holder_last_checked {
                            Some(now)
                        } else {
                            None
                        },
                        lp_lock_last_checked: if lp_lock_last_checked {
                            Some(now)
                        } else {
                            None
                        },
                    },
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
                    update_strategy: UpdateStrategy::Full,
                };

                // Calculate security metrics
                self.calculate_security_metrics(&mut security_info);

                // Save to database
                if let Err(e) = self.database.save_security_info(&security_info) {
                    log(
                        LogTag::Security,
                        "SAVE_ERROR",
                        &format!(
                            "Failed to save security info for {}: {}",
                            safe_truncate(mint, 8),
                            e
                        )
                    );
                }

                // Cache it
                self.cache.set(security_info.clone());
                results.insert(mint.clone(), security_info);
            } else {
                log(
                    LogTag::Security,
                    "NO_AUTH",
                    &format!("No authority info available for {}", safe_truncate(mint, 8))
                );
            }
        }

        log(
            LogTag::Security,
            "BATCH_COMPLETE",
            &format!("Completed batch analysis for {}/{} tokens", results.len(), mints.len())
        );

        Ok(results)
    }

    /// Get holder security information with intelligent error handling and pre-checks
    async fn get_holder_security_info(
        &self,
        mint: &str
    ) -> Result<HolderSecurityInfo, ScreenerBotError> {
        // Pre-check if token has too many accounts before attempting analysis
        match should_skip_holder_analysis(mint).await {
            Ok(should_skip) => {
                if should_skip {
                    log(
                        LogTag::Security,
                        "HOLDER_SKIP_PRECHECK",
                        &format!(
                            "Skipping holder analysis for {} - pre-check indicates too many holders",
                            safe_truncate(mint, 8)
                        )
                    );
                    // Return conservative holder info for tokens with many holders
                    return Ok(HolderSecurityInfo {
                        total_holders: u32::MAX, // Indicates very high count
                        top_10_concentration: 0.0, // Unknown
                        top_5_concentration: 0.0, // Unknown
                        largest_holder_percentage: 0.0, // Unknown
                        whale_count: 0, // Unknown
                        average_balance: 0.0, // Unknown
                        distribution_score: 50, // Neutral score for unknown distribution
                    });
                }
            }
            Err(e) => {
                log(
                    LogTag::Security,
                    "HOLDER_PRECHECK_ERROR",
                    &format!("Pre-check failed for {}: {}", safe_truncate(mint, 8), e)
                );
                // Continue with analysis if pre-check fails
            }
        }

        // Get basic holder stats
        let holder_stats = match get_holder_stats(mint).await {
            Ok(stats) => stats,
            Err(e) => {
                // Check if error is due to too many holders
                let error_msg = e.to_string();
                if
                    error_msg.contains("too many holders") ||
                    error_msg.contains("timeout") ||
                    error_msg.contains("deprioritized") ||
                    error_msg.contains("Request too large")
                {
                    // Return minimal holder info indicating high holder count
                    return Ok(HolderSecurityInfo {
                        total_holders: u32::MAX, // Indicates unknown/high count
                        top_10_concentration: 0.0,
                        top_5_concentration: 0.0,
                        largest_holder_percentage: 0.0,
                        whale_count: 0,
                        average_balance: 0.0,
                        distribution_score: 30, // Conservative score for unknown distribution
                    });
                }
                return Err(
                    ScreenerBotError::Data(crate::errors::DataError::Generic {
                        message: format!("Failed to get holder stats: {}", e),
                    })
                );
            }
        };

        // Get top holders for concentration analysis
        let top_holders = match get_top_holders_analysis(mint, Some(10)).await {
            Ok(holders) => holders,
            Err(e) => {
                let error_msg = e.to_string();
                if
                    error_msg.contains("too many holders") ||
                    error_msg.contains("timeout") ||
                    error_msg.contains("deprioritized") ||
                    error_msg.contains("Request too large")
                {
                    // Use the basic stats we already have
                    return Ok(HolderSecurityInfo {
                        total_holders: holder_stats.total_holders,
                        top_10_concentration: holder_stats.top_10_concentration,
                        top_5_concentration: holder_stats.top_10_concentration * 0.8, // Estimate
                        largest_holder_percentage: 0.0, // Unknown
                        whale_count: 0, // Unknown
                        average_balance: holder_stats.average_balance,
                        distribution_score: 40, // Conservative score
                    });
                }
                return Err(
                    ScreenerBotError::Data(crate::errors::DataError::Generic {
                        message: format!("Failed to get top holders: {}", e),
                    })
                );
            }
        };

        Ok(self.create_holder_security_info(&holder_stats, &top_holders))
    }

    /// Create holder security info from stats and top holders
    fn create_holder_security_info(
        &self,
        stats: &HolderStats,
        top_holders: &TopHoldersAnalysis
    ) -> HolderSecurityInfo {
        // Calculate concentrations
        let total_supply: f64 = top_holders.top_holders
            .iter()
            .map(|h| h.ui_amount)
            .sum();

        let top_5_supply: f64 = top_holders.top_holders
            .iter()
            .take(5)
            .map(|h| h.ui_amount)
            .sum();
        let top_5_concentration = if total_supply > 0.0 {
            (top_5_supply / total_supply) * 100.0
        } else {
            0.0
        };

        let largest_holder_percentage = if
            total_supply > 0.0 &&
            !top_holders.top_holders.is_empty()
        {
            (top_holders.top_holders[0].ui_amount / total_supply) * 100.0
        } else {
            0.0
        };

        // Count whales (holders with >5% supply)
        let whale_count = top_holders.top_holders
            .iter()
            .filter(|h| h.ui_amount / total_supply > 0.05)
            .count() as u32;

        // Calculate distribution score
        let distribution_score = self.calculate_distribution_score(
            stats,
            top_5_concentration,
            largest_holder_percentage,
            whale_count
        );

        HolderSecurityInfo {
            total_holders: stats.total_holders,
            top_10_concentration: stats.top_10_concentration,
            top_5_concentration,
            largest_holder_percentage,
            whale_count,
            average_balance: stats.average_balance,
            distribution_score,
        }
    }

    /// Calculate distribution score (0-100)
    fn calculate_distribution_score(
        &self,
        _stats: &HolderStats,
        top_5_concentration: f64,
        largest_holder: f64,
        whale_count: u32
    ) -> u8 {
        let mut score = 100u8;

        // Penalize high concentration
        if top_5_concentration > 80.0 {
            score = score.saturating_sub(50);
        } else if top_5_concentration > 60.0 {
            score = score.saturating_sub(30);
        } else if top_5_concentration > 40.0 {
            score = score.saturating_sub(15);
        }

        // Penalize large single holder
        if largest_holder > 50.0 {
            score = score.saturating_sub(40);
        } else if largest_holder > 30.0 {
            score = score.saturating_sub(25);
        } else if largest_holder > 20.0 {
            score = score.saturating_sub(15);
        }

        // Penalize too many whales
        if whale_count > 5 {
            score = score.saturating_sub(20);
        } else if whale_count > 3 {
            score = score.saturating_sub(10);
        }

        score
    }

    /// Calculate overall security metrics
    fn calculate_security_metrics(&self, info: &mut TokenSecurityInfo) {
        let mut score = 100u8;
        let mut flags = SecurityFlags {
            can_mint: !info.authority_info.is_mint_disabled(),
            can_freeze: !info.authority_info.is_freeze_disabled(),
            has_update_authority: !info.authority_info.is_update_disabled(),
            lp_locked: false,
            high_concentration: false,
            few_holders: false,
            whale_risk: false,
            analysis_incomplete: false,
        };

        // Authority analysis
        if flags.can_mint {
            score = score.saturating_sub(30); // Can mint new tokens
        }
        if flags.can_freeze {
            score = score.saturating_sub(25); // Can freeze accounts
        }
        if flags.has_update_authority {
            score = score.saturating_sub(10); // Can update metadata
        }

        // LP lock analysis
        if let Some(ref lp_info) = info.lp_lock_info {
            flags.lp_locked = lp_info.status.is_safe();
            if !flags.lp_locked {
                score = score.saturating_sub(20); // LP not locked
            }
        } else {
            score = score.saturating_sub(15); // No LP info available
            flags.analysis_incomplete = true;
        }

        // Holder analysis
        if let Some(ref holder_info) = info.holder_info {
            flags.high_concentration = holder_info.top_10_concentration > 50.0;
            flags.few_holders = holder_info.total_holders < 50;
            flags.whale_risk = holder_info.largest_holder_percentage > 20.0;

            if flags.high_concentration {
                score = score.saturating_sub(15);
            }
            if flags.few_holders {
                score = score.saturating_sub(10);
            }
            if flags.whale_risk {
                score = score.saturating_sub(15);
            }

            // Use distribution score
            let distribution_weight = 20u8;
            let distribution_penalty = distribution_weight.saturating_sub(
                (((holder_info.distribution_score as f64) * (distribution_weight as f64)) /
                    100.0) as u8
            );
            score = score.saturating_sub(distribution_penalty);
        } else {
            score = score.saturating_sub(10); // No holder info available
            flags.analysis_incomplete = true;
        }

        info.security_score = score;
        info.risk_level = SecurityRiskLevel::from_score(score);
        info.security_flags = flags;
    }

    /// Background cleanup task
    pub async fn cleanup_task(&self) {
        log(LogTag::Security, "CLEANUP", "Starting security database cleanup");

        // Clean cache
        self.cache.cleanup_expired();

        // Clean old database records (older than 30 days)
        if let Ok(deleted) = self.database.cleanup_old_data(30) {
            if deleted > 0 {
                log(
                    LogTag::Security,
                    "CLEANUP",
                    &format!("Cleaned {} old security records", deleted)
                );
            }
        }

        let (total, static_count, dynamic_count) = self.cache.stats();
        log(
            LogTag::Security,
            "CACHE_STATS",
            &format!("Cache: {} total, {} static, {} dynamic", total, static_count, dynamic_count)
        );
    }
}

/// Global security analyzer instance
static mut GLOBAL_SECURITY_ANALYZER: Option<TokenSecurityAnalyzer> = None;
static SECURITY_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global security analyzer
pub fn init_security_analyzer() -> Result<&'static TokenSecurityAnalyzer, ScreenerBotError> {
    unsafe {
        SECURITY_INIT.call_once(|| {
            let db_path = "data/security.db";
            match TokenSecurityAnalyzer::new(db_path) {
                Ok(analyzer) => {
                    GLOBAL_SECURITY_ANALYZER = Some(analyzer);
                }
                Err(e) => {
                    log(
                        LogTag::Security,
                        "INIT_ERROR",
                        &format!("Failed to initialize security analyzer: {}", e)
                    );
                }
            }
        });

        GLOBAL_SECURITY_ANALYZER.as_ref().ok_or_else(|| {
            ScreenerBotError::Configuration(crate::errors::ConfigurationError::InvalidConfig {
                field: "security_analyzer".to_string(),
                reason: "failed to initialize security analyzer".to_string(),
            })
        })
    }
}

/// Get global security analyzer
pub fn get_security_analyzer() -> &'static TokenSecurityAnalyzer {
    init_security_analyzer().expect("Failed to initialize security analyzer")
}

/// Convenience function for single token analysis
pub async fn analyze_token_security(mint: &str) -> Result<TokenSecurityInfo, ScreenerBotError> {
    get_security_analyzer().analyze_token_security(mint).await
}

/// Convenience function for single token analysis with force refresh
pub async fn analyze_token_security_force_refresh(
    mint: &str
) -> Result<TokenSecurityInfo, ScreenerBotError> {
    get_security_analyzer().analyze_token_security_with_options(mint, true).await
}

/// Convenience function for batch token analysis
pub async fn analyze_multiple_tokens_security(
    mints: &[String]
) -> Result<HashMap<String, TokenSecurityInfo>, ScreenerBotError> {
    get_security_analyzer().analyze_multiple_tokens(mints).await
}

/// Quick security check - returns just the risk level
pub async fn get_token_risk_level(mint: &str) -> Result<SecurityRiskLevel, ScreenerBotError> {
    let security_info = analyze_token_security(mint).await?;
    Ok(security_info.risk_level)
}

/// Quick security summary for logging
pub async fn get_security_summary(mint: &str) -> Result<String, ScreenerBotError> {
    let security_info = analyze_token_security(mint).await?;
    Ok(
        format!(
            "{} {} (Score: {})",
            security_info.risk_level.color_emoji(),
            security_info.risk_level.as_str(),
            security_info.security_score
        )
    )
}

// =============================================================================
// SECURITY MONITORING BACKGROUND TASK
// =============================================================================

/// Security monitoring cycle duration in seconds
const SECURITY_MONITOR_CYCLE_SECONDS: u64 = 3;

/// Security summary report interval in seconds (every 30 seconds)
const SECURITY_SUMMARY_INTERVAL_SECONDS: u64 = 30;

/// Number of tokens to analyze per cycle (not more)
const SECURITY_TOKENS_PER_CYCLE: usize = 10;

/// Security monitoring constants for update intervals
const SECURITY_STATIC_UPDATE_AGE_HOURS: i64 = 168; // Static properties: weekly
const SECURITY_DYNAMIC_UPDATE_AGE_HOURS: i64 = 12; // Dynamic properties: twice daily

use crate::tokens::cache::TokenDatabase;
use rand::seq::SliceRandom;

/// Security monitoring system for background security analysis
pub struct SecurityMonitor {
    cycle_counter: u64,
    last_summary_time: std::time::Instant,
}

impl SecurityMonitor {
    /// Create new security monitor instance
    pub fn new() -> Result<Self, ScreenerBotError> {
        Ok(Self {
            cycle_counter: 0,
            last_summary_time: std::time::Instant::now(),
        })
    }

    /// Get tokens that need security updates (missing or stale)
    async fn get_tokens_needing_security_updates(
        &self
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let database = TokenDatabase::new().map_err(|e|
            format!("Failed to create token database: {}", e)
        )?;

        let all_tokens = database
            .get_all_tokens_with_update_time().await
            .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

        let security_analyzer = get_security_analyzer();
        let mut tokens_missing_security = Vec::new();
        let mut tokens_with_stale_security = Vec::new();
        let now = Utc::now();

        for (mint, _, _, _) in all_tokens {
            match security_analyzer.database.get_security_info(&mint) {
                Ok(None) => tokens_missing_security.push(mint),
                Ok(Some(security_info)) => {
                    // Check if security info is stale
                    let static_age = now.signed_duration_since(
                        security_info.timestamps.authority_last_checked
                    );
                    let dynamic_age = security_info.timestamps.holder_last_checked
                        .map(|t| now.signed_duration_since(t))
                        .unwrap_or_else(||
                            chrono::Duration::hours(SECURITY_DYNAMIC_UPDATE_AGE_HOURS + 1)
                        );

                    if
                        static_age.num_hours() > SECURITY_STATIC_UPDATE_AGE_HOURS ||
                        dynamic_age.num_hours() > SECURITY_DYNAMIC_UPDATE_AGE_HOURS
                    {
                        tokens_with_stale_security.push(mint);
                    }
                }
                Err(_) => tokens_missing_security.push(mint), // Error = treat as missing
            }
        }

        if !tokens_missing_security.is_empty() {
            log(
                LogTag::Security,
                "MISSING",
                &format!(
                    "Found {} tokens missing security information",
                    tokens_missing_security.len()
                )
            );
        }

        if !tokens_with_stale_security.is_empty() {
            log(
                LogTag::Security,
                "STALE",
                &format!(
                    "Found {} tokens with stale security information",
                    tokens_with_stale_security.len()
                )
            );
        }

        Ok((tokens_missing_security, tokens_with_stale_security))
    }

    /// Update security information for a batch of tokens
    async fn update_security_batch(&mut self, mints: &[String]) -> Result<usize, String> {
        if mints.is_empty() {
            return Ok(0);
        }

        log(
            LogTag::Security,
            "UPDATE_BATCH",
            &format!("Updating security info for {} tokens", mints.len())
        );

        let security_analyzer = get_security_analyzer();
        let mut successful_updates = 0;

        // Process tokens in smaller batches to avoid overwhelming RPC
        for batch in mints.chunks(5) {
            let batch_vec: Vec<String> = batch.to_vec();

            match security_analyzer.analyze_multiple_tokens(&batch_vec).await {
                Ok(security_results) => {
                    successful_updates += security_results.len();

                    log(
                        LogTag::Security,
                        "BATCH_SUCCESS",
                        &format!("Successfully analyzed {} tokens in batch", security_results.len())
                    );
                }
                Err(e) => {
                    log(
                        LogTag::Security,
                        "BATCH_ERROR",
                        &format!("Failed to analyze security batch: {}", e)
                    );
                }
            }

            // Small delay between batches to avoid RPC overload
            if batch.len() == 5 {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }

        log(
            LogTag::Security,
            "UPDATE_COMPLETE",
            &format!(
                "Security update completed: {}/{} tokens analyzed",
                successful_updates,
                mints.len()
            )
        );

        Ok(successful_updates)
    }

    /// Run security monitoring cycle
    async fn run_security_monitoring_cycle(&mut self) -> Result<(), String> {
        log(LogTag::Security, "DEBUG", "Starting security monitoring cycle");

        // Get tokens that need security updates (single database call)
        let (missing_security_tokens, stale_security_tokens) =
            self.get_tokens_needing_security_updates().await?;

        // Priority 1: Tokens missing security info completely
        let mut tokens_to_process = missing_security_tokens
            .into_iter()
            .take(SECURITY_TOKENS_PER_CYCLE)
            .collect::<Vec<_>>();

        // Priority 2: Fill remaining capacity with stale security tokens
        if tokens_to_process.len() < SECURITY_TOKENS_PER_CYCLE {
            let remaining_capacity = SECURITY_TOKENS_PER_CYCLE - tokens_to_process.len();

            // Randomize to ensure fair distribution
            let mut stale_tokens = stale_security_tokens;
            stale_tokens.shuffle(&mut rand::thread_rng());

            for token in stale_tokens.into_iter().take(remaining_capacity) {
                if !tokens_to_process.contains(&token) {
                    tokens_to_process.push(token);
                }
            }
        }

        if tokens_to_process.is_empty() {
            log(LogTag::Security, "DEBUG", "No tokens need security updates");
            return Ok(());
        }

        // Update security information for selected tokens
        let updated_count = self.update_security_batch(&tokens_to_process).await?;

        log(
            LogTag::Security,
            "CYCLE_COMPLETE",
            &format!(
                "Security cycle completed: {}/{} tokens updated",
                updated_count,
                tokens_to_process.len()
            )
        );

        Ok(())
    }

    /// Display comprehensive security summary (non-debug log)
    async fn display_security_summary(&self) -> Result<(), String> {
        let security_analyzer = get_security_analyzer();

        // Get security statistics from database
        let database = &security_analyzer.database;
        let conn = Connection::open(&database.db_path).map_err(|e|
            format!("Failed to open security database: {}", e)
        )?;

        // Query overall security statistics
        let stats_query =
            r#"
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN security_score >= 80 THEN 1 ELSE 0 END) as safe_count,
                SUM(CASE WHEN security_score >= 60 AND security_score < 80 THEN 1 ELSE 0 END) as low_risk_count,
                SUM(CASE WHEN security_score >= 40 AND security_score < 60 THEN 1 ELSE 0 END) as medium_risk_count,
                SUM(CASE WHEN security_score < 40 THEN 1 ELSE 0 END) as high_risk_count,
                AVG(security_score) as avg_score
            FROM token_security
        "#;

        let stats = conn
            .query_row(stats_query, [], |row| {
                Ok((
                    row.get::<_, i64>(0)?, // total
                    row.get::<_, i64>(1)?, // safe_count
                    row.get::<_, i64>(2)?, // low_risk_count
                    row.get::<_, i64>(3)?, // medium_risk_count
                    row.get::<_, i64>(4)?, // high_risk_count
                    row.get::<_, Option<f64>>(5)?.unwrap_or(0.0), // avg_score
                ))
            })
            .map_err(|e| format!("Failed to query security stats: {}", e))?;

        let (total, safe_count, low_risk_count, medium_risk_count, high_risk_count, avg_score) =
            stats;

        // Query fresh security data (last 24 hours)
        let fresh_query =
            r#"
            SELECT COUNT(*) FROM token_security 
            WHERE (julianday('now') - julianday(updated_at)) * 24 <= 24
        "#;

        let fresh_count: i64 = conn
            .query_row(fresh_query, [], |row| row.get(0))
            .map_err(|e| format!("Failed to query fresh security data: {}", e))?;

        // Query authority statistics
        let authority_query =
            r#"
            SELECT 
                COUNT(*) as total_with_authority,
                SUM(CASE WHEN authority_info LIKE '%"mint_authority":null%' THEN 1 ELSE 0 END) as mint_disabled,
                SUM(CASE WHEN authority_info LIKE '%"freeze_authority":null%' THEN 1 ELSE 0 END) as freeze_disabled
            FROM token_security
            WHERE authority_info IS NOT NULL
        "#;

        let authority_stats = conn
            .query_row(authority_query, [], |row| {
                Ok((
                    row.get::<_, i64>(0)?, // total_with_authority
                    row.get::<_, i64>(1)?, // mint_disabled
                    row.get::<_, i64>(2)?, // freeze_disabled
                ))
            })
            .map_err(|e| format!("Failed to query authority stats: {}", e))?;

        let (total_with_authority, mint_disabled, freeze_disabled) = authority_stats;

        // Cache statistics
        let (cache_total, cache_static, cache_dynamic) = security_analyzer.cache.stats();

        // Display comprehensive summary (INFO level, not DEBUG)
        log(
            LogTag::Security,
            "SUMMARY",
            &format!(
                "Security Analysis Summary: {} tokens | Avg Score: {:.1} | Safe: {} | Low Risk: {} | Medium Risk: {} | High Risk: {} | Fresh Data: {}/{} | Mint Disabled: {}/{} | Freeze Disabled: {}/{} | Cache: {} entries ({} static, {} dynamic)",
                total,
                avg_score,
                safe_count,
                low_risk_count,
                medium_risk_count,
                high_risk_count,
                fresh_count,
                total,
                mint_disabled,
                total_with_authority,
                freeze_disabled,
                total_with_authority,
                cache_total,
                cache_static,
                cache_dynamic
            )
        );

        Ok(())
    }

    /// Check if it's time to display security summary (every 30 seconds)
    fn should_display_summary(&mut self) -> bool {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_summary_time);

        if elapsed.as_secs() >= SECURITY_SUMMARY_INTERVAL_SECONDS {
            self.last_summary_time = now;
            true
        } else {
            false
        }
    }

    /// Start continuous security monitoring loop in background
    pub async fn start_monitoring_loop(&mut self, shutdown: Arc<tokio::sync::Notify>) {
        log(LogTag::Security, "INIT", "Security monitoring loop started");

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Security, "SHUTDOWN", "Security monitoring loop stopping");
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(SECURITY_MONITOR_CYCLE_SECONDS)) => {
                    self.cycle_counter += 1;

                    log(LogTag::Security, "DEBUG", &format!("Starting security monitoring cycle #{}", self.cycle_counter));

                    // Run security monitoring cycle
                    if let Err(e) = self.run_security_monitoring_cycle().await {
                        log(
                            LogTag::Security,
                            "CYCLE_ERROR",
                            &format!("Security monitoring cycle failed: {}", e)
                        );
                    }

                    // Check if it's time to display security summary
                    if self.should_display_summary() {
                        if let Err(e) = self.display_security_summary().await {
                            log(
                                LogTag::Security,
                                "DEBUG",
                                &format!("Failed to display security summary: {}", e)
                            );
                        }
                    }
                }
            }
        }

        log(LogTag::Security, "STOP", "Security monitoring loop stopped");
    }
}

/// Start security monitoring background task
pub async fn start_security_monitoring(
    shutdown: Arc<tokio::sync::Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    log(LogTag::Security, "START", "Starting security monitoring background task");

    let handle = tokio::spawn(async move {
        let mut monitor = match SecurityMonitor::new() {
            Ok(monitor) => {
                log(LogTag::Security, "INIT", "Security monitor instance created successfully");
                monitor
            }
            Err(e) => {
                log(
                    LogTag::Security,
                    "ERROR",
                    &format!("Failed to initialize security monitor: {}", e)
                );
                return;
            }
        };

        log(LogTag::Security, "READY", "Starting security monitoring loop");
        monitor.start_monitoring_loop(shutdown).await;
        log(LogTag::Security, "EXIT", "Security monitoring task ended");
    });

    Ok(handle)
}

/// Manual security monitoring cycle for testing
pub async fn run_security_monitoring_cycle_once() -> Result<(), String> {
    let mut monitor = SecurityMonitor::new().map_err(|e|
        format!("Failed to create monitor: {}", e)
    )?;
    monitor.run_security_monitoring_cycle().await
}
