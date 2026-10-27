// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Source-scan guards that keep paper trading away from live execution: the
//! paper modules never name a swap, key, send, trade dispatch, live position or
//! live trade symbol, and never spawn a task (the runtime paper scope does not cross a
//! spawn). Every reader of the held mints goes through `positions::books`, so
//! the paper books are priced, charted and retained with the live one.
//!
//! Pure source-text scans: no network, no DB, no compilation.

use std::fs;
use std::path::{Path, PathBuf};

/// Paper modules, relative to `src/`. A directory covers every file below it.
const PAPER_MODULES: &[&str] = &[
    "trader/paper",
    "trader/manual/paper.rs",
    "positions/paper",
    "positions/paper.rs",
    "positions/database/paper.rs",
    "webserver/routes/positions/paper.rs",
];

/// Identifiers a paper module must never name.
const LIVE_IDENTIFIERS: &[&str] = &[
    // Swap execution.
    "execute_swap",
    "execute_swap_with_fallback",
    "execute_with_fallback_on",
    "execute_on",
    "execute_raw",
    "execute_raw_for_provider_kind",
    "submit_built_swap",
    "build_and_submit",
    // Key access and sending.
    "main_keypair",
    "keypair_for_wallet",
    "configured_keypair",
    "configured_keypair_async",
    "send_transaction",
    // Live position operations.
    "open_position_with_size",
    "add_to_position",
    "close_position_direct",
    "partial_close_position",
    // Live trade and slot machinery.
    "begin_trade",
    "acquire_global_position_permit",
    "release_position_slot",
    // Wallet and transition side effects.
    "request_balance_refresh",
    "process_position_loss_detection",
    // The trade dispatcher and the live executors.
    "execute_trade",
    "execute_buy",
    "execute_dca",
    "execute_sell",
    // Task spawning: the paper scope is task-local.
    "spawn",
    "spawn_blocking",
    "spawn_local",
];

/// Identifier prefixes a paper module must never name.
const LIVE_PREFIXES: &[&str] = &["sign_message"];

/// Paths a paper module must never name.
const LIVE_PATHS: &[&str] = &["positions::apply"];

/// Symbols a paper module may name although they are listed above. Shrink only.
///
/// The manual paper close hands a paper-book decision to the trade dispatcher, which
/// routes it to the paper executor by its book.
const ALLOWED: &[(&str, &str)] = &[("trader/manual/paper.rs", "execute_trade")];

fn rust_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path.to_path_buf());
        }
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        rust_files(&entry.path(), out);
    }
}

/// The code of a line without its trailing `//` comment. A `//` inside a string or char
/// literal, such as a URL, is code; a string that spans lines is read line by line.
fn code_of(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' if in_string => {
                at += 2;
                continue;
            }
            b'"' => in_string = !in_string,
            b'\'' if !in_string => {
                if bytes.get(at + 1).is_some_and(|&next| next != b'\\')
                    && bytes.get(at + 2) == Some(&b'\'')
                {
                    at += 3;
                    continue;
                }
                if bytes.get(at + 1) == Some(&b'\\') && bytes.get(at + 3) == Some(&b'\'') {
                    at += 4;
                    continue;
                }
            }
            b'/' if !in_string && bytes.get(at + 1) == Some(&b'/') => return &line[..at],
            _ => {}
        }
        at += 1;
    }
    line
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Every live symbol a source names in code, with its 1-based line number.
fn live_symbols(source: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let code = code_of(line);
        for ident in code.split(|c: char| !is_ident_char(c)) {
            if ident.is_empty() {
                continue;
            }
            if LIVE_IDENTIFIERS.contains(&ident)
                || LIVE_PREFIXES.iter().any(|prefix| ident.starts_with(prefix))
            {
                found.push((index + 1, ident.to_owned()));
            }
        }
        let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
        for path in LIVE_PATHS {
            let named = compact.match_indices(path).any(|(at, _)| {
                !compact[at + path.len()..]
                    .chars()
                    .next()
                    .is_some_and(is_ident_char)
            });
            if named {
                found.push((index + 1, (*path).to_owned()));
            }
        }
    }
    found
}

#[test]
fn live_symbol_scan_catches_a_planted_call() {
    let planted = [
        "let result = crate::swaps::execute_swap_with_fallback(token, quote).await;",
        "let keypair = main_keypair().await?;",
        "crate::chains::solana::accounts::sign_message_with_main_wallet(bytes);",
        "client.send_transaction(&tx).await",
        "crate::positions::apply(transition).await;",
        "tokio::spawn(async move { fill.await });",
        "let _guard = crate::global::begin_trade();",
        "executors::execute_trade(&decision, context).await",
        "let response = client.execute_raw(request).await?;",
        "let url = \"https://quote.example/v1\"; execute_swap(quote).await;",
        "let sep = '/'; let other = '/'; execute_swap(quote).await;",
    ];
    for line in planted {
        assert!(!live_symbols(line).is_empty(), "missed: {line}");
    }
    let clean = [
        "// execute_swap is never called from here",
        "let request = paper_sell_request(task, mint, held);",
        "crate::positions::applied_count();",
        "let executed_on = Utc::now();",
        "let url = \"https://quote.example/v1\"; // execute_swap is named in a comment",
        "let text = \"a \\\" quote\"; // execute_swap is named in a comment",
    ];
    for line in clean {
        assert!(live_symbols(line).is_empty(), "false positive: {line}");
    }
}

#[test]
fn paper_modules_name_no_live_execution_symbol() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    for module in PAPER_MODULES {
        rust_files(&src.join(module), &mut files);
    }
    assert!(
        files
            .iter()
            .any(|path| path.ends_with("trader/paper/fill.rs")),
        "the paper modules were not found under {src:?}"
    );
    let mut offenders = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&src)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let source = fs::read_to_string(&path).expect("read paper source");
        for (line, symbol) in live_symbols(&source) {
            if ALLOWED.contains(&(relative.as_str(), symbol.as_str())) {
                continue;
            }
            offenders.push(format!("src/{relative}:{line} names {symbol}"));
        }
    }
    assert!(
        offenders.is_empty(),
        "paper modules must not name live execution symbols:\n{}",
        offenders.join("\n")
    );
}

/// Files that may name the copy-trading paper holdings: their owner and its module face,
/// and the one reader that merges them into the held mints. Shrink only.
const HELD_PAPER_MINTS_NAMERS: &[&str] = &[
    "positions/books.rs",
    "trader/copy/mod.rs",
    "trader/copy/paper_exits.rs",
];

#[test]
fn held_mints_are_read_only_through_the_books() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let mut offenders = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&src)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let source = fs::read_to_string(&path).expect("read source");
        for (index, line) in source.lines().enumerate() {
            let code = code_of(line);
            let names = |symbol: &str| {
                code.split(|c: char| !is_ident_char(c))
                    .any(|ident| ident == symbol)
            };
            if names("get_open_mints") {
                offenders.push(format!("src/{relative}:{} names get_open_mints", index + 1));
            }
            if names("held_paper_mints") && !HELD_PAPER_MINTS_NAMERS.contains(&relative.as_str()) {
                offenders.push(format!(
                    "src/{relative}:{} names held_paper_mints",
                    index + 1
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "held mints are read through positions::books::held_mints:\n{}",
        offenders.join("\n")
    );
}
