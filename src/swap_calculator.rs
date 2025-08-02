use serde_json::Value;
use regex::Regex;
use crate::{
    wallet::SwapError,
    global::{ is_debug_profit_enabled, is_debug_swap_enabled },
    logger::{ log, LogTag },
};

/// SOL mint address (native SOL)
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Comprehensive swap analysis result containing all important details
#[derive(Debug, Clone)]
pub struct SwapAnalysisResult {
    // Transaction status
    pub success: bool,
    pub transaction_signature: String,
    pub error_message: Option<String>,

    // Amounts (in human-readable units with decimals)
    pub input_amount: f64,
    pub output_amount: f64,
    pub input_decimals: u8,
    pub output_decimals: u8,

    // Raw amounts (in smallest token units)
    pub input_amount_raw: u64,
    pub output_amount_raw: u64,

    // Price analysis
    pub effective_price: f64,
    pub expected_price: Option<f64>,
    pub price_difference_percent: f64,
    pub slippage_percent: f64,

    // Fee analysis
    pub transaction_fee_sol: f64,
    pub transaction_fee_lamports: u64,
    pub platform_fee_sol: Option<f64>,
    pub total_fees_sol: f64,

    // ATA analysis
    pub ata_creation_detected: bool,
    pub ata_rent_lamports: u64,
    pub ata_rent_sol: f64,

    // Analysis metadata
    pub analysis_method: String,
    pub confidence_score: f64,
    pub analysis_time_ms: u64,

    // Token information
    pub input_mint: String,
    pub output_mint: String,
    pub is_buy: bool, // true for SOL->Token, false for Token->SOL

    // Additional details
    pub wallet_address: String,
    pub block_height: Option<u64>,
    pub block_time: Option<i64>,
}

/// Token transfer data extracted from transaction
#[derive(Debug, Clone)]
struct TokenTransferData {
    input_amount: f64,
    output_amount: f64,
    input_decimals: u8,
    output_decimals: u8,
    confidence: f64,
    method: String,
}

/// Convert lamports to SOL
fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / 1_000_000_000.0
}

/// Convert SOL to lamports
fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

/// Get transaction details from RPC
async fn get_transaction_details(
    client: &reqwest::Client,
    transaction_signature: &str,
    rpc_endpoint: &str
) -> Result<String, SwapError> {
    let request_body =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            transaction_signature,
            {
                "encoding": "json",
                "maxSupportedTransactionVersion": 0,
                "commitment": "confirmed"
            }
        ]
    });

    let response = client
        .post(rpc_endpoint)
        .json(&request_body)
        .send().await
        .map_err(|e| SwapError::NetworkError(e))?;

    let response_text = response.text().await.map_err(|e| SwapError::NetworkError(e))?;

    let json: Value = serde_json
        ::from_str(&response_text)
        .map_err(|e| SwapError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    if let Some(result) = json.get("result") {
        if result.is_null() {
            return Err(SwapError::InvalidResponse("Transaction not found".to_string()));
        }
        Ok(serde_json::to_string(result).unwrap())
    } else if let Some(error) = json.get("error") {
        Err(SwapError::InvalidResponse(format!("RPC error: {}", error)))
    } else {
        Err(SwapError::InvalidResponse("Invalid RPC response format".to_string()))
    }
}

