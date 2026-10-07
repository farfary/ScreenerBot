// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Source-scanning guards for the chain-ownership boundary.
//!
//! Shared domain modules (everything outside `src/chains/solana/`) define
//! chain-neutral intent/models/contracts; `src/chains/solana` implements
//! concrete Solana mechanics; app/service composition selects it through
//! `ChainId` or the router/discovery registries. These tests fail fast if
//! that boundary regresses — e.g. a new file bypassing the
//! `crate::chains::solana` vendor façade, an alloy crate named outside the
//! `crate::chains::evm` façade, or a shared module re-exporting a
//! chain-specific type as its own public API.
//!
//! Pure source-text scans: no network, no DB, no compilation. The venue-coverage guard also
//! reads the recorded case tree under `tests/fixtures/pools/` and names `ProgramKind`.

use screenerbot::chains::solana::pools::types::ProgramKind;
use std::fs;
use std::path::{Path, PathBuf};

/// Walks `src/`, yielding `(relative_path, file_contents)` for every `.rs` file.
fn walk_src() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read_dir(src) must succeed") {
            let entry = entry.expect("dir entry must be readable");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let contents = fs::read_to_string(&path).expect("read .rs file");
                let relative = path.strip_prefix(&root).unwrap().to_path_buf();
                out.push((relative, contents));
            }
        }
    }
    out
}

fn is_solana_owned(relative: &Path) -> bool {
    relative.starts_with("chains/solana")
}

