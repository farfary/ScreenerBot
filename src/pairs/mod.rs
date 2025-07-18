pub mod types;
pub mod client;
pub mod database;
pub mod decoders;
pub mod pool_fetcher;
pub mod analyzer;

pub use types::*;
pub use client::PairsClient;
pub use database::*;
pub use decoders::*;
pub use pool_fetcher::*;
pub use analyzer::*;

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
