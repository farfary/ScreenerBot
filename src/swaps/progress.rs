//! Progress a swap reports while it runs, for callers that display it.
//!
//! A trade is quoted and executed deep inside the position operations, several
//! calls below the flow that tracks it for the user. Rather than thread a
//! callback through every layer, the tracking flow scopes a listener around the
//! work and [`execute_swap_with_fallback`](super::execute_swap_with_fallback)
//! reports into whatever listener is in scope. Work with no listener in scope
//! (the auto-trader) reports into nothing.

use futures::future::BoxFuture;
use std::future::Future;
use std::sync::Arc;

/// A stage a swap has reached.
#[derive(Debug, Clone)]
pub enum SwapStage {
    /// Quoting is finished and `router` is about to submit the transaction.
    Submitting { router: String },
    /// A built transaction was refused before signing because it would have
    /// spent `extra_lamports` of the wallet's SOL on an account owned by
    /// `venue`, outside the trade. The swap continues down another route, so
    /// this reports a decision rather than a failure.
    CostRejected {
        router: String,
        venue: String,
        extra_lamports: u64,
    },
}

/// Receives stages; the returned future is awaited before the swap continues,
/// so what the listener records can never land after a later stage.
pub type SwapStageListener = Arc<dyn Fn(SwapStage) -> BoxFuture<'static, ()> + Send + Sync>;

tokio::task_local! {
    static LISTENER: SwapStageListener;
}

/// Run `work` with `listener` receiving every stage a swap inside it reports.
pub async fn with_swap_stage_listener<F: Future>(
    listener: SwapStageListener,
    work: F,
) -> F::Output {
    LISTENER.scope(listener, work).await
}

/// Report `stage` to the listener in scope, if any.
pub(crate) async fn report_swap_stage(stage: SwapStage) {
    if let Ok(pending) = LISTENER.try_with(|listener| listener(stage)) {
        pending.await;
    }
}
