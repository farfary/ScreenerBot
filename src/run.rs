// New simplified run implementation using ServiceManager

use crate::{
    global,
    logger::{self, LogTag},
    process_lock::ProcessLock,
    profiling,
    services::ServiceManager,
};
use solana_sdk::signature::Signer;

/// Main bot execution function - handles the full bot lifecycle with ServiceManager
///
/// Acquires process lock and runs the bot. For GUI mode, use `run_bot_with_lock()` instead.
pub async fn run_bot() -> Result<(), String> {
    // 0. Initialize profiling if requested (must be done before any tokio tasks)
    profiling::init_profiling();

    // 1. Ensure all required directories exist (safety backup, already done in main.rs)
    crate::paths::ensure_all_directories()
        .map_err(|e| format!("Failed to create required directories: {}", e))?;

    // 2. Acquire process lock to prevent multiple instances
    let process_lock = ProcessLock::acquire()?;

    // Run bot with the acquired lock
    run_bot_internal(process_lock).await
}

/// Run bot with a pre-acquired process lock
///
/// Used by Electron GUI mode which acquires the lock before starting to ensure
/// the window doesn't open if another instance is running.
pub async fn run_bot_with_lock(process_lock: ProcessLock) -> Result<(), String> {
    // 0. Initialize profiling if requested (must be done before any tokio tasks)
    profiling::init_profiling();

    // 1. Ensure all required directories exist (safety backup, already done in main.rs)
    crate::paths::ensure_all_directories()
        .map_err(|e| format!("Failed to create required directories: {}", e))?;

    // Lock already acquired, run bot directly
    run_bot_internal(process_lock).await
}

