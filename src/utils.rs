//! Shared utility functions — formatting, parsing, and conversions.

use crate::Error;
use std::time::Duration;
use tokio::sync::Notify;

// =============================================================================
// NATIVE-UNIT CONVERSIONS
// =============================================================================
//
// Pubkey parsing lives in `crate::chains::solana::accounts` (concrete chain
// mechanics). Lamports/SOL conversion stays here: its signature carries no
// chain-specific type, and SOL is, per the app mission, the single monetary
// unit in trading logic today.

use crate::chains::solana::constants::LAMPORTS_PER_SOL;

/// Convert lamports to SOL with consistent precision
pub fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / (LAMPORTS_PER_SOL as f64)
}

/// Convert SOL to lamports with proper rounding
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * (LAMPORTS_PER_SOL as f64)).round() as u64
}

/// Format a mint address for log output (first 8 + last 4 chars)
pub fn format_mint_for_log(mint: &str) -> String {
    let len = mint.len();
    if len > 12 {
        format!("{}...{}", &mint[..8], &mint[len - 4..])
    } else {
        mint.to_string()
    }
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
            error: format!("Failed to load wallet address: {e}"),
        })
    })
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

/// Runs `work` to completion, but abandons it the instant `shutdown` fires.
///
/// Returns `Some(output)` if the work finished, `None` if shutdown interrupted it
/// (caller should then break/return). While the work future is awaited the task is
/// parked on `shutdown.notified()`, so an in-flight long operation (network/DB
/// batches, long computes) is interrupted at its next await point instead of running
/// to completion and blocking a graceful exit. Always wrap real work in a background
/// loop with this.
///
/// Note: `notify_waiters()` is edge-triggered, so a broadcast that lands in the gap
/// between two helper calls (before this `select!` re-registers its waiter) would be
/// missed permanently on its own. `ServiceManager::stop_all` closes that race by
/// re-broadcasting the shutdown signal on a short interval for the whole duration of
/// shutdown, so a parked waiter here is always re-woken within one interval.
pub async fn run_or_shutdown<F>(shutdown: &Notify, work: F) -> Option<F::Output>
where
    F: std::future::Future,
{
    tokio::select! {
      _ = shutdown.notified() => None,
      out = work => Some(out),
    }
}
