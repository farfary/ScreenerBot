use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::types::LicenseStatus;

const CACHE_TTL: Duration = Duration::from_secs(1800); // 30 minutes - license rarely changes
const STALE_GRACE_PERIOD: Duration = Duration::from_secs(3600); // Return stale data up to 1 hour old

pub struct LicenseCache {
    entries: Mutex<HashMap<String, CacheEntry>>,
}

struct CacheEntry {
    status: LicenseStatus,
    expires_at: Instant,
}

impl LicenseCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, wallet: &str) -> Option<LicenseStatus> {
        let mut entries = self.entries.lock().unwrap();

        if let Some(entry) = entries.get(wallet) {
            if Instant::now() < entry.expires_at {
                return Some(entry.status.clone());
            } else {
                // Expired, remove it
                entries.remove(wallet);
            }
        }

        None
    }

    /// Get cached license even if stale (for non-blocking API calls)
    /// Returns (status, is_fresh) - is_fresh=false means data is stale but usable
    pub fn get_cached_or_stale(&self, wallet: &str) -> Option<(LicenseStatus, bool)> {
        let entries = self.entries.lock().unwrap();

        if let Some(entry) = entries.get(wallet) {
            let now = Instant::now();
            if now < entry.expires_at {
                // Fresh data
                return Some((entry.status.clone(), true));
            } else if now < entry.expires_at + STALE_GRACE_PERIOD {
                // Stale but within grace period - return it anyway
                return Some((entry.status.clone(), false));
            }
        }

        None
    }

    /// Check if cache needs refresh (expired but might have stale data)
    pub fn needs_refresh(&self, wallet: &str) -> bool {
        let entries = self.entries.lock().unwrap();

        match entries.get(wallet) {
            Some(entry) => Instant::now() >= entry.expires_at,
            None => true,
        }
    }

    pub fn set(&self, wallet: &str, status: LicenseStatus) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(
            wallet.to_string(),
            CacheEntry {
                status,
                expires_at: Instant::now() + CACHE_TTL,
            },
        );
    }

    pub fn invalidate(&self, wallet: &str) {
        let mut entries = self.entries.lock().unwrap();
        entries.remove(wallet);
    }

    pub fn clear(&self) {
        let mut entries = self.entries.lock().unwrap();
        entries.clear();
    }
}

impl Default for LicenseCache {
    fn default() -> Self {
        Self::new()
    }
}
