//! `/update` — see what is waiting, and approve it from the phone.
//!
//! The dashboard is the primary surface for updates; this exists so a staged
//! release can be approved without opening the machine. It only ever applies an
//! update that is already downloaded and verified, so the command itself never
//! touches the network.

use crate::version::{self, UpdateKind, UpdatePhase};

pub async fn handle_update_command() -> String {
    let state = version::get_update_state().await;
    let current = version::get_version();

    let Some(update) = state.available_update.clone() else {
        return match state.phase {
            UpdatePhase::Applied => {
                format!("✅ <b>Up to date</b>\n\nRunning v{current}, installed automatically.")
            }
            UpdatePhase::CheckFailed => format!(
                "⚠️ <b>Update check failed</b>\n\n{}",
                state
                    .check_error
                    .unwrap_or_else(|| "screenerbot.io could not be reached.".to_owned())
            ),
            _ => format!("✅ <b>Up to date</b>\n\nRunning v{current}."),
        };
    };

    let size_mb = update.transfer_size() as f64 / (1024.0 * 1024.0);
    match state.phase {
        UpdatePhase::ReadyToApply => match version::apply_now().await {
            Ok(()) => format!(
                "🔄 <b>Installing v{}</b>\n\nScreenerBot is restarting onto the new version. \
                 Trading resumes automatically.",
                update.version
            ),
            Err(error) => format!("⚠️ <b>Could not install v{}</b>\n\n{error}", update.version),
        },
        UpdatePhase::ReadyToInstall => format!(
            "📦 <b>v{} is downloaded</b>\n\nThis release also updates the desktop app, so its \
             installer has to run on the machine. Open Settings → Updates there.",
            update.version
        ),
        UpdatePhase::Downloading | UpdatePhase::Verifying => format!(
            "⬇️ <b>Downloading v{}</b>\n\n{:.0}% of {size_mb:.1} MB.",
            update.version, state.download_progress.progress_percent
        ),
        UpdatePhase::Applying => format!("🔄 <b>Installing v{}</b>", update.version),
        _ => {
            let how = if update.kind == UpdateKind::Core {
                "Installs silently with a short restart."
            } else {
                "Needs the desktop installer to run once."
            };
            format!(
                "⬆️ <b>v{} is available</b>\n\n{how}\nDownload size: {size_mb:.1} MB.\n\n\
                 It downloads on its own; send /update again once it is ready.",
                update.version
            )
        }
    }
}
