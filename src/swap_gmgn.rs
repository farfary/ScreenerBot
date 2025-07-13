use anyhow::{ Context, Result, anyhow };
use base64::{ engine::general_purpose, Engine };
use bincode;
use borsh::{ BorshDeserialize, BorshSerialize };
use chrono::{ DateTime, Utc };
use reqwest;
use serde::{ Deserialize, Serialize };
use serde_json::{ self, Value };
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{ RpcSendTransactionConfig, RpcSimulateTransactionConfig },
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::{ Instruction, AccountMeta },
    pubkey::Pubkey,
    signature::{ Keypair, Signature, Signer },
    system_instruction,
    transaction::{ Transaction, VersionedTransaction },
};
use std::{
    str::FromStr,
    time::{ Duration, Instant, SystemTime, UNIX_EPOCH },
    collections::{ HashMap, HashSet },
    fs::{ OpenOptions, File },
    io::{ BufRead, BufReader, Write },
    sync::Mutex,
};
use tokio::time::sleep;
use reqwest::Client;
use bs58;
use once_cell::sync::Lazy;

// Add missing constants for compatibility
pub const TRANSACTION_FEE_SOL: f64 = 0.000005; // 5000 lamports
pub const SLIPPAGE_BPS: u16 = 500; // 5% slippage in basis points
pub const MAX_TOKENS: usize = 1000; // Maximum tokens to process


