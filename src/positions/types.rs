//! Position data types — core Position struct, transitions, and lifecycle events.

use super::transitions::PositionTransition;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ==================== SHARED ENUMS ====================

/// Who initiated a trade.
///
/// The auto-trader's config (`trader.dca_*`, `positions.partial_exit_enabled`) is
/// POLICY: it decides what the bot is allowed to do ON ITS OWN. It is not a
/// capability switch for the user. A `Manual` trade is the user overriding the bot
/// from the dashboard, so those policy gates do not apply to it — turning auto-DCA
/// off must not disable the dashboard's "Add to Position" button, and turning auto
/// partial exits off must not turn a manual 25% take-profit into a full close.
///
/// Correctness and safety limits — position exists, amounts finite and in range, max
/// trade size, blacklist, force-stop, core services ready — apply to BOTH origins and
/// are enforced elsewhere (routes + `trader::manual`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeOrigin {
    /// The trading engine acting on its own (strategies, evaluators, safety).
    Auto,
    /// The user, via the dashboard, Telegram, Assistant, or paired agent tools.
    Manual,
}

/// Durable provenance for a position. This records why the position exists and
/// is independent from who is currently allowed to manage it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PositionOrigin {
    Auto {
        strategy_id: Option<String>,
    },
    Manual,
    Copy {
        task_id: i64,
        source_wallet: String,
    },
    /// Derived from wallet history, not executed by the bot -- see `positions::ledger`.
    External,
}

impl PositionOrigin {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Auto { .. } => "auto",
            Self::Manual => "manual",
            Self::Copy { .. } => "copy",
            Self::External => "external",
        }
    }

    pub fn reference(&self) -> Option<String> {
        match self {
            Self::Auto { strategy_id } => strategy_id.clone(),
            Self::Manual => None,
            Self::Copy {
                task_id,
                source_wallet,
            } => Some(format!("{task_id}:{source_wallet}")),
            Self::External => None,
        }
    }

    pub fn from_columns(kind: &str, reference: Option<String>) -> super::error::Result<Self> {
        use super::error::Error;
        match kind {
            "auto" => Ok(Self::Auto {
                strategy_id: reference,
            }),
            "manual" => Ok(Self::Manual),
            "external" => Ok(Self::External),
            "copy" => {
                let value = reference.ok_or_else(|| Error::UnknownPersistedValue {
                    field: "origin_ref",
                    value: "<missing>".to_owned(),
                })?;
                let (task_id, source_wallet) =
                    value
                        .split_once(':')
                        .ok_or_else(|| Error::UnknownPersistedValue {
                            field: "origin_ref",
                            value: value.clone(),
                        })?;
                Ok(Self::Copy {
                    task_id: task_id.parse().map_err(|_| Error::UnknownPersistedValue {
                        field: "origin_ref",
                        value: value.clone(),
                    })?,
                    source_wallet: source_wallet.to_owned(),
                })
            }
            other => Err(Error::UnknownPersistedValue {
                field: "origin_kind",
                value: other.to_owned(),
            }),
        }
    }
}

/// Ownership policy for automated actions on a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionManagement {
    AutoTrader,
    UserOnly,
    CopyTask,
    Hybrid,
}

impl PositionManagement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AutoTrader => "auto_trader",
            Self::UserOnly => "user_only",
            Self::CopyTask => "copy_task",
            Self::Hybrid => "hybrid",
        }
    }

    pub fn parse(value: &str) -> super::error::Result<Self> {
        match value {
            "auto_trader" => Ok(Self::AutoTrader),
            "user_only" => Ok(Self::UserOnly),
            "copy_task" => Ok(Self::CopyTask),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(super::error::Error::UnknownPersistedValue {
                field: "management",
                value: other.to_owned(),
            }),
        }
    }

    pub fn allows_safety_exits(self) -> bool {
        !matches!(self, Self::UserOnly)
    }

    pub fn allows_policy_exits(self) -> bool {
        matches!(self, Self::AutoTrader | Self::Hybrid)
    }

    pub fn allows_auto_dca(self) -> bool {
        matches!(self, Self::AutoTrader)
    }

    pub fn allows_copy_sell(self) -> bool {
        matches!(self, Self::CopyTask | Self::Hybrid)
    }

    /// Copy-owned modes require durable copy provenance so a real task can find
    /// and manage the position. Auto-trader and user ownership are valid for
    /// every origin except `External`, which -- being derived from wallet
    /// history the bot never traded -- accepts only `UserOnly`.
    pub fn is_valid_for_origin(self, origin: &PositionOrigin) -> bool {
        if matches!(origin, PositionOrigin::External) {
            return matches!(self, Self::UserOnly);
        }
        !matches!(self, Self::CopyTask | Self::Hybrid)
            || matches!(origin, PositionOrigin::Copy { .. })
    }
}