/// Internal bot execution with pre-acquired lock
async fn run_bot_internal(_process_lock: ProcessLock) -> Result<(), String> {
    logger::info(LogTag::System, "ScreenerBot starting up...");

    // 1. Set GUI mode if --gui flag is present (must be done early for webserver security)
    if crate::arguments::is_gui_enabled() {
        global::set_gui_mode(true);
        logger::info(LogTag::System, "GUI mode enabled");
    }

    // 2. Validate CLI arguments early (before any processing)
    if let Err(e) = crate::arguments::validate_port_argument() {
        logger::error(
            LogTag::System,
            &format!("Argument validation failed: {}", e),
        );
        return Err(e);
    }

    if let Err(e) = crate::arguments::validate_host_argument() {
        logger::error(
            LogTag::System,
            &format!("Argument validation failed: {}", e),
        );
        return Err(e);
    }

    // 3. Log CLI overrides (if provided)
    if let Some(port) = crate::arguments::get_port_override() {
        if crate::arguments::is_privileged_port(port) {
            logger::warning(
                LogTag::System,
                &format!(
                    "Port {} requires elevated privileges (root/Administrator)",
                    port
                ),
            );
        }

        logger::info(
            LogTag::System,
            &format!("CLI override: Using port {}", port),
        );
    }

    if let Some(host) = crate::arguments::get_host_override() {
        logger::info(
            LogTag::System,
            &format!("CLI override: Using host {}", host),
        );

        if host == "0.0.0.0" {
            logger::warning(
                LogTag::System,
                "Binding to 0.0.0.0 allows remote access - ensure firewall is configured",
            );
        }
    }

    if crate::arguments::get_port_override().is_none()
        && crate::arguments::get_host_override().is_none()
    {
        logger::debug(
            LogTag::System,
            "No webserver CLI overrides provided, using config/defaults",
        );
    }

    // 4. Check if config.toml exists (determines initialization mode)
    let config_path = crate::paths::get_config_path();
    let config_exists = config_path.exists();

    if !config_exists {
        logger::info(
            LogTag::System,
            "No config.toml found - starting in initialization mode",
        );
        logger::info(
            LogTag::System,
            "Webserver will start on http://localhost:8080 for initial setup",
        );

        // Set initialization flag to false (services will be gated)
        global::INITIALIZATION_COMPLETE.store(false, std::sync::atomic::Ordering::SeqCst);

        // Create service manager with only webserver enabled
        let mut service_manager = ServiceManager::new().await?;
        logger::info(LogTag::System, "Service manager initialized");

        // Register all services (but only webserver will be enabled)
        register_all_services(&mut service_manager);

        // Initialize global ServiceManager for webserver access
        crate::services::init_global_service_manager(service_manager).await;

        // Get mutable reference to continue
        let manager_ref = crate::services::get_service_manager()
            .await
            .ok_or("Failed to get ServiceManager reference")?;

        let mut service_manager = {
            let mut guard = manager_ref.write().await;
            guard.take().ok_or("ServiceManager was already taken")?
        };

        // Start only enabled services (webserver only in pre-init mode)
        service_manager.start_all().await?;

        // Put it back for webserver access
        {
            let mut guard = manager_ref.write().await;
            *guard = Some(service_manager);
        }

        logger::info(
            LogTag::System,
            "Webserver started - complete initialization at http://localhost:8080",
        );
        logger::info(LogTag::System, "Waiting for initialization to complete...");

        // Wait for initialization to complete or shutdown signal
        wait_for_initialization_or_shutdown().await?;

        logger::info(
            LogTag::System,
            "Initialization complete - all services running",
        );
    } else {
        logger::info(
            LogTag::System,
            "Config.toml found - starting in normal mode",
        );

        // 4. Load configuration (if not already loaded by main.rs)
        if !crate::config::is_config_initialized() {
            crate::config::load_config().map_err(|e| format!("Failed to load config: {}", e))?;
            logger::info(LogTag::System, "Configuration loaded successfully");
        }

        // 5. Initialize wallets module (migrates from config.toml if needed)
        crate::wallets::initialize()
            .await
            .map_err(|e| format!("Failed to initialize wallets: {}", e))?;

        logger::info(LogTag::System, "Wallets module initialized");

        // 6. Validate wallet consistency
        logger::info(LogTag::System, "Validating wallet consistency...");

        match crate::wallet_validation::WalletValidator::validate_wallet_consistency().await? {
            crate::wallet_validation::WalletValidationResult::Valid => {
                logger::info(LogTag::System, "Wallet validation passed");
            }
            crate::wallet_validation::WalletValidationResult::FirstRun => {
                logger::info(LogTag::System, "First run - no existing data");
            }
            crate::wallet_validation::WalletValidationResult::Mismatch {
                current,
                stored,
                affected_systems,
            } => {
                logger::error(
                    LogTag::System,
                    &format!(
                        "WALLET MISMATCH DETECTED!\n\
             \n\
             Current wallet: {}\n\
             Stored wallet: {}\n\
             Affected systems: {}\n\
             \n\
              You MUST clean existing data before starting with a new wallet.\n\
             Run: cargo run --bin screenerbot -- --clean-wallet-data\n\
             Or manually delete: data/transactions.db data/positions.db data/wallet.db",
                        current,
                        stored,
                        affected_systems.join(", ")
                    ),
                );

                return Err(format!(
          "Wallet mismatch detected - current wallet {} does not match stored wallet {}. Clean data before proceeding.",
          current, stored
        ));
            }
        }

        // Set initialization flag to true (all services enabled)
        global::INITIALIZATION_COMPLETE.store(true, std::sync::atomic::Ordering::SeqCst);

        // 7. Initialize strategy system
        crate::strategies::init_strategy_system(crate::strategies::engine::EngineConfig::default())
            .await
            .map_err(|e| format!("Failed to initialize strategy system: {}", e))?;

        logger::info(LogTag::System, "Strategy system initialized successfully");

        // 8. Initialize actions database
        crate::actions::init_database()
            .await
            .map_err(|e| format!("Failed to initialize actions database: {}", e))?;

        logger::info(LogTag::System, "Actions database initialized successfully");

        // Sync recent incomplete actions from database to memory
        crate::actions::sync_from_db()
            .await
            .map_err(|e| format!("Failed to sync actions from database: {}", e))?;

        // 8.5. Initialize AI database (always) and AI engine (if enabled)
        // Database is always initialized so dashboard can view/edit instructions
        if let Err(e) = crate::ai::init_ai_database() {
            logger::warning(
                LogTag::System,
                &format!(
                    "Failed to initialize AI database: {} - AI instructions and history will not be available",
                    e
                ),
            );
        }

        // Initialize AI engine only if enabled
        let ai_enabled = crate::config::with_config(|cfg| cfg.ai.enabled);
        if ai_enabled {
            logger::info(LogTag::System, "Initializing AI engine...");

            crate::ai::init_ai_engine()
                .await
                .map_err(|e| format!("Failed to initialize AI engine: {}", e))?;
            logger::info(LogTag::System, "AI engine initialized successfully");

            // Initialize LLM manager with configured providers
            initialize_llm_providers().await?;
        }

        // 9. Create service manager
        let mut service_manager = ServiceManager::new().await?;

        logger::info(LogTag::System, "Service manager initialized");

        // 10. Register all services
        register_all_services(&mut service_manager);

        // 11. Initialize global ServiceManager for webserver access
        crate::services::init_global_service_manager(service_manager).await;

        // 12. Get mutable reference to continue
        let manager_ref = crate::services::get_service_manager()
            .await
            .ok_or("Failed to get ServiceManager reference")?;

        let mut service_manager = {
            let mut guard = manager_ref.write().await;
            guard.take().ok_or("ServiceManager was already taken")?
        };

        // 13. Start all enabled services
        service_manager.start_all().await?;

        // 14. Put it back for webserver access
        {
            let mut guard = manager_ref.write().await;
            *guard = Some(service_manager);
        }

        logger::info(
            LogTag::System,
            "All services started - ScreenerBot is running",
        );
    }

    // 15. Wait for shutdown signal
    wait_for_shutdown_signal().await?;

    // 16. Stop all services gracefully
    logger::info(LogTag::System, "Initiating graceful shutdown...");

    let manager_ref = crate::services::get_service_manager()
        .await
        .ok_or("Failed to get ServiceManager reference for shutdown")?;

    let mut service_manager = {
        let mut guard = manager_ref.write().await;
        guard
            .take()
            .ok_or("ServiceManager was already taken during shutdown")?
    };

    service_manager.stop_all().await?;

    logger::info(LogTag::System, "ScreenerBot shut down successfully");

    Ok(())
}

