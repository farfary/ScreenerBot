/// Global constants used across ScreenerBot
///
/// This module contains system-wide constants that are not configurable
/// and are used across multiple modules.

// ============================================================================
// SOLANA BLOCKCHAIN CONSTANTS
// ============================================================================

/// SOL token mint address (wrapped SOL / WSOL)
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Number of decimal places for SOL token
pub const SOL_DECIMALS: u8 = 9;

/// Lamports per SOL (10^9)
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
