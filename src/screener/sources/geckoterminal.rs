//! GeckoTerminal API integration for token discovery

use super::TokenSource;
use crate::core::{ BotResult, TokenOpportunity };
use reqwest::Client;
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

/// GeckoTerminal API source
pub struct GeckoTerminalSource {
    client: Client,
    base_url: String,
}

impl GeckoTerminalSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ScreenerBot/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api.geckoterminal.com".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl TokenSource for GeckoTerminalSource {
    fn name(&self) -> &str {
        "GeckoTerminal"
    }

    async fn initialize(&mut self) -> BotResult<()> {
        log::info!("✅ GeckoTerminal source initialized");
        Ok(())
    }

    async fn get_new_tokens(&self) -> BotResult<Vec<TokenOpportunity>> {
        log::info!("GeckoTerminal source not yet implemented");
        Ok(Vec::new())
    }

    async fn get_token_info(&self, _mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        Ok(None)
    }

    async fn health_check(&self) -> BotResult<bool> {
        Ok(true)
    }
}
