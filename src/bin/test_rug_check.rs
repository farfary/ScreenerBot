use screenerbot::prelude::*;
use screenerbot::dexscreener::get_rug_check_report;

#[tokio::main]
async fn main() {
    println!("🔍 Testing RugCheck API integration...\n");

    // Test with the example token from the request
    let test_mint = "4c7GJc2wrJtvjV64Q7c7QAT7zy456xFsFucovgB1pump";

    match get_rug_check_report(test_mint, true).await {
        Some(rug_data) => {
            println!("📊 Rug Check Report for {}:", test_mint);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("🔢 Score: {} (normalized: {})", rug_data.score, rug_data.score_normalised);
            println!("🚨 Rugged: {}", if rug_data.rugged { "YES ❌" } else { "NO ✅" });
            println!("👥 Total Holders: {}", rug_data.total_holders);
            println!("💰 Total Market Liquidity: ${:.2}", rug_data.total_market_liquidity);
            println!("🏦 Total Supply: {}", rug_data.total_supply);
            println!("👤 Creator Balance: {}", rug_data.creator_balance);
            println!("💸 Transfer Fee: {}%", rug_data.transfer_fee_pct);

            if let Some(mint_auth) = &rug_data.mint_authority {
                println!("⚠️ Mint Authority: {}", mint_auth);
            } else {
                println!("✅ No Mint Authority");
            }

            if let Some(freeze_auth) = &rug_data.freeze_authority {
                println!("⚠️ Freeze Authority: {}", freeze_auth);
            } else {
                println!("✅ No Freeze Authority");
            }

            if !rug_data.risks.is_empty() {
                println!("\n⚠️ Identified Risks:");
                for risk in &rug_data.risks {
                    let level_emoji = match risk.level.as_str() {
                        "danger" => "🚨",
                        "warn" => "⚠️",
                        "info" => "ℹ️",
                        _ => "❓",
                    };
                    println!("  {} {} (Score: {})", level_emoji, risk.name, risk.score);
                    println!("     {}", risk.description);
                }
            }

            // Test the safety check
            let dummy_token = Token {
                mint: test_mint.to_string(),
                symbol: "TEST".to_string(),
                name: "Test Token".to_string(),
                balance: "0".to_string(),
                ata_pubkey: "".to_string(),
                program_id: "".to_string(),
                dex_id: "".to_string(),
                url: "".to_string(),
                pair_address: "".to_string(),
                labels: Vec::new(),
                quote_address: "".to_string(),
                quote_name: "".to_string(),
                quote_symbol: "".to_string(),
                price_native: "0".to_string(),
                price_usd: "0".to_string(),
                last_price_usd: "0".to_string(),
                volume_usd: "0".to_string(),
                fdv_usd: "0".to_string(),
                image_url: "".to_string(),
                txns: Txns {
                    m5: TxnCount { buys: 0, sells: 0 },
                    h1: TxnCount { buys: 0, sells: 0 },
                    h6: TxnCount { buys: 0, sells: 0 },
                    h24: TxnCount { buys: 0, sells: 0 },
                },
                volume: Volume { m5: 0.0, h1: 0.0, h6: 0.0, h24: 0.0 },
                price_change: PriceChange { m5: 0.0, h1: 0.0, h6: 0.0, h24: 0.0 },
                liquidity: Liquidity { usd: 0.0, base: 0.0, quote: 0.0 },
                pair_created_at: 0,
                rug_check: rug_data,
            };

            println!("\n🛡️ Safety Assessment:");
            let is_safe = screenerbot::dexscreener::is_safe_to_trade(&dummy_token, true);
            println!("Trading Safety: {}", if is_safe { "SAFE ✅" } else { "UNSAFE ❌" });
        }
        None => {
            println!("❌ Failed to fetch rug check data for {}", test_mint);
        }
    }

    println!("\n✅ RugCheck integration test completed!");
}
