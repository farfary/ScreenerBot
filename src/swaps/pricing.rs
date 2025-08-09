/// Price calculation and validation functions for swap operations
/// Handles effective price calculations, quote validation, and price comparisons

use crate::global::is_debug_swap_enabled;
use crate::logger::{log, LogTag};
use crate::rpc::{SwapError, lamports_to_sol, sol_to_lamports};
use crate::swaps::types::{SwapData, SwapRequest};
use crate::swaps::interface::SwapResult;
use super::config::INTERNAL_SLIPPAGE_PERCENT;
use crate::tokens::decimals::get_token_decimals_from_chain;
use super::config::SOL_MINT;

/// Calculate effective price per token for any swap operation
/// Supports both buy and sell operations with proper decimal handling
/// Accounts for ATA rent and transaction fees
pub async fn calculate_effective_price(
    swap_result: &SwapResult,
    input_mint: &str,
    output_mint: &str,
    direction: &str, // "buy" or "sell"
    ata_rent_reclaimed: Option<u64>, // ATA rent from closure (lamports)
) -> Result<f64, SwapError> {
    if !swap_result.success {
        return Err(SwapError::InvalidAmount("Cannot calculate price from failed swap".to_string()));
    }

    // Parse input amount
    let input_amount_raw: u64 = swap_result.input_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid input amount in swap result".to_string()))?;

    // Parse output amount  
    let output_amount_raw: u64 = swap_result.output_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid output amount in swap result".to_string()))?;

    if input_amount_raw == 0 || output_amount_raw == 0 {
        return Err(SwapError::InvalidAmount("Cannot calculate price with zero amounts".to_string()));
    }

    // Get token decimals from swap data or chain
    let (input_decimals, output_decimals) = if let Some(swap_data) = &swap_result.swap_data {
        (swap_data.quote.in_decimals as u32, swap_data.quote.out_decimals as u32)
    } else {
        // Fallback: fetch decimals from chain
        let input_dec = if input_mint == SOL_MINT { 9 } else {
            get_token_decimals_from_chain(input_mint).await.unwrap_or(9)
        };
        let output_dec = if output_mint == SOL_MINT { 9 } else {
            get_token_decimals_from_chain(output_mint).await.unwrap_or(9)
        };
        (input_dec as u32, output_dec as u32)
    };

    let effective_price = match direction {
        "buy" => {
            // Buy: SOL -> Token
            // Calculate SOL per token
            let input_sol = lamports_to_sol(input_amount_raw);
            let output_tokens = (output_amount_raw as f64) / (10_f64).powi(output_decimals as i32);
            
            if output_tokens <= 0.0 {
                return Err(SwapError::InvalidAmount("Invalid token output amount".to_string()));
            }
            
            input_sol / output_tokens
        }
        "sell" => {
            // Sell: Token -> SOL
            // Calculate SOL per token using ONLY SOL received from the swap (exclude ATA rent)
            let input_tokens = (input_amount_raw as f64) / (10_f64).powi(input_decimals as i32);
            let output_sol = lamports_to_sol(output_amount_raw);

            if input_tokens <= 0.0 {
                return Err(SwapError::InvalidAmount("Invalid token input amount".to_string()));
            }

            output_sol / input_tokens
        }
        _ => return Err(SwapError::InvalidAmount(format!("Invalid direction: {}", direction)))
    };

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "PRICE_CALC",
            &format!(
                "💰 EFFECTIVE PRICE CALCULATION ({}):\n  📥 Input: {} {} (raw: {})\n  📤 Output: {} {} (raw: {})\n  🔢 Decimals: in={}, out={}\n  🏦 ATA Rent: {} SOL\n  💎 Effective Price: {:.10} SOL per token",
                direction.to_uppercase(),
                if direction == "buy" { 
                    format!("{:.6} SOL", lamports_to_sol(input_amount_raw))
                } else { 
                    format!("{:.6} tokens", (input_amount_raw as f64) / (10_f64).powi(input_decimals as i32))
                },
                if input_mint == SOL_MINT { "SOL" } else { "tokens" },
                input_amount_raw,
                if direction == "buy" { 
                    format!("{:.6} tokens", (output_amount_raw as f64) / (10_f64).powi(output_decimals as i32))
                } else { 
                    format!("{:.6} SOL", lamports_to_sol(output_amount_raw))
                },
                if output_mint == SOL_MINT { "SOL" } else { "tokens" },
                output_amount_raw,
                input_decimals,
                output_decimals,
                ata_rent_reclaimed.map(lamports_to_sol).unwrap_or(0.0),
                effective_price
            )
        );
    }

    Ok(effective_price)
}

