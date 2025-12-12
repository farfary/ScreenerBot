//! Wallet management API routes
//!
//! CRUD endpoints for multi-wallet management.
//! Includes bulk import/export with CSV and Excel support.

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::logger::{self, LogTag};
use crate::wallets::{
    self,
    bulk::{
        build_preview, detect_columns, parse_csv, parse_excel, BulkImportResult, ColumnMapping,
        ImportOptions, ImportPreview, ParsedWalletRow, WalletExportRow,
    },
    CreateWalletRequest, ExportWalletResponse, ImportWalletRequest, UpdateWalletRequest, Wallet,
    WalletsSummary,
};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

// =============================================================================
// IMPORT SESSION STORAGE
// =============================================================================

/// Parsed import data stored in session
struct ImportSession {
    /// Parsed headers from file
    headers: Vec<String>,
    /// Parsed rows from file
    rows: Vec<Vec<String>>,
    /// Auto-detected column mapping
    detected_mapping: ColumnMapping,
    /// Session creation time
    created_at: std::time::Instant,
}

/// Global session storage for import operations
static IMPORT_SESSIONS: once_cell::sync::Lazy<RwLock<HashMap<String, ImportSession>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Session expiry time (10 minutes)
const SESSION_EXPIRY_SECS: u64 = 600;

/// Max concurrent import sessions
const MAX_IMPORT_SESSIONS: usize = 10;

/// Max file size (2MB)
const MAX_FILE_SIZE: usize = 2 * 1024 * 1024;

// =============================================================================
// RESPONSE TYPES
// =============================================================================

#[derive(Serialize)]
struct WalletListResponse {
    wallets: Vec<Wallet>,
    total: usize,
}

#[derive(Serialize)]
struct WalletCreatedResponse {
    message: String,
    wallet: Wallet,
}

#[derive(Serialize)]
struct SetMainResponse {
    message: String,
    wallet: Wallet,
}

#[derive(Serialize)]
struct DeleteResponse {
    message: String,
}

/// Response for import preview
#[derive(Serialize)]
struct ImportPreviewResponse {
    session_id: String,
    preview: ImportPreview,
}

/// Request for import execute
#[derive(Deserialize)]
struct ImportExecuteRequest {
    session_id: String,
    mapping: ColumnMappingRequest,
    options: ImportOptions,
}

/// Column mapping from client
#[derive(Deserialize)]
struct ColumnMappingRequest {
    name_col: Option<usize>,
    private_key_col: Option<usize>,
    notes_col: Option<usize>,
    address_col: Option<usize>,
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
struct FullExportRequest {
    wallet_ids: Vec<i64>,
    confirmation: String,
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
    "csv".to_string()
}

// =============================================================================
// ROUTES
// =============================================================================

/// Create wallet routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_wallets))
        .route("/", post(create_wallet))
        .route("/import", post(import_wallet))
        .route("/import/preview", post(import_preview))
        .route("/import/execute", post(import_execute))
        .route("/export", get(export_wallets_csv))
        .route("/export/full", post(export_wallets_full))
        .route("/summary", get(get_summary))
        .route("/main", get(get_main_wallet))
        .route("/:id", get(get_wallet))
        .route("/:id", put(update_wallet))
        .route("/:id", delete(delete_wallet))
        .route("/:id/export", post(export_wallet))
        .route("/:id/set-main", post(set_main_wallet))
        .route("/:id/archive", post(archive_wallet))
        .route("/:id/restore", post(restore_wallet))
}

// =============================================================================
// HANDLERS
// =============================================================================

/// List all wallets
async fn list_wallets(Query(query): Query<ListWalletsQuery>) -> Response {
    match wallets::list_wallets(query.include_inactive).await {
        Ok(wallets) => {
            let total = wallets.len();
            success_response(WalletListResponse { wallets, total })
        }
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to list wallets: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_ERROR",
                "Failed to list wallets",
                Some(&e),
            )
        }
    }
}

/// Create a new wallet
async fn create_wallet(Json(request): Json<CreateWalletRequest>) -> Response {
    // Validate name
    if request.name.trim().is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_NAME",
            "Wallet name cannot be empty",
            None,
        );
    }

    match wallets::create_wallet(request).await {
        Ok(wallet) => success_response(WalletCreatedResponse {
            message: format!("Wallet '{}' created successfully", wallet.name),
            wallet,
        }),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to create wallet: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "CREATE_ERROR",
                "Failed to create wallet",
                Some(&e),
            )
        }
    }
}

