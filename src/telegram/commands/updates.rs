// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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
        _ => with_icon(
            "⬆️",
            &tg(&available_text(
                update.kind,
                version::self_install_supported(update.kind),
                &update.version,
            )
            .arg("size", UiArg::Text(format!("{size_mb:.1}")))),
        ),
    }
}

/// What an available release needs. A release this process cannot install itself (a
/// full release on a headless install) is installed by the package manager on the
/// server and never downloads on its own, so its notice names that command instead.
fn available_text(kind: UpdateKind, self_install: bool, version: &str) -> UiText {
    let (message, how) = if !self_install {
        (
            ids::TELEGRAM_UPDATE_AVAILABLE_HEADLESS,
            ids::TELEGRAM_UPDATE_HOW_HEADLESS,
        )
    } else if kind == UpdateKind::Core {
        (
            ids::TELEGRAM_UPDATE_AVAILABLE,
            ids::TELEGRAM_UPDATE_HOW_CORE,
        )
    } else {
        (
            ids::TELEGRAM_UPDATE_AVAILABLE,
            ids::TELEGRAM_UPDATE_HOW_INSTALLER,
        )
    };
    version_text(message, version).arg("how", UiArg::Nested(Box::new(UiText::new(how))))
}

#[cfg(test)]
mod tests {
    use super::available_text;
    use crate::i18n::UiArg;
    use crate::version::UpdateKind;

    fn english(kind: UpdateKind, self_install: bool) -> String {
        available_text(kind, self_install, "0.3.0")
            .arg("size", UiArg::Text("42.0".to_owned()))
            .render_source_plain()
    }

    #[test]
    fn a_headless_install_is_told_to_run_the_package_manager() {
        let text = english(UpdateKind::Full, false);
        assert!(text.contains("sudo screenerbot-manager update"), "{text}");
        assert!(!text.contains("downloads on its own"), "{text}");
    }

    #[test]
    fn a_desktop_install_downloads_the_release_itself() {
        for kind in [UpdateKind::Core, UpdateKind::Full] {
            let text = english(kind, true);
            assert!(text.contains("downloads on its own"), "{text}");
            assert!(!text.contains("screenerbot-manager"), "{text}");
        }
    }
}
