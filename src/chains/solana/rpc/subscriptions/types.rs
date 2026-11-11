// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Types for the shared RPC subscription transport.

use tokio::sync::mpsc;

/// What a subscription listens to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Topic {
    /// Every transaction that mentions the address, failed ones included (`logsSubscribe`).
    Mentions(String),
    /// Changes of the address's own account, its lamports or data (`accountSubscribe`).
    Account(String),
}

impl Topic {
    /// The subscribed address (base58).
    pub fn address(&self) -> &str {
        match self {
            Self::Mentions(address) | Self::Account(address) => address,
        }
    }

    /// The JSON-RPC subscribe method serving this topic.
    pub fn method(&self) -> &'static str {
        match self {
            Self::Mentions(_) => "logsSubscribe",
            Self::Account(_) => "accountSubscribe",
        }
    }
}

/// One transaction that mentions a subscribed address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    /// Transaction signature (base58).
    pub signature: String,
    /// Slot from the notification context, when present.
    pub slot: Option<u64>,
    /// True when the notification carried a non-null `err` (the transaction failed on chain).
    pub failed: bool,
}

/// One message the transport delivers to a subscriber.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopicEvent {
    /// The server accepted the subscription, on first subscribe and again after every
    /// reconnect. Notifications follow from here; anything earlier was not observed.
    Confirmed,
    /// A transaction mentioning the address (`Topic::Mentions`).
    Mention(Mention),
    /// The account changed in `slot` (`Topic::Account`).
    AccountChanged { slot: u64 },
}

/// Health of the single shared WebSocket connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// No connection has been attempted yet, or the last subscriber went away.
    #[default]
    Idle,
    /// Socket is open.
    Connected,
    /// Socket dropped; a reconnect is scheduled or in progress.
    Reconnecting,
    /// Reconnect attempts have been failing continuously.
    Down,
}

/// Transport counters for service metrics.
#[derive(Debug, Clone, Default)]
pub struct SubscriptionMetrics {
    pub state: ConnectionState,
    /// Topics with at least one live subscriber.
    pub active_addresses: u64,
    pub notifications_received: u64,
    pub reconnects: u64,
    /// Unix seconds of the last notification, when one has been received.
    pub last_message_unix: Option<i64>,
}

/// A live subscription to one topic. Dropping it unsubscribes.
pub struct Subscription {
    pub(super) id: u64,
    pub(super) topic: Topic,
    pub(super) receiver: mpsc::UnboundedReceiver<TopicEvent>,
    /// The actor's command channel, held so `Drop` can unsubscribe without
    /// contending for a lock — a missed unsubscribe would leak a server-side
    /// subscription for an address nobody is listening to any more.
    pub(super) commands: mpsc::UnboundedSender<super::client::Command>,
}

impl Subscription {
    /// The subscribed topic.
    pub fn topic(&self) -> &Topic {
        &self.topic
    }

    /// Await the next event. `None` means the transport dropped this subscription:
    /// the endpoint refused it, or the transport shut down.
    pub async fn recv(&mut self) -> Option<TopicEvent> {
        self.receiver.recv().await
    }
}
