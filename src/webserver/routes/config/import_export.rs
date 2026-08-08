//! Config API Import/Export
//!
//! Endpoints for importing and exporting configuration files.

use axum::{http::StatusCode, response::Response, Json};

use crate::config;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Sanitize a section by removing/masking sensitive fields
fn sanitize_section(section_name: &str, value: &mut serde_json::Value) {
    for (section, fields) in SENSITIVE_FIELDS {
        if *section != section_name {
            continue;
        }
        for field_path in *fields {
            remove_nested_field(value, field_path);
        }
    }
}

/// Remove a nested field by dot-separated path (e.g., "dashboard.lockscreen.password_hash")
fn remove_nested_field(value: &mut serde_json::Value, path: &str) {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return;
    }

    let mut current = value;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - remove the field
            if let Some(obj) = current.as_object_mut() {
                obj.remove(*part);
            }
        } else {
            // Navigate to nested object
            if let Some(obj) = current.as_object_mut() {
                if let Some(next) = obj.get_mut(*part) {
                    current = next;
                } else {
                    return; // Path not found
                }
            } else {
                return; // Not an object
            }
        }
    }
}

/// Check if a nested field exists by dot-separated path
fn has_nested_field(value: &serde_json::Value, path: &str) -> bool {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return false;
    }

    let mut current = value;
    for (i, part) in parts.iter().enumerate() {
        if let Some(obj) = current.as_object() {
            if i == parts.len() - 1 {
                // Last part - check if field exists and is non-empty
                if let Some(val) = obj.get(*part) {
                    return !val.is_null()
                        && !(val.is_string() && val.as_str().unwrap_or_default().is_empty());
                }
                return false;
            } else if let Some(next) = obj.get(*part) {
                current = next;
            } else {
                return false;
            }
        } else {
            return false;
        }
    }
    false
}

/// Helper to get section label for display
fn get_section_label(section: &str) -> String {
    match section {
        "rpc" => "RPC".to_owned(),
        "trader" => "Auto Trader".to_owned(),
        "copy_trading" => "Wallet Copy".to_owned(),
        "positions" => "Positions".to_owned(),
        "filtering" => "Filtering".to_owned(),
        "swaps" => "Swaps".to_owned(),
        "tokens" => "Tokens".to_owned(),
        "sol_price" => "SOL Price".to_owned(),
        "network" => "Network".to_owned(),
        "events" => "Events".to_owned(),
        "services" => "Services".to_owned(),
        "monitoring" => "Monitoring".to_owned(),
        "ohlcv" => "OHLCV".to_owned(),
        "gui" => "GUI".to_owned(),
        "telegram" => "Telegram".to_owned(),
        _ => section.to_string(),
    }
}

/// Count fields in a JSON value (recursive for objects)
fn count_fields(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => map.len(),
        _ => 0,
    }
}

/// Compare two JSON values and return field changes
fn compare_values(
    current: &serde_json::Value,
    imported: &serde_json::Value,
    prefix: &str,
) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    if let (Some(curr_obj), Some(imp_obj)) = (current.as_object(), imported.as_object()) {
        for (key, imp_val) in imp_obj {
            let field_path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };

            match curr_obj.get(key) {
                Some(curr_val) => {
                    if curr_val != imp_val {
                        // Check if both are objects for recursive comparison
                        if curr_val.is_object() && imp_val.is_object() {
                            changes.extend(compare_values(curr_val, imp_val, &field_path));
                        } else {
                            changes.push(FieldChange {
                                field: field_path,
                                current: curr_val.clone(),
                                imported: imp_val.clone(),
                            });
                        }
                    }
                }
                None => {
                    // New field being added
                    changes.push(FieldChange {
                        field: field_path,
                        current: serde_json::Value::Null,
                        imported: imp_val.clone(),
                    });
                }
            }
        }
    }

    changes
}

