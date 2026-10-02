// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wire type for backend-authored display text.
//!
//! A `UiText` carries a catalog id and typed arguments; the presentation layer
//! (dashboard or `render`) formats it for the viewer's locale.

use super::{format, MessageId};
use fluent_bundle::{FluentArgs, FluentValue};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use unic_langid::LanguageIdentifier;

/// Typed message argument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum UiArg {
    Text(String),
    Count(i64),
    Number(f64),
    /// Decimal SOL amount as a string so precision is never lost.
    Sol(String),
    /// Decimal USD amount as a string so precision is never lost.
    Usd(String),
    Percent(f64),
    /// Unix time in milliseconds.
    Time(i64),
    /// Duration in milliseconds.
    Duration(u64),
    Nested(Box<UiText>),
}

/// Catalog id plus arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiText {
    pub id: Cow<'static, str>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<Cow<'static, str>, UiArg>,
}

impl UiText {
    pub fn new(id: MessageId) -> Self {
        Self {
            id: Cow::Borrowed(id.as_str()),
            args: BTreeMap::new(),
        }
    }

    /// Rebuild from an id that was persisted as a plain string.
    pub fn from_stored(id: String) -> Self {
        Self {
            id: Cow::Owned(id),
            args: BTreeMap::new(),
        }
    }

    pub fn arg(mut self, name: impl Into<Cow<'static, str>>, value: UiArg) -> Self {
        self.args.insert(name.into(), value);
        self
    }

    /// Format for `locale` on the Rust side.
    pub fn render(&self, locale: &LanguageIdentifier) -> String {
        self.render_args(locale, None)
    }

    /// Like [`UiText::render`], passing every string argument through `escape`
    /// before formatting, for markup-bearing targets such as Telegram HTML. A
    /// nested text is rendered through the same escaping and inserted as
    /// finished markup.
    pub fn render_with_arg_escape(
        &self,
        locale: &LanguageIdentifier,
        escape: fn(&str) -> String,
    ) -> String {
        self.render_args(locale, Some(escape))
    }

    fn render_args(
        &self,
        locale: &LanguageIdentifier,
        escape: Option<fn(&str) -> String>,
    ) -> String {
        if self.args.is_empty() {
            return format(locale, &self.id, None);
        }
        let mut args = FluentArgs::new();
        for (name, value) in &self.args {
            args.set(name.to_string(), fluent_value(value, locale, escape));
        }
        format(locale, &self.id, Some(&args))
    }

    /// Like [`UiText::render`] without the Unicode isolation marks Fluent wraps
    /// around interpolated values, for plain-text consumers such as API messages.
    pub fn render_plain(&self, locale: &LanguageIdentifier) -> String {
        self.render(locale)
            .chars()
            .filter(|c| !matches!(*c, FSI | PDI))
            .collect()
    }

    /// Plain text in the source locale, for logs, agent output and API `message`
    /// fields that must not depend on the viewer's locale.
    pub fn render_source_plain(&self) -> String {
        let source: LanguageIdentifier = super::source_locale()
            .parse()
            .unwrap_or_else(|_| LanguageIdentifier::default());
        self.render_plain(&source)
    }
}

/// First strong isolate and pop directional isolate, inserted around placeables.
const FSI: char = '\u{2068}';
const PDI: char = '\u{2069}';

fn fluent_value(
    arg: &UiArg,
    locale: &LanguageIdentifier,
    escape: Option<fn(&str) -> String>,
) -> FluentValue<'static> {
    match arg {
        UiArg::Text(s) | UiArg::Sol(s) | UiArg::Usd(s) => match escape {
            Some(escape) => FluentValue::from(escape(s)),
            None => FluentValue::from(s.clone()),
        },
        UiArg::Count(n) => FluentValue::from(*n),
        UiArg::Number(n) | UiArg::Percent(n) => FluentValue::from(*n),
        UiArg::Time(ms) => FluentValue::from(
            chrono::DateTime::from_timestamp_millis(*ms)
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| ms.to_string()),
        ),
        UiArg::Duration(ms) => FluentValue::from(i64::try_from(*ms).unwrap_or(i64::MAX)),
        UiArg::Nested(text) => FluentValue::from(text.render_args(locale, escape)),
    }
}
