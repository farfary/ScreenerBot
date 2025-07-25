/// Compatibility layer for existing code that expects different TokenDatabase methods
use crate::tokens::cache::TokenDatabase;
use crate::tokens::types::{ Token, ApiToken };
use std::error::Error;

impl TokenDatabase {
    /// Add or update a single token (compatibility method)
    pub async fn add_or_update_token(
        &self,
        token: &Token,
        discovery_source: &str
    ) -> Result<bool, Box<dyn Error>> {
        // Convert Token to ApiToken for storage
        let api_token: ApiToken = token.clone().into();

        // Check if token exists
        let exists = self.get_tokens_by_mints(&[token.mint.clone()]).await?.len() > 0;

        // Add or update the token
        self.update_tokens(&[api_token]).await?;

        // Log the operation
        crate::logger::log(
            crate::logger::LogTag::System,
            if exists {
                "UPDATE"
            } else {
                "INSERT"
            },
            &format!("Token {} from {}", token.symbol, discovery_source)
        );

        Ok(!exists) // Return true if it was a new token
    }

    /// Get a single token by mint (compatibility method)
    pub async fn get_token(&self, mint: &str) -> Result<Option<Token>, Box<dyn Error>> {
        let tokens = self.get_tokens_by_mints(&[mint.to_string()]).await?;
        if let Some(api_token) = tokens.first() {
            Ok(Some(api_token.clone().into()))
        } else {
            Ok(None)
        }
    }
}
