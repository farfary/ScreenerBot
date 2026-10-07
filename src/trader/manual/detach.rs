// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Runs a manual trade on its own task so no caller can cancel it mid-flight.

/// Run a trade to completion whatever happens to the future awaiting it.
///
/// Every manual trade is started by something that can be dropped: an HTTP
/// handler whose client disconnects, an MCP call whose client times out, an
/// assistant tool under a deadline, a Telegram callback. A trade awaited
/// inline would then be cancelled after its swap was sent and before its
/// signature, position or exit was recorded. Spawned, the trade always
/// finishes and records; the caller only waits for its answer. The task is
/// never aborted, so a join error is the trade's own panic, raised again where
/// an inline await would have raised it.
pub(super) async fn detached<T: Send + 'static>(
    trade: impl std::future::Future<Output = T> + Send + 'static,
) -> T {
    match tokio::spawn(trade).await {
        Ok(result) => result,
        Err(error) => std::panic::resume_unwind(error.into_panic()),
    }
}

#[cfg(test)]
mod tests {
    use super::detached;

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
        let joined = tokio::spawn(detached(async { panic!("trade panicked") })).await;
        assert!(joined.expect_err("the panic propagates").is_panic());
    }
}
