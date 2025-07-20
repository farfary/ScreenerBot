// transactions/fetcher.rs - RPC fetching with batching and fallback support
use super::types::*;
use super::cache::*;
use crate::global::Configs;
use crate::logger::{ log, LogTag };
use reqwest;
use std::error::Error;
use tokio::time::{ sleep, Duration };
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Transaction fetcher with advanced batching and caching
pub struct TransactionFetcher {
    client: reqwest::Client,
    configs: Configs,
    batch_config: BatchConfig,
    db: TransactionDatabase,
}

impl TransactionFetcher {
    /// Create a new transaction fetcher
    pub fn new(configs: Configs, batch_config: Option<BatchConfig>) -> Result<Self, String> {
        let client = reqwest::Client
            ::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let db = TransactionDatabase::new().map_err(|e| e.to_string())?;
        let batch_config = batch_config.unwrap_or_default();

        Ok(Self {
            client,
            configs,
            batch_config,
            db,
        })
    }

    /// Get recent transaction signatures for a wallet address with RPC fallback support
    pub async fn get_recent_signatures(
        &self,
        wallet_address: &str,
        limit: usize
    ) -> Result<Vec<SignatureInfo>, String> {
        let actual_limit = limit.min(MAX_TRANSACTIONS_PER_REQUEST);

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSignaturesForAddress",
            "params": [
                wallet_address,
                {
                    "limit": actual_limit,
                    "commitment": "confirmed"
                }
            ]
        });

        log(
            LogTag::System,
            "INFO",
            &format!("Fetching recent {} signatures for wallet: {}", actual_limit, wallet_address)
        );

        // Try main RPC first
        match self.fetch_signatures_from_rpc(&self.configs.rpc_url, &rpc_payload).await {
            Ok(signatures) => {
                return Ok(signatures);
            }
            Err(e) => {
                if e.to_string().contains("429") {
                    log(LogTag::System, "WARNING", "Main RPC rate limited, trying fallbacks...");
                } else {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Main RPC failed: {}, trying fallbacks...", e)
                    );
                }
            }
        }

        // Try fallback RPCs
        for (i, fallback_rpc) in self.configs.rpc_fallbacks.iter().enumerate() {
            match self.fetch_signatures_from_rpc(fallback_rpc, &rpc_payload).await {
                Ok(signatures) => {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!("Got signatures from fallback RPC {} ({})", i + 1, fallback_rpc)
                    );
                    return Ok(signatures);
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!("Fallback RPC {} failed: {}", fallback_rpc, e)
                    );
                }
            }
        }

        Err("All RPC endpoints failed".to_string())
    }

    /// Get transaction signatures before a specific signature with RPC fallback support
    pub async fn get_signatures_before(
        &self,
        wallet_address: &str,
        limit: usize,
        before_signature: &str
    ) -> Result<Vec<SignatureInfo>, String> {
        let actual_limit = limit.min(MAX_TRANSACTIONS_PER_REQUEST);

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSignaturesForAddress",
            "params": [
                wallet_address,
                {
                    "limit": actual_limit,
                    "before": before_signature,
                    "commitment": "confirmed"
                }
            ]
        });

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Fetching {} signatures before {} for wallet: {}",
                actual_limit,
                before_signature,
                wallet_address
            )
        );

        // Try main RPC first
        match self.fetch_signatures_from_rpc(&self.configs.rpc_url, &rpc_payload).await {
            Ok(signatures) => {
                return Ok(signatures);
            }
            Err(e) => {
                if e.to_string().contains("429") {
                    log(LogTag::System, "WARNING", "Main RPC rate limited, trying fallbacks...");
                } else {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Main RPC failed: {}, trying fallbacks...", e)
                    );
                }
            }
        }

        // Try fallback RPCs
        for (i, fallback_rpc) in self.configs.rpc_fallbacks.iter().enumerate() {
            match self.fetch_signatures_from_rpc(fallback_rpc, &rpc_payload).await {
                Ok(signatures) => {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!("Got signatures from fallback RPC {} ({})", i + 1, fallback_rpc)
                    );
                    return Ok(signatures);
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!("Fallback RPC {} failed: {}", i + 1, e)
                    );
                }
            }
        }

        Err("All RPC endpoints failed".to_string())
    }

    /// Fetch signatures from a specific RPC endpoint
    async fn fetch_signatures_from_rpc(
        &self,
        rpc_url: &str,
        payload: &serde_json::Value
    ) -> Result<Vec<SignatureInfo>, Box<dyn Error>> {
        let response = self.client
            .post(rpc_url)
            .header("Content-Type", "application/json")
            .json(payload)
            .send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to get signatures: {}", response.status()).into());
        }

        let response_text = response.text().await?;
        let rpc_response: SignatureResponse = serde_json
            ::from_str(&response_text)
            .map_err(|e|
                format!(
                    "Failed to parse signature response: {} - Response text: {}",
                    e,
                    response_text
                )
            )?;

        if let Some(error) = rpc_response.error {
            return Err(format!("RPC error: {:?}", error).into());
        }

        match rpc_response.result {
            Some(signatures) => Ok(signatures),
            None => Err("No result in response".into()),
        }
    }

    /// Get detailed transaction information with rate limiting protection
    pub async fn get_transaction_details(
        &self,
        signature: &str,
        rpc_url: &str
    ) -> Result<Option<TransactionResult>, Box<dyn Error>> {
        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                signature,
                {
                    "encoding": "json",
                    "maxSupportedTransactionVersion": 0,
                    "commitment": "confirmed"
                }
            ]
        });

        let response = self.client
            .post(rpc_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send().await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 429 {
                return Err("Rate limited (429)".into());
            }
            return Err(format!("Failed to get transaction details: {}", status).into());
        }

        let response_text = response.text().await?;
        let rpc_response: TransactionResponse = serde_json
            ::from_str(&response_text)
            .map_err(|e|
                format!(
                    "Failed to parse transaction response: {} - Response text: {}",
                    e,
                    response_text
                )
            )?;

        if let Some(error) = rpc_response.error {
            return Err(format!("RPC error getting transaction: {:?}", error).into());
        }

        Ok(rpc_response.result)
    }

    /// Get detailed transaction information with RPC fallback support
    pub async fn get_transaction_details_with_fallback(
        &self,
        signature: &str
    ) -> Result<Option<TransactionResult>, String> {
        // Try main RPC first
        match self.get_transaction_details(signature, &self.configs.rpc_url).await {
            Ok(transaction) => {
                return Ok(transaction);
            }
            Err(e) => {
                if e.to_string().contains("429") {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!("Main RPC rate limited for transaction {}, trying fallbacks...", signature)
                    );
                } else {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!(
                            "Main RPC failed for transaction {}: {}, trying fallbacks...",
                            signature,
                            e
                        )
                    );
                }
            }
        }

        // Try fallback RPCs
        for (i, fallback_rpc) in self.configs.rpc_fallbacks.iter().enumerate() {
            match self.get_transaction_details(signature, fallback_rpc).await {
                Ok(transaction) => {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!(
                            "Got transaction details from fallback RPC {} ({})",
                            i + 1,
                            fallback_rpc
                        )
                    );
                    return Ok(transaction);
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!(
                            "Fallback RPC {} failed for transaction {}: {}",
                            fallback_rpc,
                            signature,
                            e
                        )
                    );
                }
            }
        }

        Err(format!("All RPC endpoints failed for transaction {}", signature))
    }

    /// Batch fetch transactions with intelligent caching and concurrency control
    pub async fn batch_fetch_transactions(
        &self,
        signatures: &[SignatureInfo],
        max_transactions: Option<usize>
    ) -> Result<Vec<(SignatureInfo, TransactionResult)>, Box<dyn Error>> {
        let limit = max_transactions.unwrap_or(signatures.len()).min(signatures.len());
        let target_signatures: Vec<_> = signatures.iter().take(limit).cloned().collect();

        log(
            LogTag::System,
            "INFO",
            &format!("Starting batch fetch of {} transactions", target_signatures.len())
        );

        // Get missing signatures that need to be fetched
        let signature_strings: Vec<String> = target_signatures
            .iter()
            .map(|s| s.signature.clone())
            .collect();
        let missing_signatures = self.db.get_missing_signatures(&signature_strings)?;

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Found {} cached, {} need fetching",
                target_signatures.len() - missing_signatures.len(),
                missing_signatures.len()
            )
        );

        // Fetch missing transactions in controlled batches
        let semaphore = Arc::new(Semaphore::new(self.batch_config.max_concurrent));
        let mut fetch_tasks = Vec::new();

        for batch in missing_signatures.chunks(self.batch_config.batch_size) {
            let batch_signatures = batch.to_vec();
            let semaphore = semaphore.clone();
            let fetcher = self.clone_for_task();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                fetcher.fetch_batch_with_retry(batch_signatures).await
            });

            fetch_tasks.push(task);

            // Delay between batches
            if fetch_tasks.len() < missing_signatures.chunks(self.batch_config.batch_size).len() {
                sleep(Duration::from_millis(self.batch_config.delay_between_batches_ms)).await;
            }
        }

        // Collect results from all batch tasks
        let mut fetched_transactions = Vec::new();
        for task in fetch_tasks {
            match task.await {
                Ok(Ok(batch_results)) => fetched_transactions.extend(batch_results),
                Ok(Err(e)) => log(LogTag::System, "WARNING", &format!("Batch fetch failed: {}", e)),
                Err(e) => log(LogTag::System, "WARNING", &format!("Batch task failed: {}", e)),
            }
        }

        // Store fetched transactions in database
        if !fetched_transactions.is_empty() {
            self.db.batch_upsert_transactions(&fetched_transactions)?;
        }

        // Collect all results (cached + fetched)
        let mut all_results = Vec::new();
        for sig_info in &target_signatures {
            if let Ok(Some(transaction)) = self.db.get_transaction(&sig_info.signature) {
                all_results.push((sig_info.clone(), transaction));
            }
        }

        log(
            LogTag::System,
            "SUCCESS",
            &format!("Batch fetch completed: {} total transactions retrieved", all_results.len())
        );
        Ok(all_results)
    }

    /// Clone fetcher for async tasks (without database connection)
    fn clone_for_task(&self) -> TaskFetcher {
        TaskFetcher {
            client: self.client.clone(),
            configs: self.configs.clone(),
            batch_config: self.batch_config.clone(),
        }
    }

    /// Fetch a batch of signatures with retry logic
    async fn fetch_batch_with_retry(
        &self,
        signatures: Vec<String>
    ) -> Result<Vec<(String, TransactionResult)>, String> {
        let mut results = Vec::new();

        for signature in signatures {
            let mut retry_count = 0;
            let mut delay_ms = 250;

            loop {
                match self.get_transaction_details_with_fallback(&signature).await {
                    Ok(Some(transaction)) => {
                        results.push((signature.clone(), transaction));
                        break;
                    }
                    Ok(None) => {
                        log(
                            LogTag::System,
                            "WARNING",
                            &format!("Transaction not found: {}", signature)
                        );
                        break;
                    }
                    Err(e) => {
                        if
                            e.to_string().contains("429") &&
                            retry_count < self.batch_config.max_retries
                        {
                            retry_count += 1;
                            log(
                                LogTag::System,
                                "WARNING",
                                &format!(
                                    "Rate limited on transaction {} (attempt {}/{}), waiting {}ms...",
                                    signature,
                                    retry_count,
                                    self.batch_config.max_retries + 1,
                                    delay_ms
                                )
                            );
                            sleep(Duration::from_millis(delay_ms)).await;
                            delay_ms *= 2; // Exponential backoff
                        } else {
                            log(
                                LogTag::System,
                                "WARNING",
                                &format!("Failed to get transaction {}: {}", signature, e)
                            );
                            break;
                        }
                    }
                }
            }

            // Delay between individual requests
            sleep(Duration::from_millis(self.batch_config.delay_between_requests_ms)).await;
        }

        Ok(results)
    }

    /// Sync transactions for a wallet (incremental update)
    pub async fn sync_wallet_transactions(
        &self,
        wallet_address: &str,
        max_new_transactions: Option<usize>
    ) -> Result<SyncStatus, Box<dyn Error>> {
        // Get current sync status
        let mut sync_status = self.db.get_sync_status(wallet_address)?.unwrap_or(SyncStatus {
            last_sync_slot: 0,
            last_sync_time: chrono::Utc::now() - chrono::Duration::hours(24), // Start from 24h ago
            total_transactions: 0,
            pending_transactions: 0,
        });

        // Fetch recent signatures
        let limit = max_new_transactions.unwrap_or(1000);
        let recent_signatures = self.get_recent_signatures(wallet_address, limit).await?;

        if recent_signatures.is_empty() {
            log(
                LogTag::System,
                "INFO",
                &format!("No new transactions found for wallet {}", wallet_address)
            );
            return Ok(sync_status);
        }

        // Filter signatures newer than last sync
        let new_signatures: Vec<_> = recent_signatures
            .into_iter()
            .filter(|sig| sig.slot > sync_status.last_sync_slot)
            .collect();

        if new_signatures.is_empty() {
            log(
                LogTag::System,
                "INFO",
                &format!("All transactions for wallet {} are already synced", wallet_address)
            );
            return Ok(sync_status);
        }

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Found {} new transactions for wallet {}",
                new_signatures.len(),
                wallet_address
            )
        );

        // Batch fetch new transactions
        let fetched = self.batch_fetch_transactions(&new_signatures, None).await?;

        // Update sync status
        if let Some(latest_sig) = new_signatures.first() {
            sync_status.last_sync_slot = latest_sig.slot;
        }
        sync_status.last_sync_time = chrono::Utc::now();
        sync_status.total_transactions += fetched.len() as u64;
        sync_status.pending_transactions = 0; // Reset after successful sync

        // Save updated sync status
        self.db.update_sync_status(wallet_address, &sync_status)?;

        log(
            LogTag::System,
            "SUCCESS",
            &format!("Synced {} new transactions for wallet {}", fetched.len(), wallet_address)
        );
        Ok(sync_status)
    }
}

