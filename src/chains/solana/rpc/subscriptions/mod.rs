//! The shared Solana RPC WebSocket subscription transport.
//!
//! One connection is multiplexed across every subscribed address. The transport
//! is domain-free: it delivers notifications and nothing else. Callers subscribe
//! by address and hold a `LogsSubscription`; dropping it unsubscribes.
//!
//! Reconnects are handled here. Any stream termination -- a close frame, a
//! transport error, or an ended stream -- is a disconnect: the transport backs
//! off, reconnects, and resubscribes every address still registered.

mod client;
mod payloads;
mod registry;
mod types;

pub use payloads::{
    get_websocket_url, get_websocket_url_from_http, get_websocket_urls, websocket_url_for_attempt,
};
pub use types::{ConnectionState, LogsSubscription, SubscriptionEvent, SubscriptionMetrics};

/// Subscribe to `logsSubscribe` notifications mentioning `address`.
pub async fn subscribe_logs_mentions(address: &str) -> crate::Result<LogsSubscription> {
    let handle_id = client::next_handle_id();
    let (tx, receiver) = tokio::sync::mpsc::unbounded_channel();

    let mut commands = client::ensure_actor().await?;
    if commands
        .send(client::Command::Subscribe {
            address: address.to_owned(),
            handle_id,
            tx: tx.clone(),
        })
        .is_err()
    {
        // The actor exited between ensure_actor() and our send — it shuts itself
        // down once the last subscription goes away. Start a fresh one and retry
        // exactly once.
        commands = client::ensure_actor().await?;
        if commands
            .send(client::Command::Subscribe {
                address: address.to_owned(),
                handle_id,
                tx,
            })
            .is_err()
        {
            return Err(crate::Error::Network(
                crate::errors::NetworkError::RequestFailed {
                    endpoint: "solana log subscription".to_owned(),
                    detail: "subscription transport unavailable".to_owned(),
                },
            ));
        }
    }

    Ok(LogsSubscription {
        id: handle_id,
        address: address.to_owned(),
        receiver,
        commands,
    })
}

/// Watch the transport's connection state.
pub fn connection_state() -> tokio::sync::watch::Receiver<ConnectionState> {
    client::state_receiver()
}

/// Transport counters for service metrics.
pub fn subscription_metrics() -> SubscriptionMetrics {
    client::metrics_snapshot()
}
