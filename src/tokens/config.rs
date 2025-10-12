use crate::config::{with_config, TokensConfig};

/// Execute a closure with the latest [`TokensConfig`] snapshot.
pub fn with_tokens_config<R>(selector: impl FnOnce(&TokensConfig) -> R) -> R {
    with_config(|cfg| selector(&cfg.tokens))
}

/// Get a cloned snapshot of the full [`TokensConfig`].
pub fn tokens_config_snapshot() -> TokensConfig {
    with_tokens_config(|cfg| cfg.clone())
}
