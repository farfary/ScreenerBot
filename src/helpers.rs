#![allow(warnings)]
use crate::prelude::*;

use std::{ fs, str::FromStr };
use chrono::{ DateTime, Utc };
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_sdk::{ pubkey::Pubkey, signature::{ Keypair, Signer } };
use solana_account_decoder::UiAccountData;
use once_cell::sync::Lazy;
use bs58;
use anyhow::{ anyhow, Result, bail };
use spl_token::state::{ Mint, Account };
use solana_program::program_pack::Pack;
use std::collections::HashMap;
use std::path::Path;
use std::io::{ Write };
use reqwest::blocking::Client;
use serde_json::Value;
use std::{ fs::{ File }, time::{ Instant } };
use rayon::prelude::*;
use std::{ sync::RwLock };
use std::time::{ SystemTime, UNIX_EPOCH };
use solana_client::{ rpc_config::RpcTransactionConfig };
use solana_sdk::{ commitment_config::CommitmentConfig, signature::Signature };
use solana_transaction_status::{
    option_serializer::OptionSerializer,
    UiTransactionEncoding,
    UiTransactionTokenBalance,
};
use std::sync::atomic::{ AtomicBool, Ordering };
use tokio::time::{ sleep, Duration };
use std::collections::{ VecDeque };
use tokio::task;
use futures::FutureExt;
use std::collections::HashSet;
use std::fs::{ OpenOptions };
use std::io::{ BufRead, BufReader };
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub address: String,
    pub source: String, // "dexscreener" or "geckoterminal"
    pub name: Option<String>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h_usd: Option<f64>,
    pub tx_count_24h: Option<u64>,
}

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

// ────────────── CONFIG & RPC ──────────────
#[derive(Debug, Deserialize)]
pub struct Configs {
    pub main_wallet_private: String,
    pub rpc_url: String,
}

pub static CONFIGS: Lazy<Configs> = Lazy::new(|| {
    let raw = fs::read_to_string("configs.json").expect("❌ Failed to read configs.json");
    serde_json::from_str(&raw).expect("❌ Failed to parse configs.json")
});

pub static RPC: Lazy<RpcClient> = Lazy::new(|| { RpcClient::new(CONFIGS.rpc_url.clone()) });

// ────────────── SCAN HELPER ──────────────
/// Scans all token accounts under `owner` for a given SPL program,
/// returning `(mint_address, raw_amount_u64)` for each ATA.
fn scan_program_tokens(owner: &Pubkey, program_id: &Pubkey) -> Vec<(String, u64)> {
    let filter = TokenAccountsFilter::ProgramId(*program_id);
    let accounts = RPC.get_token_accounts_by_owner(owner, filter).expect("❌ RPC error");

    let mut result = Vec::new();
    for keyed in accounts {
        let acc = keyed.account;
        if let UiAccountData::Json(parsed) = acc.data {
            let info = &parsed.parsed["info"];
            let mint = info["mint"].as_str().expect("Missing mint").to_string();
            let raw_amount_str = info["tokenAmount"]["amount"]
                .as_str()
                .expect("Missing raw amount");
            let raw_amount = raw_amount_str.parse::<u64>().expect("Invalid raw amount");
            result.push((mint, raw_amount));
        }
    }
    result
}

// ────────────── PUBLIC API ──────────────
/// Returns _all_ SPL (and Token-2022) balances for the main wallet,
/// as a list of `(mint_address, raw_amount)`.
pub fn get_all_tokens() -> Vec<(String, u64)> {
    // decode keypair
    let secret_bytes = bs58
        ::decode(&CONFIGS.main_wallet_private)
        .into_vec()
        .expect("❌ Invalid base58 key");
    let keypair = Keypair::try_from(&secret_bytes[..]).expect("❌ Invalid keypair bytes");
    let owner = keypair.pubkey();

    let mut tokens = Vec::new();

    // standard SPL
    tokens.extend(scan_program_tokens(&owner, &spl_token::id()));

    // Token-2022
    let token2022 = Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap();
    tokens.extend(scan_program_tokens(&owner, &token2022));

    tokens
}

