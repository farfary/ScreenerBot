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

// Additional canonical token constants used across the codebase
/// Wrapped SOL mint (alias for SOL_MINT)
pub const WRAPPED_SOL_MINT: &str = SOL_MINT;

/// Native SOL representation (system program ID placeholder used in some pools)
pub const NATIVE_SOL_MINT: &str = "11111111111111111111111111111111";

/// Common stablecoin mints that are ignored for SOL-based pricing
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

/// Alias for WSOL (wrapped SOL) mint (some modules import WSOL_MINT)
pub const WSOL_MINT: &str = WRAPPED_SOL_MINT;