/// Register all available services
fn register_all_services(manager: &mut ServiceManager) {
    use crate::services::implementations::*;

    logger::info(LogTag::System, "Registering services...");

    // Core infrastructure services
    manager.register(Box::new(crate::connectivity::ConnectivityService::new()));
    manager.register(Box::new(EventsService));
    manager.register(Box::new(TransactionsService));
    manager.register(Box::new(SolPriceService));

    // Pool services (4 sub-services + 1 helper coordinator)
    manager.register(Box::new(PoolDiscoveryService));
    manager.register(Box::new(PoolFetcherService));
    manager.register(Box::new(PoolCalculatorService));
    manager.register(Box::new(PoolAnalyzerService));
    manager.register(Box::new(PoolsService));

    // Centralized Tokens service
    manager.register(Box::new(TokensService::default()));

    // Application services
    manager.register(Box::new(FilteringService::new()));
    manager.register(Box::new(OhlcvService));
    manager.register(Box::new(PositionsService));
    manager.register(Box::new(WalletService));
    manager.register(Box::new(RpcStatsService));
    manager.register(Box::new(AtaCleanupService));
    manager.register(Box::new(crate::trader::TraderService::new()));
    manager.register(Box::new(WebserverService));

    // AI service (background auto-blacklisting)
    manager.register(Box::new(AiService::default()));

    // Telegram service (notifications + commands + discovery)
    manager.register(Box::new(crate::telegram::TelegramService::new()));

    // Background utility services
    manager.register(Box::new(UpdateCheckService));

    let service_count = 22; // connectivity, events, transactions, sol_price, pool_discovery, pool_fetcher,
                            // pool_calculator, pool_analyzer, pools, tokens, filtering, ohlcv,
                            // positions, wallet, rpc_stats, ata_cleanup, trader, webserver, ai, telegram, update_check
    logger::info(
        LogTag::System,
        &format!("All services registered ({} total)", service_count),
    );
}

/// Wait for shutdown signal (Ctrl+C, SIGTERM, SIGHUP, SIGQUIT on Unix)
async fn wait_for_shutdown_signal() -> Result<(), String> {
    logger::info(
        LogTag::System,
        "Waiting for shutdown signal (press Ctrl+C twice to force kill)",
    );

    // Platform-specific signal handling
    #[cfg(unix)]
    let signal_name = {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigint =
            signal(SignalKind::interrupt()).map_err(|e| format!("Failed to bind SIGINT: {}", e))?;
        let mut sigterm = signal(SignalKind::terminate())
            .map_err(|e| format!("Failed to bind SIGTERM: {}", e))?;
        let mut sighup =
            signal(SignalKind::hangup()).map_err(|e| format!("Failed to bind SIGHUP: {}", e))?;
        let mut sigquit =
            signal(SignalKind::quit()).map_err(|e| format!("Failed to bind SIGQUIT: {}", e))?;

        tokio::select! {
            _ = sigint.recv() => "SIGINT",
            _ = sigterm.recv() => "SIGTERM",
            _ = sighup.recv() => "SIGHUP",
            _ = sigquit.recv() => "SIGQUIT",
        }
    };

    #[cfg(windows)]
    let signal_name = {
        // On Windows, ctrl_c() handles Ctrl+C and Ctrl+Break
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| format!("Failed to listen for shutdown signal: {}", e))?;
        "CTRL_C"
    };

    logger::warning(
        LogTag::System,
        &format!(
            "Shutdown signal received ({}). Press Ctrl+C again to force kill.",
            signal_name
        ),
    );

    // Spawn a background listener for a second Ctrl+C to exit immediately
    tokio::spawn(async move {
        // If another Ctrl+C is received during graceful shutdown, exit immediately
        if tokio::signal::ctrl_c().await.is_ok() {
            logger::error(
                LogTag::System,
                "Second Ctrl+C detected — forcing immediate exit.",
            );
            // 130 is the conventional exit code for SIGINT
            std::process::exit(130);
        }
    });

    Ok(())
}

