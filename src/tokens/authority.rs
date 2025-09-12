/// Token Authority Management
///
/// This module provides functionality to check token authorities (minting, freeze, and metadata update)
/// for SPL tokens and Token-2022 tokens using the RPC client system.

use crate::errors::ScreenerBotError;
use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use futures;
use serde::{ Deserialize, Serialize };

/// Token authorities information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenAuthorities {
    /// Mint authority - can mint new tokens (None means minting is disabled)
    pub mint_authority: Option<String>,
    /// Freeze authority - can freeze/unfreeze token accounts (None means freeze is disabled)
    pub freeze_authority: Option<String>,
    /// Update authority - can update token metadata (None means updates are disabled)
    pub update_authority: Option<String>,
    /// Whether the token is a Token-2022 token
    pub is_token_2022: bool,
    /// Token mint address
    pub mint: String,
}

impl TokenAuthorities {
    /// Check if minting is permanently disabled (mint authority is None)
    pub fn is_mint_disabled(&self) -> bool {
        self.mint_authority.is_none()
    }

    /// Check if freeze is permanently disabled (freeze authority is None)
    pub fn is_freeze_disabled(&self) -> bool {
        self.freeze_authority.is_none()
    }

    /// Check if metadata updates are permanently disabled (update authority is None)
    pub fn is_update_disabled(&self) -> bool {
        self.update_authority.is_none()
    }

    /// Check if the token has any active authorities
    pub fn has_any_authority(&self) -> bool {
        self.mint_authority.is_some() ||
            self.freeze_authority.is_some() ||
            self.update_authority.is_some()
    }

    /// Check if all authorities are permanently disabled (safest state)
    pub fn is_fully_renounced(&self) -> bool {
        self.mint_authority.is_none() &&
            self.freeze_authority.is_none() &&
            self.update_authority.is_none()
    }

    /// Get a summary of the authority status
    pub fn get_authority_summary(&self) -> String {
        let mint_status = if self.mint_authority.is_some() { "ENABLED" } else { "DISABLED" };
        let freeze_status = if self.freeze_authority.is_some() { "ENABLED" } else { "DISABLED" };
        let update_status = if self.update_authority.is_some() { "ENABLED" } else { "DISABLED" };

        format!(
            "Mint: {} | Freeze: {} | Update: {} | Token-2022: {}",
            mint_status,
            freeze_status,
            update_status,
            self.is_token_2022
        )
    }

    /// Check if this is a "rug-safe" token (all dangerous authorities disabled)
    pub fn is_rug_safe(&self) -> bool {
        // For rug safety, we primarily care about mint and freeze being disabled
        // Update authority can be kept for legitimate metadata updates
        self.mint_authority.is_none() && self.freeze_authority.is_none()
    }

    /// Get risk level based on authorities
    pub fn get_risk_level(&self) -> TokenRiskLevel {
        if self.is_fully_renounced() {
            TokenRiskLevel::Safe
        } else if self.is_rug_safe() {
            TokenRiskLevel::Low
        } else if self.mint_authority.is_some() && self.freeze_authority.is_some() {
            TokenRiskLevel::High
        } else {
            TokenRiskLevel::Medium
        }
    }
}

/// Token risk levels based on authorities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenRiskLevel {
    /// All authorities disabled - safest
    Safe,
    /// Only update authority enabled - low risk
    Low,
    /// Some authorities enabled - medium risk
    Medium,
    /// Mint and freeze authorities enabled - high risk
    High,
}

impl TokenRiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenRiskLevel::Safe => "SAFE",
            TokenRiskLevel::Low => "LOW",
            TokenRiskLevel::Medium => "MEDIUM",
            TokenRiskLevel::High => "HIGH",
        }
    }

    pub fn get_color_code(&self) -> &'static str {
        match self {
            TokenRiskLevel::Safe => "🟢",
            TokenRiskLevel::Low => "🟡",
            TokenRiskLevel::Medium => "🟠",
            TokenRiskLevel::High => "🔴",
        }
    }
}

/// Get token authorities information for a given mint address
/// This function handles both SPL tokens and Token-2022 tokens
pub async fn get_token_authorities(mint: &str) -> Result<TokenAuthorities, ScreenerBotError> {
    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "DEBUG",
            &format!("Checking authorities for mint: {}", crate::utils::safe_truncate(mint, 12))
        );
    }

    let rpc_client = get_rpc_client();

    // Get the mint account data
    let account_data = rpc_client.get_mint_account(mint).await?;

    // Check if this is a Token-2022 mint first
    let is_token_2022 = rpc_client.is_token_2022_mint(mint).await.unwrap_or(false);

    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "DEBUG",
            &format!(
                "Token type detected: {} for mint: {}",
                if is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                },
                crate::utils::safe_truncate(mint, 12)
            )
        );
    }

    // Parse authorities from account data
    let authorities = parse_mint_authorities(&account_data, mint, is_token_2022)?;

    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "DEBUG",
            &format!(
                "Authority check complete for {}: {}",
                crate::utils::safe_truncate(mint, 12),
                authorities.get_authority_summary()
            )
        );
    }

    Ok(authorities)
}