/// Alternative signature for direct calculation from raw transaction data
/// Used by the transaction verification system
/// FIXED: Now properly handles instruction vs quote data mismatches
pub fn calculate_effective_price_from_raw(
    expected_direction: &str,
    input_amount: Option<u64>,
    output_amount: Option<u64>,
    sol_spent: Option<u64>,
    sol_received: Option<u64>,
    ata_rent_reclaimed: u64,
    input_decimals: u32,
    output_decimals: u32,
) -> Option<f64> {
    calculate_effective_price_from_raw_with_quote(
        expected_direction,
        input_amount,
        output_amount,
        sol_spent,
        sol_received,
        ata_rent_reclaimed,
        input_decimals,
        output_decimals,
        None, // quote_input_amount
        None, // quote_output_amount
    )
}

/// Enhanced version with quote fallback amounts for instruction parsing failures
pub fn calculate_effective_price_from_raw_with_quote(
    expected_direction: &str,
    input_amount: Option<u64>,
    output_amount: Option<u64>,
    sol_spent: Option<u64>,
    sol_received: Option<u64>,
    ata_rent_reclaimed: u64,
    input_decimals: u32,
    output_decimals: u32,
    quote_input_amount: Option<u64>,
    quote_output_amount: Option<u64>,
) -> Option<f64> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "PRICE_DEBUG_RAW",
            &format!(
                "🔍 Raw Price Calculation Debug:
  Direction: {}
  Input amount: {:?} (decimals: {})
  Output amount: {:?} (decimals: {})
  SOL spent: {:?}
  SOL received: {:?}
  ATA rent reclaimed: {} lamports
  Quote input: {:?}
  Quote output: {:?}",
                expected_direction,
                input_amount,
                input_decimals,
                output_amount,
                output_decimals,
                sol_spent,
                sol_received,
                ata_rent_reclaimed,
                quote_input_amount,
                quote_output_amount
            )
        );
    }

    match expected_direction {
        "buy" => {
            // Buy: calculate SOL per token using the quoted input amount and tokens received
            // Use input_amount (quoted amount) rather than sol_spent (which includes fees/rent)
            // Fallback to quote amounts if instruction analysis failed
            let sol_amount = input_amount.or(quote_input_amount);
            let token_amount = output_amount.or(quote_output_amount);
            
            if let (Some(sol_val), Some(tokens_val)) = (sol_amount, token_amount) {
                if tokens_val > 0 && sol_val > 0 {
                    let sol_spent_actual = lamports_to_sol(sol_val);
                    let tokens_received_actual = (tokens_val as f64) / (10_f64).powi(output_decimals as i32);
                    
                    if tokens_received_actual > 0.0 {
                        let price = sol_spent_actual / tokens_received_actual;
                        
                        if is_debug_swap_enabled() {
                            log(
                                LogTag::Swap,
                                "PRICE_CALC_BUY",
                                &format!(
                                    "📊 BUY Price Calculation (input_amount method):
  SOL input (quoted): {} lamports = {:.9} SOL
  Tokens received: {} raw = {:.6} tokens
  Effective price: {:.10} SOL per token",
                                    sol_val,
                                    sol_spent_actual,
                                    tokens_val,
                                    tokens_received_actual,
                                    price
                                )
                            );
                        }
                        
                        return Some(price);
                    }
                }
            }
            
            // Fallback: If input_amount is None, use sol_spent but adjust for fees/rent
            if let (Some(tokens_val), Some(sol_spent_val)) = (token_amount, sol_spent) {
                if tokens_val > 0 && sol_spent_val > 0 {
                    // Estimate the actual trade amount by subtracting typical fees and ATA rent
                    // This is less accurate but provides a reasonable estimate
                    let estimated_fees = 5000u64; // Typical transaction fee
                    let estimated_ata_rent = 2_039_280u64; // Typical ATA rent
                    let adjusted_sol_spent = sol_spent_val.saturating_sub(estimated_fees).saturating_sub(estimated_ata_rent);
                    
                    if adjusted_sol_spent > 0 {
                        let sol_spent_actual = lamports_to_sol(adjusted_sol_spent);
                        let tokens_received_actual = (tokens_val as f64) / (10_f64).powi(output_decimals as i32);
                        
                        if tokens_received_actual > 0.0 {
                            let price = sol_spent_actual / tokens_received_actual;
                            
                            if is_debug_swap_enabled() {
                                log(
                                    LogTag::Swap,
                                    "PRICE_CALC_BUY_FALLBACK",
                                    &format!(
                                        "📊 BUY Price Calculation (sol_spent fallback method):
  SOL spent (total): {} lamports = {:.9} SOL
  Estimated fees: {} lamports
  Estimated ATA rent: {} lamports
  Adjusted SOL for trade: {} lamports = {:.9} SOL
  Tokens received: {} raw = {:.6} tokens
  Effective price: {:.10} SOL per token",
                                        sol_spent_val,
                                        lamports_to_sol(sol_spent_val),
                                        estimated_fees,
                                        estimated_ata_rent,
                                        adjusted_sol_spent,
                                        sol_spent_actual,
                                        tokens_val,
                                        tokens_received_actual,
                                        price
                                    )
                                );
                            }
                            
                            return Some(price);
                        }
                    }
                }
            }
        }
    "sell" => {
        // Sell: calculate SOL per token using only SOL received (exclude ATA rent)
        // Fallback to quote amounts if instruction analysis failed
            let token_amount = input_amount.or(quote_input_amount);
            let sol_amount = sol_received.or(quote_output_amount);
            
            if let (Some(tokens_val), Some(sol_val)) = (token_amount, sol_amount) {
                if tokens_val > 0 && sol_val > 0 {
            // Use only SOL received from the swap
            let total_sol_received = lamports_to_sol(sol_val);
                    let tokens_sold_actual = (tokens_val as f64) / (10_f64).powi(input_decimals as i32);
                    
                    if tokens_sold_actual > 0.0 {
                        let price = total_sol_received / tokens_sold_actual;
                        
                        if is_debug_swap_enabled() {
                            log(
                                LogTag::Swap,
                                "PRICE_CALC_SELL",
                                &format!(
                                    "📊 SELL Price Calculation:
  Tokens sold: {} raw = {:.6} tokens
  SOL received: {} lamports = {:.9} SOL
  Total SOL: {:.9} SOL (swap only)
  Effective price: {:.10} SOL per token",
                                    tokens_val,
                                    tokens_sold_actual,
                                    sol_val,
                                    lamports_to_sol(sol_val),
                                    total_sol_received,
                                    price
                                )
                            );
                        }
                        
                        return Some(price);
                    }
                }
            }
        }
        _ => {
            if is_debug_swap_enabled() {
                log(LogTag::Swap, "PRICE_ERROR", &format!("Invalid direction: {}", expected_direction));
            }
        }
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "PRICE_CALC_FAILED",
            &format!("❌ Price calculation failed for direction: {}", expected_direction)
        );
    }
    
    None
}


