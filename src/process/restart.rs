//! Restart-after-graceful-shutdown logic.

use crate::logger::{error, info, LogTag};
use std::process::Command;

/// Electron owns its backend child and must be the component that relaunches it.
/// A distinct exit code separates an intentional restart from a crash.
const RESTART_EXIT_CODE: i32 = 75;

/// Replace/relaunch the backend only after `run_bot` has stopped all services
/// and dropped its process lock.
pub fn restart_after_graceful_shutdown() -> ! {
    use std::io::Write;

    if crate::arguments::is_gui_enabled() {
        // Electron must retain ownership of the backend process so quitting the
        // desktop app can still terminate it. Its main process handles this
        // signal/exit code and loads the new dynamic-port dashboard automatically.
        println!("SCREENERBOT_RESTART");
        let _ = std::io::stdout().flush();
        std::process::exit(RESTART_EXIT_CODE);
    }

    let executable = std::env::current_exe().unwrap_or_else(|restart_error| {
        error(
            LogTag::System,
            &format!("Could not resolve executable for restart: {restart_error}"),
        );
        std::process::exit(1);
    });
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();

    info(
        LogTag::System,
        "Graceful shutdown complete; restarting ScreenerBot",
    );

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let exec_error = Command::new(executable).args(args).exec();
        error(
            LogTag::System,
            &format!("Could not replace process during restart: {exec_error}"),
        );
        std::process::exit(1);
    }

    #[cfg(windows)]
    {
        match Command::new(executable).args(args).spawn() {
            Ok(_) => std::process::exit(0),
            Err(spawn_error) => {
                error(
                    LogTag::System,
                    &format!("Could not relaunch process during restart: {spawn_error}"),
                );
                std::process::exit(1);
            }
        }
    }
}
