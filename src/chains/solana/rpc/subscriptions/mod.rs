// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The shared Solana RPC WebSocket subscription transport.
//!
//! One connection is multiplexed across every subscribed address. The transport
//! is domain-free: it delivers notifications and nothing else. Callers subscribe
//! by topic (an address's mentions or its own account) and hold a `Subscription`;
//! dropping it unsubscribes.
//!
//! Reconnects are handled here. Any stream termination -- a close frame, a
//! transport error, or an ended stream -- is a disconnect: the transport backs
//! off, reconnects, and resubscribes every address still registered.

mod client;
mod frames;
mod payloads;
mod registry;
mod types;

pub use payloads::{
    get_websocket_url, get_websocket_url_from_http, get_websocket_urls, websocket_url_for_attempt,
};
pub use types::{ConnectionState, Mention, Subscription, SubscriptionMetrics, Topic, TopicEvent};

/// Subscribe to `topic` on the shared connection. The subscription is live once it
/// delivers `TopicEvent::Confirmed`; dropping it unsubscribes.
pub async fn subscribe(topic: Topic) -> crate::Result<Subscription> {
    let handle_id = client::next_handle_id();
    let (tx, receiver) = tokio::sync::mpsc::unbounded_channel();

    let mut commands = client::ensure_actor().await?;
    if commands
        .send(client::Command::Subscribe {
            topic: topic.clone(),
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
                topic: topic.clone(),
                handle_id,
                tx,
            })
            .is_err()
        {
            return Err(crate::Error::Network(
                crate::errors::NetworkError::RequestFailed {
                    endpoint: "solana subscription".to_owned(),
                    detail: "subscription transport unavailable".to_owned(),
                },
            ));
        }
    }

    Ok(Subscription {
        id: handle_id,
        topic,
        receiver,
        commands,
    })
}

/// The number of the current connection; it grows by one on every connect, so
/// coverage established on an earlier connection can be told apart.
/// True while no configured endpoint serves the subscription methods the app uses.
pub fn subscriptions_unsupported() -> bool {
    client::subscriptions_unsupported()
}

pub fn connection_generation() -> u64 {
    client::connection_generation()
}

/// Watch the transport's connection state.
pub fn connection_state() -> tokio::sync::watch::Receiver<ConnectionState> {
    client::state_receiver()
}

/// Transport counters for service metrics.
pub fn subscription_metrics() -> SubscriptionMetrics {
    client::metrics_snapshot()
}
