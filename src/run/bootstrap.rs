//! Runtime initialization phases shared by boot and live setup completion.

use crate::{
    errors::StartupError,
    logger::{self, LogTag},
};

/// Initialize persistence used directly by dashboard routes.
///
/// These stores do not require a wallet or Solana RPC and therefore exist in
/// initialization, preview, and full modes. Keeping them outside the service
/// tier prevents globally available dashboard surfaces from depending on the
/// wallet setup branch that happened to start the process.
pub(super) async fn initialize_dashboard_persistence() -> Result<(), StartupError> {
    crate::actions::init_database()
        .await
        .map_err(|e| format!("Failed to initialize actions database: {e}"))?;
    logger::info(LogTag::System, "Actions database initialized successfully");

    crate::actions::sync_from_db()
        .await
        .map_err(|e| format!("Failed to sync actions from database: {e}"))?;
    crate::actions::spawn_cleanup_task();

    // Maintenance covers every database that currently exists and is safe in
    // all boot modes. Database-specific initialization still owns its schema.
    tokio::spawn(crate::database::start_db_maintenance_task());

    // AI instructions and chat history are dashboard persistence, not AI
    // execution. The databases must be available even when AI itself is off.
    if let Err(e) = crate::ai::init_ai_database() {
        logger::warning(
            LogTag::System,
            &format!(
                "Failed to initialize AI database: {e} - AI instructions and history will not be available"
            ),
        );
    }

    if let Err(e) = crate::ai::init_chat_db() {
        logger::warning(
            LogTag::System,
            &format!(
                "Failed to initialize AI chat database: {e} - Chat history will not be available"
            ),
        );
    }

    // Strategy authoring and persistence are dashboard features. Evaluation is
    // consumed only by the full-mode trader, but the editor, templates, and
    // configuration APIs must remain usable in preview mode.
    crate::strategies::init_strategy_system(crate::strategies::engine::EngineConfig::default())
        .await
        .map_err(|e| format!("Failed to initialize strategy system: {e}"))?;
    logger::info(LogTag::System, "Strategy system initialized successfully");

    Ok(())
}

/// Initialize AI execution when configured.
///
/// AI providers and chat do not require a wallet or Solana RPC, so this phase
/// is valid in preview mode. The wallet/position-dependent AI background
/// services remain gated separately on full initialization.
pub(crate) async fn initialize_ai_runtime_if_enabled() -> Result<(), StartupError> {
    if !crate::config::with_config(|cfg| cfg.ai.enabled) {
        return Ok(());
    }

    if crate::ai::try_get_ai_engine().is_none() {
        logger::info(LogTag::System, "Initializing AI engine...");
        crate::ai::init_ai_engine()
            .await
            .map_err(|e| format!("Failed to initialize AI engine: {e}"))?;
        logger::info(LogTag::System, "AI engine initialized successfully");
    }

    if crate::ai::try_get_chat_engine().is_none() {
        crate::ai::init_chat_engine()
            .await
            .map_err(|e| format!("Failed to initialize AI chat engine: {e}"))?;
        logger::info(LogTag::System, "AI chat engine initialized successfully");
    }

    if crate::apis::llm::try_get_llm_manager().is_none() {
        super::llm_init::initialize_llm_providers().await?;
    }

    Ok(())
}

/// Initialize state required only by wallet/RPC-backed full mode.
///
/// Setup completion always restarts the process, so this is the one canonical
/// activation path for both first-time setup and every later full-mode boot.
pub(crate) async fn initialize_full_runtime() -> Result<(), StartupError> {
    crate::wallets::initialize()
        .await
        .map_err(|e| format!("Failed to initialize wallets: {e}"))?;
    logger::info(LogTag::System, "Wallets module initialized");

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
            return Err(StartupError::wallet_mismatch(
                &current,
                &stored,
                &affected_systems,
            ));
        }
    }

    initialize_ai_runtime_if_enabled().await
}