/// Returns the total raw token amount for `token_mint`,
/// summing across all ATAs (SPL + Token-2022).
pub fn get_token_amount(token_mint: &str) -> u64 {
    let secret_bytes = bs58
        ::decode(&CONFIGS.main_wallet_private)
        .into_vec()
        .expect("❌ Invalid base58 key");
    let keypair = Keypair::try_from(&secret_bytes[..]).expect("❌ Invalid keypair bytes");
    let owner = keypair.pubkey();
    let mint = Pubkey::from_str(token_mint).expect("Invalid mint");

    let programs = vec![
        spl_token::id(),
        Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap()
    ];

    let mut total = 0;
    for program_id in programs {
        let filter = TokenAccountsFilter::Mint(mint);
        let accounts = RPC.get_token_accounts_by_owner(&owner, filter).expect("❌ RPC error");
        for keyed in accounts {
            if let UiAccountData::Json(parsed) = keyed.account.data {
                let amt_str = &parsed.parsed["info"]["tokenAmount"]["amount"];
                if let Some(s) = amt_str.as_str() {
                    total += s.parse::<u64>().unwrap_or(0);
                }
            }
        }
    }

    println!("🔢 On-chain balance for {}: {}", token_mint, total);
    total
}

/// Returns the largest single-ATA raw amount for `token_mint`.
pub fn get_biggest_token_amount(token_mint: &str) -> u64 {
    let secret_bytes = bs58
        ::decode(&CONFIGS.main_wallet_private)
        .into_vec()
        .expect("❌ Invalid base58 key");
    let keypair = Keypair::try_from(&secret_bytes[..]).expect("❌ Invalid keypair bytes");
    let owner = keypair.pubkey();
    let mint = Pubkey::from_str(token_mint).expect("Invalid mint");

    let programs = vec![
        spl_token::id(),
        Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap()
    ];

    let mut biggest = 0;
    for program_id in programs {
        let filter = TokenAccountsFilter::Mint(mint);
        let accounts = RPC.get_token_accounts_by_owner(&owner, filter).expect("❌ RPC error");
        for keyed in accounts {
            if let UiAccountData::Json(parsed) = keyed.account.data {
                let amt_str = &parsed.parsed["info"]["tokenAmount"]["amount"];
                if let Some(s) = amt_str.as_str() {
                    let v = s.parse::<u64>().unwrap_or(0);
                    if v > biggest {
                        biggest = v;
                    }
                }
            }
        }
    }

    println!("🔢 Biggest single ATA for {}: {}", token_mint, biggest);
    biggest
}

