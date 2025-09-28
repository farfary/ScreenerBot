use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::LazyLock;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationKind {
    Entry,
    Exit,
}

#[derive(Debug, Clone)]
pub struct VerificationItem {
    pub signature: String,
    pub mint: String,
    pub position_id: Option<i64>,
    pub kind: VerificationKind,
    pub created_at: DateTime<Utc>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>, // backoff scheduling
    pub attempts: u8,
    pub expiry_height: Option<u64>,
}

impl VerificationItem {
    pub fn new(
        signature: String,
        mint: String,
        position_id: Option<i64>,
        kind: VerificationKind,
        expiry_height: Option<u64>,
    ) -> Self {
        Self {
            signature,
            mint,
            position_id,
            kind,
            created_at: Utc::now(),
            last_attempt_at: None,
            next_retry_at: None,
            attempts: 0,
            expiry_height,
        }
    }

    pub fn with_retry(&self) -> Self {
        // Compute exponential backoff (bounded) based on attempts (after increment)
        let next_attempts = self.attempts.saturating_add(1);
        // Tiered backoff in seconds (more conservative to reduce RPC pressure):
        // 5, 10, 20, 40, 60, 90, 120, 150, 180, 210, 240, 300
        let backoff_secs = match next_attempts {
            0 => 0,
            1 => 5,
            2 => 10,
            3 => 20,
            4 => 40,
            5 => 60,
            6 => 90,
            7 => 120,
            8 => 150,
            9 => 180,
            10 => 210,
            _ => 300,
        };

        // Add small jitter (±10%) to avoid thundering herd
        let jitter_fraction: f64 = 0.1;
        let jitter = {
            // Simple deterministic jitter based on signature hash and attempt number
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.signature.hash(&mut hasher);
            next_attempts.hash(&mut hasher);
            let h = hasher.finish();
            let sign = if (h & 1) == 0 { 1.0 } else { -1.0 };
            let frac = (((h >> 1) as f64) / ((u64::MAX >> 1) as f64)) * jitter_fraction;
            ((backoff_secs as f64) * frac * sign) as i64
        };
        let backoff_with_jitter = std::cmp::max(1, (backoff_secs as i64) + jitter);

        Self {
            signature: self.signature.clone(),
            mint: self.mint.clone(),
            position_id: self.position_id,
            kind: self.kind.clone(),
            created_at: self.created_at,
            last_attempt_at: Some(Utc::now()),
            next_retry_at: Some(Utc::now() + ChronoDuration::seconds(backoff_with_jitter)),
            attempts: next_attempts,
            expiry_height: self.expiry_height,
        }
    }

    pub fn is_expired(&self, current_height: Option<u64>) -> bool {
        if let (Some(expiry), Some(current)) = (self.expiry_height, current_height) {
            current > expiry
        } else {
            // Time-based fallback by kind: Entries 10m, Exits 3m
            let fallback_secs = match self.kind {
                VerificationKind::Entry => 600,
                VerificationKind::Exit => 180,
            };
            Utc::now()
                .signed_duration_since(self.created_at)
                .num_seconds()
                > fallback_secs
        }
    }

    pub fn age_seconds(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds()
    }

    pub fn is_due(&self) -> bool {
        match self.next_retry_at {
            None => true,
            Some(when) => Utc::now() >= when,
        }
    }
}

/// Verification queue
pub struct VerificationQueue {
    items: VecDeque<VerificationItem>,
}

impl VerificationQueue {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, item: VerificationItem) {
        // Check if already exists
        if !self.items.iter().any(|i| i.signature == item.signature) {
            self.items.push_back(item);
        }
    }

    pub fn poll_batch(&mut self, limit: usize) -> Vec<VerificationItem> {
        let mut batch = Vec::new();

        // Sort by priority: due items first, then recent (within 60s), then by age
        self.items.make_contiguous().sort_by(|a, b| {
            let a_due = a.is_due();
            let b_due = b.is_due();
            if a_due && !b_due {
                return std::cmp::Ordering::Less;
            }
            if !a_due && b_due {
                return std::cmp::Ordering::Greater;
            }

            let a_recent = a.age_seconds() <= 60;
            let b_recent = b.age_seconds() <= 60;

            match (a_recent, b_recent) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.age_seconds().cmp(&b.age_seconds()),
            }
        });

        // Drain up to limit DUE items and keep the rest
        let mut remaining: VecDeque<VerificationItem> = VecDeque::with_capacity(self.items.len());
        while let Some(item) = self.items.pop_front() {
            if batch.len() < limit && item.is_due() {
                batch.push(item);
            } else {
                remaining.push_back(item);
            }
        }
        self.items = remaining;

        batch
    }

    pub fn requeue(&mut self, item: VerificationItem) {
        // Allow more retries but with backoff; hard cap attempts to avoid infinite loops
        if item.attempts < 12 {
            self.items.push_back(item.with_retry());
        }
    }

    pub fn remove(&mut self, signature: &str) -> Option<VerificationItem> {
        if let Some(pos) = self.items.iter().position(|i| i.signature == signature) {
            self.items.remove(pos)
        } else {
            None
        }
    }

    pub fn gc_expired(&mut self, current_height: Option<u64>) -> Vec<VerificationItem> {
        let mut expired = Vec::new();
        let mut i = 0;

        while i < self.items.len() {
            if self.items[i].is_expired(current_height) {
                if let Some(item) = self.items.remove(i) {
                    expired.push(item);
                }
            } else {
                i += 1;
            }
        }

        expired
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_items_with_expiry(&self) -> bool {
        self.items.iter().any(|item| item.expiry_height.is_some())
    }
}

/// Global verification queue
static VERIFICATION_QUEUE: LazyLock<RwLock<VerificationQueue>> =
    LazyLock::new(|| RwLock::new(VerificationQueue::new()));

/// Enqueue verification item
pub async fn enqueue_verification(item: VerificationItem) {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.enqueue(item);
}

/// Poll batch of verification items
pub async fn poll_verification_batch(limit: usize) -> Vec<VerificationItem> {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.poll_batch(limit)
}

/// Requeue verification item
pub async fn requeue_verification(item: VerificationItem) {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.requeue(item);
}

/// Remove verification item
pub async fn remove_verification(signature: &str) -> Option<VerificationItem> {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.remove(signature)
}

/// Clean up expired items
pub async fn gc_expired_verifications(current_height: Option<u64>) -> Vec<VerificationItem> {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.gc_expired(current_height)
}

/// Get queue status
pub async fn get_queue_status() -> (usize, Vec<String>) {
    let queue = VERIFICATION_QUEUE.read().await;
    let size = queue.len();
    let signatures: Vec<String> = queue.items.iter().map(|i| i.signature.clone()).collect();
    (size, signatures)
}

/// Check if queue has items with expiry height
pub async fn queue_has_items_with_expiry() -> bool {
    let queue = VERIFICATION_QUEUE.read().await;
    queue.has_items_with_expiry()
}
