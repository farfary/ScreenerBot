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

/// Dashboard demo mode — show hardcoded showcase data for screenshots/marketing.
pub fn is_dashboard_demo_enabled() -> bool {
    has_arg("--dashboard-demo")
}

/// Demo capture mode — load the dashboard's demo capture runtime (overlays,
/// narration, quiescence reporting) so an external driver can produce
/// screenshots and promotional recordings deterministically. Implies demo data.
pub fn is_demo_capture_enabled() -> bool {
    has_arg("--demo-capture")
}

/// Demo freeze — pin every remaining live value (SOL price above all) to its
/// deterministic demo constant so two capture runs render byte-identical
/// frames. Only meaningful together with demo mode.
pub fn is_demo_freeze_enabled() -> bool {
    has_arg("--demo-freeze")
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