/// Validates if the current price is near the expected price within tolerance
pub fn validate_price_near_expected(
    current_price: f64,
    expected_price: f64,
    tolerance_percent: f64
) -> bool {
    let price_difference = (((current_price - expected_price) / expected_price) * 100.0).abs();
    price_difference <= tolerance_percent
}

/// Calculates the effective price per token from a successful buy swap result
/// Returns the price in SOL per token based on actual input/output amounts
pub fn calculate_effective_price_buy(swap_result: &SwapResult) -> Result<f64, SwapError> {
    if !swap_result.success {
        return Err(SwapError::InvalidAmount("Cannot calculate price from failed swap".to_string()));
    }

    // Parse input amount (SOL in lamports)
    let input_lamports: u64 = swap_result.input_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid input amount in swap result".to_string()))?;

    // Parse output amount (tokens in smallest unit)
    let output_tokens_raw: u64 = swap_result.output_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid output amount in swap result".to_string()))?;

    if output_tokens_raw == 0 {
        return Err(
            SwapError::InvalidAmount("Cannot calculate price with zero token output".to_string())
        );
    }

    // Convert lamports to SOL
    let input_sol = lamports_to_sol(input_lamports);

    // Get the actual token decimals from swap data if available
    let token_decimals = if let Some(swap_data) = &swap_result.swap_data {
        swap_data.quote.out_decimals as u32
    } else {
        log(LogTag::Swap, "ERROR", "Cannot calculate effective price without swap data decimals");
        return Err(SwapError::InvalidResponse("Missing decimals in swap data".to_string()));
    };

    // Convert raw token amount to actual tokens using correct decimals
    let output_tokens = (output_tokens_raw as f64) / (10_f64).powi(token_decimals as i32);

    // Calculate effective price: SOL spent / tokens received
    let effective_price = input_sol / output_tokens;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "DEBUG",
            &format!(
                "💰 EFFECTIVE PRICE CALCULATION (BUY):\n  📥 Input: {} SOL ({} lamports)\n  📤 Output: {:.6} tokens ({} raw)\n  🔢 Token Decimals: {}\n  💎 Effective Price: {:.10} SOL per token",
                input_sol,
                input_lamports,
                output_tokens,
                output_tokens_raw,
                token_decimals,
                effective_price
            )
        );
    }

    Ok(effective_price)
}

