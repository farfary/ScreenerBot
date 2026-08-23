//! Action tracking for automated trading operations (auto open, close, DCA)

use crate::actions::{
    complete_action_failed, complete_action_success, register_action, update_step, Action,
    ActionType, StepStatus,
};
use crate::trader::error::Error;
use serde_json::json;
use uuid::Uuid;

/// Steps for automated position open
const AUTO_OPEN_STEPS: &[&str] = &["Evaluating", "Getting Quote", "Executing Swap", "Verifying"];

/// Steps for automated position close
const AUTO_CLOSE_STEPS: &[&str] = &["Evaluating", "Getting Quote", "Executing Swap", "Verifying"];

/// Steps for automated DCA
const AUTO_DCA_STEPS: &[&str] = &["Evaluating", "Getting Quote", "Executing Swap", "Verifying"];

/// Action tracker for automated position open (strategy-triggered buy)
pub struct AutoOpenAction {
    pub action_id: String,
}

impl AutoOpenAction {
    /// Create and register a new automated position open action
    pub async fn new(
        mint: &str,
        symbol: Option<&str>,
        strategy_id: Option<&str>,
        reason: &str,
    ) -> Result<Self, Error> {
        let action_id = Uuid::new_v4().to_string();

        let metadata = json!({
            "mint": mint,
            "symbol": symbol.unwrap_or("Unknown"),
            "strategy_id": strategy_id,
            "reason": reason,
            "operation": "auto_open"
        });

        let action = Action::new(
            action_id.clone(),
            ActionType::PositionOpen,
            mint.to_string(),
            AUTO_OPEN_STEPS.iter().map(|s| s.to_string()).collect(),
            metadata,
        );

        register_action(action)
            .await
            .map_err(|e| Error::ManualTradeRecord { detail: e })?;
        Ok(Self { action_id })
    }

    /// Complete evaluation step
    pub async fn complete_evaluation(&self) {
        update_step(&self.action_id, 0, StepStatus::Completed, None, None).await;
    }

    /// Start quote step
    pub async fn start_quote(&self) {
        update_step(&self.action_id, 1, StepStatus::InProgress, None, None).await;
    }

    /// Complete quote step
    pub async fn complete_quote(&self) {
        update_step(&self.action_id, 1, StepStatus::Completed, None, None).await;
    }

    /// Start swap step
    pub async fn start_swap(&self) {
        update_step(&self.action_id, 2, StepStatus::InProgress, None, None).await;
    }

    /// Complete swap step
    pub async fn complete_swap(&self, signature: &str) {
        let metadata = json!({"signature": signature});
        update_step(
            &self.action_id,
            2,
            StepStatus::Completed,
            None,
            Some(metadata),
        )
        .await;
    }

    /// Complete action successfully
    pub async fn complete(&self, signature: Option<&str>) {
        let metadata = signature.map(|s| json!({"signature": s, "verification": "async"}));
        update_step(&self.action_id, 3, StepStatus::Completed, None, metadata).await;
        complete_action_success(&self.action_id).await;
    }

    /// Fail the action with error
    pub async fn fail(&self, error: &str) {
        complete_action_failed(&self.action_id, error.to_string()).await;
    }
}

/// Action tracker for automated position close (strategy-triggered sell)
pub struct AutoCloseAction {
    pub action_id: String,
}

impl AutoCloseAction {
    /// Create and register a new automated position close action
    pub async fn new(
        mint: &str,
        symbol: Option<&str>,
        position_id: Option<i64>,
        reason: &str,
    ) -> Result<Self, Error> {
        let action_id = Uuid::new_v4().to_string();

        let metadata = json!({
            "mint": mint,
            "symbol": symbol.unwrap_or("Unknown"),
            "position_id": position_id,
            "reason": reason,
            "operation": "auto_close"
        });

        let action = Action::new(
            action_id.clone(),
            ActionType::PositionClose,
            mint.to_string(),
            AUTO_CLOSE_STEPS.iter().map(|s| s.to_string()).collect(),
            metadata,
        );

        register_action(action)
            .await
            .map_err(|e| Error::ManualTradeRecord { detail: e })?;
        Ok(Self { action_id })
    }

