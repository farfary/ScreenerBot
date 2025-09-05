use crate::logger::{ log, LogTag };
use crate::pool_calculator::get_pool_calculator;
use crate::pool_cleanup::{ cleanup_service_state, CleanupServiceState };
use crate::pool_monitor::{ monitor_service_health, MonitorServiceState, TaskState, TaskStatus };
use crate::pool_tokens::{ init_pool_tokens, update_tracked_tokens_in_state };
use crate::pool_discovery::{ discover_and_process_pools, PoolData, AccountInfo };
use crate::pool_fetcher::{
    fetch_account_data_for_pool_service,
    fetch_token_accounts_for_pool_service,
};
use crate::pool_interface::{ PoolInterface, PoolStats, TokenPriceInfo };
use async_trait::async_trait;
use chrono::{ DateTime, Utc };
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::{ RwLock, Mutex };
use tokio::time::{ interval, sleep };
use crate::pool_constants::*;

// =============================================================================
// DATA STRUCTURES
// =============================================================================

impl PoolData {
    /// Convert PoolData to PoolInfo for calculator
    pub fn to_pool_info(&self) -> crate::pool_calculator::PoolInfo {
        crate::pool_calculator::PoolInfo {
            pool_address: self.pool_address.clone(),
            pool_program_id: self.get_program_id(),
            pool_type: self.dex_type.clone(),
            token_0_mint: self.token_mint.clone(),
            token_1_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL mint
            token_0_vault: None,
            token_1_vault: None,
            token_0_reserve: self.reserve_token as u64,
            token_1_reserve: self.reserve_sol as u64,
            token_0_decimals: 9, // Default token decimals
            token_1_decimals: 9, // SOL decimals
            lp_mint: None,
            lp_supply: None,
            creator: None,
            status: Some(1), // Active status
            liquidity_usd: Some(self.liquidity_usd),
            sqrt_price: None,
        }
    }

    /// Get program ID based on DEX type
    fn get_program_id(&self) -> String {
        match self.dex_type.as_str() {
            "Raydium" => RAYDIUM_CPMM_PROGRAM_ID.to_string(),
            "Meteora" => METEORA_DAMM_V2_PROGRAM_ID.to_string(),
            "Orca" => ORCA_WHIRLPOOL_PROGRAM_ID.to_string(),
            "Pump Fun" => PUMP_FUN_AMM_PROGRAM_ID.to_string(),
            _ => RAYDIUM_CPMM_PROGRAM_ID.to_string(), // Default
        }
    }
}

/// Service shared state
#[derive(Debug)]
pub struct ServiceState {
    /// All tokens being tracked
    pub tracked_tokens: HashMap<String, DateTime<Utc>>, // token_mint -> last_seen
    /// Best pool for each token (highest liquidity)
    pub best_pools: HashMap<String, PoolData>, // token_mint -> pool_data
    /// Account addresses to fetch data for
    pub account_queue: Vec<AccountInfo>,
    /// Raw account data cache
    pub account_data_cache: HashMap<String, (Vec<u8>, DateTime<Utc>)>, // address -> (data, timestamp)
    /// Task statuses
    pub task_statuses: HashMap<String, TaskStatus>,
}

impl ServiceState {
    pub fn new() -> Self {
        Self {
            tracked_tokens: HashMap::new(),
            best_pools: HashMap::new(),
            account_queue: Vec::new(),
            account_data_cache: HashMap::new(),
            task_statuses: HashMap::new(),
        }
    }
}

/// Simple pool service that provides cached price data
pub struct PoolService {
    /// Price cache: token_mint -> TokenPriceInfo
    price_cache: Arc<RwLock<HashMap<String, TokenPriceInfo>>>,
    /// Available tokens list
    available_tokens: Arc<RwLock<Vec<String>>>,
    /// Service statistics
    stats: Arc<RwLock<PoolStats>>,
    /// Service state
    is_running: Arc<RwLock<bool>>,
    /// Shared state between tasks
    shared_state: Arc<RwLock<ServiceState>>,
    /// Shutdown signal
    shutdown_signal: Arc<Mutex<bool>>,
}

// =============================================================================
// IMPLEMENTATIONS
// =============================================================================

impl PoolService {
    /// Create new pool service
    pub fn new() -> Self {
        Self {
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            available_tokens: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(PoolStats::default())),
            is_running: Arc::new(RwLock::new(false)),
            shared_state: Arc::new(RwLock::new(ServiceState::new())),
            shutdown_signal: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the pool service
    pub async fn start(&self) -> Result<(), String> {
        let mut running = self.is_running.write().await;
        if *running {
            log(LogTag::Pool, "SERVICE_ALREADY_RUNNING", "Pool service is already running");
            return Ok(());
        }
        *running = true;
        drop(running);

        log(LogTag::Pool, "SERVICE_START", "🚀 Starting Pool Service");

        // Initialize pool calculator, fetcher, discovery, cleanup, monitor, and tokens services
        crate::pool_calculator::init_pool_calculator();
        crate::pool_fetcher::init_pool_fetcher();
        crate::pool_discovery::init_pool_discovery();
        crate::pool_cleanup::init_pool_cleanup();
        crate::pool_monitor::init_pool_monitor();
        init_pool_tokens().map_err(|e| format!("Failed to initialize pool tokens service: {}", e))?;

        // Reset shutdown signal
        {
            let mut shutdown = self.shutdown_signal.lock().await;
            *shutdown = false;
        }

        // Initialize task statuses
        {
            let mut state = self.shared_state.write().await;
            state.task_statuses.insert(
                "tokens_list".to_string(),
                TaskStatus::new("Tokens List Preparing")
            );
            state.task_statuses.insert(
                "pool_discovery".to_string(),
                TaskStatus::new("Pool Discovery")
            );
            state.task_statuses.insert(
                "account_fetcher".to_string(),
                TaskStatus::new("Account Data Fetcher")
            );
            state.task_statuses.insert(
                "pool_fetcher".to_string(),
                TaskStatus::new("Pool Token Fetcher")
            );
            state.task_statuses.insert(
                "price_calculator".to_string(),
                TaskStatus::new("Price Calculator")
            );
            state.task_statuses.insert("cleanup".to_string(), TaskStatus::new("Cleanup Task"));
            state.task_statuses.insert(
                "pool_calculator".to_string(),
                TaskStatus::new("Pool Calculator")
            );
            state.task_statuses.insert(
                "state_monitor".to_string(),
                TaskStatus::new("State Monitor")
            );
        }

        // Start all background tasks
        self.start_background_tasks().await;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.last_update = Some(Utc::now());
        }

        log(LogTag::Pool, "SERVICE_READY", "✅ Pool Service started successfully");
        Ok(())
    }

