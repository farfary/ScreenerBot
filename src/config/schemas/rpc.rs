/// RPC endpoint configuration
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// RPC endpoint configuration
    pub struct RpcConfig {
        /// List of RPC URLs to use (round-robin)
        #[metadata(field_metadata! {
            label: "RPC URLs",
            hint: "Comma-separated RPC endpoints (round-robin)",
            impact: "critical",
            category: "Endpoints",
        })]
        urls: Vec<String> = vec!["https://api.mainnet-beta.solana.com".to_string()],
    }
}
