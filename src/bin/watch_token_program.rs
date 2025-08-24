//! Watch SPL Token program logs for new mints and print them.
//!
//! This tool connects to the Solana WebSocket endpoint (derived from configs.rpc_url),
//! subscribes to logs mentioning the SPL Token program (Tokenkeg...), and prints messages
//! that include InitializeMint, indicating new token mint initializations.

use clap::Parser;
use futures_util::{ SinkExt, StreamExt };
use tokio_tungstenite::connect_async;
use url::Url;

use screenerbot::logger::{ log, LogTag };
use screenerbot::rpc::{
    build_logs_subscribe_payload,
    get_premium_websocket_url,
    logs_contains_initialize_mint,
    spl_token_program_id,
};
use screenerbot::tokens::dexscreener::{
    init_dexscreener_api,
    get_token_from_mint_global_api,
    get_token_pairs_from_api,
};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Parser)]
#[command(name = "watch_token_program", about = "Watch SPL Token program for new mints")]
struct Args {
    /// Print full raw JSON for matching notifications
    #[arg(long)]
    verbose: bool,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("watch_token_program error: {}", e);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    // Init dexscreener client once
    let _ = init_dexscreener_api().await; // ignore if already initialized
    // Always use PREMIUM WS as requested
    let ws_url = get_premium_websocket_url()?;
    let url = Url::parse(&ws_url)?;

    log(LogTag::Rpc, "WS_CONNECT", &format!("Connecting to {}", ws_url));
    let (ws_stream, _resp) = connect_async(url).await?;
    log(LogTag::Rpc, "WS_CONNECTED", "Connected. Subscribing to SPL Token program logs...");

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to logs with mentions filter for SPL Token program AND Token-2022
    let spl_token_program = spl_token_program_id();
    let token_2022_program = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"; // Token-2022 program
    let subscribe_msg = build_logs_subscribe_payload(&[spl_token_program, token_2022_program]);
    let subscribe_txt = subscribe_msg.to_string();
    write.send(tokio_tungstenite::tungstenite::Message::Text(subscribe_txt)).await?;

    // Read initial confirmation message
    if let Some(msg) = read.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(txt)) => {
                log(LogTag::Rpc, "WS_SUBSCRIBED", &format!("Subscribed: {}", txt));
            }
            Ok(_) => {}
            Err(e) => {
                log(LogTag::Rpc, "WS_ERROR", &format!("Subscription ack error: {}", e));
            }
        }
    }

    log(
        LogTag::Rpc,
        "WS_LISTEN",
        "Listening for InitializeMint logs and watching for pool creation..."
    );

    let mut seen_signatures_mint: HashSet<String> = HashSet::new();
    let mut seen_mints: HashSet<String> = HashSet::new();
    let announced_pools: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let polling_mints: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    while let Some(msg) = read.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(txt)) => {
                // Parse JSON and inspect logs
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                    // Debug: Print any message we receive (not just InitializeMint)
                    if args.verbose {
                        println!("WS_MSG: {}", txt);
                    }

                    // Expected format: { method: "logsNotification", params: { result: { value: { logs: [...] } } } }
                    let logs = val
                        .get("params")
                        .and_then(|p| p.get("result"))
                        .and_then(|r| r.get("value"))
                        .and_then(|v| v.get("logs"))
                        .and_then(|l| l.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        });

                    if let Some(logs_vec) = logs {
                        if logs_contains_initialize_mint(&logs_vec) {
                            // signature for the notification
                            let signature = val
                                .get("params")
                                .and_then(|p| p.get("result"))
                                .and_then(|r| r.get("value"))
                                .and_then(|v| v.get("signature"))
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            if
                                !signature.is_empty() &&
                                !seen_signatures_mint.insert(signature.clone())
                            {
                                continue; // already processed
                            }

                            if args.verbose {
                                println!("RAW: {}", txt);
                            }

                            // Try to derive mint address from transaction
                            // Add small delay to avoid overwhelming RPC
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            if let Some(mint) = extract_mint_from_signature(&signature).await {
                                if seen_mints.insert(mint.clone()) {
                                    // Fetch DexScreener token info
                                    let token_info = get_token_from_mint_global_api(&mint).await
                                        .ok()
                                        .flatten();
                                    let (symbol, price_sol, _liq_usd) = token_info
                                        .as_ref()
                                        .map(|t| {
                                            let symbol = if !t.symbol.is_empty() {
                                                t.symbol.clone()
                                            } else {
                                                "?".to_string()
                                            };
                                            let price = t.price_dexscreener_sol
                                                .or(t.price_pool_sol)
                                                .unwrap_or(0.0);
                                            let liq = t.liquidity
                                                .as_ref()
                                                .and_then(|l| l.usd)
                                                .unwrap_or(0.0);
                                            (symbol, price, liq)
                                        })
                                        .unwrap_or(("?".to_string(), 0.0, 0.0));

                                    // Fetch pools info and announce if pool already exists; else start polling
                                    match get_token_pairs_from_api(&mint).await {
                                        Ok(pools) if !pools.is_empty() => {
                                            let mut best_url = String::new();
                                            let mut best_liq = 0.0;
                                            for p in &pools {
                                                if
                                                    let Some(liq) = p.liquidity
                                                        .as_ref()
                                                        .map(|l| l.usd)
                                                {
                                                    if liq > best_liq {
                                                        best_liq = liq;
                                                        best_url = p.url.clone();
                                                    }
                                                }
                                            }
                                            println!(
                                                "POOL CREATED {} {} price:{:.12} SOL liq:${:.0} pools:{} top:{} sig:{}",
                                                symbol,
                                                mint,
                                                price_sol,
                                                best_liq,
                                                pools.len(),
                                                best_url,
                                                &signature[..std::cmp::min(10, signature.len())]
                                            );
                                            let mut announced = announced_pools.lock().await;
                                            announced.insert(mint.clone());
                                        }
                                        _ => {
                                            // Start a short-lived poller to detect when pool appears
                                            let mint_cl = mint.clone();
                                            let symbol_cl = symbol.clone();
                                            let announced_pools_cl = Arc::clone(&announced_pools);
                                            let polling_mints_cl = Arc::clone(&polling_mints);
                                            tokio::spawn(async move {
                                                // ensure single poller per mint
                                                {
                                                    let mut set = polling_mints_cl.lock().await;
                                                    if !set.insert(mint_cl.clone()) {
                                                        return;
                                                    }
                                                }
                                                let max_attempts = 36; // ~3 minutes @5s
                                                for _ in 0..max_attempts {
                                                    // stop if already announced
                                                    if
                                                        announced_pools_cl
                                                            .lock().await
                                                            .contains(&mint_cl)
                                                    {
                                                        break;
                                                    }
                                                    if
                                                        let Ok(pools) = get_token_pairs_from_api(
                                                            &mint_cl
                                                        ).await
                                                    {
                                                        if !pools.is_empty() {
                                                            let mut best_url = String::new();
                                                            let mut best_liq = 0.0;
                                                            for p in &pools {
                                                                if
                                                                    let Some(liq) = p.liquidity
                                                                        .as_ref()
                                                                        .map(|l| l.usd)
                                                                {
                                                                    if liq > best_liq {
                                                                        best_liq = liq;
                                                                        best_url = p.url.clone();
                                                                    }
                                                                }
                                                            }
                                                            println!(
                                                                "POOL CREATED {} {} liq:${:.0} pools:{} top:{}",
                                                                symbol_cl,
                                                                mint_cl,
                                                                best_liq,
                                                                pools.len(),
                                                                best_url
                                                            );
                                                            announced_pools_cl
                                                                .lock().await
                                                                .insert(mint_cl.clone());
                                                            break;
                                                        }
                                                    }
                                                    tokio::time::sleep(
                                                        std::time::Duration::from_secs(5)
                                                    ).await;
                                                }
                                                // remove from polling set at end
                                                polling_mints_cl.lock().await.remove(&mint_cl);
                                            });
                                        }
                                    }
                                }
                            } else {
                                println!(
                                    "NEW MINT (mint: ?) sig:{}",
                                    &signature[..std::cmp::min(10, signature.len())]
                                );
                            }
                            log(LogTag::Rpc, "NEW_MINT", "InitializeMint detected");
                        }
                    }
                }
            }
            Ok(tokio_tungstenite::tungstenite::Message::Ping(p)) => {
                // Respond to ping
                write.send(tokio_tungstenite::tungstenite::Message::Pong(p)).await.ok();
            }
            Ok(_) => {}
            Err(e) => {
                log(LogTag::Rpc, "WS_RECV_ERROR", &format!("{}", e));
                break;
            }
        }
    }

    Ok(())
}

