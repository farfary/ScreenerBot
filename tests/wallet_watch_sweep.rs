// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live sweep of wallet watch across real wallets and RPC providers. Read-only; nothing is signed.
//!
//! Every provider runs in a child process of its own, all of them over the same time window:
//! 1. **Reads.** Per wallet: how the provider answers the signature reads the watch makes (the
//!    largest page, `until` boundaries, a stable newest signature), the reference rate and its
//!    failed share, and the app's decode and classification of the newest successful references,
//!    flagging swap-shaped transactions the classifier does not report as a swap.
//! 2. **Service.** The real watch service, on default configuration, watches every wallet for
//!    `SB_WATCH_SWEEP_SECONDS`; every published activity, pause and status change is recorded.
//! 3. **Ground truth.** Afterwards the window's successful references are read back and decoded,
//!    so the swaps the service missed are counted per wallet.
//!
//! Inputs, read from the environment:
//! - `SB_WATCH_SWEEP_WALLETS`: file with one address per line, an optional label after it.
//! - `SB_WATCH_SWEEP_PROVIDERS`: file with `label url` per line. URLs carry keys and are never
//!   written to a report; provider errors pass through `redact_urls_in`.
//! - `SB_WATCH_SWEEP_OUT`: output directory, one sub-directory per provider label.
//! - `SB_WATCH_SWEEP_SECONDS`: the service window (default 600).
//! - `SB_WATCH_SWEEP_DECODES`: newest successful references decoded per wallet (default 25).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use screenerbot::chains::solana::rpc::{get_rpc_client, RpcClientMethods, SignatureInfo};
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::solana_sdk::signature::Signature;
use screenerbot::chains::solana::transactions::program_ids::detect_router_from_program_id;
use screenerbot::config::with_config;
use screenerbot::transactions::Transaction;
use screenerbot::wallets::watch::runtime::{get_runtime, WalletWatchRuntime};
use screenerbot::wallets::watch::{
    ActivityKind, SwapSide, WalletActivity, DEFAULT_PAGE_BUDGET, PAGE_SIZE,
};
use serde_json::Value;
use tokio::sync::broadcast::error::RecvError;

use crate::common;

const TEST: &str = "every_listed_wallet_is_watched_on_every_provider";
/// The largest page `getSignaturesForAddress` accepts.
const MAX_SIGNATURE_PAGE: usize = 1000;
/// Pages read back per wallet for the ground truth, newest first.
const TRUTH_PAGES: usize = 100;
/// Unobserved successful references decoded per wallet for the ground truth.
const TRUTH_DECODES: usize = 200;
const TRUTH_CONCURRENCY: usize = 6;
const STATUS_EVERY: Duration = Duration::from_secs(10);
const WRAPPED_SOL: &str = "So11111111111111111111111111111111111111112";
/// A native move smaller than this is fees and rent, not a trade leg.
const NATIVE_LEG_LAMPORTS: i128 = 1_000_000;

#[test]
#[ignore = "live: reads mainnet through every provider in SB_WATCH_SWEEP_PROVIDERS for minutes"]
fn every_listed_wallet_is_watched_on_every_provider() {
    let Some(providers) = std::env::var_os("SB_WATCH_SWEEP_PROVIDERS") else {
        eprintln!("SKIP wallet watch sweep: SB_WATCH_SWEEP_PROVIDERS is not set");
        return;
    };
    let providers = read_providers(Path::new(&providers));
    let labels: Vec<&str> = providers.iter().map(|(label, _)| label.as_str()).collect();
    let out = out_dir();
    common::run_isolated_cases_concurrently(
        module_path!(),
        TEST,
        &labels,
        |label| out.join(label).join("app.log"),
        sweep_provider,
    );
}

struct Wallet {
    address: String,
    label: String,
}

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} names a file")))
}