pub async fn print_open_positions() {
    use comfy_table::{ Table, presets::UTF8_FULL };

    let positions_guard = OPEN_POSITIONS.read().await;
    let closed_guard = RECENT_CLOSED_POSITIONS.read().await;

    // ── quick stats ───────────────────────────
    let open_count = positions_guard.len();
    let mut total_unrealized_sol = 0.0;

    // ── prepare open-positions table ──────────
    let mut positions_vec: Vec<_> = positions_guard.iter().collect();
    positions_vec.sort_by_key(|(_, pos)| pos.open_time);

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header([
            "Mint",
            "Entry Price",
            "Current Price",
            "Profit %",
            "Peak Price",
            "DCA Count",
            "Tokens",
            "SOL Spent",
            "Open Time",
        ]);

    for (mint, pos) in positions_vec {
        let current_price = PRICE_CACHE.read()
            .unwrap()
            .get(mint)
            .map(|&(_ts, price)| price)
            .unwrap_or(0.0);

        let profit_pct = if pos.entry_price > 0.0 && current_price > 0.0 {
            ((current_price - pos.entry_price) / pos.entry_price) * 100.0
        } else {
            0.0
        };

        total_unrealized_sol += current_price * pos.token_amount - pos.sol_spent;

        table.add_row([
            mint.clone(),
            format!("{:.12}", pos.entry_price),
            format!("{:.12}", current_price),
            format!("{:+.2}%", profit_pct),
            format!("{:.12}", pos.peak_price),
            pos.dca_count.to_string(),
            format!("{:.9}", pos.token_amount),
            format!("{:.9}", pos.sol_spent),
            format_duration_ago(pos.open_time),
        ]);
    }

    println!(
        "\n📂 [Open Positions] — count: {} | unrealized P/L: {:+.3} SOL\n{}\n",
        open_count,
        total_unrealized_sol,
        table
    );

    // ── recent-closed table (WITH EXACT PROFIT SOL) ───────────
    if !closed_guard.is_empty() {
        let mut closed_vec: Vec<_> = closed_guard.values().cloned().collect();
        closed_vec.sort_by_key(|pos| pos.close_time.unwrap_or(pos.open_time));

        let mut table_closed = Table::new();
        table_closed.load_preset(UTF8_FULL).set_header([
            "Mint",
            "Entry Price",
            "Close Price",
            "Profit %",
            "Profit SOL", // NEW COLUMN
            "Peak Price",
            "Tokens",
            "SOL Spent",
            "SOL Received",
            "Open Time",
            "Close Time",
        ]);

        for pos in closed_vec {
            let close_price = if pos.token_amount > 0.0 {
                pos.sol_received / pos.token_amount
            } else {
                0.0
            };
            let profit_pct = if pos.sol_spent > 0.0 {
                ((pos.sol_received - pos.sol_spent) / pos.sol_spent) * 100.0
            } else {
                0.0
            };

            let profit_sol = pos.sol_received - pos.sol_spent;

            table_closed.add_row([
                "(closed)".into(),
                format!("{:.9}", pos.entry_price),
                format!("{:.9}", close_price),
                format!("{:+.2}%", profit_pct),
                format!("{:+.9}", profit_sol), // EXACT PROFIT SOL
                format!("{:.9}", pos.peak_price),
                format!("{:.9}", pos.token_amount),
                format!("{:.9}", pos.sol_spent),
                format!("{:.9}", pos.sol_received),
                format_duration_ago(pos.open_time),
                pos.close_time.map(format_duration_ago).unwrap_or_else(|| "-".into()),
            ]);
        }

        println!("📁 [Recent Closed Positions]\n{}\n", table_closed);
    }
}

