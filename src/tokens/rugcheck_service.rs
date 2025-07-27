/// Smart Rugcheck Service for High-Priority Token Updates
///
/// This service provides:
/// - 5-minute updates for high-liquidity tokens
/// - 30-minute updates for medium-priority tokens
/// - Daily updates for all database tokens
/// - Risk assessment for filtering integration

use crate::logger::{ log, LogTag };
use crate::tokens::cache::TokenDatabase;
use crate::tokens::rugcheck::{ RugcheckService, RugcheckResponse };
use crate::tokens::price_service::get_priority_tokens_safe;
use std::sync::Arc;
use tokio::time::{ sleep, Duration, Instant };
use std::collections::HashSet;

// ===== CONFIGURATION CONSTANTS =====

/// High-priority update interval (5 minutes)
const HIGH_PRIORITY_UPDATE_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Medium-priority update interval (30 minutes)
const MEDIUM_PRIORITY_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Low-priority update interval (24 hours)
const LOW_PRIORITY_UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Maximum tokens per high-priority update cycle
const MAX_HIGH_PRIORITY_TOKENS: usize = 50;

/// Maximum tokens per medium-priority update cycle
const MAX_MEDIUM_PRIORITY_TOKENS: usize = 200;

/// Rate limiting delay between API requests (1 second)
const API_RATE_LIMIT_DELAY: Duration = Duration::from_secs(1);

// ===== RUGCHECK RISK ASSESSMENT =====

/// Rugcheck risk levels for filtering
#[derive(Debug, Clone, PartialEq)]
pub enum RugcheckRiskLevel {
    Safe, // No significant risks
    Warning, // Minor risks, proceed with caution
    Dangerous, // Major risks, avoid trading
    Critical, // Extreme risks, blacklist immediately
}

/// Rugcheck risk assessment result
#[derive(Debug, Clone)]
pub struct RugcheckRiskAssessment {
    pub risk_level: RugcheckRiskLevel,
    pub freeze_authority_safe: bool,
    pub lp_unlocked_risk: bool,
    pub score: Option<i32>,
    pub risk_reasons: Vec<String>,
}

/// Check if freeze authority is safe (null or false)
pub fn is_freeze_authority_safe(rugcheck_data: &RugcheckResponse) -> bool {
    // Check token freeze authority
    if let Some(token) = &rugcheck_data.token {
        if let Some(freeze_auth) = &token.freeze_authority {
            // If freeze authority exists and is not null/empty, it's not safe
            return freeze_auth.is_empty() || freeze_auth == "null";
        }
    }

    // Check main freeze authority field
    if let Some(freeze_auth) = &rugcheck_data.freeze_authority {
        // If it's not null and has content, analyze it
        if !freeze_auth.is_null() && freeze_auth.as_str().map_or(false, |s| !s.is_empty()) {
            return false;
        }
    }

    // Default to safe if no freeze authority found
    true
}

