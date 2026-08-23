//! Action tracking for manual trading operations (buy, sell, DCA/add)

use crate::actions::{
    complete_action_failed, complete_action_success, register_action, update_step, Action,
    ActionType, StepStatus,
};
use crate::trader::error::Error;
use serde_json::{json, Value};
use uuid::Uuid;

use super::{STEP_QUOTE, STEP_SWAP, STEP_VALIDATE, STEP_VERIFY};

/// Steps for manual buy action
const BUY_STEPS: &[&str] = &["Validating", "Getting Quote", "Executing Swap", "Verifying"];

/// Steps for manual sell action
const SELL_STEPS: &[&str] = &["Validating", "Getting Quote", "Executing Swap", "Verifying"];

/// Steps for manual DCA/add action
const ADD_STEPS: &[&str] = &["Validating", "Getting Quote", "Executing Swap", "Verifying"];

/// Action tracker for manual buy operation
pub struct ManualBuyAction {
    pub action_id: String,
}

impl ManualBuyAction {
    /// Create and register a new manual buy action
    pub async fn new(mint: &str, symbol: Option<&str>, size_sol: f64) -> Result<Self, Error> {
        let action_id = Uuid::new_v4().to_string();

        let metadata = json!({
            "mint": mint,
            "symbol": symbol.unwrap_or("Unknown"),
            "size_sol": size_sol,
            "operation": "manual_buy"
        });

        let action = Action::new(
            action_id.clone(),
            ActionType::SwapBuy,
            mint.to_string(),
            BUY_STEPS.iter().map(|s| s.to_string()).collect(),
            metadata,
        );

        register_action(action)
            .await
            .map_err(|e| Error::ManualTradeRecord { detail: e })?;
        Ok(Self { action_id })
    }

