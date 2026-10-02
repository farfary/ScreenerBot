// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The system-prompt line that sets the assistant's reply language.
//!
//! The model answers in the dashboard interface language unless the user writes
//! in another one. Tool names, arguments and JSON stay as defined, so only the
//! prose changes.

use crate::config::{is_config_initialized, with_config};
use crate::i18n::{locale_info, resolve_known_setting, source_locale, LanguageIdentifier};

/// Reply-language instruction for the configured interface language.
pub(super) fn reply_language_line() -> String {
    line_for(&interface_locale())
}

fn interface_locale() -> LanguageIdentifier {
    let setting = if is_config_initialized() {
        with_config(|cfg| cfg.gui.dashboard.interface.language.clone())
    } else {
        source_locale().to_owned()
    };
    resolve_known_setting(&setting).unwrap_or_else(|| {
        source_locale()
            .parse()
            .unwrap_or_else(|_| LanguageIdentifier::default())
    })
}

fn line_for(locale: &LanguageIdentifier) -> String {
    let code = locale.to_string();
    let language = locale_info(&code)
        .map(|info| format!("{} ({code})", info.name))
        .unwrap_or(code);
    format!(
        "Reply in {language}, the user's interface language, unless the user writes in \
         another language; then reply in theirs. Keep tool names, arguments, token symbols, \
         addresses and numbers exactly as they are.\n\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_registered_language_and_code() {
        let line = line_for(&"en".parse().unwrap());
        assert!(line.starts_with("Reply in English (en)"), "{line}");
    }

    #[test]
    fn falls_back_to_the_code_for_an_unregistered_locale() {
        let line = line_for(&"xx".parse().unwrap());
        assert!(line.starts_with("Reply in xx,"), "{line}");
    }
}