/// Price source for logging and tracking
#[derive(Debug, Clone, Copy)]
pub enum PriceSource {
    Pool,
    Api,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationKind {
    Entry,
    Exit,
}

#[derive(Debug, Clone, Serialize)]
pub enum GiveUpReason {
    MaxAttemptsReached { attempts: u8, max: u8 },
    MaxAgeReached { age_hours: i64, max: i64 },
}

#[derive(Debug)]
pub enum VerificationOutcome {
    Transition(PositionTransition),
    RetryTransient(String),
    PermanentFailure(PositionTransition),
}

// ==================== PENDING SWAP TYPES ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPartialExit {
    pub signature: String,
    pub mint: String,
    pub position_id: i64,
    pub expected_exit_amount: u64,
    pub requested_exit_percentage: f64,
    pub expiry_height: Option<u64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDcaSwap {
    pub signature: String,
    pub mint: String,
    pub position_id: i64,
    pub expiry_height: Option<u64>,
    pub created_at: DateTime<Utc>,
    /// SOL committed by this add. The position does not book it until `DcaVerified`
    /// lands, so this is the only place the amount exists while the swap confirms —
    /// it is what the dashboard shows as "still confirming". Defaulted so an entry
    /// persisted before this field existed still rehydrates after a restart.
    #[serde(default)]
    pub size_sol: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntrySubmission {
    pub transaction_signature: String,
    pub confirmation_pending: bool,
    pub entry_price_sol: f64,
}

// ==================== POSITION STRUCTURES ====================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Position {
    pub id: Option<i64>, // Database ID - None for new positions
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub entry_price: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_price: Option<f64>,
    pub exit_time: Option<DateTime<Utc>>,
    pub position_type: String, // "buy" or "sell"
    pub entry_size_sol: f64,   // Initial SOL spent on first entry
    pub total_size_sol: f64,   // Cumulative SOL invested (includes DCA)
    pub price_highest: f64,
    pub price_lowest: f64,
    // Transaction signatures
    pub entry_transaction_signature: Option<String>,
    pub exit_transaction_signature: Option<String>,
    pub token_amount: Option<u64>, // Initial amount of tokens bought (first entry)
    pub effective_entry_price: Option<f64>, // Initial entry price (deprecated, use average_entry_price)
    pub effective_exit_price: Option<f64>,  // Final exit price (deprecated, use average_exit_price)
    pub sol_received: Option<f64>,          // Total SOL received after all exits
    // Profit targets
    pub profit_target_min: Option<f64>, // Minimum profit target percentage
    pub profit_target_max: Option<f64>, // Maximum profit target percentage
    pub liquidity_tier: Option<String>, // Liquidity tier for reference
    // Verification status
    pub transaction_entry_verified: bool, // Whether entry transaction is fully verified
    pub transaction_exit_verified: bool,  // Whether exit transaction is fully verified
    // Fee tracking
    pub entry_fee_lamports: Option<u64>, // Actual entry transaction fee
    pub exit_fee_lamports: Option<u64>,  // Actual exit transaction fee
    // Price tracking
    pub current_price: Option<f64>, // Current market price (updated by monitoring system)
    pub current_price_updated: Option<DateTime<Utc>>, // When current_price was last updated
    // Phantom position handling
    pub phantom_remove: bool,
    pub phantom_confirmations: u32, // How many times we confirmed zero wallet balance while still open
    pub phantom_first_seen: Option<DateTime<Utc>>, // When first confirmed phantom
    pub synthetic_exit: bool,       // True if we synthetically closed due to missing exit tx
    pub closed_reason: Option<String>, // Optional reason for closure (e.g., "synthetic_phantom_closure")

    // ==================== CALCULATED P&L VALUES ====================
    // Pre-calculated P&L values - updated automatically by positions system
    // For open positions: updated every time current_price changes
    // For closed positions: calculated once at close time and stored permanently
    pub pnl: Option<f64>,         // Realized P&L in SOL (closed positions only)
    pub pnl_percent: Option<f64>, // Realized P&L percentage (closed positions only)
    pub unrealized_pnl: Option<f64>, // Unrealized P&L in SOL (open positions only)
    pub unrealized_pnl_percent: Option<f64>, // Unrealized P&L percentage (open positions only)

    // ==================== PARTIAL EXIT & DCA SUPPORT ====================
    // Partial exit tracking
    pub remaining_token_amount: Option<u64>, // Current holdings after partial exits
    pub total_exited_amount: u64,            // Cumulative tokens sold
    pub average_exit_price: Option<f64>,     // Weighted average exit price
    pub partial_exit_count: u32,             // Number of partial exits executed

    // DCA tracking
    pub dca_count: u32,                       // Number of additional entries (DCA)
    pub average_entry_price: f64,             // Weighted average entry price (all entries)
    pub last_dca_time: Option<DateTime<Utc>>, // Last DCA timestamp for cooldown

    // ==================== ARCHIVAL ====================
    // User-driven "Remove -> Archive" from the dashboard. Reversible: archived
    // positions are hidden from the open/closed lists and surfaced in the Archived
    // tab. Not persisted by price/state updates — only by the dedicated archive ops.
    #[serde(default)]
    pub archived: bool, // True when the user archived this position
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>, // When it was archived

    // ==================== PROVENANCE & OWNERSHIP ====================
    pub origin: PositionOrigin,
    pub management: PositionManagement,

    // ==================== WALLET-HISTORY LEDGER ====================
    /// Identity of a wallet-derived round: the signature of the acquisition that took
    /// the balance from zero, or `genesis:<mint>` for a round already open before our
    /// history begins. `None` for positions the bot executed itself.
    #[serde(default)]
    pub round_key: Option<String>,
    /// False when the cost basis cannot be established from what we have observed —
    /// an airdrop, a USD-quoted leg, a mixed-quote round, or history that does not
    /// reconcile. A position without a complete basis must never be shown a P&L.
    #[serde(default = "default_true")]
    pub basis_complete: bool,
    /// False when the observed deltas do not reconcile with the on-chain balance.
    #[serde(default = "default_true")]
    pub history_complete: bool,
    /// `None` normally; `Some("frozen")` when the wallet's token account is frozen and
    /// the holding therefore cannot be sold.
    #[serde(default)]
    pub holding_state: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Position {
    /// True when this row was DERIVED from wallet history rather than executed by us.
    ///
    /// Such a position holds no global position slot (it never consumed a permit), must
    /// not count against `max_open_positions`, and must never enter the bot's own
    /// trading statistics or the loss limiter — it is the user's pre-existing holding,
    /// not a trade the bot made.
    pub fn is_wallet_derived(&self) -> bool {
        matches!(self.origin, PositionOrigin::External)
    }

    /// True when the position may be shown a realized/unrealized P&L.
    ///
    /// A round whose cost basis could not be established (airdrop, USD-quoted fill,
    /// token -> token swap with no SOL leg, unreconciled history) has no honest P&L, and
    /// inventing one would be a plausible, permanently wrong number. This is NOT limited
    /// to wallet-derived rows: a bot-executed position the user bought more of in
    /// another wallet app has an outside leg the ledger may not be able to price, and
    /// the reconciliation clears `basis_complete` on the trader's own row to say so.
    pub fn has_trustworthy_pnl(&self) -> bool {
        self.basis_complete && self.history_complete
    }

    /// True when the wallet's token account is frozen, so the holding cannot be sold.
    pub fn is_frozen(&self) -> bool {
        self.holding_state.as_deref() == Some(HOLDING_STATE_FROZEN)
    }
}

/// `holding_state` value for a token account the mint authority has frozen.
pub const HOLDING_STATE_FROZEN: &str = "frozen";

// ==================== EXIT & ENTRY HISTORY ====================

/// Record of a single exit (partial or full)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExitRecord {
    pub id: Option<i64>,  // Database ID
    pub position_id: i64, // Parent position ID
    pub timestamp: DateTime<Utc>,
    pub amount: u64,       // Tokens sold
    pub price: f64,        // Exit price per token
    pub sol_received: f64, // SOL received
    pub transaction_signature: String,
    pub is_partial: bool,           // true if partial, false if full exit
    pub percentage: f64,            // % of position sold at this exit
    pub fees_lamports: Option<u64>, // Transaction fee
}

/// Record of a single entry (initial or DCA)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntryRecord {
    pub id: Option<i64>,  // Database ID
    pub position_id: i64, // Parent position ID
    pub timestamp: DateTime<Utc>,
    pub amount: u64,    // Tokens bought
    pub price: f64,     // Entry price per token
    pub sol_spent: f64, // SOL spent
    pub transaction_signature: String,
    pub is_dca: bool,               // true if DCA, false if initial entry
    pub fees_lamports: Option<u64>, // Transaction fee
}