/// Calculates the effective price per token from a successful sell swap result
/// Returns the price in SOL per token based on actual input/output amounts
pub fn calculate_effective_price_sell(swap_result: &SwapResult) -> Result<f64, SwapError> {
    if !swap_result.success {
        return Err(SwapError::InvalidAmount("Cannot calculate price from failed swap".to_string()));
    }

    // Parse input amount (tokens in smallest unit)
    let input_tokens_raw: u64 = swap_result.input_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid input amount in swap result".to_string()))?;

    // Parse output amount (SOL in lamports)
    let output_lamports: u64 = swap_result.output_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid output amount in swap result".to_string()))?;

    if input_tokens_raw == 0 {
        return Err(
            SwapError::InvalidAmount("Cannot calculate price with zero token input".to_string())
        );
    }

    // Convert lamports to SOL
    let output_sol = lamports_to_sol(output_lamports);

    // Get the actual token decimals from swap data if available
    let token_decimals = if let Some(swap_data) = &swap_result.swap_data {
        swap_data.quote.in_decimals as u32
    } else {
        log(LogTag::Swap, "ERROR", "Cannot calculate effective price without swap data decimals");
        return Err(SwapError::InvalidResponse("Missing decimals in swap data".to_string()));
    };

    // Convert raw token amount to actual tokens using correct decimals
    let input_tokens = (input_tokens_raw as f64) / (10_f64).powi(token_decimals as i32);

    // Calculate effective price: SOL received / tokens sold
    let effective_price = output_sol / input_tokens;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "DEBUG",
            &format!(
                "💰 EFFECTIVE PRICE CALCULATION (SELL):\n  📥 Input: {:.6} tokens ({} raw)\n  📤 Output: {} SOL ({} lamports)\n  🔢 Token Decimals: {}\n  💎 Effective Price: {:.10} SOL per token",
                input_tokens,
                input_tokens_raw,
                output_sol,
                output_lamports,
                token_decimals,
                effective_price
            )
        );
    }
    Ok(effective_price)
}