/// Helper to apply a section value to a config struct (used for validation before commit)
fn apply_section_to_config(
    cfg: &mut config::Config,
    section: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    match section {
        "rpc" => {
            cfg.rpc =
                serde_json::from_value(value).map_err(|e| format!("Invalid RpcConfig: {e}"))?;
        }
        "trader" => {
            cfg.trader =
                serde_json::from_value(value).map_err(|e| format!("Invalid TraderConfig: {e}"))?;
        }
        "copy_trading" => {
            let copy: config::CopyTradingConfig = serde_json::from_value(value)
                .map_err(|e| format!("Invalid CopyTradingConfig: {e}"))?;
            copy.validate()?;
            cfg.copy_trading = copy;
        }
        "positions" => {
            cfg.positions = serde_json::from_value(value)
                .map_err(|e| format!("Invalid PositionsConfig: {e}"))?;
        }
        "filtering" => {
            cfg.filtering = serde_json::from_value(value)
                .map_err(|e| format!("Invalid FilteringConfig: {e}"))?;
        }
        "swaps" => {
            cfg.swaps =
                serde_json::from_value(value).map_err(|e| format!("Invalid SwapsConfig: {e}"))?;
        }
        "tokens" => {
            cfg.tokens =
                serde_json::from_value(value).map_err(|e| format!("Invalid TokensConfig: {e}"))?;
        }
        "sol_price" => {
            cfg.sol_price = serde_json::from_value(value)
                .map_err(|e| format!("Invalid SolPriceConfig: {e}"))?;
        }
        "network" => {
            cfg.network =
                serde_json::from_value(value).map_err(|e| format!("Invalid NetworkConfig: {e}"))?;
        }
        "events" => {
            cfg.events =
                serde_json::from_value(value).map_err(|e| format!("Invalid EventsConfig: {e}"))?;
        }
        "services" => {
            cfg.services = serde_json::from_value(value)
                .map_err(|e| format!("Invalid ServicesConfig: {e}"))?;
        }
        "monitoring" => {
            cfg.monitoring = serde_json::from_value(value)
                .map_err(|e| format!("Invalid MonitoringConfig: {e}"))?;
        }
        "ohlcv" => {
            cfg.ohlcv =
                serde_json::from_value(value).map_err(|e| format!("Invalid OhlcvConfig: {e}"))?;
        }
        "gui" => {
            cfg.gui =
                serde_json::from_value(value).map_err(|e| format!("Invalid GuiConfig: {e}"))?;
        }
        "telegram" => {
            cfg.telegram = serde_json::from_value(value)
                .map_err(|e| format!("Invalid TelegramConfig: {e}"))?;
        }
        _ => return Err(format!("Unknown section: {section}")),
    }
    Ok(())
}

// ============================================================================
// EXPORT ENDPOINT
// ============================================================================