/// Import an existing wallet
async fn import_wallet(Json(request): Json<ImportWalletRequest>) -> Response {
    // Validate name
    if request.name.trim().is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_NAME",
            "Wallet name cannot be empty",
            None,
        );
    }

    // Validate private key is provided
    if request.private_key.trim().is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_KEY",
            "Private key cannot be empty",
            None,
        );
    }

    match wallets::import_wallet(request).await {
        Ok(wallet) => success_response(WalletCreatedResponse {
            message: format!("Wallet '{}' imported successfully", wallet.name),
            wallet,
        }),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to import wallet: {}", e));

            // Check for specific error types
            let (status, code, msg) = if e.contains("already exists") {
                (
                    axum::http::StatusCode::CONFLICT,
                    "DUPLICATE",
                    "Wallet already exists",
                )
            } else if e.contains("Invalid") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "INVALID_KEY",
                    "Invalid private key format",
                )
            } else {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "IMPORT_ERROR",
                    "Failed to import wallet",
                )
            };

            error_response(status, code, msg, Some(&e))
        }
    }
}

/// Get wallet summary for dashboard
async fn get_summary() -> Response {
    match wallets::get_wallets_summary().await {
        Ok(summary) => success_response(summary),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get summary: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SUMMARY_ERROR",
                "Failed to get wallets summary",
                Some(&e),
            )
        }
    }
}

/// Get main wallet info
async fn get_main_wallet() -> Response {
    match wallets::get_main_wallet().await {
        Ok(Some(wallet)) => success_response(wallet),
        Ok(None) => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "NO_MAIN_WALLET",
            "No main wallet configured",
            None,
        ),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get main wallet: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "MAIN_WALLET_ERROR",
                "Failed to get main wallet",
                Some(&e),
            )
        }
    }
}

/// Get a specific wallet by ID
async fn get_wallet(Path(id): Path<i64>) -> Response {
    match wallets::get_wallet(id).await {
        Ok(Some(wallet)) => success_response(wallet),
        Ok(None) => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Wallet not found",
            None,
        ),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get wallet {}: {}", id, e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "GET_ERROR",
                "Failed to get wallet",
                Some(&e),
            )
        }
    }
}

/// Update wallet metadata
async fn update_wallet(Path(id): Path<i64>, Json(request): Json<UpdateWalletRequest>) -> Response {
    match wallets::update_wallet(id, request).await {
        Ok(wallet) => success_response(wallet),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to update wallet {}: {}", id, e),
            );
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "UPDATE_ERROR",
                "Failed to update wallet",
                Some(&e),
            )
        }
    }
}

/// Delete a wallet permanently
async fn delete_wallet(Path(id): Path<i64>) -> Response {
    match wallets::delete_wallet(id).await {
        Ok(()) => success_response(DeleteResponse {
            message: "Wallet deleted successfully".to_string(),
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to delete wallet {}: {}", id, e),
            );

            let (status, code) = if e.contains("main wallet") {
                (axum::http::StatusCode::BAD_REQUEST, "MAIN_WALLET")
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "DELETE_ERROR")
            };

            error_response(status, code, "Failed to delete wallet", Some(&e))
        }
    }
}

/// Export wallet private key
async fn export_wallet(Path(id): Path<i64>) -> Response {
    match wallets::export_wallet(id).await {
        Ok(export) => success_response(export),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to export wallet {}: {}", id, e),
            );
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "EXPORT_ERROR",
                "Failed to export wallet",
                Some(&e),
            )
        }
    }
}

/// Set a wallet as the main wallet
async fn set_main_wallet(Path(id): Path<i64>) -> Response {
    match wallets::set_main_wallet(id).await {
        Ok(wallet) => success_response(SetMainResponse {
            message: format!("'{}' is now the main wallet", wallet.name),
            wallet,
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to set main wallet {}: {}", id, e),
            );
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SET_MAIN_ERROR",
                "Failed to set main wallet",
                Some(&e),
            )
        }
    }
}

/// Archive a wallet (soft delete)
async fn archive_wallet(Path(id): Path<i64>) -> Response {
    match wallets::archive_wallet(id).await {
        Ok(()) => success_response(DeleteResponse {
            message: "Wallet archived successfully".to_string(),
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to archive wallet {}: {}", id, e),
            );

            let (status, code) = if e.contains("main wallet") {
                (axum::http::StatusCode::BAD_REQUEST, "MAIN_WALLET")
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "ARCHIVE_ERROR")
            };

            error_response(status, code, "Failed to archive wallet", Some(&e))
        }
    }
}

