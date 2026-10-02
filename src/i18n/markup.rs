// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Inline markup in catalog messages.
//!
//! A message may contain the text-level tags in [`ALLOWED_TAGS`], written
//! without attributes, so emphasised text is one translatable message instead
//! of fixed-order runs. Markup is opt-in per call site. The pipeline is:
//!
//! 1. [`escape_args`] escapes every string argument,
//! 2. Fluent formats the pattern,
//! 3. [`sanitize`] rewrites the result into well-formed markup.
//!
//! Because arguments are escaped first and [`sanitize`] passes the five
//! entities [`escape_text`] produces through unchanged, each character is
//! escaped exactly once and an argument can never introduce a tag.
//!
//! `sanitizeMarkup` and `escapeMarkup` in
//! `src/webserver/templates/scripts/core/i18n.js` mirror this module; both are
//! pinned by `tools/tests/fixtures/i18n_markup.json`.

use fluent_bundle::{FluentArgs, FluentValue};

/// Tags a message may use. Mirrored by `MARKUP_TAGS` in `scripts/core/i18n.js`
/// and by `MARKUP_TAGS` in `tools/i18n/catalogs.mjs`.
pub const ALLOWED_TAGS: &[&str] = &["strong", "em", "b", "i", "code", "br"];

/// The void tag of [`ALLOWED_TAGS`].
const VOID_TAG: &str = "br";

/// Entities that [`escape_text`] emits and [`sanitize`] leaves untouched.
const ENTITIES: &[&str] = &["&lt;", "&gt;", "&amp;", "&quot;", "&#39;"];

/// Escape `&`, `<`, `>`, `"` and `'` so `s` is inert as markup text.
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Copy of `args` with every string value escaped. Numbers are kept; any other
/// value type is dropped.
pub fn escape_args(args: &FluentArgs) -> FluentArgs<'static> {
    let mut out = FluentArgs::new();
    for (name, value) in args.iter() {
        match value {
            FluentValue::String(s) => out.set(name.to_string(), escape_text(s)),
            FluentValue::Number(n) => out.set(name.to_string(), FluentValue::Number(n.clone())),
            _ => {}
        }
    }
    out
}

enum Tag {
    Open(&'static str),
    Close(&'static str),
    Break,
}

/// Parse an allowlisted, attribute-less tag at the start of `rest` (`<name>`,
/// `</name>`, or `<br/>`); returns the tag and its byte length.
fn parse_tag(rest: &str) -> Option<(Tag, usize)> {
    let body = rest.strip_prefix('<')?;
    let (closing, body) = match body.strip_prefix('/') {
        Some(after) => (true, after),
        None => (false, body),
    };
    let name_len = body.bytes().take_while(u8::is_ascii_alphanumeric).count();
    let name = body[..name_len].to_ascii_lowercase();
    let tag = *ALLOWED_TAGS.iter().find(|t| **t == name)?;
    let after = &body[name_len..];
    let prefix = rest.len() - after.len();
    if tag == VOID_TAG {
        if closing {
            return None;
        }
        if after.starts_with('>') {
            return Some((Tag::Break, prefix + 1));
        }
        if after.starts_with("/>") {
            return Some((Tag::Break, prefix + 2));
        }
        return None;
    }
    if !after.starts_with('>') {
        return None;
    }
    let tag = if closing {
        Tag::Close(tag)
    } else {
        Tag::Open(tag)
    };
    Some((tag, prefix + 1))
}

/// Rewrite a formatted message into well-formed markup: allowlisted
/// attribute-less tags are emitted in lowercase; every other `<` is emitted as
/// `&lt;` so it appears as literal text; unmatched closing tags are dropped and
/// unclosed tags are closed at the end.
pub fn sanitize(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    let mut open: Vec<&'static str> = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let rest = &input[i..];
        let Some(c) = rest.chars().next() else { break };
        match c {
            '<' => {
                if let Some((tag, len)) = parse_tag(rest) {
                    match tag {
                        Tag::Break => out.push_str("<br>"),
                        Tag::Open(name) => {
                            open.push(name);
                            out.push('<');
                            out.push_str(name);
                            out.push('>');
                        }
                        Tag::Close(name) => {
                            if let Some(depth) = open.iter().rposition(|t| *t == name) {
                                while open.len() > depth {
                                    if let Some(top) = open.pop() {
                                        out.push_str("</");
                                        out.push_str(top);
                                        out.push('>');
                                    }
                                }
                            }
                        }
                    }
                    i += len;
                    continue;
                }
                out.push_str("&lt;");
            }
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '&' => {
                if let Some(entity) = ENTITIES.iter().find(|e| rest.starts_with(**e)) {
                    out.push_str(entity);
                    i += entity.len();
                    continue;
                }
                out.push_str("&amp;");
            }
            _ => out.push(c),
        }
        i += c.len_utf8();
    }
    while let Some(top) = open.pop() {
        out.push_str("</");
        out.push_str(top);
        out.push('>');
    }
    out
}