    /// Stop the pool service
    pub async fn stop(&self) {
        log(LogTag::Pool, "SERVICE_STOPPING", "🛑 Stopping Pool Service...");

        // Signal all tasks to shutdown
        {
            let mut shutdown = self.shutdown_signal.lock().await;
            *shutdown = true;
        }

        // Wait a bit for tasks to finish gracefully
        sleep(Duration::from_secs(2)).await;

        let mut running = self.is_running.write().await;
        *running = false;

        log(LogTag::Pool, "SERVICE_STOP", "🛑 Pool Service stopped");
    }

    /// Update available tokens list
    pub async fn update_available_tokens(&self, tokens: Vec<String>) {
        let mut available = self.available_tokens.write().await;
        *available = tokens;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_tokens_available = available.len();
        stats.last_update = Some(Utc::now());
    }

    /// Update price cache with new price data
    pub async fn update_price_cache(&self, token_mint: String, price_info: TokenPriceInfo) {
        let mut cache = self.price_cache.write().await;
        cache.insert(token_mint, price_info);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.successful_price_fetches += 1;
        stats.last_update = Some(Utc::now());
    }

    /// Get service statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    /// Check if service is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Start all background tasks
    async fn start_background_tasks(&self) {
        log(LogTag::Pool, "TASKS_START", "🚀 Starting background tasks");

        // Create clones of the necessary data for background tasks
        let price_cache = self.price_cache.clone();
        let available_tokens = self.available_tokens.clone();
        let stats = self.stats.clone();
        let shared_state = self.shared_state.clone();
        let shutdown_signal = self.shutdown_signal.clone();

        // 1. Tokens List Preparing Task
        {
            let price_cache = price_cache.clone();
            let available_tokens = available_tokens.clone();
            let stats = stats.clone();
            let shared_state = shared_state.clone();
            let shutdown_signal = shutdown_signal.clone();

            tokio::spawn(async move {
                Self::tokens_list_task_impl(shared_state, shutdown_signal).await;
            });
        }

        // 2. Pool Discovery Task
        {
            let price_cache = price_cache.clone();
            let available_tokens = available_tokens.clone();
            let stats = stats.clone();
            let shared_state = shared_state.clone();
            let shutdown_signal = shutdown_signal.clone();

            tokio::spawn(async move {
                Self::pool_discovery_task_impl(shared_state, shutdown_signal).await;
            });
        }

        // 3. Account Data Fetcher Task
        {
            let price_cache = price_cache.clone();
            let available_tokens = available_tokens.clone();
            let stats = stats.clone();
            let shared_state = shared_state.clone();
            let shutdown_signal = shutdown_signal.clone();

            tokio::spawn(async move {
                Self::account_fetcher_task_impl(shared_state, shutdown_signal).await;
            });
        }

        // 4. Pool Fetcher Task
        {
            let price_cache = price_cache.clone();
            let available_tokens = available_tokens.clone();
            let stats = stats.clone();
            let shared_state = shared_state.clone();
            let shutdown_signal = shutdown_signal.clone();

            tokio::spawn(async move {
                Self::pool_fetcher_task_impl(shared_state, shutdown_signal).await;
            });
        }

        // 5. Price Calculator Task
        {
            let price_cache = price_cache.clone();
            let available_tokens = available_tokens.clone();
            let stats = stats.clone();
            let shared_state = shared_state.clone();
            let shutdown_signal = shutdown_signal.clone();

            tokio::spawn(async move {
                Self::price_calculator_task_impl(shared_state, shutdown_signal, price_cache).await;
            });
        }

        // 6. Cleanup Task
        {
            let price_cache = price_cache.clone();
            let available_tokens = available_tokens.clone();
            let stats = stats.clone();
            let shared_state = shared_state.clone();
            let shutdown_signal = shutdown_signal.clone();

            tokio::spawn(async move {
                Self::cleanup_task_impl(shared_state, shutdown_signal, price_cache).await;
            });
        }

        // 7. Pool Calculator Task
        {
            let price_cache = price_cache.clone();
            let available_tokens = available_tokens.clone();
            let stats = stats.clone();
            let shared_state = shared_state.clone();
            let shutdown_signal = shutdown_signal.clone();

            tokio::spawn(async move {
                Self::pool_calculator_task_impl(shared_state, shutdown_signal, price_cache).await;
            });
        }

        // 8. State Monitor Task
        {
            let shared_state = shared_state.clone();
            let shutdown_signal = shutdown_signal.clone();

            tokio::spawn(async move {
                Self::state_monitor_task_impl(shared_state, shutdown_signal).await;
            });
        }

        log(LogTag::Pool, "TASKS_STARTED", "✅ All background tasks started");
    }

