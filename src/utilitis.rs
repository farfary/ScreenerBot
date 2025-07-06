// src/utils.rs
//! Little helper: fetch the `decimals` field of any SPL-Token mint.

use anyhow::{ anyhow, Result, bail };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::{ Mint, Account };
use solana_program::program_pack::Pack;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io::{ Write };
use std::str::FromStr;
use reqwest::blocking::Client;
use serde_json::Value;

use std::time::{ SystemTime, UNIX_EPOCH };

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

use std::{ fs::{ File }, time::{ Instant } };
use rayon::prelude::*; // ADD THIS

use std::{ sync::RwLock };
use once_cell::sync::Lazy;

// Cache: token_mint -> (timestamp_secs, price)
pub static PRICE_CACHE: Lazy<RwLock<HashMap<String, (u64, f64)>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

/// Pull every *Solana* `pairAddress` for the given token mint from DexScreener, with 2h cache.
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
                    .map(|s| Pubkey::from_str(s).unwrap())
                    .collect()
            );
        }
    }

    // Fetch fresh
    let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", token_mint);
    let json: Value = Client::new().get(&url).send()?.json()?;

    let mut out = Vec::new();
    if let Some(arr) = json.get("pairs").and_then(|v| v.as_array()) {
        for p in arr {
            if p.get("chainId").and_then(|v| v.as_str()) != Some("solana") {
                continue;
            }
            if let Some(addr) = p.get("pairAddress").and_then(|v| v.as_str()) {
                out.push(addr.to_string());
            }
        }
    }
    if out.is_empty() {
        bail!("DexScreener: no Solana pools for mint {}", token_mint);
    }

    // Update cache
    cache.insert(token_mint.to_string(), (now, out.clone()));
    let _ = fs::File::create(cache_path).and_then(|mut f| {
        let s = serde_json::to_string(&cache).unwrap();
        f.write_all(s.as_bytes())
    });

    // Return as Vec<Pubkey>
    Ok(
        out
            .iter()
            .map(|s| Pubkey::from_str(s).unwrap())
            .collect()
    )
}

use solana_client::{ rpc_config::RpcTransactionConfig };
use solana_sdk::{ commitment_config::CommitmentConfig, signature::Signature };
use solana_transaction_status::{
    option_serializer::OptionSerializer,
    UiTransactionEncoding,
    UiTransactionTokenBalance,
};

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

use std::sync::atomic::{ AtomicBool, Ordering };

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

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
