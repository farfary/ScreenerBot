/// Token Holder Counting Module
///
/// This module provides a single function to count token holders directly from Solana RPC
/// using the existing RPC client system for consistent error handling and rate limiting.
use crate::{ errors::ScreenerBotError, logger::{ log, LogTag }, rpc::get_rpc_client };

/// Get holder count for any token
/// This is the primary function that should be used everywhere
pub async fn get_count_holders(mint_address: &str) -> Result<u32, String> {
    log(LogTag::Rpc, "HOLDER_COUNT", &format!("Counting holders for mint {}", &mint_address[..8]));

    let rpc_client = get_rpc_client();

    // Determine token type first
    let is_token_2022 = match rpc_client.is_token_2022_mint(mint_address).await {
        Ok(is_2022) => is_2022,
        Err(e) => {
            log(
                LogTag::Rpc,
                "ERROR",
                &format!("Failed to determine token type for {}: {}", &mint_address[..8], e)
            );
            return Err(format!("Failed to determine token type: {}", e));
        }
    };

    let program_id = if is_token_2022 {
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    } else {
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    };

    // Create filters for getProgramAccounts
    let filters =
        serde_json::json!([
        {
            "dataSize": 165  // Standard token account size
        },
        {
            "memcmp": {
                "offset": 0,
                "bytes": mint_address
            }
        }
    ]);

    log(
        LogTag::Rpc,
        "HOLDER_COUNT",
        &format!(
            "Querying {} holders for mint {} (60s timeout)",
            if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            },
            &mint_address[..8]
        )
    );

    // Use the RPC client's get_program_accounts method
    match
        rpc_client.get_program_accounts(
            program_id,
            Some(filters),
            Some("jsonParsed"),
            Some(60) // 60 second timeout
        ).await
    {
        Ok(accounts) => {
            let total_accounts = accounts.len();

            log(
                LogTag::Rpc,
                "HOLDER_COUNT",
                &format!(
                    "Processing {} total accounts for mint {}",
                    total_accounts,
                    &mint_address[..8]
                )
            );

            // Count accounts with non-zero balance
            let holder_count = accounts
                .iter()
                .filter_map(|account| {
                    if let Some(parsed) = account.get("account")?.get("data")?.get("parsed") {
                        if let Some(info) = parsed.get("info") {
                            if let Some(token_amount) = info.get("tokenAmount") {
                                if let Some(amount) = token_amount.get("amount")?.as_str() {
                                    if amount != "0" {
                                        return Some(());
                                    }
                                }
                            }
                        }
                    }
                    None
                })
                .count();

            log(
                LogTag::Rpc,
                "HOLDER_COUNT",
                &format!(
                    "Found {} {} holders out of {} total accounts for mint {}",
                    holder_count,
                    if is_token_2022 {
                        "Token-2022"
                    } else {
                        "SPL Token"
                    },
                    total_accounts,
                    &mint_address[..8]
                )
            );

            Ok(holder_count as u32)
        }
        Err(e) => {
            let error_msg = match e {
                ScreenerBotError::Network(ref net_err) => {
                    match net_err {
                        crate::errors::NetworkError::ConnectionTimeout { endpoint, timeout_ms } => {
                            format!(
                                "Token has too many holders for single query ({}ms timeout): {}",
                                timeout_ms,
                                endpoint
                            )
                        }
                        _ => format!("Network error: {}", net_err),
                    }
                }
                ScreenerBotError::RpcProvider(ref rpc_err) => {
                    match rpc_err {
                        crate::errors::RpcProviderError::RateLimitExceeded {
                            provider_name,
                            ..
                        } => {
                            format!("RPC rate limited ({}): Try again later", provider_name)
                        }
                        _ => format!("RPC provider error: {}", rpc_err),
                    }
                }
                _ => format!("Error: {}", e),
            };

            log(
                LogTag::Rpc,
                "ERROR",
                &format!("Failed to get holders for mint {}: {}", &mint_address[..8], error_msg)
            );

            Err(error_msg)
        }
    }
}