    /// Check if shutdown signal is set
    async fn should_shutdown(shutdown_signal: &Arc<Mutex<bool>>) -> bool {
        *shutdown_signal.lock().await
    }

    /// Update task status
    async fn update_task_status(
        shared_state: &Arc<RwLock<ServiceState>>,
        task_name: &str,
        state: TaskState,
        error: Option<String>
    ) {
        let mut state_lock = shared_state.write().await;
        if let Some(status) = state_lock.task_statuses.get_mut(task_name) {
            status.state = state;
            status.last_run = Some(Utc::now());
            status.run_count += 1;
            if let Some(err) = error {
                status.error_count += 1;
                status.last_error = Some(err);
            }
        }
    }

    // =============================================================================
    // BACKGROUND TASKS (Static implementations)
    // =============================================================================

    /// Task 1: Tokens List Preparing Task
    /// Maintains the list of tokens to track based on various criteria
    async fn tokens_list_task_impl(
        shared_state: Arc<RwLock<ServiceState>>,
        shutdown_signal: Arc<Mutex<bool>>
    ) {
        log(LogTag::Pool, "TOKENS_LIST_START", "🔄 Starting Tokens List Task");
        let mut interval = interval(Duration::from_secs(TOKENS_LIST_INTERVAL_SECS));

        loop {
            if Self::should_shutdown(&shutdown_signal).await {
                break;
            }

            Self::update_task_status(&shared_state, "tokens_list", TaskState::Running, None).await;

            // TODO: Implement token list preparation logic
            // - Fetch trending tokens from various APIs
            // - Filter tokens based on criteria (volume, market cap, etc.)
            // - Merge with existing tracked tokens
            // - Remove inactive tokens
            let result = Self::prepare_tokens_list_impl(&shared_state).await;

            match result {
                Ok(count) => {
                    log(
                        LogTag::Pool,
                        "TOKENS_LIST_SUCCESS",
                        &format!("Updated tokens list: {} tokens", count)
                    );
                    Self::update_task_status(
                        &shared_state,
                        "tokens_list",
                        TaskState::Running,
                        None
                    ).await;
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "TOKENS_LIST_ERROR",
                        &format!("Failed to update tokens list: {}", e)
                    );
                    Self::update_task_status(
                        &shared_state,
                        "tokens_list",
                        TaskState::Error(e.clone()),
                        Some(e)
                    ).await;
                }
            }

            interval.tick().await;
        }

