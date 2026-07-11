//! Position transitions — state machine logic for position lifecycle changes.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum PositionTransition {
    EntryVerified {
        position_id: i64,
        effective_entry_price: f64,
        token_amount_units: u64,
        fee_lamports: u64,
        sol_size: f64,
    },
    ExitVerified {
        position_id: i64,
        effective_exit_price: f64,
        sol_received: f64,
        fee_lamports: u64,
        exit_time: DateTime<Utc>,
    },
    ExitFailedClearForRetry {
        position_id: i64,
    },
    ExitPermanentFailureSynthetic {
        position_id: i64,
        exit_time: DateTime<Utc>,
    },
    RemoveOrphanEntry {
        position_id: i64,
    },
    UpdatePriceTracking {
        mint: String,
        current_price: f64,
        highest: Option<f64>,
        lowest: Option<f64>,
    },
    // ==================== PARTIAL EXIT TRANSITIONS ====================
    PartialExitSubmitted {
        position_id: i64,
        exit_signature: String,
        exit_amount: u64,     // Tokens to sell
        exit_percentage: f64, // % of position
        market_price: f64,    // Price at submission
    },
    PartialExitVerified {
        position_id: i64,
        exit_amount: u64,          // Actual tokens sold
        sol_received: f64,         // Actual SOL received
        effective_exit_price: f64, // Actual price
        fee_lamports: u64,         // Transaction fee
        exit_time: DateTime<Utc>,
        exit_signature: String,
        exit_percentage: f64,
    },
    PartialExitFailed {
        position_id: i64,
        reason: String,
    },
    /// A FULL close swap succeeded but left a real (non-dust) balance behind — the
    /// classic case being a wallet whose tokens are split across several accounts, where
    /// `close_position_direct` deliberately sells only the primary ATA.
    ///
    /// The swap is real: it sold tokens and received SOL. Booking it as a plain failure
    /// (which is what `ExitFailedClearForRetry` did) silently threw those proceeds away.
    /// This records the fill exactly like a partial exit AND clears the exit signature so
    /// the residual can be closed on the next pass.
    ExitResidualClearForRetry {
        position_id: i64,
        exit_amount: u64,
        sol_received: f64,
        effective_exit_price: f64,
        fee_lamports: u64,
        exit_time: DateTime<Utc>,
        exit_signature: String,
        exit_percentage: f64,
    },
    // ==================== DCA TRANSITIONS ====================
    DcaSubmitted {
        position_id: i64,
        dca_signature: String,
        dca_amount_sol: f64, // Additional SOL invested
        market_price: f64,   // Price at DCA
    },
    DcaVerified {
        position_id: i64,
        tokens_bought: u64,   // Additional tokens
        sol_spent: f64,       // Actual SOL spent
        effective_price: f64, // Actual price
        fee_lamports: u64,    // Transaction fee
        dca_time: DateTime<Utc>,
        dca_signature: String,
    },
    DcaFailed {
        position_id: i64,
        dca_signature: String,
        reason: String,
    },
}

impl PositionTransition {
    pub fn position_id(&self) -> Option<i64> {
        match self {
            Self::EntryVerified { position_id, .. }
            | Self::ExitVerified { position_id, .. }
            | Self::ExitFailedClearForRetry { position_id }
            | Self::ExitPermanentFailureSynthetic { position_id, .. }
            | Self::RemoveOrphanEntry { position_id }
            | Self::PartialExitSubmitted { position_id, .. }
            | Self::PartialExitVerified { position_id, .. }
            | Self::PartialExitFailed { position_id, .. }
            | Self::ExitResidualClearForRetry { position_id, .. }
            | Self::DcaSubmitted { position_id, .. }
            | Self::DcaVerified { position_id, .. }
            | Self::DcaFailed { position_id, .. } => Some(*position_id),
            Self::UpdatePriceTracking { .. } => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::ExitVerified { .. }
                | Self::ExitPermanentFailureSynthetic { .. }
                | Self::RemoveOrphanEntry { .. }
        )
    }

    pub fn requires_db_update(&self) -> bool {
        !matches!(self, Self::UpdatePriceTracking { .. })
    }
}
