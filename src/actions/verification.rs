//! Actions whose last step waits for on-chain verification.
//!
//! A manual trade used to mark its "Verifying" step complete the moment the swap
//! was sent, so a transaction that never landed still reported success. The step
//! now stays in progress until the position verifier settles the signature, and
//! the action completes or fails on that verdict.
//!
//! The verifier can settle a signature before the trade flow registers to wait
//! for it, so an unclaimed verdict is kept for a while and handed over on
//! registration. Both maps live under one lock, which is what makes that
//! handover race-free.

use super::failure::ActionFailure;
use super::state::{complete_action_failed, complete_action_success, update_step};
use super::types::StepStatus;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// How long a verdict nobody was waiting for is kept for a late registration.
const UNCLAIMED_VERDICT_TTL: Duration = Duration::from_secs(15 * 60);

/// Upper bound on unclaimed verdicts. The bot's own trades are verified through
/// the same path and never register, so this must stay bounded.
const MAX_UNCLAIMED_VERDICTS: usize = 512;

/// The result of verifying one signature: `Err` carries the failure shown to the user.
pub type VerificationVerdict = Result<(), ActionFailure>;

#[derive(Default)]
struct Registry {
    /// signature -> (action id, verification step index)
    awaiting: HashMap<String, (String, usize)>,
    /// signature -> (when it was settled, verdict)
    unclaimed: HashMap<String, (Instant, VerificationVerdict)>,
}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| Mutex::new(Registry::default()));

/// Keep `step_index` of `action_id` in progress until `signature` is verified.
pub async fn await_verification(action_id: &str, step_index: usize, signature: &str) {
    update_step(
        action_id,
        step_index,
        StepStatus::InProgress,
        None,
        Some(json!({ "signature": signature })),
    )
    .await;

    let settled = {
        let Ok(mut registry) = REGISTRY.lock() else {
            return;
        };
        match registry.unclaimed.remove(signature) {
            Some((_, verdict)) => Some(verdict),
            None => {
                registry
                    .awaiting
                    .insert(signature.to_owned(), (action_id.to_owned(), step_index));
                None
            }
        }
    };

    if let Some(verdict) = settled {
        finish(action_id, step_index, signature, verdict).await;
    }
}

/// Record the verifier's verdict for `signature`, finishing any action waiting on it.
pub async fn settle_verification(signature: &str, verdict: VerificationVerdict) {
    let waiting = {
        let Ok(mut registry) = REGISTRY.lock() else {
            return;
        };
        match registry.awaiting.remove(signature) {
            Some(waiting) => Some(waiting),
            None => {
                let now = Instant::now();
                registry.unclaimed.retain(|_, (settled_at, _)| {
                    now.duration_since(*settled_at) < UNCLAIMED_VERDICT_TTL
                });
                if registry.unclaimed.len() >= MAX_UNCLAIMED_VERDICTS {
                    if let Some(oldest) = registry
                        .unclaimed
                        .iter()
                        .min_by_key(|(_, (settled_at, _))| *settled_at)
                        .map(|(key, _)| key.clone())
                    {
                        registry.unclaimed.remove(&oldest);
                    }
                }
                registry
                    .unclaimed
                    .insert(signature.to_owned(), (now, verdict.clone()));
                None
            }
        }
    };

    if let Some((action_id, step_index)) = waiting {
        finish(&action_id, step_index, signature, verdict).await;
    }
}

async fn finish(action_id: &str, step_index: usize, signature: &str, verdict: VerificationVerdict) {
    match verdict {
        Ok(()) => {
            update_step(
                action_id,
                step_index,
                StepStatus::Completed,
                None,
                Some(json!({ "signature": signature })),
            )
            .await;
            complete_action_success(action_id).await;
        }
        Err(reason) => {
            update_step(
                action_id,
                step_index,
                StepStatus::Failed,
                Some(reason.clone()),
                Some(json!({ "signature": signature })),
            )
            .await;
            complete_action_failed(action_id, reason).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::state::{get_action, register_action};
    use crate::actions::{Action, ActionState, ActionStepCode, ActionType};
    use crate::i18n::ids;

    async fn registered(id: &str) -> Action {
        let action = Action::new(
            id.to_owned(),
            ActionType::ManualOrder,
            "mint".to_owned(),
            vec![ActionStepCode::Swap, ActionStepCode::Verify],
            json!({}),
        );
        register_action(action.clone()).await.unwrap();
        action
    }

    async fn state_of(id: &str) -> ActionState {
        get_action(id).await.unwrap().state
    }

    #[tokio::test]
    async fn failed_verdict_fails_the_action_and_step_with_its_failure() {
        registered("verdict-fail-waiting").await;
        await_verification("verdict-fail-waiting", 1, "sig-fail-waiting").await;
        assert!(matches!(
            state_of("verdict-fail-waiting").await,
            ActionState::InProgress { .. }
        ));

        let failure = ActionFailure::with_details(ids::ACTIONS_FAILURE_VERIFICATION_GAVE_UP, "x");
        settle_verification("sig-fail-waiting", Err(failure.clone())).await;

        let action = get_action("verdict-fail-waiting").await.unwrap();
        assert_eq!(
            action.state,
            ActionState::Failed {
                error: failure.clone()
            }
        );
        assert_eq!(action.steps[1].status, StepStatus::Failed);
        assert_eq!(action.steps[1].error, Some(failure));
    }

    #[tokio::test]
    async fn verdict_settled_before_registration_is_handed_over() {
        registered("verdict-early-ok").await;
        registered("verdict-early-fail").await;
        settle_verification("sig-early-ok", Ok(())).await;
        let failure = ActionFailure::new(ids::ACTIONS_FAILURE_TRANSACTION_FAILED);
        settle_verification("sig-early-fail", Err(failure.clone())).await;

        await_verification("verdict-early-ok", 1, "sig-early-ok").await;
        await_verification("verdict-early-fail", 1, "sig-early-fail").await;

        assert_eq!(state_of("verdict-early-ok").await, ActionState::Completed);
        assert_eq!(
            state_of("verdict-early-fail").await,
            ActionState::Failed { error: failure }
        );
    }

    #[tokio::test]
    async fn ok_verdict_completes_a_waiting_action() {
        registered("verdict-ok-waiting").await;
        await_verification("verdict-ok-waiting", 1, "sig-ok-waiting").await;
        settle_verification("sig-ok-waiting", Ok(())).await;
        assert_eq!(state_of("verdict-ok-waiting").await, ActionState::Completed);
    }
}