/// Strips doc-comment lines (`//!`, `///`), which are free to name Solana
/// concepts in prose when explaining the chain boundary — only code lines
/// are a real import/reference.
fn code_lines(contents: &str) -> String {
    contents
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Vendor crates that must be reached through `crate::chains::solana` (the
/// single façade declared in `src/chains/solana/mod.rs`), never imported raw.
const VENDOR_CRATES: &[&str] = &[
    "solana_sdk",
    "solana_client",
    "solana_packet",
    "solana_program",
    "solana_transaction_status",
    "solana_account_decoder",
    "spl_token_2022",
    "spl_associated_token_account",
    "spl_token",
    "bs58",
    "borsh",
];

#[test]
fn shared_modules_never_import_solana_vendor_crates_raw() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_solana_owned(&relative) {
            continue; // the façade itself is allowed to name the vendor crates.
        }
        for line in contents.lines() {
            let trimmed = line.trim_start();
            let Some(rest) = trimmed.strip_prefix("use ") else {
                continue;
            };
            for crate_name in VENDOR_CRATES {
                let raw_prefix = format!("{crate_name}::");
                let raw_bare = format!("{crate_name};");
                if rest.starts_with(&raw_prefix) || rest.starts_with(&raw_bare) {
                    violations.push(format!(
                        "src/{}: raw `use {crate_name}` — import via `crate::chains::solana::{crate_name}` instead",
                        relative.display()
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "shared (non-Solana-owned) modules must reach Solana vendor crates through the \
         crate::chains::solana façade, never import them raw:\n{}",
        violations.join("\n")
    );
}

/// Line numbers in `source` whose code (comments stripped) names an EVM vendor
/// crate: the `alloy` meta-crate or an `alloy_*` sub-crate, as a raw import, a
/// fully-qualified path or a path through the `chains::evm` façade.
fn evm_vendor_crate_lines(source: &str) -> Vec<usize> {
    let pattern = regex::Regex::new(r"\balloy(?:_\w+)?\b").expect("the vendor pattern is valid");
    strip_comment_text(source)
        .lines()
        .enumerate()
        .filter(|(_, line)| pattern.is_match(line))
        .map(|(idx, _)| idx + 1)
        .collect()
}

#[test]
fn evm_vendor_matcher_names_crates_only() {
    let source = "\
use alloy::primitives::Address;
use crate::chains::evm::alloy::primitives::U256;
fn decode() -> alloy_sol_types::Result<()> { todo!() }
extern crate alloy;
// alloy named in a comment
/* alloy_primitives in a block comment */
let alloyed = 1;
fn non_alloy_helper() {}
";
    assert_eq!(evm_vendor_crate_lines(source), vec![1, 2, 3, 4]);
}

/// The alloy crates are reached only inside `src/chains/evm`, the EVM vendor
/// façade. Unlike the Solana façade, no path through it is open to shared
/// code either: an EVM value crosses into neutral modules as a neutral type
/// (`RawAmount`, `AssetId`, `AccountId`), never as an alloy type. Test code is
/// held to the same rule.
#[test]
fn evm_vendor_crates_only_in_chains_evm() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if relative.starts_with("chains/evm") {
            continue;
        }
        for line in evm_vendor_crate_lines(&contents) {
            violations.push(format!("src/{}:{line}", relative.display()));
        }
    }
    assert!(
        violations.is_empty(),
        "alloy crates may be named only under src/chains/evm:\n{}",
        violations.join("\n")
    );
}

/// DEX/aggregator program-ID literals owned by `chains/solana/constants.rs`.
/// Kept in sync manually with that file — this is a small, explicit
/// allowlist of exact base58 strings, not a pattern match, so it only ever
/// fires on a genuine reintroduced duplicate.
const OWNED_PROGRAM_ID_LITERALS: &[(&str, &str)] = &[
    (
        "METAPLEX_PROGRAM_ID",
        "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
    ),
    ("SYSTEM_PROGRAM_ID", "11111111111111111111111111111111"),
    (
        "SPL_TOKEN_PROGRAM_ID",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    ),
    (
        "TOKEN_2022_PROGRAM_ID",
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
    ),
    (
        "ASSOCIATED_TOKEN_PROGRAM_ID",
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
    ),
    (
        "MEMO_PROGRAM_ID",
        "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr",
    ),
    (
        "JUPITER_V6_PROGRAM_ID",
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",
    ),
    (
        "JUPITER_V4_PROGRAM_ID",
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB",
    ),
    (
        "RAYDIUM_LEGACY_AMM_PROGRAM_ID",
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
    ),
    (
        "RAYDIUM_CPMM_PROGRAM_ID",
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
    ),
    (
        "RAYDIUM_CLMM_PROGRAM_ID",
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
    ),
    (
        "ORCA_WHIRLPOOL_PROGRAM_ID",
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
    ),
    (
        "METEORA_DAMM_PROGRAM_ID",
        "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG",
    ),
    (
        "METEORA_DLMM_PROGRAM_ID",
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
    ),
    (
        "METEORA_DBC_PROGRAM_ID",
        "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN",
    ),
    (
        "PUMP_FUN_AMM_PROGRAM_ID",
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
    ),
    (
        "PUMP_FUN_LEGACY_PROGRAM_ID",
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
    ),
    (
        "MOONIT_AMM_PROGRAM_ID",
        "MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG",
    ),
    (
        "FLUXBEAM_AMM_PROGRAM_ID",
        "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X",
    ),
];

/// The one file allowed to define each literal, plus its re-export site.
const PROGRAM_ID_OWNER: &str = "chains/solana/constants.rs";
const PROGRAM_ID_REEXPORTER: &str = "chains/solana/transactions/program_ids.rs";

/// True if `line` is a `const NAME: &str = "<literal>";` definition (any
/// visibility) — not merely a line that happens to mention the literal
/// (e.g. matching it against transaction program IDs, or a placeholder
/// all-ones address used for an unrelated purpose).
fn defines_const_literal(line: &str, literal: &str) -> bool {
    let trimmed = line.trim_start();
    (trimmed.starts_with("const ") || trimmed.starts_with("pub const "))
        && trimmed.contains(": &str")
        && trimmed.trim_end().ends_with(&format!("\"{literal}\";"))
}

#[test]
fn dex_program_id_literals_have_exactly_one_owner() {
    let files = walk_src();
    let mut violations = Vec::new();
    for (name, literal) in OWNED_PROGRAM_ID_LITERALS {
        for (relative, contents) in &files {
            let path_str = relative.to_string_lossy();
            if path_str == PROGRAM_ID_OWNER || path_str == PROGRAM_ID_REEXPORTER {
                continue;
            }
            if contents
                .lines()
                .any(|line| defines_const_literal(line, literal))
            {
                violations.push(format!(
                    "src/{path_str}: redefines {name} ({literal}) instead of importing it \
                     from crate::chains::solana::constants"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "DEX/aggregator program IDs must have exactly one literal `const` definition, in \
         crate::chains::solana::constants:\n{}",
        violations.join("\n")
    );
}

/// Chain-specific discovery/fetcher types must not leak into the shared
/// `crate::pools` public surface — callers that need them import
/// `crate::chains::solana::pools` directly (regression guard, see
/// `src/pools/mod.rs` doc comment).
#[test]
fn shared_pools_module_does_not_reexport_solana_discovery_types() {
    let pools_mod =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pools/mod.rs"))
            .expect("src/pools/mod.rs must exist");

    for banned_reexport in [
        "pub use crate::chains::solana::pools::discovery",
        "pub use crate::chains::solana::pools::fetcher::AccountData",
    ] {
        assert!(
            !pools_mod.contains(banned_reexport),
            "src/pools/mod.rs must not re-export Solana-specific discovery/fetcher types \
             (`{banned_reexport}`) — callers should import crate::chains::solana::pools directly"
        );
    }
}

/// `PoolDescriptor` and the rest of the shared pool domain
/// (`src/pools/types.rs`, `cache.rs`, `api.rs`, `database/`) must
/// stay chain-neutral: no `Pubkey`, no `crate::chains::solana::pools::types`
/// (the Solana `ProgramKind` enum), and no vendor-crate façade for Solana
/// address types. This is a regression guard for the leak fixed by moving
/// `PoolDescriptor` to typed `PoolId`/`AssetId`/`AccountId`/`ProtocolId`
/// identities and relocating the `Pubkey`-driven pool price calculator to
/// `crate::chains::solana::pools::calculator`.
#[test]
fn shared_pools_domain_never_names_a_solana_address_type() {
    let banned_needles = [
        "Pubkey",
        "solana_sdk",
        "chains::solana::pools::types::ProgramKind",
        "chains::solana::solana_sdk",
    ];

    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path_str = relative.to_string_lossy();
        if !path_str.starts_with("pools/") {
            continue;
        }
        // Only scan code lines — doc comments (`//!`, `///`) are free to name
        // Solana concepts in prose when explaining the chain boundary.
        let code_only: String = contents
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                !trimmed.starts_with("//!") && !trimmed.starts_with("///")
            })
            .collect::<Vec<_>>()
            .join("\n");
        for needle in banned_needles {
            if code_only.contains(needle) {
                violations.push(format!("src/{path_str}: names `{needle}`"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "the shared pool domain (src/pools/) must stay chain-neutral — Solana address \
         types belong under src/chains/solana/pools instead, with PoolDescriptor's \
         PoolId/AssetId/AccountId/ProtocolId converted at that boundary:\n{}",
        violations.join("\n")
    );
}

/// Wallet ownership boundary: shared wallet records, manager APIs,
/// configuration and multi-wallet tooling must never hold a decrypted
/// `Keypair` or a `Pubkey` — only `crate::chains::solana::accounts` (and the
/// concrete swap/asset executors it hands a resolved keypair to) may. Shared
/// code passes a `wallet_id`, an address string, or relies on "the main
/// wallet"; it gets back signatures and addresses, never key material.
///
/// Scoped to the exact files audited when this boundary was introduced, not
/// a whole-directory ban: `src/wallets/watch/**` still parses a `Pubkey`
/// inline for the observation pipeline's own use (RPC subscriptions,
/// Solana subject conversion) and is out of scope. `balance_ops.rs`/
/// `balance_queries.rs` are IN scope — their RPC balance reads were moved
/// behind `crate::chains::solana::accounts::{fetch_wallet_sol_balance,
/// fetch_wallet_token_balances}`.
#[test]
fn wallet_ownership_never_names_a_solana_key_type() {
    const SCOPED_FILES: &[&str] = &[
        "wallets/types.rs",
        "wallets/mod.rs",
        "wallets/manager.rs",
        "wallets/manager/access.rs",
        "wallets/manager/cache.rs",
        "wallets/manager/main_wallet.rs",
        "wallets/manager/tools.rs",
        "wallets/manager/crud.rs",
        "wallets/manager/bulk_ops.rs",
        "wallets/manager/migration.rs",
        "wallets/manager/balance_ops.rs",
        "wallets/manager/balance_queries.rs",
        "config/wallet.rs",
        "tools/swap_executor.rs",
        "tools/multi_wallet/buy.rs",
        "tools/multi_wallet/sell.rs",
        "tools/multi_wallet/consolidate.rs",
        "tools/multi_wallet/transfer.rs",
    ];
    let banned_needles = ["Keypair", "Pubkey", "solana_sdk"];

    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path_str = relative.to_string_lossy();
        if !SCOPED_FILES.contains(&path_str.as_ref()) {
            continue;
        }
        // Only scan code lines — doc comments are free to name Solana
        // concepts in prose when explaining the chain boundary.
        let code_only: String = contents
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                !trimmed.starts_with("//!") && !trimmed.starts_with("///")
            })
            .collect::<Vec<_>>()
            .join("\n");
        for needle in banned_needles {
            if code_only.contains(needle) {
                violations.push(format!("src/{path_str}: names `{needle}`"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "shared wallet ownership code must never hold a decrypted Keypair or a Pubkey — \
         resolve/sign through crate::chains::solana::accounts (by wallet_id or \"the main \
         wallet\") instead:\n{}",
        violations.join("\n")
    );
}

/// Pool runtime composition boundary: `src/pools/service.rs` (the
/// chain-neutral supervisor: running flag, event recording, db/cache init)
/// must never import the concrete Solana pool runtime — it reaches each
/// chain's components through `ChainRuntime::pricing_driver`. Regression
/// guard for the leak fixed by moving `PoolAnalyzer`/`PoolDiscovery`/
/// `AccountFetcher`/`PriceCalculator` management to
/// `crate::chains::solana::pools::service`.
#[test]
fn shared_pools_service_never_imports_solana_runtime() {
    let path = "pools/service.rs";
    let contents = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(path))
        .expect("src/pools/service.rs must exist");

    assert!(
        !code_lines(&contents).contains("chains::solana"),
        "src/{path} must not import crate::chains::solana — it orchestrates lifecycle \
         generically and reaches the concrete runtime through its pricing driver"
    );
}

/// Directories whose `.rs` files may not read account bytes at a literal offset.
const ACCOUNT_OFFSET_SCANNED_DIRS: &[&str] = &[
    "chains/solana/pools/decoders",
    "chains/solana/swaps/direct/venues",
];

/// Single files scanned beside [`ACCOUNT_OFFSET_SCANNED_DIRS`].
const ACCOUNT_OFFSET_SCANNED_FILES: &[&str] = &[
    "chains/solana/pools/analyzer_extractors.rs",
    "chains/solana/pools/reserve_accounts.rs",
];

/// Files that still read pool account bytes at a literal offset, with the exact number of reads.
/// Each entry leaves when its reads move into the venue's `layouts/<venue>.rs`; the list only
/// shrinks and ends empty. A file over its count, an unlisted file with any count, and a file
/// under its count all fail, so the list shrinks in the same change that removes a read.
const ACCOUNT_OFFSET_READS: &[(&str, usize)] = &[
    ("chains/solana/pools/decoders/fluxbeam_amm.rs", 1),
    ("chains/solana/pools/decoders/meteora_damm.rs", 2),
    ("chains/solana/pools/decoders/meteora_dbc.rs", 1),
    ("chains/solana/pools/decoders/meteora_dlmm.rs", 2),
    ("chains/solana/pools/decoders/moonit_amm.rs", 1),
    ("chains/solana/pools/decoders/orca_whirlpool.rs", 1),
    ("chains/solana/pools/decoders/pumpfun_amm.rs", 4),
    ("chains/solana/pools/decoders/pumpfun_legacy.rs", 6),
    ("chains/solana/pools/decoders/raydium_clmm.rs", 1),
    ("chains/solana/pools/decoders/raydium_cpmm.rs", 31),
    ("chains/solana/pools/decoders/raydium_legacy_amm.rs", 1),
    ("chains/solana/swaps/direct/venues/clmm_ticks.rs", 1),
    ("chains/solana/swaps/direct/venues/meteora_dlmm.rs", 1),
    ("chains/solana/swaps/direct/venues/pumpfun_amm.rs", 2),
    ("chains/solana/swaps/direct/venues/pumpfun_legacy.rs", 1),
];

/// Reads of account bytes at a literal offset in the production text of `source`: a typed
/// `*_at(data, <number>` read, a `read_*_at_offset(` call, or a `[<number>..` slice.
fn account_offset_reads(source: &str) -> usize {
    let pattern = regex::Regex::new(
        r"\b(?:pubkey|u8|u16|u32|u64|u128|i32|i64|i128)_at\(\s*[^,]+,\s*\d+|read_\w+_at_offset\(|\[\s*\d+\s*\.\.",
    )
    .expect("the offset pattern is valid");
    pattern
        .find_iter(&strip_comment_text(&production_text(source)))
        .count()
}

#[test]
fn account_offset_matcher_counts_literal_offsets_only() {
    let source = "\
fn read(data: &[u8]) {
    let a = u64_at(data, 8);
    let b = pubkey_at(
        data,
        32,
    );
    let c = read_u32_at_offset(data, 4);
    let d = &data[8..40];
    let e = &data[ 0 ..4];
    // let hidden = u64_at(data, 12);
    let named = u64_at(data, MINT_OFFSET);
    let ranged = &data[start..end];
}
#[cfg(test)]
mod tests {
    fn fixture(data: &[u8]) -> u64 {
        u64_at(data, 16)
    }
}
";
    assert_eq!(account_offset_reads(source), 5);
}

/// Pool account offsets, discriminators and sizes are owned by `pools/layouts/<venue>.rs`.
/// Decoders, direct-swap venues and the analyzer call the layout and never re-derive an
/// offset, so a layout fixed in one place is fixed for pricing, swapping and discovery.
#[test]
fn pool_account_offsets_are_read_only_by_the_layouts() {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for (relative, contents) in walk_src() {
        let scanned = ACCOUNT_OFFSET_SCANNED_FILES
            .iter()
            .any(|file| relative == Path::new(file))
            || ACCOUNT_OFFSET_SCANNED_DIRS
                .iter()
                .any(|dir| relative.parent() == Some(Path::new(dir)));
        if !scanned {
            continue;
        }
        let count = account_offset_reads(&contents);
        if count > 0 {
            counts.push((relative.to_string_lossy().into_owned(), count));
        }
    }
    let allowed = |path: &str| {
        ACCOUNT_OFFSET_READS
            .iter()
            .find(|(file, _)| *file == path)
            .map(|(_, count)| *count)
    };
    let mut problems = Vec::new();
    for (path, count) in &counts {
        match allowed(path) {
            None => problems.push(format!(
                "src/{path}: {count} literal-offset reads, not on the allowlist"
            )),
            Some(listed) if listed != *count => problems.push(format!(
                "src/{path}: {count} literal-offset reads, the allowlist says {listed}"
            )),
            Some(_) => {}
        }
    }
    for (file, listed) in ACCOUNT_OFFSET_READS {
        if !counts.iter().any(|(path, _)| path == file) {
            problems.push(format!(
                "src/{file}: allowlisted for {listed} reads but has none, or is not scanned"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "account bytes are read at a literal offset only inside pools/layouts/<venue>.rs; move \
         the read into the layout, and lower or remove the allowlist entry in the same change:\n{}",
        problems.join("\n")
    );
}

/// The slug of a program kind that has a decoder, which is every kind except `Unknown`. The
/// match is exhaustive so a new kind fails to compile here until it is classified.
fn priced_slug(kind: ProgramKind) -> Option<&'static str> {
    match kind {
        ProgramKind::RaydiumCpmm
        | ProgramKind::RaydiumLegacyAmm
        | ProgramKind::RaydiumClmm
        | ProgramKind::OrcaWhirlpool
        | ProgramKind::MeteoraDamm
        | ProgramKind::MeteoraDlmm
        | ProgramKind::MeteoraDbc
        | ProgramKind::PumpFunAmm
        | ProgramKind::PumpFunLegacy
        | ProgramKind::Moonit
        | ProgramKind::FluxbeamAmm => Some(kind.protocol_slug()),
        ProgramKind::Unknown => None,
    }
}

const ALL_PROGRAM_KINDS: [ProgramKind; 12] = [
    ProgramKind::RaydiumCpmm,
    ProgramKind::RaydiumLegacyAmm,
    ProgramKind::RaydiumClmm,
    ProgramKind::OrcaWhirlpool,
    ProgramKind::MeteoraDamm,
    ProgramKind::MeteoraDlmm,
    ProgramKind::MeteoraDbc,
    ProgramKind::PumpFunAmm,
    ProgramKind::PumpFunLegacy,
    ProgramKind::Moonit,
    ProgramKind::FluxbeamAmm,
    ProgramKind::Unknown,
];

/// The cells of the venue coverage matrix a venue has not filled yet, as `(slug, cells)`. A
/// missing cell that is not listed fails, and a listed cell that is now filled fails, so the list
/// only shrinks and ends empty. Cells: `spec` and `spec-source` (the vendored layout spec and its
/// provenance), `case` (a recorded case), `program-truth` and `observations` (a case carrying the
/// program's own simulated output, or independent market data), `test-module`
/// (`tests/solana_pools/<slug>.rs`).
const MISSING_VENUE_CELLS: &[(&str, &[&str])] = &[
    ("fluxbeam_amm", TRUTH_AND_TESTS_PENDING),
    ("meteora_damm_v2", TRUTH_AND_TESTS_PENDING),
    ("meteora_dbc", TRUTH_AND_TESTS_PENDING),
    ("meteora_dlmm", TRUTH_AND_TESTS_PENDING),
    ("moonit_amm", TRUTH_AND_TESTS_PENDING),
    ("orca_whirlpool", TRUTH_AND_TESTS_PENDING),
    ("pumpfun_amm", TRUTH_AND_TESTS_PENDING),
    ("pumpfun_legacy", TRUTH_AND_TESTS_PENDING),
    ("raydium_clmm", TRUTH_AND_TESTS_PENDING),
    ("raydium_cpmm", TRUTH_AND_TESTS_PENDING),
    ("raydium_legacy_amm", TRUTH_AND_TESTS_PENDING),
];

/// The cells that follow the spec: no venue has program truth, market
/// observations or a test module of its own yet.
const TRUTH_AND_TESTS_PENDING: &[&str] = &["program-truth", "observations", "test-module"];

/// The cells of `slug` that are empty, given the case tree `fixtures` and the suite directory
/// `suite` (`tests/solana_pools`).
fn missing_venue_cells(fixtures: &Path, suite: &Path, slug: &str) -> Vec<&'static str> {
    let venue = fixtures.join(slug);
    let mut cases = Vec::new();
    if let Ok(entries) = fs::read_dir(&venue) {
        for entry in entries {
            let path = entry.expect("venue directory entry").path();
            let stem = path.file_stem().and_then(|stem| stem.to_str());
            if path.extension().is_some_and(|ext| ext == "json")
                && stem != Some("spec")
                && stem != Some("spec-source")
            {
                let text = fs::read_to_string(&path).expect("read case");
                let case: serde_json::Value = serde_json::from_str(&text)
                    .unwrap_or_else(|e| panic!("parse case {}: {e}", path.display()));
                cases.push(case);
            }
        }
    }
    let carries = |key: &str| {
        cases.iter().any(|case| {
            case.get(key)
                .and_then(|value| value.as_array())
                .is_some_and(|entries| !entries.is_empty())
        })
    };
    let mut missing = Vec::new();
    if !venue.join("spec.json").is_file() {
        missing.push("spec");
    }
    if !venue.join("spec-source.json").is_file() {
        missing.push("spec-source");
    }
    if cases.is_empty() {
        missing.push("case");
    }
    if !carries("program_truth") {
        missing.push("program-truth");
    }
    if !carries("observations") {
        missing.push("observations");
    }
    if !suite.join(format!("{slug}.rs")).is_file() {
        missing.push("test-module");
    }
    missing
}

#[test]
fn venue_cell_detection_reads_the_case_tree() {
    let root = tempfile::tempdir().expect("temp dir");
    let fixtures = root.path().join("fixtures");
    let suite = root.path().join("suite");
    fs::create_dir_all(fixtures.join("venue_a")).expect("venue directory");
    fs::create_dir_all(&suite).expect("suite directory");
    fs::write(fixtures.join("venue_a/spec.json"), "{}").expect("spec");
    fs::write(
        fixtures.join("venue_a/case.json"),
        r#"{"program_truth": [], "observations": [{"source": "jupiter"}]}"#,
    )
    .expect("case");
    fs::write(suite.join("venue_a.rs"), "").expect("test module");
    assert_eq!(
        missing_venue_cells(&fixtures, &suite, "venue_a"),
        ["spec-source", "program-truth"]
    );
    assert_eq!(
        missing_venue_cells(&fixtures, &suite, "venue_b"),
        [
            "spec",
            "spec-source",
            "case",
            "program-truth",
            "observations",
            "test-module"
        ]
    );
}

/// Every venue that prices pools has a vendored spec, recorded cases that carry program truth
/// and market observations, and a test module. Venues fill their cells over time; the missing
/// ones are listed in [`MISSING_VENUE_CELLS`], which only shrinks.
#[test]
fn every_priced_program_kind_has_a_spec_cases_and_tests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let fixtures = root.join("fixtures/pools/solana");
    let suite = root.join("solana_pools");
    let mut problems = Vec::new();
    let mut priced: Vec<&str> = Vec::new();
    for kind in ALL_PROGRAM_KINDS {
        let Some(slug) = priced_slug(kind) else {
            continue;
        };
        priced.push(slug);
        let listed: &[&str] = MISSING_VENUE_CELLS
            .iter()
            .find(|(listed_slug, _)| *listed_slug == slug)
            .map_or(&[], |(_, cells)| cells);
        let missing = missing_venue_cells(&fixtures, &suite, slug);
        for cell in &missing {
            if !listed.contains(cell) {
                problems.push(format!(
                    "{slug}: {cell} is missing and not on the allowlist"
                ));
            }
        }
        for cell in listed {
            if !missing.contains(cell) {
                problems.push(format!(
                    "{slug}: {cell} is filled, remove it from MISSING_VENUE_CELLS"
                ));
            }
        }
    }
    for (slug, _) in MISSING_VENUE_CELLS {
        if !priced.contains(slug) {
            problems.push(format!("{slug}: allowlisted but not a priced program kind"));
        }
    }
    for entry in fs::read_dir(&fixtures).expect("read the solana case tree") {
        let path = entry.expect("case tree entry").path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("venue directory name is UTF-8");
            if !priced.contains(&name) {
                problems.push(format!(
                    "{name}: a case directory of no priced program kind"
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "venue coverage differs from the allowlist:\n{}",
        problems.join("\n")
    );
}

/// Swap router registry boundary: `src/swaps/registry.rs` must hold only
/// `Arc<dyn SwapRouter>` injected via `set_router_factory`, never construct a
/// concrete Solana router itself. Regression guard for the leak fixed by
/// moving `JupiterRouter`/`RaydiumRouter` construction to
/// `crate::chains::solana::swaps::routers::build_routers`, registered once by
/// the composition root (`src/run/services.rs`).
#[test]
fn shared_swaps_registry_never_imports_solana_routers() {
    let path = "swaps/registry.rs";
    let contents = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(path))
        .expect("src/swaps/registry.rs must exist");

    assert!(
        !code_lines(&contents).contains("chains::solana"),
        "src/{path} must not import crate::chains::solana — the router factory is injected \
         by the composition root, never constructed here"
    );
}

/// Registry access is fallible. Boot registers a factory; quote/execution
/// paths convert a missing factory into a structured error. Reintroducing
/// `expect`/`unwrap`/`panic` on initialization would abort tests and
/// library callers that reach swaps before `set_router_factory`.
#[test]
fn shared_swaps_registry_never_panics_on_missing_factory() {
    let path = "swaps/registry.rs";
    let contents = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(path))
        .expect("src/swaps/registry.rs must exist");
    let production = contents
        .split("#[cfg(test)]")
        .next()
        .expect("production source");
    let code = code_lines(&production);

    for needle in [
        ".expect(",
        "panic!(",
        ".unwrap()",
        ".unwrap_or_else(|| panic!",
    ] {
        assert!(
            !code.contains(needle),
            "src/{path} must not {needle} — uninitialized registry access is fallible"
        );
    }
}

/// Tool swaps must bind execution to the quoting router via the SwapRouter
/// contract. Calling Jupiter's wallet helper with another router's quote
/// submits foreign `execution_data` to Jupiter.
#[test]
fn tool_swap_executor_never_calls_jupiter_wallet_helper() {
    let path = "tools/swap_executor.rs";
    let contents = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(path))
        .expect("src/tools/swap_executor.rs must exist");
    let code = code_lines(&contents);

    for needle in [
        "swaps::routers::execute_for_wallet",
        "execute_with_keypair",
        "JupiterRouter",
        "enabled_routers()",
        "enabled[0]",
    ] {
        assert!(
            !code.contains(needle),
            "src/{path} must not {needle} — quote and wallet execution go through \
             crate::swaps::quote_and_execute_for_wallet so the producing router owns the payload"
        );
    }
}

/// Shared transaction subject and delta domain files stay chain-neutral:
/// Solana pubkey conversion lives under `src/chains/solana/transactions/subject.rs`,
/// and native fees use a raw-unit name rather than Solana lamports.
#[test]
fn shared_transaction_subject_and_delta_domain_stay_chain_neutral() {
    const SCOPED_FILES: &[&str] = &["transactions/subject.rs", "transactions/deltas.rs"];
    let banned_needles = ["solana_sdk", "lamports", "Pubkey"];

    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path_str = relative.to_string_lossy();
        if !SCOPED_FILES.contains(&path_str.as_ref()) {
            continue;
        }
        let code = code_lines(&contents);
        for needle in banned_needles {
            if code.contains(needle) {
                violations.push(format!("src/{path_str}: names `{needle}`"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "src/transactions/subject.rs and src/transactions/deltas.rs must not import \
         solana_sdk or name lamports — Solana conversion belongs under \
         src/chains/solana/transactions:\n{}",
        violations.join("\n")
    );
}

/// The empty `src/constants.rs` compatibility façade was removed. Solana
/// mint/native-unit constants live in `crate::chains::solana::constants`;
/// do not reintroduce a crate-root constants module as a re-export shim.
#[test]
fn crate_root_constants_facade_must_not_return() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/constants.rs");
    assert!(
        !path.exists(),
        "src/constants.rs must not return — Solana literals belong in \
         crate::chains::solana::constants, not a crate-root façade"
    );
}

/// The file with its `#[cfg(test)]` items removed, line numbering preserved.
///
/// This used to truncate at the FIRST `#[cfg(test)]`, which made every guard
/// blind to everything after an inline test module. In `swaps/operations.rs`
/// that module sits at line 409 and the quote path it hid, at line 490, is
/// exactly where the no-route blacklist was silently reading error prose.
/// Removing only the test items — and replacing them with blank lines so
/// reported line numbers still point at the real source — closes that hole.
fn production_text(contents: &str) -> String {
    let mut out = String::with_capacity(contents.len());
    let mut skipping_depth: Option<i32> = None;
    let mut awaiting_block = false;
    for line in contents.lines() {
        if skipping_depth.is_none()
            && !awaiting_block
            && line.trim_start().starts_with("#[cfg(test)]")
        {
            awaiting_block = true;
            out.push('\n');
            continue;
        }
        if awaiting_block || skipping_depth.is_some() {
            let mut depth = skipping_depth.unwrap_or(0);
            let mut in_str = false;
            let mut escaped = false;
            for c in line.chars() {
                if in_str {
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        in_str = false;
                    }
                    continue;
                }
                match c {
                    '"' => in_str = true,
                    '{' => {
                        depth += 1;
                        awaiting_block = false;
                    }
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            out.push('\n');
            if !awaiting_block && depth <= 0 {
                skipping_depth = None;
            } else {
                skipping_depth = Some(depth);
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn is_chain_module(relative: &Path) -> bool {
    relative.starts_with("chains")
}

fn is_composition_root(relative: &Path) -> bool {
    relative.starts_with("run")
}

/// Test-support files are exempt from the production-code guards. A file
/// named `tests.rs`, or one whose name ends with `_tests.rs`, is gated out
/// of the build by a file-level `#[cfg(test)] mod ...;` declaration in its
/// parent module — gating `production_text` cannot see, because it only
/// strips `#[cfg(test)]` blocks declared inside the same file. `_tests.rs`
/// is the repo's test-support suffix.
fn is_test_support_file(relative: &Path) -> bool {
    relative
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "tests.rs" || name.ends_with("_tests.rs"))
}

/// Schema-evolution / backfill owners may name Solana as a historical data
/// fact (unscoped rows inherited by the only chain that existed then).
fn is_legacy_schema_evolution(relative: &Path) -> bool {
    let path = relative.to_string_lossy();
    path.contains("migration")
        || relative
            .file_name()
            .is_some_and(|name| name == "data_version.rs")
}

fn is_solana_identity_constructor(previous_lines: &[&str]) -> bool {
    for line in previous_lines.iter().rev() {
        let trimmed = line.trim_start();
        if trimmed.contains("fn solana(") {
            return true;
        }
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub(crate) fn ")
            || trimmed.starts_with("pub const fn ")
            || trimmed.starts_with("const fn ")
        {
            return false;
        }
    }
    false
}

/// Operational shared code selects the process chain through
/// `crate::chains::active_chain()`. Direct `ChainId::Solana` literals are
/// reserved for the chain module, the composition root, adapter tests,
/// Solana-typed identity constructors (`fn solana`), and legacy schema
/// backfills that record a historical unscoped-row default.
#[test]
fn operational_shared_code_uses_active_chain_seam() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_chain_module(&relative)
            || is_composition_root(&relative)
            || is_test_support_file(&relative)
            || is_legacy_schema_evolution(&relative)
        {
            continue;
        }
        let production = production_text(&contents);
        let mut previous = Vec::new();
        for (idx, line) in production.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//!") || trimmed.starts_with("///") || trimmed.starts_with("//")
            {
                previous.push(line);
                continue;
            }
            if line.contains("ChainId::Solana") && !is_solana_identity_constructor(&previous) {
                violations.push(format!(
                    "src/{}:{}: operational `ChainId::Solana` — call crate::chains::active_chain() \
                     instead",
                    relative.display(),
                    idx + 1
                ));
            }
            previous.push(line);
        }
    }
    assert!(
        violations.is_empty(),
        "shared operational code must select the process chain through \
         crate::chains::active_chain(), not by naming ChainId::Solana:\n{}",
        violations.join("\n")
    );
}

/// Removes comment text from production-filtered source, string-aware: a
/// `//` or `/*` inside a string literal is code, not a comment. Line
/// comments drop everything to end-of-line; block comments drop the whole
/// (nestable) `/* .. */` span. Newlines are preserved so reported line
/// numbers still point at the real source.
///
/// The per-line `starts_with("//")` skip only removes lines that START with
/// a comment, so a trailing `// chains::solana` after real code would
/// otherwise freeze comment text into the ratchet as if it were a call
/// site. The three ratchet guards below run this over their
/// [`production_text`] output before trigger matching; `production_text`
/// itself is shared with the other guards and stays unchanged.
fn strip_comment_text(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_str = false;
    let mut escaped = false;
    let mut in_line_comment = false;
    let mut block_depth = 0usize;
    while let Some(c) = chars.next() {
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            out.push(c);
        } else if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
                out.push(c);
            }
        } else if block_depth > 0 {
            if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                block_depth += 1;
            } else if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_depth -= 1;
            }
            if c == '\n' {
                out.push('\n');
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if c == '/' && chars.peek() == Some(&'/') {
            chars.next();
            in_line_comment = true;
            out.pop();
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_depth = 1;
            out.pop();
        } else {
            out.push(c);
        }
    }
    out
}

/// Files outside `src/chains/` whose production code still names
/// `chains::solana` directly. Each entry is coupling that must move behind
/// the chain runtime; the list freezes the exact current set, so a file that
/// names the concrete chain without an entry here is a new bypass and an
/// entry whose file no longer names it must be removed — the list only ever
/// shrinks.
const NEUTRAL_FILES_NAMING_CHAINS_SOLANA: &[&str] = &[
    "account/mod.rs",
    "agent_control/tools/portfolio.rs",
    "config/error.rs",
    "config/wallet.rs",
    "connectivity/monitors/rpc.rs",
    "errors/error.rs",
    "positions/ledger/sync.rs",
    "positions/operations/close.rs",
    "positions/operations/partial_close.rs",
    "services/implementations/referral_service.rs",
    "swaps/operations.rs",
    "telegram/commands/status.rs",
    "tools/ata_cleanup/operations.rs",
    "tools/ata_cleanup/types.rs",
    "tools/multi_wallet/buy.rs",
    "tools/multi_wallet/consolidate.rs",
    "tools/multi_wallet/sell.rs",
    "tools/multi_wallet/transfer.rs",
    "trader/copy/service.rs",
    "transactions/database/deltas.rs",
    "transactions/debug.rs",
    "transactions/debug_helpers.rs",
    "transactions/service/bootstrap.rs",
    "transactions/service/lifecycle.rs",
    "transactions/service/reclassify.rs",
    "transactions/verifier.rs",
    "wallets/balance_monitor/database.rs",
    "wallets/balance_monitor/service.rs",
    "wallets/bulk/validator.rs",
    "wallets/manager/balance_ops.rs",
    "wallets/manager/balance_queries.rs",
    "wallets/manager/bulk_ops.rs",
    "wallets/manager/crud.rs",
    "wallets/manager/migration.rs",
    "webserver/routes/initialization/handlers.rs",
    "webserver/routes/initialization/types.rs",
    "webserver/routes/tools/ata_cleanup.rs",
    "webserver/routes/tools/burn_tokens.rs",
    "webserver/routes/tools/multi_wallet/multi_buy.rs",
    "webserver/routes/tools/multi_wallet/multi_sell.rs",
    "webserver/routes/tools/multi_wallet/wallet_ops.rs",
    "webserver/routes/trader/manual.rs",
    "webserver/routes/transactions/handlers.rs",
];

/// Neutral code — everything outside `src/chains/` — must reach chains only
/// through the chain runtime. A direct `chains::solana` name in production
/// code binds the file to one concrete chain implementation.
#[test]
fn neutral_code_reaches_chains_only_through_runtime() {
    let mut hits: Vec<String> = Vec::new();
    let mut new_violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_chain_module(&relative) || is_test_support_file(&relative) {
            continue;
        }
        let path = relative.to_string_lossy().into_owned();
        // Comment text is not code: a trailing `// chains::solana` must not
        // freeze a comment into the ratchet as a production reference.
        let production = strip_comment_text(&production_text(&contents));
        for (idx, line) in production.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line.contains("chains::solana") {
                hits.push(path.clone());
                if !NEUTRAL_FILES_NAMING_CHAINS_SOLANA.contains(&path.as_str()) {
                    new_violations.push(format!("src/{path}:{}: {}", idx + 1, line.trim()));
                }
            }
        }
    }
    let stale: Vec<&str> = NEUTRAL_FILES_NAMING_CHAINS_SOLANA
        .iter()
        .copied()
        .filter(|entry| !hits.iter().any(|hit| hit.as_str() == *entry))
        .collect();
    assert!(
        new_violations.is_empty() && stale.is_empty(),
        "neutral code must reach chains only through the chain runtime — route this through \
         the chain runtime (new files naming chains::solana outside src/chains):\n{}\n\
         remove it from the allowlist (entries that no longer name chains::solana):\n{}",
        new_violations.join("\n"),
        stale.join("\n")
    );
}

/// The transitions that undo a swap the chain proved did not land. Only the settlement
/// disposition (`positions/settle.rs`) decides them, so no second decision table can act on
/// weaker evidence than a signature verdict.
const SETTLEMENT_FAILURE_TRANSITIONS: &[&str] = &[
    "RemoveOrphanEntry",
    "DcaFailed",
    "PartialExitFailed",
    "ExitFailedClearForRetry",
];

/// Files whose production code builds or matches a settlement-failure transition: the
/// variants' owner (which also maps each to its trade-action verdict), the decision's owner
/// and the applier. The list freezes the exact current set and only shrinks.
const FILES_NAMING_SETTLEMENT_FAILURE_TRANSITIONS: &[&str] = &[
    "positions/apply.rs",
    "positions/settle.rs",
    "positions/transitions.rs",
];

/// True when `line` builds or matches a settlement-failure transition: the variant name as a
/// whole identifier followed by its field block, on the same line or, when the line ends at
/// the name, on `next_line`, the next non-blank line. Prose naming the variant is not a hit.
fn names_settlement_failure_transition(line: &str, next_line: Option<&str>) -> bool {
    let is_identifier_char = |c: char| c.is_alphanumeric() || c == '_';
    SETTLEMENT_FAILURE_TRANSITIONS.iter().any(|variant| {
        let mut search_from = 0;
        while let Some(found) = line[search_from..].find(variant) {
            let at = search_from + found;
            let end = at + variant.len();
            search_from = end;
            let starts_word = !line[..at]
                .chars()
                .next_back()
                .is_some_and(is_identifier_char);
            let rest = line[end..].trim_start();
            let opens_block = rest.starts_with('{')
                || (rest.is_empty()
                    && next_line.is_some_and(|next| next.trim_start().starts_with('{')));
            if starts_word && opens_block {
                return true;
            }
        }
        false
    })
}

#[test]
fn names_settlement_failure_transition_matches_values_and_patterns_only() {
    let cases = [
        (
            "Disposition::Apply(PositionTransition::DcaFailed {",
            None,
            true,
        ),
        ("T::ExitFailedClearForRetry { .. } => {", None, true),
        ("    RemoveOrphanEntry {", None, true),
        (
            "PositionTransition::PartialExitFailed{ position_id, reason }",
            None,
            true,
        ),
        (
            "let transition = PositionTransition::ExitFailedClearForRetry",
            Some("    {"),
            true,
        ),
        ("    Self::DcaFailed", Some("{ reason, .. } => {"), true),
        (
            "// a bare ExitFailedClearForRetry recorded nothing",
            None,
            false,
        ),
        (
            "PositionTransition::ExitFailedClearForRetryLater {",
            None,
            false,
        ),
        ("NotDcaFailed {", None, false),
        (
            "matches!(transition, T::DcaFailed",
            Some("    | T::Other"),
            false,
        ),
        ("label(PositionTransition::DcaFailed)", Some("{"), false),
    ];
    for (line, next_line, expected) in cases {
        assert_eq!(
            names_settlement_failure_transition(line, next_line),
            expected,
            "{line} / {next_line:?}"
        );
    }
}

/// What a signature that failed or did not land means for a position is decided in one
/// place, the settlement disposition; the verifier and every other path only retry.
#[test]
fn settlement_failure_transitions_are_decided_only_by_the_disposition() {
    let mut hits: Vec<String> = Vec::new();
    let mut new_violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_test_support_file(&relative) {
            continue;
        }
        let path = relative.to_string_lossy().into_owned();
        let production = strip_comment_text(&production_text(&contents));
        let lines: Vec<&str> = production.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let next_line = lines[idx + 1..]
                .iter()
                .copied()
                .find(|next| !next.trim().is_empty());
            if names_settlement_failure_transition(line, next_line) {
                hits.push(path.clone());
                if !FILES_NAMING_SETTLEMENT_FAILURE_TRANSITIONS.contains(&path.as_str()) {
                    new_violations.push(format!("src/{path}:{}: {}", idx + 1, line.trim()));
                }
            }
        }
    }
    let stale: Vec<&str> = FILES_NAMING_SETTLEMENT_FAILURE_TRANSITIONS
        .iter()
        .copied()
        .filter(|entry| !hits.iter().any(|hit| hit.as_str() == *entry))
        .collect();
    assert!(
        new_violations.is_empty() && stale.is_empty(),
        "a failed or not-landed signature is settled only through `settle::disposition` - \
         return a retry instead (new files building or matching a settlement-failure \
         transition):\n{}\nremove it from the allowlist (entries that no longer name \
         one):\n{}",
        new_violations.join("\n"),
        stale.join("\n")
    );
}

/// Files under `chains/solana/rpc/client/` whose production code binds only the success of
/// an `execute_raw` call, so a failed read falls through as if it returned nothing. The list
/// freezes the exact current set and only shrinks.
const RPC_CLIENT_FILES_SKIPPING_FAILED_READS: &[&str] = &[];

/// The 1-based lines of `source` that open an `if let Ok(..)` or `while let Ok(..)` whose
/// scrutinee, up to the opening brace of its block, is an `execute_raw` call.
fn execute_raw_success_bindings(source: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    for pattern in ["if let Ok(", "while let Ok("] {
        let mut search_from = 0;
        while let Some(found) = source[search_from..].find(pattern) {
            let at = search_from + found;
            search_from = at + pattern.len();
            let block = source[search_from..]
                .find('{')
                .map_or(source.len(), |brace| search_from + brace);
            if source[at..block].contains("execute_raw") {
                lines.push(source[..at].matches('\n').count() + 1);
            }
        }
    }
    lines.sort_unstable();
    lines
}

#[test]
fn execute_raw_success_bindings_find_split_scrutinees_only() {
    let source = "\
fn read() {
    if let Ok(result) = self
        .manager
        .execute_raw(\"getTokenAccountsByOwner\", params)
        .await
    {
        use_it(result);
    }
    let result = self.manager.execute_raw(\"getBalance\", params).await?;
    if let Ok(Some(_)) = self.get_account(&ata).await {
        return;
    }
    while let Ok(page) = self.manager.execute_raw(\"getProgramAccounts\", p).await {}
}
";
    assert_eq!(execute_raw_success_bindings(source), [2, 13]);
}

/// A failed RPC read in the Solana client is an error to its caller, never an empty
/// success: a caller summing a holding or deciding on an account list reads an empty
/// result as a known zero.
#[test]
fn rpc_client_never_reports_a_failed_read_as_success() {
    let mut hits: Vec<String> = Vec::new();
    let mut new_violations = Vec::new();
    for (relative, contents) in walk_src() {
        if !relative.starts_with("chains/solana/rpc/client") || is_test_support_file(&relative) {
            continue;
        }
        let path = relative.to_string_lossy().into_owned();
        let production = strip_comment_text(&production_text(&contents));
        for line in execute_raw_success_bindings(&production) {
            hits.push(path.clone());
            if !RPC_CLIENT_FILES_SKIPPING_FAILED_READS.contains(&path.as_str()) {
                new_violations.push(format!("src/{path}:{line}"));
            }
        }
    }
    let stale: Vec<&str> = RPC_CLIENT_FILES_SKIPPING_FAILED_READS
        .iter()
        .copied()
        .filter(|entry| !hits.iter().any(|hit| hit.as_str() == *entry))
        .collect();
    assert!(
        new_violations.is_empty() && stale.is_empty(),
        "propagate the `execute_raw` error with `?` instead of binding only its success \
         (new sites):\n{}\nremove it from the allowlist (entries that no longer skip a \
         failed read):\n{}",
        new_violations.join("\n"),
        stale.join("\n")
    );
}

/// Neutral code reads chain settings through `ChainsConfig` methods that take
/// a `ChainId`; a direct `.chains.solana` field read binds the file to one chain.
/// The list freezes the current exceptions and only shrinks.
const NEUTRAL_FILES_READING_SOLANA_SETTINGS: &[&str] =
    &["webserver/routes/initialization/handlers.rs"];

#[test]
fn neutral_code_reads_chain_settings_by_chain_id() {
    let field_read = regex::Regex::new(r"\.chains\s*\.\s*solana\b").expect("valid pattern");
    let mut hits: Vec<String> = Vec::new();
    let mut new_violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_chain_module(&relative)
            || relative.starts_with("config")
            || is_test_support_file(&relative)
        {
            continue;
        }
        let path = relative.to_string_lossy().into_owned();
        let production = strip_comment_text(&production_text(&contents));
        // Whole-text scan: a field chain split across lines by rustfmt is
        // still one read.
        for found in field_read.find_iter(&production) {
            let line = production[..found.start()].matches('\n').count() + 1;
            hits.push(path.clone());
            if !NEUTRAL_FILES_READING_SOLANA_SETTINGS.contains(&path.as_str()) {
                new_violations.push(format!("src/{path}:{line}: {}", found.as_str()));
            }
        }
    }
    let stale: Vec<&str> = NEUTRAL_FILES_READING_SOLANA_SETTINGS
        .iter()
        .copied()
        .filter(|entry| !hits.iter().any(|hit| hit.as_str() == *entry))
        .collect();
    assert!(
        new_violations.is_empty() && stale.is_empty(),
        "neutral code must read chain settings through ChainsConfig methods taking a ChainId \
         (new direct .chains.solana reads outside src/chains and src/config):\n{}\n\
         remove it from the allowlist (entries that no longer read .chains.solana):\n{}",
        new_violations.join("\n"),
        stale.join("\n")
    );
}

/// `ChainsConfig` accessors take the chain as an argument; none of them may
/// pick one, so a missing chain id can never silently fall back to Solana.
#[test]
fn chain_settings_accessors_never_choose_a_chain() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/chains/config.rs");
    let contents = fs::read_to_string(&path).expect("src/chains/config.rs is readable");
    let production = strip_comment_text(&production_text(&contents));
    let chosen: Vec<&str> = ["active_chain(", "legacy_row_chain(", "enabled_chains("]
        .into_iter()
        .filter(|call| production.contains(call))
        .collect();
    assert!(
        chosen.is_empty(),
        "src/chains/config.rs must take the chain from its caller, never choose one: {}",
        chosen.join(", ")
    );
}

/// `PRE_CHAINS_LAYOUT_CHAIN` names the chain whose settings files written
/// before `[chains]` kept in global sections. It is a fact about that layout,
/// read only by its relocation; anywhere else it is a default chain under
/// another name. Its owner defines it, `chains/mod.rs` re-exports it to the
/// crate, and the relocation in `config/migrate.rs` is its one reader.
const PRE_CHAINS_LAYOUT_CHAIN_FILES: &[&str] = &["chains/config.rs", "config/migrate.rs"];
const PRE_CHAINS_LAYOUT_CHAIN_REEXPORT: &str = "pub(crate) use config::PRE_CHAINS_LAYOUT_CHAIN;";

#[test]
fn pre_chains_layout_chain_is_read_only_by_the_legacy_relocation() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path = relative.to_string_lossy().into_owned();
        if PRE_CHAINS_LAYOUT_CHAIN_FILES.contains(&path.as_str()) {
            continue;
        }
        let code = strip_comment_text(&contents);
        for (index, line) in code.lines().enumerate() {
            if !line.contains("PRE_CHAINS_LAYOUT_CHAIN") {
                continue;
            }
            if path == "chains/mod.rs" && line.trim() == PRE_CHAINS_LAYOUT_CHAIN_REEXPORT {
                continue;
            }
            violations.push(format!("src/{path}:{}: {}", index + 1, line.trim()));
        }
    }
    assert!(
        violations.is_empty(),
        "PRE_CHAINS_LAYOUT_CHAIN may be read only by the legacy relocation in \
         src/config/migrate.rs; take the chain from the caller instead:\n{}",
        violations.join("\n")
    );
}

/// Files outside `src/chains/` whose production code selects the process
/// chain through `active_chain()` or takes facts straight from
/// `chains::adapter()` — fully qualified, or the bare `adapter()` reached
/// through a `use crate::chains::adapter` import. The allowlist freezes the
/// exact current caller set; it shrinks as callers move behind the injected
/// chain runtime and grows only for a genuinely new direct caller, which is
/// the violation the guard exists to catch.
const PROCESS_CHAIN_SEAM_CALLER_FILES: &[&str] = &[
    "agent_control/tools/analysis.rs",
    "agent_control/tools/portfolio.rs",
    "apis/dexscreener/mod.rs",
    "apis/geckoterminal/mod.rs",
    "apis/native_price.rs",
    "connectivity/monitors/dexscreener.rs",
    "events/recorders/lifecycle.rs",
    "ohlcvs/cache.rs",
    "ohlcvs/fetcher.rs",
    "ohlcvs/manager.rs",
    "ohlcvs/service.rs",
    "positions/apply.rs",
    "positions/database/global.rs",
    "positions/ledger/reducer.rs",
    "positions/operations/close.rs",
    "positions/operations/dca.rs",
    "positions/operations/open.rs",
    "positions/operations/partial_close.rs",
    "positions/pnl.rs",
    "positions/verifier.rs",
    "services/implementations/copy_trading_service.rs",
    "swaps/operations.rs",
    "swaps/registry.rs",
    "telegram/wallet_alerts.rs",
    "tools/ata_cleanup/operations.rs",
    "tools/multi_wallet/buy.rs",
    "tools/multi_wallet/consolidate.rs",
    "tools/multi_wallet/sell.rs",
    "tools/multi_wallet/transfer.rs",
    "tools/swap_executor.rs",
    "tools/trade_watcher/monitor.rs",
    "trader/copy/control.rs",
    "trader/copy/service.rs",
    "trader/copy/workspace/profile.rs",
    "trader/manual/guard.rs",
    "trader/policy.rs",
    "trader/safety/blacklist.rs",
    "transactions/database/global.rs",
    "transactions/debug.rs",
    "transactions/debug_helpers.rs",
    "transactions/subject.rs",
    "transactions/utils.rs",
    "wallets/balance_monitor/service.rs",
    "wallets/balance_monitor/worth.rs",
    "wallets/manager.rs",
    "wallets/manager/balance_queries.rs",
    "wallets/watch/mod.rs",
    "webserver/promo/copy_trading.rs",
    "webserver/promo/copy_trading/desk.rs",
    "webserver/routes/blacklist/handlers.rs",
    "webserver/routes/dashboard/overview.rs",
    "webserver/routes/featured/cache.rs",
    "webserver/routes/featured/identity.rs",
    "webserver/routes/positions/activity/drafts.rs",
    "webserver/routes/positions/activity/merge.rs",
    "webserver/routes/positions/activity/mod.rs",
    "webserver/routes/positions/debug.rs",
    "webserver/routes/positions/detail.rs",
    "webserver/routes/positions/types.rs",
    "webserver/routes/tokens/detail.rs",
    "webserver/routes/tokens/ohlcv.rs",
    "webserver/routes/tokens/types.rs",
    "webserver/routes/tools/ata_cleanup.rs",
    "webserver/routes/tools/burn_tokens.rs",
    "webserver/routes/tools/multi_wallet/multi_buy.rs",
    "webserver/routes/tools/multi_wallet/multi_sell.rs",
    "webserver/routes/trader/manual.rs",
];

/// True when `line` calls `adapter(...)` unqualified — the chains adapter
/// reached through an imported `use crate::chains::adapter`. A method call
/// (`.adapter(`) or a longer identifier (`http_adapter(`) is not a hit, and
/// neither is a definition: `fn ` right before the name marks a fn or
/// method declaration (`pub fn adapter(&self)`), the same special case the
/// template guard applies to the Solana identity constructor's signature.
fn is_bare_adapter_call(line: &str) -> bool {
    let mut search_from = 0;
    while let Some(found) = line[search_from..].find("adapter(") {
        let at = search_from + found;
        let boundary = match line[..at].chars().next_back() {
            None => true,
            Some(prev) => !(prev.is_alphanumeric() || prev == '_' || prev == '.'),
        };
        let is_definition = line[..at].ends_with("fn ");
        if boundary && !is_definition {
            return true;
        }
        search_from = at + 1;
    }
    false
}

/// True when `line` names the process adapter `chains::adapter` itself — a
/// call (`chains::adapter()`) or an import (`use crate::chains::adapter;`).
/// A longer identifier (`chains::adapter_for`) or a path into the adapter
/// module (`chains::adapter::ChainAdapter`) names something else.
fn names_process_adapter(line: &str) -> bool {
    const NEEDLE: &str = "chains::adapter";
    let mut search_from = 0;
    while let Some(found) = line[search_from..].find(NEEDLE) {
        let end = search_from + found + NEEDLE.len();
        let rest = &line[end..];
        let continues_identifier = rest
            .chars()
            .next()
            .is_some_and(|next| next.is_alphanumeric() || next == '_');
        if !continues_identifier && !rest.starts_with("::") {
            return true;
        }
        search_from = end;
    }
    false
}

#[test]
fn names_process_adapter_matches_only_the_process_adapter() {
    let cases = [
        ("crate::chains::adapter()", true),
        ("use crate::chains::adapter;", true),
        ("crate::chains::adapter_for(c)", false),
        ("use crate::chains::adapter_for;", false),
        ("crate::chains::adapter::ChainAdapter", false),
    ];
    for (line, expected) in cases {
        assert_eq!(names_process_adapter(line), expected, "{line}");
    }
}

/// True when `line` calls `active_chain()` bare or path-qualified
/// (`chains::active_chain(`) — the same identifier/dot boundary as
/// [`is_bare_adapter_call`]. A method call on some other receiver
/// (`reader.active_chain(`) is not a seam call.
fn is_active_chain_call(line: &str) -> bool {
    let mut search_from = 0;
    while let Some(found) = line[search_from..].find("active_chain(") {
        let at = search_from + found;
        let boundary = match line[..at].chars().next_back() {
            None => true,
            Some(':') => true,
            Some(prev) => !(prev.is_alphanumeric() || prev == '_' || prev == '.'),
        };
        if boundary {
            return true;
        }
        search_from = at + 1;
    }
    false
}

/// The process chain and the adapter facts flow to neutral code only through
/// the chain runtime. Every direct call below is a seam call site that a
/// later step must lift behind the injected runtime; the caller file set is
/// ratcheted so it can only shrink.
#[test]
fn process_chain_seam_shrinks() {
    let mut hits: Vec<String> = Vec::new();
    let mut new_violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_chain_module(&relative) {
            continue;
        }
        let path = relative.to_string_lossy().into_owned();
        // Comment text is not code: a trailing `// adapter(...)` must not
        // freeze a comment into the ratchet as a seam call site.
        let production = strip_comment_text(&production_text(&contents));
        for (idx, line) in production.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let calls_seam = is_active_chain_call(line)
                || names_process_adapter(line)
                || is_bare_adapter_call(line);
            if calls_seam {
                hits.push(path.clone());
                if !PROCESS_CHAIN_SEAM_CALLER_FILES.contains(&path.as_str()) {
                    new_violations.push(format!("src/{path}:{}: {}", idx + 1, line.trim()));
                }
            }
        }
    }
    let stale: Vec<&str> = PROCESS_CHAIN_SEAM_CALLER_FILES
        .iter()
        .copied()
        .filter(|entry| !hits.iter().any(|hit| hit.as_str() == *entry))
        .collect();
    assert!(
        new_violations.is_empty() && stale.is_empty(),
        "the process chain and adapter facts must come through the chain runtime — route \
         this through the chain runtime (new callers of active_chain()/adapter() outside \
         src/chains):\n{}\nremove it from the allowlist (entries that no longer call the \
         seam):\n{}",
        new_violations.join("\n"),
        stale.join("\n")
    );
}

