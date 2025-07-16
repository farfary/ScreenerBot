use anyhow::Result;
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_client::rpc_response::RpcKeyedAccount;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::{ Transaction, TransactionError };
use std::sync::Arc;
use std::time::Duration;

pub struct RpcManager {
    clients: Vec<RpcClient>,
    current_index: Arc<std::sync::atomic::AtomicUsize>,
}

impl RpcManager {
    pub fn new(endpoints: Vec<String>) -> Result<Self> {
        let mut clients = Vec::new();

        for endpoint in endpoints {
            log::info!("🔗 Adding RPC endpoint: {}", endpoint);
            let client = RpcClient::new_with_timeout_and_commitment(
                endpoint,
                Duration::from_secs(30),
                CommitmentConfig::confirmed()
            );
            clients.push(client);
        }

        if clients.is_empty() {
            return Err(anyhow::anyhow!("No valid RPC endpoints provided"));
        }

        log::info!("✅ RPC Manager initialized with {} endpoints", clients.len());

        Ok(Self {
            clients,
            current_index: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    pub async fn execute_with_fallback_async<T, F, Fut>(&self, operation: F) -> Result<T>
        where
            F: Fn(&RpcClient) -> Fut + Send + Sync,
            Fut: std::future::Future<Output = Result<T>> + Send,
            T: Send + 'static
    {
        let start_index = self.current_index.load(std::sync::atomic::Ordering::Relaxed);

        for attempt in 0..self.clients.len() {
            let index = (start_index + attempt) % self.clients.len();
            let client = &self.clients[index];

            log::debug!("🔄 Trying RPC endpoint {} (attempt {})", index + 1, attempt + 1);

            match operation(client).await {
                Ok(result) => {
                    log::debug!("✅ RPC endpoint {} succeeded", index + 1);
                    if index != start_index {
                        self.current_index.store(index, std::sync::atomic::Ordering::Relaxed);
                    }
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

    pub fn execute_with_fallback<T, F>(&self, operation: F) -> Result<T>
        where F: Fn(&RpcClient) -> Result<T> + Send + Sync, T: Send + 'static
    {
        let start_index = self.current_index.load(std::sync::atomic::Ordering::Relaxed);

        for attempt in 0..self.clients.len() {
            let index = (start_index + attempt) % self.clients.len();
            let client = &self.clients[index];

            log::debug!("🔄 Trying RPC endpoint {} (attempt {})", index + 1, attempt + 1);

            match operation(client) {
                Ok(result) => {
                    log::debug!("✅ RPC endpoint {} succeeded", index + 1);
                    if index != start_index {
                        self.current_index.store(index, std::sync::atomic::Ordering::Relaxed);
                    }
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
        let result = self.execute_with_fallback(move |client| {
            client.get_balance(&pubkey).map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await; // Yield to allow other tasks to run
        result
    }

    pub async fn get_token_account_balance(&self, pubkey: &Pubkey) -> Result<UiTokenAmount> {
        let pubkey = *pubkey;
        let result = self.execute_with_fallback(move |client| {
            client
                .get_token_account_balance(&pubkey)
                .map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }

    pub async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature> {
        let transaction = transaction.clone();
        let result = self.execute_with_fallback(move |client| {
            client.send_transaction(&transaction).map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }

    pub async fn send_transaction_with_config(
        &self,
        transaction: &Transaction,
        config: RpcSendTransactionConfig
    ) -> Result<Signature> {
        let transaction = transaction.clone();
        let result = self.execute_with_fallback(move |client| {
            client
                .send_transaction_with_config(&transaction, config)
                .map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }

    pub async fn send_and_confirm_transaction(
        &self,
        transaction: &Transaction
    ) -> Result<Signature> {
        let transaction = transaction.clone();
        let result = self.execute_with_fallback(move |client| {
            client
                .send_and_confirm_transaction(&transaction)
                .map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }

    pub async fn get_block_height(&self) -> Result<u64> {
        let result = self.execute_with_fallback(|client| {
            client.get_block_height().map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }

    pub async fn get_signature_status_with_commitment(
        &self,
        signature: &Signature,
        commitment: CommitmentConfig
    ) -> Result<Option<Result<(), TransactionError>>> {
        let signature = *signature;
        let result = self.execute_with_fallback(move |client| {
            client
                .get_signature_status_with_commitment(&signature, commitment)
                .map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }

    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<solana_sdk::account::Account> {
        let pubkey = *pubkey;
        let result = self.execute_with_fallback(move |client| {
            client.get_account(&pubkey).map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }

    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        filter: TokenAccountsFilter
    ) -> Result<Vec<RpcKeyedAccount>> {
        let owner = *owner;

        // Extract the necessary data from filter before moving into closure
        let (is_mint, value) = match filter {
            TokenAccountsFilter::Mint(mint) => (true, mint),
            TokenAccountsFilter::ProgramId(program_id) => (false, program_id),
        };

        let result = self.execute_with_fallback(move |client| {
            let filter_clone = if is_mint {
                TokenAccountsFilter::Mint(value)
            } else {
                TokenAccountsFilter::ProgramId(value)
            };
            client
                .get_token_accounts_by_owner(&owner, filter_clone)
                .map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }

    pub async fn get_latest_blockhash(&self) -> Result<solana_sdk::hash::Hash> {
        let result = self.execute_with_fallback(move |client| {
            client.get_latest_blockhash().map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }

    pub async fn send_and_confirm_versioned_transaction(
        &self,
        transaction: &solana_sdk::transaction::VersionedTransaction
    ) -> Result<Signature> {
        let transaction_clone = transaction.clone();
        let result = self.execute_with_fallback(move |client| {
            client
                .send_and_confirm_transaction(&transaction_clone)
                .map_err(|e| anyhow::anyhow!("RPC error: {}", e))
        });
        tokio::task::yield_now().await;
        result
    }
}
