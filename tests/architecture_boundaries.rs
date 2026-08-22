//! Source-scanning guards for the chain-ownership boundary.
//!
//! Shared domain modules (everything outside `src/chains/solana/`) define
//! chain-neutral intent/models/contracts; `src/chains/solana` implements
//! concrete Solana mechanics; app/service composition selects it through
//! `ChainId` or the router/discovery registries. These tests fail fast if
//! that boundary regresses — e.g. a new file bypassing the
//! `crate::chains::solana` vendor façade, or a shared module re-exporting a
//! chain-specific type as its own public API.
//!
//! Pure source-text scans: no network, no DB, no compilation.

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
    "solana_program",
    "solana_transaction_status",
    "solana_account_decoder",
    "spl_token_2022",
    "spl_associated_token_account",
    "spl_token",
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
/// (`src/pools/types.rs`, `cache.rs`, `api.rs`, `utils.rs`, `database/`) must
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
/// chain-neutral supervisor: running flag, shutdown protocol, event
/// recording, db/cache init) must never import the concrete Solana pool
/// runtime — it selects an implementation via an injected closure instead
/// (see `initialize_pool_components`/`stop_pool_service` and their caller in
/// `src/services/implementations/pools_service.rs`). Regression guard for
/// the leak fixed by moving `PoolAnalyzer`/`PoolDiscovery`/`AccountFetcher`/
/// `PriceCalculator` management to `crate::chains::solana::pools::service`.
#[test]
fn shared_pools_service_never_imports_solana_runtime() {
    let path = "pools/service.rs";
    let contents = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(path))
        .expect("src/pools/service.rs must exist");

    assert!(
        !code_lines(&contents).contains("chains::solana"),
        "src/{path} must not import crate::chains::solana — it orchestrates lifecycle \
         generically and takes the concrete runtime as an injected closure"
    );
}

/// Swap router registry boundary: `src/swaps/registry.rs` must hold only
/// `Arc<dyn SwapRouter>` injected via `set_router_factory`, never construct a
/// concrete Solana router itself. Regression guard for the leak fixed by
/// moving `JupiterRouter`/`GmgnRouter`/`RaydiumRouter` construction to
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
    let code = code_lines(production);

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

fn production_text(contents: &str) -> &str {
    contents.split("#[cfg(test)]").next().unwrap_or(contents)
}

fn is_chain_module(relative: &Path) -> bool {
    relative.starts_with("chains")
}

fn is_composition_root(relative: &Path) -> bool {
    relative.starts_with("run")
}

fn is_test_support_file(relative: &Path) -> bool {
    relative
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "tests.rs")
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