/// Files outside `src/chains/` whose production code derives a chain from an
/// address (`chain_for_address(`) or from the only enabled chain
/// (`.sole_chain(`), because their inputs carry no chain yet. Each entry leaves
/// when its input carries the chain: routes keyed by chain, positions with a
/// chain column, typed tool input. The list freezes the exact current set and
/// only shrinks; tokens and filtering never resolve a chain themselves, they
/// take it from the caller.
const IMPLICIT_CHAIN_RESOLUTION_FILES: &[&str] = &[
    "llm_analysis/background_worker.rs",
    "positions/helpers.rs",
    "positions/ledger/sync.rs",
    "positions/loss_detection.rs",
    "positions/price_resolution.rs",
    "telegram/commands/callback_positions.rs",
    "telegram/commands/callback_tokens.rs",
    "trader/actions/manual.rs",
    "trader/copy/notify.rs",
    "trader/entry.rs",
    "trader/evaluators/entry.rs",
    "trader/evaluators/exit.rs",
    "trader/manual/api.rs",
    "trader/manual/force.rs",
    "trader/monitors/exit.rs",
    "wallets/balance_monitor/dashboard/token_metadata.rs",
    "webserver/routes/featured/cards.rs",
    "webserver/routes/featured/handlers.rs",
    "webserver/routes/filtering/analytics.rs",
    "webserver/routes/filtering/stats.rs",
    "webserver/routes/filtering/tokens.rs",
    "webserver/routes/positions/list.rs",
    "webserver/routes/tokens/blacklist.rs",
    "webserver/routes/tokens/favorites.rs",
    "webserver/routes/tokens/identity.rs",
    "webserver/routes/tokens/list.rs",
    "webserver/routes/wallet/handlers.rs",
];