fn env_number(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn out_dir() -> PathBuf {
    env_path("SB_WATCH_SWEEP_OUT")
}

fn content_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

fn read_providers(path: &Path) -> Vec<(String, String)> {
    content_lines(path)
        .into_iter()
        .filter_map(|line| {
            let (label, url) = line.split_once(char::is_whitespace)?;
            Some((label.to_owned(), url.trim().to_owned()))
        })
        .collect()
}

fn read_wallets(path: &Path) -> Vec<Wallet> {
    content_lines(path)
        .into_iter()
        .map(|line| {
            let mut parts = line.splitn(2, char::is_whitespace);
            let address = parts.next().unwrap_or_default().to_owned();
            let label = parts
                .next()
                .map(str::trim)
                .filter(|label| !label.is_empty())
                .map_or_else(|| address[..4.min(address.len())].to_owned(), str::to_owned);
            Wallet { address, label }
        })
        .collect()
}

fn one_line(text: &str) -> String {
    let text = screenerbot::logger::redact_urls_in(text);
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect()
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

/// The value at `percent` of `samples`, by nearest rank.
fn percentile(samples: &[u64], percent: usize) -> Option<u64> {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = (sorted.len() * percent).div_ceil(100);
    sorted.get(rank.saturating_sub(1)).copied()
}

fn show(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| value.to_string())
}

async fn sweep_provider(label: String) {
    let url = read_providers(&env_path("SB_WATCH_SWEEP_PROVIDERS"))
        .into_iter()
        .find(|(listed, _)| *listed == label)
        .map(|(_, url)| url)
        .expect("the case names a listed provider");
    let wallets = read_wallets(&env_path("SB_WATCH_SWEEP_WALLETS"));
    let out = out_dir().join(&label);
    std::fs::create_dir_all(&out).expect("create the provider output directory");
    let window = Duration::from_secs(env_number("SB_WATCH_SWEEP_SECONDS", 600));
    let decodes = env_number("SB_WATCH_SWEEP_DECODES", 25) as usize;

    let _data_dir = common::isolated_env();
    common::set_config(|cfg| cfg.chains.solana.rpc.urls = vec![url]);
    screenerbot::paths::ensure_all_directories().expect("create data directories");
    common::install_chain_runtimes();
    common::configure_own_wallet();
    screenerbot::wallets::initialize()
        .await
        .expect("open the wallets database");
    screenerbot::transactions::database::init_transaction_database()
        .await
        .expect("open the transactions database");
    let runtime = get_runtime().expect("the Solana watch runtime is installed");

    let mut decoded = String::from(
        "provider\twallet\tsignature\tdecode_ms\tkind\tdetail\tfee_payer\tsigner\tswap_shaped\tprograms\n",
    );
    let mut reads = Vec::new();
    for wallet in &wallets {
        let probe = probe_wallet(runtime.as_ref(), &label, wallet, decodes, &mut decoded).await;
        println!(
            "SWEEP reads {label} {}: {}",
            wallet.label,
            probe.row(&label)
        );
        reads.push(probe);
    }
    write(&out.join("decoded.tsv"), &decoded);
    write(
        &out.join("reads.tsv"),
        &(ReadProbe::HEADER.to_owned()
            + &reads
                .iter()
                .map(|probe| probe.row(&label))
                .collect::<Vec<_>>()
                .join("\n")
            + "\n"),
    );
    let calls_after_reads = rpc_calls().await;

    let run = watch_service(&wallets, window).await;
    let calls_after_service = rpc_calls().await;
    let socket = screenerbot::chains::solana::rpc::subscription_metrics();

    let truth = ground_truth(runtime.as_ref(), &wallets, &run).await;
    write_service_reports(&out, &label, &wallets, &run, &truth);

    let mut rpc = String::from("method\tcalls_reads\tcalls_service\terrors_total\n");
    let methods: HashSet<&String> = calls_after_service.0.keys().collect();
    for method in methods {
        let reads = calls_after_reads.0.get(method).copied().unwrap_or(0);
        let total = calls_after_service.0.get(method).copied().unwrap_or(0);
        let errors = calls_after_service.1.get(method).copied().unwrap_or(0);
        let _ = writeln!(rpc, "{method}\t{reads}\t{}\t{errors}", total - reads);
    }
    let _ = writeln!(
        rpc,
        "# websocket notifications={} reconnects={} window_secs={}",
        socket.notifications_received,
        socket.reconnects,
        window.as_secs()
    );
    write(&out.join("rpc.tsv"), &rpc);
    println!("SWEEP done {label}");
}

/// Session calls and errors per RPC method.
async fn rpc_calls() -> (HashMap<String, u64>, HashMap<String, u64>) {
    let stats = get_rpc_client().manager().get_stats().await;
    (
        stats.breakdown.calls_per_method,
        stats.breakdown.errors_per_method,
    )
}

#[derive(Default)]
struct ReadProbe {
    wallet: String,
    address: String,
    page_limit: String,
    references: usize,
    failed: usize,
    span_secs: Option<i64>,
    until_honored: Option<bool>,
    head_regressions: usize,
    decode_ms: Vec<u64>,
    decode_errors: usize,
    signed: usize,
    kinds: BTreeMap<String, usize>,
    swap_shaped_unclassified: usize,
}

impl ReadProbe {
    const HEADER: &'static str = "provider\twallet\taddress\tpage_limit\treferences\tfailed_pct\tspan_secs\trefs_per_min\tsuccessful_per_min\trefs_per_poll\tdefault_poller\tuntil_honored\thead_regressions\tdecoded\tdecode_errors\tdecode_p50_ms\tdecode_p90_ms\tdecode_max_ms\tsigned_pct\tkinds\tswap_shaped_unclassified\n";

    fn per_minute(&self, count: usize) -> Option<f64> {
        self.span_secs
            .filter(|span| *span > 0)
            .map(|span| count as f64 * 60.0 / span as f64)
    }

    /// The default poller reads every reference since its cursor once per poll interval and
    /// pauses the wallet when that range exceeds its page budget.
    fn default_poller(&self) -> (String, String) {
        let poll_secs = with_config(|cfg| cfg.wallet.watch_poll_interval_secs) as f64;
        let budget = (DEFAULT_PAGE_BUDGET * PAGE_SIZE) as f64;
        match self.per_minute(self.references) {
            Some(rate) => {
                let per_poll = rate * poll_secs / 60.0;
                let verdict = if per_poll > budget { "pauses" } else { "fits" };
                (format!("{per_poll:.0}"), verdict.to_owned())
            }
            None => ("-".to_owned(), "-".to_owned()),
        }
    }

    fn row(&self, provider: &str) -> String {
        let decoded = self.decode_ms.len();
        let (per_poll, verdict) = self.default_poller();
        let rate = |count| {
            self.per_minute(count)
                .map_or_else(|| "-".to_owned(), |rate| format!("{rate:.1}"))
        };
        let kinds = self
            .kinds
            .iter()
            .map(|(kind, count)| format!("{kind}={count}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{provider}\t{}\t{}\t{}\t{}\t{:.0}\t{}\t{}\t{}\t{per_poll}\t{verdict}\t{}\t{}\t{decoded}\t{}\t{}\t{}\t{}\t{:.0}\t{kinds}\t{}",
            self.wallet,
            self.address,
            self.page_limit,
            self.references,
            if self.references == 0 { 0.0 } else { self.failed as f64 * 100.0 / self.references as f64 },
            self.span_secs.map_or_else(|| "-".to_owned(), |span| span.to_string()),
            rate(self.references),
            rate(self.references - self.failed),
            self.until_honored.map_or("-", |honored| if honored { "yes" } else { "NO" }),
            self.head_regressions,
            self.decode_errors,
            show(percentile(&self.decode_ms, 50)),
            show(percentile(&self.decode_ms, 90)),
            show(self.decode_ms.iter().max().copied()),
            if decoded == 0 { 0.0 } else { self.signed as f64 * 100.0 / decoded as f64 },
            self.swap_shaped_unclassified,
        )
    }
}

async fn signatures(
    address: &Pubkey,
    limit: usize,
    before: Option<&Signature>,
    until: Option<&Signature>,
) -> Result<Vec<SignatureInfo>, String> {
    get_rpc_client()
        .get_signatures_for_address(address, Some(limit), before, until)
        .await
        .map_err(|error| one_line(&error.to_string()))
}

async fn probe_wallet(
    runtime: &dyn WalletWatchRuntime,
    provider: &str,
    wallet: &Wallet,
    decodes: usize,
    decoded: &mut String,
) -> ReadProbe {
    let pubkey = Pubkey::from_str(&wallet.address).expect("a listed wallet is a Solana address");
    let mut probe = ReadProbe {
        wallet: wallet.label.clone(),
        address: wallet.address.clone(),
        ..ReadProbe::default()
    };
    let page = match signatures(&pubkey, MAX_SIGNATURE_PAGE, None, None).await {
        Ok(page) => {
            probe.page_limit = MAX_SIGNATURE_PAGE.to_string();
            page
        }
        Err(refused) => match signatures(&pubkey, PAGE_SIZE, None, None).await {
            Ok(page) => {
                probe.page_limit = format!("{PAGE_SIZE} ({MAX_SIGNATURE_PAGE} refused: {refused})");
                page
            }
            Err(error) => {
                probe.page_limit = format!("failed: {error}");
                return probe;
            }
        },
    };
    probe.references = page.len();
    probe.failed = page.iter().filter(|item| item.err.is_some()).count();
    probe.span_secs = match (
        page.first().and_then(|item| item.block_time),
        page.last().and_then(|item| item.block_time),
    ) {
        (Some(newest), Some(oldest)) => Some(newest - oldest),
        _ => None,
    };

    // A provider that ignores `until` returns references at or behind the boundary, which the
    // watch would count against the wallet's budget and replay.
    if page.len() >= 60 {
        let boundary = page[49].signature;
        let behind: HashSet<Signature> = page[49..].iter().map(|item| item.signature).collect();
        probe.until_honored = signatures(&pubkey, MAX_SIGNATURE_PAGE, None, Some(&boundary))
            .await
            .ok()
            .map(|after| !after.iter().any(|item| behind.contains(&item.signature)));
    }

    // A load-balanced provider can answer from a node behind the last one.
    let mut newest_slot = 0;
    for _ in 0..3 {
        if let Ok(head) = signatures(&pubkey, 1, None, None).await {
            if let Some(item) = head.first() {
                if item.slot < newest_slot {
                    probe.head_regressions += 1;
                }
                newest_slot = newest_slot.max(item.slot);
            }
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    let successful = page
        .iter()
        .filter(|item| item.err.is_none())
        .take(decodes)
        .map(|item| item.signature.to_string())
        .collect::<Vec<_>>();
    for signature in successful {
        let started = Instant::now();
        let result = runtime
            .decode_transaction(&wallet.address, &signature, false)
            .await;
        let elapsed = started.elapsed().as_millis() as u64;
        match result {
            Ok(transaction) => {
                probe.decode_ms.push(elapsed);
                let reading = Reading::of(runtime, &wallet.address, &transaction);
                probe.signed += usize::from(reading.signer);
                *probe.kinds.entry(reading.kind.clone()).or_default() += 1;
                probe.swap_shaped_unclassified += usize::from(reading.missed_swap());
                let _ = writeln!(
                    decoded,
                    "{provider}\t{}\t{signature}\t{elapsed}\t{}",
                    wallet.label,
                    reading.columns()
                );
            }
            Err(error) => {
                probe.decode_errors += 1;
                let _ = writeln!(
                    decoded,
                    "{provider}\t{}\t{signature}\t{elapsed}\terror\t{}\t-\t-\t-\t-",
                    wallet.label,
                    one_line(&error.to_string())
                );
            }
        }
    }
    probe
}

/// What one decoded transaction did for the subject, as the classifier and the raw balances
/// each see it.
struct Reading {
    kind: String,
    detail: String,
    fee_payer: bool,
    signer: bool,
    swap_shaped: bool,
    programs: Vec<String>,
}

impl Reading {
    fn of(runtime: &dyn WalletWatchRuntime, address: &str, transaction: &Transaction) -> Self {
        let (kind, detail) = match runtime.classify(address, transaction) {
            Some((kind, skip)) => {
                let (name, detail) = kind_name(&kind);
                (name, skip.map_or(detail, |reason| format!("{reason}")))
            }
            None => ("ignored".to_owned(), String::new()),
        };
        let raw = transaction.raw_transaction_data.as_ref();
        let keys = raw.map(account_keys).unwrap_or_default();
        let fee_payer = keys.first().is_some_and(|(key, _)| key == address);
        let signer = keys.iter().any(|(key, signer)| key == address && *signer);
        let swap_shaped = raw.is_some_and(|raw| swap_shaped(raw, address, &keys));
        let programs = raw.map(top_level_programs).unwrap_or_default();
        Self {
            kind,
            detail,
            fee_payer,
            signer,
            swap_shaped,
            programs,
        }
    }

    /// A trade the balances show that the classifier did not report as a swap.
    fn missed_swap(&self) -> bool {
        self.swap_shaped && !self.kind.contains("swap")
    }

    fn columns(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            self.kind,
            one_line(&self.detail),
            self.fee_payer,
            self.signer,
            self.swap_shaped,
            self.programs.join(",")
        )
    }
}

fn kind_name(kind: &ActivityKind) -> (String, String) {
    let side = |side: &SwapSide| match side {
        SwapSide::Buy => "buy",
        SwapSide::Sell => "sell",
    };
    match kind {
        ActivityKind::Swap {
            side: swap_side,
            mint,
            venue,
            ..
        } => (
            format!("swap-{}", side(swap_side)),
            format!("{mint} via {}", venue.as_deref().unwrap_or("-")),
        ),
        ActivityKind::StableSwap {
            side: swap_side,
            mint,
            venue,
            ..
        } => (
            format!("stable-swap-{}", side(swap_side)),
            format!("{mint} via {}", venue.as_deref().unwrap_or("-")),
        ),
        ActivityKind::UnresolvedSwap { reason, venue, .. } => (
            "unresolved-swap".to_owned(),
            format!("{reason:?} via {}", venue.as_deref().unwrap_or("-")),
        ),
        ActivityKind::Transfer { .. } => ("transfer".to_owned(), String::new()),
        ActivityKind::Other => ("other".to_owned(), String::new()),
    }
}

/// `(address, signed)` for every account key of a `jsonParsed` transaction.
fn account_keys(raw: &Value) -> Vec<(String, bool)> {
    raw.pointer("/transaction/message/accountKeys")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|key| match key {
            Value::String(address) => Some((address.clone(), false)),
            Value::Object(fields) => Some((
                fields.get("pubkey")?.as_str()?.to_owned(),
                fields
                    .get("signer")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )),
            _ => None,
        })
        .collect()
}

fn top_level_programs(raw: &Value) -> Vec<String> {
    let mut programs = Vec::new();
    for instruction in raw
        .pointer("/transaction/message/instructions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(program) = instruction.get("programId").and_then(Value::as_str) else {
            continue;
        };
        let named = match detect_router_from_program_id(program) {
            Some(router) => format!("{program}({router})"),
            None => program.to_owned(),
        };
        if !programs.contains(&named) {
            programs.push(named);
        }
    }
    programs
}

/// Whether the subject's own balances moved like a trade: a token one way and native SOL (or
/// another token) the other way.
fn swap_shaped(raw: &Value, address: &str, keys: &[(String, bool)]) -> bool {
    let Some(index) = keys.iter().position(|(key, _)| key == address) else {
        return false;
    };
    let lamports = |field: &str| {
        raw.pointer(&format!("/meta/{field}/{index}"))
            .and_then(Value::as_i64)
            .map(i128::from)
    };
    let fee = raw
        .pointer("/meta/fee")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let mut native = match (lamports("preBalances"), lamports("postBalances")) {
        (Some(pre), Some(post)) => post - pre + if index == 0 { i128::from(fee) } else { 0 },
        _ => 0,
    };
    let mut tokens = HashMap::<String, i128>::new();
    for (field, sign) in [("preTokenBalances", -1), ("postTokenBalances", 1)] {
        for balance in raw
            .pointer(&format!("/meta/{field}"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if balance.get("owner").and_then(Value::as_str) != Some(address) {
                continue;
            }
            let (Some(mint), Some(amount)) = (
                balance.get("mint").and_then(Value::as_str),
                balance
                    .pointer("/uiTokenAmount/amount")
                    .and_then(Value::as_str)
                    .and_then(|amount| amount.parse::<i128>().ok()),
            ) else {
                continue;
            };
            *tokens.entry(mint.to_owned()).or_default() += sign * amount;
        }
    }
    native += tokens.remove(WRAPPED_SOL).unwrap_or(0);
    let moved: Vec<i128> = tokens.into_values().filter(|delta| *delta != 0).collect();
    let native_leg = native.abs() >= NATIVE_LEG_LAMPORTS;
    match moved.as_slice() {
        [token] => native_leg && token.signum() != native.signum(),
        [first, second] => first.signum() != second.signum(),
        _ => false,
    }
}

struct ServiceRun {
    started: DateTime<Utc>,
    finished: DateTime<Utc>,
    activities: Vec<WalletActivity>,
    lagged: u64,
    timeline: Vec<String>,
    final_state: HashMap<String, String>,
}

impl ServiceRun {
    fn observed(&self, address: &str) -> HashSet<&str> {
        self.activities
            .iter()
            .filter(|activity| activity.subject == address)
            .map(|activity| activity.signature.as_str())
            .collect()
    }
}

/// Run the real watch service over every wallet for `window`, recording what it publishes and
/// every change of each target's state.
async fn watch_service(wallets: &[Wallet], window: Duration) -> ServiceRun {
    use screenerbot::wallets::watch;

    screenerbot::global::mark_transactions_system_ready();
    let mut activity = watch::subscribe_activity();
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let handle = watch::start(Arc::clone(&shutdown))
        .await
        .expect("start the watch service");
    let started = Utc::now();
    let clock = Instant::now();
    let label_of: HashMap<&str, &str> = wallets
        .iter()
        .map(|wallet| (wallet.address.as_str(), wallet.label.as_str()))
        .collect();
    let mut timeline = Vec::new();
    for wallet in wallets {
        if let Err(error) = watch::add_target(&wallet.address, Some(&wallet.label)).await {
            timeline.push(format!(
                "+0s {} not added: {}",
                wallet.label,
                one_line(&error.to_string())
            ));
        }
    }

    let mut activities = Vec::new();
    let mut lagged = 0;
    let mut states: HashMap<String, String> = HashMap::new();
    let mut status_tick = tokio::time::interval(STATUS_EVERY);
    let end = tokio::time::sleep(window);
    tokio::pin!(end);
    loop {
        tokio::select! {
            _ = &mut end => break,
            received = activity.recv() => match received {
                Ok(item) => activities.push(item),
                Err(RecvError::Lagged(missed)) => lagged += missed,
                Err(RecvError::Closed) => break,
            },
            _ = status_tick.tick() => {
                for target in watch::list_targets().await.unwrap_or_default() {
                    let state = target_state(&target).await;
                    if states.get(&target.address) != Some(&state) {
                        timeline.push(format!(
                            "+{}s {} {state}",
                            clock.elapsed().as_secs(),
                            label_of.get(target.address.as_str()).unwrap_or(&"?")
                        ));
                        states.insert(target.address.clone(), state);
                    }
                }
            }
        }
    }
    let finished = Utc::now();
    shutdown.notify_waiters();
    shutdown.notify_one();
    let _ = tokio::time::timeout(Duration::from_secs(15), handle).await;
    ServiceRun {
        started,
        finished,
        activities,
        lagged,
        timeline,
        final_state: states,
    }
}

async fn target_state(target: &screenerbot::wallets::watch::WatchTarget) -> String {
    if !target.enabled {
        return format!("paused {:?}", target.disable_reason);
    }
    let Some(id) = target.id else {
        return "unsaved".to_owned();
    };
    match screenerbot::wallets::watch::get_status(id).await {
        Ok(status) => format!(
            "{:?}{} subscribed={}{}",
            status.mode,
            if status.catching_up {
                " catching-up"
            } else {
                ""
            },
            status.subscribed,
            status
                .last_error
                .map(|error| format!(" error={}", error.id))
                .unwrap_or_default()
        ),
        Err(error) => format!("status unreadable: {}", one_line(&error.to_string())),
    }
}

#[derive(Default)]
struct Truth {
    references: usize,
    successful: usize,
    truncated: bool,
    decoded: usize,
    decode_errors: usize,
    swaps: usize,
    missed: Vec<String>,
}

/// Read back each wallet's references inside the service window and decode the successful
/// ones the service did not publish, so its missed swaps are counted.
async fn ground_truth(
    runtime: &dyn WalletWatchRuntime,
    wallets: &[Wallet],
    run: &ServiceRun,
) -> HashMap<String, Truth> {
    let (from, to) = (run.started.timestamp(), run.finished.timestamp());
    let mut truth = HashMap::new();
    for wallet in wallets {
        let pubkey =
            Pubkey::from_str(&wallet.address).expect("a listed wallet is a Solana address");
        let observed = run.observed(&wallet.address);
        let mut result = Truth::default();
        let mut unobserved = Vec::new();
        let mut before: Option<Signature> = None;
        let mut pages = 0;
        'pages: loop {
            let Ok(page) = signatures(&pubkey, MAX_SIGNATURE_PAGE, before.as_ref(), None).await
            else {
                result.truncated = true;
                break;
            };
            pages += 1;
            for item in &page {
                let Some(time) = item.block_time else {
                    continue;
                };
                if time < from {
                    break 'pages;
                }
                if time > to {
                    continue;
                }
                result.references += 1;
                if item.err.is_none() {
                    result.successful += 1;
                    let signature = item.signature.to_string();
                    if !observed.contains(signature.as_str()) {
                        unobserved.push(signature);
                    }
                }
            }
            before = page.last().map(|item| item.signature);
            if page.len() < MAX_SIGNATURE_PAGE || before.is_none() {
                break;
            }
            if pages >= TRUTH_PAGES {
                result.truncated = true;
                break;
            }
        }
        if unobserved.len() > TRUTH_DECODES {
            result.truncated = true;
            unobserved.truncate(TRUTH_DECODES);
        }
        let address = wallet.address.as_str();
        let decoded = stream::iter(unobserved)
            .map(|signature| async move {
                let result = runtime.decode_transaction(address, &signature, false).await;
                (signature, result)
            })
            .buffer_unordered(TRUTH_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;
        for (signature, decoded) in decoded {
            match decoded {
                Ok(transaction) => {
                    result.decoded += 1;
                    if let Some((kind, _)) = runtime.classify(address, &transaction) {
                        if matches!(
                            kind,
                            ActivityKind::Swap { .. } | ActivityKind::StableSwap { .. }
                        ) {
                            result.missed.push(signature);
                        }
                    }
                }
                Err(_) => result.decode_errors += 1,
            }
        }
        result.swaps = result.missed.len()
            + run
                .activities
                .iter()
                .filter(|activity| activity.subject == wallet.address && is_swap(&activity.kind))
                .count();
        truth.insert(wallet.address.clone(), result);
    }
    truth
}

fn is_swap(kind: &ActivityKind) -> bool {
    matches!(
        kind,
        ActivityKind::Swap { .. } | ActivityKind::StableSwap { .. }
    )
}

fn write_service_reports(
    out: &Path,
    provider: &str,
    wallets: &[Wallet],
    run: &ServiceRun,
    truth: &HashMap<String, Truth>,
) {
    let arrival_limit = with_config(|cfg| cfg.copy_trading.max_arrival_distance_ms);
    let mut activity_rows = String::from(
        "provider\twallet\tsignature\tkind\tbackfill\tblock_time\tdetected_ms_after_block\tdecoded_ms_after_block\n",
    );
    let mut service_rows = String::from(
        "provider\twallet\tactivities\tswaps_published\tbackfill\tdetected_p50_ms\tdetected_p90_ms\tdecoded_p50_ms\tdecoded_p90_ms\tdecoded_max_ms\tover_arrival_limit\ttruth_references\ttruth_successful\ttruth_swaps\tmissed_swaps\ttruth_truncated\tfinal_state\n",
    );
    for wallet in wallets {
        let mine: Vec<&WalletActivity> = run
            .activities
            .iter()
            .filter(|activity| activity.subject == wallet.address)
            .collect();
        let mut detected = Vec::new();
        let mut decoded = Vec::new();
        for activity in &mine {
            let block_ms = activity.block_time.map(|time| time * 1000);
            let after = |at: DateTime<Utc>| {
                block_ms.and_then(|block| u64::try_from(at.timestamp_millis() - block).ok())
            };
            let (detected_after, decoded_after) =
                (after(activity.detected_at), after(activity.decoded_at));
            if !activity.backfill && is_swap(&activity.kind) {
                detected.extend(detected_after);
                decoded.extend(decoded_after);
            }
            let _ = writeln!(
                activity_rows,
                "{provider}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                wallet.label,
                activity.signature,
                kind_name(&activity.kind).0,
                activity.backfill,
                show(
                    activity
                        .block_time
                        .and_then(|time| u64::try_from(time).ok())
                ),
                show(detected_after),
                show(decoded_after)
            );
        }
        let swaps = mine
            .iter()
            .filter(|activity| is_swap(&activity.kind))
            .count();
        let backfill = mine.iter().filter(|activity| activity.backfill).count();
        let late = decoded.iter().filter(|ms| **ms > arrival_limit).count();
        let empty = Truth::default();
        let wallet_truth = truth.get(&wallet.address).unwrap_or(&empty);
        let _ = writeln!(
            service_rows,
            "{provider}\t{}\t{}\t{swaps}\t{backfill}\t{}\t{}\t{}\t{}\t{}\t{late}\t{}\t{}\t{}\t{}\t{}\t{}",
            wallet.label,
            mine.len(),
            show(percentile(&detected, 50)),
            show(percentile(&detected, 90)),
            show(percentile(&decoded, 50)),
            show(percentile(&decoded, 90)),
            show(decoded.iter().max().copied()),
            wallet_truth.references,
            wallet_truth.successful,
            wallet_truth.swaps,
            wallet_truth.missed.len(),
            wallet_truth.truncated,
            run.final_state
                .get(&wallet.address)
                .map_or("never reported", String::as_str),
        );
        println!(
            "SWEEP service {provider} {}: published {} swaps of {} found, missed {}, final {}",
            wallet.label,
            swaps,
            wallet_truth.swaps,
            wallet_truth.missed.len(),
            run.final_state
                .get(&wallet.address)
                .map_or("never reported", String::as_str)
        );
    }
    write(&out.join("activity.tsv"), &activity_rows);
    write(&out.join("service.tsv"), &service_rows);
    let mut timeline = run.timeline.join("\n");
    let _ = write!(
        timeline,
        "\n# activities={} lagged={} window={}..{}\n",
        run.activities.len(),
        run.lagged,
        run.started.to_rfc3339(),
        run.finished.to_rfc3339()
    );
    write(&out.join("timeline.txt"), &timeline);
}
