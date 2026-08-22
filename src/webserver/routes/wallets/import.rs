//! Bulk import handlers for wallet CSV/Excel uploads
//!
//! Handles file parsing, preview generation, and batch wallet imports.

use axum::{extract::Multipart, http::StatusCode, response::Response, Json};
use uuid::Uuid;

use crate::logger::{self, LogTag};
use crate::wallets::{
    self,
    bulk::{build_preview, detect_columns, parse_csv, parse_excel, ColumnMapping},
};
use crate::webserver::utils::{error_response, success_response};

use super::types::{
    ImportExecuteRequest, ImportPreviewResponse, ImportSession, IMPORT_SESSIONS, MAX_FILE_SIZE,
};
use super::utils::cleanup_expired_sessions;

// =============================================================================
// BULK IMPORT HANDLERS
// =============================================================================

/// Import preview - parse file and return preview with column mapping
///
/// POST /api/wallets/import/preview
/// Content-Type: multipart/form-data
/// - file: CSV or Excel file (.csv, .xlsx, .xls)
pub async fn import_preview(mut multipart: Multipart) -> Response {
    // Clean up expired sessions first
    cleanup_expired_sessions().await;

    // Extract file from multipart
    let mut file_data: Option<(String, Vec<u8>)> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("unknown").to_string();

            match field.bytes().await {
                Ok(bytes) => {
                    if bytes.len() > MAX_FILE_SIZE {
                        return error_response(
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "FILE_TOO_LARGE",
                            &format!(
                                "File exceeds maximum size of {}MB",
                                MAX_FILE_SIZE / 1024 / 1024
                            ),
                            None,
                        );
                    }
                    file_data = Some((filename, bytes.to_vec()));
                }
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "READ_ERROR",
                        "Failed to read uploaded file",
                        Some(&e.to_string()),
                    );
                }
            }
            break;
        }
    }

    let (filename, bytes) = match file_data {
        Some(data) => data,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "NO_FILE",
                "No file uploaded. Use 'file' field in multipart form",
                None,
            );
        }
    };

    // Determine file type from extension
    let extension = filename
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_lowercase();

    // Parse file based on extension
    let (headers, rows) = match extension.as_str() {
        "csv" => {
            let content = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "INVALID_ENCODING",
                        "CSV file must be UTF-8 encoded",
                        Some(&e.to_string()),
                    );
                }
            };

            match parse_csv(&content) {
                Ok(data) => data,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "PARSE_ERROR",
                        "Failed to parse CSV file",
                        Some(&e),
                    );
                }
            }
        }
        "xlsx" | "xls" | "xlsm" => match parse_excel(&bytes, None) {
            Ok(data) => data,
            Err(e) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "PARSE_ERROR",
                    "Failed to parse Excel file",
                    Some(&e),
                );
            }
        },
        _ => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_FORMAT",
                "Unsupported file format. Use .csv, .xlsx, or .xls",
                None,
            );
        }
    };

    if rows.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EMPTY_FILE",
            "File contains no data rows",
            None,
        );
    }

    // Auto-detect column mapping
    let detected_mapping = detect_columns(&headers);

    // Get existing addresses for duplicate detection
    let existing_addresses = match wallets::get_existing_wallet_addresses().await {
        Ok(addrs) => addrs,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to check existing wallets",
                Some(&e),
            );
        }
    };

    // Build preview
    let preview = build_preview(&headers, &rows, &detected_mapping, &existing_addresses);

    // Generate session ID and store data
    let session_id = Uuid::new_v4().to_string();

    {
        let mut sessions = IMPORT_SESSIONS.write().await;
        sessions.insert(
            session_id.clone(),
            ImportSession {
                headers: headers.clone(),
                rows,
                detected_mapping,
                created_at: std::time::Instant::now(),
            },
        );
    }

    logger::info(
        LogTag::Wallet,
        &format!(
            "Import preview created: session={}, rows={}, valid={}, invalid={}",
            &session_id[..8],
            preview.total_rows,
            preview.valid_count,
            preview.invalid_count
        ),
    );

    success_response(ImportPreviewResponse {
        session_id,
        preview,
    })
}

/// Execute bulk import with specified mapping
///
/// POST /api/wallets/import/execute
pub async fn import_execute(Json(request): Json<ImportExecuteRequest>) -> Response {
    // Validate mapping
    let mapping: ColumnMapping = (&request.mapping).into();
    if !mapping.is_valid() {
        let missing = mapping.missing_columns();
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_MAPPING",
            &format!("Missing required columns: {}", missing.join(", ")),
            None,
        );
    }

    // Get session data
    let session_data = {
        let sessions = IMPORT_SESSIONS.read().await;
        sessions.get(&request.session_id).map(|s| s.rows.clone())
    };

    let rows = match session_data {
        Some(data) => data,
        None => {
            return error_response(
                StatusCode::NOT_FOUND,
                "SESSION_NOT_FOUND",
                "Import session not found or expired. Please upload the file again",
                None,
            );
        }
    };

    // Get existing addresses
    let existing_addresses = match wallets::get_existing_wallet_addresses().await {
        Ok(addrs) => addrs,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to check existing wallets",
                Some(&e),
            );
        }
    };

    // Extract valid rows using the validator
    let parsed_rows =
        crate::wallets::bulk::validator::extract_valid_rows(&rows, &mapping, &existing_addresses);

    if parsed_rows.is_empty() {
        // Clean up session
        {
            let mut sessions = IMPORT_SESSIONS.write().await;
            sessions.remove(&request.session_id);
        }

        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_VALID_ROWS",
            "No valid rows to import",
            None,
        );
    }

    // Execute bulk import
    let result = wallets::bulk_import_wallets(parsed_rows, &request.options).await;

    // Clean up session
    {
        let mut sessions = IMPORT_SESSIONS.write().await;
        sessions.remove(&request.session_id);
    }

    logger::info(
        LogTag::Wallet,
        &format!(
            "Bulk import completed: total={}, success={}, failed={}, skipped={}",
            result.total_rows, result.success_count, result.failed_count, result.skipped_duplicates
        ),
    );

    success_response(result)
}
