//! Preflight gates shared by every manual trade entry point (dashboard routes and
//! agent tools): emergency stop, core-service readiness, address shape,
//! blacklist, and the per-trade slippage bound. A refused trade is also recorded
//! as a failed action so it shows in the dashboard like any other attempt.

use crate::trader::constants::MAX_MANUAL_SLIPPAGE_PCT;
use crate::trader::Error;

/// Which manual operation is being gated; selects the failed-action recorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualTradeKind {
    Buy,
    Add,
    Sell,
}

/// Blacklist handling for the gate: only a buy may be explicitly forced past it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlacklistPolicy {
    Enforce,
    Override,
    /// Sells must always be able to leave a position, blacklisted or not.
    Ignore,
}

async fn record_failure(kind: ManualTradeKind, mint: &str, error: &Error) {
    let message = error.to_string();
    match kind {
        ManualTradeKind::Buy => {
            crate::trader::actions::create_failed_buy_action(mint, &message).await
        }
        ManualTradeKind::Add => {
            crate::trader::actions::create_failed_add_action(mint, &message).await
        }
        ManualTradeKind::Sell => {
            crate::trader::actions::create_failed_sell_action(mint, &message).await
        }
    }
}

async fn is_blacklisted(mint: &str) -> bool {
    let Some(db) = crate::tokens::database::get_global_database() else {
        return false;
    };
    let mint = mint.to_owned();
    tokio::task::spawn_blocking(move || db.is_blacklisted(&mint))
        .await
        .ok()
        .and_then(|result| result.ok())
        .unwrap_or(false)
}

/// Run every gate a manual trade must pass before it may be submitted.
pub async fn preflight(
    kind: ManualTradeKind,
    mint: &str,
    blacklist: BlacklistPolicy,
) -> Result<(), Error> {
    if crate::global::is_force_stopped() {
        return Err(Error::ForceStopped);
    }
    let error = if !crate::global::are_core_services_ready() {
        Some(Error::CoreServicesNotReady {
            pending: crate::global::get_pending_services().join(", "),
        })
    } else if crate::chains::adapter().validate_address(mint).is_err() {
        Some(Error::InvalidMint {
            mint: mint.to_owned(),
        })
    } else if blacklist == BlacklistPolicy::Enforce && is_blacklisted(mint).await {
        Some(Error::Blacklisted {
            mint: mint.to_owned(),
        })
    } else if kind != ManualTradeKind::Buy && !crate::positions::is_open_position(mint).await {
        Some(Error::NoOpenPosition {
            mint: mint.to_owned(),
        })
    } else {
        None
    };
    match error {
        Some(error) => {
            record_failure(kind, mint, &error).await;
            Err(error)
        }
        None => Ok(()),
    }
}

/// Validate a per-trade slippage override. `None` follows the configured
/// slippage; a value must be finite and within (0, MAX_MANUAL_SLIPPAGE_PCT].
pub fn validate_slippage(slippage_pct: Option<f64>) -> Result<Option<f64>, Error> {
    match slippage_pct {
        Some(pct) if !pct.is_finite() || pct <= 0.0 || pct > MAX_MANUAL_SLIPPAGE_PCT => {
            Err(Error::InvalidSlippage {
                slippage_pct: pct,
                maximum_pct: MAX_MANUAL_SLIPPAGE_PCT,
            })
        }
        other => Ok(other),
    }
}

/// Validate a partial-sell percentage: finite and within (0, 100].
pub fn validate_percentage(percentage: f64) -> Result<f64, Error> {
    if !percentage.is_finite() || percentage <= 0.0 || percentage > 100.0 {
        return Err(Error::InvalidPercentage { percentage });
    }
    Ok(percentage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slippage_override_is_bounded() {
        assert_eq!(validate_slippage(None).unwrap(), None);
        assert_eq!(validate_slippage(Some(1.5)).unwrap(), Some(1.5));
        assert!(validate_slippage(Some(0.0)).is_err());
        assert!(validate_slippage(Some(f64::NAN)).is_err());
        assert!(validate_slippage(Some(MAX_MANUAL_SLIPPAGE_PCT + 1.0)).is_err());
    }

    #[test]
    fn sell_percentage_is_bounded() {
        assert!(validate_percentage(100.0).is_ok());
        assert!(validate_percentage(0.5).is_ok());
        assert!(validate_percentage(0.0).is_err());
        assert!(validate_percentage(101.0).is_err());
    }
}