/// True when `line` names `chain_for_address` or `sole_chain` as a whole
/// identifier in any call form — a direct call, a path-qualified or UFCS call
/// (`ChainScope::sole_chain(&scope)`) or a point-free use
/// (`.map(chain_for_address)`). A longer identifier containing either name,
/// and the `fn` definitions themselves, are not a resolution.
fn resolves_chain_implicitly(line: &str) -> bool {
    let is_identifier_char = |c: char| c.is_alphanumeric() || c == '_';
    ["chain_for_address", "sole_chain"].iter().any(|needle| {
        let mut search_from = 0;
        while let Some(found) = line[search_from..].find(needle) {
            let at = search_from + found;
            let end = at + needle.len();
            search_from = end;
            let before = &line[..at];
            let starts_word = !before.chars().next_back().is_some_and(is_identifier_char);
            let ends_word = !line[end..].chars().next().is_some_and(is_identifier_char);
            let is_definition = before.trim_end().ends_with("fn")
                && !before
                    .trim_end()
                    .trim_end_matches("fn")
                    .chars()
                    .next_back()
                    .is_some_and(is_identifier_char);
            if starts_word && ends_word && !is_definition {
                return true;
            }
        }
        false
    })
}

#[test]
fn resolves_chain_implicitly_matches_every_call_form() {
    let cases = [
        ("crate::chains::chain_for_address(&mint)?", true),
        ("let chain = chain_for_address(mint);", true),
        (".map(chain_for_address)", true),
        (".and_then(crate::chains::chain_for_address)", true),
        ("ChainScope::All.sole_chain()?", true),
        ("ChainScope::sole_chain(&scope)", true),
        ("scopes.iter().map(ChainScope::sole_chain)", true),
        (
            "pub fn chain_for_address(address: &str) -> Result<ChainId> {",
            false,
        ),
        ("pub fn sole_chain(self) -> Result<ChainId> {", false),
        ("resolve_chain_for_address_cached(mint)", false),
        ("let sole_chain_count = 1;", false),
    ];
    for (line, expected) in cases {
        assert_eq!(resolves_chain_implicitly(line), expected, "{line}");
    }
}