    /// Start validation step
    pub async fn start_validation(&self) {
        update_step(
            &self.action_id,
            STEP_VALIDATE,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete validation step
    pub async fn complete_validation(&self) {
        update_step(
            &self.action_id,
            STEP_VALIDATE,
            StepStatus::Completed,
            None,
            None,
        )
        .await;
    }

    /// Fail validation step
    pub async fn fail_validation(&self, error: &str) {
        update_step(
            &self.action_id,
            STEP_VALIDATE,
            StepStatus::Failed,
            Some(error.to_string()),
            None,
        )
        .await;
        complete_action_failed(&self.action_id, error.to_string()).await;
    }

    /// Start quote step
    pub async fn start_quote(&self) {
        update_step(
            &self.action_id,
            STEP_QUOTE,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete quote step
    pub async fn complete_quote(&self, router: Option<&str>) {
        let metadata = router.map(|r| json!({"router": r}));
        update_step(
            &self.action_id,
            STEP_QUOTE,
            StepStatus::Completed,
            None,
            metadata,
        )
        .await;
    }

    /// Fail quote step
    pub async fn fail_quote(&self, error: &str) {
        update_step(
            &self.action_id,
            STEP_QUOTE,
            StepStatus::Failed,
            Some(error.to_string()),
            None,
        )
        .await;
        complete_action_failed(&self.action_id, error.to_string()).await;
    }

    /// Start swap step
    pub async fn start_swap(&self) {
        update_step(
            &self.action_id,
            STEP_SWAP,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete swap step
    pub async fn complete_swap(&self, signature: &str) {
        let metadata = json!({"signature": signature});
        update_step(
            &self.action_id,
            STEP_SWAP,
            StepStatus::Completed,
            None,
            Some(metadata),
        )
        .await;
    }

    /// Fail swap step
    pub async fn fail_swap(&self, error: &str) {
        update_step(
            &self.action_id,
            STEP_SWAP,
            StepStatus::Failed,
            Some(error.to_string()),
            None,
        )
        .await;
        complete_action_failed(&self.action_id, error.to_string()).await;
    }

    /// Start verification step
    pub async fn start_verify(&self) {
        update_step(
            &self.action_id,
            STEP_VERIFY,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete verification step and mark action as successful
    pub async fn complete_verify(&self, position_id: Option<i64>) {
        let metadata: Option<Value> = position_id.map(|id| json!({"position_id": id}));
        update_step(
            &self.action_id,
            STEP_VERIFY,
            StepStatus::Completed,
            None,
            metadata,
        )
        .await;
        complete_action_success(&self.action_id).await;
    }

    /// Skip verification (for async verification)
    pub async fn skip_verify_async(&self, signature: &str) {
        let metadata = json!({
            "verification": "async",
            "signature": signature
        });
        update_step(
            &self.action_id,
            STEP_VERIFY,
            StepStatus::Completed,
            None,
            Some(metadata),
        )
        .await;
        complete_action_success(&self.action_id).await;
    }

    /// Fail the action with error
    pub async fn fail(&self, error: &str) {
        complete_action_failed(&self.action_id, error.to_string()).await;
    }
}

/// Action tracker for manual sell operation
pub struct ManualSellAction {
    pub action_id: String,
}

impl ManualSellAction {
    /// Create and register a new manual sell action
    pub async fn new(
        mint: &str,
        symbol: Option<&str>,
        percentage: f64,
        position_id: Option<i64>,
    ) -> Result<Self, Error> {
        let action_id = Uuid::new_v4().to_string();

        let metadata = json!({
            "mint": mint,
            "symbol": symbol.unwrap_or("Unknown"),
            "percentage": percentage,
            "position_id": position_id,
            "operation": if percentage >= 100.0 { "manual_sell_full" } else { "manual_sell_partial" }
        });

        let action = Action::new(
            action_id.clone(),
            ActionType::SwapSell,
            mint.to_string(),
            SELL_STEPS.iter().map(|s| s.to_string()).collect(),
            metadata,
        );

        register_action(action)
            .await
            .map_err(|e| Error::ManualTradeRecord { detail: e })?;
        Ok(Self { action_id })
    }

    /// Start validation step
    pub async fn start_validation(&self) {
        update_step(
            &self.action_id,
            STEP_VALIDATE,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete validation step
    pub async fn complete_validation(&self) {
        update_step(
            &self.action_id,
            STEP_VALIDATE,
            StepStatus::Completed,
            None,
            None,
        )
        .await;
    }

    /// Fail validation step
    pub async fn fail_validation(&self, error: &str) {
        update_step(
            &self.action_id,
            STEP_VALIDATE,
            StepStatus::Failed,
            Some(error.to_string()),
            None,
        )
        .await;
        complete_action_failed(&self.action_id, error.to_string()).await;
    }

    /// Start quote step
    pub async fn start_quote(&self) {
        update_step(
            &self.action_id,
            STEP_QUOTE,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete quote step
    pub async fn complete_quote(&self, router: Option<&str>) {
        let metadata = router.map(|r| json!({"router": r}));
        update_step(
            &self.action_id,
            STEP_QUOTE,
            StepStatus::Completed,
            None,
            metadata,
        )
        .await;
    }

    /// Fail quote step
    pub async fn fail_quote(&self, error: &str) {
        update_step(
            &self.action_id,
            STEP_QUOTE,
            StepStatus::Failed,
            Some(error.to_string()),
            None,
        )
        .await;
        complete_action_failed(&self.action_id, error.to_string()).await;
    }

    /// Start swap step
    pub async fn start_swap(&self) {
        update_step(
            &self.action_id,
            STEP_SWAP,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete swap step
    pub async fn complete_swap(&self, signature: &str, sol_received: Option<f64>) {
        let metadata = json!({
            "signature": signature,
            "sol_received": sol_received
        });
        update_step(
            &self.action_id,
            STEP_SWAP,
            StepStatus::Completed,
            None,
            Some(metadata),
        )
        .await;
    }

    /// Fail swap step
    pub async fn fail_swap(&self, error: &str) {
        update_step(
            &self.action_id,
            STEP_SWAP,
            StepStatus::Failed,
            Some(error.to_string()),
            None,
        )
        .await;
        complete_action_failed(&self.action_id, error.to_string()).await;
    }

    /// Start verification step
    pub async fn start_verify(&self) {
        update_step(
            &self.action_id,
            STEP_VERIFY,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete verification step and mark action as successful
    pub async fn complete_verify(&self) {
        update_step(
            &self.action_id,
            STEP_VERIFY,
            StepStatus::Completed,
            None,
            None,
        )
        .await;
        complete_action_success(&self.action_id).await;
    }

    /// Skip verification (for async verification)
    pub async fn skip_verify_async(&self, signature: &str) {
        let metadata = json!({
            "verification": "async",
            "signature": signature
        });
        update_step(
            &self.action_id,
            STEP_VERIFY,
            StepStatus::Completed,
            None,
            Some(metadata),
        )
        .await;
        complete_action_success(&self.action_id).await;
    }

    /// Fail the action with error
    pub async fn fail(&self, error: &str) {
        complete_action_failed(&self.action_id, error.to_string()).await;
    }
}

/// Action tracker for manual DCA/add operation
pub struct ManualAddAction {
    pub action_id: String,
}

impl ManualAddAction {
    /// Create and register a new manual add (DCA) action
    pub async fn new(
        mint: &str,
        symbol: Option<&str>,
        size_sol: f64,
        position_id: Option<i64>,
    ) -> Result<Self, Error> {
        let action_id = Uuid::new_v4().to_string();

        let metadata = json!({
            "mint": mint,
            "symbol": symbol.unwrap_or("Unknown"),
            "size_sol": size_sol,
            "position_id": position_id,
            "operation": "manual_dca"
        });

        let action = Action::new(
            action_id.clone(),
            ActionType::PositionDca,
            mint.to_string(),
            ADD_STEPS.iter().map(|s| s.to_string()).collect(),
            metadata,
        );

        register_action(action)
            .await
            .map_err(|e| Error::ManualTradeRecord { detail: e })?;
        Ok(Self { action_id })
    }

    /// Start validation step
    pub async fn start_validation(&self) {
        update_step(
            &self.action_id,
            STEP_VALIDATE,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete validation step
    pub async fn complete_validation(&self) {
        update_step(
            &self.action_id,
            STEP_VALIDATE,
            StepStatus::Completed,
            None,
            None,
        )
        .await;
    }

    /// Fail validation step
    pub async fn fail_validation(&self, error: &str) {
        update_step(
            &self.action_id,
            STEP_VALIDATE,
            StepStatus::Failed,
            Some(error.to_string()),
            None,
        )
        .await;
        complete_action_failed(&self.action_id, error.to_string()).await;
    }

    /// Start quote step
    pub async fn start_quote(&self) {
        update_step(
            &self.action_id,
            STEP_QUOTE,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete quote step
    pub async fn complete_quote(&self, router: Option<&str>) {
        let metadata = router.map(|r| json!({"router": r}));
        update_step(
            &self.action_id,
            STEP_QUOTE,
            StepStatus::Completed,
            None,
            metadata,
        )
        .await;
    }

    /// Fail quote step
    pub async fn fail_quote(&self, error: &str) {
        update_step(
            &self.action_id,
            STEP_QUOTE,
            StepStatus::Failed,
            Some(error.to_string()),
            None,
        )
        .await;
        complete_action_failed(&self.action_id, error.to_string()).await;
    }

    /// Start swap step
    pub async fn start_swap(&self) {
        update_step(
            &self.action_id,
            STEP_SWAP,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete swap step
    pub async fn complete_swap(&self, signature: &str) {
        let metadata = json!({"signature": signature});
        update_step(
            &self.action_id,
            STEP_SWAP,
            StepStatus::Completed,
            None,
            Some(metadata),
        )
        .await;
    }

    /// Fail swap step
    pub async fn fail_swap(&self, error: &str) {
        update_step(
            &self.action_id,
            STEP_SWAP,
            StepStatus::Failed,
            Some(error.to_string()),
            None,
        )
        .await;
        complete_action_failed(&self.action_id, error.to_string()).await;
    }

    /// Start verification step
    pub async fn start_verify(&self) {
        update_step(
            &self.action_id,
            STEP_VERIFY,
            StepStatus::InProgress,
            None,
            None,
        )
        .await;
    }

    /// Complete verification step and mark action as successful
    pub async fn complete_verify(&self, new_entry_count: Option<u32>) {
        let metadata: Option<Value> = new_entry_count.map(|c| json!({"dca_count": c}));
        update_step(
            &self.action_id,
            STEP_VERIFY,
            StepStatus::Completed,
            None,
            metadata,
        )
        .await;
        complete_action_success(&self.action_id).await;
    }

    /// Skip verification (for async verification)
    pub async fn skip_verify_async(&self, signature: &str) {
        let metadata = json!({
            "verification": "async",
            "signature": signature
        });
        update_step(
            &self.action_id,
            STEP_VERIFY,
            StepStatus::Completed,
            None,
            Some(metadata),
        )
        .await;
        complete_action_success(&self.action_id).await;
    }

    /// Fail the action with error
    pub async fn fail(&self, error: &str) {
        complete_action_failed(&self.action_id, error.to_string()).await;
    }
}

// =============================================================================
// PREFLIGHT FAILURE HELPERS
// =============================================================================

/// Create an immediate-failure action for preflight errors (services not ready, blacklist, etc.)
/// This ensures errors that occur BEFORE the main trading logic are still tracked in the UI.
pub async fn create_failed_buy_action(mint: &str, error: &str) {
    // Try to get symbol
    let symbol = crate::tokens::get_full_token_async(mint)
        .await
        .ok()
        .flatten()
        .map(|t| t.symbol);

    if let Ok(action) = ManualBuyAction::new(mint, symbol.as_deref(), 0.0).await {
        action.start_validation().await;
        action.fail_validation(error).await;
    }
}

/// Create an immediate-failure action for preflight sell errors
pub async fn create_failed_sell_action(mint: &str, error: &str) {
    let symbol = crate::tokens::get_full_token_async(mint)
        .await
        .ok()
        .flatten()
        .map(|t| t.symbol);

    if let Ok(action) = ManualSellAction::new(mint, symbol.as_deref(), 100.0, None).await {
        action.start_validation().await;
        action.fail_validation(error).await;
    }
}

/// Create an immediate-failure action for preflight add (DCA) errors
pub async fn create_failed_add_action(mint: &str, error: &str) {
    let symbol = crate::tokens::get_full_token_async(mint)
        .await
        .ok()
        .flatten()
        .map(|t| t.symbol);

    if let Ok(action) = ManualAddAction::new(mint, symbol.as_deref(), 0.0, None).await {
        action.start_validation().await;
        action.fail_validation(error).await;
    }
}