        log(LogTag::Pool, "TOKENS_LIST_STOP", "🛑 Tokens List Task stopped");
    }

    /// Task 2: Pool Discovery Task
    /// Discovers pools from APIs and caches them in database
    async fn pool_discovery_task_impl(
        shared_state: Arc<RwLock<ServiceState>>,
        shutdown_signal: Arc<Mutex<bool>>
    ) {
        log(LogTag::Pool, "POOL_DISCOVERY_START", "🔄 Starting Pool Discovery Task");
        let mut interval = interval(Duration::from_secs(POOL_DISCOVERY_INTERVAL_SECS));

        loop {
            if Self::should_shutdown(&shutdown_signal).await {
                break;
            }

            Self::update_task_status(
                &shared_state,
                "pool_discovery",
                TaskState::Running,
                None
            ).await;

            // Use pool discovery service to find pools for tracked tokens
            let result = Self::discover_pools_impl(&shared_state).await;

            match result {
                Ok(count) => {
                    log(
                        LogTag::Pool,
                        "POOL_DISCOVERY_SUCCESS",
                        &format!("Discovered {} pools", count)
                    );
                    Self::update_task_status(
                        &shared_state,
                        "pool_discovery",
                        TaskState::Running,
                        None
                    ).await;
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "POOL_DISCOVERY_ERROR",
                        &format!("Pool discovery failed: {}", e)
                    );
                    Self::update_task_status(
                        &shared_state,
                        "pool_discovery",
                        TaskState::Error(e.clone()),
                        Some(e)
                    ).await;
                }
            }

            interval.tick().await;
        }

        log(LogTag::Pool, "POOL_DISCOVERY_STOP", "🛑 Pool Discovery Task stopped");
    }

    /// Task 3: Account Data Fetcher Task
    /// Efficiently fetches account data using get_multiple_accounts
    async fn account_fetcher_task_impl(
        shared_state: Arc<RwLock<ServiceState>>,
        shutdown_signal: Arc<Mutex<bool>>
    ) {
        log(LogTag::Pool, "ACCOUNT_FETCHER_START", "🔄 Starting Account Data Fetcher Task");
        let mut interval = interval(Duration::from_secs(ACCOUNT_FETCH_INTERVAL_SECS));

        loop {
            if Self::should_shutdown(&shutdown_signal).await {
                break;
            }

            Self::update_task_status(
                &shared_state,
                "account_fetcher",
                TaskState::Running,
                None
            ).await;

            // Use pool fetcher service to fetch account data
            let result = Self::fetch_account_data_impl(&shared_state).await;

            match result {
                Ok(count) => {
                    if count > 0 {
                        log(
                            LogTag::Pool,
                            "ACCOUNT_FETCHER_SUCCESS",
                            &format!("Fetched {} accounts", count)
                        );
                    }
                    Self::update_task_status(
                        &shared_state,
                        "account_fetcher",
                        TaskState::Running,
                        None
                    ).await;
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "ACCOUNT_FETCHER_ERROR",
                        &format!("Account fetching failed: {}", e)
                    );
                    Self::update_task_status(
                        &shared_state,
                        "account_fetcher",
                        TaskState::Error(e.clone()),
                        Some(e)
                    ).await;
                }
            }

            interval.tick().await;
        }

        log(LogTag::Pool, "ACCOUNT_FETCHER_STOP", "🛑 Account Data Fetcher Task stopped");
    }

    /// Task 4: Pool Fetcher Task
    /// Fetches token account data for all tokens that pool service tasks need
    async fn pool_fetcher_task_impl(
        shared_state: Arc<RwLock<ServiceState>>,
        shutdown_signal: Arc<Mutex<bool>>
    ) {
        log(LogTag::Pool, "POOL_FETCHER_START", "🔄 Starting Pool Fetcher Task");
        let mut interval = interval(Duration::from_secs(ACCOUNT_FETCH_INTERVAL_SECS));

        loop {
            if Self::should_shutdown(&shutdown_signal).await {
                break;
            }

            Self::update_task_status(&shared_state, "pool_fetcher", TaskState::Running, None).await;

            // Fetch token account data for all tracked tokens
            let result = Self::fetch_token_accounts_impl(&shared_state).await;

            match result {
                Ok(count) => {
                    if count > 0 {
                        log(
                            LogTag::Pool,
                            "POOL_FETCHER_SUCCESS",
                            &format!("Fetched {} token accounts", count)
                        );
                    }
                    Self::update_task_status(
                        &shared_state,
                        "pool_fetcher",
                        TaskState::Running,
                        None
                    ).await;
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "POOL_FETCHER_ERROR",
                        &format!("Token account fetching failed: {}", e)
                    );
                    Self::update_task_status(
                        &shared_state,
                        "pool_fetcher",
                        TaskState::Error(e.clone()),
                        Some(e)
                    ).await;
                }
            }

            interval.tick().await;
        }

        log(LogTag::Pool, "POOL_FETCHER_STOP", "🛑 Pool Fetcher Task stopped");
    }

    /// Task 5: Price Calculator Task
    /// Calculates token prices from available account data
    async fn price_calculator_task_impl(
        shared_state: Arc<RwLock<ServiceState>>,
        shutdown_signal: Arc<Mutex<bool>>,
        price_cache: Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) {
        log(LogTag::Pool, "PRICE_CALCULATOR_START", "🔄 Starting Price Calculator Task");
        let mut interval = interval(Duration::from_secs(PRICE_CALC_INTERVAL_SECS));

        loop {
            if Self::should_shutdown(&shutdown_signal).await {
                break;
            }

            Self::update_task_status(
                &shared_state,
                "price_calculator",
                TaskState::Running,
                None
            ).await;

            // Calculate prices from available account data
            let result = Self::calculate_prices_impl(&shared_state, &price_cache).await;

            match result {
                Ok(count) => {
                    if count > 0 {
                        log(
                            LogTag::Pool,
                            "PRICE_CALCULATOR_SUCCESS",
                            &format!("Calculated {} prices", count)
                        );
                    }
                    Self::update_task_status(
                        &shared_state,
                        "price_calculator",
                        TaskState::Running,
                        None
                    ).await;
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "PRICE_CALCULATOR_ERROR",
                        &format!("Price calculation failed: {}", e)
                    );
                    Self::update_task_status(
                        &shared_state,
                        "price_calculator",
                        TaskState::Error(e.clone()),
                        Some(e)
                    ).await;
                }
            }

            interval.tick().await;
        }

        log(LogTag::Pool, "PRICE_CALCULATOR_STOP", "🛑 Price Calculator Task stopped");
    }

    /// Task 6: Cleanup Task
    /// Manages memory and database cleanup
    async fn cleanup_task_impl(
        shared_state: Arc<RwLock<ServiceState>>,
        shutdown_signal: Arc<Mutex<bool>>,
        price_cache: Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) {
        log(LogTag::Pool, "CLEANUP_START", "🔄 Starting Cleanup Task");
        let mut interval = interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));

        loop {
            if Self::should_shutdown(&shutdown_signal).await {
                break;
            }

            Self::update_task_status(&shared_state, "cleanup", TaskState::Running, None).await;

            // TODO: Implement cleanup logic
            // - Remove stale price cache entries
            // - Clean up old account data cache
            // - Remove inactive tokens from tracking
            // - Database cleanup operations
            let result = Self::cleanup_data_impl(&shared_state, &price_cache).await;

            match result {
                Ok(count) => {
                    log(LogTag::Pool, "CLEANUP_SUCCESS", &format!("Cleaned up {} items", count));
                    Self::update_task_status(
                        &shared_state,
                        "cleanup",
                        TaskState::Running,
                        None
                    ).await;
                }
                Err(e) => {
                    log(LogTag::Pool, "CLEANUP_ERROR", &format!("Cleanup failed: {}", e));
                    Self::update_task_status(
                        &shared_state,
                        "cleanup",
                        TaskState::Error(e.clone()),
                        Some(e)
                    ).await;
                }
            }

            interval.tick().await;
        }

        log(LogTag::Pool, "CLEANUP_STOP", "🛑 Cleanup Task stopped");
    }

    /// Task 7: Pool Calculator Task
    /// Processes pool calculations using the dedicated calculator service
    async fn pool_calculator_task_impl(
        shared_state: Arc<RwLock<ServiceState>>,
        shutdown_signal: Arc<Mutex<bool>>,
        price_cache: Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) {
        log(LogTag::Pool, "POOL_CALCULATOR_START", "🔄 Starting Pool Calculator Task");
        let mut interval = interval(Duration::from_secs(PRICE_CALC_INTERVAL_SECS));

        loop {
            if Self::should_shutdown(&shutdown_signal).await {
                break;
            }

            Self::update_task_status(
                &shared_state,
                "pool_calculator",
                TaskState::Running,
                None
            ).await;

            // Process pool calculations using the calculator service
            let result = Self::process_pool_calculations_impl(&shared_state, &price_cache).await;

            match result {
                Ok(count) => {
                    if count > 0 {
                        log(
                            LogTag::Pool,
                            "POOL_CALCULATOR_SUCCESS",
                            &format!("Processed {} pool calculations", count)
                        );
                    }
                    Self::update_task_status(
                        &shared_state,
                        "pool_calculator",
                        TaskState::Running,
                        None
                    ).await;
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "POOL_CALCULATOR_ERROR",
                        &format!("Pool calculation failed: {}", e)
                    );
                    Self::update_task_status(
                        &shared_state,
                        "pool_calculator",
                        TaskState::Error(e.clone()),
                        Some(e)
                    ).await;
                }
            }

            interval.tick().await;
        }

        log(LogTag::Pool, "POOL_CALCULATOR_STOP", "🛑 Pool Calculator Task stopped");
    }

    /// Task 8: State Monitor Task
    /// Monitors all task states and provides health checking
    async fn state_monitor_task_impl(
        shared_state: Arc<RwLock<ServiceState>>,
        shutdown_signal: Arc<Mutex<bool>>
    ) {
        log(LogTag::Pool, "STATE_MONITOR_START", "🔄 Starting State Monitor Task");
        let mut interval = interval(Duration::from_secs(STATE_MONITOR_INTERVAL_SECS));

        loop {
            if Self::should_shutdown(&shutdown_signal).await {
                break;
            }

            Self::update_task_status(
                &shared_state,
                "state_monitor",
                TaskState::Running,
                None
            ).await;

            // TODO: Implement state monitoring logic
            // - Check health of all tasks
            // - Report task statistics
            // - Detect and handle task failures
            // - Update service statistics
            let result = Self::monitor_states_impl(&shared_state).await;

            match result {
                Ok(_) => {
                    Self::update_task_status(
                        &shared_state,
                        "state_monitor",
                        TaskState::Running,
                        None
                    ).await;
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "STATE_MONITOR_ERROR",
                        &format!("State monitoring failed: {}", e)
                    );
                    Self::update_task_status(
                        &shared_state,
                        "state_monitor",
                        TaskState::Error(e.clone()),
                        Some(e)
                    ).await;
                }
            }

            interval.tick().await;
        }

        log(LogTag::Pool, "STATE_MONITOR_STOP", "🛑 State Monitor Task stopped");
    }

    // =============================================================================
    // TASK IMPLEMENTATION METHODS
    // =============================================================================

    /// Prepare tokens list from database using pool tokens service
    async fn prepare_tokens_list_impl(
        shared_state: &Arc<RwLock<ServiceState>>
    ) -> Result<usize, String> {
        // Load tokens from database using pool tokens service
        let loaded_count = crate::pool_tokens::load_tokens_from_database().await?;

        // Update tracked tokens in shared state
        let updated_count = {
            let mut state = shared_state.write().await;
            update_tracked_tokens_in_state(&mut state.tracked_tokens).await?
        };

        log(
            LogTag::Pool,
            "TOKENS_LIST_UPDATE",
            &format!(
                "Loaded {} tokens from database, updated {} tracked tokens",
                loaded_count,
                updated_count
            )
        );

        Ok(updated_count)
    }

    /// Discover pools for tracked tokens using the pool discovery service
    async fn discover_pools_impl(
        shared_state: &Arc<RwLock<ServiceState>>
    ) -> Result<usize, String> {
        // Get all tracked tokens
        let tracked_tokens: Vec<String> = {
            let state = shared_state.read().await;
            state.tracked_tokens.keys().cloned().collect()
        };

        if tracked_tokens.is_empty() {
            return Ok(0);
        }

        // Use the pool discovery service to discover and process pools
        let (best_pools, account_queue) = discover_and_process_pools(&tracked_tokens).await?;

        let discovered_count = best_pools.len();

        // Update shared state with discovered pools and account queue
        {
            let mut state = shared_state.write().await;
            state.best_pools.extend(best_pools);
            state.account_queue.extend(account_queue);
        }

        Ok(discovered_count)
    }

    /// Fetch account data in batches using the pool fetcher service
    async fn fetch_account_data_impl(
        shared_state: &Arc<RwLock<ServiceState>>
    ) -> Result<usize, String> {
        // Use the pool fetcher service to fetch account data
        let mut state = shared_state.write().await;
        // Fetch and collect account data in a single call
        let fetched_map = crate::pool_fetcher
            ::get_pool_fetcher()
            .fetch_and_collect_account_data_for_pool_service(&mut state.account_queue).await?;

        let now = Utc::now();
        let fetched_len = fetched_map.len();
        for (addr, data) in fetched_map {
            state.account_data_cache.insert(addr, (data, now));
        }

        // Decode pool accounts to update reserves for best pools
        let sol_mint = "So11111111111111111111111111111111111111112";
        let mut updated = 0usize;
        // Prepare a vector of (token_mint, pool_address, program_id)
        let mut decode_targets: Vec<(String, String, String)> = Vec::new();
        for (token_mint, pool) in state.best_pools.iter() {
            decode_targets.push((
                token_mint.clone(),
                pool.pool_address.clone(),
                pool.get_program_id(),
            ));
        }

        for (token_mint, pool_addr, program_id) in decode_targets {
            if let Some((pool_bytes, _ts)) = state.account_data_cache.get(&pool_addr) {
                match crate::pool_decoders::decode_pool_data_by_program_id(&program_id, pool_bytes) {
                    Ok(decoded) => {
                        if let Some(pool) = state.best_pools.get_mut(&token_mint) {
                            let mint_a = decoded.token_a_mint.to_string();
                            let mint_b = decoded.token_b_mint.to_string();
                            if mint_a == sol_mint {
                                pool.reserve_sol = decoded.token_a_reserve as f64;
                                pool.reserve_token = decoded.token_b_reserve as f64;
                            } else if mint_b == sol_mint {
                                pool.reserve_sol = decoded.token_b_reserve as f64;
                                pool.reserve_token = decoded.token_a_reserve as f64;
                            } else {
                                pool.reserve_token = decoded.token_a_reserve as f64;
                                pool.reserve_sol = decoded.token_b_reserve as f64;
                            }
                            updated += 1;
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "DECODE_WARN",
                            &format!(
                                "Decode failed for pool {}: {}",
                                crate::utils::safe_truncate(&pool_addr, 8),
                                e
                            )
                        );
                    }
                }
            }
        }

        if updated > 0 {
            log(
                LogTag::Pool,
                "RESERVES_UPDATED",
                &format!("Updated reserves for {} pools", updated)
            );
        }

        Ok(fetched_len)
    }

    /// Fetch token account data for all tracked tokens
    async fn fetch_token_accounts_impl(
        shared_state: &Arc<RwLock<ServiceState>>
    ) -> Result<usize, String> {
        // Get list of tracked tokens from shared state
        let tracked_tokens: Vec<String> = {
            let state = shared_state.read().await;
            state.tracked_tokens.keys().cloned().collect()
        };

        // Use the pool fetcher service to fetch token account data
        fetch_token_accounts_for_pool_service(&tracked_tokens).await
    }

    /// Calculate prices from account data
    async fn calculate_prices_impl(
        shared_state: &Arc<RwLock<ServiceState>>,
        price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) -> Result<usize, String> {
        // Calculate prices from account data and pool information

        let mut calculated_count = 0;
        let now = Utc::now();

        {
            let state = shared_state.read().await;
            let mut cache = price_cache.write().await;

            for (token_mint, pool_data) in &state.best_pools {
                if
                    let Some((account_data, fetch_time)) = state.account_data_cache.get(
                        &pool_data.pool_address
                    )
                {
                    let age = now.signed_duration_since(*fetch_time);
                    if age.num_seconds() < 60 {
                        let price_sol = if pool_data.reserve_token > 0.0 {
                            pool_data.reserve_sol / pool_data.reserve_token
                        } else {
                            0.0
                        };

                        let sol_usd_rate = 150.0;
                        let price_usd = price_sol * sol_usd_rate;

                        let price_info = TokenPriceInfo {
                            token_mint: token_mint.clone(),
                            pool_price_sol: Some(price_sol),
                            pool_price_usd: Some(price_usd),
                            api_price_sol: None,
                            api_price_usd: None,
                            pool_address: Some(pool_data.pool_address.clone()),
                            pool_type: Some(pool_data.dex_type.clone()),
                            reserve_sol: Some(pool_data.reserve_sol),
                            reserve_token: Some(pool_data.reserve_token),
                            liquidity_usd: Some(pool_data.liquidity_usd),
                            volume_24h_usd: Some(pool_data.volume_24h),
                            calculated_at: now,
                            error: None,
                        };

                        cache.insert(token_mint.clone(), price_info);
                        calculated_count += 1;
                    }
                }
            }
        }

        Ok(calculated_count)
    }

    /// Clean up stale data using the pool cleanup service
    async fn cleanup_data_impl(
        shared_state: &Arc<RwLock<ServiceState>>,
        price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) -> Result<usize, String> {
        // Use the pool cleanup service to clean up all data
        // We'll create a temporary CleanupServiceState and then apply the changes back
        let (cleaned_count, cleaned_state) = {
            let state = shared_state.read().await;
            let cleanup_state = CleanupServiceState {
                tracked_tokens: state.tracked_tokens.clone(),
                best_pools: state.best_pools.clone(),
                account_queue: state.account_queue.clone(),
                account_data_cache: state.account_data_cache.clone(),
            };

            cleanup_service_state(&Arc::new(RwLock::new(cleanup_state)), price_cache).await?
        };

        // Apply the cleaned state back to the original shared state
        {
            let mut state = shared_state.write().await;
            state.tracked_tokens = cleaned_state.tracked_tokens;
            state.best_pools = cleaned_state.best_pools;
            state.account_queue = cleaned_state.account_queue;
            state.account_data_cache = cleaned_state.account_data_cache;
        }

        Ok(cleaned_count)
    }

    /// Process pool calculations using the calculator service
    async fn process_pool_calculations_impl(
        shared_state: &Arc<RwLock<ServiceState>>,
        price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) -> Result<usize, String> {
        // TODO: Implement pool calculation processing
        // - Get tokens that need price calculations
        // - Use the pool calculator service to calculate prices
        // - Update price cache with calculated results
        // - Handle different pool types and programs

        let calculator = get_pool_calculator();
        let mut processed_count = 0;

        {
            let state = shared_state.read().await;
            let mut cache = price_cache.write().await;

            // Process tokens that have pool data but need price calculations
            for (token_mint, pool_data) in &state.best_pools {
                // Check if we already have fresh price data
                if let Some(existing_price) = cache.get(token_mint) {
                    let age = Utc::now().signed_duration_since(existing_price.calculated_at);
                    if age.num_seconds() < PRICE_CACHE_TTL_SECS {
                        continue; // Skip if we have fresh data
                    }
                }

                // Calculate price using the pool calculator
                let pool_info = pool_data.to_pool_info();
                match calculator.calculate_token_price(&pool_info, token_mint).await {
                    Ok(Some(price_info)) => {
                        // Convert PoolPriceInfo to TokenPriceInfo
                        let token_price_info = TokenPriceInfo {
                            token_mint: token_mint.clone(),
                            pool_price_sol: Some(price_info.price_sol),
                            pool_price_usd: None, // Will be calculated later
                            api_price_sol: None,
                            api_price_usd: None,
                            pool_address: Some(pool_data.pool_address.clone()),
                            pool_type: Some(pool_data.dex_type.clone()),
                            reserve_sol: Some(pool_data.reserve_sol),
                            reserve_token: Some(pool_data.reserve_token),
                            liquidity_usd: Some(pool_data.liquidity_usd),
                            volume_24h_usd: Some(pool_data.volume_24h),
                            calculated_at: Utc::now(),
                            error: None,
                        };

                        cache.insert(token_mint.clone(), token_price_info);
                        processed_count += 1;
                    }
                    Ok(None) => {
                        // Pool calculation returned None (no price available)
                        log(
                            LogTag::Pool,
                            "POOL_CALC_NO_PRICE",
                            &format!(
                                "No price available for {} in pool {}",
                                token_mint,
                                pool_data.pool_address
                            )
                        );
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "POOL_CALC_ERROR",
                            &format!("Failed to calculate price for {}: {}", token_mint, e)
                        );
                    }
                }
            }
        }

        Ok(processed_count)
    }

    /// Monitor all task states using the pool monitor service
    async fn monitor_states_impl(shared_state: &Arc<RwLock<ServiceState>>) -> Result<(), String> {
        // Convert ServiceState to MonitorServiceState for the monitor service
        let monitor_state = {
            let state = shared_state.read().await;
            MonitorServiceState {
                tracked_tokens: state.tracked_tokens.clone(),
                best_pools: state.best_pools.clone(),
                account_queue: state.account_queue.clone(),
                account_data_cache: state.account_data_cache.clone(),
                task_statuses: state.task_statuses.clone(),
            }
        };

        // Use the pool monitor service to monitor service health
        let _health_percentage = monitor_service_health(
            &Arc::new(RwLock::new(monitor_state))
        ).await?;

        Ok(())
    }

    /// Optimized pool data fetching with vault accounts (reduces RPC calls)
    pub async fn fetch_pools_optimized(
        &self,
        pool_addresses: &[String]
    ) -> Result<HashMap<String, TokenPriceInfo>, String> {
        if pool_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let start_time = Instant::now();

        log(
            LogTag::Pool,
            "OPTIMIZED_FETCH_START",
            &format!("🚀 Starting optimized fetch for {} pools", pool_addresses.len())
        );

        // Use the enhanced pool fetcher to get pools with vault data in fewer RPC calls
        let pool_fetcher = crate::pool_fetcher::get_pool_fetcher();
        let pools_with_vaults = pool_fetcher.fetch_pools_with_vaults(pool_addresses).await?;

        let mut result = HashMap::new();
        let decoder_factory = crate::pool_decoders::PoolDecoderFactory::new();

        // Process each pool with its vault data
        let pools_count = pools_with_vaults.len();
        for (pool_address, pool_with_vaults) in pools_with_vaults {
            if let Some(decoder) = decoder_factory.get_decoder(&pool_with_vaults.pool_type) {
                // Decode pool with vault data for accurate reserves
                match
                    decoder.decode_pool_data_with_vaults(
                        &pool_with_vaults.pool_data,
                        &pool_with_vaults.vault_data
                    )
                {
                    Ok(decoded_pool) => {
                        // Convert to PoolInfo for price calculation
                        let pool_info = crate::pool_calculator::PoolInfo {
                            pool_address: pool_address.clone(),
                            pool_program_id: crate::pool_decoders
                                ::get_program_id_from_pool_type(&pool_with_vaults.pool_type)
                                .to_string(),
                            pool_type: crate::pool_decoders
                                ::get_pool_type_display_name(&pool_with_vaults.pool_type)
                                .to_string(),
                            token_0_mint: decoded_pool.token_a_mint.to_string(),
                            token_1_mint: decoded_pool.token_b_mint.to_string(),
                            token_0_vault: None,
                            token_1_vault: None,
                            token_0_reserve: decoded_pool.token_a_reserve,
                            token_1_reserve: decoded_pool.token_b_reserve,
                            token_0_decimals: decoded_pool.token_a_decimals,
                            token_1_decimals: decoded_pool.token_b_decimals,
                            lp_mint: None,
                            lp_supply: None,
                            creator: None,
                            status: None,
                            liquidity_usd: None,
                            sqrt_price: None,
                        };

                        // Calculate price for both tokens in the pool
                        let calculator = crate::pool_calculator::get_pool_calculator();

                        // Try token_0
                        if
                            let Ok(Some(price_info)) = calculator.calculate_token_price(
                                &pool_info,
                                &pool_info.token_0_mint
                            ).await
                        {
                            let token_price_info = crate::pool_interface::TokenPriceInfo {
                                token_mint: pool_info.token_0_mint.clone(),
                                pool_price_sol: Some(price_info.price_sol),
                                pool_price_usd: None,
                                api_price_sol: None,
                                api_price_usd: None,
                                pool_address: Some(pool_address.clone()),
                                pool_type: Some(pool_info.pool_type.clone()),
                                reserve_sol: Some(pool_info.token_1_reserve as f64), // Assuming token_1 is SOL
                                reserve_token: Some(pool_info.token_0_reserve as f64),
                                liquidity_usd: None,
                                volume_24h_usd: None,
                                calculated_at: chrono::Utc::now(),
                                error: None,
                            };
                            result.insert(pool_info.token_0_mint.clone(), token_price_info);
                        }

                        // Try token_1
                        if
                            let Ok(Some(price_info)) = calculator.calculate_token_price(
                                &pool_info,
                                &pool_info.token_1_mint
                            ).await
                        {
                            let token_price_info = crate::pool_interface::TokenPriceInfo {
                                token_mint: pool_info.token_1_mint.clone(),
                                pool_price_sol: Some(price_info.price_sol),
                                pool_price_usd: None,
                                api_price_sol: None,
                                api_price_usd: None,
                                pool_address: Some(pool_address.clone()),
                                pool_type: Some(pool_info.pool_type.clone()),
                                reserve_sol: Some(pool_info.token_0_reserve as f64), // Assuming token_0 is SOL
                                reserve_token: Some(pool_info.token_1_reserve as f64),
                                liquidity_usd: None,
                                volume_24h_usd: None,
                                calculated_at: chrono::Utc::now(),
                                error: None,
                            };
                            result.insert(pool_info.token_1_mint.clone(), token_price_info);
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "DECODE_ERROR",
                            &format!("Failed to decode pool {}: {}", pool_address, e)
                        );
                    }
                }
            }
        }

        log(
            LogTag::Pool,
            "OPTIMIZED_FETCH_SUCCESS",
            &format!(
                "✅ Optimized fetch completed: {} pools processed, {} prices calculated in {:.2}ms",
                pools_count,
                result.len(),
                start_time.elapsed().as_millis()
            )
        );

        Ok(result)
    }

    /// Get all task statuses for monitoring
    pub async fn get_task_statuses(&self) -> HashMap<String, TaskStatus> {
        let state = self.shared_state.read().await;
        state.task_statuses.clone()
    }
}

