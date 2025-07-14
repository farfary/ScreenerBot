use crate::core::{ BotError, BotResult };
use solana_client::rpc_client::{ RpcClient, GetConfirmedSignaturesForAddress2Config };
use solana_transaction_status::UiTransactionEncoding;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use std::sync::{ Arc, Mutex };
use std::time::{ Duration, Instant };

pub struct RpcManager {
    pub client: RpcClient,
    last_request_time: Arc<Mutex<Instant>>,
    request_delay: Duration,
}

impl std::fmt::Debug for RpcManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcManager").field("request_delay", &self.request_delay).finish()
    }
}

impl RpcManager {
    pub fn new(rpc_url: &str) -> BotResult<Self> {
        let client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed()
        );

        Ok(Self {
            client,
            last_request_time: Arc::new(Mutex::new(Instant::now())),
            request_delay: Duration::from_millis(100), // 100ms delay between requests
        })
    }

    /// Execute operation with retry logic for network resilience
    fn retry_operation_sync<T, F>(&self, mut operation: F) -> BotResult<T>
        where F: FnMut() -> Result<T, solana_client::client_error::ClientError>
    {
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(1000 * attempt));
            }

            // Rate limiting
            {
                let mut last_time = self.last_request_time.lock().unwrap();
                let now = Instant::now();
                let elapsed = now.duration_since(*last_time);

                if elapsed < self.request_delay {
                    std::thread::sleep(self.request_delay - elapsed);
                }
                *last_time = Instant::now();
            }

            match operation() {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(BotError::Rpc(format!("Max retries exceeded: {:?}", last_error.unwrap())))
    }

    /// Get token accounts for a wallet
    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        program_id: &Pubkey
    ) -> BotResult<Vec<(Pubkey, solana_client::rpc_response::RpcKeyedAccount)>> {
        use solana_client::rpc_request::TokenAccountsFilter;

        let accounts = self.retry_operation_sync(|| {
            self.client.get_token_accounts_by_owner(
                owner,
                TokenAccountsFilter::ProgramId(*program_id)
            )
        })?;

        Ok(
            accounts
                .into_iter()
                .map(|account| { (account.pubkey.parse().unwrap(), account) })
                .collect()
        )
    }

    /// Get signatures for an address
    pub async fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        limit: usize
    ) -> BotResult<Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>> {
        self.retry_operation_sync(|| {
            let config = GetConfirmedSignaturesForAddress2Config {
                before: None,
                until: None,
                limit: Some(limit),
                commitment: Some(self.client.commitment()),
            };
            self.client.get_signatures_for_address_with_config(address, config)
        })
    }

    /// Get transaction details
    pub async fn get_transaction_with_config(
        &self,
        signature: &Signature,
        encoding: UiTransactionEncoding
    ) -> BotResult<solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta> {
        self.retry_operation_sync(|| {
            let config = solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(encoding),
                commitment: Some(self.client.commitment()),
                max_supported_transaction_version: Some(0),
            };
            self.client.get_transaction_with_config(signature, config)
        })
    }

    /// Send transaction
    pub async fn send_transaction(&self, transaction: &Transaction) -> BotResult<Signature> {
        self.retry_operation_sync(|| { self.client.send_and_confirm_transaction(transaction) })
    }

    /// Get current slot
    pub async fn get_slot(&self) -> BotResult<u64> {
        self.retry_operation_sync(|| { self.client.get_slot() })
    }

    /// Get block time
    pub async fn get_block_time(&self, slot: u64) -> BotResult<Option<i64>> {
        self.retry_operation_sync(|| { self.client.get_block_time(slot) }).map(Some)
    }

    /// Get balance
    pub async fn get_balance(&self, pubkey: &Pubkey) -> BotResult<u64> {
        self.retry_operation_sync(|| { self.client.get_balance(pubkey) })
    }

    /// Health check
    pub async fn health_check(&self) -> BotResult<()> {
        // Simple health check by getting the slot
        self.get_slot().await.map(|_| ())
    }

    /// Get latest blockhash for signing transactions
    pub fn get_latest_blockhash(
        &self
    ) -> Result<solana_sdk::hash::Hash, solana_client::client_error::ClientError> {
        self.client.get_latest_blockhash()
    }
}