#[cfg(test)]
mod tests {
    use super::{PositionManagement, PositionOrigin};

    #[test]
    fn copy_owned_management_requires_copy_provenance() {
        let manual = PositionOrigin::Manual;
        let copy = PositionOrigin::Copy {
            task_id: 7,
            source_wallet: "wallet".to_owned(),
        };

        assert!(PositionManagement::AutoTrader.is_valid_for_origin(&manual));
        assert!(PositionManagement::UserOnly.is_valid_for_origin(&manual));
        assert!(!PositionManagement::CopyTask.is_valid_for_origin(&manual));
        assert!(!PositionManagement::Hybrid.is_valid_for_origin(&manual));
        assert!(PositionManagement::CopyTask.is_valid_for_origin(&copy));
        assert!(PositionManagement::Hybrid.is_valid_for_origin(&copy));
    }

    #[test]
    fn an_external_origin_only_accepts_user_only_management() {
        let external = PositionOrigin::External;

        assert!(PositionManagement::UserOnly.is_valid_for_origin(&external));
        assert!(!PositionManagement::AutoTrader.is_valid_for_origin(&external));
        assert!(!PositionManagement::CopyTask.is_valid_for_origin(&external));
        assert!(!PositionManagement::Hybrid.is_valid_for_origin(&external));
    }
}
