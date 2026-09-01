//! Automatic update configuration.

use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// UPDATES CONFIGURATION
// ============================================================================

config_struct! {
    /// How ScreenerBot keeps itself current.
    ///
    /// A release ships two components: the core binary (which also carries the
    /// dashboard) and the Electron desktop shell. Core-only releases install
    /// silently with a backend restart; a release that also changes the shell
    /// needs the operating-system installer to run once.
    pub struct UpdatesConfig {
        #[metadata(field_metadata! {
            label: "Check for Updates",
            hint: "Periodically ask screenerbot.io whether a newer release is published",
            impact: "medium",
            category: "Checking",
        })]
        auto_check: bool = true,

        #[metadata(field_metadata! {
            label: "Check Interval",
            hint: "How often to check for a newer release",
            impact: "low",
            category: "Checking",
            min: 1.0,
            max: 168.0,
            step: 1.0,
            unit: "hours",
        })]
        check_interval_hours: u64 = 6,

        #[metadata(field_metadata! {
            label: "Download Automatically",
            hint: "Fetch and verify a new release in the background as soon as it is found",
            impact: "medium",
            category: "Installing",
        })]
        auto_download: bool = true,

        #[metadata(field_metadata! {
            label: "Install Automatically",
            hint: "Apply a verified core update on its own, with a short backend restart. Turn this off to be asked first.",
            impact: "high",
            category: "Installing",
        })]
        auto_install: bool = true,

        #[metadata(field_metadata! {
            label: "Wait While Trading",
            hint: "Postpone the restart while positions are open. The update still applies the next time ScreenerBot starts.",
            impact: "high",
            category: "Installing",
        })]
        defer_while_trading: bool = true,

        #[metadata(field_metadata! {
            label: "Announce on Telegram",
            hint: "Send a Telegram message when an update is found, staged, or applied",
            impact: "low",
            category: "Notifications",
        })]
        notify_telegram: bool = true,
    }
}
