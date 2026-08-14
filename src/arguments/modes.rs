//! Execution and modifier mode checkers.

use super::has_arg;

// =============================================================================
// EXECUTION MODES
// =============================================================================

/// GUI mode — launch with desktop window (requires 'gui' feature at compile time).
pub fn is_gui_enabled() -> bool {
    has_arg("--gui")
}

/// Reset mode — clears pending verifications and database files.
pub fn is_reset_enabled() -> bool {
    has_arg("--reset")
}

/// Promo fixtures — show deterministic showcase data for owner-only media capture.
pub fn is_promo_fixtures_enabled() -> bool {
    has_arg("--promo-fixtures")
}

/// Promo capture — load the dashboard's owner-only capture runtime (overlays,
/// narration, quiescence reporting) so an external driver can produce
/// screenshots and promotional recordings deterministically. Implies promo fixtures.
pub fn is_promo_capture_enabled() -> bool {
    has_arg("--promo-capture")
}

/// Promo freeze — pin every remaining live value (SOL price above all) to its
/// deterministic promo constant so two capture runs render byte-identical
/// frames. Only meaningful together with promo fixtures.
pub fn is_promo_freeze_enabled() -> bool {
    has_arg("--promo-freeze")
}

/// Force onboarding display — show onboarding even if already completed.
pub fn is_dashboard_onboarding_forced() -> bool {
    has_arg("--dashboard-onboarding")
}

// =============================================================================
// MODIFIER FLAGS
// =============================================================================

/// Force mode — skip confirmation prompts.
pub fn is_force_enabled() -> bool {
    has_arg("--force")
}

/// Cache-only mode — read from local DB only, never call RPC.
pub fn is_cache_only_enabled() -> bool {
    has_arg("--cache-only")
}

/// Force-refresh mode — refresh from RPC even if cached.
pub fn is_force_refresh_enabled() -> bool {
    has_arg("--force-refresh")
}

/// Clean wallet data — delete all wallet-specific databases.
pub fn is_clean_wallet_data_enabled() -> bool {
    has_arg("--clean-wallet-data")
}

/// Reset configuration to defaults while preserving wallet and RPC URLs.
pub fn is_reset_default_configs_enabled() -> bool {
    has_arg("--reset-default-configs")
}

// =============================================================================
// PROFILING FLAGS
// =============================================================================

/// Enable tokio-console for async task profiling.
pub fn is_profile_tokio_console_enabled() -> bool {
    has_arg("--profile-tokio-console")
}

/// Enable detailed tracing for performance analysis.
pub fn is_profile_tracing_enabled() -> bool {
    has_arg("--profile-tracing")
}

/// Enable CPU profiling with pprof and flamegraph generation.
pub fn is_profile_cpu_enabled() -> bool {
    has_arg("--profile-cpu")
}
