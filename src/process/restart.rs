// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Restart-after-graceful-shutdown logic.

use crate::logger::{error, info, LogTag};
use std::ffi::OsString;
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
    let args = restart_args(std::env::args_os().skip(1));

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

/// The arguments a relaunched process receives: this launch's arguments without the
/// one-shot wallet data reset, which acts once per launch and must not repeat on every
/// in-process restart.
fn restart_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    args.into_iter()
        .filter(|arg| arg.as_os_str() != crate::arguments::CLEAN_WALLET_DATA_FLAG)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_restart_drops_the_wallet_data_reset_and_keeps_every_other_argument_in_order() {
        let args = [
            "--port",
            "9000",
            "--clean-wallet-data",
            "--debug-api",
            "--clean-wallet-data",
        ]
        .map(OsString::from);
        assert_eq!(
            restart_args(args),
            ["--port", "9000", "--debug-api"]
                .map(OsString::from)
                .to_vec()
        );
    }
}