// =============================================================================
// POOL INTERFACE IMPLEMENTATION
// =============================================================================

#[async_trait]
impl PoolInterface for PoolService {
    /// Get current price for a token
    async fn get_price(&self, token_address: &str) -> Option<TokenPriceInfo> {
        let cache = self.price_cache.read().await;
        let price_info = cache.get(token_address)?;

        // Check if price is still fresh
        let now = Utc::now();
        let age = now.signed_duration_since(price_info.calculated_at);
        if age.num_seconds() > PRICE_CACHE_TTL_SECS {
            // Price is stale, return None
            return None;
        }

        // Update cache hit stats
        {
            let mut stats = self.stats.write().await;
            stats.cache_hits += 1;
        }

        Some(price_info.clone())
    }

    /// Get price history for a token
    async fn get_price_history(&self, _token_address: &str) -> Vec<(DateTime<Utc>, f64)> {
        // TODO: Implement price history retrieval from database
        vec![]
    }

    /// Get list of tokens with available prices
    async fn get_available_tokens(&self) -> Vec<String> {
        let available = self.available_tokens.read().await;
        available.clone()
    }

    /// Get batch prices for multiple tokens
    async fn get_batch_prices(
        &self,
        token_addresses: &[String]
    ) -> HashMap<String, TokenPriceInfo> {
        let cache = self.price_cache.read().await;
        let mut result = HashMap::new();

        for token_address in token_addresses {
            if let Some(price_info) = cache.get(token_address) {
                // Check if price is still fresh
                let now = Utc::now();
                let age = now.signed_duration_since(price_info.calculated_at);
                if age.num_seconds() <= PRICE_CACHE_TTL_SECS {
                    result.insert(token_address.clone(), price_info.clone());
                }
            }
        }

        // Update cache hit stats
        {
            let mut stats = self.stats.write().await;
            stats.cache_hits += result.len() as u64;
        }

        result
    }
}

// =============================================================================
// GLOBAL INSTANCE
// =============================================================================

use std::sync::OnceLock;

static POOL_SERVICE: OnceLock<PoolService> = OnceLock::new();

/// Initialize the global pool service instance
pub fn init_pool_service() -> &'static PoolService {
    POOL_SERVICE.get_or_init(|| {
        log(LogTag::Pool, "INIT", "🏗️ Initializing Pool Service");
        PoolService::new()
    })
}

/// Get the global pool service instance
pub fn get_pool_service() -> &'static PoolService {
    POOL_SERVICE.get().expect("Pool service not initialized")
}
