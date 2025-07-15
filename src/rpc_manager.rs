use anyhow::Result;
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

pub struct RpcManager {
    clients: Vec<RpcClient>,
    current_index: std::sync::atomic::AtomicUsize,
}

impl RpcManager {
    pub fn new(primary_url: String, fallback_urls: Vec<String>) -> Self {
        let mut urls = vec![primary_url];
        urls.extend(fallback_urls);

        let clients: Vec<RpcClient> = urls
            .into_iter()
            .map(|url| {
                log::info!("🔗 Initializing RPC client: {}", url);
                RpcClient::new_with_timeout_and_commitment(
                    url,
                    Duration::from_secs(30),
                    CommitmentConfig::confirmed()
                )
            })
            .collect();

        log::info!("✅ RPC Manager initialized with {} endpoints", clients.len());

        Self {
            clients,
            current_index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub async fn execute_with_fallback<T, F>(&self, operation: F) -> Result<T>
        where F: Fn(&RpcClient) -> Result<T> + Clone + Send + Sync + 'static, T: Send + 'static
    {
        let start_index = self.current_index.load(std::sync::atomic::Ordering::Relaxed);

        for attempt in 0..self.clients.len() {
            let index = (start_index + attempt) % self.clients.len();
            let client = &self.clients[index];

            log::debug!("🔄 Trying RPC endpoint {} (attempt {})", index + 1, attempt + 1);

            // Try the operation directly without spawn_blocking for simplicity
            match operation(client) {
                Ok(result) => {
                    // Update current index to successful RPC
                    self.current_index.store(index, std::sync::atomic::Ordering::Relaxed);
                    log::debug!("✅ RPC endpoint {} succeeded", index + 1);
                    return Ok(result);
                }
                Err(e) => {
                    log::warn!("❌ RPC endpoint {} failed: {}", index + 1, e);
                    continue;
                }
            }
        }

        log::error!("💥 All {} RPC endpoints failed", self.clients.len());
        Err(anyhow::anyhow!("All RPC endpoints failed"))
    }

    pub fn get_current_client(&self) -> &RpcClient {
        let index = self.current_index.load(std::sync::atomic::Ordering::Relaxed);
        &self.clients[index]
    }

    pub fn get_client_count(&self) -> usize {
        self.clients.len()
    }

    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        let pubkey = *pubkey;
        self.execute_with_fallback(move |client| {
            client.get_balance(&pubkey).map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        }).await
    }

    pub async fn get_token_account_balance(&self, pubkey: &Pubkey) -> Result<UiTokenAmount> {
        let pubkey = *pubkey;
        self.execute_with_fallback(move |client| {
            client
                .get_token_account_balance(&pubkey)
                .map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        }).await
    }
}
