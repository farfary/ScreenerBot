//! Type definitions for wallet API
//!
//! Response/request structs and query parameters for wallet endpoints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::wallets::{
    bulk::{ColumnMapping, ImportOptions, ImportPreview},
    Wallet,
};

// =============================================================================
// IMPORT SESSION STORAGE
// =============================================================================

/// Parsed import data stored in session
pub struct ImportSession {
    /// Parsed headers from file
    pub headers: Vec<String>,
    /// Parsed rows from file
    pub rows: Vec<Vec<String>>,
    /// Auto-detected column mapping
    pub detected_mapping: ColumnMapping,
    /// Session creation time
    pub created_at: std::time::Instant,
}

/// Global session storage for import operations
pub static IMPORT_SESSIONS: std::sync::LazyLock<RwLock<HashMap<String, ImportSession>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// Session expiry time (10 minutes)
pub const SESSION_EXPIRY_SECS: u64 = 600;

/// Max concurrent import sessions
pub const MAX_IMPORT_SESSIONS: usize = 10;

/// Max file size (2MB)
pub const MAX_FILE_SIZE: usize = 2 * 1024 * 1024;

// =============================================================================
// RESPONSE TYPES
// =============================================================================

#[derive(Serialize)]
pub struct WalletListResponse {
    pub wallets: Vec<Wallet>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct WalletCreatedResponse {
    pub message: String,
    pub wallet: Wallet,
}

#[derive(Serialize)]
pub struct SetMainResponse {
    pub message: String,
    pub wallet: Wallet,
}

#[derive(Serialize)]
pub struct DeleteResponse {
    pub message: String,
}

/// Response for import preview
#[derive(Serialize)]
pub struct ImportPreviewResponse {
    pub session_id: String,
    pub preview: ImportPreview,
}

/// Request for import execute
#[derive(Deserialize)]
pub struct ImportExecuteRequest {
    pub session_id: String,
    pub mapping: ColumnMappingRequest,
    pub options: ImportOptions,
}

/// Column mapping from client
#[derive(Deserialize)]
pub struct ColumnMappingRequest {
    pub name_col: Option<usize>,
    pub private_key_col: Option<usize>,
    pub notes_col: Option<usize>,
    pub address_col: Option<usize>,
}

impl From<&ColumnMappingRequest> for ColumnMapping {
    fn from(req: &ColumnMappingRequest) -> Self {
        ColumnMapping {
            name_col: req.name_col,
            private_key_col: req.private_key_col,
            notes_col: req.notes_col,
            address_col: req.address_col,
        }
    }
}

/// Request for full export with private keys
#[derive(Deserialize)]
pub struct FullExportRequest {
    pub wallet_ids: Vec<i64>,
    pub confirmation: String,
}

// =============================================================================
// QUERY PARAMS
// =============================================================================

#[derive(Deserialize)]
pub struct ListWalletsQuery {
    #[serde(default)]
    pub include_inactive: bool,
}

#[derive(Deserialize)]
pub struct ExportQuery {
    #[serde(default = "default_csv_format")]
    pub format: String,
    #[serde(default)]
    pub include_inactive: bool,
}

fn default_csv_format() -> String {
    "csv".to_owned()
}
