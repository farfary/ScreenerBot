use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::types::LicenseStatus;

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

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
