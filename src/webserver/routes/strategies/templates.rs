use axum::response::Response;
use chrono::Utc;
use std::collections::HashMap;

use crate::webserver::utils::success_response;

use super::types::{StrategyTemplateItem, StrategyTemplatesResponse};

/// GET /api/strategies/templates - List available strategy templates
pub async fn list_templates() -> Response {
    // For now, load templates from DB if available; otherwise return empty list.
    // The DB schema exists; add a simple read using rusqlite directly here to avoid exposing internals.
    // We provide an empty response if not implemented in db module.
    let mut items: Vec<StrategyTemplateItem> = Vec::new();

    // Attempt to query templates table
    let result: Result<Vec<StrategyTemplateItem>, String> = (|| {
        // Open a read-only connection to strategies DB directly for listing templates
        let db_path = crate::paths::get_strategies_db_path();
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| format!("Failed to open strategies db: {}", e))?;

        // Apply centralized PRAGMA configuration
        crate::database::configure_connection(&conn, crate::database::STRATEGIES_DB)
            .map_err(|e| format!("Failed to configure connection: {}", e))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, category, risk_level, rules_json, parameters_json, created_at, author FROM strategy_templates ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Failed to prepare templates query: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                let rules_json: String = row.get(5)?;
                let params_json: String = row.get(6)?;
                let rules_val: serde_json::Value =
                    serde_json::from_str(&rules_json).unwrap_or(serde_json::Value::Null);
                let params_val: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&params_json).unwrap_or_default();
                Ok(StrategyTemplateItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    category: row.get(3)?,
                    risk_level: row.get::<_, String>(4)?,
                    rules: rules_val,
                    parameters: params_val,
                    created_at: row.get::<_, String>(7)?,
                    author: row.get(8)?,
                })
            })
            .map_err(|e| format!("Failed to query templates: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect templates: {}", e))?;
        Ok(rows)
    })();

    if let Ok(rows) = result {
        items = rows;
    }

    let total = items.len();
    let response = StrategyTemplatesResponse {
        items,
        total,
        timestamp: Utc::now().to_rfc3339(),
    };

    success_response(response)
}
