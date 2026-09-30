//! `/update` — see what is waiting, and approve it from the phone.
//!
//! The dashboard is the primary surface for updates; this exists so a staged
//! release can be approved without opening the machine. It only ever applies an
//! update that is already downloaded and verified, so the command itself never
//! touches the network.

use crate::i18n::{ids, MessageId, UiArg, UiText};
use crate::telegram::text::{tg, tg_id, tg_plain, with_icon};
use crate::version::{self, UpdateKind, UpdatePhase};

fn version_text(id: MessageId, version: &str) -> UiText {
    UiText::new(id).arg("version", UiArg::Text(version.to_owned()))
}

pub async fn handle_update_command() -> String {
    let state = version::get_update_state().await;
    let current = version::get_version();

    let Some(update) = state.available_update.clone() else {
        return match state.phase {
            UpdatePhase::Applied => with_icon(
                "✅",
                &tg(&version_text(
                    ids::TELEGRAM_UPDATE_UP_TO_DATE_AUTO,
                    &current,
                )),
            ),
            UpdatePhase::CheckFailed => {
                // The failure text comes from another catalog domain and is plain
                // text, so it travels as an escaped argument, not as markup.
                let reason = tg_plain(
                    &state
                        .check_error
                        .unwrap_or_else(|| UiText::new(ids::TELEGRAM_UPDATE_UNREACHABLE)),
                );
                with_icon(
                    "⚠️",
                    &tg(&UiText::new(ids::TELEGRAM_UPDATE_CHECK_FAILED)
                        .arg("reason", UiArg::Text(reason))),
                )
            }
            _ => with_icon(
                "✅",
                &tg(&version_text(ids::TELEGRAM_UPDATE_UP_TO_DATE, &current)),
            ),
        };
    };

    let size_mb = update.transfer_size() as f64 / (1024.0 * 1024.0);
    let installing = || {
        with_icon(
            "🔄",
            &tg(&version_text(
                ids::TELEGRAM_UPDATE_INSTALLING,
                &update.version,
            )),
        )
    };
    match state.phase {
        UpdatePhase::ReadyToApply => match version::apply_now().await {
            Ok(()) => format!(
                "{}\n\n{}",
                installing(),
                tg_id(ids::TELEGRAM_UPDATE_RESTARTING)
            ),
            Err(error) => with_icon(
                "⚠️",
                &tg(
                    &version_text(ids::TELEGRAM_UPDATE_INSTALL_FAILED, &update.version)
                        .arg("detail", UiArg::Text(error.to_string())),
                ),
            ),
        },
        UpdatePhase::ReadyToInstall => with_icon(
            "📦",
            &tg(&version_text(
                ids::TELEGRAM_UPDATE_DOWNLOADED,
                &update.version,
            )),
        ),
        UpdatePhase::Downloading | UpdatePhase::Verifying => with_icon(
            "⬇️",
            &tg(
                &version_text(ids::TELEGRAM_UPDATE_DOWNLOADING, &update.version)
                    .arg(
                        "percent",
                        UiArg::Text(format!("{:.0}", state.download_progress.progress_percent)),
                    )
                    .arg("size", UiArg::Text(format!("{size_mb:.1}"))),
            ),
        ),
        UpdatePhase::Applying => installing(),
        _ => {
            let how = if update.kind == UpdateKind::Core {
                ids::TELEGRAM_UPDATE_HOW_CORE
            } else {
                ids::TELEGRAM_UPDATE_HOW_INSTALLER
            };
            with_icon(
                "⬆️",
                &tg(
                    &version_text(ids::TELEGRAM_UPDATE_AVAILABLE, &update.version)
                        .arg("how", UiArg::Nested(Box::new(UiText::new(how))))
                        .arg("size", UiArg::Text(format!("{size_mb:.1}"))),
                ),
            )
        }
    }
}