#[test]
fn implicit_chain_resolution_shrinks() {
    let mut hits: Vec<String> = Vec::new();
    let mut new_violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_chain_module(&relative) {
            continue;
        }
        let path = relative.to_string_lossy().into_owned();
        let production = strip_comment_text(&production_text(&contents));
        for (idx, line) in production.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if resolves_chain_implicitly(line) {
                hits.push(path.clone());
                if !IMPLICIT_CHAIN_RESOLUTION_FILES.contains(&path.as_str()) {
                    new_violations.push(format!("src/{path}:{}: {}", idx + 1, line.trim()));
                }
            }
        }
    }
    let stale: Vec<&str> = IMPLICIT_CHAIN_RESOLUTION_FILES
        .iter()
        .copied()
        .filter(|entry| !hits.iter().any(|hit| hit.as_str() == *entry))
        .collect();
    assert!(
        new_violations.is_empty() && stale.is_empty(),
        "a chain must come from the caller's data, not be guessed from an address or the \
         only enabled chain — take the chain from the input (new files resolving a chain \
         implicitly):\n{}\nremove it from the allowlist (entries that no longer resolve \
         a chain):\n{}",
        new_violations.join("\n"),
        stale.join("\n")
    );
}

/// Domains that take the chain from their caller: no file under these
/// prefixes may resolve a chain from an address or a scope.
const CHAIN_THREADED_DOMAINS: &[&str] = &["tokens/", "filtering/", "pools/"];

#[test]
fn chain_threaded_domains_never_resolve_a_chain_implicitly() {
    let inside: Vec<&str> = IMPLICIT_CHAIN_RESOLUTION_FILES
        .iter()
        .copied()
        .filter(|entry| {
            CHAIN_THREADED_DOMAINS
                .iter()
                .any(|prefix| entry.starts_with(prefix))
        })
        .collect();
    assert!(
        inside.is_empty(),
        "{} take the chain from their caller; these entries resolve it inside the \
         domain:\n{}",
        CHAIN_THREADED_DOMAINS.join(", "),
        inside.join("\n")
    );
}

/// Pools never reads candle data: OHLCV drives strategies and indicators
/// only, and no OHLCV value may reach the pool price that trading and P&L
/// read. The chain-neutral `src/pools` and every chain's price producers under
/// `src/chains/<id>/pools` are both in scope.
#[test]
fn pools_never_read_ohlcv() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let in_chain_pools = relative.starts_with("chains")
            && relative
                .components()
                .nth(2)
                .is_some_and(|component| component.as_os_str() == "pools");
        if !relative.starts_with("pools") && !in_chain_pools {
            continue;
        }
        for (idx, line) in code_lines(&contents).lines().enumerate() {
            if line.contains("crate::ohlcvs") {
                violations.push(format!("src/{}:{}", relative.display(), idx + 1));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "src/pools and src/chains/*/pools must never read OHLCV data:\n{}",
        violations.join("\n")
    );
}

/// Statics in the trading domains whose map is keyed by a bare `String`, or
/// whose set or vector holds bare `String`s — an address, a signature, a
/// mint — carry no chain scope, so a second chain
/// would silently share (or collide with) every entry. Entries are
/// `file::STATIC` and freeze today's set; each shrinks away as its map is
/// keyed through the chain runtime with an explicit chain-scoped key.
const BARE_STRING_KEYED_STATICS: &[&str] = &[
    "positions/price_resolution.rs::FORCE_FETCH_COOLDOWN",
    "positions/state.rs::MINT_TO_POSITION_INDEX",
    "positions/state.rs::PENDING_OPEN_SWAPS",
    "positions/state.rs::POSITION_LOCKS",
    "positions/state.rs::SIG_TO_MINT_INDEX",
    "positions/state_pending.rs::PENDING_DCA_SWAPS",
    "positions/state_pending.rs::PENDING_PARTIAL_EXITS",
    "positions/state_pending.rs::PENDING_PARTIAL_EXIT_DETAILS",
    "positions/verifier.rs::LAST_TOKEN_ACCOUNTS_CHECK",
    "swaps/operations.rs::NO_ROUTE_STRIKES",
    "trader/copy/paper_exits.rs::HELD_PAPER_MINTS",
    "trader/entry.rs::ENTRY_CYCLE_RESERVATIONS",
    "transactions/utils.rs::GLOBAL_KNOWN_SIGNATURES",
    "transactions/utils.rs::GLOBAL_PENDING_TRANSACTIONS",
];

/// Top-level trading domains in scope for [`no_bare_string_keyed_statics`].
const BARE_STRING_KEYED_STATIC_DIRS: &[&str] = &[
    "pools",
    "positions",
    "transactions",
    "trader",
    "swaps",
    "tokens",
    "filtering",
];

/// The name of the static declared on `line`, if the line starts a `static`
/// item declaration (any visibility).
fn static_declaration_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let after_visibility = trimmed
        .strip_prefix("pub(crate) ")
        .or_else(|| trimmed.strip_prefix("pub(super) "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .unwrap_or(trimmed);
    let after_static = after_visibility.strip_prefix("static ")?;
    let rest = after_static.strip_prefix("mut ").unwrap_or(after_static);
    let name_end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    (name_end > 0).then_some(&rest[..name_end])
}

/// True when the declared type of a `static NAME: TYPE` header has `PerChain`
/// as its outermost type.
fn is_per_chain_declaration(declared_type: &str) -> bool {
    declared_type
        .split_once(':')
        .map(|(_, ty)| ty.trim_start())
        .is_some_and(|ty| ty.starts_with("PerChain<") || ty.starts_with("crate::chains::PerChain<"))
}

#[test]
fn per_chain_declarations_are_recognized_by_their_outermost_type() {
    let cases = [
        (
            "static CACHE: PerChain<moka::sync::Cache<String, u8>> ",
            true,
        ),
        (
            "static CACHE: crate::chains::PerChain<Cache<String, u8>> ",
            true,
        ),
        (
            "static CACHE: LazyLock<PerChain<Cache<String, u8>>> ",
            false,
        ),
        ("static CACHE: LazyLock<HashMap<String, u8>> ", false),
    ];
    for (header, expected) in cases {
        assert_eq!(is_per_chain_declaration(header), expected, "{header}");
    }
}

/// True when the declared type of a `static NAME: TYPE` header holds bare
/// `String` keys or members — a `String`-keyed map, a set of `String`s or a
/// vector of `String`s — outside a `PerChain` slot.
fn declares_bare_string_keys(declared_type: &str) -> bool {
    let holds_strings = declared_type.contains("<String,")
        || declared_type.contains("Set<String>")
        || declared_type.contains("Vec<String>");
    holds_strings && !is_per_chain_declaration(declared_type)
}

#[test]
fn bare_string_key_declarations_cover_maps_sets_and_vectors() {
    let cases = [
        ("static CACHE: LazyLock<HashMap<String, u8>> ", true),
        ("static CACHE: LazyLock<DashMap<String, u8>> ", true),
        (
            "static MINTS: LazyLock<RwLock<std::collections::HashSet<String>>> ",
            true,
        ),
        ("static MINTS: LazyLock<BTreeSet<String>> ", true),
        ("static MINTS: LazyLock<DashSet<String>> ", true),
        ("static MINTS: LazyLock<RwLock<Vec<String>>> ", true),
        ("static MINTS: LazyLock<RwLock<Option<Vec<String>>>> ", true),
        ("static MINTS: PerChain<ArcSwap<HashSet<String>>> ", false),
        (
            "static CACHE: LazyLock<HashMap<(ChainId, String), u8>> ",
            false,
        ),
        ("static NAMES: LazyLock<Vec<&'static str>> ", false),
        ("static COUNT: AtomicU64 ", false),
    ];
    for (header, expected) in cases {
        assert_eq!(declares_bare_string_keys(header), expected, "{header}");
    }
}

/// A trading-domain static must not key its map by a bare `String`, nor hold
/// bare `String`s in a set or vector. The declaration header (everything from
/// `static NAME` up to the `=`) is scanned for the `<String,` map-key
/// pattern and the `Set<String>` and `Vec<String>` member patterns, which
/// cover `HashSet`, `BTreeSet`, `DashSet` and `Option<Vec<String>>`; the
/// allowlist ratchets the current set in both directions.
///
/// A conforming chain-scoped static takes one of two shapes, neither of
/// which needs an entry:
/// - the map is keyed by the tuple `(ChainId, String)`, which does not match
///   the pattern;
/// - the declared type's outermost type is `PerChain<..>`, one slot per chain,
///   so the `String` key inside a slot is already scoped to that slot's chain.
///
/// A nested per-chain map (`LazyLock<HashMap<ChainId, HashMap<String, T>>>`)
/// DOES match and would false-fail; such a static must take one of the two
/// shapes instead.
///
/// Known latent gaps: [`static_declaration_name`] does not recognize
/// `pub(in path)` visibility (none exist today), and a `String` reached
/// through a type alias or a wrapper type is not seen in the header.
#[test]
fn no_bare_string_keyed_statics() {
    let mut hits: Vec<String> = Vec::new();
    let mut new_violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path = relative.to_string_lossy().into_owned();
        let top_level = path.split('/').next().unwrap_or("");
        if !BARE_STRING_KEYED_STATIC_DIRS.contains(&top_level) {
            continue;
        }
        // Comment text is not code: a `// ... <String,` comment must not
        // freeze a declaration into the ratchet.
        let production = strip_comment_text(&production_text(&contents));
        // A multi-line declaration header accumulates here until its `=`.
        let mut pending: Option<(String, String)> = None;
        for (idx, line) in production.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let (name, mut header) = match pending.take() {
                Some((name, header)) => (name, header),
                None => match static_declaration_name(line) {
                    Some(name) => (name.to_owned(), String::new()),
                    None => continue,
                },
            };
            header.push_str(line);
            header.push('\n');
            if !header.contains('=') {
                pending = Some((name, header));
                continue;
            }
            let declared_type = header.split('=').next().unwrap_or("");
            if !declares_bare_string_keys(declared_type) {
                continue;
            }
            let key = format!("{path}::{name}");
            hits.push(key.clone());
            if !BARE_STRING_KEYED_STATICS.contains(&key.as_str()) {
                new_violations.push(format!(
                    "src/{path}:{}: static {name} holds bare String keys",
                    idx + 1
                ));
            }
        }
    }
    let stale: Vec<&str> = BARE_STRING_KEYED_STATICS
        .iter()
        .copied()
        .filter(|entry| !hits.iter().any(|hit| hit.as_str() == *entry))
        .collect();
    assert!(
        new_violations.is_empty() && stale.is_empty(),
        "trading-domain statics must not key their maps by a bare String — route this \
         through the chain runtime (new bare-String-keyed statics):\n{}\nremove it from \
         the allowlist (entries that no longer declare a bare-String key):\n{}",
        new_violations.join("\n"),
        stale.join("\n")
    );
}

/// `src/apis/` hosts only multi-chain providers plus its neutral plumbing.
/// Solana-only providers live under `src/chains/solana/apis/`. Any new entry
/// in the directory is a single-chain provider coming back (or unexpected
/// plumbing) and fails here.
const APIS_NEUTRAL_ENTRIES: &[&str] = &[
    "coingecko",
    "defillama",
    "dexscreener",
    "geckoterminal",
    "llm",
    "rugcheck",
    "client.rs",
    "error.rs",
    "manager.rs",
    "mod.rs",
    "native_price.rs",
    "stats.rs",
];

