//! File parsers for bulk import
//!
//! Handles CSV and Excel file parsing with column detection.

use calamine::{open_workbook_auto_from_rs, Data, Reader};
use std::io::Cursor;

use super::types::ColumnMapping;

// =============================================================================
// CSV PARSING
// =============================================================================

/// Parse CSV content into headers and rows
///
/// Returns (headers, rows) where each row is a vector of cell values
pub fn parse_csv(content: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(content.as_bytes());

    // Parse headers
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("Failed to parse CSV headers: {}", e))?
        .iter()
        .map(|s| s.to_string())
        .collect();

    if headers.is_empty() {
        return Err("CSV file has no headers".to_string());
    }

    // Parse rows
    let mut rows = Vec::new();
    for (idx, result) in reader.records().enumerate() {
        let record = result.map_err(|e| format!("Failed to parse CSV row {}: {}", idx + 2, e))?;
        let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
        rows.push(row);
    }

    Ok((headers, rows))
}

// =============================================================================
// EXCEL PARSING
// =============================================================================

/// Parse Excel file bytes into headers and rows
///
/// Supports .xlsx, .xls, .xlsm formats via calamine
pub fn parse_excel(
    bytes: &[u8],
    sheet_name: Option<&str>,
) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let cursor = Cursor::new(bytes);
    let mut workbook = open_workbook_auto_from_rs(cursor)
        .map_err(|e| format!("Failed to open Excel file: {}", e))?;

    // Get sheet names
    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err("Excel file has no sheets".to_string());
    }

    // Select sheet
    let target_sheet = match sheet_name {
        Some(name) => {
            if !sheet_names.contains(&name.to_string()) {
                return Err(format!(
                    "Sheet '{}' not found. Available: {}",
                    name,
                    sheet_names.join(", ")
                ));
            }
            name.to_string()
        }
        None => sheet_names[0].clone(),
    };

    // Read sheet
    let range = workbook
        .worksheet_range(&target_sheet)
        .map_err(|e| format!("Failed to read sheet '{}': {}", target_sheet, e))?;

    if range.is_empty() {
        return Err("Sheet is empty".to_string());
    }

    // Convert to rows
    let mut all_rows: Vec<Vec<String>> = range
        .rows()
        .map(|row| {
            row.iter()
                .map(|cell| cell_to_string(cell))
                .collect()
        })
        .collect();

    if all_rows.is_empty() {
        return Err("No data found in sheet".to_string());
    }

    // First row is headers
    let headers = all_rows.remove(0);

    if headers.is_empty() || headers.iter().all(|h| h.is_empty()) {
        return Err("Excel file has no headers".to_string());
    }

    Ok((headers, all_rows))
}

/// Convert Excel cell to string
fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => {
            // Handle integers stored as floats
            if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 {
                (*f as i64).to_string()
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("ERROR: {:?}", e),
    }
}

// =============================================================================
// COLUMN DETECTION
// =============================================================================

/// Common column name patterns for auto-detection
const NAME_PATTERNS: &[&str] = &["name", "wallet_name", "walletname", "label", "alias"];
const PRIVATE_KEY_PATTERNS: &[&str] = &[
    "private_key",
    "privatekey",
    "private",
    "key",
    "secret",
    "secret_key",
    "secretkey",
];
const NOTES_PATTERNS: &[&str] = &["notes", "note", "description", "desc", "comment", "comments"];
const ADDRESS_PATTERNS: &[&str] = &["address", "public_key", "publickey", "pubkey", "wallet"];

/// Auto-detect column mapping from headers
pub fn detect_columns(headers: &[String]) -> ColumnMapping {
    let mut mapping = ColumnMapping::default();

    for (idx, header) in headers.iter().enumerate() {
        let lower = header.to_lowercase();
        let normalized = lower.replace([' ', '-', '_'], "");

        // Check each pattern type
        if mapping.name_col.is_none() && matches_pattern(&normalized, NAME_PATTERNS) {
            mapping.name_col = Some(idx);
        } else if mapping.private_key_col.is_none()
            && matches_pattern(&normalized, PRIVATE_KEY_PATTERNS)
        {
            mapping.private_key_col = Some(idx);
        } else if mapping.notes_col.is_none() && matches_pattern(&normalized, NOTES_PATTERNS) {
            mapping.notes_col = Some(idx);
        } else if mapping.address_col.is_none() && matches_pattern(&normalized, ADDRESS_PATTERNS) {
            mapping.address_col = Some(idx);
        }
    }

    mapping
}

/// Check if a normalized header matches any pattern
fn matches_pattern(normalized: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| {
        let pattern_normalized = p.replace([' ', '-', '_'], "");
        normalized == pattern_normalized || normalized.contains(&pattern_normalized)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_columns_basic() {
        let headers = vec![
            "Name".to_string(),
            "Private Key".to_string(),
            "Notes".to_string(),
        ];
        let mapping = detect_columns(&headers);
        assert_eq!(mapping.name_col, Some(0));
        assert_eq!(mapping.private_key_col, Some(1));
        assert_eq!(mapping.notes_col, Some(2));
        assert!(mapping.is_valid());
    }

    #[test]
    fn test_detect_columns_variations() {
        let headers = vec![
            "wallet_name".to_string(),
            "secret_key".to_string(),
            "description".to_string(),
            "address".to_string(),
        ];
        let mapping = detect_columns(&headers);
        assert_eq!(mapping.name_col, Some(0));
        assert_eq!(mapping.private_key_col, Some(1));
        assert_eq!(mapping.notes_col, Some(2));
        assert_eq!(mapping.address_col, Some(3));
    }

    #[test]
    fn test_parse_csv_basic() {
        let csv = "Name,Private Key,Notes\nWallet1,abc123,Test\nWallet2,def456,";
        let (headers, rows) = parse_csv(csv).unwrap();
        assert_eq!(headers, vec!["Name", "Private Key", "Notes"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["Wallet1", "abc123", "Test"]);
        assert_eq!(rows[1], vec!["Wallet2", "def456", ""]);
    }
}
