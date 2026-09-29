use super::*;
use axum::http::{header::ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    input: String,
    accented: String,
    bidi: String,
}

fn cases() -> Vec<Case> {
    serde_json::from_str(include_str!("../../tools/tests/fixtures/i18n_pseudo.json"))
        .expect("pseudo fixture")
}

fn langid(tag: &str) -> LanguageIdentifier {
    tag.parse().expect("valid language tag")
}

const ACCENT_UPPER: &str = "ȦƁƇḒḖƑƓĦĪĴĶĿḾȠǾƤɊŘŞŦŬṼẆẊẎẐ";
const ACCENT_LOWER: &str = "ȧƀƈḓḗƒɠħīĵķŀḿƞǿƥɋřşŧŭṽẇẋẏẑ";
const BIDI_UPPER: &str = "∀ԐↃᗡƎℲ⅁HIſӼ⅂WNOԀÒᴚS⊥∩ɅMX⅄Z";
const BIDI_LOWER: &str = "ɐqɔpǝɟƃɥıɾʞʅɯuodbɹsʇnʌʍxʎz";

#[test]
fn tables_hold_26_scalars() {
    for table in [ACCENT_UPPER, ACCENT_LOWER, BIDI_UPPER, BIDI_LOWER] {
        assert_eq!(table.chars().count(), 26);
    }
}

#[test]
fn fixture_matches_both_transforms() {
    for case in cases() {
        assert_eq!(transform_accented(&case.input), case.accented);
        assert_eq!(transform_bidi(&case.input), case.bidi);
    }
}

#[test]
fn fixture_is_verified_independently() {
    let inverse: std::collections::HashMap<char, char> = BIDI_UPPER
        .chars()
        .zip('A'..='Z')
        .chain(BIDI_LOWER.chars().zip('a'..='z'))
        .collect();
    for case in cases() {
        // Bidi: dropping the wrappers and inverting the map restores the input.
        let restored: String = case
            .bidi
            .chars()
            .filter(|c| *c != '\u{202E}' && *c != '\u{202C}')
            .map(|c| inverse.get(&c).copied().unwrap_or(c))
            .collect();
        assert_eq!(restored, case.input, "bidi round trip");
        assert_eq!(
            case.bidi.matches('\u{202E}').count(),
            case.bidi.matches('\u{202C}').count()
        );
        // Accented: no ASCII letter survives and non-letters are untouched.
        assert!(!case.accented.chars().any(|c| c.is_ascii_alphabetic()));
        let kept = |s: &str| -> String { s.chars().filter(|c| !c.is_alphabetic()).collect() };
        assert_eq!(kept(&case.accented), kept(&case.input));
        // Unchanged input borrows.
        if !case.input.bytes().any(|b| b.is_ascii_alphabetic()) {
            assert!(matches!(
                transform_accented(&case.input),
                std::borrow::Cow::Borrowed(_)
            ));
            assert!(matches!(
                transform_bidi(&case.input),
                std::borrow::Cow::Borrowed(_)
            ));
        }
    }
}

/// Text of the first line of a message, with placeables removed.
fn literal_text(source: &str, id: &str) -> Option<String> {
    let line = source.lines().find_map(|l| {
        l.strip_prefix(id)
            .and_then(|rest| rest.trim_start().strip_prefix('='))
    })?;
    let mut depth = 0;
    let mut out = String::new();
    for c in line.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    Some(out)
}

#[test]
fn every_en_message_formats_under_pseudo_locales() {
    for pseudo in PSEUDO_LOCALES {
        let locale = langid(pseudo.code());
        let mut checked = 0;
        for file in CATALOGS.iter().filter(|f| f.locale == "en") {
            for line in file.source.lines() {
                let Some((id, _)) = line.split_once('=') else {
                    continue;
                };
                let id = id.trim();
                if id.is_empty()
                    || id.starts_with(['#', '-', '.', '*', '['])
                    || id.contains(char::is_whitespace)
                {
                    continue;
                }
                let plain = format_message(&langid("en"), id, None).expect("en message");
                let pseudo_msg = format_message(&locale, id, None).expect("pseudo message");
                checked += 1;
                let has_letters = literal_text(file.source, id)
                    .is_some_and(|t| t.bytes().any(|b| b.is_ascii_alphabetic()));
                if has_letters && plain.value.is_some() {
                    assert_ne!(
                        pseudo_msg.value,
                        plain.value,
                        "{id} under {}",
                        pseudo.code()
                    );
                }
            }
        }
        assert!(checked > 0);
    }
}

#[test]
fn pseudo_bundle_transforms_source_text() {
    assert_eq!(
        format(&langid("en-XA"), "common-loading", None),
        transform_accented("Loading…")
    );
    assert_eq!(
        format(&langid("ar-XB"), "common-loading", None),
        transform_bidi("Loading…")
    );
}

#[test]
fn explicit_pseudo_setting_is_honoured() {
    assert_eq!(resolve_locale("en-XA", None), langid("en-XA"));
    assert_eq!(resolve_locale("ar-XB", Some("en")), langid("ar-XB"));
}

#[test]
fn accept_language_never_selects_a_pseudo_locale() {
    for tag in ["en-XA", "ar-XB"] {
        let resolved = resolve_locale("system", Some(tag));
        assert_ne!(resolved, langid(tag));
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_str(tag).unwrap());
        assert_ne!(resolve_request_locale(&headers), langid(tag));
    }
}

#[test]
fn pseudo_locales_are_not_offered() {
    for pseudo in PSEUDO_LOCALES {
        assert!(!available_locales().iter().any(|l| l.code == pseudo.code()));
        assert!(locale_info(pseudo.code()).is_none());
        let info = display_locale_info(pseudo.code()).expect("display info");
        assert_eq!(info.dir, pseudo.dir());
    }
    assert_eq!(text_direction(&langid("ar-XB")), TextDirection::Rtl);
    assert_eq!(text_direction(&langid("en-XA")), TextDirection::Ltr);
    assert_eq!(PseudoLocale::Bidi.name(), "Pseudo (bidi)");
    assert_eq!(PseudoLocale::Accented.name(), "Pseudo (accented)");
}

#[test]
fn pseudo_catalog_chain_is_source_only() {
    let chain = dashboard_catalog_chain(&langid("ar-XB"));
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].0, "en");
}
