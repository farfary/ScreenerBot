pub mod dexscreener;
pub mod rugcheck;

use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait SourceTrait: Send + Sync {
    fn name(&self) -> &str;
    async fn discover_mints(&self) -> Result<Vec<String>>;
}