/// Return the `decimals` of a mint account on–chain, with disk cache.
/// Cache is in ".token_decimals_cache.json"
pub fn get_token_decimals(rpc: &RpcClient, mint: &Pubkey) -> Result<u8> {
    let cache_path = ".token_decimals_cache.json";
    let mut cache: HashMap<String, u8> = if Path::new(cache_path).exists() {
        fs::read_to_string(cache_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mint_str = mint.to_string();
    if let Some(&decimals) = cache.get(&mint_str) {
        return Ok(decimals);
    }

    let acc = rpc.get_account(mint)?;
    if acc.owner != spl_token::id() {
        return Err(anyhow!("account {mint} is not an SPL-Token mint"));
    }
    let mint_state = Mint::unpack(&acc.data).map_err(|e| anyhow!("failed to unpack Mint: {e}"))?;
    let decimals = mint_state.decimals;

    cache.insert(mint_str, decimals);

    // Write cache (ignore errors)
    let _ = fs::File::create(cache_path).and_then(|mut f| {
        let s = serde_json::to_string(&cache).unwrap();
        f.write_all(s.as_bytes())
    });
    println!("🔍 Fetching decimals for mint: {mint}");
    println!("🔍 Account owner: {}", acc.owner);
    println!("🔍 Mint state decimals: {decimals}");
    Ok(decimals)
}

/// Return the `mint` address of a token account, with disk cache.
/// Cache is in ".token_account_mint_cache.json"
pub fn get_token_account_mint(rpc: &RpcClient, token_account: &Pubkey) -> Result<Pubkey> {
    let cache_path = ".token_account_mint_cache.json";
    let mut cache: HashMap<String, String> = if Path::new(cache_path).exists() {
        fs::read_to_string(cache_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let acct_str = token_account.to_string();
    if let Some(mint_str) = cache.get(&acct_str) {
        return Pubkey::from_str(mint_str).map_err(|e| anyhow!("Invalid cached mint: {e}"));
    }

    let acc = rpc.get_account(token_account)?;
    if acc.owner != spl_token::id() {
        return Err(anyhow!("account {token_account} is not an SPL-Token account"));
    }
    let account_state = Account::unpack(&acc.data).map_err(|e|
        anyhow!("failed to unpack Token Account: {e}")
    )?;
    let mint = account_state.mint;

    cache.insert(acct_str, mint.to_string());
    let _ = fs::File::create(cache_path).and_then(|mut f| {
        let s = serde_json::to_string(&cache).unwrap();
        f.write_all(s.as_bytes())
    });

    Ok(mint)
}

// Cache: token_mint -> (timestamp_secs, price)
pub static PRICE_CACHE: Lazy<RwLock<HashMap<String, (u64, f64)>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

/// Pull every *Solana* `pairAddress` for the given token mint from both DexScreener and GeckoTerminal, with 2h cache.
pub fn fetch_solana_pairs(token_mint: &str) -> Result<Vec<Pubkey>> {
    let cache_path = ".solana_pairs_cache.json";
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let expire_secs = 2 * 3600;

    // {mint: [timestamp, [array of pair pubkeys as string]]}
    let mut cache: HashMap<String, (u64, Vec<String>)> = if Path::new(cache_path).exists() {
        fs::read_to_string(cache_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Check cache
    if let Some((ts, pairs)) = cache.get(token_mint) {
        if now < *ts + expire_secs {
            return Ok(
                pairs
                    .iter()
                    .filter_map(|s| Pubkey::from_str(s).ok())
                    .collect()
            );
        }
    }

    // Fetch fresh from both sources
    println!("🔄 Fetching pools from both DexScreener and GeckoTerminal...");
    let pools = fetch_combined_pools(token_mint)?;

    if pools.is_empty() {
        bail!("No Solana pools found for mint {} from any source", token_mint);
    }

    let addresses: Vec<String> = pools
        .iter()
        .map(|p| p.address.clone())
        .collect();

    // Update cache
    cache.insert(token_mint.to_string(), (now, addresses.clone()));
    let _ = fs::File::create(cache_path).and_then(|mut f| {
        let s = serde_json::to_string(&cache).unwrap();
        f.write_all(s.as_bytes())
    });

    // Return as Vec<Pubkey>
    Ok(
        addresses
            .iter()
            .filter_map(|s| Pubkey::from_str(s).ok())
            .collect()
    )
}

/// Return the effective price you actually paid (SOL per token)
/// and print full debug info.
///
/// Change `SWAP_FEE_SOL` below if you want a different hard-coded fee.
pub fn effective_swap_price(
    rpc: &RpcClient,
    tx_sig_str: &str,
    wallet: &Pubkey,
    token_mint: &Pubkey,
    lamports_in: u64
) -> Result<f64> {
    // ───── hard-coded fee you want to deduct ─────
    const SWAP_FEE_SOL: f64 = 0.000005; // <── tweak here
    const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;
    let fee_lamports = (SWAP_FEE_SOL * LAMPORTS_PER_SOL) as u64;
    // ---------------------------------------------

    println!("🔍 fetching tx {tx_sig_str}");
    let sig = Signature::from_str(tx_sig_str)?;

    let tx = rpc.get_transaction_with_config(&sig, RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    })?;
    let meta = tx.transaction.meta.ok_or_else(|| anyhow!("transaction meta missing"))?;

    // ---- helpers ----------------------------------------------------------
    fn opt_ref<'a, T>(os: &'a OptionSerializer<T>) -> Option<&'a T> {
        Option::<&T>::from(os.as_ref())
    }
    fn balance(
        list: &OptionSerializer<Vec<UiTransactionTokenBalance>>,
        owner: &Pubkey,
        mint: &Pubkey
    ) -> f64 {
        opt_ref(list)
            .and_then(|v| {
                v.iter().find(|b| {
                    b.mint == mint.to_string() && opt_ref(&b.owner) == Some(&owner.to_string())
                })
            })
            .and_then(|b| b.ui_token_amount.ui_amount)
            .unwrap_or(0.0)
    }
    // -----------------------------------------------------------------------

    let pre = balance(&meta.pre_token_balances, wallet, token_mint);
    let post = balance(&meta.post_token_balances, wallet, token_mint);
    let delta = post - pre;

    println!("🧮 balances  → pre: {pre}, post: {post}, delta: {delta}");

    if delta <= 0.0 {
        return Err(anyhow!("no token balance increase detected"));
    }

    // subtract fee before computing price
    let effective_lamports = lamports_in.saturating_sub(fee_lamports);
    let sol_spent = (effective_lamports as f64) / LAMPORTS_PER_SOL;
    let price = sol_spent / delta;

    println!(
        "💰 lamports_in: {lamports_in}  (= {:.9} SOL)",
        (lamports_in as f64) / LAMPORTS_PER_SOL
    );
    println!("💸 minus fee  : {fee_lamports} (= {SWAP_FEE_SOL:.9} SOL)");
    println!("💰 spent used : {effective_lamports} (= {sol_spent:.9} SOL)");
    println!("📈 effective price: {price:.12} SOL per token");

    Ok(price)
}

pub fn install_sigint_handler() -> Result<()> {
    // plain Ctrl‑C (works cross‑platform)
    ctrlc::set_handler(|| {
        SHUTDOWN.store(true, Ordering::SeqCst);
    })?;

    // Spawn async SIGTERM listener for Unix
    #[cfg(unix)]
    {
        use tokio::signal::unix::{ signal, SignalKind };
        tokio::spawn(async {
            let mut sigterm = signal(SignalKind::terminate()).expect(
                "cannot install SIGTERM handler"
            );
            sigterm.recv().await;
            SHUTDOWN.store(true, Ordering::SeqCst);
        });
    }

    Ok(())
}

pub fn format_duration_ago(from: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(from);

    if diff.num_seconds() < 60 {
        format!("{}s ago", diff.num_seconds())
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else {
        format!("{}d ago", diff.num_days())
    }
}

/// Waits until either Ctrl‑C (SIGINT) or SIGTERM (from systemd) is received.
pub async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{ signal, SignalKind };
        let mut term = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = term.recv()             => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

// Global set for permanently skipped tokens (now async Mutex)
pub static SKIPPED_SELLS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| {
    // Initialize from file, can't do async here, so use blocking
    let mut set = HashSet::new();
    if let Ok(file) = File::open(".skipped_sells") {
        for line in BufReader::new(file).lines().flatten() {
            set.insert(line.trim().to_string());
        }
    }
    Mutex::new(set)
});

pub async fn add_skipped_sell(mint: &str) {
    {
        let mut set = SKIPPED_SELLS.lock().await;
        if set.insert(mint.to_string()) {
            // File I/O must be blocking (but this is rare)
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(".skipped_sells") {
                let _ = writeln!(f, "{mint}");
            }
        }
    }
}

/// returns `Some(rsi)` if `values` has `period+1` points, otherwise `None`
pub fn rsi(values: &VecDeque<f64>, period: usize) -> Option<f64> {
    if values.len() <= period {
        return None;
    }
    let mut gain = 0.0;
    let mut loss = 0.0;
    for i in values.len() - period..values.len() - 1 {
        let diff = values[i + 1] - values[i];
        if diff >= 0.0 {
            gain += diff;
        } else {
            loss += -diff;
        }
    }
    if loss == 0.0 {
        return Some(100.0);
    }
    let rs = gain / loss;
    Some(100.0 - 100.0 / (1.0 + rs))
}

#[inline]
pub fn pct_change(old: f64, new_: f64) -> f64 {
    ((new_ - old) / old) * 100.0
}

pub fn ema(series: &VecDeque<f64>, period: usize) -> Option<f64> {
    if series.len() < period {
        return None;
    }
    let k = 2.0 / ((period as f64) + 1.0);
    let mut e = series[series.len() - period];
    for i in series.len() - period + 1..series.len() {
        e = series[i] * k + e * (1.0 - k);
    }
    Some(e)
}

pub fn atr_pct(hist: &VecDeque<f64>, period: usize) -> Option<f64> {
    if hist.len() < period + 1 {
        return None;
    }
    let mut sum = 0.0;
    for i in hist.len() - period + 1..hist.len() {
        let pct = ((hist[i] - hist[i - 1]).abs() / hist[i - 1]) * 100.0;
        sum += pct;
    }
    Some(sum / (period as f64)) // average % true range
}

/// Fetch pools from GeckoTerminal API with different sorting options
pub fn fetch_gecko_pools(token_mint: &str, sort: &str) -> Result<Vec<PoolInfo>> {
    let valid_sorts = ["h24_volume_usd_liquidity_desc", "h24_tx_count_desc", "h24_volume_usd_desc"];
    if !valid_sorts.contains(&sort) {
        return Err(anyhow!("Invalid sort parameter. Valid options: {:?}", valid_sorts));
    }

    let url = format!(
        "https://api.geckoterminal.com/api/v2/networks/solana/tokens/{}/pools?include=base_token%2C%20quote_token%2C%20dex&page=1&sort={}",
        token_mint,
        sort
    );

    println!("🦎 Fetching GeckoTerminal pools with sort: {}", sort);
    let json: Value = Client::new().get(&url).send()?.json()?;

    let mut pools = Vec::new();
    if let Some(data) = json.get("data").and_then(|v| v.as_array()) {
        for pool in data {
            if let Some(attrs) = pool.get("attributes") {
                let address = attrs
                    .get("address")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let name = attrs
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let liquidity_usd = attrs
                    .get("reserve_in_usd")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok());

                let volume_24h_usd = attrs
                    .get("volume_usd")
                    .and_then(|v| v.get("h24"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok());

                let tx_count_24h = attrs
                    .get("transactions")
                    .and_then(|v| v.get("h24"))
                    .and_then(|v| v.get("buys"))
                    .and_then(|v| v.as_u64())
                    .and_then(|buys| {
                        attrs
                            .get("transactions")
                            .and_then(|v| v.get("h24"))
                            .and_then(|v| v.get("sells"))
                            .and_then(|v| v.as_u64())
                            .map(|sells| buys + sells)
                    });

                if !address.is_empty() {
                    pools.push(PoolInfo {
                        address,
                        source: "geckoterminal".to_string(),
                        name,
                        liquidity_usd,
                        volume_24h_usd,
                        tx_count_24h,
                    });
                }
            }
        }
    }

    println!("🦎 Found {} pools from GeckoTerminal", pools.len());
    Ok(pools)
}

/// Fetch pools from DexScreener API and convert to PoolInfo format
pub fn fetch_dexscreener_pools(token_mint: &str) -> Result<Vec<PoolInfo>> {
    let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", token_mint);
    println!("📊 Fetching DexScreener pools...");
    let json: Value = Client::new().get(&url).send()?.json()?;

    let mut pools = Vec::new();
    if let Some(arr) = json.get("pairs").and_then(|v| v.as_array()) {
        for p in arr {
            if p.get("chainId").and_then(|v| v.as_str()) != Some("solana") {
                continue;
            }
            if let Some(addr) = p.get("pairAddress").and_then(|v| v.as_str()) {
                let name = p
                    .get("baseToken")
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|base| {
                        let quote = p
                            .get("quoteToken")
                            .and_then(|v| v.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown");
                        format!("{} / {}", base, quote)
                    });

                let liquidity_usd = p
                    .get("liquidity")
                    .and_then(|v| v.get("usd"))
                    .and_then(|v| v.as_f64());

                let volume_24h_usd = p
                    .get("volume")
                    .and_then(|v| v.get("h24"))
                    .and_then(|v| v.as_f64());

                let tx_count_24h = p
                    .get("txns")
                    .and_then(|v| v.get("h24"))
                    .and_then(|v| {
                        let buys = v
                            .get("buys")
                            .and_then(|b| b.as_u64())
                            .unwrap_or(0);
                        let sells = v
                            .get("sells")
                            .and_then(|s| s.as_u64())
                            .unwrap_or(0);
                        Some(buys + sells)
                    });

                pools.push(PoolInfo {
                    address: addr.to_string(),
                    source: "dexscreener".to_string(),
                    name,
                    liquidity_usd,
                    volume_24h_usd,
                    tx_count_24h,
                });
            }
        }
    }

    println!("📊 Found {} pools from DexScreener", pools.len());
    Ok(pools)
}

/// Fetch pools from both DexScreener and GeckoTerminal, combine and deduplicate
pub fn fetch_combined_pools(token_mint: &str) -> Result<Vec<PoolInfo>> {
    let mut all_pools = Vec::new();
    let mut seen_addresses = HashSet::new();

    // Fetch from DexScreener
    match fetch_dexscreener_pools(token_mint) {
        Ok(dex_pools) => {
            for pool in dex_pools {
                if seen_addresses.insert(pool.address.clone()) {
                    all_pools.push(pool);
                }
            }
        }
        Err(e) => println!("⚠️ DexScreener fetch failed: {}", e),
    }

    // Fetch from GeckoTerminal with different sorting options
    let gecko_sorts = ["h24_volume_usd_desc", "h24_tx_count_desc", "h24_volume_usd_liquidity_desc"];

    for sort in &gecko_sorts {
        match fetch_gecko_pools(token_mint, sort) {
            Ok(gecko_pools) => {
                for pool in gecko_pools {
                    if seen_addresses.insert(pool.address.clone()) {
                        all_pools.push(pool);
                    }
                }
            }
            Err(e) => println!("⚠️ GeckoTerminal fetch failed for sort {}: {}", sort, e),
        }
    }

    // Sort by liquidity (highest first), then by volume
    all_pools.sort_by(|a, b| {
        let a_liq = a.liquidity_usd.unwrap_or(0.0);
        let b_liq = b.liquidity_usd.unwrap_or(0.0);
        let a_vol = a.volume_24h_usd.unwrap_or(0.0);
        let b_vol = b.volume_24h_usd.unwrap_or(0.0);

        b_liq
            .partial_cmp(&a_liq)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b_vol.partial_cmp(&a_vol).unwrap_or(std::cmp::Ordering::Equal))
    });

    println!("🔗 Combined {} unique pools from both sources", all_pools.len());

    // Print summary of pools found
    for (i, pool) in all_pools.iter().take(5).enumerate() {
        println!(
            "  {}. {} [{}] - Liq: ${:.0} Vol: ${:.0} Txs: {}",
            i + 1,
            pool.name.as_ref().unwrap_or(&"Unknown".to_string()),
            pool.source,
            pool.liquidity_usd.unwrap_or(0.0),
            pool.volume_24h_usd.unwrap_or(0.0),
            pool.tx_count_24h.unwrap_or(0)
        );
    }

    Ok(all_pools)
}

/// Updated function that returns PoolInfo structs instead of just Pubkeys
pub fn fetch_solana_pools_detailed(token_mint: &str) -> Result<Vec<PoolInfo>> {
    fetch_combined_pools(token_mint)
}
