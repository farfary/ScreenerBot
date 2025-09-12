use chrono::{ DateTime, Utc };

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
}

impl PositionTransition {
    pub fn position_id(&self) -> Option<i64> {
        match self {
            | Self::EntryVerified { position_id, .. }
            | Self::ExitVerified { position_id, .. }
            | Self::ExitFailedClearForRetry { position_id }
            | Self::ExitPermanentFailureSynthetic { position_id, .. }
            | Self::RemoveOrphanEntry { position_id } => Some(*position_id),
            Self::UpdatePriceTracking { .. } => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::ExitVerified { .. } |
                Self::ExitPermanentFailureSynthetic { .. } |
                Self::RemoveOrphanEntry { .. }
        )
    }

    pub fn requires_db_update(&self) -> bool {
        !matches!(self, Self::UpdatePriceTracking { .. })
    }
}
