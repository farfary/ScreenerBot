//! CPU Profiling Module
//!
//! Provides CPU profiling capabilities using multiple backends:
//! - tokio-console: Async task inspector (requires `console` feature)
//! - tracing: Detailed trace logging with thread/timing info
//! - pprof: CPU profiling with flamegraph generation (requires `flamegraph` feature)
//!
//! Usage:
//! - Call `init_profiling()` early in program initialization (before tokio tasks)
//! - Optionally call `start_cpu_profiling()` to start pprof-based profiling
//!
//! Command-line flags (see src/arguments.rs):
//! - `--profile-tokio-console`: Enable tokio-console
//! - `--profile-tracing`: Enable tracing subscriber
//! - `--profile-cpu`: Enable CPU profiling with pprof
//! - `--profile-duration <seconds>`: Set profiling duration (default: 60)

use crate::{
    arguments::{
        get_profile_duration, is_profile_cpu_enabled, is_profile_tokio_console_enabled,
        is_profile_tracing_enabled,
    },
    logger::{self, LogTag},
};

/// Initialize CPU profiling based on command-line flags
///
/// Must be called BEFORE any tokio tasks are spawned.
/// Checks flags in priority order:
/// 1. tokio-console (requires `console` feature)
/// 2. tracing subscriber
/// 3. pprof CPU profiling (requires `flamegraph` feature)
///
/// Only one profiling mode can be active at a time.
pub fn init_profiling() {
    // Tokio console profiling (async task inspector)
    #[cfg(feature = "console")]
    if is_profile_tokio_console_enabled() {
        console_subscriber::init();
        logger::info(
            LogTag::System,
            &"🔍 Tokio console enabled - connect with: tokio-console".to_string(),
        );
        logger::info(
            LogTag::System,
            &"   Install: cargo install tokio-console".to_string(),
        );
        logger::info(
            LogTag::System,
            &"   Connect: tokio-console".to_string(),
        );
        return;
    }

    // Tracing-based profiling
    if is_profile_tracing_enabled() {
        use tracing_subscriber::{fmt, EnvFilter};

        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_target(true)
            .with_line_number(true)
            .init();

        logger::info(
            LogTag::System,
            &"🔍 Tracing profiling enabled".to_string(),
        );
        logger::info(
            LogTag::System,
            &"   View detailed traces with thread IDs and timing".to_string(),
        );
        return;
    }

    // CPU profiling with pprof (will generate flamegraph on exit)
    #[cfg(feature = "flamegraph")]
    if is_profile_cpu_enabled() {
        let duration = get_profile_duration();
        logger::info(
            LogTag::System,
            &"🔥 CPU profiling enabled with pprof".to_string(),
        );
        logger::info(
            LogTag::System,
            &format!("   Duration: {} seconds", duration),
        );
        logger::info(
            LogTag::System,
            &"   Flamegraph will be generated on exit".to_string(),
        );
        logger::info(
            LogTag::System,
            &"   Press Ctrl+C to stop and generate flamegraph".to_string(),
        );

        // Note: pprof profiling is initialized later in the async context
        // This is just a notification
        return;
    }
}

/// Start CPU profiling guard (pprof-based)
///
/// Returns a guard that will generate a flamegraph when dropped.
/// The guard must be kept alive for the duration of profiling.
///
/// Returns:
/// - `Some(ProfilerGuard)` if CPU profiling is enabled and started successfully
/// - `None` if profiling is disabled or failed to start
///
/// Requires `flamegraph` feature and `--profile-cpu` flag.
#[cfg(feature = "flamegraph")]
pub fn start_cpu_profiling() -> Option<pprof::ProfilerGuard<'static>> {
    if !is_profile_cpu_enabled() {
        return None;
    }

    match pprof::ProfilerGuardBuilder::default()
        .frequency(997) // Sample at ~1000 Hz
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
    {
        Ok(guard) => {
            logger::info(LogTag::System, "🔥 CPU profiling started (pprof)");
            Some(guard)
        }
        Err(e) => {
            logger::error(
                LogTag::System,
                &format!("Failed to start CPU profiling: {}", e),
            );
            None
        }
    }
}

/// No-op version when flamegraph feature is not enabled
#[cfg(not(feature = "flamegraph"))]
pub fn start_cpu_profiling() -> Option<()> {
    None
}