/// Method 1: Comprehensive Analysis (Combines all methods)
pub async fn analyze_swap_comprehensive(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    rpc_endpoint: &str,
    intended_amount: Option<f64>
) -> Result<SwapAnalysisResult, SwapError> {
    let start_time = std::time::Instant::now();

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "ANALYSIS_START",
            &format!(
                "🔄 Starting comprehensive swap analysis\n  TX: {}\n  Input: {} -> Output: {}\n  Wallet: {}\n  Intended: {:?}",
                transaction_signature,
                if input_mint == SOL_MINT {
                    "SOL"
                } else {
                    &input_mint[..8]
                },
                if output_mint == SOL_MINT {
                    "SOL"
                } else {
                    &output_mint[..8]
                },
                &wallet_address[..8],
                intended_amount
            )
        );
    }

    if is_debug_profit_enabled() {
        log(
            LogTag::Wallet,
            "SWAP_ANALYSIS",
            &format!("Starting comprehensive swap analysis for TX: {}", transaction_signature)
        );
    }

    // Wait for transaction to be fully confirmed
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Get transaction details
    let tx_response = get_transaction_details(client, transaction_signature, rpc_endpoint).await?;
    let transaction_json: Value = serde_json
        ::from_str(&tx_response)
        .map_err(|e| SwapError::InvalidResponse(format!("Failed to parse transaction: {}", e)))?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "TX_FETCHED",
            &format!("📥 Transaction data retrieved from RPC endpoint: {}", rpc_endpoint)
        );

        // Log transaction structure overview
        if let Some(result) = transaction_json.get("result") {
            if let Some(meta) = result.get("meta") {
                let fee = meta
                    .get("fee")
                    .and_then(|f| f.as_u64())
                    .unwrap_or(0);
                let compute_units = meta
                    .get("computeUnitsConsumed")
                    .and_then(|c| c.as_u64())
                    .unwrap_or(0);
                let err = meta.get("err");

                log(
                    LogTag::Swap,
                    "TX_META",
                    &format!(
                        "📊 Transaction metadata - Fee: {} lamports ({:.6} SOL), Compute Units: {}, Error: {}",
                        fee,
                        lamports_to_sol(fee),
                        compute_units,
                        if err.is_some() && !err.unwrap().is_null() {
                            "❌ FAILED"
                        } else {
                            "✅ SUCCESS"
                        }
                    )
                );
            }
        }
    }

    // Check transaction success
    let success = check_transaction_success(&transaction_json)?;
    let error_message = if !success { extract_error_message(&transaction_json) } else { None };

    if !success {
        return Ok(SwapAnalysisResult {
            success: false,
            transaction_signature: transaction_signature.to_string(),
            error_message,
            input_amount: 0.0,
            output_amount: 0.0,
            input_decimals: 0,
            output_decimals: 0,
            input_amount_raw: 0,
            output_amount_raw: 0,
            effective_price: 0.0,
            expected_price: intended_amount,
            price_difference_percent: 0.0,
            slippage_percent: 0.0,
            transaction_fee_sol: 0.0,
            transaction_fee_lamports: 0,
            platform_fee_sol: None,
            total_fees_sol: 0.0,
            ata_creation_detected: false,
            ata_rent_lamports: 0,
            ata_rent_sol: 0.0,
            analysis_method: "Failed Transaction".to_string(),
            confidence_score: 1.0,
            analysis_time_ms: start_time.elapsed().as_millis() as u64,
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            is_buy: input_mint == SOL_MINT,
            wallet_address: wallet_address.to_string(),
            block_height: extract_block_height(&transaction_json),
            block_time: extract_block_time(&transaction_json),
        });
    }

    // Try multiple analysis methods
    let methods = vec![
        analyze_inner_instructions(&transaction_json, input_mint, output_mint, wallet_address),
        analyze_token_balances(&transaction_json, input_mint, output_mint, wallet_address),
        analyze_log_messages(&transaction_json, input_mint, output_mint)
    ];

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "ANALYSIS_METHODS",
            "🔍 Running 3 analysis methods: Inner Instructions, Token Balances, Log Messages"
        );
    }

    // Get valid results
    let valid_results: Vec<_> = methods
        .into_iter()
        .enumerate()
        .filter_map(|(i, r)| {
            match r {
                Ok(result) => {
                    if is_debug_swap_enabled() {
                        let method_name = match i {
                            0 => "Inner Instructions",
                            1 => "Token Balances",
                            2 => "Log Messages",
                            _ => "Unknown",
                        };
                        log(
                            LogTag::Swap,
                            "METHOD_SUCCESS",
                            &format!(
                                "✅ {} - Input: {:.6}, Output: {:.6}, Confidence: {:.2}",
                                method_name,
                                result.input_amount,
                                result.output_amount,
                                result.confidence
                            )
                        );
                    }
                    Some(result)
                }
                Err(e) => {
                    if is_debug_swap_enabled() {
                        let method_name = match i {
                            0 => "Inner Instructions",
                            1 => "Token Balances",
                            2 => "Log Messages",
                            _ => "Unknown",
                        };
                        log(
                            LogTag::Swap,
                            "METHOD_FAILED",
                            &format!("❌ {} failed: {}", method_name, e)
                        );
                    }
                    None
                }
            }
        })
        .collect();

    if valid_results.is_empty() {
        if is_debug_swap_enabled() {
            log(
                LogTag::Swap,
                "ANALYSIS_FAILED",
                "❌ No valid analysis methods succeeded - unable to determine swap amounts"
            );
        }
        return Err(SwapError::InvalidResponse("No valid analysis methods succeeded".to_string()));
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "CONSENSUS_START",
            &format!("🎯 Calculating consensus from {} valid results", valid_results.len())
        );
    }

    // Calculate consensus result
    let consensus_result = calculate_consensus_result(valid_results, intended_amount)?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "CONSENSUS_RESULT",
            &format!(
                "📊 Consensus: Input={:.6} (decimals={}), Output={:.6} (decimals={}), Method={}, Confidence={:.2}",
                consensus_result.input_amount,
                consensus_result.input_decimals,
                consensus_result.output_amount,
                consensus_result.output_decimals,
                consensus_result.method,
                consensus_result.confidence
            )
        );
    }

    // Extract fee information
    let (tx_fee_lamports, tx_fee_sol) = extract_transaction_fee(&transaction_json);
    let platform_fee_sol = extract_platform_fee(&transaction_json);
    let total_fees_sol = tx_fee_sol + platform_fee_sol.unwrap_or(0.0);

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "FEE_ANALYSIS",
            &format!(
                "💰 Fee breakdown - TX Fee: {:.6} SOL ({} lamports), Platform Fee: {:.6} SOL, Total: {:.6} SOL",
                tx_fee_sol,
                tx_fee_lamports,
                platform_fee_sol.unwrap_or(0.0),
                total_fees_sol
            )
        );
    }

    // Detect ATA creation
    let (ata_detected, ata_rent_lamports, ata_rent_sol) = detect_ata_creation(
        &transaction_json,
        wallet_address
    );

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "ATA_DETECTION",
            &format!(
                "🏦 ATA Analysis - Detected: {}, Rent: {:.6} SOL ({} lamports)",
                if ata_detected {
                    "✅ YES"
                } else {
                    "❌ NO"
                },
                ata_rent_sol,
                ata_rent_lamports
            )
        );
    }

    // Calculate effective price correctly (SOL per token)
    // For SOL->Token: price = SOL_amount / token_amount
    // For Token->SOL: price = SOL_amount / token_amount
    let effective_price = if input_mint == SOL_MINT {
        // SOL -> Token: SOL spent / tokens received
        consensus_result.input_amount / consensus_result.output_amount
    } else {
        // Token -> SOL: SOL received / tokens spent
        consensus_result.output_amount / consensus_result.input_amount
    };

    if is_debug_swap_enabled() {
        let swap_type = if input_mint == SOL_MINT {
            "SOL -> Token (BUY)"
        } else {
            "Token -> SOL (SELL)"
        };
        log(
            LogTag::Swap,
            "PRICE_CALC",
            &format!(
                "💹 Price calculation - Type: {}, Effective Price: {:.12} SOL per token",
                swap_type,
                effective_price
            )
        );

        if input_mint == SOL_MINT {
            log(
                LogTag::Swap,
                "PRICE_DETAIL",
                &format!(
                    "📈 BUY: Spent {:.6} SOL → Received {:.6} tokens = {:.12} SOL per token",
                    consensus_result.input_amount,
                    consensus_result.output_amount,
                    effective_price
                )
            );
        } else {
            log(
                LogTag::Swap,
                "PRICE_DETAIL",
                &format!(
                    "📉 SELL: Spent {:.6} tokens → Received {:.6} SOL = {:.12} SOL per token",
                    consensus_result.input_amount,
                    consensus_result.output_amount,
                    effective_price
                )
            );
        }
    }

    // Calculate price difference and slippage based on expected vs actual amounts
    let (price_diff_percent, slippage_percent) = if let Some(intended) = intended_amount {
        if input_mint == SOL_MINT {
            // For SOL->Token: intended is SOL amount, compare with actual tokens received
            // Expected tokens = intended_sol_amount / effective_price
            let expected_tokens = intended / effective_price;
            let actual_tokens = consensus_result.output_amount;
            let token_diff_percent = ((actual_tokens - expected_tokens) / expected_tokens) * 100.0;
            let slippage = token_diff_percent.abs();

            if is_debug_swap_enabled() {
                log(
                    LogTag::Swap,
                    "SLIPPAGE_BUY",
                    &format!(
                        "📊 BUY Slippage - Intended: {:.6} SOL, Expected tokens: {:.6}, Actual tokens: {:.6}, Diff: {:.3}%, Slippage: {:.3}%",
                        intended,
                        expected_tokens,
                        actual_tokens,
                        token_diff_percent,
                        slippage
                    )
                );
            }

            (token_diff_percent, slippage)
        } else {
            // For Token->SOL: intended is token amount, compare with actual SOL received
            // Expected SOL = intended_tokens * effective_price
            let expected_sol = intended * effective_price;
            let actual_sol = consensus_result.output_amount;
            let sol_diff_percent = ((actual_sol - expected_sol) / expected_sol) * 100.0;
            let slippage = sol_diff_percent.abs();

            if is_debug_swap_enabled() {
                log(
                    LogTag::Swap,
                    "SLIPPAGE_SELL",
                    &format!(
                        "📊 SELL Slippage - Intended: {:.6} tokens, Expected SOL: {:.6}, Actual SOL: {:.6}, Diff: {:.3}%, Slippage: {:.3}%",
                        intended,
                        expected_sol,
                        actual_sol,
                        sol_diff_percent,
                        slippage
                    )
                );
            }

            (sol_diff_percent, slippage)
        }
    } else {
        if is_debug_swap_enabled() {
            log(
                LogTag::Swap,
                "NO_SLIPPAGE",
                "⚠️ No intended amount provided - cannot calculate slippage"
            );
        }
        (0.0, 0.0)
    };

    // Convert to raw amounts
    let input_raw = (consensus_result.input_amount *
        (10_f64).powi(consensus_result.input_decimals as i32)) as u64;
    let output_raw = (consensus_result.output_amount *
        (10_f64).powi(consensus_result.output_decimals as i32)) as u64;

    let analysis_time = start_time.elapsed().as_millis() as u64;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "ANALYSIS_COMPLETE",
            &format!(
                "🎉 Comprehensive analysis complete in {}ms\n  ✅ Success: {}\n  📊 Method: {} (confidence: {:.2})\n  💹 Price: {:.12} SOL per token\n  📈 Slippage: {:.3}%\n  💰 Total Fees: {:.6} SOL\n  🏦 ATA Detected: {}",
                analysis_time,
                true,
                consensus_result.method,
                consensus_result.confidence,
                effective_price,
                slippage_percent,
                total_fees_sol,
                if ata_detected {
                    "YES"
                } else {
                    "NO"
                }
            )
        );
    }

    if is_debug_profit_enabled() {
        log(
            LogTag::Wallet,
            "SWAP_ANALYSIS",
            &format!(
                "Analysis complete: method={}, confidence={:.2}, price={:.12}, slippage={:.3}%, time={}ms",
                consensus_result.method,
                consensus_result.confidence,
                effective_price,
                slippage_percent,
                analysis_time
            )
        );
    }

    Ok(SwapAnalysisResult {
        success: true,
        transaction_signature: transaction_signature.to_string(),
        error_message: None,
        input_amount: consensus_result.input_amount,
        output_amount: consensus_result.output_amount,
        input_decimals: consensus_result.input_decimals,
        output_decimals: consensus_result.output_decimals,
        input_amount_raw: input_raw,
        output_amount_raw: output_raw,
        effective_price,
        expected_price: intended_amount,
        price_difference_percent: price_diff_percent,
        slippage_percent,
        transaction_fee_sol: tx_fee_sol,
        transaction_fee_lamports: tx_fee_lamports,
        platform_fee_sol,
        total_fees_sol,
        ata_creation_detected: ata_detected,
        ata_rent_lamports,
        ata_rent_sol,
        analysis_method: consensus_result.method,
        confidence_score: consensus_result.confidence,
        analysis_time_ms: analysis_time,
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        is_buy: input_mint == SOL_MINT,
        wallet_address: wallet_address.to_string(),
        block_height: extract_block_height(&transaction_json),
        block_time: extract_block_time(&transaction_json),
    })
}

