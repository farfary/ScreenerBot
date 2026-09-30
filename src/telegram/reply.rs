//! Persistent reply keyboard commands.
//!
//! Telegram delivers a reply-keyboard tap as plain text equal to the button
//! label. Labels are localized, so incoming text is matched against the label
//! in the current language, the source language and every registered language;
//! a keyboard sent before a language change keeps working.

use crate::i18n::{
    available_locales, ids, source_locale, LanguageIdentifier, MessageId, UiText, PSEUDO_LOCALES,
};
use crate::telegram::text::{locale, tg_plain_with, with_icon};

/// One button of the reply keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyCommand {
    Status,
    Balance,
    Positions,
    Pause,
    Resume,
    Stop,
    Stats,
    Menu,
    Help,
}

impl ReplyCommand {
    /// Keyboard order, row by row.
    pub const ROWS: [[ReplyCommand; 3]; 3] = [
        [Self::Status, Self::Balance, Self::Positions],
        [Self::Pause, Self::Resume, Self::Stop],
        [Self::Stats, Self::Menu, Self::Help],
    ];

    pub const ALL: [ReplyCommand; 9] = [
        Self::Status,
        Self::Balance,
        Self::Positions,
        Self::Pause,
        Self::Resume,
        Self::Stop,
        Self::Stats,
        Self::Menu,
        Self::Help,
    ];

    pub fn icon(self) -> &'static str {
        match self {
            Self::Status => "📊",
            Self::Balance => "💰",
            Self::Positions => "📈",
            Self::Pause => "⏸️",
            Self::Resume => "▶️",
            Self::Stop => "🛑",
            Self::Stats => "📉",
            Self::Menu => "⚙️",
            Self::Help => "❓",
        }
    }

    pub fn label_id(self) -> MessageId {
        match self {
            Self::Status => ids::TELEGRAM_REPLY_STATUS,
            Self::Balance => ids::TELEGRAM_REPLY_BALANCE,
            Self::Positions => ids::TELEGRAM_REPLY_POSITIONS,
            Self::Pause => ids::TELEGRAM_REPLY_PAUSE,
            Self::Resume => ids::TELEGRAM_REPLY_RESUME,
            Self::Stop => ids::TELEGRAM_REPLY_STOP,
            Self::Stats => ids::TELEGRAM_REPLY_STATS,
            Self::Menu => ids::TELEGRAM_REPLY_MENU,
            Self::Help => ids::TELEGRAM_REPLY_HELP,
        }
    }

    /// Slash command this button runs.
    pub fn command(self) -> &'static str {
        match self {
            Self::Status => "/status",
            Self::Balance => "/balance",
            Self::Positions => "/positions",
            Self::Pause => "/pause",
            Self::Resume => "/resume",
            Self::Stop => "/force_stop",
            Self::Stats => "/stats",
            Self::Menu => "/menu",
            Self::Help => "/help",
        }
    }

    /// Button text: the icon followed by the word in `locale`.
    pub fn label(self, locale: &LanguageIdentifier) -> String {
        with_icon(
            self.icon(),
            &tg_plain_with(locale, &UiText::new(self.label_id())),
        )
    }

    /// The command whose label in the current, source, any registered or any
    /// pseudo language equals `text`.
    pub fn from_label(text: &str) -> Option<Self> {
        let mut locales = vec![locale()];
        locales.extend(source_locale().parse::<LanguageIdentifier>().ok());
        locales.extend(
            available_locales()
                .iter()
                .filter_map(|info| info.code.parse::<LanguageIdentifier>().ok()),
        );
        locales.extend(
            PSEUDO_LOCALES
                .iter()
                .filter_map(|pseudo| pseudo.code().parse::<LanguageIdentifier>().ok()),
        );
        Self::ALL
            .into_iter()
            .find(|cmd| locales.iter().any(|l| cmd.label(l) == text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::source_message_ids;
    use std::collections::HashSet;

    fn tested_locales() -> Vec<LanguageIdentifier> {
        let mut out: Vec<LanguageIdentifier> = available_locales()
            .iter()
            .map(|info| info.code.parse().expect("registered code parses"))
            .collect();
        out.push("en-XA".parse().expect("pseudo-locale parses"));
        out
    }

    #[test]
    fn labels_are_distinct_and_route_back_in_every_locale() {
        for locale in tested_locales() {
            let labels: HashSet<String> = ReplyCommand::ALL
                .iter()
                .map(|cmd| cmd.label(&locale))
                .collect();
            assert_eq!(labels.len(), ReplyCommand::ALL.len(), "{locale}");
            for cmd in ReplyCommand::ALL {
                assert_eq!(
                    ReplyCommand::from_label(&cmd.label(&locale)),
                    Some(cmd),
                    "{locale}: {cmd:?}"
                );
            }
        }
    }

    #[test]
    fn label_ids_exist_in_the_source_catalog() {
        for cmd in ReplyCommand::ALL {
            assert!(
                source_message_ids().contains(&cmd.label_id().as_str()),
                "{cmd:?}"
            );
        }
    }

    #[test]
    fn rows_cover_every_command_once() {
        let flat: Vec<ReplyCommand> = ReplyCommand::ROWS.iter().flatten().copied().collect();
        assert_eq!(flat, ReplyCommand::ALL);
    }

    #[test]
    fn source_labels_match_the_established_keyboard_text() {
        let source: LanguageIdentifier = source_locale().parse().unwrap();
        let expected = [
            "📊 Status",
            "💰 Balance",
            "📈 Positions",
            "⏸️ Pause",
            "▶️ Resume",
            "🛑 Stop",
            "📉 Stats",
            "⚙️ Menu",
            "❓ Help",
        ];
        for (cmd, want) in ReplyCommand::ALL.iter().zip(expected) {
            assert_eq!(cmd.label(&source), want);
        }
    }

    #[test]
    fn unrelated_text_does_not_route() {
        assert_eq!(ReplyCommand::from_label("hello"), None);
    }
}
