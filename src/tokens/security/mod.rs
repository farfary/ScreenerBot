use std::sync::Arc;
use once_cell::sync::OnceCell;

use crate::tokens::types::SecurityRisk;

#[derive(Debug, Clone)]
pub struct SecuritySnapshot {
    pub score: Option<i32>,
    pub rugged: bool,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub total_holders: Option<i64>,
    pub top_10_concentration: Option<f64>,
    pub risks: Vec<SecurityRisk>,
}

#[async_trait::async_trait]
pub trait SecurityProvider: Send + Sync {
    async fn get(&self, mint: &str) -> Option<SecuritySnapshot>;
}

#[derive(Debug, Default)]
struct NoopSecurityProvider;

#[async_trait::async_trait]
impl SecurityProvider for NoopSecurityProvider {
    async fn get(&self, _mint: &str) -> Option<SecuritySnapshot> {
        None
    }
}

static GLOBAL: OnceCell<Arc<dyn SecurityProvider>> = OnceCell::new();

pub fn init_security_provider() {
    let _ = GLOBAL.set(Arc::new(NoopSecurityProvider::default()));
}

pub fn get_security_provider() -> Arc<dyn SecurityProvider> {
    GLOBAL
        .get()
        .cloned()
        .unwrap_or_else(|| Arc::new(NoopSecurityProvider::default()))
}
