//! The chain-adapter seam: chain-specific behaviour behind one trait.
//!
//! Neutral modules that need a chain fact (native-unit conversion, address
//! validation, explorer URLs, market-data network slugs) call [`adapter()`]
//! instead of naming a `ChainId` variant or a Solana constant directly. With
//! one supported chain this always resolves to the Solana adapter; a second
//! chain adds a match arm here, not a call-site branch.

use crate::chains::{active_chain, ChainError, ChainId, ChainMetadata};

/// Chain-specific behaviour behind a single object-safe trait.
pub trait ChainAdapter: Send + Sync + 'static {
    /// Returns the stable identifier of this chain.
    fn id(&self) -> ChainId;
    /// Returns read-only metadata for this chain.
    fn metadata(&self) -> ChainMetadata;

    // --- native units ---
    /// Base units in one whole native asset (lamports per SOL, wei per ETH).
    fn raw_units_per_native(&self) -> u64;
    /// Converts a raw base-unit amount to whole native-asset units.
    fn raw_to_native(&self, raw: u64) -> f64 {
        raw as f64 / self.raw_units_per_native() as f64
    }
    /// Converts a whole native-asset amount to raw base units, rounding to the
    /// nearest unit.
    fn native_to_raw(&self, native: f64) -> u64 {
        (native * self.raw_units_per_native() as f64).round() as u64
    }

    // --- identity shapes ---
    /// Strict validation: the address is well-formed for this chain.
    fn validate_address(&self, address: &str) -> Result<(), ChainError>;
    /// Cheap shape heuristic for "does this input look like an address?"
    /// (search boxes, LLM tool-argument screening). Never a substitute for
    /// `validate_address` on a money path.
    fn looks_like_address(&self, candidate: &str) -> bool;
    /// Shape check for a transaction hash/signature of this chain.
    fn looks_like_transaction_hash(&self, candidate: &str) -> bool;

    // --- asset policy ---
    /// Returns the canonical address of this chain's native asset.
    fn native_asset_address(&self) -> &'static str;
    /// Returns the decimal precision of this chain's native asset.
    fn native_asset_decimals(&self) -> u8;
    /// True for any address denoting the native asset (Solana: the wrapped
    /// mint or the system program).
    fn is_native_asset(&self, address: &str) -> bool;
    /// Rewrites any native-asset spelling to the canonical one.
    fn normalize_native_asset(&self, address: &str) -> String;
    /// Canonical stablecoin quote assets on this chain.
    fn stable_assets(&self) -> &'static [&'static str];
    /// True if `address` is one of this chain's canonical stablecoin quote
    /// assets.
    fn is_stable_asset(&self, address: &str) -> bool {
        self.stable_assets().contains(&address)
    }

    // --- external identity ---
    /// This chain's slug at market-data providers (GeckoTerminal network,
    /// DexScreener chainId, CoinGecko platform key, DefiLlama chain name).
    fn market_data_network(&self) -> &'static str;
    /// Returns the block explorer's token page URL for `address`.
    fn explorer_token_url(&self, address: &str) -> String;
    /// Returns the block explorer's account page URL for `address`.
    fn explorer_account_url(&self, address: &str) -> String;
    /// Returns the block explorer's transaction page URL for `hash`.
    fn explorer_transaction_url(&self, hash: &str) -> String;
    /// Chart page for a token at the DEX aggregator front-end.
    fn dex_chart_url(&self, address: &str) -> String;
    /// Token page at the market-analytics front-end.
    fn analytics_token_url(&self, address: &str) -> String;
}

/// The adapter for the chain this process operates on.
pub fn adapter() -> &'static dyn ChainAdapter {
    match active_chain() {
        ChainId::Solana => crate::chains::solana::adapter::ADAPTER,
    }
}