/// Validates the price from a swap quote against expected price
pub fn validate_quote_price(
    swap_data: &SwapData,
    input_amount: u64,
    expected_price: f64,
    is_sol_to_token: bool
) -> Result<(), SwapError> {
    let output_amount_str = &swap_data.quote.out_amount;
    log(
        LogTag::Swap,
        "DEBUG",
        &format!("Quote validation - Raw out_amount string: '{}'", output_amount_str)
    );

    let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
        log(
            LogTag::Swap,
            "ERROR",
            &format!("Quote validation - Failed to parse out_amount '{}': {}", output_amount_str, e)
        );
        0.0
    });

    log(
        LogTag::Swap,
        "DEBUG",
        &format!("Quote validation - Parsed output_amount_raw: {}", output_amount_raw)
    );

    // Use actual token decimals from quote response
    let token_decimals = swap_data.quote.out_decimals as u32;
    let output_tokens = output_amount_raw / (10_f64).powi(token_decimals as i32);

    let actual_price_per_token = if is_sol_to_token {
        // For SOL to token: price = SOL spent / tokens received
        let input_sol = lamports_to_sol(input_amount);
        if output_tokens > 0.0 {
            input_sol / output_tokens
        } else {
            0.0
        }
    } else {
        // For token to SOL: price = SOL received / tokens spent
        let input_token_decimals = swap_data.quote.in_decimals as u32;
        let input_tokens = (input_amount as f64) / (10_f64).powi(input_token_decimals as i32);
        let output_sol = lamports_to_sol(output_amount_raw as u64);
        if input_tokens > 0.0 {
            output_sol / input_tokens
        } else {
            0.0
        }
    };

    log(
        LogTag::Swap,
        "DEBUG",
        &format!(
            "Quote validation - Price calc debug: input_amount={}, output_amount_raw={}, output_decimals={}, actual_price={:.12}",
            input_amount,
            output_amount_raw,
            token_decimals,
            actual_price_per_token
        )
    );

    let price_difference = (
        ((actual_price_per_token - expected_price) / expected_price) *
        100.0
    ).abs();

    log(
        LogTag::Swap,
        "PRICE",
        &format!(
            "Quote validation - Expected {:.12} SOL/token, Actual {:.12} SOL/token, Diff: {:.2}%",
            expected_price,
            actual_price_per_token,
            price_difference
        )
    );

    if price_difference > INTERNAL_SLIPPAGE_PERCENT {
        return Err(
            SwapError::SlippageExceeded(
                format!(
                    "Price difference {:.2}% exceeds tolerance {:.2}%",
                    price_difference,
                    INTERNAL_SLIPPAGE_PERCENT
                )
            )
        );
    }

    Ok(())
}