/// Wait for initialization to complete or shutdown signal during pre-init mode
async fn wait_for_initialization_or_shutdown() -> Result<(), String> {
    use tokio::time::{sleep, Duration, Instant};

    const MAX_WAIT_DURATION: Duration = Duration::from_secs(30 * 60); // 30 minutes
    const WARNING_INTERVAL: Duration = Duration::from_secs(5 * 60); // Warn every 5 minutes

    let start = Instant::now();
    let mut last_warning = start;

    loop {
        // Check if initialization is complete
        if global::is_initialization_complete() {
            logger::info(
                LogTag::System,
                "Initialization complete - services started successfully",
            );
            return Ok(());
        }

        // Check elapsed time
        let elapsed = start.elapsed();
        if elapsed >= MAX_WAIT_DURATION {
            logger::error(
                LogTag::System,
                &format!(
                    "Initialization timeout after {} minutes - initialization never completed",
                    MAX_WAIT_DURATION.as_secs() / 60
                ),
            );
            return Err(format!(
                "Initialization timeout after {} minutes",
                MAX_WAIT_DURATION.as_secs() / 60
            ));
        }

        // Periodic warning logs
        if elapsed - (last_warning - start) >= WARNING_INTERVAL {
            logger::warning(
                LogTag::System,
                &format!(
                    "Still waiting for initialization... ({} minutes elapsed)",
                    elapsed.as_secs() / 60
                ),
            );
            last_warning = Instant::now();
        }

        // Check for Ctrl+C (non-blocking)
        tokio::select! {
          _ = tokio::signal::ctrl_c() => {
            logger::warning(
              LogTag::System,
              "Shutdown signal received during initialization",
            );
            return Err("Shutdown during initialization".to_string());
          }
          _ = sleep(Duration::from_millis(500)) => {
            // Continue polling
          }
        }
    }
}

