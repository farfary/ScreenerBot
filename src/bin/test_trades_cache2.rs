use screenerbot::trades::*;
use anyhow::Result;
use reqwest;
use serde::Deserialize;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing Trades Cache System");

    // Test fetch trades functionality
    let test_pool_address = "5fMrnHPsUzwC5ptBkGtxsxu3D5JaVMLEw5VhppK5aMwv";
    println!("Testing fetch_token_trades for pool: {}", test_pool_address);

    match fetch_token_trades(test_pool_address).await {
        Ok(response_text) => {
            match serde_json::from_str::<GeckoTradesResponse>(&response_text) {
                Ok(gecko_response) => {
                    println!("Successfully fetched trades:");
                    println!("  - Total trades: {}", gecko_response.data.len());

                    // Show first few trades
                    for (i, trade_data) in gecko_response.data.iter().take(3).enumerate() {
                        if let Ok(trade) = convert_gecko_trade_to_trade(trade_data.clone()) {
                            println!(
                                "  - Trade {}: {} {} USD, kind: {}",
                                i + 1,
                                trade.volume_usd,
                                trade.price_usd,
                                trade.kind
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to parse JSON response: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Failed to fetch trades: {}", e);
        }
    }

    // Test cache directory creation
    println!("\nTesting cache directory...");
    if let Err(e) = tokio::fs::create_dir_all(".cache_trades").await {
        println!("Failed to create cache directory: {}", e);
    } else {
        println!("Cache directory created/verified");
    }

    // Test cache functions
    println!("\nTesting cache functions...");
    println!("   - Cache directory exists: {}", std::path::Path::new(".cache_trades").exists());

    println!("\nTrades cache system test completed!");
    Ok(())
}

async fn fetch_token_trades(
    pool_address: &str
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let url =
        format!("https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/trades?limit=100", pool_address);

    let client = reqwest::Client::new();
    let response = client.get(&url).header("accept", "application/json").send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow("API request failed: {}", response.status()).into());
    }

    let response_text = response.text().await?;

    // Debug: Print first 1000 characters of response to understand format
    println!("API Response (first 1000 chars):");
    println!("{}", &response_text[..std::cmp::min(1000, response_text.len())]);

    Ok(response_text)
}

#[derive(Debug, Deserialize)]
struct GeckoTradesResponse {
    data: Vec<TradeData>,
}

#[derive(Debug, Deserialize, Clone)]
struct TradeData {
    id: String,
    #[serde(rename = "type")]
    trade_type: String,
    attributes: TradeAttributes,
}

#[derive(Debug, Deserialize, Clone)]
struct TradeAttributes {
    block_number: u64,
    block_timestamp: String,
    tx_hash: String,
    tx_from_address: String,
    from_token_amount: String,
    to_token_amount: String,
    price_from_in_currency_token: String,
    price_to_in_currency_token: String,
    price_from_in_usd: String,
    price_to_in_usd: String,
    volume_in_usd: String,
    kind: String, // "buy" or "sell"
    from_token_address: String,
    to_token_address: String,
}

fn convert_gecko_trade_to_trade(
    trade_data: TradeData
) -> Result<Trade, Box<dyn std::error::Error + Send + Sync>> {
    use chrono::DateTime;
    let attrs = trade_data.attributes;

    // Parse timestamp
    let timestamp = DateTime::parse_from_rfc3339(&attrs.block_timestamp)?.timestamp() as u64;

    Ok(Trade {
        timestamp,
        tx_hash: attrs.tx_hash,
        kind: attrs.kind,
        from_token_amount: attrs.from_token_amount.parse().unwrap_or(0.0),
        to_token_amount: attrs.to_token_amount.parse().unwrap_or(0.0),
        price_usd: attrs.price_from_in_usd.parse().unwrap_or(0.0),
        volume_usd: attrs.volume_in_usd.parse().unwrap_or(0.0),
        from_address: attrs.tx_from_address,
        to_address: attrs.to_token_address,
    })
}
