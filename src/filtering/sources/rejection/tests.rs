use super::*;
use crate::i18n::source_message_ids;

fn has_message(id: &str) -> bool {
    source_message_ids().binary_search(&id).is_ok()
}

fn llm() -> FilterRejectionReason {
    FilterRejectionReason::LlmAnalysisRejected {
        reason: "unverifiable team".to_owned(),
        confidence: 72,
        provider: "anthropic".to_owned(),
    }
}

#[test]
fn every_variant_has_a_catalog_message() {
    let mut all: Vec<FilterRejectionReason> = FilterRejectionReason::UNIT_VARIANTS.to_vec();
    all.push(llm());
    assert_eq!(all.len(), 81);
    for reason in &all {
        let id = reason.ui_text().id;
        assert!(
            has_message(&id),
            "{} has no catalog message {id}",
            reason.label()
        );
    }
}

#[test]
fn stored_codes_round_trip_to_the_same_message() {
    let mut seen = std::collections::HashSet::new();
    for reason in FilterRejectionReason::UNIT_VARIANTS {
        let code = reason.label();
        assert!(seen.insert(code.clone()), "duplicate code {code}");
        assert_eq!(rejection_text(&code), reason.ui_text(), "{code}");
        assert_eq!(
            FilterRejectionReason::from_code(&code).as_ref(),
            Some(reason)
        );
    }
}

#[test]
fn llm_code_uses_the_no_argument_message() {
    let text = rejection_text(&llm().label());
    assert_eq!(
        text.id,
        ids::FILTERING_REJECT_LLM_ANALYSIS_REJECTED_GENERIC.as_str()
    );
    assert!(text.args.is_empty());
    assert!(has_message(&text.id));
}

#[test]
fn llm_reason_carries_typed_arguments() {
    let text = llm().ui_text();
    assert_eq!(
        text.id,
        ids::FILTERING_REJECT_LLM_ANALYSIS_REJECTED.as_str()
    );
    assert_eq!(
        text.args["reason"],
        UiArg::Text("unverifiable team".to_owned())
    );
    assert_eq!(text.args["confidence"], UiArg::Count(72));
    assert_eq!(text.args["provider"], UiArg::Text("anthropic".to_owned()));
    let english: unic_langid::LanguageIdentifier = "en".parse().unwrap();
    assert_eq!(
        text.render_plain(&english),
        "LLM Analysis Rejected: unverifiable team (72% conf, anthropic)"
    );
}

#[test]
fn unknown_code_renders_as_the_code() {
    let text = rejection_text("retired_code");
    assert_eq!(text.id, ids::FILTERING_REJECT_UNKNOWN.as_str());
    assert_eq!(text.args["code"], UiArg::Text("retired_code".to_owned()));
    assert!(has_message(&text.id));
}

#[test]
fn legacy_codes_resolve_and_do_not_shadow_live_codes() {
    assert!(!LEGACY_REJECTION_CODES.is_empty());
    for (code, id) in LEGACY_REJECTION_CODES {
        assert!(has_message(id.as_str()), "{code} has no catalog message");
        assert_eq!(rejection_text(code).id, id.as_str(), "{code}");
        assert!(rejection_text(code).args.is_empty());
        assert!(
            FilterRejectionReason::from_code(code).is_none() && *code != LLM_CODE,
            "{code} collides with a live code"
        );
    }
}
