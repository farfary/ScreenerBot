//! Bulk import/export types
//!
//! Data structures for CSV/Excel wallet import/export operations.

use serde::{Deserialize, Serialize};

// =============================================================================
// PARSED DATA
// =============================================================================

/// A parsed wallet row ready for import
#[derive(Debug, Clone)]
pub struct ParsedWalletRow {
    /// Row number in source file (1-indexed)
    pub row_num: usize,
    /// Wallet name (required)
    pub name: String,
    /// Private key - base58 or JSON array format (required)
    pub private_key: String,
    /// Optional notes/description
    pub notes: Option<String>,
}

/// Export row for a wallet
#[derive(Debug, Clone, Serialize)]
pub struct WalletExportRow {
    /// Wallet name
    pub name: String,
    /// Wallet address (public key)
    pub address: String,
    /// Private key in base58 format
    pub private_key: String,
    /// Wallet role
    pub role: String,
    /// Whether this is the main wallet
    pub is_main: bool,
    /// Notes
    pub notes: String,
    /// Created timestamp
    pub created_at: String,
}

// =============================================================================
// COLUMN MAPPING
// =============================================================================

/// Column mapping for import files
#[derive(Debug, Clone, Default)]
pub struct ColumnMapping {
    /// Index of name column (required)
    pub name_col: Option<usize>,
    /// Index of private key column (required)
    pub private_key_col: Option<usize>,
    /// Index of notes column (optional)
    pub notes_col: Option<usize>,
    /// Index of address column (optional, for validation only)
    pub address_col: Option<usize>,
}

impl ColumnMapping {
    /// Check if required columns are mapped
    pub fn is_valid(&self) -> bool {
        self.name_col.is_some() && self.private_key_col.is_some()
    }

    /// Get list of missing required columns
    pub fn missing_columns(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.name_col.is_none() {
            missing.push("name");
        }
        if self.private_key_col.is_none() {
            missing.push("private_key");
        }
        missing
    }
}

// =============================================================================
// VALIDATION RESULTS
// =============================================================================

/// Result of validating a single row
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status")]
pub enum RowValidationResult {
    /// Row is valid and ready for import
    Valid {
        row_num: usize,
        name: String,
        address: String,
    },
    /// Row has validation errors
    Invalid { row_num: usize, errors: Vec<String> },
}

impl RowValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, RowValidationResult::Valid { .. })
    }

    pub fn row_num(&self) -> usize {
        match self {
            RowValidationResult::Valid { row_num, .. } => *row_num,
            RowValidationResult::Invalid { row_num, .. } => *row_num,
        }
    }
}

// =============================================================================
// IMPORT PREVIEW
// =============================================================================

/// Preview of import before execution
#[derive(Debug, Clone, Serialize)]
pub struct ImportPreview {
    /// Headers from the file
    pub headers: Vec<String>,
    /// Column mapping detected/configured
    pub column_mapping: ColumnMappingInfo,
    /// Total rows in file
    pub total_rows: usize,
    /// Number of valid rows
    pub valid_count: usize,
    /// Number of invalid rows
    pub invalid_count: usize,
    /// Number of duplicate rows (address already exists)
    pub duplicate_count: usize,
    /// Validation results for each row
    pub rows: Vec<RowValidationResult>,
}

/// Serializable column mapping info
#[derive(Debug, Clone, Serialize)]
pub struct ColumnMappingInfo {
    pub name_col: Option<usize>,
    pub private_key_col: Option<usize>,
    pub notes_col: Option<usize>,
    pub address_col: Option<usize>,
    pub is_valid: bool,
}

impl From<&ColumnMapping> for ColumnMappingInfo {
    fn from(mapping: &ColumnMapping) -> Self {
        Self {
            name_col: mapping.name_col,
            private_key_col: mapping.private_key_col,
            notes_col: mapping.notes_col,
            address_col: mapping.address_col,
            is_valid: mapping.is_valid(),
        }
    }
}

// =============================================================================
// IMPORT OPTIONS
// =============================================================================

/// Options for bulk import operation
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ImportOptions {
    /// Skip rows with validation errors (otherwise abort on first error)
    #[serde(default)]
    pub skip_invalid: bool,
    /// Skip duplicate addresses (otherwise report as error)
    #[serde(default)]
    pub skip_duplicates: bool,
    /// Set the first imported wallet as main (if no main exists)
    #[serde(default)]
    pub set_first_as_main: bool,
}

// =============================================================================
// IMPORT RESULTS
// =============================================================================

/// Result of a single row import
#[derive(Debug, Clone, Serialize)]
pub struct ImportRowResult {
    /// Row number in source file
    pub row_num: usize,
    /// Wallet name
    pub name: String,
    /// Wallet address (if successful)
    pub address: Option<String>,
    /// Whether import was successful
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Result of bulk import operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkImportResult {
    /// Total rows processed
    pub total_rows: usize,
    /// Number of successful imports
    pub success_count: usize,
    /// Number of failed imports
    pub failed_count: usize,
    /// Number of skipped duplicates
    pub skipped_duplicates: usize,
    /// Results for each row
    pub rows: Vec<ImportRowResult>,
    /// IDs of successfully imported wallets
    pub imported_wallet_ids: Vec<i64>,
}
