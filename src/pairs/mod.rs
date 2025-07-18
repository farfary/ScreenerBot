pub mod types;
pub mod client;
pub mod database;

pub use types::*;
pub use client::PairsClient;
pub use database::*;

use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait PairsTrait {
    async fn get_token_pairs(&self, token_address: &str) -> Result<Vec<TokenPair>>;
    async fn get_token_pairs_by_chain(
        &self,
        chain_id: &str,
        token_address: &str
    ) -> Result<Vec<TokenPair>>;
}
