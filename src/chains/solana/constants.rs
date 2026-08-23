//! Solana program IDs, protocol addresses and native-unit constants.
//!
//! Single source of truth for everything Solana-address-shaped: on-chain
//! program identifiers used by decoders, swap builders and the RPC/
//! transaction layers, and the SOL/WSOL/USDC/USDT mint addresses and
//! lamport/decimals conversion constants used across the app. The Solana
//! adapter builds [`super::NATIVE_ASSET`] from `SOL_MINT`/`SOL_DECIMALS`;
//! chain-neutral `NativeAsset`/`ChainMetadata` types stay in `crate::chains`.

/// SOL token mint address (wrapped SOL = SOL on Solana). Per the app mission,
/// SOL is the single monetary unit in trading logic today.
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const SOL_DECIMALS: u8 = 9;
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Convert lamports to SOL with consistent precision
pub fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / (LAMPORTS_PER_SOL as f64)
}

/// Convert SOL to lamports with proper rounding
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * (LAMPORTS_PER_SOL as f64)).round() as u64
}

pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

/// Standard ATA creation/closure cost in SOL
pub const ATA_RENT_COST_SOL: f64 = 0.00203928;

/// Tolerance for ATA rent variations (lamports)
pub const ATA_RENT_TOLERANCE_LAMPORTS: i64 = 10000;

/// Default compute unit price (micro-lamports)
pub const DEFAULT_COMPUTE_UNIT_PRICE: u64 = 1000;

/// Metaplex Token Metadata Program ID (standard, already deployed)
pub const METAPLEX_PROGRAM_ID: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

/// Solana System Program ID (NOT a token mint)
pub const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";

// ============================================================================
// SPL TOKEN PROGRAM IDS
// ============================================================================

pub const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

// ============================================================================
// SOLANA SYSTEM PROGRAM IDS
// ============================================================================

pub const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";

// ============================================================================
// AGGREGATOR/ROUTER PROGRAM IDS
// ============================================================================

pub const JUPITER_V6_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
pub const JUPITER_V4_PROGRAM_ID: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";

// ============================================================================
// DEX PROGRAM IDS
// ============================================================================

pub const RAYDIUM_LEGACY_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const METEORA_DAMM_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
pub const METEORA_DBC_PROGRAM_ID: &str = "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN";
pub const PUMP_FUN_AMM_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMP_FUN_LEGACY_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const MOONIT_AMM_PROGRAM_ID: &str = "MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG";
pub const FLUXBEAM_AMM_PROGRAM_ID: &str = "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X";
