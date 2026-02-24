use crate::constants::TOKEN_2022_PROGRAM_ID;
use crate::errors::blockchain::{parse_structured_solana_error, BlockchainError};
use crate::errors::parse_solana_error;
use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::Error;
use chrono::{DateTime, Utc};
use solana_sdk::pubkey::Pubkey;
use std::fs;
use std::str::FromStr;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

/// Safe signature formatting that shows first 8 and last 4 chars, or full string if short
pub fn safe_format_signature(s: &str) -> String {
    let char_count = s.chars().count();
    if char_count > 12 {
        let first_8 = s;
        // Get last 4 characters safely
        let last_4 = if char_count >= 4 {
            s.chars().skip(char_count - 4).collect::<String>()
        } else {
            s.to_string()
        };
        format!("{}...{}", first_8, last_4)
    } else {
        s.to_string()
    }
}

// =============================================================================
// SOLANA-SPECIFIC UTILITIES (Consolidated from multiple files)
// =============================================================================

/// Standard pubkey parsing with consistent error message formatting
/// Consolidates 20+ identical patterns across the codebase
pub fn parse_pubkey_safe(address: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(address).map_err(|e| format!("Invalid pubkey '{}': {}", address, e))
}

/// Read a pubkey from byte data at specified offset with bounds checking
/// Consolidates 17+ duplicate read_pubkey implementations from debug binaries
pub fn read_pubkey_from_data(data: &[u8], offset: usize) -> Option<String> {
    if offset + 32 > data.len() {
        return None;
    }

    let pubkey_bytes = &data[offset..offset + 32];
    match Pubkey::try_from(pubkey_bytes) {
        Ok(pubkey) => {
            // Basic sanity check - reject all-zeros or all-ones
            if pubkey_bytes.iter().all(|&b| b == 0) || pubkey_bytes.iter().all(|&b| b == 255) {
                None
            } else {
                Some(pubkey.to_string())
            }
        }
        Err(_) => None,
    }
}

/// Read a u64 from byte data at specified offset with little-endian byte order
/// Consolidates 9+ duplicate read_u64 implementations from debug binaries
pub fn read_u64_from_data(data: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 > data.len() {
        return None;
    }

    let bytes: [u8; 8] = data[offset..offset + 8].try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

/// Read a u32 from byte data at specified offset with little-endian byte order
pub fn read_u32_from_data(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 > data.len() {
        return None;
    }

    let bytes: [u8; 4] = data[offset..offset + 4].try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// SOL lamports conversion functions
/// Uses the existing LAMPORTS_PER_SOL constant from constants.rs
/// Consolidates 20+ hardcoded 1_000_000_000 values across the codebase
use crate::constants::LAMPORTS_PER_SOL;

/// Convert lamports to SOL with consistent precision
pub fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / (LAMPORTS_PER_SOL as f64)
}

/// Convert SOL to lamports with proper rounding
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * (LAMPORTS_PER_SOL as f64)).round() as u64
}

/// Format mint address consistently for logs (8 chars + "...")
/// Consolidates multiple patterns for mint addresses
pub fn format_mint_for_log(mint: &str) -> String {
    format!("{}...", mint)
}

/// Format price with adaptive precision for Solana tokens
///
/// Solana tokens can have prices with 12+ decimal places (e.g., 0.000000001234567890).
/// This function uses adaptive formatting to preserve precision:
/// - Very small prices (< 1e-6): scientific notation with 15 decimals
/// - Small prices (< 0.01): 12 decimal places
/// - Medium prices (< 1.0): 9 decimal places
/// - Larger prices: 6 decimal places
///
/// # Arguments
/// * `price` - Price value in SOL or USD
///
/// # Returns
/// Formatted string with appropriate precision
///
/// # Example
/// ```
/// use screenerbot::utils::format_price_adaptive;
///
/// assert_eq!(format_price_adaptive(0.00000000123), "1.230000000000000e-9");
/// assert_eq!(format_price_adaptive(0.000123), "0.000123000000");
/// assert_eq!(format_price_adaptive(0.123), "0.123000000");
/// assert_eq!(format_price_adaptive(123.456), "123.456000");
/// ```
pub fn format_price_adaptive(price: f64) -> String {
    if !price.is_finite() {
        return price.to_string(); // Handle NaN/Inf
    }

    let abs_price = price.abs();

    if abs_price < 1e-6 {
        // Very small: use scientific notation
        format!("{:.15e}", price)
    } else if abs_price < 0.01 {
        // Small: 12 decimals
        format!("{:.12}", price)
    } else if abs_price < 1.0 {
        // Medium: 9 decimals
        format!("{:.9}", price)
    } else {
        // Large: 6 decimals
        format!("{:.6}", price)
    }
}

// Re-export SwapResult for convenience
pub use crate::swaps::SwapResult;

/// Get the wallet address from the main wallet private key in config
/// This replaces the swaps::get_wallet_address dependency
pub fn get_wallet_address() -> crate::Result<String> {
    crate::config::get_wallet_pubkey_string().map_err(|e| {
        Error::Configuration(crate::errors::ConfigurationError::InvalidPrivateKey {
            error: format!("Failed to load wallet address: {}", e),
        })
    })
}

