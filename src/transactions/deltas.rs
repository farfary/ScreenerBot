//! Subject-relative balance deltas — the chain-neutral shape.
//!
//! `SubjectAssetDelta` is the ledger the positions module's wallet-history reducer
//! (`positions::ledger`) is built on: a wallet-derived `External` position is only ever as
//! good as these rows. Extraction from a decoded transaction is chain-specific and lives
//! next to the chain's decode pipeline — see
//! `crate::chains::solana::transactions::deltas::extract_subject_deltas` for Solana.

/// The literal mint used for native SOL rows (there is no real mint for it).
pub const NATIVE_SOL_SENTINEL: &str = "native";

/// Bumping this re-runs `TransactionDatabase::backfill_subject_deltas` for every
/// wallet on next boot (its once-only guard compares against this value).
/// v2 re-extracts every stored transaction so rows written before the `block_time`
/// fallback existed regain their timestamps.
pub const SUBJECT_DELTAS_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaKind {
    Trade,
    Transfer,
    DeFi,
    Other,
}

impl DeltaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trade => "trade",
            Self::Transfer => "transfer",
            Self::DeFi => "defi",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "trade" => Ok(Self::Trade),
            "transfer" => Ok(Self::Transfer),
            "defi" => Ok(Self::DeFi),
            "other" => Ok(Self::Other),
            other => Err(format!("unknown delta kind: {other}")),
        }
    }
}

/// One subject-relative asset movement within one transaction. See module docs.
#[derive(Debug, Clone, PartialEq)]
pub struct SubjectAssetDelta {
    pub wallet_address: String,
    pub signature: String,
    /// SPL mint, or [`NATIVE_SOL_SENTINEL`] for native SOL.
    pub mint: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub tx_index: u32,
    /// Signed, raw base units.
    pub delta_raw: i128,
    /// `None` when not knowable.
    pub before_raw: Option<u128>,
    pub after_raw: Option<u128>,
    pub decimals: u8,
    pub kind: DeltaKind,
    pub venue: Option<String>,
    pub fee_lamports: Option<u64>,
    pub success: bool,
}
