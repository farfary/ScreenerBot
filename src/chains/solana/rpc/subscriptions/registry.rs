// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Subscriber bookkeeping for the shared subscription transport.

use std::collections::HashMap;
use tokio::sync::mpsc;

use super::types::{Topic, TopicEvent};

/// Tracks live subscribers, server-assigned subscription ids, and in-flight
/// subscribe requests for the shared WebSocket actor, per topic.
///
/// A server subscription lives only while its topic has a subscriber. Whichever
/// way the last one goes (an unsubscribe command, a receiver found closed during
/// fanout, or a confirmation arriving after its stream was dropped), the server
/// subscription is queued in `releases` for the actor to unsubscribe; otherwise
/// the server keeps streaming a busy address's notifications to nobody.
pub(super) struct Registry {
    /// topic -> live subscribers, each with the handle id used to remove it
    subscribers: HashMap<Topic, Vec<(u64, mpsc::UnboundedSender<TopicEvent>)>>,
    /// server-assigned subscription id -> topic
    by_sub_id: HashMap<u64, Topic>,
    /// topic -> server-assigned subscription id (for unsubscribe)
    sub_id_of: HashMap<Topic, u64>,
    /// in-flight subscribe request id -> topic
    pending: HashMap<u64, Topic>,
    /// in-flight subscribe requests whose topic lost every subscriber
    orphaned: HashMap<u64, Topic>,
    /// server subscriptions no subscriber needs any more, to unsubscribe
    releases: Vec<(Topic, u64)>,
}