/// Validates quote predictions against actual transaction results
/// NEW: Detects when quotes don't match reality for debugging
pub fn validate_quote_vs_actual(
    quote_input: u64,
    quote_output: u64,
    actual_input: Option<u64>,
    actual_output: Option<u64>,
    direction: &str,
    token_name: &str,
) -> (bool, String) {
    let mut issues = Vec::new();
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "QUOTE_VALIDATION",
            &format!(
                "🔍 Quote vs Actual Validation for {}:
  Direction: {}
  Quote - Input: {}, Output: {}
  Actual - Input: {:?}, Output: {:?}",
                token_name,
                direction,
                quote_input,
                quote_output,
                actual_input,
                actual_output
            )
        );
    }
    
    // Check input amount accuracy
    if let Some(actual_in) = actual_input {
        let input_diff_percent = if quote_input > 0 {
            ((actual_in as f64 - quote_input as f64) / quote_input as f64 * 100.0).abs()
        } else {
            0.0
        };
        
        if input_diff_percent > 5.0 { // 5% tolerance
            issues.push(format!(
                "Input mismatch: quoted {} vs actual {} ({:.1}% difference)",
                quote_input, actual_in, input_diff_percent
            ));
        }
    }
    
    // Check output amount accuracy  
    if let Some(actual_out) = actual_output {
        let output_diff_percent = if quote_output > 0 {
            ((actual_out as f64 - quote_output as f64) / quote_output as f64 * 100.0).abs()
        } else {
            0.0
        };
        
        if output_diff_percent > 10.0 { // 10% tolerance for output
            issues.push(format!(
                "Output mismatch: quoted {} vs actual {} ({:.1}% difference)",
                quote_output, actual_out, output_diff_percent
            ));
        }
        
        // Critical check: massive deviation (>50% indicates fundamental error)
        if output_diff_percent > 50.0 {
            issues.push(format!(
                "⚠️ CRITICAL: Output deviation {}% indicates quote/execution mismatch", 
                output_diff_percent
            ));
        }
    }
    
    let is_valid = issues.is_empty();
    let summary = if is_valid {
        "✅ Quote predictions match actual results".to_string()
    } else {
        format!("❌ Quote validation failed: {}", issues.join("; "))
    };
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            if is_valid { "QUOTE_VALID" } else { "QUOTE_INVALID" },
            &summary
        );
    }
    
    (is_valid, summary)
}

/// Gets current token price by requesting a small quote
pub async fn get_token_price_sol(token_mint: &str) -> Result<f64, SwapError> {
    let wallet_address = crate::swaps::transaction::get_wallet_address()?;
    let small_amount = 0.001; // 0.001 SOL

    // Get best quote using the unified swap system
    let quote = crate::swaps::get_best_quote(
        SOL_MINT,
        token_mint,
        sol_to_lamports(small_amount),
        &wallet_address,
        1.0, // 1% slippage for price checking
        "ExactIn", // swap_mode
        0.0, // No fee for quote
        false, // No anti-MEV for price checking
    ).await?;

    let output_tokens = quote.output_amount as f64;
    let price_per_token = (small_amount * 1_000_000_000.0) / output_tokens; // Price in lamports per token

    Ok(price_per_token / 1_000_000_000.0) // Convert back to SOL
}

