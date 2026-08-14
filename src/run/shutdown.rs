//! Shutdown and initialization signal handling.

use crate::{
    global,
    logger::{self, LogTag},
};

/// Wait for shutdown signal (Ctrl+C, SIGTERM, SIGQUIT on Unix).
///
/// NOTE: SIGHUP is intentionally NOT handled. SIGHUP is sent when a terminal
/// disconnects (e.g., SSH session closes, nohup usage). A headless trading bot
/// must survive terminal disconnects. Use SIGTERM or Ctrl+C to stop the bot.
pub(super) async fn wait_for_shutdown_signal() -> Result<(), String> {
    logger::info(
        LogTag::System,
        "Waiting for shutdown signal (press Ctrl+C twice to force kill)",
    );

    if global::is_restart_requested() {
        logger::info(LogTag::System, "Graceful restart requested");
        return Ok(());
    }

    let signal_name = tokio::select! {
        _ = global::wait_for_restart_request() => {
            logger::info(LogTag::System, "Graceful restart requested");
            return Ok(());
        }
        result = wait_for_os_shutdown_signal() => result?,
    };

    logger::warning(
        LogTag::System,
        &format!(
            "Shutdown signal received ({}). Press Ctrl+C again to force kill.",
            signal_name
        ),
    );

    // Spawn a background listener for a second Ctrl+C to exit immediately
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            logger::error(
                LogTag::System,
                "Second Ctrl+C detected — forcing immediate exit.",
            );
            std::process::exit(130);
        }
    });

    Ok(())
}

/// Wait for a platform shutdown signal and return its display name.
async fn wait_for_os_shutdown_signal() -> Result<&'static str, String> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigint =
            signal(SignalKind::interrupt()).map_err(|e| format!("Failed to bind SIGINT: {e}"))?;
        let mut sigterm =
            signal(SignalKind::terminate()).map_err(|e| format!("Failed to bind SIGTERM: {e}"))?;
        let mut sigquit =
            signal(SignalKind::quit()).map_err(|e| format!("Failed to bind SIGQUIT: {e}"))?;

        Ok(tokio::select! {
            _ = sigint.recv() => "SIGINT",
            _ = sigterm.recv() => "SIGTERM",
            _ = sigquit.recv() => "SIGQUIT",
        })
    }

    #[cfg(windows)]
    {
        // On Windows, ctrl_c() handles Ctrl+C and Ctrl+Break
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| format!("Failed to listen for shutdown signal: {e}"))?;
        Ok("CTRL_C")
    }
}

/// Wait until setup enters either Explore Mode or full mode, or for shutdown.
pub(super) async fn wait_for_operational_mode_or_shutdown() -> Result<(), String> {
    use tokio::time::{sleep, Duration, Instant};

    const MAX_WAIT_DURATION: Duration = Duration::from_secs(30 * 60); // 30 minutes
    const WARNING_INTERVAL: Duration = Duration::from_secs(5 * 60); // Warn every 5 minutes

    let start = Instant::now();
    let mut last_warning = start;

    loop {
        if global::is_restart_requested() {
            logger::info(LogTag::System, "Restart requested during initialization");
            return Ok(());
        }

        // Choosing Explore Mode is also a completed transition: it keeps the
        // dashboard running without wallet/RPC-backed services.
        if global::is_explore_or_full() {
            logger::info(
                LogTag::System,
                "Dashboard mode selected - services started successfully",
            );
            return Ok(());
        }

        // Check elapsed time
        let elapsed = start.elapsed();
        if elapsed >= MAX_WAIT_DURATION {
            logger::error(
                LogTag::System,
                &format!(
                    "Setup timeout after {} minutes - no dashboard mode was selected",
                    MAX_WAIT_DURATION.as_secs() / 60
                ),
            );
            return Err(format!(
                "Setup timeout after {} minutes",
                MAX_WAIT_DURATION.as_secs() / 60
            ));
        }

        // Periodic warning logs
        if elapsed - (last_warning - start) >= WARNING_INTERVAL {
            logger::warning(
                LogTag::System,
                &format!(
                    "Still waiting for initialization... ({} minutes elapsed)",
                    elapsed.as_secs() / 60
                ),
            );
            last_warning = Instant::now();
        }

        // Check for Ctrl+C (non-blocking)
        tokio::select! {
          _ = tokio::signal::ctrl_c() => {
            logger::warning(
              LogTag::System,
              "Shutdown signal received during initialization",
            );
            return Err("Shutdown during initialization".to_owned());
          }
          _ = sleep(Duration::from_millis(500)) => {
            // Continue polling
          }
        }
    }
}
