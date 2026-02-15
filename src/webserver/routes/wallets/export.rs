//! Export handlers for wallet data
//!
//! CSV export endpoints - basic (no keys) and full (with private keys).

use axum::{
    body::Body,
    extract::Query,
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::wallets::{self, bulk::WalletExportRow};
use crate::webserver::utils::error_response;

use super::types::{ExportQuery, FullExportRequest};
use super::utils::escape_csv_field;

// =============================================================================
// BULK EXPORT HANDLERS
// =============================================================================

/// Export wallets to CSV (without private keys)
///
/// GET /api/wallets/export?format=csv&include_inactive=false
pub async fn export_wallets_csv(Query(query): Query<ExportQuery>) -> impl IntoResponse {
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
        &format!(
            "Exported {} wallets to CSV (no private keys)",
            wallets.len()
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

/// Export wallets with private keys (security-sensitive)
///
/// POST /api/wallets/export/full
/// Requires confirmation string and logs the operation
pub async fn export_wallets_full(Json(request): Json<FullExportRequest>) -> impl IntoResponse {
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
    let mut csv_content = String::from("name,address,private_key,role,is_main,notes,created_at\n");

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