/// Method 2: Inner Instructions Analysis
pub async fn analyze_swap_inner_instructions(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    rpc_endpoint: &str,
    intended_amount: Option<f64>
) -> Result<SwapAnalysisResult, SwapError> {
    let start_time = std::time::Instant::now();

    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    let tx_response = get_transaction_details(client, transaction_signature, rpc_endpoint).await?;
    let transaction_json: Value = serde_json::from_str(&tx_response)?;

    let success = check_transaction_success(&transaction_json)?;
    if !success {
        return Err(SwapError::TransactionError("Transaction failed".to_string()));
    }

    let result = analyze_inner_instructions(
        &transaction_json,
        input_mint,
        output_mint,
        wallet_address
    )?;

    // Build result using inner instructions data
    build_swap_result(
        transaction_signature,
        &transaction_json,
        &result,
        input_mint,
        output_mint,
        wallet_address,
        intended_amount,
        start_time.elapsed().as_millis() as u64
    )
}

/// Method 3: Token Balance Changes Analysis
pub async fn analyze_swap_balance_changes(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    rpc_endpoint: &str,
    intended_amount: Option<f64>
) -> Result<SwapAnalysisResult, SwapError> {
    let start_time = std::time::Instant::now();

    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    let tx_response = get_transaction_details(client, transaction_signature, rpc_endpoint).await?;
    let transaction_json: Value = serde_json::from_str(&tx_response)?;

    let success = check_transaction_success(&transaction_json)?;
    if !success {
        return Err(SwapError::TransactionError("Transaction failed".to_string()));
    }

    let result = analyze_token_balances(
        &transaction_json,
        input_mint,
        output_mint,
        wallet_address
    )?;

    build_swap_result(
        transaction_signature,
        &transaction_json,
        &result,
        input_mint,
        output_mint,
        wallet_address,
        intended_amount,
        start_time.elapsed().as_millis() as u64
    )
}

/// Method 4: Log Messages Analysis
pub async fn analyze_swap_log_messages(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    rpc_endpoint: &str,
    intended_amount: Option<f64>
) -> Result<SwapAnalysisResult, SwapError> {
    let start_time = std::time::Instant::now();

    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    let tx_response = get_transaction_details(client, transaction_signature, rpc_endpoint).await?;
    let transaction_json: Value = serde_json::from_str(&tx_response)?;

    let success = check_transaction_success(&transaction_json)?;
    if !success {
        return Err(SwapError::TransactionError("Transaction failed".to_string()));
    }

    let result = analyze_log_messages(&transaction_json, input_mint, output_mint)?;

    build_swap_result(
        transaction_signature,
        &transaction_json,
        &result,
        input_mint,
        output_mint,
        wallet_address,
        intended_amount,
        start_time.elapsed().as_millis() as u64
    )
}

// Helper functions for analysis methods

