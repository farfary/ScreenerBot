//! Helper functions for the main run loop — service registration, signal handling, and LLM init.

use crate::{
    global,
    logger::{self, LogTag},
    services::ServiceManager,
};

/// Register all available services
pub(crate) fn register_all_services(manager: &mut ServiceManager) {
    use crate::services::implementations::*;

    logger::info(LogTag::System, "Registering services...");

    // Core infrastructure services
    manager.register(Box::new(ConnectivityService::new()));
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
    manager.register(Box::new(TraderService::new()));
    manager.register(Box::new(WebserverService));

    // AI service (background auto-blacklisting)
    manager.register(Box::new(AiService::default()));

    // Telegram service (notifications + commands + discovery)
    manager.register(Box::new(TelegramService::new()));

    // Background utility services
    manager.register(Box::new(UpdateCheckService));

    let service_count = 21; // connectivity, events, transactions, sol_price, pool_discovery, pool_fetcher,
                            // pool_calculator, pool_analyzer, pools, tokens, filtering, ohlcv,
                            // positions, wallet, rpc_stats, ata_cleanup, trader, webserver, ai, telegram, update_check
    logger::info(
        LogTag::System,
        &format!("All services registered ({service_count} total)"),
    );
}

/// Wait for shutdown signal (Ctrl+C, SIGTERM, SIGQUIT on Unix)
///
/// NOTE: SIGHUP is intentionally NOT handled. SIGHUP is sent when a terminal
/// disconnects (e.g., SSH session closes, nohup usage). A headless trading bot
/// must survive terminal disconnects. Use SIGTERM or Ctrl+C to stop the bot.
pub(crate) async fn wait_for_shutdown_signal() -> Result<(), String> {
    logger::info(
        LogTag::System,
        "Waiting for shutdown signal (press Ctrl+C twice to force kill)",
    );

    // Platform-specific signal handling
    #[cfg(unix)]
    let signal_name = {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigint =
            signal(SignalKind::interrupt()).map_err(|e| format!("Failed to bind SIGINT: {e}"))?;
        let mut sigterm =
            signal(SignalKind::terminate()).map_err(|e| format!("Failed to bind SIGTERM: {e}"))?;
        let mut sigquit =
            signal(SignalKind::quit()).map_err(|e| format!("Failed to bind SIGQUIT: {e}"))?;

        tokio::select! {
            _ = sigint.recv() => "SIGINT",
            _ = sigterm.recv() => "SIGTERM",
            _ = sigquit.recv() => "SIGQUIT",
        }
    };

    #[cfg(windows)]
    let signal_name = {
        // On Windows, ctrl_c() handles Ctrl+C and Ctrl+Break
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| format!("Failed to listen for shutdown signal: {e}"))?;
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
pub(crate) async fn wait_for_initialization_or_shutdown() -> Result<(), String> {
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
            return Err("Shutdown during initialization".to_owned());
          }
          _ = sleep(Duration::from_millis(500)) => {
            // Continue polling
          }
        }
    }
}

/// Initialize LLM providers based on configuration
pub(crate) async fn initialize_llm_providers() -> Result<(), String> {
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
                        &format!("Failed to initialize OpenRouter: {e}"),
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
                    logger::warning(LogTag::System, &format!("Failed to initialize OpenAI: {e}"));
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
                        &format!("Failed to initialize Anthropic: {e}"),
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
                    logger::warning(LogTag::System, &format!("Failed to initialize Groq: {e}"));
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
                        &format!("Failed to initialize DeepSeek: {e}"),
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
                    logger::warning(LogTag::System, &format!("Failed to initialize Gemini: {e}"));
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
                    logger::warning(LogTag::System, &format!("Failed to initialize Ollama: {e}"));
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
                        &format!("Failed to initialize Together: {e}"),
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
                        &format!("Failed to initialize Mistral: {e}"),
                    );
                }
            }
        }

        // Assistant (no API key needed, uses OAuth tokens)
        if cfg.ai.providers.Assistant.enabled {
            use crate::apis::llm::Assistant::AssistantClient;
            let model = get_model(&cfg.ai.providers.Assistant.model);
            let client = AssistantClient::new(model, cfg.ai.providers.Assistant.enabled);
            llm_manager.set_Assistant(std::sync::Arc::new(client));
            if AssistantClient::is_authenticated() {
                enabled_providers.push("Assistant (authenticated)");
            } else {
                enabled_providers.push("Assistant (not authenticated)");
            }
        }
    });

    init_llm_manager(llm_manager)
        .await
        .map_err(|e| format!("Failed to initialize LLM manager: {e}"))?;

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
