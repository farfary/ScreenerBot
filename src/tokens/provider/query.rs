// Provider query: Query methods for accessing stored data

use crate::tokens::api::dexscreener_types::DexScreenerPool;
use crate::tokens::api::geckoterminal_types::GeckoTerminalPool;
use crate::tokens::api::rugcheck_types::RugcheckInfo;
use crate::tokens::storage::{
    get_dexscreener_pools, get_geckoterminal_pools, get_rugcheck_info, get_token_metadata, Database,
};
use crate::tokens::types::TokenMetadata;
use log::debug;
use std::sync::Arc;

/// Query interface for accessing stored token data
pub struct Query {
    pub(crate) database: Arc<Database>,
}

impl Query {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Get token metadata from database
    pub fn get_token_metadata(&self, mint: &str) -> Result<Option<TokenMetadata>, String> {
        debug!("[TOKENS] Querying token metadata: mint={}", mint);
        get_token_metadata(&self.database, mint)
    }

    /// Get DexScreener pools from database
    pub fn get_dexscreener_pools(&self, mint: &str) -> Result<Vec<DexScreenerPool>, String> {
        debug!("[TOKENS] Querying DexScreener pools: mint={}", mint);
        get_dexscreener_pools(&self.database, mint)
    }

    /// Get GeckoTerminal pools from database
    pub fn get_geckoterminal_pools(&self, mint: &str) -> Result<Vec<GeckoTerminalPool>, String> {
        debug!("[TOKENS] Querying GeckoTerminal pools: mint={}", mint);
        get_geckoterminal_pools(&self.database, mint)
    }

    /// Get Rugcheck info from database
    pub fn get_rugcheck_info(&self, mint: &str) -> Result<Option<RugcheckInfo>, String> {
        debug!("[TOKENS] Querying Rugcheck info: mint={}", mint);
        get_rugcheck_info(&self.database, mint)
    }

    /// Check if token exists in database
    pub fn token_exists(&self, mint: &str) -> bool {
        self.get_token_metadata(mint).ok().flatten().is_some()
    }

    /// Get all token mints in database
    pub fn get_all_mints(&self) -> Result<Vec<String>, String> {
        let conn = self.database.get_connection();
        let conn = conn
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT mint FROM tokens ORDER BY updated_at DESC")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let mints = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to query mints: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(mints)
    }
}