/// Parse mint authorities from RPC account data
fn parse_mint_authorities(
    account_data: &serde_json::Value,
    mint: &str,
    is_token_2022: bool
) -> Result<TokenAuthorities, ScreenerBotError> {
    // Check if account exists
    if account_data.is_null() {
        return Err(
            ScreenerBotError::Data(crate::errors::DataError::Generic {
                message: format!("Mint account not found: {}", mint),
            })
        );
    }

    // Get the parsed data
    let parsed_data = account_data
        .get("value")
        .and_then(|v| v.get("data"))
        .and_then(|d| d.get("parsed"))
        .and_then(|p| p.get("info"))
        .ok_or_else(|| {
            ScreenerBotError::Data(crate::errors::DataError::ParseError {
                data_type: "mint account data".to_string(),
                error: "Missing or invalid parsed data structure".to_string(),
            })
        })?;

    // Extract authorities
    let mint_authority = parsed_data
        .get("mintAuthority")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let freeze_authority = parsed_data
        .get("freezeAuthority")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // For Token-2022, we might need to check extensions for additional authorities
    // For now, we'll handle the basic authorities that are standard
    let update_authority = if is_token_2022 {
        // Token-2022 might have metadata extensions with update authority
        // For now, we'll set this to None and can extend later if needed
        parsed_data
            .get("extensions")
            .and_then(|ext| ext.as_array())
            .and_then(|arr| {
                arr.iter().find_map(|item| {
                    if let Some(metadata) = item.get("metadata") {
                        metadata.get("updateAuthority").and_then(|v| v.as_str())
                    } else {
                        None
                    }
                })
            })
            .map(|s| s.to_string())
    } else {
        // SPL tokens don't have metadata update authority at the mint level
        None
    };

    Ok(TokenAuthorities {
        mint_authority,
        freeze_authority,
        update_authority,
        is_token_2022,
        mint: mint.to_string(),
    })
}

/// Batch check authorities for multiple tokens
/// This is more efficient than checking them one by one
pub async fn get_multiple_token_authorities(
    mints: &[String]
) -> Result<Vec<TokenAuthorities>, ScreenerBotError> {
    if mints.is_empty() {
        return Ok(Vec::new());
    }

    if crate::arguments::is_debug_security_enabled() {
        log(LogTag::Security, "DEBUG", &format!("Checking authorities for {} tokens", mints.len()));
    }

    let mut results = Vec::with_capacity(mints.len());
    let mut successful_checks = 0;
    let mut failed_checks = 0;

    // Process tokens in small batches to avoid overwhelming the RPC
    const BATCH_SIZE: usize = 10;
    for chunk in mints.chunks(BATCH_SIZE) {
        let mut batch_futures = Vec::new();

        for mint in chunk {
            batch_futures.push(get_token_authorities(mint));
        }

        // Execute batch concurrently
        let batch_results = futures::future::join_all(batch_futures).await;

        for (i, result) in batch_results.into_iter().enumerate() {
            match result {
                Ok(authorities) => {
                    results.push(authorities);
                    successful_checks += 1;
                }
                Err(e) => {
                    failed_checks += 1;
                    log(
                        LogTag::Rpc,
                        "AUTH_ERROR",
                        &format!(
                            "Failed to check authorities for {}: {}",
                            crate::utils::safe_truncate(&chunk[i], 12),
                            e
                        )
                    );
                }
            }
        }

        // Small delay between batches to be respectful to RPC
        if mints.len() > BATCH_SIZE {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    log(
        LogTag::Rpc,
        "AUTH_BATCH_COMPLETE",
        &format!(
            "Authority batch check complete: {}/{} successful",
            successful_checks,
            successful_checks + failed_checks
        )
    );

    Ok(results)
}

/// Check if a token is considered "safe" based on its authorities
/// This is a convenience function for quick safety checks
pub async fn is_token_safe(mint: &str) -> Result<bool, ScreenerBotError> {
    let authorities = get_token_authorities(mint).await?;
    Ok(authorities.is_rug_safe())
}

/// Get a quick authority summary string for a token
/// This is useful for logging and display purposes
pub async fn get_authority_summary(mint: &str) -> Result<String, ScreenerBotError> {
    let authorities = get_token_authorities(mint).await?;
    Ok(
        format!(
            "{} {} - {}",
            authorities.get_risk_level().get_color_code(),
            authorities.get_risk_level().as_str(),
            authorities.get_authority_summary()
        )
    )
}
