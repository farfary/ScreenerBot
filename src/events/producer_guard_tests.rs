//! Guard: an event payload's `message` is derived from its catalog `text`
//! (`crate::events::with_text`). A literal `message` key in Rust source is only
//! accepted in the files listed here, none of which produce event rows.

use std::fs;
use std::path::{Path, PathBuf};

/// Files that legitimately contain a literal `message` key, relative to `src/`.
const ALLOWED: &[&str] = &[
    // Tool and account responses, not event payloads.
    "agent_control/tools/config.rs",
    "agent_control/tools/system.rs",
    "services/health.rs",
    "account/client.rs",
    // The single writer of `message` for event payloads.
    "events/display_text.rs",
    // Owner-only fixture rows kept in the legacy shape.
    "webserver/promo/events.rs",
    // API error envelope derived from catalog text.
    "webserver/api_error.rs",
    // Reader tests for the events route response shape.
    "webserver/routes/events/types.rs",
    // JSON-RPC error fixture.
    "chains/solana/rpc/subscriptions/client.rs",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// A `message` key holding a value other than a nested object (Solana
/// transaction fixtures carry `"message": {`).
fn has_message_key(line: &str) -> bool {
    let needle = ["\"mess", "age\":"].concat();
    line.match_indices(&needle).any(|(at, _)| {
        !line[at + needle.len()..]
            .trim_start()
            .starts_with(['{', '}'])
    })
}

#[test]
fn only_allow_listed_files_write_a_literal_message_key() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(!files.is_empty(), "no sources found under {src:?}");

    let mut offenders = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&src)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if ALLOWED.contains(&relative.as_str()) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if has_message_key(line) {
                offenders.push(format!("src/{relative}:{}", index + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "literal message key outside the allow-list; use crate::events::with_text:\n{}",
        offenders.join("\n")
    );
}