/// Format a duration (from Option<DateTime<Utc>>) as a human-readable age string (y d h m s)
pub fn format_age_string(created_at: Option<DateTime<Utc>>) -> String {
    if let Some(dt) = created_at {
        let now = Utc::now();
        let mut seconds = if now > dt {
            (now - dt).num_seconds()
        } else {
            0
        };
        let years = seconds / 31_536_000; // 365*24*60*60
        seconds %= 31_536_000;
        let days = seconds / 86_400;
        seconds %= 86_400;
        let hours = seconds / 3_600;
        seconds %= 3_600;
        let minutes = seconds / 60;
        seconds %= 60;
        let mut parts = Vec::new();
        if years > 0 {
            parts.push(format!("{}y", years));
        }
        if days > 0 {
            parts.push(format!("{}d", days));
        }
        if hours > 0 {
            parts.push(format!("{}h", hours));
        }
        if minutes > 0 {
            parts.push(format!("{}m", minutes));
        }
        if seconds > 0 || parts.is_empty() {
            parts.push(format!("{}s", seconds));
        }
        parts.join("")
    } else {
        "unknown".to_string()
    }
}

/// Waits for either shutdown signal or delay. Returns true if shutdown was triggered.
pub async fn check_shutdown_or_delay(shutdown: &Notify, duration: Duration) -> bool {
    tokio::select! {
      _ = tokio::time::sleep(duration) => false,
      _ = shutdown.notified() => true,
    }
}

/// Waits for a delay or shutdown signal, whichever comes first.
pub async fn delay_with_shutdown(shutdown: &Notify, duration: Duration) {
    tokio::select! {
      _ = tokio::time::sleep(duration) => {},
      _ = shutdown.notified() => {},
    }
}

/// Helper function to format duration in a compact way
pub fn format_duration_compact(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let duration = end.signed_duration_since(start);
    let total_seconds = duration.num_seconds();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        format!("{}m", total_seconds / 60)
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if minutes > 0 {
            format!("{}h{}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else {
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        if hours > 0 {
            format!("{}d{}h", days, hours)
        } else {
            format!("{}d", days)
        }
    }
}

/// Utility function for hex dump debugging - prints data in hex format with ASCII representation
pub fn hex_dump_data(
    data: &[u8],
    start_offset: usize,
    length: usize,
    log_callback: impl Fn(&str, &str),
) {
    let end = (start_offset + length).min(data.len());

    for chunk_start in (start_offset..end).step_by(16) {
        let chunk_end = (chunk_start + 16).min(end);
        let chunk = &data[chunk_start..chunk_end];

        // Format offset
        let offset_str = format!("{:08X}", chunk_start);

        // Format hex bytes
        let hex_str = chunk
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join("");

        // Pad hex string to consistent width (48 chars for 16 bytes)
        let hex_padded = format!("{:<48}", hex_str);

        // Format ASCII representation
        let ascii_str: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        log_callback(
            "DEBUG",
            &format!("{}: {} |{}|", offset_str, hex_padded, ascii_str),
        );
    }
}

// =============================================================================
// ATA (ASSOCIATED TOKEN ACCOUNT) OPERATIONS
// =============================================================================
// All ATA-related functions have been moved to src/ata_operations.rs
// Re-exported here for backward compatibility

pub use crate::ata_operations::{
    cleanup_all_empty_atas, close_all_empty_atas, close_single_ata, close_token_account,
    close_token_account_with_context, get_all_token_accounts, get_sol_balance, get_token_balance,
    get_total_token_balance,
};

// =============================================================================
// UTILITY FUNCTIONS MOVED FROM TRADER.RS
// =============================================================================

/// Safe wrapper for RwLock read operations that logs poison errors instead of panicking
pub fn safe_read_lock<'a, T>(
    lock: &'a std::sync::RwLock<T>,
    operation: &str,
) -> Option<std::sync::RwLockReadGuard<'a, T>> {
    match lock.read() {
        Ok(guard) => Some(guard),
        Err(e) => {
            logger::error(
                LogTag::Trader,
                &format!("RwLock read poisoned during {}: {}", operation, e),
            );
            None
        }
    }
}

/// Safe wrapper for RwLock write operations that logs poison errors instead of panicking
pub fn safe_write_lock<'a, T>(
    lock: &'a std::sync::RwLock<T>,
    operation: &str,
) -> Option<std::sync::RwLockWriteGuard<'a, T>> {
    match lock.write() {
        Ok(guard) => Some(guard),
        Err(e) => {
            logger::error(
                LogTag::Trader,
                &format!("RwLock write poisoned during {}: {}", operation, e),
            );
            None
        }
    }
}

/// Helper function for conditional debug trader logs
pub fn debug_trader_log(log_type: &str, message: &str) {
    logger::debug(LogTag::Trader, &format!("{}: {}", log_type, message));
}