fn check_transaction_success(transaction_json: &Value) -> Result<bool, SwapError> {
    if let Some(meta) = transaction_json.get("meta") {
        Ok(meta.get("err").is_none() || meta.get("err").unwrap().is_null())
    } else {
        Err(SwapError::InvalidResponse("Missing transaction metadata".to_string()))
    }
}

fn extract_error_message(transaction_json: &Value) -> Option<String> {
    transaction_json
        .get("meta")
        .and_then(|meta| meta.get("err"))
        .and_then(|err| err.as_str())
        .map(|s| s.to_string())
}

fn extract_block_height(transaction_json: &Value) -> Option<u64> {
    transaction_json.get("slot").and_then(|slot| slot.as_u64())
}

fn extract_block_time(transaction_json: &Value) -> Option<i64> {
    transaction_json.get("blockTime").and_then(|time| time.as_i64())
}

fn analyze_inner_instructions(
    transaction_json: &Value,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str
) -> Result<TokenTransferData, SwapError> {
    if is_debug_swap_enabled() {
        log(LogTag::Swap, "INNER_START", "🔍 Analyzing inner instructions for token transfers");
    }

    let meta = transaction_json
        .get("meta")
        .ok_or_else(|| SwapError::InvalidResponse("Missing metadata".to_string()))?;

    let inner_instructions = meta
        .get("innerInstructions")
        .ok_or_else(|| SwapError::InvalidResponse("Missing inner instructions".to_string()))?
        .as_array()
        .ok_or_else(|| SwapError::InvalidResponse("Inner instructions not an array".to_string()))?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "INNER_COUNT",
            &format!("📋 Found {} inner instruction groups", inner_instructions.len())
        );
    }

    let mut input_amount = 0.0;
    let mut output_amount = 0.0;
    let mut input_decimals = 0u8;
    let mut output_decimals = 0u8;
    let mut transfer_count = 0;
    let mut found_wallet_input = false;
    let mut found_wallet_output = false;

    for inner_ix_group in inner_instructions {
        if let Some(instructions) = inner_ix_group.get("instructions").and_then(|i| i.as_array()) {
            for instruction in instructions {
                if let Some(parsed) = instruction.get("parsed") {
                    if let Some(info) = parsed.get("info") {
                        if let Some(instruction_type) = parsed.get("type").and_then(|t| t.as_str()) {
                            // Handle both transferChecked and regular transfer instructions
                            if
                                instruction_type == "transferChecked" ||
                                instruction_type == "transfer"
                            {
                                let mint = if instruction_type == "transferChecked" {
                                    info.get("mint")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("")
                                } else {
                                    // For regular transfer, we need to look at account keys or infer from context
                                    ""
                                };

                                let amount = if instruction_type == "transferChecked" {
                                    info.get("tokenAmount")
                                        .and_then(|ta| ta.get("uiAmount"))
                                        .and_then(|ua| ua.as_f64())
                                        .unwrap_or(0.0)
                                } else {
                                    // For regular transfer, get lamports and convert if it's SOL
                                    info.get("lamports")
                                        .and_then(|l| l.as_u64())
                                        .map(|l| lamports_to_sol(l))
                                        .unwrap_or(0.0)
                                };

                                let decimals = if instruction_type == "transferChecked" {
                                    info
                                        .get("tokenAmount")
                                        .and_then(|ta| ta.get("decimals"))
                                        .and_then(|d| d.as_u64())
                                        .unwrap_or(0) as u8
                                } else {
                                    9 // SOL decimals
                                };

                                let source = info
                                    .get("source")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("");
                                let destination = info
                                    .get("destination")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("");

                                // Check for wallet involvement in transfers
                                let wallet_in_source =
                                    source.contains(wallet_address) || source == wallet_address;
                                let wallet_in_dest =
                                    destination.contains(wallet_address) ||
                                    destination == wallet_address;

                                // Determine if this is input or output based on wallet involvement and mint
                                if
                                    (mint == input_mint ||
                                        (mint.is_empty() && input_mint == SOL_MINT)) &&
                                    wallet_in_source
                                {
                                    input_amount = amount;
                                    input_decimals = decimals;
                                    transfer_count += 1;
                                    found_wallet_input = true;

                                    if is_debug_swap_enabled() {
                                        log(
                                            LogTag::Swap,
                                            "INNER_INPUT",
                                            &format!(
                                                "📤 INPUT transfer: {:.6} {} (decimals: {}) from {} to {}",
                                                amount,
                                                if mint == SOL_MINT || mint.is_empty() {
                                                    "SOL"
                                                } else {
                                                    &mint[..8]
                                                },
                                                decimals,
                                                &source[..8],
                                                &destination[..8]
                                            )
                                        );
                                    }
                                } else if
                                    (mint == output_mint ||
                                        (mint.is_empty() && output_mint == SOL_MINT)) &&
                                    wallet_in_dest
                                {
                                    output_amount = amount;
                                    output_decimals = decimals;
                                    transfer_count += 1;
                                    found_wallet_output = true;

                                    if is_debug_swap_enabled() {
                                        log(
                                            LogTag::Swap,
                                            "INNER_OUTPUT",
                                            &format!(
                                                "📥 OUTPUT transfer: {:.6} {} (decimals: {}) from {} to {}",
                                                amount,
                                                if mint == SOL_MINT || mint.is_empty() {
                                                    "SOL"
                                                } else {
                                                    &mint[..8]
                                                },
                                                decimals,
                                                &source[..8],
                                                &destination[..8]
                                            )
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Handle SOL transfers for SOL-token swaps - use balance changes for more accuracy
    if input_mint == SOL_MINT || output_mint == SOL_MINT {
        match calculate_sol_balance_change(transaction_json, wallet_address) {
            Ok(sol_change) => {
                if input_mint == SOL_MINT && (!found_wallet_input || input_amount == 0.0) {
                    input_amount = sol_change;
                    input_decimals = 9;
                    found_wallet_input = true;

                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "INNER_SOL_IN",
                            &format!("💰 SOL input amount: {:.6} SOL", sol_change)
                        );
                    }
                } else if output_mint == SOL_MINT && (!found_wallet_output || output_amount == 0.0) {
                    output_amount = sol_change;
                    output_decimals = 9;
                    found_wallet_output = true;

                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "INNER_SOL_OUT",
                            &format!("💰 SOL output amount: {:.6} SOL", sol_change)
                        );
                    }
                }
            }
            Err(e) => {
                if is_debug_swap_enabled() {
                    log(
                        LogTag::Swap,
                        "INNER_SOL_ERROR",
                        &format!("⚠️ Failed to calculate SOL balance change: {}", e)
                    );
                }
            }
        }
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "INNER_RESULT",
            &format!(
                "📊 Inner instructions analysis - Transfers: {}, Input: {:.6} (decimals: {}), Output: {:.6} (decimals: {})",
                transfer_count,
                input_amount,
                input_decimals,
                output_amount,
                output_decimals
            )
        );
    }

    // Require both input and output amounts to be found for success
    if input_amount > 0.0 && output_amount > 0.0 && found_wallet_input && found_wallet_output {
        Ok(TokenTransferData {
            input_amount,
            output_amount,
            input_decimals,
            output_decimals,
            confidence: 0.95,
            method: "Inner Instructions".to_string(),
        })
    } else {
        Err(
            SwapError::InvalidResponse(
                format!(
                    "Could not extract transfer amounts from inner instructions. Input: {:.6}, Output: {:.6}, WalletInput: {}, WalletOutput: {}",
                    input_amount,
                    output_amount,
                    found_wallet_input,
                    found_wallet_output
                )
            )
        )
    }
}

fn analyze_token_balances(
    transaction_json: &Value,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str
) -> Result<TokenTransferData, SwapError> {
    let meta = transaction_json
        .get("meta")
        .ok_or_else(|| SwapError::InvalidResponse("Missing metadata".to_string()))?;

    let empty_vec = vec![];
    let pre_token_balances = meta
        .get("preTokenBalances")
        .and_then(|b| b.as_array())
        .unwrap_or(&empty_vec);
    let post_token_balances = meta
        .get("postTokenBalances")
        .and_then(|b| b.as_array())
        .unwrap_or(&empty_vec);

    let mut input_amount = 0.0;
    let mut output_amount = 0.0;
    let mut input_decimals = 0u8;
    let mut output_decimals = 0u8;

    // Find token balance changes
    let input_change = calculate_token_balance_change(
        pre_token_balances,
        post_token_balances,
        input_mint,
        wallet_address
    )?;
    let output_change = calculate_token_balance_change(
        pre_token_balances,
        post_token_balances,
        output_mint,
        wallet_address
    )?;

    // Get decimals
    input_decimals = get_decimals_from_balances(
        pre_token_balances,
        post_token_balances,
        input_mint
    )?;
    output_decimals = get_decimals_from_balances(
        pre_token_balances,
        post_token_balances,
        output_mint
    )?;

    // Handle SOL separately
    if input_mint == SOL_MINT || output_mint == SOL_MINT {
        let sol_change = calculate_sol_balance_change(transaction_json, wallet_address)?;
        if input_mint == SOL_MINT {
            input_amount = sol_change;
            input_decimals = 9;
            output_amount = output_change;
        } else {
            input_amount = input_change;
            output_amount = sol_change;
            output_decimals = 9;
        }
    } else {
        input_amount = input_change;
        output_amount = output_change;
    }

    if input_amount > 0.0 && output_amount > 0.0 {
        Ok(TokenTransferData {
            input_amount,
            output_amount,
            input_decimals,
            output_decimals,
            confidence: 0.9,
            method: "Token Balances".to_string(),
        })
    } else {
        Err(SwapError::InvalidResponse("Could not extract amounts from token balances".to_string()))
    }
}

fn analyze_log_messages(
    transaction_json: &Value,
    input_mint: &str,
    output_mint: &str
) -> Result<TokenTransferData, SwapError> {
    if is_debug_swap_enabled() {
        log(LogTag::Swap, "LOG_START", "🔍 Analyzing log messages for swap patterns");
    }

    let meta = transaction_json
        .get("meta")
        .ok_or_else(|| SwapError::InvalidResponse("Missing metadata".to_string()))?;

    if let Some(log_messages) = meta.get("logMessages").and_then(|logs| logs.as_array()) {
        if is_debug_swap_enabled() {
            log(
                LogTag::Swap,
                "LOG_COUNT",
                &format!("📋 Found {} log messages to analyze", log_messages.len())
            );
        }

        for (i, log_msg) in log_messages.iter().enumerate() {
            if let Some(log_text) = log_msg.as_str() {
                if is_debug_swap_enabled() && i < 5 {
                    // Only log first 5 for debugging
                    log(
                        LogTag::Swap,
                        "LOG_ENTRY",
                        &format!(
                            "🔍 Log {}: {}",
                            i + 1,
                            &log_text[..std::cmp::min(100, log_text.len())]
                        )
                    );
                }

                // Try to parse different swap log formats
                if let Ok(parsed) = parse_swap_log(log_text, input_mint, output_mint) {
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "LOG_PARSED",
                            &format!("✅ Successfully parsed swap from log message")
                        );
                    }
                    return Ok(parsed);
                }
            }
        }
    }

    if is_debug_swap_enabled() {
        log(LogTag::Swap, "LOG_FAILED", "❌ No recognizable swap patterns found in logs");
    }

    Err(SwapError::InvalidResponse("No recognizable swap logs found".to_string()))
}

fn calculate_token_balance_change(
    pre_balances: &[Value],
    post_balances: &[Value],
    mint: &str,
    wallet_address: &str
) -> Result<f64, SwapError> {
    let mut pre_amount = 0.0;
    let mut post_amount = 0.0;

    // Find pre-balance
    for balance in pre_balances {
        if
            let (Some(balance_mint), Some(ui_amount)) = (
                balance.get("mint").and_then(|m| m.as_str()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|ta| ta.get("uiAmount"))
                    .and_then(|ua| ua.as_f64()),
            )
        {
            if balance_mint == mint {
                if let Some(owner) = balance.get("owner").and_then(|o| o.as_str()) {
                    if owner == wallet_address {
                        pre_amount = ui_amount;
                        break;
                    }
                }
            }
        }
    }

    // Find post-balance
    for balance in post_balances {
        if
            let (Some(balance_mint), Some(ui_amount)) = (
                balance.get("mint").and_then(|m| m.as_str()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|ta| ta.get("uiAmount"))
                    .and_then(|ua| ua.as_f64()),
            )
        {
            if balance_mint == mint {
                if let Some(owner) = balance.get("owner").and_then(|o| o.as_str()) {
                    if owner == wallet_address {
                        post_amount = ui_amount;
                        break;
                    }
                }
            }
        }
    }

    // Return the actual change (positive = received, negative = spent)
    // But since we're dealing with amounts, return absolute value
    // The sign logic is handled in the calling function
    let change = post_amount - pre_amount;
    Ok(change.abs())
}

fn calculate_sol_balance_change(
    transaction_json: &Value,
    wallet_address: &str
) -> Result<f64, SwapError> {
    let meta = transaction_json
        .get("meta")
        .ok_or_else(|| SwapError::InvalidResponse("Missing metadata".to_string()))?;

    let transaction = transaction_json
        .get("transaction")
        .ok_or_else(|| SwapError::InvalidResponse("Missing transaction".to_string()))?;

    let message = transaction
        .get("message")
        .ok_or_else(|| SwapError::InvalidResponse("Missing message".to_string()))?;

    let account_keys = message
        .get("accountKeys")
        .ok_or_else(|| SwapError::InvalidResponse("Missing accountKeys".to_string()))?
        .as_array()
        .ok_or_else(|| SwapError::InvalidResponse("accountKeys not array".to_string()))?;

    // Find wallet index
    let mut wallet_index = None;
    for (i, key) in account_keys.iter().enumerate() {
        if let Some(pubkey) = key.as_str() {
            if pubkey == wallet_address {
                wallet_index = Some(i);
                break;
            }
        }
    }

    let wallet_index = wallet_index.ok_or_else(||
        SwapError::InvalidResponse("Wallet not found in transaction".to_string())
    )?;

    let pre_balances = meta
        .get("preBalances")
        .ok_or_else(|| SwapError::InvalidResponse("Missing preBalances".to_string()))?
        .as_array()
        .ok_or_else(|| SwapError::InvalidResponse("preBalances not array".to_string()))?;

    let post_balances = meta
        .get("postBalances")
        .ok_or_else(|| SwapError::InvalidResponse("Missing postBalances".to_string()))?
        .as_array()
        .ok_or_else(|| SwapError::InvalidResponse("postBalances not array".to_string()))?;

    let pre_balance = pre_balances
        .get(wallet_index)
        .ok_or_else(||
            SwapError::InvalidResponse("Wallet index out of bounds in preBalances".to_string())
        )?
        .as_u64()
        .ok_or_else(|| SwapError::InvalidResponse("Invalid preBalance".to_string()))?;

    let post_balance = post_balances
        .get(wallet_index)
        .ok_or_else(||
            SwapError::InvalidResponse("Wallet index out of bounds in postBalances".to_string())
        )?
        .as_u64()
        .ok_or_else(|| SwapError::InvalidResponse("Invalid postBalance".to_string()))?;

    // Calculate actual SOL change (positive = received, negative = spent)
    let sol_change_lamports = (post_balance as i64) - (pre_balance as i64);

    // Exclude transaction fee from the calculation for better accuracy
    let fee = meta
        .get("fee")
        .and_then(|f| f.as_u64())
        .unwrap_or(0) as i64;

    // For SOL outgoing (buying tokens), add back the fee to get pure trade amount
    // For SOL incoming (selling tokens), the fee was already deducted from balance
    let adjusted_lamports = if sol_change_lamports < 0 {
        // Spent SOL: remove fee from the spent amount to get pure trade
        (sol_change_lamports + fee).abs() as u64
    } else {
        // Received SOL: use as-is (fee already deducted)
        sol_change_lamports as u64
    };

    Ok(lamports_to_sol(adjusted_lamports))
}

fn get_decimals_from_balances(
    pre_balances: &[Value],
    post_balances: &[Value],
    mint: &str
) -> Result<u8, SwapError> {
    // Try post balances first
    for balance in post_balances {
        if let Some(balance_mint) = balance.get("mint").and_then(|m| m.as_str()) {
            if balance_mint == mint {
                if
                    let Some(decimals) = balance
                        .get("uiTokenAmount")
                        .and_then(|ta| ta.get("decimals"))
                        .and_then(|d| d.as_u64())
                {
                    return Ok(decimals as u8);
                }
            }
        }
    }

    // Try pre balances
    for balance in pre_balances {
        if let Some(balance_mint) = balance.get("mint").and_then(|m| m.as_str()) {
            if balance_mint == mint {
                if
                    let Some(decimals) = balance
                        .get("uiTokenAmount")
                        .and_then(|ta| ta.get("decimals"))
                        .and_then(|d| d.as_u64())
                {
                    return Ok(decimals as u8);
                }
            }
        }
    }

    // Default decimals
    if mint == SOL_MINT {
        Ok(9)
    } else {
        Ok(6) // Common default for SPL tokens
    }
}

fn parse_swap_log(
    log_text: &str,
    input_mint: &str,
    output_mint: &str
) -> Result<TokenTransferData, SwapError> {
    // Parse various DEX log formats

    // Jupiter/Meteora swap logs often contain swap amounts
    if log_text.contains("Swap") || log_text.contains("swap") {
        // Look for numeric patterns that might be amounts
        let numbers: Vec<&str> = log_text
            .split_whitespace()
            .filter(|s| s.chars().all(|c| (c.is_numeric() || c == '.')))
            .collect();

        if numbers.len() >= 2 {
            if let (Ok(first), Ok(second)) = (numbers[0].parse::<f64>(), numbers[1].parse::<f64>()) {
                // Try to determine which is input and which is output
                // This is a heuristic approach - might need refinement based on actual log formats
                return Ok(TokenTransferData {
                    input_amount: first,
                    output_amount: second,
                    input_decimals: if input_mint == SOL_MINT {
                        9
                    } else {
                        6
                    },
                    output_decimals: if output_mint == SOL_MINT {
                        9
                    } else {
                        6
                    },
                    confidence: 0.7, // Lower confidence since this is pattern matching
                    method: "Log Messages".to_string(),
                });
            }
        }
    }

    // Jupiter V6 specific log patterns
    if log_text.contains("jupiter") || log_text.contains("Jupiter") {
        // Look for structured log data
        if let Some(amount_in_pos) = log_text.find("amountIn:") {
            if let Some(amount_out_pos) = log_text.find("amountOut:") {
                let amount_in_part = &log_text[amount_in_pos + 9..];
                let amount_out_part = &log_text[amount_out_pos + 10..];

                if
                    let (Some(in_end), Some(out_end)) = (
                        amount_in_part.find(' ').or_else(|| amount_in_part.find(',')),
                        amount_out_part.find(' ').or_else(|| amount_out_part.find(',')),
                    )
                {
                    let in_str = &amount_in_part[..in_end];
                    let out_str = &amount_out_part[..out_end];

                    if
                        let (Ok(amount_in), Ok(amount_out)) = (
                            in_str.parse::<f64>(),
                            out_str.parse::<f64>(),
                        )
                    {
                        return Ok(TokenTransferData {
                            input_amount: amount_in,
                            output_amount: amount_out,
                            input_decimals: if input_mint == SOL_MINT {
                                9
                            } else {
                                6
                            },
                            output_decimals: if output_mint == SOL_MINT {
                                9
                            } else {
                                6
                            },
                            confidence: 0.85,
                            method: "Log Messages".to_string(),
                        });
                    }
                }
            }
        }
    }

    // Raydium swap log patterns
    if log_text.contains("raydium") || log_text.contains("Raydium") {
        // Parse Raydium specific log formats
        if log_text.contains("SwapEvent") {
            // Look for amount patterns in Raydium logs
            let parts: Vec<&str> = log_text.split(',').collect();
            let mut amounts: Vec<f64> = Vec::new();

            for part in parts {
                if let Some(colon_pos) = part.find(':') {
                    let value_part = &part[colon_pos + 1..].trim();
                    if let Ok(amount) = value_part.parse::<f64>() {
                        amounts.push(amount);
                    }
                }
            }

            if amounts.len() >= 2 {
                return Ok(TokenTransferData {
                    input_amount: amounts[0],
                    output_amount: amounts[1],
                    input_decimals: if input_mint == SOL_MINT {
                        9
                    } else {
                        6
                    },
                    output_decimals: if output_mint == SOL_MINT {
                        9
                    } else {
                        6
                    },
                    confidence: 0.8,
                    method: "Log Messages".to_string(),
                });
            }
        }
    }

    // Meteora DLMM specific patterns
    if log_text.contains("meteora") || log_text.contains("Meteora") || log_text.contains("DLMM") {
        // Look for swap amounts in Meteora logs
        if log_text.contains("amount") {
            if let Ok(amount_regex) = Regex::new(r"amount[^:]*:\s*(\d+(?:\.\d+)?)") {
                let amounts: Vec<f64> = amount_regex
                    .captures_iter(log_text)
                    .filter_map(|cap| cap.get(1)?.as_str().parse().ok())
                    .collect();

                if amounts.len() >= 2 {
                    return Ok(TokenTransferData {
                        input_amount: amounts[0],
                        output_amount: amounts[1],
                        input_decimals: if input_mint == SOL_MINT {
                            9
                        } else {
                            6
                        },
                        output_decimals: if output_mint == SOL_MINT {
                            9
                        } else {
                            6
                        },
                        confidence: 0.75,
                        method: "Log Messages".to_string(),
                    });
                }
            }
        }
    }

    // Generic numeric extraction as fallback
    if log_text.len() > 20 && (log_text.contains("amount") || log_text.contains("transfer")) {
        // Extract all decimal numbers from the log
        if let Ok(number_regex) = Regex::new(r"\d+(?:\.\d+)?") {
            let numbers: Vec<f64> = number_regex
                .find_iter(log_text)
                .filter_map(|m| m.as_str().parse().ok())
                .filter(|&n| n > 0.0 && n < 1e12) // Filter reasonable amounts
                .collect();

            if numbers.len() >= 2 {
                // Use the first two reasonable amounts
                return Ok(TokenTransferData {
                    input_amount: numbers[0],
                    output_amount: numbers[1],
                    input_decimals: if input_mint == SOL_MINT {
                        9
                    } else {
                        6
                    },
                    output_decimals: if output_mint == SOL_MINT {
                        9
                    } else {
                        6
                    },
                    confidence: 0.6, // Lower confidence for generic parsing
                    method: "Log Messages".to_string(),
                });
            }
        }
    }

    Err(SwapError::InvalidResponse("No recognizable swap pattern in log".to_string()))
}

fn calculate_consensus_result(
    valid_results: Vec<TokenTransferData>,
    _intended_amount: Option<f64>
) -> Result<TokenTransferData, SwapError> {
    if valid_results.is_empty() {
        return Err(
            SwapError::InvalidResponse("No valid results to calculate consensus".to_string())
        );
    }

    // For now, return the result with highest confidence
    // You can implement more sophisticated consensus logic here
    let best_result = valid_results
        .into_iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
        .unwrap();

    Ok(best_result)
}

fn extract_transaction_fee(transaction_json: &Value) -> (u64, f64) {
    let fee_lamports = transaction_json
        .get("meta")
        .and_then(|meta| meta.get("fee"))
        .and_then(|fee| fee.as_u64())
        .unwrap_or(0);

    (fee_lamports, lamports_to_sol(fee_lamports))
}

fn extract_platform_fee(transaction_json: &Value) -> Option<f64> {
    // Look for platform-specific fees in logs
    if let Some(meta) = transaction_json.get("meta") {
        if let Some(logs) = meta.get("logMessages").and_then(|l| l.as_array()) {
            for log in logs {
                if let Some(log_text) = log.as_str() {
                    if log_text.contains("platform fee") || log_text.contains("Platform Fee") {
                        // Parse platform fee from log message
                        // This is implementation specific to each DEX
                    }
                }
            }
        }
    }
    None
}

/// Comprehensive ATA detection with multiple strategies
/// Detects both ATA creation (rent spent) and ATA closure (rent reclaimed)
/// Analyzes transaction logs, instructions, and balance changes for accurate detection
fn detect_ata_creation(transaction_json: &Value, wallet_address: &str) -> (bool, u64, f64) {
    let mut ata_rent_spent = 0u64;
    let mut ata_rent_reclaimed = 0u64;
    let mut wsol_ata_detected = false;
    let mut confidence_score = 0.0;

    // Method 1: Analyze log messages for ATA operations
    if let Some(meta) = transaction_json.get("meta") {
        if let Some(log_messages) = meta.get("logMessages").and_then(|logs| logs.as_array()) {
            for log in log_messages {
                if let Some(log_str) = log.as_str() {
                    // Check for various ATA creation patterns
                    if log_str.contains("CreateAccount") || log_str.contains("InitializeAccount") {
                        ata_rent_spent += 2_039_280; // Standard ATA rent
                        confidence_score += 0.4;
                    }

                    // Check for ATA close operations (rent reclaimed)
                    if log_str.contains("CloseAccount") || log_str.contains("close_account") {
                        ata_rent_reclaimed += 2_039_280;
                        confidence_score += 0.4;
                    }

                    // Check for WSOL ATA operations (common in swaps)
                    if log_str.contains("So11111111111111111111111111111111111111112") {
                        wsol_ata_detected = true;
                        confidence_score += 0.2;
                    }

                    // Check for specific SPL Token operations
                    if log_str.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") {
                        confidence_score += 0.1;
                    }
                }
            }
        }

        // Method 2: Analyze inner instructions for account creation/closure
        if
            let Some(inner_instructions) = meta
                .get("innerInstructions")
                .and_then(|ii| ii.as_array())
        {
            for inner_ix_group in inner_instructions {
                if
                    let Some(instructions) = inner_ix_group
                        .get("instructions")
                        .and_then(|i| i.as_array())
                {
                    for instruction in instructions {
                        if let Some(parsed) = instruction.get("parsed") {
                            if
                                let Some(instruction_type) = parsed
                                    .get("type")
                                    .and_then(|t| t.as_str())
                            {
                                match instruction_type {
                                    "createAccount" | "create" => {
                                        // Analyze account creation details
                                        if let Some(info) = parsed.get("info") {
                                            if
                                                let Some(space) = info
                                                    .get("space")
                                                    .and_then(|s| s.as_u64())
                                            {
                                                // Token account space is typically 165 bytes
                                                if space == 165 {
                                                    ata_rent_spent += 2_039_280;
                                                    confidence_score += 0.5;
                                                }
                                            }
                                        }
                                    }
                                    "closeAccount" | "close" => {
                                        // ATA closure detected
                                        ata_rent_reclaimed += 2_039_280;
                                        confidence_score += 0.5;
                                    }
                                    "initializeAccount" => {
                                        // Token account initialization
                                        confidence_score += 0.3;
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Check raw instruction data for program IDs
                        if
                            let Some(program_id) = instruction
                                .get("programId")
                                .and_then(|p| p.as_str())
                        {
                            // SPL Token program
                            if program_id == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" {
                                confidence_score += 0.1;
                            }
                            // Associated Token Account program
                            if program_id == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
                                ata_rent_spent += 2_039_280;
                                confidence_score += 0.6;
                            }
                        }
                    }
                }
            }
        }

        // Method 3: Analyze SOL balance changes for ATA rent patterns
        if let Some(pre_balances) = meta.get("preBalances").and_then(|pb| pb.as_array()) {
            if let Some(post_balances) = meta.get("postBalances").and_then(|pb| pb.as_array()) {
                // Find wallet's balance change
                if
                    let Some(account_keys) = transaction_json
                        .get("transaction")
                        .and_then(|tx| tx.get("message"))
                        .and_then(|msg| msg.get("accountKeys"))
                        .and_then(|ak| ak.as_array())
                {
                    for (i, account) in account_keys.iter().enumerate() {
                        if let Some(account_str) = account.as_str() {
                            if account_str == wallet_address {
                                if
                                    let (Some(pre_bal), Some(post_bal)) = (
                                        pre_balances.get(i).and_then(|b| b.as_u64()),
                                        post_balances.get(i).and_then(|b| b.as_u64()),
                                    )
                                {
                                    let balance_diff = if pre_bal > post_bal {
                                        pre_bal - post_bal
                                    } else {
                                        post_bal - pre_bal
                                    };

                                    // Check if balance change indicates ATA rent
                                    // Common patterns: 2,039,280 (ATA rent) ± transaction fees
                                    if balance_diff >= 2_030_000 && balance_diff <= 2_050_000 {
                                        if pre_bal > post_bal {
                                            ata_rent_spent += 2_039_280;
                                        } else {
                                            ata_rent_reclaimed += 2_039_280;
                                        }
                                        confidence_score += 0.3;
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Calculate net ATA impact (spent - reclaimed)
    let net_ata_rent = if ata_rent_spent >= ata_rent_reclaimed {
        ata_rent_spent - ata_rent_reclaimed
    } else {
        0 // If more reclaimed than spent, no net cost
    };

    // Determine if ATA activity was detected with sufficient confidence
    let ata_detected = confidence_score >= 0.4 || net_ata_rent > 0;

    if ata_detected && is_debug_profit_enabled() {
        log(
            LogTag::Wallet,
            "ATA_DETECT",
            &format!(
                "ATA detected: spent={} lamports, reclaimed={} lamports, net={} lamports, WSOL={}, confidence={:.2}",
                ata_rent_spent,
                ata_rent_reclaimed,
                net_ata_rent,
                wsol_ata_detected,
                confidence_score
            )
        );
    }

    (ata_detected, net_ata_rent, lamports_to_sol(net_ata_rent))
}

fn build_swap_result(
    transaction_signature: &str,
    transaction_json: &Value,
    result: &TokenTransferData,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    intended_amount: Option<f64>,
    analysis_time_ms: u64
) -> Result<SwapAnalysisResult, SwapError> {
    let (tx_fee_lamports, tx_fee_sol) = extract_transaction_fee(transaction_json);
    let platform_fee_sol = extract_platform_fee(transaction_json);
    let total_fees_sol = tx_fee_sol + platform_fee_sol.unwrap_or(0.0);
    let (ata_detected, ata_rent_lamports, ata_rent_sol) = detect_ata_creation(
        transaction_json,
        wallet_address
    );

    let effective_price = if input_mint == SOL_MINT {
        result.input_amount / result.output_amount
    } else {
        result.output_amount / result.input_amount
    };

    let (price_diff_percent, slippage_percent) = if let Some(expected) = intended_amount {
        let price_diff = ((effective_price - expected) / expected) * 100.0;
        let slippage = price_diff.abs();
        (price_diff, slippage)
    } else {
        (0.0, 0.0)
    };

    let input_raw = (result.input_amount * (10_f64).powi(result.input_decimals as i32)) as u64;
    let output_raw = (result.output_amount * (10_f64).powi(result.output_decimals as i32)) as u64;

    Ok(SwapAnalysisResult {
        success: true,
        transaction_signature: transaction_signature.to_string(),
        error_message: None,
        input_amount: result.input_amount,
        output_amount: result.output_amount,
        input_decimals: result.input_decimals,
        output_decimals: result.output_decimals,
        input_amount_raw: input_raw,
        output_amount_raw: output_raw,
        effective_price,
        expected_price: intended_amount,
        price_difference_percent: price_diff_percent,
        slippage_percent,
        transaction_fee_sol: tx_fee_sol,
        transaction_fee_lamports: tx_fee_lamports,
        platform_fee_sol,
        total_fees_sol,
        ata_creation_detected: ata_detected,
        ata_rent_lamports,
        ata_rent_sol,
        analysis_method: result.method.clone(),
        confidence_score: result.confidence,
        analysis_time_ms,
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        is_buy: input_mint == SOL_MINT,
        wallet_address: wallet_address.to_string(),
        block_height: extract_block_height(transaction_json),
        block_time: extract_block_time(transaction_json),
    })
}
