use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use serde_json::Value;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct Token {
    pub mint: String,
    pub balance: String,
    pub ata_pubkey: String,
    pub program_id: String,
    pub name: String,
    pub symbol: String,
    pub price_usd: String,
    pub last_price_usd: String,
    pub volume_usd: String,
    pub fdv_usd: String,
    pub image_url: String,
}

// ────────────── GLOBAL STATIC ──────────────
pub static TOKENS: Lazy<RwLock<Vec<Token>>> = Lazy::new(|| RwLock::new(Vec::new()));

// ────────────── CHUNK HELPER ──────────────
fn chunked(vec: Vec<String>, chunk_size: usize) -> Vec<Vec<String>> {
    vec.chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

pub async fn start_dexscreener_loop() {
    let client = Client::new();

    // ────────────── TASK: Insert new tokens every 60 sec ──────────────
    let client_insert = client.clone();
    tokio::spawn(async move {
        loop {
            println!("\n🔄 [Screener] Fetching DexScreener token lists...");

            let mut new_tokens: Vec<Token> = Vec::new();
            let endpoints = vec![
                "https://api.dexscreener.com/token-profiles/latest/v1",
                "https://api.dexscreener.com/token-boosts/latest/v1",
                "https://api.dexscreener.com/token-boosts/top/v1"
            ];

            for url in endpoints {
                println!("🌐 [Screener] Requesting: {}", url);
                if let Ok(resp) = client_insert.get(url).send().await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(arr) = json.as_array() {
                            println!("✅ [Screener] Got {} tokens from {}", arr.len(), url);
                            for item in arr {
                                let mint = item["tokenAddress"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string();
                                let ata_pubkey = item["url"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string();
                                let program_id = item["chainId"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string();

                                new_tokens.push(Token {
                                    mint,
                                    balance: "0".to_string(),
                                    ata_pubkey,
                                    program_id,
                                    name: "".to_string(),
                                    symbol: "".to_string(),
                                    price_usd: "".to_string(),
                                    last_price_usd: "".to_string(),
                                    volume_usd: "".to_string(),
                                    fdv_usd: "".to_string(),
                                    image_url: "".to_string(),
                                });
                            }
                        }
                    }
                }
            }

            println!("🔢 [Screener] Total new tokens fetched: {}", new_tokens.len());

            // ────────────── GeckoTerminal batches ──────────────
            let mints: Vec<String> = new_tokens
                .iter()
                .map(|t| t.mint.clone())
                .collect();
            let batches = chunked(mints, 30);

            for (i, batch) in batches.iter().enumerate() {
                let mints_joined = batch.join(",");
                let url =
                    format!("https://api.geckoterminal.com/api/v2/networks/solana/tokens/multi/{}", mints_joined);
                println!(
                    "🔗 [Screener] Fetching GeckoTerminal info (batch {}/{})",
                    i + 1,
                    batches.len()
                );

                if let Ok(resp) = client_insert.get(&url).send().await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(data) = json["data"].as_array() {
                            println!("✅ [Screener] Got {} tokens from GeckoTerminal", data.len());
                            for token in &mut new_tokens {
                                for item in data {
                                    let attr = &item["attributes"];
                                    let addr = attr["address"].as_str().unwrap_or_default();
                                    if addr == token.mint {
                                        token.name = attr["name"]
                                            .as_str()
                                            .unwrap_or_default()
                                            .to_string();
                                        token.symbol = attr["symbol"]
                                            .as_str()
                                            .unwrap_or_default()
                                            .to_string();
                                        token.price_usd = attr["price_usd"]
                                            .as_str()
                                            .unwrap_or_default()
                                            .to_string();
                                        token.volume_usd = attr["volume_usd"]["h24"]
                                            .as_str()
                                            .unwrap_or_default()
                                            .to_string();
                                        token.fdv_usd = attr["fdv_usd"]
                                            .as_f64()
                                            .unwrap_or_default()
                                            .to_string();
                                        token.image_url = attr["image_url"]
                                            .as_str()
                                            .unwrap_or_default()
                                            .to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            {
                let mut lock = TOKENS.write().await;
                let mut existing: Vec<String> = lock
                    .iter()
                    .map(|t| t.mint.clone())
                    .collect();
                for t in new_tokens {
                    if
                        !existing.contains(&t.mint) &&
                        !t.symbol.is_empty() &&
                        !t.price_usd.is_empty() &&
                        !t.name.is_empty()
                    {
                        println!(
                            "🆕 [Screener] Added: {} ({}) | Price: ${}",
                            t.symbol,
                            t.mint,
                            t.price_usd
                        );
                        lock.push(t.clone());
                        existing.push(t.mint.clone());
                    }
                }
                println!("✅ [Screener] TOKENS updated: {} unique tokens", lock.len());
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    // ────────────── TASK: Update prices every 5 sec ──────────────
    let client_update = client.clone();
    tokio::spawn(async move {
        loop {
            let mints: Vec<String> = {
                let lock = TOKENS.read().await;
                lock.iter()
                    .map(|t| t.mint.clone())
                    .collect()
            };

            let batches = chunked(mints, 30);

            for (i, batch) in batches.iter().enumerate() {
                let joined = batch.join(",");
                let url =
                    format!("https://api.geckoterminal.com/api/v2/networks/solana/tokens/multi/{}", joined);
                println!(
                    "🔗 [Screener] Fetching GeckoTerminal price updates (batch {}/{})",
                    i + 1,
                    batches.len()
                );

                if let Ok(resp) = client_update.get(&url).send().await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(data) = json["data"].as_array() {
                            let mut lock = TOKENS.write().await;
                            for token in &mut *lock {
                                for item in data {
                                    let attr = &item["attributes"];
                                    let addr = attr["address"].as_str().unwrap_or_default();
                                    if addr == token.mint {
                                        let new_price_str = attr["price_usd"]
                                            .as_str()
                                            .unwrap_or_default();
                                        let new_price = new_price_str.parse::<f64>().unwrap_or(0.0);
                                        let last_price = token.price_usd
                                            .parse::<f64>()
                                            .unwrap_or(0.0);

                                        if last_price > 0.0 {
                                            let pct =
                                                ((new_price - last_price) / last_price) * 100.0;
                                            if pct.abs() >= 5.0 {
                                                println!(
                                                    "💲 [Screener] {}: ${:.8} ({:+.2}%)",
                                                    token.symbol,
                                                    new_price,
                                                    pct
                                                );
                                            }
                                        }

                                        token.last_price_usd = token.price_usd.clone();
                                        token.price_usd = new_price_str.to_string();
                                        token.volume_usd = attr["volume_usd"]["h24"]
                                            .as_str()
                                            .unwrap_or_default()
                                            .to_string();
                                        token.fdv_usd = attr["fdv_usd"]
                                            .as_f64()
                                            .unwrap_or_default()
                                            .to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });
}
