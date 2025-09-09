use screenerbot::{ rpc::get_rpc_client, logger::{ log, LogTag } };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Testing RPC limits with SOL (should have many accounts)");

    let rpc_client = get_rpc_client();

    // SOL token mint - this should have millions of accounts
    let sol_mint = "So11111111111111111111111111111111111111112";

    let filters =
        serde_json::json!([
        {
            "dataSize": 165
        },
        {
            "memcmp": {
                "offset": 0,
                "bytes": sol_mint
            }
        }
    ]);

    // Test with very small limits to see if they're respected
    let limits = vec![5, 10, 25, 50];

    for limit in limits {
        log(LogTag::System, "TEST", &format!("Testing SOL with limit: {}", limit));

        match
            rpc_client.get_program_accounts(
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                Some(filters.clone()),
                Some("base64"), // Use base64 to minimize data transfer
                Some(5) // Short timeout
            ).await
        {
            Ok(accounts) => {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!(
                        "SOL: Fetched {} accounts with limit {} (expected: {})",
                        accounts.len(),
                        limit,
                        limit
                    )
                );

                if accounts.len() == limit {
                    log(LogTag::System, "LIMIT_WORKING", "✅ Limit is being respected!");
                } else {
                    log(
                        LogTag::System,
                        "LIMIT_IGNORED",
                        "⚠️  Limit may be ignored or insufficient accounts"
                    );
                }
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed SOL test with limit {}: {}", limit, e)
                );
            }
        }
    }

    log(LogTag::System, "INFO", "SOL limits test completed");
    Ok(())
}
