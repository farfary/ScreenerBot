use screenerbot::{
    rpc::get_rpc_client,
    tokens::holders::get_count_holders,
    logger::{ log, LogTag },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Testing RPC limits with different tokens");

    // Test with tokens from our database that should have reasonable holder counts
    let test_tokens = vec![
        "9ect83UJcBaenBhw2fiWumMK1y9qznPkSmZ5rpmE1VNQ", // From our previous test - 213 holders
        "6ZJJns7Wtg7way62XvftuYkDdR8uRZUpRaDidNWuh3Y3" // From our previous test - 111 holders
    ];

    for (i, token) in test_tokens.iter().enumerate() {
        log(
            LogTag::System,
            "TEST",
            &format!("Testing token {} ({}/{}): {}", i + 1, i + 1, test_tokens.len(), &token[..8])
        );

        match get_count_holders(token).await {
            Ok(count) => {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("Token {} has {} holders", &token[..8], count)
                );
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to get holder count for {}: {}", &token[..8], e)
                );
            }
        }

        log(LogTag::System, "INFO", "---");
    }

    // Test RPC client directly with limits
    let rpc_client = get_rpc_client();

    log(LogTag::System, "INFO", "Testing RPC client with different limits");

    let filters =
        serde_json::json!([
        {
            "dataSize": 165
        },
        {
            "memcmp": {
                "offset": 0,
                "bytes": test_tokens[0]
            }
        }
    ]);

    // Test with different limits
    let limits = vec![10, 50, 100, 500];

    for limit in limits {
        log(LogTag::System, "TEST", &format!("Testing RPC call with limit: {}", limit));

        match
            rpc_client.get_program_accounts(
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                Some(filters.clone()),
                Some("jsonParsed"),
                Some(10)
            ).await
        {
            Ok(accounts) => {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("Fetched {} accounts with limit {}", accounts.len(), limit)
                );
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Failed with limit {}: {}", limit, e));
            }
        }
    }

    log(LogTag::System, "INFO", "RPC limits test completed");
    Ok(())
}
