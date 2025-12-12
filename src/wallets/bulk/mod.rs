//! Bulk wallet import/export functionality
//!
//! Supports CSV and Excel file formats for importing/exporting multiple wallets.

mod parser;
mod types;
pub mod validator;

pub use parser::{detect_columns, parse_csv, parse_excel};
pub use types::{
    BulkImportResult, ColumnMapping, ImportOptions, ImportPreview, ImportRowResult,
    ParsedWalletRow, RowValidationResult, WalletExportRow,
};
pub use validator::{build_preview, extract_valid_rows, validate_row};