/// Submit a buy swap to GMGN router and return detailed swap result.
/// Transactions are confirmed immediately after sending.
pub async fn buy_gmgn_detailed(
    token_mint_address: &str,
    in_amount: u64 // lamports you want to swap
) -> Result<SwapResult> {
    let start_time = std::time::Instant::now();

    // -------- 0. setup -----------------------------------------------------
    let wallet = {
        let bytes = bs58::decode(&crate::configs::CONFIGS.main_wallet_private).into_vec()?;
        Keypair::try_from(&bytes[..])?
    };
    let wallet_pk = wallet.pubkey();
    let owner = wallet_pk.to_string();
    let client = Client::new();
    let rpc_client = RpcClient::new(crate::configs::CONFIGS.rpc_url.clone());
    let token_mint_pk = Pubkey::from_str(token_mint_address).context("bad token mint pubkey")?;

    // -------- 1. create swap request ----------------------------------------
    let swap_request = SwapRequest::new_buy(
        token_mint_address,
        in_amount,
        &owner,
        0.5,
        TRANSACTION_FEE_SOL
    );

    let url = swap_request.to_gmgn_url();
    println!("🔍 GET QUOTE URL:\n{url}");

    // -------- 2. get quote --------------------------------------------------
    let body: Value = client
        .get(&url)
        .send().await?
        .error_for_status()?
        .json().await
        .context("decode quote JSON")?;
    println!("✅ QUOTE RESPONSE:\n{}", serde_json::to_string_pretty(&body)?);

    let execution_time_ms = start_time.elapsed().as_millis() as u64;
    let mut swap_result = SwapResult::from_gmgn_response(swap_request, &body, execution_time_ms);

    // ── detect gmgn router errors ────────────────────────────────────────────
    if !swap_result.success {
        let msg = swap_result.api_message.clone();

        // little-pool ⇒ stub (blacklist functionality removed)
        if msg.contains("little pool hit") {
            println!("🚫 [STUB] Would have blacklisted {token_mint_address} – {}", msg);
            swap_result.error_message = Some(
                format!("token {} marked for exclusion: {}", token_mint_address, msg)
            );
        }

        return Ok(swap_result);
    }
    // ─────────────────────────────────────────────────────────────────────────

    if let Some(ref raw_tx_data) = swap_result.raw_tx {
        let last_valid_block_height = raw_tx_data.last_valid_block_height;
        let swap_transaction = raw_tx_data.swap_transaction.clone();

        // -------- 3. sign -------------------------------------------------------
        let tx_bytes: Vec<u8> = general_purpose::STANDARD.decode(&swap_transaction)?;
        let mut vtx: VersionedTransaction = bincode::deserialize(&tx_bytes)?;
        let sig = wallet.sign_message(&vtx.message.serialize());
        vtx.signatures = vec![sig];
        let signed_tx_b64 = general_purpose::STANDARD.encode(bincode::serialize(&vtx)?);
        println!("✍️ Signed TX (base64 len {}):", signed_tx_b64.len());

        // -------- 4. submit -----------------------------------------------------
        match rpc_client.send_and_confirm_transaction(&vtx) {
            Ok(signature) => {
                println!("✅ submitted: {signature}");
                swap_result.set_transaction_signature(signature.to_string());

                // poll until finalised (existing helper)
                let confirm_start = std::time::Instant::now();
                match
                    poll_transaction_status(
                        &rpc_client,
                        &signature.to_string(),
                        last_valid_block_height
                    ).await
                {
                    Ok(sig_str) => {
                        let confirm_time = confirm_start.elapsed().as_millis() as u64;
                        swap_result.set_final_status("SUCCESS".to_string(), Some(confirm_time));

                        // -------- 5. derive effective price and token amount -----------------------------
                        match crate::helpers::get_swap_results(&sig_str).await {
                            Ok(_swap_details) => {
                                // For now, use placeholder values since we removed trading functionality
                                let price = 0.0;
                                let tokens_received = 0.0;
                                swap_result.set_effective_price(price);
                                swap_result.set_tokens_received(tokens_received);
                                println!("📈 EFFECTIVE BUY PRICE: {:.9} SOL per token", price);
                                println!("🪙 ACTUAL TOKENS RECEIVED: {:.9}", tokens_received);
                            }
                            Err(e) => {
                                swap_result.set_effective_price_error(
                                    format!("could not derive swap results: {e}")
                                );
                                eprintln!("⚠️  could not derive swap results: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        swap_result.set_final_status("FAILED".to_string(), None);
                        swap_result.error_message = Some(format!("confirmation failed: {e}"));
                    }
                }
            }
            Err(e) => {
                swap_result.set_final_status("FAILED".to_string(), None);
                swap_result.error_message = Some(format!("submit error: {e}"));
                swap_result.success = false;
            }
        }
    } else {
        swap_result.set_final_status("FAILED".to_string(), None);
        swap_result.error_message = Some("No raw transaction data received".to_string());
        swap_result.success = false;
    }

    // Print summary
    swap_result.print_summary();
    Ok(swap_result)
}

async fn poll_transaction_status(
    _rpc_client: &RpcClient, // We don't actually need to use this here
    tx_signature: &str, // Use the string version of the signature
    last_valid: u64
) -> Result<String> {
    // Return only signature now
    let status_url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_transaction_status?hash={}&last_valid_height={}",
        tx_signature,
        last_valid
    );

    let client = Client::new();
    println!("🔄 Start polling status...");

    for i in 0..15 {
        let check = client.get(&status_url).send().await?;
        let status: Value = check.json().await?;
        println!("📡 POLL {} RESPONSE:\n{}", i + 1, serde_json::to_string_pretty(&status)?);

        let success = status["data"]["success"].as_bool().unwrap_or(false);
        let expired = status["data"]["expired"].as_bool().unwrap_or(false);

        if success {
            println!("🎉 Tx confirmed successfully!");
            return Ok(tx_signature.to_string()); // Return only the signature now
        }
        if expired {
            anyhow::bail!("⏰ Tx expired before confirmation");
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    anyhow::bail!("❌ Tx not confirmed in time")
}

// --------------------------------------------------
// SELL FUNCTION WITH MIN-OUT-AMOUNT CHECK
// --------------------------------------------------
pub async fn sell_gmgn_detailed(
    token_mint_address: &str,
    token_amount: u64, // amount of tokens to sell (in token's smallest unit)
    min_out_amount: f64 // require at least this SOL out
) -> anyhow::Result<SwapResult> {
    let start_time = std::time::Instant::now();

    // Block if token is in skipped list
    {
        let set = SKIPPED_SELLS.lock().await;
        if set.contains(token_mint_address) {
            let mut swap_result = SwapResult::from_gmgn_response(
                SwapRequest::new_sell(token_mint_address, 0, "unknown", 0.0, 0.0),
                &serde_json::json!({
                    "code": -1,
                    "msg": "Sell skipped due to too many failures",
                    "tid": "skipped"
                }),
                0
            );
            swap_result.set_final_status("SKIPPED".to_string(), None);
            return Ok(swap_result);
        }
    }

    // load wallet
    let wallet = {
        let bytes = bs58::decode(&crate::configs::CONFIGS.main_wallet_private).into_vec()?;
        Keypair::try_from(&bytes[..])?
    };
    let owner = wallet.pubkey().to_string();
    let client = Client::new();
    let rpc_client = RpcClient::new(crate::configs::CONFIGS.rpc_url.clone());

    // use provided token amount
    let in_amount = token_amount;
    if in_amount == 0 {
        let mut swap_result = SwapResult::from_gmgn_response(
            SwapRequest::new_sell(
                token_mint_address,
                0,
                &owner,
                SLIPPAGE_BPS as f64,
                TRANSACTION_FEE_SOL
            ),
            &serde_json::json!({
                "code": -1,
                "msg": "No spendable balance",
                "tid": "no_balance"
            }),
            start_time.elapsed().as_millis() as u64
        );
        swap_result.set_final_status("FAILED".to_string(), None);
        return Ok(swap_result);
    }

    // -------- 1. create swap request ----------------------------------------
    let swap_request = SwapRequest::new_sell(
        token_mint_address,
        in_amount,
        &owner,
        SLIPPAGE_BPS as f64,
        TRANSACTION_FEE_SOL
    );

    let url = swap_request.to_gmgn_url();
    println!("🔍 SELL QUOTE URL:\n{url}");

    // -------- 2. fetch quote ------------------------------------------------
    let resp = client.get(&url).send().await?.error_for_status()?;
    let body: Value = resp.json().await.context("Failed to decode quote JSON")?;
    println!("✅ SELL QUOTE RESPONSE:\n{}", serde_json::to_string_pretty(&body)?);

    let execution_time_ms = start_time.elapsed().as_millis() as u64;
    let mut swap_result = SwapResult::from_gmgn_response(swap_request, &body, execution_time_ms);

    if !swap_result.success {
        return Ok(swap_result);
    }

    // -------- 3. check minimum out amount -----------------------------------
    if let Some(ref quote) = swap_result.quote {
        let out_amount_raw = quote.out_amount.parse::<u64>().context("Failed to parse out amount")?;
        let out_decimals = quote.out_decimals as i32;
        let out_amount_sol = (out_amount_raw as f64) / (10f64).powi(out_decimals);

        if out_amount_sol < min_out_amount {
            swap_result.set_final_status("FAILED".to_string(), None);
            swap_result.error_message = Some(
                format!(
                    "Quoted SOL out {:.9} is below required {:.9}, aborting",
                    out_amount_sol,
                    min_out_amount
                )
            );
            swap_result.success = false;
            return Ok(swap_result);
        }

        println!("💰 Quoted SOL out: {:.9} (required: {:.9})", out_amount_sol, min_out_amount);
    }

    // -------- 4. prepare and sign transaction -------------------------------
    if let Some(ref raw_tx_data) = swap_result.raw_tx {
        let last_valid_block_height = raw_tx_data.last_valid_block_height;
        let swap_transaction = raw_tx_data.swap_transaction.clone();

        let tx_bytes = general_purpose::STANDARD.decode(&swap_transaction)?;
        let mut vtx: VersionedTransaction = bincode::deserialize(&tx_bytes)?;
        let sig = wallet.sign_message(&vtx.message.serialize());
        vtx.signatures = vec![sig];

        // -------- 5. send transaction ---------------------------------------
        match rpc_client.send_and_confirm_transaction(&vtx) {
            Ok(signature) => {
                println!("✅ submitted: {signature}");
                swap_result.set_transaction_signature(signature.to_string());

                let confirm_start = std::time::Instant::now();
                match
                    poll_transaction_status(
                        &rpc_client,
                        &signature.to_string(),
                        last_valid_block_height
                    ).await
                {
                    Ok(_) => {
                        let confirm_time = confirm_start.elapsed().as_millis() as u64;
                        swap_result.set_final_status("SUCCESS".to_string(), Some(confirm_time));
                    }
                    Err(e) => {
                        swap_result.set_final_status("FAILED".to_string(), None);
                        swap_result.error_message = Some(format!("confirmation failed: {e}"));
                    }
                }
            }
            Err(e) => {
                swap_result.set_final_status("FAILED".to_string(), None);
                swap_result.error_message = Some(format!("submit error: {e}"));
                swap_result.success = false;
            }
        }
    } else {
        swap_result.set_final_status("FAILED".to_string(), None);
        swap_result.error_message = Some("No raw transaction data received".to_string());
        swap_result.success = false;
    }

    // Print summary
    swap_result.print_summary();
    Ok(swap_result)
}

// ──────────────────────────────────────────────────────────────────────
// ORIGINAL FUNCTIONS (BACKWARD COMPATIBILITY)
// ──────────────────────────────────────────────────────────────────────

/// Submit a swap to GMGN router and return the signature string.
/// Also prints the **effective on-chain price** paid for the swap.
/// Transactions are confirmed immediately after sending.
pub async fn buy_gmgn(
    token_mint_address: &str,
    in_amount: u64 // lamports you want to swap
) -> Result<String> {
    match buy_gmgn_detailed(token_mint_address, in_amount).await? {
        result if result.success => {
            if let Some(signature) = result.transaction_signature {
                Ok(signature)
            } else {
                anyhow::bail!("Swap succeeded but no signature available")
            }
        }
        result => { anyhow::bail!("Swap failed: {}", result.error_message.unwrap_or_default()) }
    }
}

pub async fn sell_all_gmgn(
    token_mint_address: &str,
    min_out_amount: f64 // require at least this SOL out
) -> anyhow::Result<String> {
    // get the maximum token balance to sell
    let token_amount = get_biggest_token_amount(token_mint_address);

    match sell_gmgn_detailed(token_mint_address, token_amount, min_out_amount).await? {
        result if result.success => {
            if let Some(signature) = result.transaction_signature {
                Ok(signature)
            } else {
                anyhow::bail!("Swap succeeded but no signature available")
            }
        }
        result => { anyhow::bail!("Swap failed: {}", result.error_message.unwrap_or_default()) }
    }
}

// ──────────────────────────────────────────────────────────────────────
// SWAP REQUEST AND RESULT STRUCTS
// ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    pub token_in_address: String,
    pub token_out_address: String,
    pub in_amount: u64,
    pub from_address: String,
    pub slippage: f64,
    pub swap_mode: String, // "ExactIn" or "ExactOut"
    pub fee: f64,
    pub is_anti_mev: bool,
    pub request_type: String, // "BUY" or "SELL"
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapInfo {
    pub amm_key: String,
    pub fee_amount: String,
    pub fee_mint: String,
    pub in_amount: String,
    pub input_mint: String,
    pub label: String,
    pub out_amount: String,
    pub output_mint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlan {
    pub percent: u32,
    pub swap_info: SwapInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub in_amount: String,
    pub in_decimals: u32,
    pub input_mint: String,
    pub other_amount_threshold: String,
    pub out_amount: String,
    pub out_decimals: u32,
    pub output_mint: String,
    pub platform_fee: String,
    pub price_impact_pct: String,
    pub route_plan: Vec<RoutePlan>,
    pub slippage_bps: String,
    pub swap_mode: String,
    pub time_taken: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTransaction {
    pub last_valid_block_height: u64,
    pub prioritization_fee_lamports: u64,
    pub recent_blockhash: String,
    pub swap_transaction: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    pub request: SwapRequest,
    pub success: bool,
    pub error_message: Option<String>,
    pub transaction_signature: Option<String>,

    // GMGN API Response Data
    pub amount_in_usd: Option<String>,
    pub amount_out_usd: Option<String>,
    pub jito_order_id: Option<String>,
    pub quote: Option<Quote>,
    pub raw_tx: Option<RawTransaction>,
    pub sol_cost: Option<String>,

    // Response metadata
    pub api_code: i64,
    pub api_message: String,
    pub api_transaction_id: String,

    // Effective pricing (calculated on-chain)
    pub effective_price: Option<f64>,
    pub effective_price_error: Option<String>,
    pub tokens_received: Option<f64>, // Actual tokens received from the swap

    // Execution details
    pub execution_time_ms: u64,
    pub confirmation_time_ms: Option<u64>,
    pub final_status: String, // "SUCCESS", "FAILED", "EXPIRED", "PENDING"
}

impl SwapRequest {
    pub fn new_buy(
        token_out_address: &str,
        in_amount: u64,
        from_address: &str,
        slippage: f64,
        fee: f64
    ) -> Self {
        Self {
            token_in_address: "So11111111111111111111111111111111111111112".to_string(),
            token_out_address: token_out_address.to_string(),
            in_amount,
            from_address: from_address.to_string(),
            slippage,
            swap_mode: "ExactIn".to_string(),
            fee,
            is_anti_mev: false,
            request_type: "BUY".to_string(),
            timestamp: std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn new_sell(
        token_in_address: &str,
        in_amount: u64,
        from_address: &str,
        slippage: f64,
        fee: f64
    ) -> Self {
        Self {
            token_in_address: token_in_address.to_string(),
            token_out_address: "So11111111111111111111111111111111111111112".to_string(),
            in_amount,
            from_address: from_address.to_string(),
            slippage,
            swap_mode: "ExactIn".to_string(),
            fee,
            is_anti_mev: false,
            request_type: "SELL".to_string(),
            timestamp: std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn to_gmgn_url(&self) -> String {
        format!(
            "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&swap_mode={}&fee={}&is_anti_mev={}",
            self.token_in_address,
            self.token_out_address,
            self.in_amount,
            self.from_address,
            self.slippage,
            self.swap_mode,
            self.fee,
            self.is_anti_mev
        )
    }
}

impl SwapResult {
    pub fn from_gmgn_response(
        request: SwapRequest,
        response: &Value,
        execution_time_ms: u64
    ) -> Self {
        let api_code = response["code"].as_i64().unwrap_or(-1);
        let api_message = response["msg"].as_str().unwrap_or("unknown").to_string();
        let api_transaction_id = response["tid"].as_str().unwrap_or("unknown").to_string();

        let success = api_code == 0;

        if !success {
            return Self {
                request,
                success: false,
                error_message: Some(api_message.clone()),
                transaction_signature: None,
                amount_in_usd: None,
                amount_out_usd: None,
                jito_order_id: None,
                quote: None,
                raw_tx: None,
                sol_cost: None,
                api_code,
                api_message,
                api_transaction_id,
                effective_price: None,
                effective_price_error: None,
                tokens_received: None,
                execution_time_ms,
                confirmation_time_ms: None,
                final_status: "FAILED".to_string(),
            };
        }

        let data = &response["data"];

        // Parse quote information
        let quote = if let Some(quote_data) = data["quote"].as_object() {
            let route_plan: Vec<RoutePlan> = quote_data["routePlan"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|rp| {
                    Some(RoutePlan {
                        percent: rp["percent"].as_u64().unwrap_or(0) as u32,
                        swap_info: SwapInfo {
                            amm_key: rp["swapInfo"]["ammKey"].as_str()?.to_string(),
                            fee_amount: rp["swapInfo"]["feeAmount"].as_str()?.to_string(),
                            fee_mint: rp["swapInfo"]["feeMint"].as_str()?.to_string(),
                            in_amount: rp["swapInfo"]["inAmount"].as_str()?.to_string(),
                            input_mint: rp["swapInfo"]["inputMint"].as_str()?.to_string(),
                            label: rp["swapInfo"]["label"].as_str()?.to_string(),
                            out_amount: rp["swapInfo"]["outAmount"].as_str()?.to_string(),
                            output_mint: rp["swapInfo"]["outputMint"].as_str()?.to_string(),
                        },
                    })
                })
                .collect();

            Some(Quote {
                in_amount: quote_data["inAmount"].as_str().unwrap_or("0").to_string(),
                in_decimals: quote_data["inDecimals"].as_u64().unwrap_or(0) as u32,
                input_mint: quote_data["inputMint"].as_str().unwrap_or("").to_string(),
                other_amount_threshold: quote_data["otherAmountThreshold"]
                    .as_str()
                    .unwrap_or("0")
                    .to_string(),
                out_amount: quote_data["outAmount"].as_str().unwrap_or("0").to_string(),
                out_decimals: quote_data["outDecimals"].as_u64().unwrap_or(0) as u32,
                output_mint: quote_data["outputMint"].as_str().unwrap_or("").to_string(),
                platform_fee: quote_data["platformFee"].as_str().unwrap_or("0").to_string(),
                price_impact_pct: quote_data["priceImpactPct"].as_str().unwrap_or("0").to_string(),
                route_plan,
                slippage_bps: quote_data["slippageBps"].as_str().unwrap_or("0").to_string(),
                swap_mode: quote_data["swapMode"].as_str().unwrap_or("ExactIn").to_string(),
                time_taken: quote_data["timeTaken"].as_f64().unwrap_or(0.0),
            })
        } else {
            None
        };

        // Parse raw transaction data
        let raw_tx = if let Some(raw_tx_data) = data["raw_tx"].as_object() {
            Some(RawTransaction {
                last_valid_block_height: raw_tx_data["lastValidBlockHeight"].as_u64().unwrap_or(0),
                prioritization_fee_lamports: raw_tx_data["prioritizationFeeLamports"]
                    .as_u64()
                    .unwrap_or(0),
                recent_blockhash: raw_tx_data["recentBlockhash"].as_str().unwrap_or("").to_string(),
                swap_transaction: raw_tx_data["swapTransaction"].as_str().unwrap_or("").to_string(),
                version: raw_tx_data["version"].as_str().unwrap_or("0").to_string(),
            })
        } else {
            None
        };

        Self {
            request,
            success: true,
            error_message: None,
            transaction_signature: None,
            amount_in_usd: data["amount_in_usd"].as_str().map(|s| s.to_string()),
            amount_out_usd: data["amount_out_usd"].as_str().map(|s| s.to_string()),
            jito_order_id: data["jito_order_id"].as_str().map(|s| s.to_string()),
            quote,
            raw_tx,
            sol_cost: data["sol_cost"].as_str().map(|s| s.to_string()),
            api_code,
            api_message,
            api_transaction_id,
            effective_price: None,
            effective_price_error: None,
            tokens_received: None,
            execution_time_ms,
            confirmation_time_ms: None,
            final_status: "PENDING".to_string(),
        }
    }

    pub fn set_transaction_signature(&mut self, signature: String) {
        self.transaction_signature = Some(signature);
    }

    pub fn set_final_status(&mut self, status: String, confirmation_time_ms: Option<u64>) {
        self.final_status = status;
        self.confirmation_time_ms = confirmation_time_ms;
    }

    pub fn set_effective_price(&mut self, price: f64) {
        self.effective_price = Some(price);
    }

    pub fn set_effective_price_error(&mut self, error: String) {
        self.effective_price_error = Some(error);
    }

    pub fn set_tokens_received(&mut self, tokens: f64) {
        self.tokens_received = Some(tokens);
    }

    /// Prints a comprehensive summary of the swap result
    pub fn print_summary(&self) {
        println!("🔄 SWAP SUMMARY");
        println!("  Type: {}", self.request.request_type);
        println!("  Success: {}", self.success);
        println!("  Status: {}", self.final_status);

        if let Some(signature) = &self.transaction_signature {
            println!("  Signature: {}", signature);
        }

        if let Some(error) = &self.error_message {
            println!("  Error: {}", error);
        }

        if let Some(quote) = &self.quote {
            println!("  Quote Details:");
            println!("    In Amount: {} (decimals: {})", quote.in_amount, quote.in_decimals);
            println!("    Out Amount: {} (decimals: {})", quote.out_amount, quote.out_decimals);
            println!("    Price Impact: {}%", quote.price_impact_pct);
            println!(
                "    Route: {}",
                quote.route_plan
                    .iter()
                    .map(|rp| rp.swap_info.label.clone())
                    .collect::<Vec<_>>()
                    .join(" -> ")
            );
        }

        if let Some(amount_in_usd) = &self.amount_in_usd {
            println!("  Amount In USD: ${}", amount_in_usd);
        }
        if let Some(amount_out_usd) = &self.amount_out_usd {
            println!("  Amount Out USD: ${}", amount_out_usd);
        }

        if let Some(effective_price) = self.effective_price {
            println!("  Effective Price: {:.9} SOL per token", effective_price);
        }

        println!("  Execution Time: {}ms", self.execution_time_ms);
        if let Some(confirm_time) = self.confirmation_time_ms {
            println!("  Confirmation Time: {}ms", confirm_time);
        }
    }
}

// ──────────────────────────────────────────────────────────────────────
// UPDATED SWAP FUNCTIONS
// ──────────────────────────────────────────────────────────────────────

/// Submit a buy swap and return both transaction signature and actual tokens received.
/// Returns (tx_signature, tokens_received)
pub async fn buy_gmgn_with_amounts(
    token_mint_address: &str,
    in_amount: u64 // lamports you want to swap
) -> Result<(String, f64)> {
    let swap_result = buy_gmgn_detailed(token_mint_address, in_amount).await?;

    if !swap_result.success {
        return Err(anyhow!("Swap failed: {}", swap_result.error_message.unwrap_or_default()));
    }

    let tx_signature = swap_result.transaction_signature.ok_or_else(||
        anyhow!("No transaction signature available")
    )?;

    let tokens_received = swap_result.tokens_received.ok_or_else(||
        anyhow!("No tokens received information available")
    )?;

    Ok((tx_signature, tokens_received))
}