pub async fn calculate_effective_price_with_verification(
    swap_result: &SwapResult,
    input_mint: &str,
    output_mint: &str,
    direction: &str, // "buy" or "sell"
    ata_rent_reclaimed: Option<u64>, // ATA rent from closure (lamports)
) -> Result<f64, SwapError> {
    if !swap_result.success {
        return Err(SwapError::InvalidAmount("Cannot calculate price from failed swap".to_string()));
    }

    let tx_sig = swap_result.transaction_signature.as_ref()
        .ok_or_else(|| SwapError::TransactionError("No transaction signature available for verification".to_string()))?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "PRICE_VERIFICATION",
            &format!("🔍 Calculating effective price using instruction analysis for {} tx: {}", direction, &tx_sig[..8])
        );
    }

    // Verify on-chain
    let verification_result = super::transaction::verify_swap_transaction(
        tx_sig,
        input_mint,
        output_mint,
        direction
    ).await?;

    if !verification_result.success {
        let err_msg = verification_result.error.unwrap_or_else(|| "verification failed".to_string());
        return Err(SwapError::TransactionError(format!("Transaction verification failed: {}", err_msg)));
    }

    let input_raw_opt = verification_result.input_amount.map(|v| v as u128);
    let input_decimals = verification_result.input_decimals;
    let sol_spent_opt = verification_result.sol_spent;
    let output_raw_opt = verification_result.output_amount.map(|v| v as u128);
    let output_decimals = verification_result.output_decimals;
    let sol_received_opt = verification_result.sol_received;
    let transaction_fee = verification_result.transaction_fee;
    let ata_rent_paid = verification_result.ata_rent_paid;
    let ata_rent_reclaimed = verification_result.ata_rent_reclaimed;

    // Validate presence depending on direction
    match direction {
        "buy" => {
            let have_sol_spent = sol_spent_opt.is_some() || (input_raw_opt.is_some() && input_mint == SOL_MINT);
            let have_tokens_out = output_raw_opt.is_some() || (output_mint == SOL_MINT && sol_received_opt.is_some());
            if !have_sol_spent || !have_tokens_out {
                return Err(SwapError::TransactionError("Missing required amounts in verification result for buy".to_string()));
            }
        }
        "sell" => {
            let have_tokens_in = input_raw_opt.is_some() || (input_mint == SOL_MINT && sol_spent_opt.is_some());
            let have_sol_received = sol_received_opt.is_some() || (output_raw_opt.is_some() && output_mint == SOL_MINT);
            if !have_tokens_in || !have_sol_received {
                return Err(SwapError::TransactionError("Missing required amounts in verification result for sell".to_string()));
            }
        }
        _ => return Err(SwapError::InvalidAmount(format!("Invalid direction: {}", direction))),
    }

    let lamports_to_sol = |lam: u64| -> f64 { lam as f64 / 1_000_000_000.0 };

    // Normalize using instruction values; for BUY prefer quoted SOL input (exclude fees), for SELL use token amount
    let input_normalized: f64 = match direction {
        "buy" => {
            if input_mint == SOL_MINT {
                if let Some(raw) = input_raw_opt { // quoted SOL input in lamports
                    lamports_to_sol(raw as u64)
                } else if let Some(spent) = sol_spent_opt { // fallback if needed
                    lamports_to_sol(spent)
                } else {
                    return Err(SwapError::TransactionError("Unable to determine SOL input for buy".to_string()));
                }
            } else if let Some(raw) = input_raw_opt {
                (raw as f64) / (10f64).powi(input_decimals as i32)
            } else {
                return Err(SwapError::TransactionError("Unable to determine input normalized amount".to_string()));
            }
        }
        "sell" => {
            if let Some(raw) = input_raw_opt {
                (raw as f64) / (10f64).powi(input_decimals as i32)
            } else {
                return Err(SwapError::TransactionError("Unable to determine tokens sold".to_string()));
            }
        }
        _ => unreachable!(),
    };

    let output_normalized: f64 = match direction {
        "buy" => {
            if let Some(raw) = output_raw_opt {
                (raw as f64) / (10f64).powi(output_decimals as i32)
            } else {
                return Err(SwapError::TransactionError("Unable to determine tokens received".to_string()));
            }
        }
        "sell" => {
            if let Some(recv) = sol_received_opt {
                lamports_to_sol(recv)
            } else {
                return Err(SwapError::TransactionError("Unable to determine SOL received".to_string()));
            }
        }
        _ => unreachable!(),
    };

    // Compute effective price
    let effective_price = match direction {
        "buy" => {
            if output_normalized == 0.0 {
                return Err(SwapError::InvalidAmount("Zero tokens received".to_string()));
            }
            input_normalized / output_normalized
        }
        "sell" => {
            if input_normalized == 0.0 {
                return Err(SwapError::InvalidAmount("Zero tokens sold".to_string()));
            }
            output_normalized / input_normalized
        }
        _ => unreachable!(),
    };

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "PRICE_VERIFICATION",
            &format!("✅ Verified price: dir={} input={} output={} price={:.12} SOL/token",
                direction, input_normalized, output_normalized, effective_price)
        );
    }

    Ok(effective_price)
}