impl Registry {
    /// Create an empty registry.
    pub(super) fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
            by_sub_id: HashMap::new(),
            sub_id_of: HashMap::new(),
            pending: HashMap::new(),
            orphaned: HashMap::new(),
            releases: Vec::new(),
        }
    }

    /// Add a subscriber for `topic`. Returns true if this topic had no
    /// subscribers before (the caller must then send its subscribe request).
    pub(super) fn add_subscriber(
        &mut self,
        topic: &Topic,
        handle_id: u64,
        tx: mpsc::UnboundedSender<TopicEvent>,
    ) -> bool {
        match self.subscribers.get_mut(topic) {
            Some(existing) => {
                existing.push((handle_id, tx));
                false
            }
            None => {
                self.subscribers
                    .insert(topic.clone(), vec![(handle_id, tx)]);
                true
            }
        }
    }

    /// Remove one subscriber handle from `topic`, releasing the topic when that
    /// was its last subscriber.
    pub(super) fn remove_subscriber(&mut self, topic: &Topic, handle_id: u64) {
        let Some(list) = self.subscribers.get_mut(topic) else {
            return;
        };
        list.retain(|(id, _)| *id != handle_id);
        if list.is_empty() {
            self.release(topic);
        }
    }

    /// Forget a topic nobody subscribes to: queue its server subscription for
    /// unsubscribing and mark its in-flight requests so their confirmations are
    /// released too.
    fn release(&mut self, topic: &Topic) {
        self.subscribers.remove(topic);
        if let Some(sub_id) = self.sub_id_of.remove(topic) {
            self.by_sub_id.remove(&sub_id);
            self.releases.push((topic.clone(), sub_id));
        }
        let requests: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, pending)| *pending == topic)
            .map(|(id, _)| *id)
            .collect();
        for id in requests {
            if let Some(pending) = self.pending.remove(&id) {
                self.orphaned.insert(id, pending);
            }
        }
    }

    /// Server subscriptions to unsubscribe, oldest first.
    pub(super) fn take_releases(&mut self) -> Vec<(Topic, u64)> {
        std::mem::take(&mut self.releases)
    }

    /// All topics with at least one live subscriber.
    pub(super) fn topics(&self) -> Vec<Topic> {
        self.subscribers.keys().cloned().collect()
    }

    /// True when no topics remain.
    pub(super) fn is_empty(&self) -> bool {
        self.subscribers.is_empty()
    }

    /// Record an in-flight subscribe request for `topic`.
    pub(super) fn note_pending(&mut self, request_id: u64, topic: &Topic) {
        self.pending.insert(request_id, topic.clone());
    }

    /// The topic of an in-flight subscribe request.
    pub(super) fn pending_topic(&self, request_id: u64) -> Option<&Topic> {
        self.pending
            .get(&request_id)
            .or_else(|| self.orphaned.get(&request_id))
    }

    /// Resolve a subscribe confirmation: moves the pending request into the
    /// confirmed id maps, tells the topic's subscribers, and returns the topic.
    /// A confirmation for a topic that lost every subscriber is released instead.
    pub(super) fn confirm(&mut self, request_id: u64, subscription_id: u64) -> Option<Topic> {
        if let Some(topic) = self.orphaned.remove(&request_id) {
            self.releases.push((topic, subscription_id));
            return None;
        }
        let topic = self.pending.remove(&request_id)?;
        self.by_sub_id.insert(subscription_id, topic.clone());
        self.sub_id_of.insert(topic.clone(), subscription_id);
        self.fanout(&topic, &TopicEvent::Confirmed);
        Some(topic)
    }

    /// Resolve a refused subscribe request: forgets it and drops every subscriber
    /// of its topic, which ends their streams. Returns the topic.
    pub(super) fn refuse(&mut self, request_id: u64) -> Option<Topic> {
        if let Some(topic) = self.orphaned.remove(&request_id) {
            return Some(topic);
        }
        let topic = self.pending.remove(&request_id)?;
        self.subscribers.remove(&topic);
        Some(topic)
    }

    /// Look up the topic a server subscription id belongs to.
    pub(super) fn topic_for_subscription(&self, subscription_id: u64) -> Option<&Topic> {
        self.by_sub_id.get(&subscription_id)
    }

    /// Forget server subscription ids and pending requests. Called on every
    /// disconnect, because server subscription ids do not survive a reconnect.
    /// Subscribers are kept.
    pub(super) fn clear_server_state(&mut self) {
        self.by_sub_id.clear();
        self.sub_id_of.clear();
        self.pending.clear();
        self.orphaned.clear();
        self.releases.clear();
    }

    /// Deliver a clone of `event` to every subscriber of `topic`, pruning any
    /// whose receiver has been dropped. Releases the topic if pruning empties it.
    pub(super) fn fanout(&mut self, topic: &Topic, event: &TopicEvent) {
        let Some(list) = self.subscribers.get_mut(topic) else {
            return;
        };
        list.retain(|(_, tx)| tx.send(event.clone()).is_ok());
        if list.is_empty() {
            self.release(topic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::rpc::subscriptions::types::Mention;

    fn mentions(address: &str) -> Topic {
        Topic::Mentions(address.to_owned())
    }

    fn event() -> TopicEvent {
        TopicEvent::Mention(Mention {
            signature: "sig".to_owned(),
            slot: Some(1),
            failed: false,
        })
    }

    #[test]
    fn add_subscriber_reports_first_and_subsequent() {
        let mut registry = Registry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        assert!(registry.add_subscriber(&mentions("addr"), 1, tx1));
        assert!(!registry.add_subscriber(&mentions("addr"), 2, tx2));
    }

    #[test]
    fn the_same_address_has_one_subscription_per_topic() {
        let mut registry = Registry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        assert!(registry.add_subscriber(&mentions("addr"), 1, tx1));
        assert!(registry.add_subscriber(&Topic::Account("addr".to_owned()), 2, tx2));
        assert_eq!(registry.topics().len(), 2);
    }

    #[test]
    fn remove_subscriber_only_unsubscribes_on_last() {
        let mut registry = Registry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let topic = mentions("addr");
        registry.add_subscriber(&topic, 1, tx1);
        registry.add_subscriber(&topic, 2, tx2);
        registry.note_pending(7, &topic);
        registry.confirm(7, 42);

        registry.remove_subscriber(&topic, 1);
        assert!(registry.take_releases().is_empty());
        registry.remove_subscriber(&topic, 2);
        assert_eq!(registry.take_releases(), vec![(topic, 42)]);
        assert!(registry.is_empty());
    }

    #[test]
    fn a_stream_dropped_before_its_unsubscribe_arrives_is_still_released() {
        let mut registry = Registry::new();
        let (tx, rx) = mpsc::unbounded_channel();
        let topic = mentions("addr");
        registry.add_subscriber(&topic, 1, tx);
        registry.note_pending(7, &topic);
        registry.confirm(7, 42);
        drop(rx);

        registry.fanout(&topic, &event());
        registry.remove_subscriber(&topic, 1);

        assert_eq!(registry.take_releases(), vec![(topic, 42)]);
        assert_eq!(registry.topic_for_subscription(42), None);
        assert!(registry.is_empty());
    }

    #[test]
    fn a_stream_dropped_before_confirmation_is_released_when_confirmed() {
        let mut registry = Registry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let topic = mentions("addr");
        registry.add_subscriber(&topic, 1, tx);
        registry.note_pending(7, &topic);
        registry.remove_subscriber(&topic, 1);
        assert!(registry.take_releases().is_empty());

        assert_eq!(registry.confirm(7, 42), None);
        assert_eq!(registry.take_releases(), vec![(topic, 42)]);
        assert_eq!(registry.topic_for_subscription(42), None);
    }

    #[test]
    fn a_resubscribe_after_a_release_gets_only_its_own_subscription() {
        let mut registry = Registry::new();
        let (old_tx, old_rx) = mpsc::unbounded_channel();
        let topic = mentions("addr");
        registry.add_subscriber(&topic, 1, old_tx);
        registry.note_pending(1, &topic);
        registry.confirm(1, 41);
        drop(old_rx);
        registry.fanout(&topic, &event());

        let (tx, _rx) = mpsc::unbounded_channel();
        assert!(registry.add_subscriber(&topic, 2, tx));
        registry.note_pending(2, &topic);
        registry.confirm(2, 42);
        registry.remove_subscriber(&topic, 1);

        assert_eq!(registry.take_releases(), vec![(topic.clone(), 41)]);
        assert_eq!(registry.topic_for_subscription(41), None);
        assert_eq!(registry.topic_for_subscription(42), Some(&topic));
    }

    #[test]
    fn a_confirmation_tells_every_subscriber_it_is_live() {
        let mut registry = Registry::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let topic = Topic::Account("addr".to_owned());
        registry.add_subscriber(&topic, 1, tx);
        registry.note_pending(3, &topic);

        assert_eq!(registry.confirm(3, 9), Some(topic.clone()));
        assert_eq!(rx.try_recv().ok(), Some(TopicEvent::Confirmed));
        assert_eq!(registry.topic_for_subscription(9), Some(&topic));
    }

    #[test]
    fn fanout_delivers_to_all_live_subscribers() {
        let mut registry = Registry::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        registry.add_subscriber(&mentions("addr"), 1, tx1);
        registry.add_subscriber(&mentions("addr"), 2, tx2);

        registry.fanout(&mentions("addr"), &event());

        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn fanout_prunes_dropped_receivers() {
        let mut registry = Registry::new();
        let (tx1, rx1) = mpsc::unbounded_channel();
        drop(rx1);
        registry.add_subscriber(&mentions("addr"), 1, tx1);

        registry.fanout(&mentions("addr"), &event());

        assert!(registry.is_empty());
    }

    #[test]
    fn a_refused_subscribe_ends_its_subscribers_streams() {
        let mut registry = Registry::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let topic = mentions("Addr");
        registry.add_subscriber(&topic, 1, tx);
        registry.note_pending(7, &topic);

        assert_eq!(registry.refuse(7), Some(topic));
        assert!(registry.is_empty());
        assert!(rx.try_recv().is_err());
        assert!(rx.is_closed(), "the subscriber's stream has ended");
        assert_eq!(registry.refuse(7), None, "a request is refused once");
    }

    #[test]
    fn clear_server_state_keeps_subscribers_but_forgets_ids() {
        let mut registry = Registry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let topic = mentions("addr");
        registry.add_subscriber(&topic, 1, tx1);
        registry.note_pending(1, &topic);
        registry.confirm(1, 5);

        registry.clear_server_state();

        assert_eq!(registry.topic_for_subscription(5), None);
        assert_eq!(registry.topics(), vec![topic]);
    }
}
