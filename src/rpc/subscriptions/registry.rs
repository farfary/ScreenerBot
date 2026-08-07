//! Subscriber bookkeeping for the shared subscription transport.

use std::collections::HashMap;
use tokio::sync::mpsc;

use super::types::SubscriptionEvent;

/// Tracks live subscribers, server-assigned subscription ids, and in-flight
/// subscribe requests for the shared WebSocket actor.
pub(super) struct Registry {
    /// address -> live subscribers, each with the handle id used to remove it
    subscribers: HashMap<String, Vec<(u64, mpsc::UnboundedSender<SubscriptionEvent>)>>,
    /// server-assigned subscription id -> address
    by_sub_id: HashMap<u64, String>,
    /// address -> server-assigned subscription id (for unsubscribe)
    sub_id_of: HashMap<String, u64>,
    /// in-flight subscribe request id -> address
    pending: HashMap<u64, String>,
}

impl Registry {
    /// Create an empty registry.
    pub(super) fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
            by_sub_id: HashMap::new(),
            sub_id_of: HashMap::new(),
            pending: HashMap::new(),
        }
    }

    /// Add a subscriber for `address`. Returns true if this address had no
    /// subscribers before (the caller must then send a `logsSubscribe`).
    pub(super) fn add_subscriber(
        &mut self,
        address: &str,
        handle_id: u64,
        tx: mpsc::UnboundedSender<SubscriptionEvent>,
    ) -> bool {
        match self.subscribers.get_mut(address) {
            Some(existing) => {
                existing.push((handle_id, tx));
                false
            }
            None => {
                self.subscribers
                    .insert(address.to_owned(), vec![(handle_id, tx)]);
                true
            }
        }
    }

    /// Remove one subscriber handle from `address`. If that was the last
    /// subscriber, removes the address entirely and returns its server
    /// subscription id (if one was assigned) so the caller can unsubscribe.
    /// Returns `None` when subscribers remain or no id was assigned.
    pub(super) fn remove_subscriber(&mut self, address: &str, handle_id: u64) -> Option<u64> {
        let list = self.subscribers.get_mut(address)?;
        list.retain(|(id, _)| *id != handle_id);
        if !list.is_empty() {
            return None;
        }
        self.subscribers.remove(address);
        let sub_id = self.sub_id_of.remove(address);
        if let Some(id) = sub_id {
            self.by_sub_id.remove(&id);
        }
        sub_id
    }

    /// All addresses with at least one live subscriber.
    pub(super) fn addresses(&self) -> Vec<String> {
        self.subscribers.keys().cloned().collect()
    }

    /// True when no addresses remain.
    pub(super) fn is_empty(&self) -> bool {
        self.subscribers.is_empty()
    }

    /// Record an in-flight subscribe request for `address`.
    pub(super) fn note_pending(&mut self, request_id: u64, address: &str) {
        self.pending.insert(request_id, address.to_owned());
    }

    /// Resolve a subscribe confirmation: moves the pending request into the
    /// confirmed id maps and returns the address it belongs to.
    pub(super) fn confirm(&mut self, request_id: u64, subscription_id: u64) -> Option<String> {
        let address = self.pending.remove(&request_id)?;
        self.by_sub_id.insert(subscription_id, address.clone());
        self.sub_id_of.insert(address.clone(), subscription_id);
        Some(address)
    }

    /// Look up the address a server subscription id belongs to.
    pub(super) fn address_for_subscription(&self, subscription_id: u64) -> Option<&str> {
        self.by_sub_id.get(&subscription_id).map(|s| s.as_str())
    }

    /// Forget server subscription ids and pending requests. Called on every
    /// disconnect, because server subscription ids do not survive a reconnect.
    /// Subscribers are kept.
    pub(super) fn clear_server_state(&mut self) {
        self.by_sub_id.clear();
        self.sub_id_of.clear();
        self.pending.clear();
    }

    /// Deliver a clone of `event` to every subscriber of `address`, pruning any
    /// whose receiver has been dropped. Removes the address if pruning empties it.
    pub(super) fn fanout(&mut self, address: &str, event: &SubscriptionEvent) {
        let Some(list) = self.subscribers.get_mut(address) else {
            return;
        };
        list.retain(|(_, tx)| tx.send(event.clone()).is_ok());
        if list.is_empty() {
            self.subscribers.remove(address);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(address: &str) -> SubscriptionEvent {
        SubscriptionEvent {
            address: address.to_owned(),
            signature: "sig".to_owned(),
            slot: Some(1),
            failed: false,
            logs: Vec::new(),
        }
    }

    #[test]
    fn add_subscriber_reports_first_and_subsequent() {
        let mut registry = Registry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        assert!(registry.add_subscriber("addr", 1, tx1));
        assert!(!registry.add_subscriber("addr", 2, tx2));
    }

    #[test]
    fn remove_subscriber_only_unsubscribes_on_last() {
        let mut registry = Registry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        registry.add_subscriber("addr", 1, tx1);
        registry.add_subscriber("addr", 2, tx2);
        registry.note_pending(7, "addr");
        registry.confirm(7, 42);

        assert_eq!(registry.remove_subscriber("addr", 1), None);
        assert_eq!(registry.remove_subscriber("addr", 2), Some(42));
    }

    #[test]
    fn fanout_delivers_to_all_live_subscribers() {
        let mut registry = Registry::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        registry.add_subscriber("addr", 1, tx1);
        registry.add_subscriber("addr", 2, tx2);

        registry.fanout("addr", &event("addr"));

        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn fanout_prunes_dropped_receivers() {
        let mut registry = Registry::new();
        let (tx1, rx1) = mpsc::unbounded_channel();
        drop(rx1);
        registry.add_subscriber("addr", 1, tx1);

        registry.fanout("addr", &event("addr"));

        assert!(registry.is_empty());
    }

    #[test]
    fn clear_server_state_keeps_subscribers_but_forgets_ids() {
        let mut registry = Registry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        registry.add_subscriber("addr", 1, tx1);
        registry.note_pending(7, "addr");
        registry.confirm(7, 42);

        registry.clear_server_state();

        assert_eq!(registry.addresses(), vec!["addr".to_owned()]);
        assert_eq!(registry.address_for_subscription(42), None);
    }
}