/// Restore an archived wallet
async fn restore_wallet(Path(id): Path<i64>) -> Response {
    match wallets::restore_wallet(id).await {
        Ok(()) => success_response(DeleteResponse {
            message: "Wallet restored successfully".to_string(),
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to restore wallet {}: {}", id, e),
            );

            let (status, code) = if e.contains("not archived") {
                (axum::http::StatusCode::BAD_REQUEST, "NOT_ARCHIVED")
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "RESTORE_ERROR")
            };

            error_response(status, code, "Failed to restore wallet", Some(&e))
        }
    }
}

// =============================================================================
// BULK IMPORT HANDLERS
// =============================================================================

/// Clean up expired sessions and enforce session limit
async fn cleanup_expired_sessions() {
    let mut sessions = IMPORT_SESSIONS.write().await;
    let now = std::time::Instant::now();

    // Remove expired sessions
    sessions.retain(|_, session| {
        now.duration_since(session.created_at).as_secs() < SESSION_EXPIRY_SECS
    });

    // Warn if approaching session limit
    if sessions.len() >= MAX_IMPORT_SESSIONS {
        crate::logger::warning(
            crate::logger::LogTag::Webserver,
            &format!(
                "Import session limit reached ({}/{}). Oldest sessions will be dropped.",
                sessions.len(),
                MAX_IMPORT_SESSIONS
            ),
        );

        // Remove oldest sessions to stay under limit
        while sessions.len() >= MAX_IMPORT_SESSIONS {
            // Find oldest session
            if let Some(oldest_id) = sessions
                .iter()
                .min_by_key(|(_, s)| s.created_at)
                .map(|(id, _)| id.clone())
            {
                sessions.remove(&oldest_id);
            } else {
                break;
            }
        }
    }
}

/// Import preview - parse file and return preview with column mapping
///
/// POST /api/wallets/import/preview
/// Content-Type: multipart/form-data
/// - file: CSV or Excel file (.csv, .xlsx, .xls)
async fn import_preview(mut multipart: Multipart) -> Response {
    // Clean up expired sessions first
    cleanup_expired_sessions().await;

    // Extract file from multipart
    let mut file_data: Option<(String, Vec<u8>)> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("unknown").to_string();

            match field.bytes().await {
                Ok(bytes) => {
                    if bytes.len() > MAX_FILE_SIZE {
                        return error_response(
                            axum::http::StatusCode::PAYLOAD_TOO_LARGE,
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
                        axum::http::StatusCode::BAD_REQUEST,
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
                axum::http::StatusCode::BAD_REQUEST,
                "NO_FILE",
                "No file uploaded. Use 'file' field in multipart form",
                None,
            );
        }
    };

    // Determine file type from extension
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    // Parse file based on extension
    let (headers, rows) = match extension.as_str() {
        "csv" => {
            let content = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => {
                    return error_response(
                        axum::http::StatusCode::BAD_REQUEST,
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
                        axum::http::StatusCode::BAD_REQUEST,
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
                    axum::http::StatusCode::BAD_REQUEST,
                    "PARSE_ERROR",
                    "Failed to parse Excel file",
                    Some(&e),
                );
            }
        },
        _ => {
            return error_response(
                axum::http::StatusCode::BAD_REQUEST,
                "INVALID_FORMAT",
                "Unsupported file format. Use .csv, .xlsx, or .xls",
                None,
            );
        }
    };

    if rows.is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
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
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
async fn import_execute(Json(request): Json<ImportExecuteRequest>) -> Response {
    // Validate mapping
    let mapping: ColumnMapping = (&request.mapping).into();
    if !mapping.is_valid() {
        let missing = mapping.missing_columns();
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
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
                axum::http::StatusCode::NOT_FOUND,
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
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
            axum::http::StatusCode::BAD_REQUEST,
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

// =============================================================================
// BULK EXPORT HANDLERS
// =============================================================================

/// Export wallets to CSV (without private keys)
///
/// GET /api/wallets/export?format=csv&include_inactive=false
async fn export_wallets_csv(Query(query): Query<ExportQuery>) -> impl IntoResponse {
    if query.format != "csv" {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_FORMAT",
            "Only CSV format is currently supported",
            None,
        )
        .into_response();
    }

    // Get wallets (without private keys for basic export)
    let wallets = match wallets::list_wallets(query.include_inactive).await {
        Ok(w) => w,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_ERROR",
                "Failed to list wallets",
                Some(&e),
            )
            .into_response();
        }
    };

    // Build CSV content
    let mut csv_content = String::from("name,address,role,is_main,is_active,notes,created_at\n");

    for wallet in &wallets {
        let notes = wallet.notes.as_deref().unwrap_or("");
        // Escape CSV fields that might contain commas or quotes
        let escaped_name = escape_csv_field(&wallet.name);
        let escaped_notes = escape_csv_field(notes);

        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            escaped_name,
            wallet.address,
            wallet.role,
            wallet.is_main(),
            wallet.is_active,
            escaped_notes,
            wallet.created_at.format("%Y-%m-%d %H:%M:%S")
        ));
    }

    let filename = format!(
        "screenerbot_wallets_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    logger::info(
        LogTag::Wallet,
        &format!("Exported {} wallets to CSV (no private keys)", wallets.len()),
    );

    (
        axum::http::StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        csv_content,
    )
        .into_response()
}