/// POST /api/config/export - Export configuration with options
pub async fn export_config(Json(request): Json<ExportConfigRequest>) -> Response {
    // Determine which sections to export
    let sections_to_export: Vec<&str> = match &request.sections {
        Some(sections) if !sections.is_empty() => sections
            .iter()
            .filter(|s| CONFIG_SECTIONS.contains(&s.as_str()))
            .map(String::as_str)
            .collect(),
        _ => CONFIG_SECTIONS.to_vec(),
    };

    // Filter out GUI if requested
    let sections_to_export: Vec<&str> = if !request.include_gui {
        sections_to_export
            .into_iter()
            .filter(|s| *s != "gui")
            .collect()
    } else {
        sections_to_export
    };

    // Build the export object
    let mut export_obj = serde_json::Map::new();
    let sanitize = request.sanitize_secrets;

    config::with_config(|cfg| {
        for section in &sections_to_export {
            let section_value = match *section {
                "rpc" => serde_json::to_value(&cfg.rpc).ok(),
                "trader" => serde_json::to_value(&cfg.trader).ok(),
                "copy_trading" => serde_json::to_value(&cfg.copy_trading).ok(),
                "positions" => serde_json::to_value(&cfg.positions).ok(),
                "filtering" => serde_json::to_value(&cfg.filtering).ok(),
                "swaps" => serde_json::to_value(&cfg.swaps).ok(),
                "tokens" => serde_json::to_value(&cfg.tokens).ok(),
                "sol_price" => serde_json::to_value(&cfg.sol_price).ok(),
                "network" => serde_json::to_value(&cfg.network).ok(),
                "events" => serde_json::to_value(&cfg.events).ok(),
                "services" => serde_json::to_value(&cfg.services).ok(),
                "monitoring" => serde_json::to_value(&cfg.monitoring).ok(),
                "ohlcv" => serde_json::to_value(&cfg.ohlcv).ok(),
                "gui" => serde_json::to_value(&cfg.gui).ok(),
                "telegram" => serde_json::to_value(&cfg.telegram).ok(),
                _ => None,
            };

            if let Some(mut value) = section_value {
                // Sanitize sensitive fields if requested
                if sanitize {
                    sanitize_section(section, &mut value);
                }
                export_obj.insert(section.to_string(), value);
            }
        }
    });

    // Add metadata if requested
    if request.include_metadata {
        export_obj.insert(
            "timestamp".to_owned(),
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
        );
    }

    success_response(ExportConfigResponse {
        config: serde_json::Value::Object(export_obj),
        sections: sections_to_export.iter().map(|s| s.to_string()).collect(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// ============================================================================
// IMPORT PREVIEW ENDPOINT
// ============================================================================

/// POST /api/config/import/preview - Preview what would be imported
pub async fn import_config_preview(Json(request): Json<ImportConfigPreviewRequest>) -> Response {
    let imported = request.config;
    let mut sections = Vec::new();
    let mut warnings = Vec::new();
    let mut total_changes = 0;

    let imported_obj = match imported.as_object() {
        Some(obj) => obj,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_FORMAT",
                "Config must be a JSON object",
                None,
            );
        }
    };

    // Check for unknown sections
    for key in imported_obj.keys() {
        if key != "timestamp" && !CONFIG_SECTIONS.contains(&key.as_str()) {
            warnings.push(format!("Unknown section '{key}' will be ignored"));
        }
    }

    // Check for security-sensitive fields being imported
    for (section, fields) in SENSITIVE_FIELDS {
        if let Some(section_val) = imported_obj.get(*section) {
            for field_path in *fields {
                if has_nested_field(section_val, field_path) {
                    warnings.push(format!(
                        "⚠️ Security warning: Importing '{}.{}' may overwrite authentication settings",
                        section, field_path
                    ));
                }
            }
        }
    }

    // Analyze each known section
    for section in CONFIG_SECTIONS {
        let section_value = imported_obj.get(*section);
        let present = section_value.is_some();

        if !present {
            sections.push(SectionPreview {
                name: section.to_string(),
                label: get_section_label(section),
                present: false,
                valid: true,
                field_count: 0,
                error: None,
                changes: Vec::new(),
            });
            continue;
        }

        let value = section_value.unwrap();
        let field_count = count_fields(value);

        // Validate by attempting to deserialize
        let validation_result: Result<(), String> = match *section {
            "rpc" => serde_json::from_value::<config::RpcConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "trader" => serde_json::from_value::<config::TraderConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "copy_trading" => serde_json::from_value::<config::CopyTradingConfig>(value.clone())
                .map_err(|e| e.to_string())
                .and_then(|copy| copy.validate()),
            "positions" => serde_json::from_value::<config::PositionsConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "filtering" => serde_json::from_value::<config::FilteringConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "swaps" => serde_json::from_value::<config::SwapsConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "tokens" => serde_json::from_value::<config::TokensConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "sol_price" => serde_json::from_value::<config::SolPriceConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "network" => serde_json::from_value::<config::NetworkConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "events" => serde_json::from_value::<config::EventsConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "services" => serde_json::from_value::<config::ServicesConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "monitoring" => serde_json::from_value::<config::MonitoringConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "ohlcv" => serde_json::from_value::<config::OhlcvConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "gui" => serde_json::from_value::<config::GuiConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "telegram" => serde_json::from_value::<config::TelegramConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            _ => Ok(()),
        };

        // Get current config for comparison
        let current_value = config::with_config(|cfg| match *section {
            "rpc" => serde_json::to_value(&cfg.rpc).ok(),
            "trader" => serde_json::to_value(&cfg.trader).ok(),
            "copy_trading" => serde_json::to_value(&cfg.copy_trading).ok(),
            "positions" => serde_json::to_value(&cfg.positions).ok(),
            "filtering" => serde_json::to_value(&cfg.filtering).ok(),
            "swaps" => serde_json::to_value(&cfg.swaps).ok(),
            "tokens" => serde_json::to_value(&cfg.tokens).ok(),
            "sol_price" => serde_json::to_value(&cfg.sol_price).ok(),
            "network" => serde_json::to_value(&cfg.network).ok(),
            "events" => serde_json::to_value(&cfg.events).ok(),
            "services" => serde_json::to_value(&cfg.services).ok(),
            "monitoring" => serde_json::to_value(&cfg.monitoring).ok(),
            "ohlcv" => serde_json::to_value(&cfg.ohlcv).ok(),
            "gui" => serde_json::to_value(&cfg.gui).ok(),
            "telegram" => serde_json::to_value(&cfg.telegram).ok(),
            _ => None,
        });

        let changes = if let Some(curr) = current_value {
            compare_values(&curr, value, "")
        } else {
            Vec::new()
        };

        total_changes += changes.len();

        sections.push(SectionPreview {
            name: section.to_string(),
            label: get_section_label(section),
            present: true,
            valid: validation_result.is_ok(),
            field_count,
            error: validation_result.err(),
            changes,
        });
    }

    let all_valid = sections.iter().filter(|s| s.present).all(|s| s.valid);

    success_response(ImportPreviewResponse {
        valid: all_valid,
        sections,
        warnings,
        total_changes,
    })
}

// ============================================================================
// IMPORT ENDPOINT
// ============================================================================

/// POST /api/config/import - Import configuration
pub async fn import_config(Json(request): Json<ImportConfigRequest>) -> Response {
    let imported = request.config;

    let imported_obj = match imported.as_object() {
        Some(obj) => obj,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_FORMAT",
                "Config must be a JSON object",
                None,
            );
        }
    };

    // Determine which sections to import
    let sections_to_import: Vec<String> = match &request.sections {
        Some(sections) if !sections.is_empty() => sections
            .iter()
            .filter(|s| {
                CONFIG_SECTIONS.contains(&s.as_str()) && imported_obj.contains_key(s.as_str())
            })
            .cloned()
            .collect(),
        _ => imported_obj
            .keys()
            .filter(|k| CONFIG_SECTIONS.contains(&k.as_str()))
            .cloned()
            .collect(),
    };

    if sections_to_import.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_SECTIONS",
            "No valid sections found to import",
            None,
        );
    }

    // PHASE 1: Build a candidate config by cloning current and applying all changes
    // This allows us to validate BEFORE modifying the live config
    let mut candidate_config = config::get_config_clone();
    let mut imported_sections = Vec::new();
    let mut errors = Vec::new();

    for section in &sections_to_import {
        let value = match imported_obj.get(section) {
            Some(v) => v.clone(),
            None => continue,
        };

        let result: Result<serde_json::Value, String> = (|| {
            // Get current config section for merging if needed
            let final_value = if request.merge {
                let current = match section.as_str() {
                    "rpc" => serde_json::to_value(&candidate_config.rpc).ok(),
                    "trader" => serde_json::to_value(&candidate_config.trader).ok(),
                    "copy_trading" => serde_json::to_value(&candidate_config.copy_trading).ok(),
                    "positions" => serde_json::to_value(&candidate_config.positions).ok(),
                    "filtering" => serde_json::to_value(&candidate_config.filtering).ok(),
                    "swaps" => serde_json::to_value(&candidate_config.swaps).ok(),
                    "tokens" => serde_json::to_value(&candidate_config.tokens).ok(),
                    "sol_price" => serde_json::to_value(&candidate_config.sol_price).ok(),
                    "network" => serde_json::to_value(&candidate_config.network).ok(),
                    "events" => serde_json::to_value(&candidate_config.events).ok(),
                    "services" => serde_json::to_value(&candidate_config.services).ok(),
                    "monitoring" => serde_json::to_value(&candidate_config.monitoring).ok(),
                    "ohlcv" => serde_json::to_value(&candidate_config.ohlcv).ok(),
                    "gui" => serde_json::to_value(&candidate_config.gui).ok(),
                    "telegram" => serde_json::to_value(&candidate_config.telegram).ok(),
                    _ => None,
                };

                if let Some(mut curr) = current {
                    // Merge: imported values override current
                    if let (Some(curr_obj), Some(imp_obj)) =
                        (curr.as_object_mut(), value.as_object())
                    {
                        for (key, val) in imp_obj {
                            curr_obj.insert(key.clone(), val.clone());
                        }
                    }
                    curr
                } else {
                    value
                }
            } else {
                value
            };

            Ok(final_value)
        })();

        match result {
            Ok(final_value) => {
                // Apply to candidate config
                if let Err(e) = apply_section_to_config(&mut candidate_config, section, final_value)
                {
                    errors.push(format!("{section}: {e}"));
                } else {
                    imported_sections.push(section.clone());
                }
            }
            Err(e) => errors.push(format!("{section}: {e}")),
        }
    }

    // PHASE 2: Validate the full candidate config BEFORE committing
    // This catches cross-field validation errors (e.g., DCA settings require valid thresholds)
    if !imported_sections.is_empty() {
        if let Err(validation_error) = config::validate_config(&candidate_config) {
            // Validation failed - don't commit anything
            return error_response(
                StatusCode::BAD_REQUEST,
                "VALIDATION_FAILED",
                &format!(
                    "Config validation failed: {}. No changes were applied.",
                    validation_error
                ),
                None,
            );
        }
    }

    // PHASE 3: Validation passed - commit the changes atomically
    if !imported_sections.is_empty() {
        if let Err(e) = config::update_config_section(
            |cfg| {
                // Apply all validated sections at once
                for section in &imported_sections {
                    match section.as_str() {
                        "rpc" => cfg.rpc = candidate_config.rpc.clone(),
                        "trader" => cfg.trader = candidate_config.trader.clone(),
                        "copy_trading" => cfg.copy_trading = candidate_config.copy_trading.clone(),
                        "positions" => cfg.positions = candidate_config.positions.clone(),
                        "filtering" => cfg.filtering = candidate_config.filtering.clone(),
                        "swaps" => cfg.swaps = candidate_config.swaps.clone(),
                        "tokens" => cfg.tokens = candidate_config.tokens.clone(),
                        "sol_price" => cfg.sol_price = candidate_config.sol_price.clone(),
                        "network" => cfg.network = candidate_config.network.clone(),
                        "events" => cfg.events = candidate_config.events.clone(),
                        "services" => cfg.services = candidate_config.services.clone(),
                        "monitoring" => cfg.monitoring = candidate_config.monitoring.clone(),
                        "ohlcv" => cfg.ohlcv = candidate_config.ohlcv.clone(),
                        "gui" => cfg.gui = candidate_config.gui.clone(),
                        "telegram" => cfg.telegram = candidate_config.telegram.clone(),
                        _ => {}
                    }
                }
            },
            false, // Don't save to disk yet
        ) {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "COMMIT_FAILED",
                &format!("Failed to commit config changes: {e}"),
                None,
            );
        }
    }

    // PHASE 4: Save to disk if requested and no errors
    let saved_to_disk =
        if request.save_to_disk && !imported_sections.is_empty() && errors.is_empty() {
            match config::save_config(None) {
                Ok(()) => true,
                Err(e) => {
                    errors.push(format!("Failed to save to disk: {e}"));
                    false
                }
            }
        } else {
            false
        };

    if imported_sections.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "IMPORT_FAILED",
            &format!("Failed to import config: {}", errors.join(", ")),
            None,
        );
    }

    let message = if errors.is_empty() {
        format!(
            "Successfully imported {} section(s)",
            imported_sections.len()
        )
    } else {
        format!(
            "Imported {} section(s) with {} warning(s): {}",
            imported_sections.len(),
            errors.len(),
            errors.join(", ")
        )
    };

    success_response(ImportConfigResponse {
        success: true,
        message,
        imported_sections,
        saved_to_disk,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
