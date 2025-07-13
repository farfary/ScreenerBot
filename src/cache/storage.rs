// Storage utilities for the cache system

use crate::core::{ BotResult, BotError };
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use serde::{ Serialize, Deserialize };

/// In-memory storage for frequently accessed data
#[derive(Debug)]
pub struct MemoryStorage {
    data: Arc<Mutex<HashMap<String, StorageEntry>>>,
    max_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEntry {
    pub data: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
}

impl MemoryStorage {
    pub fn new(max_size: usize) -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            max_size,
        }
    }

    pub fn insert(&self, key: String, value: String) -> BotResult<()> {
        let mut data = self.data.lock().unwrap();

        // If we're at capacity, remove least recently used
        if data.len() >= self.max_size {
            let lru_key = data
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(k, _)| k.clone());

            if let Some(lru_key) = lru_key {
                data.remove(&lru_key);
            }
        }

        let now = chrono::Utc::now();
        data.insert(key, StorageEntry {
            data: value,
            created_at: now,
            access_count: 0,
            last_accessed: now,
        });

        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut data = self.data.lock().unwrap();

        if let Some(entry) = data.get_mut(key) {
            entry.access_count += 1;
            entry.last_accessed = chrono::Utc::now();
            Some(entry.data.clone())
        } else {
            None
        }
    }

    pub fn remove(&self, key: &str) -> Option<String> {
        let mut data = self.data.lock().unwrap();
        data.remove(key).map(|entry| entry.data)
    }

    pub fn clear(&self) {
        let mut data = self.data.lock().unwrap();
        data.clear();
    }

    pub fn size(&self) -> usize {
        let data = self.data.lock().unwrap();
        data.len()
    }

    pub fn get_stats(&self) -> MemoryStorageStats {
        let data = self.data.lock().unwrap();
        let total_accesses: u64 = data
            .values()
            .map(|entry| entry.access_count)
            .sum();

        MemoryStorageStats {
            entry_count: data.len(),
            total_accesses,
            max_size: self.max_size,
        }
    }
}

#[derive(Debug)]
pub struct MemoryStorageStats {
    pub entry_count: usize,
    pub total_accesses: u64,
    pub max_size: usize,
}
