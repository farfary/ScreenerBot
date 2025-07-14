use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    println!("🚀 Starting ScreenerBot - Simulation Mode");
    println!("📡 Testing DexScreener API integration...");

    // Test DexScreener API call
    let client = reqwest::Client::new();
    let url = "https://api.dexscreener.com/token-profiles/latest/v1";

    match client.get(url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.text().await {
                    Ok(body) => {
                        println!("✅ DexScreener API call successful!");
                        println!("📊 Response preview: {} characters", body.len());

                        // Try to parse as JSON to validate structure
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(json) => {
                                println!("🔍 Examining API response structure...");

                                // Debug: Print the top-level keys
                                if let Some(obj) = json.as_object() {
                                    println!(
                                        "📋 Top-level keys: {:?}",
                                        obj.keys().collect::<Vec<_>>()
                                    );
                                }

                                // Try different possible response structures
                                let token_array = if let Some(data) = json.get("data") {
                                    data.as_array()
                                } else if json.is_array() {
                                    json.as_array()
                                } else {
                                    println!(
                                        "📄 Full response preview: {}",
                                        serde_json
                                            ::to_string_pretty(&json)
                                            .unwrap_or_else(|_|
                                                "Unable to pretty print".to_string()
                                            )
                                            [..(500).min(json.to_string().len())].to_string()
                                    );
                                    None
                                };

                                if let Some(array) = token_array {
                                    println!("� Found {} token profiles", array.len());

                                    // Show first few tokens as examples
                                    println!("\n🪙 Sample tokens discovered:");
                                    for (i, token) in array.iter().take(3).enumerate() {
                                        println!(
                                            "Token {}: {}",
                                            i + 1,
                                            serde_json
                                                ::to_string_pretty(token)
                                                .unwrap_or_else(|_|
                                                    "Unable to serialize".to_string()
                                                )
                                        );
                                        println!("---");
                                    }
                                } else {
                                    println!("⚠️  Unable to find token array in response");
                                }

                                println!("✅ JSON parsing successful - API is working correctly");
                                println!("🎯 Simulation ready for DexScreener token discovery");
                            }
                            Err(e) => {
                                println!("⚠️  JSON parsing failed: {}", e);
                                println!("Raw response: {}", &body[..body.len().min(500)]);
                            }
                        }
                    }
                    Err(e) => println!("❌ Failed to read response body: {}", e),
                }
            } else {
                println!("❌ API request failed with status: {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to make API request: {}", e);
        }
    }

    println!("\n🏁 ScreenerBot simulation test completed!");
    Ok(())
}