#[test]
fn apis_hosts_only_multi_chain_providers() {
    let apis_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/apis");
    let mut violations = Vec::new();
    for entry in fs::read_dir(&apis_dir).expect("read_dir(src/apis) must succeed") {
        let entry = entry.expect("dir entry must be readable");
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.path().is_dir();
        if !is_dir && !name.ends_with(".rs") {
            continue; // README.md and other non-source entries
        }
        if !APIS_NEUTRAL_ENTRIES.contains(&name.as_str()) {
            violations.push(format!(
                "src/apis/{name}: not a multi-chain provider — Solana-only providers \
                 belong under src/chains/solana/apis"
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "src/apis may host only multi-chain providers:\n{}",
        violations.join("\n")
    );
}

/// Neutral modules must not re-export Solana-owned items. Callers that need
/// ATA helpers, mint constants, classification, or address validation import
/// `crate::chains::solana` directly.
#[test]
fn modules_outside_chains_must_not_reexport_solana_items() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_chain_module(&relative) {
            continue;
        }
        for (idx, line) in code_lines(&contents).lines().enumerate() {
            let trimmed = line.trim_start();
            let is_pub_use = trimmed.starts_with("pub use ")
                || trimmed.starts_with("pub(crate) use ")
                || trimmed.starts_with("pub(super) use ");
            if !is_pub_use {
                continue;
            }
            if trimmed.contains("chains::solana")
                || trimmed.contains("SOL_MINT")
                || trimmed.contains("SOL_DECIMALS")
            {
                violations.push(format!("src/{}:{}: {trimmed}", relative.display(), idx + 1));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "modules outside src/chains must not pub-use Solana-owned items \
         (`crate::chains::solana`, SOL_MINT, SOL_DECIMALS):\n{}",
        violations.join("\n")
    );
}

/// Wallet management must not alias the Solana keypair module as a local
/// `crypto` façade. Import `crate::chains::solana::accounts` at the call site.
#[test]
fn wallets_module_must_not_alias_solana_crypto() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_chain_module(&relative) {
            continue;
        }
        for (idx, line) in code_lines(&contents).lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.contains("chains::solana") && trimmed.contains(" as crypto") {
                violations.push(format!("src/{}:{}: {trimmed}", relative.display(), idx + 1));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "shared modules must not alias crate::chains::solana items as `crypto`:\n{}",
        violations.join("\n")
    );
}

/// Wallet-watch execution boundary: `src/wallets/watch/**` production code
/// owns targets, persistence, dedupe, scheduling and lifecycle, and must
/// reach chain execution only through the injected `runtime::WalletWatchRuntime`
/// seam (`crate::wallets::watch::runtime`) — never by importing
/// `crate::chains::solana`, `solana_sdk`, or naming a concrete
/// `TransactionFetcher`/`TransactionProcessor`/`Pubkey` directly. The concrete
/// Solana runtime lives in `crate::chains::solana::wallets::runtime::
/// build_runtime`, registered once by the composition root
/// (`src/run/services.rs`). Test code is scanned too: a co-located unit test
/// must build its fixtures from chain-neutral `AccountId`/`Subject`
/// constructors or the `runtime::test_support::FakeRuntime`, never a
/// Solana-typed constructor merely to satisfy a test helper.
#[test]
fn wallet_watch_production_code_never_reaches_solana_directly() {
    let banned_needles = [
        "chains::solana",
        "solana_sdk",
        "TransactionFetcher",
        "TransactionProcessor",
        "Pubkey",
    ];

    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path_str = relative.to_string_lossy();
        if !path_str.starts_with("wallets/watch/") {
            continue;
        }
        for (idx, line) in code_lines(&contents).lines().enumerate() {
            for needle in banned_needles {
                if line.contains(needle) {
                    violations.push(format!("src/{path_str}:{}: names `{needle}`", idx + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "src/wallets/watch must reach chain execution only through \
         runtime::WalletWatchRuntime, injected by the composition root — never by \
         importing crate::chains::solana or a concrete Solana type directly:\n{}",
        violations.join("\n")
    );
}

/// Every SQLite write path must open its transaction through
/// `database::WriteTransaction::write_tx` (IMMEDIATE), never through
/// rusqlite's bare `Connection::transaction()` (DEFERRED).
///
/// A DEFERRED transaction that reads before it writes must upgrade its lock on
/// the first write statement, and in WAL mode SQLite fails that upgrade with
/// `SQLITE_BUSY` **immediately, ignoring `busy_timeout`** — because another
/// connection may have committed since the read snapshot was taken. That is
/// what produced `Failed to clear token pools: database is locked` under the
/// eight concurrent `TOKEN_POOLS` refresh workers, and it was latent in every
/// other read-then-write transaction in the tree.
///
/// Every transaction in this codebase writes, so there is no legitimate bare
/// `.transaction()` call site. See `src/database/transaction.rs`.
#[test]
fn sqlite_writers_use_immediate_transactions() {
    let mut offenders: Vec<String> = Vec::new();

    for (relative, contents) in walk_src() {
        // The trait's own unit test calls `.transaction()` deliberately, to
        // demonstrate the upgrade failure it exists to prevent.
        if relative == Path::new("database/transaction.rs") {
            continue;
        }
        for (idx, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//!") || trimmed.starts_with("///") {
                continue;
            }
            if line.contains(".transaction()") {
                offenders.push(format!(
                    "{}:{} -> {}",
                    relative.display(),
                    idx + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "bare DEFERRED `.transaction()` is forbidden — use `write_tx()` from \
         `crate::database::WriteTransaction` so the write lock is taken before \
         the first read and `busy_timeout` actually applies:\n{}",
        offenders.join("\n")
    );
}

/// The vendor façade is not a loophole. `shared_modules_never_import_solana_vendor_crates_raw`
/// only inspects lines beginning `use <crate>::`, so a fully-qualified
/// `crate::chains::solana::solana_sdk::pubkey::Pubkey` in a type position slipped
/// straight past it — which is how `Pubkey` survived in `src/transactions` long
/// after that module's own doc comment declared it chain-neutral. Reaching a vendor
/// type through the façade path is the same dependency as importing it raw.
///
/// Test code is exempt: a `#[cfg(test)]` block may build a real `Keypair` when that
/// is what the test is proving (see `services/implementations/referral_service.rs`,
/// where the test signs a referral proof for real). Weakening such a test to satisfy
/// a textual scan would delete the check, not the coupling.
#[test]
fn shared_modules_never_name_solana_vendor_types_through_the_facade() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_solana_owned(&relative) {
            continue;
        }
        let production = production_text(&contents);
        for (idx, line) in code_lines(&production).lines().enumerate() {
            for crate_name in VENDOR_CRATES {
                if line.contains(&format!("chains::solana::{crate_name}")) {
                    violations.push(format!(
                        "src/{}:{}: reaches `{crate_name}` through the chains::solana façade — \
                         shared code must use a chain-neutral type (Subject, AccountId, AssetId) \
                         and let src/chains/solana convert at the boundary",
                        relative.display(),
                        idx + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "shared modules must not name a Solana vendor type, even through the façade:\n{}",
        violations.join("\n")
    );
}

/// Asset and native-unit facts come from `chains::adapter()`, not from a direct
/// import of the Solana constants. `SOL_MINT`, the stable mints, `SOL_DECIMALS`
/// and the lamport converters all have adapter equivalents
/// (`native_asset_address`, `is_native_asset`, `stable_assets`,
/// `native_asset_decimals`, `raw_to_native`, `native_to_raw`).
///
/// The ATA/rent constants are deliberately NOT on this list: the ATA-cleanup and
/// multi-wallet tools model a Solana-only concept end to end, so importing the
/// constant is honest there. When those tools move behind a chain-asset-operations
/// seam, add the names here.
#[test]
fn shared_modules_take_asset_and_unit_facts_from_the_adapter() {
    const ADAPTER_OWNED: &[&str] = &[
        "SOL_MINT",
        "USDC_MINT",
        "USDT_MINT",
        "SOL_DECIMALS",
        "LAMPORTS_PER_SOL",
        "SYSTEM_PROGRAM_ID",
        "lamports_to_sol",
        "sol_to_lamports",
    ];

    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_chain_module(&relative) {
            continue;
        }
        for (idx, line) in code_lines(&contents).lines().enumerate() {
            if !line.contains("chains::solana::constants") {
                continue;
            }
            for name in ADAPTER_OWNED {
                if line.contains(name) {
                    violations.push(format!(
                        "src/{}:{}: imports `{name}` — ask crate::chains::adapter() instead",
                        relative.display(),
                        idx + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "shared modules must take asset and native-unit facts from the chain adapter, \
         not from Solana constants:\n{}",
        violations.join("\n")
    );
}

/// A provider's name for this chain is a chain fact. Hardcoding `"solana"` as a
/// network/chainId/platform argument pins every market-data call to one chain;
/// it comes from `chains::adapter().market_data_network()`.
///
/// Doc comments may still name Solana when documenting a parameter, and the
/// legacy schema-evolution files record it as a historical row value.
#[test]
fn shared_modules_never_hardcode_the_market_data_network_slug() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_chain_module(&relative)
            || is_composition_root(&relative)
            || is_test_support_file(&relative)
            || is_legacy_schema_evolution(&relative)
        {
            continue;
        }
        let production = production_text(&contents);
        for (idx, line) in code_lines(&production).lines().enumerate() {
            if line.contains("\"solana\"") {
                violations.push(format!(
                    "src/{}:{}: hardcodes the \"solana\" network slug — call \
                     crate::chains::adapter().market_data_network()",
                    relative.display(),
                    idx + 1
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "the provider network slug must come from the chain adapter:\n{}",
        violations.join("\n")
    );
}

/// Every chain-parameter value has exactly one owning `const`. These four were
/// each duplicated across modules — the rent-exempt minimum existed twice in
/// different units (890_880 lamports and 0.00089088 SOL) and the ATA rent value
/// seven times in three forms. Copies that share a value but not a name are
/// invisible to a name-based search, so this guard scans by value.
#[test]
fn chain_parameter_values_have_exactly_one_owning_const() {
    const OWNED_VALUES: &[(&str, &str)] = &[
        ("ATA rent (lamports)", "2_039_280"),
        ("ATA rent (SOL)", "0.00203928"),
        ("rent-exempt minimum (lamports)", "890_880"),
        ("rent-exempt minimum (SOL)", "0.00089088"),
        ("lamports per SOL", "1_000_000_000"),
    ];
    const OWNER: &str = "chains/solana/constants.rs";

    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path_str = relative.to_string_lossy();
        if path_str == OWNER {
            continue;
        }
        let production = production_text(&contents);
        for (label, value) in OWNED_VALUES {
            for (idx, line) in production.lines().enumerate() {
                let trimmed = line.trim_start();
                let is_const_def = (trimmed.starts_with("const ")
                    || trimmed.starts_with("pub const ")
                    || trimmed.starts_with("pub(crate) const ")
                    || trimmed.starts_with("pub(super) const "))
                    && trimmed.contains('=');
                if is_const_def && trimmed.contains(value) {
                    violations.push(format!(
                        "src/{path_str}:{}: redefines the {label} value {value} — import it \
                         from crate::chains::solana::constants",
                        idx + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "chain parameter values must have exactly one owning const, in \
         crate::chains::solana::constants:\n{}",
        violations.join("\n")
    );
}

// =============================================================================
// Error-architecture migration ratchet (see the migration contract).
// =============================================================================

/// Top-level module directories under `src/` that have finished migrating off
/// stringly-typed errors onto their own `error.rs`. Each migration task
/// appends its module here — this list may only ever grow.
const MIGRATED_TO_TYPED_ERRORS: &[&str] = &[
    "net",
    "ohlcvs",
    "swaps",
    "filtering",
    "positions",
    "transactions",
    "trader",
    "wallets",
    "tools",
    "chains",
    "apis",
    "pools",
    "tokens",
    "agent_control",
    "assistant",
    "llm_analysis",
    "telegram",
    "actions",
    "strategies",
    "events",
    "version",
    "config",
    "webserver",
    "reset",
    "secure_storage",
    "run",
    "database",
    "paths",
    "connectivity",
    "account",
    "arguments",
    "logger",
    "process",
];

/// True when `line` declares a two-parameter `Result<_, String>` — a signature
/// that still flattens its error channel to a bare `String` instead of a real
/// type.
///
/// A one-parameter `Result<String>` is deliberately NOT a violation: through a
/// module's own `Result<T>` alias it means the *success* value is a `String`
/// (a signature, a mint address), and `std::result::Result` cannot be spelled
/// with a single parameter at all. Only the error half counts. Angle-bracket
/// depth is tracked so a nested `Result<Vec<String>, String>` is still caught.
fn contains_result_of_string(line: &str) -> bool {
    let mut haystack = line;
    while let Some((_, after)) = haystack.split_once("Result<") {
        let mut depth = 1usize;
        let mut params: Vec<&str> = Vec::new();
        let mut param_start = 0usize;
        let mut closed = false;
        for (idx, ch) in after.char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        params.push(&after[param_start..idx]);
                        closed = true;
                        break;
                    }
                }
                ',' if depth == 1 => {
                    params.push(&after[param_start..idx]);
                    param_start = idx + 1;
                }
                _ => {}
            }
        }
        if closed && params.len() == 2 && params[1].trim() == "String" {
            return true;
        }
        haystack = after;
    }
    false
}

/// **The ratchet.** Once a module is listed in [`MIGRATED_TO_TYPED_ERRORS`],
/// its production code may never again return a bare `Result<_, String>`.
#[test]
fn migrated_modules_never_return_string_errors() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path_str = relative.to_string_lossy();
        let top_level = path_str.split('/').next().unwrap_or("");
        if !MIGRATED_TO_TYPED_ERRORS.contains(&top_level) {
            continue;
        }
        let production = production_text(&contents);
        for (idx, line) in production.lines().enumerate() {
            if contains_result_of_string(line) {
                violations.push(format!("src/{path_str}:{}: {}", idx + 1, line.trim()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "migrated modules must not return Result<_, String> — define the module's own \
         error type in src/<module>/error.rs and return that instead (never re-add the \
         module to MIGRATED_TO_TYPED_ERRORS to make this pass):\n{}",
        violations.join("\n")
    );
}

/// A string-to-error conversion lets any caller launder prose into a typed
/// error, and re-guessing the type from that text is how the boot path kept
/// sniffing messages long after its signatures were typed.
#[test]
fn errors_never_convert_strings_into_typed_errors() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if !relative.starts_with("errors") {
            continue;
        }
        for (index, line) in code_lines(&contents).lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("impl From<String> for")
                || trimmed.starts_with("impl From<&str> for")
            {
                violations.push(format!(
                    "src/{}:{}: {}",
                    relative.display(),
                    index + 1,
                    trimmed
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "src/errors must never implement From<String> or From<&str> for a typed error; \
         callers must construct a named variant instead:\n{}",
        violations.join("\n")
    );
}

/// Exact current violators of [`error_types_live_in_their_module`], scanned
/// against today's tree. Entries are removed as each module moves its error type
/// into an `error.rs`. The list shrinks as that work lands; it grows only when
/// the guard itself is tightened and reveals debt an earlier, looser rule had
/// been hiding — never to make a new violation pass.
///
/// The three `chains/solana` entries are exactly that case: the rule used to
/// exempt everything under `src/chains/` wholesale, so these were never
/// reported. They are misfiled by the same standard that put `ApiError` inside
/// `tokens/types.rs`, and they must reach zero with the rest.
const PENDING_RELOCATION: &[&str] = &[
    "apis/llm/types.rs",
    "ohlcvs/types.rs",
    "rpc/errors.rs",
    "chains/solana/swaps/types.rs",
    "chains/solana/assets/metaplex.rs",
    "chains/solana/pools/reserve_accounts.rs",
];

/// A `pub enum <Something>Error` (or `pub enum Error`) may only be declared in
/// `src/errors/*.rs` or in a file named `error.rs` beside the code it describes
/// — at any depth, so a submodule with a genuinely separate failure domain
/// (`apis/llm/error.rs`, `chains/solana/error.rs`) owns its own vocabulary
/// without a special case. The rule is "errors live in an `error.rs`"; anything
/// else means the type is filed under an unrelated subject, which is how
/// `ApiError` ended up inside `tokens/types.rs`.
#[test]
fn error_types_live_in_their_module() {
    fn is_allowed_location(relative: &Path) -> bool {
        let mut components = relative.components();
        let Some(first) = components.next() else {
            return false;
        };
        let first = first.as_os_str().to_string_lossy();
        if first == "errors" {
            return true;
        }
        if relative.file_name().and_then(|n| n.to_str()) == Some("error.rs") {
            return true;
        }
        false
    }

    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path_str = relative.to_string_lossy().into_owned();
        if is_allowed_location(&relative) {
            continue;
        }
        let production = production_text(&contents);
        for (idx, line) in production.lines().enumerate() {
            let trimmed = line.trim_start();
            let enum_name = trimmed.strip_prefix("pub enum ").map(|rest| {
                rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or("")
            });
            let declares_error_enum = enum_name.is_some_and(|name| name.ends_with("Error"));
            if declares_error_enum {
                if PENDING_RELOCATION.contains(&path_str.as_str()) {
                    continue;
                }
                violations.push(format!(
                    "src/{path_str}:{}: {} — error enums may only live in src/errors/*.rs or in an \
                     error.rs beside the code they describe; move the type into one rather \
                     than adding a path here — PENDING_RELOCATION tracks pre-existing debt \
                     only and must reach zero",
                    idx + 1,
                    trimmed
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "error types must live in their owning module:\n{}",
        violations.join("\n")
    );
}

/// No error enum may declare a catch-all `Generic { ... }`, `Other(String)`
/// or `Unknown(String)` variant — that is the exact escape hatch that let
/// errors collapse back into strings. Scoped to [`MIGRATED_TO_TYPED_ERRORS`]
/// modules only in T0: several `src/errors/` central types (`AccountError`,
/// `NetworkError`, ...) still carry `Generic` because they are kept alive by
/// the builder helpers on `crate::Error`. `src/errors/` joins this guard once
/// those builder helpers — and the `Generic` variants they construct — are
/// deleted in a later task.
#[test]
fn no_catch_all_error_variants() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path_str = relative.to_string_lossy();
        let top_level = path_str.split('/').next().unwrap_or("");
        if !MIGRATED_TO_TYPED_ERRORS.contains(&top_level) {
            continue;
        }
        // Only files that are allowed to declare an error type. `Generic`,
        // `Other(String)` and `Unknown(String)` are perfectly ordinary variant
        // names in a domain enum (a DEX detector, a parse result); this guard
        // is about error vocabularies, so scanning every file would force
        // unrelated domain types to be renamed to satisfy a test.
        if relative.file_name().and_then(|n| n.to_str()) != Some("error.rs") {
            continue;
        }
        let production = production_text(&contents);
        for (idx, line) in production.lines().enumerate() {
            let trimmed = line.trim_start();
            let is_catch_all = trimmed.starts_with("Generic {")
                || trimmed.starts_with("Other(String)")
                || trimmed.starts_with("Unknown(String)");
            if is_catch_all {
                violations.push(format!("src/{path_str}:{}: {}", idx + 1, trimmed));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "no catch-all error variant (Generic/Other(String)/Unknown(String)) is allowed in a \
         migrated module's error type — name the failure instead:\n{}",
        violations.join("\n")
    );
}

/// The only functions in `src/` allowed to return a bare error value instead of
/// constructing one at the site that fails.
///
/// Each entry earns its place by doing real work: `map_llm_error` maps one
/// error enum onto another arm by arm, `classify_quote_failure` inspects a list
/// of router failures to decide which failure it was, and `json_error` adapts a
/// `serde_json::Error` into the foreign `rusqlite::Error` that rusqlite's row
/// mappers are required to return. None of them is a variant in disguise.
/// Functions that MAP an existing failure onto another vocabulary rather than
/// inventing one. Each already holds the failure it is translating, so there is
/// no "failure site" further in for it to be pushed to.
///
/// - `select_quote_failure` picks among per-router errors already built at
///   their own failure sites.
/// - `into_quote_error` converts a captured Jupiter HTTP failure
///   (status + body) into the quote channel. Keeping the status and
///   the raw body structured until this point is the whole reason a quote
///   failure can still be classified by type; rendering a message at the
///   failure site is what the migration is removing.
/// - `raptor_quote_error` maps a captured Raptor HTTP failure (status + body)
///   onto the quote channel — `into_quote_error`'s pattern for another router.
/// - `provider_failure` / `apis_failure` / `assistant_failure` /
///   `analysis_failure` map a provider's own typed error onto the API error
///   envelope, carrying the message id; each already holds the failure it is
///   translating.
/// - `invalid_config` maps a config-walk `Error` onto the API error envelope.
/// - `unknown_path` renders the walk's dead-end context (the available child
///   keys at the point the walk stopped) into the typed `InvalidParameters`
///   variant; that context exists only where the walk stopped.
const ERROR_MAPPING_FUNCTIONS: &[&str] = &[
    "map_llm_error",
    "select_quote_failure",
    "into_quote_error",
    "json_error",
    "raptor_quote_error",
    "provider_failure",
    "apis_failure",
    "assistant_failure",
    "analysis_failure",
    "invalid_config",
    "unknown_path",
];

/// A function whose whole job is to return an error is a variant wearing a
/// function costume: `fn db_not_initialized() -> Error` and
/// `fn database_error(operation, e) -> Error` say nothing that
/// `Error::NotInitialized` and `DatabaseError::Query { operation, message }`
/// do not, while hiding the vocabulary from every reader and diverging from
/// the hundreds of sites that construct the same variants inline.
///
/// The failure mode this prevents is silent: a module gets a private
/// constructor helper, callers use it because it is nearest, and the module's
/// errors stop looking like the rest of the codebase. Construct the variant at
/// the site that fails. A function that genuinely *maps* one error type onto
/// another belongs in [`ERROR_MAPPING_FUNCTIONS`] with a reason.
#[test]
fn errors_are_constructed_at_the_failure_site() {
    fn returned_type(signature: &str) -> Option<&str> {
        let after = signature.split("->").nth(1)?;
        let returned = after
            .trim()
            .trim_end_matches(|c: char| c == '{' || c.is_whitespace());
        (!returned.contains("Result") && returned.ends_with("Error")).then_some(returned)
    }

    fn declared_name(trimmed: &str) -> Option<&str> {
        let after = trimmed.split_once("fn ")?.1;
        Some(
            after
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .unwrap_or(""),
        )
    }

    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path_str = relative.to_string_lossy();
        let production = production_text(&contents);
        for (idx, line) in production.lines().enumerate() {
            let trimmed = line.trim_start();
            let is_fn = trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || (trimmed.starts_with("pub(") && trimmed.contains(") fn "));
            if !is_fn {
                continue;
            }
            let Some(returned) = returned_type(trimmed) else {
                continue;
            };
            let name = declared_name(trimmed).unwrap_or("");
            if ERROR_MAPPING_FUNCTIONS.contains(&name) {
                continue;
            }
            violations.push(format!(
                "src/{path_str}:{}: fn {name}(..) -> {returned} — construct the variant at the \
                 failure site instead of behind a helper",
                idx + 1
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "an error-returning helper hides the module's error vocabulary and diverges from every \
         other construction site — build the variant where the failure happens:\n{}",
        violations.join("\n")
    );
}

/// Identifiers that hold an error, or the text of one, in a route handler.
/// Matching `.contains(..)` against any of them is the boot path's
/// message-sniffing defect moved into the HTTP layer.
const ERROR_TEXT_BINDINGS: &[&str] = &[
    "e",
    "e_str",
    "err",
    "err_str",
    "error",
    "error_msg",
    "error_str",
    "msg",
    "message",
];

/// Directories whose decisions must come from an error's TYPE, never its text.
///
/// `webserver/routes` picks an HTTP status; `swaps` decides whether a token has
/// a market; `positions/operations` and `trader` decide whether to back off,
/// blacklist, or which step of a trade to report as failed. All four were
/// caught guessing from prose, and all four failed silently when the prose
/// changed — the money-path ones in the direction of doing nothing at all.
const TYPED_DECISION_DIRS: &[&str] = &[
    "webserver/routes",
    "swaps",
    "positions/operations",
    "trader",
];

/// `positions/verifier.rs` is out of scope on purpose: its entire job is to
/// translate the RPC's own free-text transaction errors into verification
/// outcomes. That is an anti-corruption boundary, not a decision taken from one
/// of OUR errors, and the same is true of the four wire values below — Solana
/// program error codes and markers that arrive as text from the chain, never
/// from a Rust error type we control.
const EXTERNAL_WIRE_VALUES: &[&str] = &["0x1787", "6023", "insufficient funds", "[PERMANENT]"];

/// Locals that are an error's text: `let x = err.to_string()`, or anything
/// derived from such a local with `.to_lowercase()`/`.to_uppercase()`.
///
/// Naming the bindings alone was not enough — `manual.rs` renamed the error's
/// text to `raw`, lowercased it into `low`, and ran a five-branch prose ladder
/// that this guard could not see.
fn error_text_locals(production: &str) -> Vec<String> {
    let mut locals: Vec<String> = ERROR_TEXT_BINDINGS
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    // Two passes so a local derived from a derived local is also caught.
    for _ in 0..2 {
        for line in production.lines() {
            let trimmed = line.trim_start();
            let Some(rest) = trimmed.strip_prefix("let ") else {
                continue;
            };
            let Some((binding, value)) = rest.split_once('=') else {
                continue;
            };
            let name = binding
                .trim()
                .trim_start_matches("mut ")
                .trim()
                .trim_end_matches(':')
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .to_owned();
            if name.is_empty() || locals.contains(&name) {
                continue;
            }
            let receiver = |call: &str| -> Option<String> {
                let idx = value.find(call)?;
                Some(
                    value[..idx]
                        .trim_end()
                        .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("")
                        .to_owned(),
                )
            };
            let derived = [".to_string()", ".to_lowercase()", ".to_uppercase()"]
                .iter()
                .filter_map(|call| receiver(call))
                .any(|r| locals.contains(&r));
            if derived {
                locals.push(name);
            }
        }
    }
    locals
}

/// **Decisions come from the error's type, never from its prose.**
///
/// Every module error implements `ErrorClass`, so code that re-derives an
/// answer with `msg.contains("not found")` is guessing at a fact the value
/// already carries — and the guess breaks silently when the error vocabulary is
/// reworded. Three separate incidents:
///
/// - `src/wallets/` migrated, `Watch target {id} not found` became `watch
///   target {address} is not being watched`, and three endpoints started
///   answering 500 where they had answered 404.
/// - `classify_quote_failure` rendered a friendly "No swap route available"
///   message, and `get_best_quote_for_opening` then searched that message for
///   the lowercase words "no route" — which it no longer contained. No token
///   was blacklisted for having no market for as long as that stood.
/// - Five copies of `error.contains("Quote")` picked which step of a manual
///   trade to mark failed, and reported a failed SWAP for trades where nothing
///   had been submitted.
///
/// Take the answer from the value: `webserver::utils::status_for(&error)`,
/// `ErrorClass::is_rate_limited()`, the `QuoteError` variant, `TradeStep`.
///
/// Reading a PROVIDER's body — Jupiter's `errorCode`, for instance —
/// is a different thing and stays allowed: that is the boundary whose whole
/// job is translating a wire format into our vocabulary. It is allowed because
/// those router files live outside `TYPED_DECISION_DIRS`.
#[test]
fn decisions_are_never_made_from_error_text() {
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        let path = relative.to_string_lossy().replace('\\', "/");
        if !TYPED_DECISION_DIRS.iter().any(|dir| path.starts_with(dir)) {
            continue;
        }
        let production = production_text(&contents);
        let bindings = error_text_locals(&production);
        for (index, line) in production.lines().enumerate() {
            // A doc comment describing the defect is not the defect.
            if line.trim_start().starts_with("//") {
                continue;
            }
            if EXTERNAL_WIRE_VALUES.iter().any(|v| line.contains(v)) {
                continue;
            }
            let mut haystack = line;
            while let Some((before, after)) = haystack.split_once(".contains(") {
                let receiver = before
                    .trim_end()
                    .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or("")
                    .to_owned();
                if bindings.contains(&receiver) {
                    violations.push(format!(
                        "src/{}:{}: {}",
                        relative.display(),
                        index + 1,
                        line.trim()
                    ));
                    break;
                }
                haystack = after;
            }
        }
    }
    assert!(
        violations.is_empty(),
        "a decision must not be made by reading an error's message — take it from the value: \
         ErrorClass::http_status() (via webserver::utils::status_for), \
         ErrorClass::is_rate_limited(), the QuoteError variant, or TradeStep:\n{}",
        violations.join("\n")
    );
}

// ============================================================================
// Agent-control native MCP boundary (checkpoint 3)
// ============================================================================
//
// The stdio MCP adapter is a bridge client with no local tool execution; the
// live-app bridge is the only agent-control route exempt from the dashboard
// token, and only from that; every bridge handler authenticates a pairing
// credential; and pairing secrets never reach a list/summary type.

fn read_src(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {relative}: {e}"))
}

#[test]
fn mcp_adapter_has_no_local_tool_execution_or_policy() {
    let code = code_lines(&read_src("mcp/mod.rs"));
    for forbidden in [
        "create_tool_registry",
        "InvocationSource",
        "agent_control::decide",
        "::decide(",
        "tool.execute(",
        "permissions::",
    ] {
        assert!(
            !code.contains(forbidden),
            "src/mcp/mod.rs must not reference `{forbidden}` — all tool listing, policy and \
             execution belong to the live app reached through the bridge, never the MCP \
             subprocess"
        );
    }
    // It must actually go through the bridge.
    assert!(
        code.contains("/api/agent-bridge/"),
        "src/mcp/mod.rs must call the live-app bridge"
    );
}

#[test]
fn only_the_bridge_prefix_is_token_exempt_and_only_from_the_token() {
    let middleware = read_src("webserver/middleware.rs");

    // Both gates reference the shared const, never a hardcoded path.
    let uses = middleware.matches("agent_bridge::BRIDGE_PREFIX").count();
    assert!(
        uses >= 2,
        "both `is_security_token_exempt_path` and `auth_gate` must exempt \
         agent_bridge::BRIDGE_PREFIX (found {uses} references)"
    );
    assert!(
        !middleware.contains("\"/api/agent-bridge"),
        "the bridge path must come from routes::agent_bridge::BRIDGE_PREFIX, not a literal"
    );
    // The management API is never exempted.
    assert!(
        !middleware.contains("/api/agent-control"),
        "no agent-control management route may be named in a middleware exemption"
    );

    // The const is exactly the intended prefix.
    let routes = read_src("webserver/routes/agent_bridge/mod.rs");
    assert!(
        routes.contains(r#"BRIDGE_PREFIX: &str = "/api/agent-bridge/""#),
        "BRIDGE_PREFIX must be \"/api/agent-bridge/\""
    );

    // The exemption is only in the two intended gates. `initialization_gate`
    // must NOT list the bridge (pre-init it fails closed with 503).
    let init_gate = middleware
        .split_once("pub async fn initialization_gate")
        .map(|(_, rest)| rest.split("pub async fn").next().unwrap_or(""))
        .unwrap_or("");
    assert!(
        !init_gate.contains("agent_bridge") && !init_gate.contains("agent-bridge"),
        "initialization_gate must not exempt the bridge"
    );
}

#[test]
fn every_bridge_handler_authenticates_a_pairing_credential() {
    let handlers = read_src("webserver/routes/agent_bridge/handlers.rs");
    let handler_count = handlers.matches("pub async fn ").count();
    let auth_calls = handlers.matches("credential(&headers)").count();
    assert!(handler_count >= 4, "expected the four bridge handlers");
    assert_eq!(
        handler_count, auth_calls,
        "every bridge handler must call `credential(&headers)` before doing anything else"
    );
}

#[test]
fn pairing_secret_never_appears_in_a_list_or_summary_type() {
    let pairing = read_src("agent_control/pairing.rs");

    // The only struct allowed to carry the secret field is the one-time
    // creation response.
    let summary = pairing
        .split_once("pub struct PairingSummary")
        .and_then(|(_, r)| r.split_once('}'))
        .map(|(b, _)| b)
        .unwrap_or("");
    for banned in ["secret", "verifier"] {
        assert!(
            !summary.contains(banned),
            "PairingSummary must not expose `{banned}`"
        );
    }

    // The verifier is stored, never serialized: it must not be a field of any
    // `Serialize` type. `AuthedClient` is not Serialize and may hold scope, not
    // the secret.
    let authed = pairing
        .split_once("pub struct AuthedClient")
        .and_then(|(_, r)| r.split_once('}'))
        .map(|(b, _)| b)
        .unwrap_or("");
    assert!(
        !authed.contains("secret") && !authed.contains("verifier"),
        "AuthedClient must not carry the secret or verifier"
    );
}

#[test]
fn agent_approvals_asset_is_fully_registered() {
    let embeds = read_src("webserver/embeds.rs");
    assert!(
        embeds.contains("CORE_AGENT_APPROVALS")
            && embeds.contains("scripts/core/agent_approvals.js"),
        "agent_approvals.js needs an include_str! const in embeds.rs"
    );
    let serving = read_src("webserver/routes/asset_serving/handlers.rs");
    assert!(
        serving.contains(r#""agent_approvals.js" => Some(embeds::CORE_AGENT_APPROVALS)"#),
        "agent_approvals.js needs a match arm in asset_serving handlers"
    );
    let base = read_src("webserver/templates/base.html");
    assert!(
        base.contains("/scripts/core/agent_approvals.js"),
        "agent_approvals.js needs a <script> tag in base.html"
    );
}

#[test]
fn approval_binding_is_unique_and_race_safe() {
    let store = read_src("agent_control/store.rs");
    assert!(
        store.contains("CREATE UNIQUE INDEX IF NOT EXISTS idx_approvals_binding"),
        "the (client_id, tool, args_digest) binding must be a UNIQUE index — a plain \
         index lets a SELECT-then-INSERT race create duplicate approval rows"
    );
    let approvals = read_src("agent_control/approvals.rs");
    assert!(
        approvals.contains("ON CONFLICT(client_id, tool, args_digest) DO NOTHING"),
        "create_or_reuse must insert with ON CONFLICT DO NOTHING and read the row back, \
         not branch on a prior SELECT"
    );
    // Every terminal state is reused; there is no "open a fresh row on failed/expired" path.
    assert!(
        !approvals.contains("Only `expired` and `failed` let a retry open a fresh request"),
        "stale doc: failed/expired approvals are terminal and reused, never replayed"
    );
}

/// The RPC owners: the manager and the Solana client that speak JSON-RPC.
fn is_rpc_owner(relative: &Path) -> bool {
    relative.starts_with("rpc") || relative.starts_with("chains/solana/rpc")
}

/// Files allowed to ask a node to simulate or send a transaction, with the
/// calls each may make. The pre-send gate measures every swap against the
/// packet limit first, types every refusal and settles every send by its
/// signature; a second caller would send a swap nobody measured or settled.
/// The asset owners send plain transfers and burns, never a swap. Only
/// shrinks.
const SIMULATE_OR_SEND_OWNERS: &[(&str, &[&str])] = &[
    (
        "chains/solana/swaps/presend.rs",
        &["simulate_transaction", "send_transaction"],
    ),
    (
        "chains/solana/assets/burn.rs",
        &["send_and_confirm_signed_transaction"],
    ),
    (
        "chains/solana/assets/ata/helpers.rs",
        &["send_and_confirm_signed_transaction"],
    ),
    (
        "chains/solana/assets/transfer.rs",
        &["send_and_confirm_signed_transaction"],
    ),
];

/// RPC client calls that simulate a transaction or hand one to a node.
const SIMULATE_OR_SEND_CALLS: &[&str] = &[
    "simulate_transaction",
    "send_transaction",
    "send_raw_transaction",
    "send_and_confirm_signed_transaction",
];

/// Spellings that reach a node's simulate or send without the client's typed
/// methods: the raw JSON-RPC entry points and the method names themselves.
const RAW_RPC_SPELLINGS: &[&str] = &[
    "execute_raw(",
    "execute_raw_for_provider_kind(",
    "\"sendTransaction\"",
    "\"simulateTransaction\"",
];

#[test]
fn only_the_pre_send_gate_simulates_or_sends_a_swap() {
    let call = regex::Regex::new(&format!(
        r"(?:\.|::)\s*({})\s*\(",
        SIMULATE_OR_SEND_CALLS.join("|")
    ))
    .expect("the call pattern is valid");
    let mut hits: Vec<String> = Vec::new();
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_rpc_owner(&relative) || is_test_support_file(&relative) {
            continue;
        }
        let path = relative.to_string_lossy().into_owned();
        let production = strip_comment_text(&production_text(&contents));
        let allowed = SIMULATE_OR_SEND_OWNERS
            .iter()
            .find(|(owner, _)| *owner == path)
            .map(|(_, calls)| *calls)
            .unwrap_or(&[]);
        for (idx, line) in production.lines().enumerate() {
            for found in call.captures_iter(line) {
                let method = &found[1];
                if allowed.contains(&method) {
                    hits.push(path.clone());
                } else {
                    violations.push(format!("src/{path}:{}: {}", idx + 1, line.trim()));
                }
            }
            for spelling in RAW_RPC_SPELLINGS {
                if line.contains(spelling) {
                    violations.push(format!("src/{path}:{}: {}", idx + 1, line.trim()));
                }
            }
        }
    }
    let stale: Vec<&str> = SIMULATE_OR_SEND_OWNERS
        .iter()
        .map(|(owner, _)| *owner)
        .filter(|owner| !hits.iter().any(|hit| hit.as_str() == *owner))
        .collect();
    assert!(
        violations.is_empty() && stale.is_empty(),
        "a swap transaction is simulated and sent only through \
         chains::solana::swaps::presend (gate / send_and_settle / submit_built_swap), and \
         raw JSON-RPC stays inside rpc/ and chains/solana/rpc/:\n{}\n\
         remove it from the allowlist (entries that no longer simulate or send):\n{}",
        violations.join("\n"),
        stale.join("\n")
    );
}

/// `source` with the contents of string and char literals blanked, so braces
/// and parentheses inside them are not mistaken for code.
fn blank_literals(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        if c == '"' {
            out.push('"');
            index += 1;
            while index < chars.len() && chars[index] != '"' {
                if chars[index] == '\\' {
                    out.push(' ');
                    index += 1;
                }
                if index < chars.len() {
                    out.push(if chars[index] == '\n' { '\n' } else { ' ' });
                    index += 1;
                }
            }
            if index < chars.len() {
                out.push('"');
                index += 1;
            }
        } else if c == '\'' && chars.get(index + 2) == Some(&'\'') {
            out.push_str("' '");
            index += 3;
        } else if c == '\''
            && chars.get(index + 1) == Some(&'\\')
            && chars.get(index + 3) == Some(&'\'')
        {
            out.push_str("'  '");
            index += 4;
        } else {
            out.push(c);
            index += 1;
        }
    }
    out
}

/// The byte range from `open` (an opening `{` or `(`) to its matching close.
fn matching_span(code: &str, open: usize) -> Option<(usize, usize)> {
    let bytes = code.as_bytes();
    let (opener, closer) = match bytes[open] {
        b'{' => (b'{', b'}'),
        b'(' => (b'(', b')'),
        _ => return None,
    };
    let mut depth = 0i32;
    for (offset, byte) in bytes[open..].iter().enumerate() {
        if *byte == opener {
            depth += 1;
        } else if *byte == closer {
            depth -= 1;
            if depth == 0 {
                return Some((open, open + offset));
            }
        }
    }
    None
}

/// Calls that send a trade, directly or through the exit ladder's sender.
const SWAP_SENDS: &[&str] = &[
    "execute_swap_with_fallback(",
    "execute_with_fallback_on(",
    "execute_on(",
    ".execute_swap(",
    ".execute_swap_for_wallet(",
    "execute_quote(",
];

/// Every loop that may send a trade again decides from one reading of the
/// failure, `swaps::failed_swap`: a signature that may still land is
/// reconciled, an unproven failure stops, and only a failure that provably
/// moved nothing is sent again. A loop that asks anything else — or nothing —
/// re-sends a swap that may have landed: a double buy, or a partial exit that
/// sells the same share twice.
#[test]
fn every_loop_that_may_send_a_trade_again_decides_from_failed_swap() {
    let loop_keyword =
        regex::Regex::new(r"\b(?:for|while|loop)\b").expect("the loop pattern is valid");
    let mut sending_loops = 0;
    let mut violations = Vec::new();
    for (relative, contents) in walk_src() {
        if is_test_support_file(&relative) {
            continue;
        }
        let code = blank_literals(&strip_comment_text(&production_text(&contents)));
        for keyword in loop_keyword.find_iter(&code) {
            let Some(open) = code[keyword.end()..].find('{').map(|at| keyword.end() + at) else {
                continue;
            };
            let Some((start, end)) = matching_span(&code, open) else {
                continue;
            };
            let body = &code[start..=end];
            if !SWAP_SENDS.iter().any(|send| body.contains(send)) {
                continue;
            }
            sending_loops += 1;
            if !body.contains("failed_swap(") {
                let line = code[..keyword.start()].lines().count();
                violations.push(format!("src/{}:{line}", relative.display()));
            }
        }
    }
    assert!(
        sending_loops >= 2,
        "the guard must see the fallback chain and the exit ladder ({sending_loops} seen)"
    );
    assert!(
        violations.is_empty(),
        "a loop that sends a trade decides whether to send again only from \
         crate::swaps::failed_swap:\n{}",
        violations.join("\n")
    );

    // Both exits sell through the one ladder rather than a loop of their own.
    for exit in [
        "positions/operations/close.rs",
        "positions/operations/partial_close.rs",
    ] {
        let source =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(exit))
                .expect("the exit source reads");
        assert!(
            source.contains("run_exit_ladder("),
            "src/{exit} must sell through positions::operations::exit_ladder"
        );
    }
}

/// Functions that send a trade and record its outcome in the caller's own
/// future, matched by bare name so every path and import spelling is found.
/// Awaited inline from something that can be dropped — an HTTP handler, an
/// MCP or assistant call, a Telegram callback, a timeout — a trade is
/// cancelled after its send and before its signature is recorded.
const TRADE_RUNNERS: &[&str] = &[
    "execute_trade",
    "execute_buy",
    "execute_buy_managed",
    "execute_sell",
    "open_position_with_size",
    "open_position_direct",
    "close_position_direct",
    "partial_close_position",
    "add_to_position",
    "submit_entry",
    "submit_entry_with_context",
    "quote_and_execute_for_wallet",
    "execute_tool_swap",
    "tool_buy",
    "tool_sell",
    "execute_multi_buy",
    "execute_multi_sell",
];

/// Where a trade runner may be awaited inline, each because nothing that can
/// be dropped awaits it there. Only shrinks.
const INLINE_TRADE_OWNERS: &[(&str, &str)] = &[
    (
        "positions/",
        "the position operations own the swap they send",
    ),
    ("trader/executors/", "the executors are the trade runners"),
    ("trader/manual/", "every public face runs detached"),
    (
        "trader/entry.rs",
        "the entry monitor and copy service own it",
    ),
    ("trader/monitors/", "service loops a request never drops"),
    (
        "trader/copy/service.rs",
        "the copy service loop, never a request",
    ),
    ("tools/", "multi-wallet sessions, started on their own task"),
];

/// Every trade is awaited only where no caller can cancel it: inside its
/// owners, or inside a spawned task. Every other caller reaches a trade
/// through `trader::manual`, whose public faces run detached.
#[test]
fn every_trade_runs_where_no_caller_can_cancel_it() {
    let runner = regex::Regex::new(&format!(
        r"(?:\bfn\s+)?(?:::)?\b({})\s*\(",
        TRADE_RUNNERS.join("|")
    ))
    .expect("the runner pattern is valid");
    let sources: Vec<(PathBuf, String)> = walk_src()
        .into_iter()
        .filter(|(relative, _)| !is_test_support_file(relative))
        .map(|(relative, contents)| {
            let code = blank_literals(&strip_comment_text(&production_text(&contents)));
            (relative, code)
        })
        .collect();
    let mut calls = 0;
    let mut violations = Vec::new();
    for (relative, code) in &sources {
        if INLINE_TRADE_OWNERS
            .iter()
            .any(|(owner, _)| relative.to_string_lossy().starts_with(owner))
        {
            continue;
        }
        let spawned: Vec<(usize, usize)> = code
            .match_indices("tokio::spawn(")
            .filter_map(|(at, call)| matching_span(code, at + call.len() - 1))
            .collect();
        for found in runner.captures_iter(code) {
            let whole = found.get(0).expect("the match exists");
            if whole.as_str().starts_with("fn") {
                continue;
            }
            let name = &found[1];
            // A same-named helper of the caller's own module is not a runner.
            let local_helper = !whole.as_str().starts_with("::")
                && sources.iter().any(|(sibling, text)| {
                    sibling.parent() == relative.parent() && text.contains(&format!("fn {name}("))
                });
            if local_helper {
                continue;
            }
            calls += 1;
            let at = whole.start();
            if !spawned.iter().any(|(start, end)| *start < at && at < *end) {
                let line = code[..at].lines().count();
                violations.push(format!("src/{}:{line}: {name}", relative.display()));
            }
        }
    }
    assert!(
        calls > 0,
        "the guard must see the spawned multi-wallet sessions"
    );
    assert!(
        violations.is_empty(),
        "a trade runner is awaited outside its owners and outside a spawned task; \
         call trader::manual, which runs detached:\n{}",
        violations.join("\n")
    );
}

/// The public faces of `trader::manual` are what every handler, MCP call,
/// approval, assistant tool and Telegram callback awaits. Each runs its trade
/// detached, so dropping the caller never cancels a sent swap.
#[test]
fn every_manual_trade_runs_detached_from_its_caller() {
    let public_trade = regex::Regex::new(r"pub async fn (\w+)\(").expect("valid pattern");
    let mut faces = 0;
    for file in ["trader/manual/api.rs", "trader/manual/force.rs"] {
        let contents =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file))
                .expect("the manual trade source reads");
        let code = blank_literals(&strip_comment_text(&production_text(&contents)));
        for found in public_trade.captures_iter(&code) {
            faces += 1;
            let open = found.get(0).expect("the match exists").end();
            let body_open = open + code[open..].find('{').expect("the function has a body");
            let (start, end) = matching_span(&code, body_open).expect("the body closes");
            assert!(
                code[start..=end].contains("detached("),
                "src/{file}: {} must run its trade through detached",
                &found[1]
            );
        }
    }
    assert!(
        faces >= 5,
        "the guard must see every manual trade ({faces} seen)"
    );

    // A tool call or approval may be a trade: the routes that run one keep
    // their bookkeeping alive past a dropped request.
    for (file, call) in [
        (
            "webserver/routes/agent_bridge/handlers.rs",
            "bridge::call_tool(",
        ),
        (
            "webserver/routes/agent_control/approvals.rs",
            "bridge::execute_approved(",
        ),
    ] {
        let contents =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file))
                .expect("the route source reads");
        let code = blank_literals(&strip_comment_text(&production_text(&contents)));
        let at = code.find(call).expect("the route runs the tool");
        let spawned = code
            .match_indices("tokio::spawn(")
            .filter_map(|(open, spawn)| matching_span(&code, open + spawn.len() - 1))
            .any(|(start, end)| start < at && at < end);
        assert!(spawned, "src/{file}: {call} runs on its own task");
    }
}

/// Every position operation that submits a swap marks its mint as busy before the swap and
/// holds the mark until the swap's row or pending state is recorded: a confirmed swap moves
/// the wallet first, and the wallet-history sync and late-fill attribution read that
/// movement as an outside trade otherwise.
#[test]
fn every_position_swap_marks_its_mint_before_it_is_sent() {
    let operations = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/positions/operations");
    let mut swapping = 0;
    for entry in fs::read_dir(&operations).expect("read the operations directory") {
        let path = entry.expect("dir entry").path();
        let contents = fs::read_to_string(&path).expect("read operation source");
        let code = blank_literals(&strip_comment_text(&production_text(&contents)));
        let Some(swap) = code.find("execute_swap_with_fallback(") else {
            continue;
        };
        swapping += 1;
        let marked = ["mark_swap_in_flight(", "mark_partial_exit_pending("]
            .iter()
            .filter_map(|mark| code.find(mark))
            .any(|mark| mark < swap);
        assert!(
            marked,
            "{}: the swap is sent before its mint is marked busy",
            path.display()
        );
    }
    assert!(
        swapping >= 4,
        "the guard must see every swapping operation ({swapping} seen)"
    );
}