/// Initialize LLM providers based on configuration
async fn initialize_llm_providers() -> Result<(), String> {
    use crate::apis::llm::{init_llm_manager, LlmManager};
    use crate::config::with_config;

    let mut llm_manager = LlmManager::new();
    let mut enabled_providers = Vec::new();

    // Helper to get model option
    let get_model = |model_str: &str| -> Option<String> {
        if model_str.is_empty() || model_str == "auto" {
            None
        } else {
            Some(model_str.to_string())
        }
    };

    with_config(|cfg| {
        // OpenRouter (has extra parameters for site_url and site_name)
        if cfg.ai.providers.openrouter.enabled && !cfg.ai.providers.openrouter.api_key.is_empty() {
            use crate::apis::llm::openrouter::OpenRouterClient;
            let model = get_model(&cfg.ai.providers.openrouter.model);
            match OpenRouterClient::new(
                cfg.ai.providers.openrouter.api_key.clone(),
                model,
                cfg.ai.providers.openrouter.enabled,
                None, // site_url - would need to be added to config
                None, // site_name - would need to be added to config
            ) {
                Ok(client) => {
                    llm_manager.set_openrouter(std::sync::Arc::new(client));
                    enabled_providers.push("OpenRouter");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize OpenRouter: {}", e),
                    );
                }
            }
        }

        // OpenAI
        if cfg.ai.providers.openai.enabled && !cfg.ai.providers.openai.api_key.is_empty() {
            use crate::apis::llm::openai::OpenAiClient;
            let model = get_model(&cfg.ai.providers.openai.model);
            match OpenAiClient::new(
                cfg.ai.providers.openai.api_key.clone(),
                model,
                cfg.ai.providers.openai.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_openai(std::sync::Arc::new(client));
                    enabled_providers.push("OpenAI");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize OpenAI: {}", e),
                    );
                }
            }
        }

        // Anthropic
        if cfg.ai.providers.anthropic.enabled && !cfg.ai.providers.anthropic.api_key.is_empty() {
            use crate::apis::llm::anthropic::AnthropicClient;
            let model = get_model(&cfg.ai.providers.anthropic.model);
            match AnthropicClient::new(
                cfg.ai.providers.anthropic.api_key.clone(),
                model,
                cfg.ai.providers.anthropic.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_anthropic(std::sync::Arc::new(client));
                    enabled_providers.push("Anthropic");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize Anthropic: {}", e),
                    );
                }
            }
        }

        // Groq
        if cfg.ai.providers.groq.enabled && !cfg.ai.providers.groq.api_key.is_empty() {
            use crate::apis::llm::groq::GroqClient;
            let model = get_model(&cfg.ai.providers.groq.model);
            match GroqClient::new(
                cfg.ai.providers.groq.api_key.clone(),
                model,
                cfg.ai.providers.groq.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_groq(std::sync::Arc::new(client));
                    enabled_providers.push("Groq");
                }
                Err(e) => {
                    logger::warning(LogTag::System, &format!("Failed to initialize Groq: {}", e));
                }
            }
        }

        // DeepSeek
        if cfg.ai.providers.deepseek.enabled && !cfg.ai.providers.deepseek.api_key.is_empty() {
            use crate::apis::llm::deepseek::DeepSeekClient;
            let model = get_model(&cfg.ai.providers.deepseek.model);
            match DeepSeekClient::new(
                cfg.ai.providers.deepseek.api_key.clone(),
                model,
                cfg.ai.providers.deepseek.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_deepseek(std::sync::Arc::new(client));
                    enabled_providers.push("DeepSeek");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize DeepSeek: {}", e),
                    );
                }
            }
        }

        // Gemini
        if cfg.ai.providers.gemini.enabled && !cfg.ai.providers.gemini.api_key.is_empty() {
            use crate::apis::llm::gemini::GeminiClient;
            let model = get_model(&cfg.ai.providers.gemini.model);
            match GeminiClient::new(
                cfg.ai.providers.gemini.api_key.clone(),
                model,
                cfg.ai.providers.gemini.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_gemini(std::sync::Arc::new(client));
                    enabled_providers.push("Gemini");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize Gemini: {}", e),
                    );
                }
            }
        }

        // Ollama (no API key, uses base_url instead)
        if cfg.ai.providers.ollama.enabled {
            use crate::apis::llm::ollama::OllamaClient;
            let base_url = if !cfg.ai.providers.ollama.base_url.is_empty() {
                Some(cfg.ai.providers.ollama.base_url.clone())
            } else {
                None
            };
            let model = get_model(&cfg.ai.providers.ollama.model);
            match OllamaClient::new(base_url, model, cfg.ai.providers.ollama.enabled) {
                Ok(client) => {
                    llm_manager.set_ollama(std::sync::Arc::new(client));
                    enabled_providers.push("Ollama");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize Ollama: {}", e),
                    );
                }
            }
        }

        // Together
        if cfg.ai.providers.together.enabled && !cfg.ai.providers.together.api_key.is_empty() {
            use crate::apis::llm::together::TogetherClient;
            let model = get_model(&cfg.ai.providers.together.model);
            match TogetherClient::new(
                cfg.ai.providers.together.api_key.clone(),
                model,
                cfg.ai.providers.together.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_together(std::sync::Arc::new(client));
                    enabled_providers.push("Together");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize Together: {}", e),
                    );
                }
            }
        }

        // Mistral
        if cfg.ai.providers.mistral.enabled && !cfg.ai.providers.mistral.api_key.is_empty() {
            use crate::apis::llm::mistral::MistralClient;
            let model = get_model(&cfg.ai.providers.mistral.model);
            match MistralClient::new(
                cfg.ai.providers.mistral.api_key.clone(),
                model,
                cfg.ai.providers.mistral.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_mistral(std::sync::Arc::new(client));
                    enabled_providers.push("Mistral");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize Mistral: {}", e),
                    );
                }
            }
        }
    });

    init_llm_manager(llm_manager)
        .await
        .map_err(|e| format!("Failed to initialize LLM manager: {}", e))?;

    if enabled_providers.is_empty() {
        logger::info(
            LogTag::System,
            "LLM manager initialized (no providers enabled)",
        );
    } else {
        logger::info(
            LogTag::System,
            &format!(
                "LLM manager initialized with {} provider(s): {}",
                enabled_providers.len(),
                enabled_providers.join(", ")
            ),
        );
    }

    Ok(())
}