/// Export wallets with private keys (security-sensitive)
///
/// POST /api/wallets/export/full
/// Requires confirmation string and logs the operation
async fn export_wallets_full(Json(request): Json<FullExportRequest>) -> impl IntoResponse {
    // Require explicit confirmation
    const REQUIRED_CONFIRMATION: &str = "I understand the risks";

    if request.confirmation != REQUIRED_CONFIRMATION {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "CONFIRMATION_REQUIRED",
            &format!(
                "You must confirm by providing: \"{}\"",
                REQUIRED_CONFIRMATION
            ),
            None,
        )
        .into_response();
    }

    if request.wallet_ids.is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "NO_WALLETS",
            "No wallet IDs provided",
            None,
        )
        .into_response();
    }

    // Log this security-sensitive operation
    logger::warning(
        LogTag::Wallet,
        &format!(
            "SECURITY: Full wallet export requested for {} wallets - INCLUDES PRIVATE KEYS",
            request.wallet_ids.len()
        ),
    );

    // Get all exportable wallets with private keys
    let all_exports = match wallets::export_wallets(true).await {
        Ok(exports) => exports,
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to export wallets: {}", e));
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "EXPORT_ERROR",
                "Failed to export wallets",
                Some(&e),
            )
            .into_response();
        }
    };

    // Get wallets to match IDs
    let wallets_list = match wallets::list_wallets(true).await {
        Ok(w) => w,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_ERROR",
                "Failed to list wallets",
                Some(&e),
            )
            .into_response();
        }
    };

    // Create address to ID mapping
    let address_to_id: HashMap<&str, i64> = wallets_list
        .iter()
        .map(|w| (w.address.as_str(), w.id))
        .collect();

    // Filter exports to only requested wallet IDs
    let requested_ids: std::collections::HashSet<i64> =
        request.wallet_ids.iter().copied().collect();

    let filtered_exports: Vec<&WalletExportRow> = all_exports
        .iter()
        .filter(|export| {
            address_to_id
                .get(export.address.as_str())
                .map(|id| requested_ids.contains(id))
                .unwrap_or(false)
        })
        .collect();

    if filtered_exports.is_empty() {
        return error_response(
            axum::http::StatusCode::NOT_FOUND,
            "NO_MATCHING_WALLETS",
            "No wallets found matching the provided IDs",
            None,
        )
        .into_response();
    }

    // Build CSV with private keys
    let mut csv_content =
        String::from("name,address,private_key,role,is_main,notes,created_at\n");

    for export in &filtered_exports {
        let escaped_name = escape_csv_field(&export.name);
        let escaped_notes = escape_csv_field(&export.notes);

        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            escaped_name,
            export.address,
            export.private_key,
            export.role,
            export.is_main,
            escaped_notes,
            export.created_at
        ));
    }

    let filename = format!(
        "screenerbot_wallets_FULL_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    logger::warning(
        LogTag::Wallet,
        &format!(
            "SECURITY: Exported {} wallets WITH PRIVATE KEYS",
            filtered_exports.len()
        ),
    );

    (
        axum::http::StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        csv_content,
    )
        .into_response()
}

/// Escape a field for CSV output
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}
