use reqwest;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

// OHLCV response structures
#[derive(Debug, Deserialize)]
struct OhlcvResponse {
    data: OhlcvData,
}

#[derive(Debug, Deserialize)]
struct OhlcvData {
    id: String,
    #[serde(rename = "type")]
    data_type: String,
    attributes: OhlcvAttributes,
}

#[derive(Debug, Deserialize)]
struct OhlcvAttributes {
    ohlcv_list: Vec<Vec<f64>>, // [timestamp, open, high, low, close, volume]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing GeckoTerminal OHLCV API...");

    let pool_address = "4CHY5cahkqpiV2x35hZUdgw7q1qvgdfQywJ3cPWMbB6U";

    // Test different timeframes
    let timeframes = vec!["day", "hour", "minute"];

    for timeframe in timeframes {
        println!("\n📊 Testing {} timeframe...", timeframe);

        let url = format!(
            "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/{}?limit=10",
            pool_address,
            timeframe
        );

        println!("🔗 URL: {}", url);

        let client = reqwest::Client::new();
        let response = client.get(&url).header("accept", "application/json").send().await?;

        println!("📈 Status: {}", response.status());

        if response.status().is_success() {
            let text = response.text().await?;
            println!("📄 Response length: {} bytes", text.len());

            // Try to parse as JSON
            match serde_json::from_str::<OhlcvResponse>(&text) {
                Ok(ohlcv_data) => {
                    println!("✅ Successfully parsed OHLCV data");
                    println!("📊 Data ID: {}", ohlcv_data.data.id);
                    println!("📊 Data Type: {}", ohlcv_data.data.data_type);
                    println!("📊 OHLCV entries: {}", ohlcv_data.data.attributes.ohlcv_list.len());

                    // Show first few entries
                    for (i, ohlcv) in ohlcv_data.data.attributes.ohlcv_list
                        .iter()
                        .take(3)
                        .enumerate() {
                        if ohlcv.len() >= 6 {
                            println!(
                                "  Entry {}: timestamp={}, open={}, high={}, low={}, close={}, volume={}",
                                i + 1,
                                ohlcv[0],
                                ohlcv[1],
                                ohlcv[2],
                                ohlcv[3],
                                ohlcv[4],
                                ohlcv[5]
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to parse JSON: {}", e);
                    println!("📄 Raw response (first 500 chars):");
                    println!("{}", &text[..text.len().min(500)]);
                }
            }
        } else {
            let error_text = response.text().await?;
            println!("❌ Request failed: {}", error_text);
        }

        // Rate limiting between requests
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Test with different parameters
    println!("\n🔧 Testing with different parameters...");

    // Test with aggregation
    let url_with_agg =
        format!("https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/minute?aggregate=15&limit=5", pool_address);

    println!("🔗 URL with 15m aggregation: {}", url_with_agg);

    let client = reqwest::Client::new();
    let response = client.get(&url_with_agg).header("accept", "application/json").send().await?;

    println!("📈 Status: {}", response.status());

    if response.status().is_success() {
        let text = response.text().await?;
        match serde_json::from_str::<OhlcvResponse>(&text) {
            Ok(ohlcv_data) => {
                println!("✅ 15m aggregation successful");
                println!("📊 OHLCV entries: {}", ohlcv_data.data.attributes.ohlcv_list.len());
            }
            Err(e) => {
                println!("❌ Failed to parse 15m aggregation: {}", e);
            }
        }
    } else {
        let error_text = response.text().await?;
        println!("❌ 15m aggregation failed: {}", error_text);
    }

    // Test with token currency
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let url_with_token =
        format!("https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/hour?currency=token&limit=5", pool_address);

    println!("\n🔗 URL with token currency: {}", url_with_token);

    let response = client.get(&url_with_token).header("accept", "application/json").send().await?;

    println!("📈 Status: {}", response.status());

    if response.status().is_success() {
        let text = response.text().await?;
        match serde_json::from_str::<OhlcvResponse>(&text) {
            Ok(ohlcv_data) => {
                println!("✅ Token currency successful");
                println!("📊 OHLCV entries: {}", ohlcv_data.data.attributes.ohlcv_list.len());

                // Show first entry with token pricing
                if let Some(first_entry) = ohlcv_data.data.attributes.ohlcv_list.first() {
                    if first_entry.len() >= 6 {
                        println!(
                            "  Token prices - open={}, high={}, low={}, close={}, volume={}",
                            first_entry[1],
                            first_entry[2],
                            first_entry[3],
                            first_entry[4],
                            first_entry[5]
                        );
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to parse token currency: {}", e);
            }
        }
    } else {
        let error_text = response.text().await?;
        println!("❌ Token currency failed: {}", error_text);
    }

    println!("\n✅ OHLCV API testing completed!");

    Ok(())
}
