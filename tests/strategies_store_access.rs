// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Source-scanning guards for the strategies store: entry and exit strategies are read once per trading cycle, evaluations reach SQLite only through the recorder, and the strategy routes reach the store only through its blocking-pool wrappers.

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

/// The enabled entry strategies are read by the entry cycle alone, on the blocking pool;
/// every token of the cycle takes them from `EntryCycle`, however many tokens the cycle holds.
#[test]
fn enabled_entry_strategies_are_read_once_per_entry_cycle() {
    assert_eq!(
        files_naming("get_enabled_strategies_async(StrategyType::Entry)"),
        ["trader/evaluators/entry.rs"]
    );
    assert!(
        files_naming("get_enabled_strategies(StrategyType::Entry)").is_empty(),
        "an entry path reads the strategies store on the runtime"
    );
}

/// The enabled exit strategies are read by the exit cycle alone, on the blocking pool;
/// every position of the cycle takes them from `ExitCycle`, however many positions it holds.
#[test]
fn enabled_exit_strategies_are_read_once_per_exit_cycle() {
    assert_eq!(
        files_naming("get_enabled_strategies_async(StrategyType::Exit)"),
        ["trader/evaluators/exit.rs"]
    );
    assert!(
        files_naming("get_enabled_strategies(StrategyType::Exit)").is_empty(),
        "an exit path reads the strategies store on the runtime"
    );
}

/// No trading path asks the store per token or per position whether a strategy is enabled.
#[test]
fn no_trading_path_counts_enabled_strategies_per_item() {
    assert!(files_naming("has_enabled_strategies").is_empty());
}

/// The synchronous strategies store operations, each named with its opening parenthesis
/// so the `_async` wrapper of the same name does not match.
const SYNCHRONOUS_STORE_CALLS: &[&str] = &[
    "get_strategy(",
    "get_all_strategies(",
    "get_enabled_strategies(",
    "insert_strategy(",
    "update_strategy(",
    "set_strategy_mode(",
    "delete_strategy(",
    "get_strategy_performance(",
];

/// Every strategy route, the overview and readiness reads behind them, the arming flow its
/// mode route calls, the entry cycle, the paper executor's intent check and the live buy's
/// strategy check reach the store through the `*_async` wrappers, so none holds a SQLite
/// connection on a runtime worker.
#[test]
fn strategy_routes_reach_the_store_only_on_the_blocking_pool() {
    let violations: Vec<String> = source_code()
        .into_iter()
        .filter(|(path, _)| {
            path.starts_with("webserver/routes/strategies")
                || path.ends_with("strategies/overview.rs")
                || path.ends_with("strategies/readiness.rs")
                || path.ends_with("strategies/arm.rs")
                || path.ends_with("trader/evaluators/entry.rs")
                || path.ends_with("trader/paper/executor/io.rs")
                || path.ends_with("trader/executors/buy.rs")
        })
        .flat_map(|(path, code)| {
            SYNCHRONOUS_STORE_CALLS
                .iter()
                .filter(|call| {
                    code.match_indices(*call).any(|(at, _)| {
                        !code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_')
                    })
                })
                .map(|call| format!("{} calls {call}", path.display()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        violations.is_empty(),
        "synchronous strategies store calls in async handlers: {violations:?}"
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
