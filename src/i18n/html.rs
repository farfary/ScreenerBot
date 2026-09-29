//! Server-side localization of static HTML.
//!
//! Elements carrying `data-l10n-id` are rewritten in one streaming pass so the
//! first paint is already translated. `data-l10n-args` holds an optional JSON
//! object of message arguments. The message value replaces the element's text
//! content as escaped text; message attributes are written as element
//! attributes when they are on [`L10N_ATTRIBUTES`].

use super::{format_message, LocalizedMessage};
use crate::logger::{self, LogTag};
use fluent_bundle::{FluentArgs, FluentValue};
use lol_html::html_content::{ContentType, Element};
use lol_html::{element, rewrite_str, HandlerResult, RewriteStrSettings};
use unic_langid::LanguageIdentifier;

const ID_ATTRIBUTE: &str = "data-l10n-id";
const ARGS_ATTRIBUTE: &str = "data-l10n-args";

/// Message attributes that may be written to an element. Mirrored by
/// `ATTRIBUTE_ALLOWLIST` in `scripts/core/i18n.js`.
pub const L10N_ATTRIBUTES: &[&str] = &[
    "title",
    "aria-label",
    "placeholder",
    "alt",
    "aria-description",
    "label",
];

/// Localize every `data-l10n-id` element in `html` for `locale`. On a rewrite
/// error the input is returned unchanged.
pub fn localize_html(html: &str, locale: &LanguageIdentifier) -> String {
    rewrite_with(html, &|id, args| format_message(locale, id, args))
}

/// Message lookup used by the rewriter: id and arguments to a formatted message.
pub(super) type Lookup<'a> = dyn Fn(&str, Option<&FluentArgs>) -> Option<LocalizedMessage> + 'a;

pub(super) fn rewrite_with(html: &str, lookup: &Lookup<'_>) -> String {
    let settings = RewriteStrSettings::new()
        .append_element_content_handler(element!("[data-l10n-id]", |el| localize_element(
            el, lookup
        )));
    match rewrite_str(html, settings) {
        Ok(out) => out,
        Err(err) => {
            logger::warning(
                LogTag::System,
                &format!("HTML localization failed, serving source text: {err}"),
            );
            html.to_string()
        }
    }
}

fn localize_element(el: &mut Element<'_, '_>, lookup: &Lookup<'_>) -> HandlerResult {
    let Some(id) = el.get_attribute(ID_ATTRIBUTE) else {
        return Ok(());
    };
    let args = el
        .get_attribute(ARGS_ATTRIBUTE)
        .map(|raw| parse_args(&id, &raw));
    let Some(message) = lookup(&id, args.as_ref()) else {
        logger::debug(LogTag::System, &format!("Unknown l10n id {id:?}"));
        return Ok(());
    };
    if let Some(value) = message.value {
        el.set_inner_content(&value, ContentType::Text);
    }
    for (name, value) in message.attributes {
        if L10N_ATTRIBUTES.contains(&name.as_str()) {
            el.set_attribute(&name, &value)?;
        } else {
            logger::debug(
                LogTag::System,
                &format!("Skipping non-allowlisted l10n attribute {name:?} on {id:?}"),
            );
        }
    }
    Ok(())
}

fn parse_args(id: &str, raw: &str) -> FluentArgs<'static> {
    let mut args = FluentArgs::new();
    let parsed: serde_json::Value = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(err) => {
            logger::debug(
                LogTag::System,
                &format!("Invalid data-l10n-args on {id:?}: {err}"),
            );
            return args;
        }
    };
    let Some(object) = parsed.as_object() else {
        logger::debug(
            LogTag::System,
            &format!("data-l10n-args on {id:?} is not an object"),
        );
        return args;
    };
    for (name, value) in object {
        let converted = match value {
            serde_json::Value::String(s) => Some(FluentValue::from(s.clone())),
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(i) => Some(FluentValue::from(i)),
                None => n.as_f64().map(FluentValue::from),
            },
            _ => None,
        };
        match converted {
            Some(value) => args.set(name.clone(), value),
            None => logger::debug(
                LogTag::System,
                &format!("Skipping unsupported l10n arg {name:?} on {id:?}"),
            ),
        }
    }
    args
}
