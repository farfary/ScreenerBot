//! Runtime initialization phases shared by boot and live setup completion.

use crate::{
    errors::StartupError,
    logger::{self, LogTag},
};

/// Initialize persistence used directly by dashboard routes.
///
/// These stores do not require a wallet or Solana RPC and therefore exist in
/// initialization, Explore Mode, and full boot states. Keeping them outside the service
/// tier prevents globally available dashboard surfaces from depending on the
/// wallet setup branch that happened to start the process.
pub(super) async fn initialize_dashboard_persistence() -> Result<(), StartupError> {
    crate::actions::init_database().await.map_err(|e| {
        StartupError::generic(format!("Failed to initialize actions database: {e}"))
    })?;
    logger::info(LogTag::System, "Actions database initialized successfully");

    crate::actions::sync_from_db()
        .await
        .map_err(|e| StartupError::generic(format!("Failed to sync actions from database: {e}")))?;
    crate::actions::spawn_cleanup_task();

    // Maintenance covers every database that currently exists and is safe in
    // all boot modes. Database-specific initialization still owns its schema.
    tokio::spawn(crate::database::start_db_maintenance_task());

    // Analysis instructions and Assistant chat history are dashboard
    // persistence, not model execution. Their databases exist even when LLM
    // features are off.
    if let Err(e) = crate::llm_analysis::init_analysis_database() {
        logger::warning(
            LogTag::System,
            &format!(
                "Failed to initialize analysis database: {e} - analysis instructions and history will not be available"
            ),
        );
    }

    if let Err(e) = crate::assistant::init_chat_db() {
        logger::warning(
            LogTag::System,
            &format!(
                "Failed to initialize Assistant chat database: {e} - chat history will not be available"
            ),
        );
    }

    // Agent-control pairing/approval/audit store. Dashboard persistence, not LLM
    // execution: the management API and the live-app bridge must work in every
    // boot state that serves the webserver. A failure here leaves the bridge
    // fail-closed (no pairings resolvable) rather than aborting boot.
    if let Err(e) = crate::agent_control::init_store() {
        logger::warning(
            LogTag::System,
            &format!(
                "Failed to initialize agent-control store: {e} - agent pairing and the MCP bridge will be unavailable"
            ),
        );
    }

    // Strategy authoring and persistence are dashboard features. Evaluation is
    // consumed only by the full-mode trader, but the editor, templates, and
    // configuration APIs must remain usable in Explore Mode.
    crate::strategies::init_strategy_system(crate::strategies::engine::EngineConfig::default())
        .await
        .map_err(|e| StartupError::generic(format!("Failed to initialize strategy system: {e}")))?;
    logger::info(LogTag::System, "Strategy system initialized successfully");

    Ok(())
}

/// Initialize model-backed execution when configured.
///
/// LLM providers, analysis, and Assistant chat do not require a wallet or
/// Solana RPC, so this phase
/// is valid in Explore Mode. The wallet/position-dependent LLM-analysis background
/// services remain gated separately on full initialization.
pub(crate) async fn initialize_model_features_if_enabled() -> Result<(), StartupError> {
    if !crate::config::with_config(|cfg| cfg.llm.enabled) {
        return Ok(());
    }

    if crate::llm_analysis::try_get_analysis_engine().is_none() {
        logger::info(LogTag::System, "Initializing analysis engine...");
        crate::llm_analysis::init_analysis_engine()
            .await
            .map_err(|e| {
                StartupError::generic(format!("Failed to initialize analysis engine: {e}"))
            })?;
        logger::info(LogTag::System, "Analysis engine initialized successfully");
    }

    if crate::assistant::try_get_chat_engine().is_none() {
        crate::assistant::init_chat_engine().await.map_err(|e| {
            StartupError::generic(format!("Failed to initialize Assistant chat engine: {e}"))
        })?;
        logger::info(
            LogTag::System,
            "Assistant chat engine initialized successfully",
        );
    }

    if crate::apis::llm::try_get_llm_manager().is_none() {
        crate::apis::llm::init::init_providers_from_config()
            .await
            .map_err(|e| StartupError::generic(e.to_string()))?;
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
        .map_err(|e| StartupError::generic(format!("Failed to initialize wallets: {e}")))?;
    logger::info(LogTag::System, "Wallets module initialized");

    logger::info(LogTag::System, "Validating wallet consistency...");
    match crate::wallet_validation::WalletValidator::validate_wallet_consistency()
        .await
        .map_err(|e| StartupError::generic(format!("Failed to validate wallet consistency: {e}")))?
    {
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

    initialize_model_features_if_enabled().await
}
