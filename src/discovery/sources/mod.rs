pub mod dexscreener;
pub mod rugcheck;

use crate::types::TokenInfo;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait SourceTrait: Send + Sync {
    fn name(&self) -> &str;
    async fn discover(&self) -> Result<Vec<TokenInfo>>;
}
