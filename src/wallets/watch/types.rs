//! Types for the wallet observation service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Why a wallet is under observation. One address can serve more than one source at
/// once (`sources` on `WatchTarget` is a `Vec`).
///
/// `Serialize`/`Deserialize`: persisted as the `watch_targets.sources` JSON column,
/// and surfaced verbatim in the target-list/status API responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatchSource {
    /// The bot's own trading wallet. Always watched while `wallet.watch_enabled` is
    /// on; not a row in `watch_targets` and not addressable through the watch API --
    /// it is structural, not something the user adds or removes.
    OwnWallet,
    /// A copy task consumes this subject's activity. The task id lets matching avoid
    /// an event-time database lookup and binds execution to the intended task.
    Copy { task_id: i64 },
    /// Notify-only: format matching activity through Telegram (and, later, the
    /// dashboard). No money moves. `rule_id` is the owning `watch_targets.id` --
    /// there is no separate alert-rule table in this phase, so a target IS the rule.
    Alert { rule_id: i64 },
}

/// A wallet under observation, as stored in `watch_targets` (`wallets.db`).
#[derive(Debug, Clone, Serialize)]
pub struct WatchTarget {
    /// Row id in `watch_targets`. `None` for the synthesized own-wallet target, which
    /// is never persisted.
    pub id: Option<i64>,
    /// Base58 Solana address.
    pub address: String,
    /// User-facing label, if any.
    pub label: Option<String>,
    /// Every reason this address is being watched.
    pub sources: Vec<WatchSource>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Side of a detected swap, subject-relative (did the subject's holding of `mint`
/// grow or shrink).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapSide {
    Buy,
    Sell,
}

/// Direction of a plain (non-swap) transfer, subject-relative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    In,
    Out,
}

/// What a decoded, successful transaction did, from the subject's perspective.
#[derive(Debug, Clone)]
pub enum ActivityKind {
    /// A DEX/aggregator swap with exactly one resolved primary mint, SOL-quoted (see
    /// `classify::classify_transaction_activity`).
    Swap {
        mint: String,
        side: SwapSide,
        sol_amount: f64,
        token_amount: f64,
        venue: Option<String>,
        price_sol: Option<f64>,
    },
    /// A plain SPL/SOL transfer -- no DEX program involved.
    Transfer {
        mint: String,
        amount: f64,
        direction: TransferDirection,
    },
    /// Decoded successfully but not a swap or a simple transfer (liquidity ops,
    /// program interactions, an ambiguous/skipped multi-hop or non-SOL-quoted route).
    Other,
}

/// One piece of on-chain activity for a watched subject, published on the shared
/// broadcast channel. `detected_at` / `decoded_at` are the day-one latency
/// instrumentation the plan calls for (§9): the gap between them is the decode
/// budget, independent of how long the notification took to arrive.
#[derive(Debug, Clone)]
pub struct WalletActivity {
    /// Base58 address this activity is about.
    pub subject: String,
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    /// When the triggering notification (WS push, poll page, or gap-fill) reached us.
    pub detected_at: DateTime<Utc>,
    /// When `decode()` finished (fetch + analyze complete).
    pub decoded_at: DateTime<Utc>,
    pub success: bool,
    pub kind: ActivityKind,
    /// Consumers filter locally by why this subject is watched; carrying the source
    /// set avoids an event-time database lookup and keeps alert/copy dispatch out of
    /// the detection hot path.
    pub sources: Vec<WatchSource>,
}

/// Per-target status, surfaced by `/api/wallets/watch/:id/status`.
#[derive(Debug, Clone, Serialize)]
pub struct WatchStatus {
    pub target: WatchTarget,
    /// True when the shared subscription transport is `Connected` and this address is
    /// registered with it.
    pub subscribed: bool,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub last_signature: Option<String>,
    pub last_error: Option<String>,
}
