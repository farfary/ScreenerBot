// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Runs a manual trade on its own task so no caller can cancel it mid-flight.

use crate::trader::Error;

/// Run a trade to completion whatever happens to the future awaiting it.
///
/// Every manual trade is started by something that can be dropped: an HTTP
/// handler whose client disconnects, an MCP call whose client times out, an
/// assistant tool under a deadline, a Telegram callback. A trade awaited
/// inline would then be cancelled after its swap was sent and before its
/// signature, position or exit was recorded. Spawned, the trade always
/// finishes and records; the caller only waits for its answer.
pub(super) async fn detached<T: Send + 'static>(
    trade: impl std::future::Future<Output = Result<T, Error>> + Send + 'static,
) -> Result<T, Error> {
    answer(tokio::spawn(trade).await)
}

/// The caller's answer from its trade's task. A panic is the trade's own and is
/// raised again where an inline await would have raised it. Nothing aborts a
/// trade task, so a cancellation means the runtime is shutting down: the trade
/// may have sent its swap, and the caller gets a typed error instead of a panic.
fn answer<T>(joined: Result<Result<T, Error>, tokio::task::JoinError>) -> Result<T, Error> {
    match joined {
        Ok(result) => result,
        Err(error) => match error.try_into_panic() {
            Ok(panic) => std::panic::resume_unwind(panic),
            Err(_) => Err(Error::TradeTaskCancelled),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{answer, detached};
    use crate::trader::Error;

    /// A caller that goes away mid-trade drops its own future, and with it
    /// everything awaited inline. The trade itself keeps running to its end
    /// and records its outcome.
    #[tokio::test]
    async fn a_dropped_caller_does_not_cancel_its_trade() {
        let (sent, confirm) = tokio::sync::oneshot::channel::<()>();
        let (recorded, outcome) = tokio::sync::oneshot::channel::<&'static str>();
        let caller = detached(async move {
            // The swap was sent; the trade now waits for its confirmation.
            confirm.await.expect("the confirmation arrives");
            recorded.send("recorded").expect("the outcome is read");
            Ok(())
        });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), caller)
                .await
                .is_err(),
            "the caller is still waiting when it goes away"
        );

        sent.send(()).expect("the trade is still alive to confirm");
        assert_eq!(outcome.await, Ok("recorded"));
    }

    #[tokio::test]
    async fn a_trade_panic_reaches_its_caller() {
        let joined = tokio::spawn(detached::<()>(async { panic!("trade panicked") })).await;
        assert!(joined.expect_err("the panic propagates").is_panic());
    }

    /// A trade task cancelled under the caller (runtime shutdown) answers with
    /// a typed error; the caller never panics on it.
    #[tokio::test]
    async fn a_cancelled_trade_task_answers_with_a_typed_error() {
        let task = tokio::spawn(std::future::pending::<Result<(), Error>>());
        task.abort();
        let joined = task.await;
        assert!(joined.as_ref().is_err_and(|e| e.is_cancelled()));
        assert!(matches!(answer(joined), Err(Error::TradeTaskCancelled)));
    }
}