/// Check if token has large unlocked LP risk
pub fn has_lp_unlocked_risk(rugcheck_data: &RugcheckResponse) -> bool {
    // Check for LP unlocked risk in risks array
    if let Some(risks) = &rugcheck_data.risks {
        for risk in risks {
            if
                risk.name.to_lowercase().contains("lp") &&
                risk.name.to_lowercase().contains("unlock")
            {
                // If risk level is high or critical, it's a problem
                if let Some(level) = &risk.level {
                    return level.to_lowercase() == "high" || level.to_lowercase() == "critical";
                }
                // If no level specified but risk exists, assume it's a problem
                return true;
            }
        }
    }

    // Check market data for LP information
    if let Some(markets) = &rugcheck_data.markets {
        for market in markets {
            if let Some(lp) = &market.lp {
                // Check LP locked percentage - if less than 50% locked, it's risky
                if let Some(lp_locked_pct) = lp.lp_locked_pct {
                    if lp_locked_pct < 50.0 {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Get overall rugcheck risk level
pub fn get_rugcheck_risk_level(rugcheck_data: &RugcheckResponse) -> RugcheckRiskAssessment {
    let mut risk_reasons = Vec::new();
    let freeze_authority_safe = is_freeze_authority_safe(rugcheck_data);
    let lp_unlocked_risk = has_lp_unlocked_risk(rugcheck_data);

    // Check freeze authority
    if !freeze_authority_safe {
        risk_reasons.push("Freeze authority enabled - tokens can be frozen".to_string());
    }

    // Check LP unlocked risk
    if lp_unlocked_risk {
        risk_reasons.push("Large amount of LP tokens unlocked".to_string());
    }

    // Check if token is marked as rugged
    if rugcheck_data.rugged.unwrap_or(false) {
        risk_reasons.push("Token marked as rugged".to_string());
        return RugcheckRiskAssessment {
            risk_level: RugcheckRiskLevel::Critical,
            freeze_authority_safe,
            lp_unlocked_risk,
            score: rugcheck_data.score,
            risk_reasons,
        };
    }

    // Check score (lower is worse)
    let risk_level = match rugcheck_data.score {
        Some(score) if score < 20 => {
            risk_reasons.push(format!("Very low rugcheck score: {}", score));
            RugcheckRiskLevel::Critical
        }
        Some(score) if score < 50 => {
            risk_reasons.push(format!("Low rugcheck score: {}", score));
            RugcheckRiskLevel::Dangerous
        }
        Some(score) if score < 70 => {
            risk_reasons.push(format!("Medium rugcheck score: {}", score));
            RugcheckRiskLevel::Warning
        }
        _ => {
            // Check individual risk factors
            if !freeze_authority_safe || lp_unlocked_risk {
                RugcheckRiskLevel::Dangerous
            } else {
                RugcheckRiskLevel::Safe
            }
        }
    };

    RugcheckRiskAssessment {
        risk_level,
        freeze_authority_safe,
        lp_unlocked_risk,
        score: rugcheck_data.score,
        risk_reasons,
    }
}

// ===== SMART RUGCHECK SERVICE =====

pub struct SmartRugcheckService {
    rugcheck_service: RugcheckService,
    database: TokenDatabase,
    last_high_priority_update: Option<Instant>,
    last_medium_priority_update: Option<Instant>,
    last_low_priority_update: Option<Instant>,
}

impl SmartRugcheckService {
    /// Create new smart rugcheck service
    pub fn new() -> Result<Self, String> {
        let database = TokenDatabase::new().map_err(|e|
            format!("Failed to initialize database: {}", e)
        )?;

        let rugcheck_service = RugcheckService::new(database.clone());

        Ok(Self {
            rugcheck_service,
            database,
            last_high_priority_update: None,
            last_medium_priority_update: None,
            last_low_priority_update: None,
        })
    }

    /// Start smart rugcheck update service
    pub async fn start_update_service(&mut self, shutdown: Arc<tokio::sync::Notify>) {
        log(LogTag::Rugcheck, "START", "Smart rugcheck update service started");

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Rugcheck, "STOP", "Smart rugcheck service stopping");
                    break;
                }
                
                _ = sleep(Duration::from_secs(30)) => {
                    if let Err(e) = self.run_update_cycle().await {
                        log(LogTag::Rugcheck, "ERROR", 
                            &format!("Rugcheck update cycle failed: {}", e));
                    }
                }
            }
        }

        log(LogTag::Rugcheck, "STOP", "Smart rugcheck service stopped");
    }

    /// Run intelligent update cycle
    async fn run_update_cycle(&mut self) -> Result<(), String> {
        let now = Instant::now();

        // High-priority updates (every 5 minutes)
        if self.should_run_high_priority_update(now) {
            log(LogTag::Rugcheck, "UPDATE", "Running high-priority rugcheck updates");
            self.update_high_priority_tokens().await?;
            self.last_high_priority_update = Some(now);
        }

        // Medium-priority updates (every 30 minutes)
        if self.should_run_medium_priority_update(now) {
            log(LogTag::Rugcheck, "UPDATE", "Running medium-priority rugcheck updates");
            self.update_medium_priority_tokens().await?;
            self.last_medium_priority_update = Some(now);
        }

        // Low-priority updates (every 24 hours)
        if self.should_run_low_priority_update(now) {
            log(LogTag::Rugcheck, "UPDATE", "Running low-priority rugcheck updates");
            self.update_low_priority_tokens().await?;
            self.last_low_priority_update = Some(now);
        }

        Ok(())
    }

    /// Check if high-priority update should run
    fn should_run_high_priority_update(&self, now: Instant) -> bool {
        match self.last_high_priority_update {
            Some(last) => now.duration_since(last) >= HIGH_PRIORITY_UPDATE_INTERVAL,
            None => true,
        }
    }

    /// Check if medium-priority update should run
    fn should_run_medium_priority_update(&self, now: Instant) -> bool {
        match self.last_medium_priority_update {
            Some(last) => now.duration_since(last) >= MEDIUM_PRIORITY_UPDATE_INTERVAL,
            None => true,
        }
    }

    /// Check if low-priority update should run
    fn should_run_low_priority_update(&self, now: Instant) -> bool {
        match self.last_low_priority_update {
            Some(last) => now.duration_since(last) >= LOW_PRIORITY_UPDATE_INTERVAL,
            None => true,
        }
    }

    /// Update high-priority tokens (top liquidity + open positions)
    async fn update_high_priority_tokens(&self) -> Result<(), String> {
        // Get high-priority tokens from existing price service
        let priority_mints = get_priority_tokens_safe().await;

        if priority_mints.is_empty() {
            log(LogTag::Rugcheck, "INFO", "No high-priority tokens to update");
            return Ok(());
        }

        // Limit to max tokens per cycle
        let mints_to_update: Vec<String> = priority_mints
            .into_iter()
            .take(MAX_HIGH_PRIORITY_TOKENS)
            .collect();

        log(
            LogTag::Rugcheck,
            "UPDATE",
            &format!("Updating rugcheck data for {} high-priority tokens", mints_to_update.len())
        );

        self.rugcheck_service.update_rugcheck_data(mints_to_update).await?;

        Ok(())
    }

    /// Update medium-priority tokens (discovered but not top priority)
    async fn update_medium_priority_tokens(&self) -> Result<(), String> {
        // Get high-priority tokens to exclude
        let high_priority_mints: HashSet<String> = get_priority_tokens_safe().await
            .into_iter()
            .collect();

        // Get tokens with medium liquidity (but not high priority)
        let medium_tokens = self.database
            .get_tokens_by_liquidity_threshold(10000.0).await
            .map_err(|e| format!("Failed to get medium-priority tokens: {}", e))?;

        let medium_mints: Vec<String> = medium_tokens
            .into_iter()
            .filter(|token| !high_priority_mints.contains(&token.mint))
            .take(MAX_MEDIUM_PRIORITY_TOKENS)
            .map(|token| token.mint)
            .collect();

        if medium_mints.is_empty() {
            log(LogTag::Rugcheck, "INFO", "No medium-priority tokens to update");
            return Ok(());
        }

        log(
            LogTag::Rugcheck,
            "UPDATE",
            &format!("Updating rugcheck data for {} medium-priority tokens", medium_mints.len())
        );

        self.rugcheck_service.update_rugcheck_data(medium_mints).await?;

        Ok(())
    }

    /// Update low-priority tokens (all database tokens)
    async fn update_low_priority_tokens(&self) -> Result<(), String> {
        // Get all tokens from database
        let all_tokens = self.database
            .get_all_tokens().await
            .map_err(|e| format!("Failed to get all tokens: {}", e))?;

        let all_mints: Vec<String> = all_tokens
            .into_iter()
            .map(|token| token.mint)
            .collect();

        if all_mints.is_empty() {
            log(LogTag::Rugcheck, "INFO", "No tokens in database to update");
            return Ok(());
        }

        log(
            LogTag::Rugcheck,
            "UPDATE",
            &format!("Updating rugcheck data for {} database tokens", all_mints.len())
        );

        self.rugcheck_service.update_rugcheck_data(all_mints).await?;

        Ok(())
    }

    /// Get rugcheck risk assessment for a token
    pub async fn get_token_risk_assessment(
        &self,
        mint: &str
    ) -> Result<Option<RugcheckRiskAssessment>, String> {
        if let Some(rugcheck_data) = self.rugcheck_service.get_rugcheck_data(mint).await? {
            Ok(Some(get_rugcheck_risk_level(&rugcheck_data)))
        } else {
            Ok(None)
        }
    }
}

// ===== PUBLIC API FUNCTIONS =====

/// Get rugcheck risk assessment for filtering
pub async fn get_token_rugcheck_risk_assessment(
    mint: &str
) -> Result<Option<RugcheckRiskAssessment>, String> {
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to initialize database: {}", e)
    )?;

    let rugcheck_service = RugcheckService::new(database);

    if let Some(rugcheck_data) = rugcheck_service.get_rugcheck_data(mint).await? {
        Ok(Some(get_rugcheck_risk_level(&rugcheck_data)))
    } else {
        Ok(None)
    }
}

/// Check if token should be filtered out due to rugcheck risks
pub async fn should_filter_token_rugcheck(mint: &str) -> bool {
    match get_token_rugcheck_risk_assessment(mint).await {
        Ok(Some(assessment)) => {
            // Filter out dangerous and critical risk tokens
            matches!(
                assessment.risk_level,
                RugcheckRiskLevel::Dangerous | RugcheckRiskLevel::Critical
            )
        }
        Ok(None) => {
            // No rugcheck data available - allow trading but log warning
            log(
                LogTag::Rugcheck,
                "WARN",
                &format!("No rugcheck data for token: {} - allowing trade", mint)
            );
            false
        }
        Err(e) => {
            // Error getting rugcheck data - allow trading but log error
            log(
                LogTag::Rugcheck,
                "ERROR",
                &format!("Failed to get rugcheck data for {}: {} - allowing trade", mint, e)
            );
            false
        }
    }
}
