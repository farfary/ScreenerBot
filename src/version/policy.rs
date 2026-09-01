//! The rules that decide when an update may act on its own.
//!
//! Downloading in the background is cheap and always safe. *Applying* an update
//! is not: it restarts the trading engine. So a staged core is only activated by
//! itself when the owner allows automatic installation and nothing is riding on
//! the current process. Otherwise the update simply waits — a staged core costs
//! nothing to hold and activates at the next launch regardless.

use super::types::*;
use crate::config::with_config;

/// The update-relevant settings, read once so no lock is held across an await.
#[derive(Debug, Clone, Copy)]
pub(super) struct UpdatePolicy {
    pub auto_check: bool,
    pub check_interval_secs: u64,
    pub auto_download: bool,
    pub auto_install: bool,
    pub defer_while_trading: bool,
    pub notify_telegram: bool,
}

impl UpdatePolicy {
    pub(super) fn load() -> Self {
        let (
            auto_check,
            check_interval_hours,
            auto_download,
            auto_install,
            defer_while_trading,
            notify_telegram,
        ) = with_config(|config| {
            (
                config.updates.auto_check,
                config.updates.check_interval_hours,
                config.updates.auto_download,
                config.updates.auto_install,
                config.updates.defer_while_trading,
                config.updates.notify_telegram,
            )
        });

        Self {
            auto_check,
            // Clamp defensively: a hand-edited 0 would spin the check loop.
            check_interval_secs: check_interval_hours.clamp(1, 168) * 3600,
            auto_download,
            auto_install,
            defer_while_trading,
            notify_telegram,
        }
    }
}

/// Whether a verified update may be applied right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyReadiness {
    /// Apply it — restart the backend onto the staged core.
    Ready,
    /// Hold it, for a reason the dashboard shows verbatim.
    Deferred(DeferReason),
}

impl ApplyReadiness {
    pub fn is_ready(self) -> bool {
        matches!(self, ApplyReadiness::Ready)
    }

    pub fn reason(self) -> Option<DeferReason> {
        match self {
            ApplyReadiness::Ready => None,
            ApplyReadiness::Deferred(reason) => Some(reason),
        }
    }
}

/// Decide whether the automatic path may apply an update of this kind.
pub async fn apply_readiness(kind: UpdateKind) -> ApplyReadiness {
    let policy = UpdatePolicy::load();
    let open_positions = crate::positions::state::get_open_positions_count().await;
    decide(
        kind,
        policy.auto_install,
        policy.defer_while_trading,
        open_positions,
        crate::global::are_tools_active(),
    )
}

/// The decision itself, with every input explicit so it is testable.
pub(super) fn decide(
    kind: UpdateKind,
    auto_install: bool,
    defer_while_trading: bool,
    open_positions: usize,
    tools_active: bool,
) -> ApplyReadiness {
    if !kind.is_silent() {
        return ApplyReadiness::Deferred(DeferReason::NeedsInstaller);
    }
    if !auto_install {
        return ApplyReadiness::Deferred(DeferReason::AutomaticInstallDisabled);
    }
    if defer_while_trading && (open_positions > 0 || tools_active) {
        return ApplyReadiness::Deferred(DeferReason::TradingActive);
    }
    ApplyReadiness::Ready
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_update_is_never_applied_automatically() {
        assert_eq!(
            decide(UpdateKind::Full, true, false, 0, false),
            ApplyReadiness::Deferred(DeferReason::NeedsInstaller)
        );
    }

    #[test]
    fn automatic_installation_can_be_switched_off() {
        assert_eq!(
            decide(UpdateKind::Core, false, true, 0, false),
            ApplyReadiness::Deferred(DeferReason::AutomaticInstallDisabled)
        );
    }

    #[test]
    fn open_positions_and_running_tools_hold_the_restart() {
        assert_eq!(
            decide(UpdateKind::Core, true, true, 2, false),
            ApplyReadiness::Deferred(DeferReason::TradingActive)
        );
        assert_eq!(
            decide(UpdateKind::Core, true, true, 0, true),
            ApplyReadiness::Deferred(DeferReason::TradingActive)
        );
        // Deferring is itself optional; without it the restart goes ahead.
        assert_eq!(
            decide(UpdateKind::Core, true, false, 2, true),
            ApplyReadiness::Ready
        );
    }

    #[test]
    fn an_idle_process_applies_a_core_update_on_its_own() {
        assert_eq!(
            decide(UpdateKind::Core, true, true, 0, false),
            ApplyReadiness::Ready
        );
    }
}