/// Extract the mint address from a transaction signature by inspecting the Token program instruction
async fn extract_mint_from_signature(signature: &str) -> Option<String> {
    use screenerbot::rpc::get_rpc_client;
    let rpc = get_rpc_client();

    // Add small delay and retry logic to handle rate limits
    for attempt in 0..3 {
        match rpc.get_transaction_details_premium(signature).await {
            Ok(details) => {
                // Resolve account keys list (strings)
                let account_keys = details.transaction.message
                    .get("accountKeys")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|x| {
                                if let Some(s) = x.as_str() {
                                    s.to_string()
                                } else {
                                    x.get("pubkey")
                                        .and_then(|p| p.as_str())
                                        .unwrap_or("")
                                        .to_string()
                                }
                            })
                            .collect::<Vec<String>>()
                    });

                if let Some(account_keys) = account_keys {
                    let token_prog = spl_token_program_id();

                    // Find Token program instruction
                    if
                        let Some(instrs) = details.transaction.message
                            .get("instructions")
                            .and_then(|v| v.as_array())
                    {
                        for ins in instrs {
                            let pid_idx = ins
                                .get("programIdIndex")
                                .and_then(|i| i.as_u64())
                                .unwrap_or(u64::MAX) as usize;
                            if let Some(prog) = account_keys.get(pid_idx) {
                                if prog == token_prog {
                                    // First account index is mint
                                    if
                                        let Some(accs) = ins
                                            .get("accounts")
                                            .and_then(|a| a.as_array())
                                    {
                                        if
                                            let Some(first_idx) = accs
                                                .first()
                                                .and_then(|i| i.as_u64())
                                        {
                                            let idx = first_idx as usize;
                                            if let Some(mint) = account_keys.get(idx) {
                                                if mint.len() >= 32 {
                                                    return Some(mint.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                return None; // Transaction found but no mint extracted
            }
            Err(_) => {
                if attempt < 2 {
                    // Wait before retry
                    tokio::time::sleep(std::time::Duration::from_millis(200 * (attempt + 1))).await;
                    continue;
                }
            }
        }
    }
    None
}

// holder tracking removed per user request (focus on pool creation)
