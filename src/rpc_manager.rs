use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

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
            .map(|url| { RpcClient::new_with_commitment(url, CommitmentConfig::confirmed()) })
            .collect();

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

            // Try the operation directly without spawn_blocking for simplicity
            match operation(client) {
                Ok(result) => {
                    // Update current index to successful RPC
                    self.current_index.store(index, std::sync::atomic::Ordering::Relaxed);
                    return Ok(result);
                }
                Err(e) => {
                    log::warn!("RPC attempt {} failed: {}", index + 1, e);
                    continue;
                }
            }
        }

        Err(anyhow::anyhow!("All RPC endpoints failed"))
    }

    pub fn get_current_client(&self) -> &RpcClient {
        let index = self.current_index.load(std::sync::atomic::Ordering::Relaxed);
        &self.clients[index]
    }

    pub fn get_client_count(&self) -> usize {
        self.clients.len()
    }
}