/// Lightweight fetcher for async tasks
#[derive(Clone)]
struct TaskFetcher {
    client: reqwest::Client,
    configs: Configs,
    batch_config: BatchConfig,
}

impl TaskFetcher {
    async fn get_transaction_details_with_fallback(
        &self,
        signature: &str
    ) -> Result<Option<TransactionResult>, String> {
        // Simplified version without database access for async tasks
        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                signature,
                {
                    "encoding": "json",
                    "maxSupportedTransactionVersion": 0,
                    "commitment": "confirmed"
                }
            ]
        });

        // Try main RPC first
        if
            let Ok(result) = self.fetch_transaction_from_rpc(
                &self.configs.rpc_url,
                &rpc_payload
            ).await
        {
            return Ok(result);
        }

        // Try fallback RPCs
        for fallback_rpc in &self.configs.rpc_fallbacks {
            if let Ok(result) = self.fetch_transaction_from_rpc(fallback_rpc, &rpc_payload).await {
                return Ok(result);
            }
        }

        Err(format!("All RPC endpoints failed for transaction {}", signature))
    }

    async fn fetch_transaction_from_rpc(
        &self,
        rpc_url: &str,
        payload: &serde_json::Value
    ) -> Result<Option<TransactionResult>, String> {
        let response = self.client
            .post(rpc_url)
            .header("Content-Type", "application/json")
            .json(payload)
            .send().await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let response_text = response.text().await.map_err(|e| e.to_string())?;
        let rpc_response: TransactionResponse = serde_json
            ::from_str(&response_text)
            .map_err(|e| e.to_string())?;

        if let Some(error) = rpc_response.error {
            return Err(format!("RPC error: {:?}", error));
        }

        Ok(rpc_response.result)
    }

    async fn fetch_batch_with_retry(
        &self,
        signatures: Vec<String>
    ) -> Result<Vec<(String, TransactionResult)>, String> {
        let mut results = Vec::new();

        for signature in signatures {
            let mut retry_count = 0;
            let mut delay_ms = 250;

            loop {
                match self.get_transaction_details_with_fallback(&signature).await {
                    Ok(Some(transaction)) => {
                        results.push((signature.clone(), transaction));
                        break;
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(e) => {
                        let error_str = e.to_string();
                        if error_str.contains("429") && retry_count < self.batch_config.max_retries {
                            retry_count += 1;
                            sleep(Duration::from_millis(delay_ms)).await;
                            delay_ms *= 2;
                        } else {
                            break;
                        }
                    }
                }
            }

            sleep(Duration::from_millis(self.batch_config.delay_between_requests_ms)).await;
        }

        Ok(results)
    }
}

/// Legacy compatibility functions
pub async fn get_recent_signatures_with_fallback(
    client: &reqwest::Client,
    wallet_address: &str,
    configs: &Configs,
    limit: usize
) -> Result<Vec<SignatureInfo>, String> {
    let fetcher = TransactionFetcher::new(configs.clone(), None)?;
    fetcher.get_recent_signatures(wallet_address, limit).await
}

pub async fn get_transactions_with_cache_and_fallback(
    client: &reqwest::Client,
    signatures: &[SignatureInfo],
    configs: &Configs,
    max_transactions: Option<usize>
) -> Vec<(SignatureInfo, TransactionResult)> {
    match TransactionFetcher::new(configs.clone(), None) {
        Ok(fetcher) => {
            match fetcher.batch_fetch_transactions(signatures, max_transactions).await {
                Ok(results) => results,
                Err(e) => {
                    log(LogTag::System, "ERROR", &format!("Batch fetch failed: {}", e));
                    Vec::new()
                }
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to create fetcher: {}", e));
            Vec::new()
        }
    }
}
