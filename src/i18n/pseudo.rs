//! Pseudo-locales for visual QA of untranslated text, truncation and
//! right-to-left layout. They are derived from the source catalog by a Fluent
//! bundle transform and are never registered, offered or negotiated.
//!
//! `src/webserver/templates/scripts/core/i18n.js` mirrors the transforms and
//! tables below; `tools/tests/fixtures/i18n_pseudo.json` pins both.

use super::registry::{LocaleInfo, TextDirection};
use std::borrow::Cow;

const ACCENT_UPPER: &str = "ȦƁƇḒḖƑƓĦĪĴĶĿḾȠǾƤɊŘŞŦŬṼẆẊẎẐ";
const ACCENT_LOWER: &str = "ȧƀƈḓḗƒɠħīĵķŀḿƞǿƥɋřşŧŭṽẇẋẏẑ";
const BIDI_UPPER: &str = "∀ԐↃᗡƎℲ⅁HIſӼ⅂WNOԀÒᴚS⊥∩ɅMX⅄Z";
const BIDI_LOWER: &str = "ɐqɔpǝɟƃɥıɾʞʅɯuodbɹsʇnʌʍxʎz";

/// Right-to-left override, and pop directional formatting.
const RLO: char = '\u{202E}';
const PDF: char = '\u{202C}';

/// Number of Unicode scalar values in `s` (UTF-8 bytes that are not continuation bytes).
const fn scalar_count(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] & 0xC0 != 0x80 {
            count += 1;
        }
        i += 1;
    }
    count
}

const _: () = {
    assert!(scalar_count(ACCENT_UPPER) == 26);
    assert!(scalar_count(ACCENT_LOWER) == 26);
    assert!(scalar_count(BIDI_UPPER) == 26);
    assert!(scalar_count(BIDI_LOWER) == 26);
};

/// A developer-only display locale that is active solely when configured by exact code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoLocale {
    Accented,
    Bidi,
}

pub const PSEUDO_LOCALES: &[PseudoLocale] = &[PseudoLocale::Accented, PseudoLocale::Bidi];

impl PseudoLocale {
    pub fn code(&self) -> &'static str {
        match self {
            PseudoLocale::Accented => "en-XA",
            PseudoLocale::Bidi => "ar-XB",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PseudoLocale::Accented => "Pseudo (accented)",
            PseudoLocale::Bidi => "Pseudo (bidi)",
        }
    }

    pub fn dir(&self) -> TextDirection {
        match self {
            PseudoLocale::Accented => TextDirection::Ltr,
            PseudoLocale::Bidi => TextDirection::Rtl,
        }
    }

    /// Value of the `pseudo` field in the dashboard catalog payload.
    pub fn kind(&self) -> &'static str {
        match self {
            PseudoLocale::Accented => "accented",
            PseudoLocale::Bidi => "bidi",
        }
    }

    pub fn from_code(code: &str) -> Option<PseudoLocale> {
        PSEUDO_LOCALES.iter().copied().find(|p| p.code() == code)
    }

    pub(crate) fn info(&self) -> LocaleInfo {
        LocaleInfo {
            code: self.code().to_string(),
            name: self.name().to_string(),
            dir: self.dir(),
        }
    }

    pub(crate) fn transform(&self) -> fn(&str) -> Cow<'_, str> {
        match self {
            PseudoLocale::Accented => transform_accented,
            PseudoLocale::Bidi => transform_bidi,
        }
    }
}

/// Replacement for an ASCII letter from a pair of 26-scalar tables.
fn map_letter(c: char, upper: &str, lower: &str) -> char {
    let (table, base) = if c.is_ascii_uppercase() {
        (upper, 'A')
    } else {
        (lower, 'a')
    };
    table.chars().nth(c as usize - base as usize).unwrap_or(c)
}

/// Double ASCII vowels, then replace ASCII letters with accented forms.
pub fn transform_accented(s: &str) -> Cow<'_, str> {
    if !s.bytes().any(|b| b.is_ascii_alphabetic()) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if c.is_ascii_alphabetic() {
            let mapped = map_letter(c, ACCENT_UPPER, ACCENT_LOWER);
            out.push(mapped);
            if matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u') {
                out.push(mapped);
            }
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

/// Flip ASCII letters and wrap each run of ASCII letters in a right-to-left override.
pub fn transform_bidi(s: &str) -> Cow<'_, str> {
    if !s.bytes().any(|b| b.is_ascii_alphabetic()) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() * 2);
    let mut in_word = false;
    for c in s.chars() {
        let letter = c.is_ascii_alphabetic();
        if letter && !in_word {
            out.push(RLO);
        } else if !letter && in_word {
            out.push(PDF);
        }
        in_word = letter;
        out.push(if letter {
            map_letter(c, BIDI_UPPER, BIDI_LOWER)
        } else {
            c
        });
    }
    if in_word {
        out.push(PDF);
    }
    Cow::Owned(out)
}
