use screenerbot::logger::{ init_file_logging, log, LogTag };
use screenerbot::tokens::discovery::{
    fetch_jupiter_recent_tokens,
    fetch_jupiter_top_organic_score,
    fetch_jupiter_top_traded,
    fetch_jupiter_top_trending,
    fetch_coingecko_solana_markets,
    fetch_defillama_protocols,
    fetch_defillama_token_price,
};

#[tokio::main]
async fn main() {
    // Initialize logging
    init_file_logging();

    log(LogTag::Test, "INFO", "Testing new API integrations");

    // Test Jupiter Recent Tokens API
    log(LogTag::Test, "INFO", "Testing Jupiter Recent Tokens API...");
    match fetch_jupiter_recent_tokens().await {
        Ok(tokens) => {
            log(LogTag::Test, "SUCCESS", &format!("Jupiter API: {} tokens found", tokens.len()));
            if !tokens.is_empty() {
                log(LogTag::Test, "INFO", &format!("Sample token: {}", tokens[0]));
            }
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("Jupiter API failed: {}", e));
        }
    }

    // Test Jupiter Top Organic Score API
    log(LogTag::Test, "INFO", "Testing Jupiter Top Organic Score API...");
    match fetch_jupiter_top_organic_score().await {
        Ok(tokens) => {
            log(
                LogTag::Test,
                "SUCCESS",
                &format!("Jupiter Top Organic Score API: {} tokens found", tokens.len())
            );
            if !tokens.is_empty() {
                log(LogTag::Test, "INFO", &format!("Sample token: {}", tokens[0]));
            }
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("Jupiter Top Organic Score API failed: {}", e));
        }
    }

    // Test Jupiter Top Traded API
    log(LogTag::Test, "INFO", "Testing Jupiter Top Traded API...");
    match fetch_jupiter_top_traded().await {
        Ok(tokens) => {
            log(
                LogTag::Test,
                "SUCCESS",
                &format!("Jupiter Top Traded API: {} tokens found", tokens.len())
            );
            if !tokens.is_empty() {
                log(LogTag::Test, "INFO", &format!("Sample token: {}", tokens[0]));
            }
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("Jupiter Top Traded API failed: {}", e));
        }
    }

    // Test Jupiter Top Trending API
    log(LogTag::Test, "INFO", "Testing Jupiter Top Trending API...");
    match fetch_jupiter_top_trending().await {
        Ok(tokens) => {
            log(
                LogTag::Test,
                "SUCCESS",
                &format!("Jupiter Top Trending API: {} tokens found", tokens.len())
            );
            if !tokens.is_empty() {
                log(LogTag::Test, "INFO", &format!("Sample token: {}", tokens[0]));
            }
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("Jupiter Top Trending API failed: {}", e));
        }
    }

    // Test CoinGecko Solana Markets API
    log(LogTag::Test, "INFO", "Testing CoinGecko Solana Markets API...");
    match fetch_coingecko_solana_markets().await {
        Ok(tokens) => {
            log(
                LogTag::Test,
                "SUCCESS",
                &format!("CoinGecko Markets API: {} tokens found", tokens.len())
            );
            if !tokens.is_empty() {
                log(LogTag::Test, "INFO", &format!("Sample token: {}", tokens[0]));
            }
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("CoinGecko Markets API failed: {}", e));
        }
    }

    // Test DeFiLlama Protocols API
    log(LogTag::Test, "INFO", "Testing DeFiLlama Protocols API...");
    match fetch_defillama_protocols().await {
        Ok(tokens) => {
            log(
                LogTag::Test,
                "SUCCESS",
                &format!("DeFiLlama Protocols API: {} tokens found", tokens.len())
            );
            if !tokens.is_empty() {
                log(LogTag::Test, "INFO", &format!("Sample token: {}", tokens[0]));
            }
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("DeFiLlama Protocols API failed: {}", e));
        }
    }

    // Test DeFiLlama Pricing API with SOL token
    log(LogTag::Test, "INFO", "Testing DeFiLlama Pricing API...");
    let sol_mint = "So11111111111111111111111111111111111111112";
    match fetch_defillama_token_price(sol_mint).await {
        Ok(price) => {
            log(
                LogTag::Test,
                "SUCCESS",
                &format!("DeFiLlama Pricing API: SOL price = ${:.2}", price)
            );
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("DeFiLlama Pricing API failed: {}", e));
        }
    }

    log(LogTag::Test, "INFO", "API testing completed");
}