    /// Complete evaluation step
    pub async fn complete_evaluation(&self) {
        update_step(&self.action_id, 0, StepStatus::Completed, None, None).await;
    }

    /// Start quote step
    pub async fn start_quote(&self) {
        update_step(&self.action_id, 1, StepStatus::InProgress, None, None).await;
    }

    /// Complete quote step
    pub async fn complete_quote(&self) {
        update_step(&self.action_id, 1, StepStatus::Completed, None, None).await;
    }

    /// Start swap step
    pub async fn start_swap(&self) {
        update_step(&self.action_id, 2, StepStatus::InProgress, None, None).await;
    }

    /// Complete swap step
    pub async fn complete_swap(&self, signature: &str, sol_received: Option<f64>) {
        let metadata = json!({"signature": signature, "sol_received": sol_received});
        update_step(
            &self.action_id,
            2,
            StepStatus::Completed,
            None,
            Some(metadata),
        )
        .await;
    }

    /// Complete action successfully
    pub async fn complete(&self, signature: Option<&str>) {
        let metadata = signature.map(|s| json!({"signature": s, "verification": "async"}));
        update_step(&self.action_id, 3, StepStatus::Completed, None, metadata).await;
        complete_action_success(&self.action_id).await;
    }

    /// Fail the action with error
    pub async fn fail(&self, error: &str) {
        complete_action_failed(&self.action_id, error.to_string()).await;
    }
}

/// Action tracker for automated DCA (strategy-triggered position add)
pub struct AutoDcaAction {
    pub action_id: String,
}

impl AutoDcaAction {
    /// Create and register a new automated DCA action
    pub async fn new(
        mint: &str,
        symbol: Option<&str>,
        position_id: Option<&str>,
        dca_count: u32,
    ) -> Result<Self, Error> {
        let action_id = Uuid::new_v4().to_string();

        let metadata = json!({
            "mint": mint,
            "symbol": symbol.unwrap_or("Unknown"),
            "position_id": position_id,
            "dca_count": dca_count,
            "operation": "auto_dca"
        });

        let action = Action::new(
            action_id.clone(),
            ActionType::PositionDca,
            mint.to_string(),
            AUTO_DCA_STEPS.iter().map(|s| s.to_string()).collect(),
            metadata,
        );

        register_action(action)
            .await
            .map_err(|e| Error::ManualTradeRecord { detail: e })?;
        Ok(Self { action_id })
    }

    /// Complete evaluation step
    pub async fn complete_evaluation(&self) {
        update_step(&self.action_id, 0, StepStatus::Completed, None, None).await;
    }

    /// Start quote step
    pub async fn start_quote(&self) {
        update_step(&self.action_id, 1, StepStatus::InProgress, None, None).await;
    }

    /// Complete quote step
    pub async fn complete_quote(&self) {
        update_step(&self.action_id, 1, StepStatus::Completed, None, None).await;
    }

    /// Start swap step
    pub async fn start_swap(&self) {
        update_step(&self.action_id, 2, StepStatus::InProgress, None, None).await;
    }

    /// Complete swap step
    pub async fn complete_swap(&self, signature: &str) {
        let metadata = json!({"signature": signature});
        update_step(
            &self.action_id,
            2,
            StepStatus::Completed,
            None,
            Some(metadata),
        )
        .await;
    }

    /// Complete action successfully
    pub async fn complete(&self, signature: Option<&str>, new_dca_count: Option<u32>) {
        let metadata = json!({
            "signature": signature,
            "new_dca_count": new_dca_count,
            "verification": "async"
        });
        update_step(
            &self.action_id,
            3,
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
