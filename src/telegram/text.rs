//! Localized text for Telegram messages and buttons.
//!
//! Messages are sent with `ParseMode::Html`. [`tg`] renders a [`UiText`] for
//! the configured Telegram language and HTML-escapes every string argument, so
//! interpolated values (symbols, error details, chat names) can never
//! introduce markup; only the tags written in the catalog survive. Button
//! labels and other plain-text targets use [`tg_plain`], which does not escape.
//!
//! Emoji that act as icons are prepended by the caller through [`with_icon`];
//! catalog text never contains them.

use crate::config::{is_config_initialized, try_with_config, with_config};
use crate::i18n::{
    escape_text, locale_info, resolve_known_setting, resolve_locale, source_locale,
    LanguageIdentifier, MessageId, PseudoLocale, UiText, SYSTEM_SETTING,
};

/// `telegram.language` value that follows the dashboard language.
pub const FOLLOW_APP: &str = "app";

/// First strong isolate and pop directional isolate that Fluent wraps around
/// placeables. They would ride along when a value is copied from chat.
const ISOLATES: [char; 2] = ['\u{2068}', '\u{2069}'];

/// Language of outgoing Telegram text: `telegram.language`, or the dashboard
/// language when it is `app`, empty or not a known locale code.
pub fn locale() -> LanguageIdentifier {
    let (own, app) = if is_config_initialized() {
        with_config(|cfg| {
            (
                cfg.telegram.language.clone(),
                cfg.gui.dashboard.interface.language.clone(),
            )
        })
    } else {
        (FOLLOW_APP.to_owned(), SYSTEM_SETTING.to_owned())
    };
    let own = own.trim();
    let known = locale_info(own).is_some() || PseudoLocale::from_code(own).is_some();
    resolve_locale(if known { own } else { &app }, None)
}

/// [`locale`] for contexts that must never wait, such as the panic hook.
/// Reads the configuration only through `try_read`; when the lock is contended
/// or poisoned, or the configuration is not loaded, it returns the source locale.
pub fn locale_nonblocking() -> LanguageIdentifier {
    try_with_config(|cfg| {
        let own = cfg.telegram.language.trim();
        let known = locale_info(own).is_some() || PseudoLocale::from_code(own).is_some();
        let setting = if known {
            own
        } else {
            cfg.gui.dashboard.interface.language.as_str()
        };
        resolve_known_setting(setting)
    })
    .flatten()
    .unwrap_or_else(|| {
        source_locale()
            .parse()
            .unwrap_or_else(|_| LanguageIdentifier::default())
    })
}

/// HTML-escaped rendering of `text` for `locale`.
pub fn tg_with(locale: &LanguageIdentifier, text: &UiText) -> String {
    text.render_with_arg_escape(locale, escape_text)
        .chars()
        .filter(|c| !ISOLATES.contains(c))
        .collect()
}

/// HTML rendering of `text` for the Telegram language.
pub fn tg(text: &UiText) -> String {
    tg_with(&locale(), text)
}

/// HTML rendering of an argument-free message.
pub fn tg_id(id: MessageId) -> String {
    tg(&UiText::new(id))
}

/// Plain-text rendering for `locale`, for button labels.
pub fn tg_plain_with(locale: &LanguageIdentifier, text: &UiText) -> String {
    text.render_plain(locale)
}

/// Plain-text rendering of `text` for the Telegram language.
pub fn tg_plain(text: &UiText) -> String {
    tg_plain_with(&locale(), text)
}

/// Plain-text rendering of an argument-free message.
pub fn tg_plain_id(id: MessageId) -> String {
    tg_plain(&UiText::new(id))
}

/// Escape a runtime string (symbol, error detail) for Telegram HTML.
pub fn tg_escape(value: &str) -> String {
    escape_text(value)
}

/// Prefix `label` with an icon glyph.
pub fn with_icon(icon: &str, label: &str) -> String {
    format!("{icon} {label}")
}
