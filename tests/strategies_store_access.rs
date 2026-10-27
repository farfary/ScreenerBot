// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Source-scanning guards for the strategies store on the trading hot paths: entry strategies are read once per entry cycle, and evaluations reach SQLite only through the recorder.

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.rs` file under `src/`, as `(relative_path, code_lines)` with doc and line
/// comments removed, so prose naming a function is not a call.
fn source_code() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read_dir(src) must succeed") {
            let path = entry.expect("dir entry must be readable").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let code = fs::read_to_string(&path)
                    .expect("read .rs file")
                    .lines()
                    .filter(|line| !line.trim_start().starts_with("//"))
                    .collect::<Vec<_>>()
                    .join("\n");
                out.push((path.strip_prefix(&root).unwrap().to_path_buf(), code));
            }
        }
    }
    out
}

/// The files whose code contains `needle`, sorted.
fn files_naming(needle: &str) -> Vec<String> {
    let mut files: Vec<String> = source_code()
        .into_iter()
        .filter(|(_, code)| code.contains(needle))
        .map(|(path, _)| path.to_string_lossy().replace('\\', "/"))
        .collect();
    files.sort();
    files
}

/// The enabled entry strategies are read by the entry cycle alone; every token of the
/// cycle takes them from `EntryCycle`, however many tokens the cycle holds.
#[test]
fn enabled_entry_strategies_are_read_once_per_entry_cycle() {
    assert_eq!(
        files_naming("get_enabled_strategies(StrategyType::Entry)"),
        ["trader/evaluators/entry.rs"]
    );
    assert!(
        files_naming("has_enabled_strategies(StrategyType::Entry").is_empty(),
        "a per-token entry check reads the strategies store"
    );
}

/// Strategy evaluations are written by the recorder's writer on the blocking pool, never
/// by an evaluation on the async runtime.
#[test]
fn strategy_evaluations_reach_sqlite_only_through_the_recorder() {
    assert_eq!(
        files_naming("record_evaluations"),
        ["strategies/database.rs", "strategies/recorder.rs"]
    );
}
