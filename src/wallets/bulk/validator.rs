//! Row validation for bulk import
//!
//! Validates wallet data before import, checking keys and duplicates.

use std::collections::HashSet;

use super::types::{
    ColumnMapping, ColumnMappingInfo, ImportPreview, ParsedWalletRow, RowValidationResult,
};
use crate::wallets::crypto::{keypair_to_address, parse_private_key, validate_address};

// =============================================================================
// ROW VALIDATION
// =============================================================================

/// Validate a single row from the import file
///
/// Checks:
/// - Required fields are present
/// - Private key is valid format (base58 or JSON array)
/// - Private key produces valid keypair
/// - Address doesn't already exist (if existing_addresses provided)
/// - If address column present, it matches derived address
pub fn validate_row(
    row: &[String],
    row_num: usize,
    mapping: &ColumnMapping,
    existing_addresses: &HashSet<String>,
) -> RowValidationResult {
    let mut errors = Vec::new();

    // Get name
    let name = mapping
        .name_col
        .and_then(|idx| row.get(idx))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if name.is_empty() {
        errors.push("Name is required".to_string());
    }

    // Get private key
    let private_key = mapping
        .private_key_col
        .and_then(|idx| row.get(idx))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if private_key.is_empty() {
        errors.push("Private key is required".to_string());
        return RowValidationResult::Invalid { row_num, errors };
    }

    // Validate and derive address from private key
    let address = match parse_private_key(&private_key) {
        Ok(keypair) => keypair_to_address(&keypair),
        Err(e) => {
            // Don't include the actual key in error message
            errors.push(format!("Invalid private key: {}", e));
            return RowValidationResult::Invalid { row_num, errors };
        }
    };

    // Check if address column is present and matches
    if let Some(addr_col) = mapping.address_col {
        if let Some(expected_addr) = row.get(addr_col) {
            let expected_addr = expected_addr.trim();
            if !expected_addr.is_empty() {
                // Validate expected address format
                if let Err(e) = validate_address(expected_addr) {
                    errors.push(format!("Invalid address in file: {}", e));
                } else if expected_addr != address {
                    errors.push(format!(
                        "Address mismatch: file has {}, but key derives to {}",
                        truncate_address(expected_addr),
                        truncate_address(&address)
                    ));
                }
            }
        }
    }

    // Check for duplicates
    if existing_addresses.contains(&address) {
        errors.push(format!(
            "Wallet {} already exists",
            truncate_address(&address)
        ));
    }

    if errors.is_empty() {
        RowValidationResult::Valid {
            row_num,
            name,
            address,
        }
    } else {
        RowValidationResult::Invalid { row_num, errors }
    }
}

/// Truncate address for display (first 8...last 4)
fn truncate_address(addr: &str) -> String {
    if addr.len() > 16 {
        format!("{}...{}", &addr[..8], &addr[addr.len() - 4..])
    } else {
        addr.to_string()
    }
}

// =============================================================================
// PREVIEW BUILDING
// =============================================================================

/// Build an import preview with all rows validated
///
/// This allows users to review before committing the import
pub fn build_preview(
    headers: &[String],
    rows: &[Vec<String>],
    mapping: &ColumnMapping,
    existing_addresses: &HashSet<String>,
) -> ImportPreview {
    let mut validation_results = Vec::with_capacity(rows.len());
    let mut valid_count = 0;
    let mut invalid_count = 0;
    let mut duplicate_count = 0;
    let mut seen_addresses = existing_addresses.clone();

    for (idx, row) in rows.iter().enumerate() {
        let row_num = idx + 2; // 1-indexed, skip header row
        let result = validate_row(row, row_num, mapping, &seen_addresses);

        match &result {
            RowValidationResult::Valid { address, .. } => {
                valid_count += 1;
                // Track address to detect duplicates within file
                seen_addresses.insert(address.clone());
            }
            RowValidationResult::Invalid { errors, .. } => {
                invalid_count += 1;
                // Check if it's specifically a duplicate error
                if errors.iter().any(|e| e.contains("already exists")) {
                    duplicate_count += 1;
                }
            }
        }

        validation_results.push(result);
    }

    ImportPreview {
        headers: headers.to_vec(),
        column_mapping: ColumnMappingInfo::from(mapping),
        total_rows: rows.len(),
        valid_count,
        invalid_count,
        duplicate_count,
        rows: validation_results,
    }
}

// =============================================================================
// EXTRACT PARSED ROWS
// =============================================================================

/// Extract valid parsed rows from raw data
///
/// Returns only rows that pass validation, ready for import
pub fn extract_valid_rows(
    rows: &[Vec<String>],
    mapping: &ColumnMapping,
    existing_addresses: &HashSet<String>,
) -> Vec<ParsedWalletRow> {
    let mut result = Vec::new();
    let mut seen_addresses = existing_addresses.clone();

    for (idx, row) in rows.iter().enumerate() {
        let row_num = idx + 2;
        let validation = validate_row(row, row_num, mapping, &seen_addresses);

        if let RowValidationResult::Valid { name, address, .. } = validation {
            // Get private key and notes
            let private_key = mapping
                .private_key_col
                .and_then(|col| row.get(col))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let notes = mapping
                .notes_col
                .and_then(|col| row.get(col))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            seen_addresses.insert(address);
            result.push(ParsedWalletRow {
                row_num,
                name,
                private_key,
                notes,
            });
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mapping() -> ColumnMapping {
        ColumnMapping {
            name_col: Some(0),
            private_key_col: Some(1),
            notes_col: Some(2),
            address_col: None,
        }
    }

    #[test]
    fn test_validate_row_missing_name() {
        let row = vec!["".to_string(), "somekey".to_string(), "notes".to_string()];
        let result = validate_row(&row, 1, &test_mapping(), &HashSet::new());
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validate_row_missing_key() {
        let row = vec!["Wallet".to_string(), "".to_string(), "notes".to_string()];
        let result = validate_row(&row, 1, &test_mapping(), &HashSet::new());
        assert!(!result.is_valid());
    }

    #[test]
    fn test_truncate_address() {
        let addr = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
        let truncated = truncate_address(addr);
        assert_eq!(truncated, "7xKXtg2C...sAsU");
    }
}
