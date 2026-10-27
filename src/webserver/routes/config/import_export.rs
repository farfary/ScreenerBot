// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Config API Import/Export
//!
//! Endpoints for importing and exporting configuration files.

use axum::{
    response::{IntoResponse as _, Response},
    Json,
};

use crate::config;
use crate::i18n::{ids, UiArg, UiText};
use crate::webserver::{
    api_error::{ApiError, ApiErrorCode},
    utils::success_response,
    Error, Result,
};

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

/// The serialized value of one importable section; `None` for a section with
/// no import/export arm.
fn section_value(cfg: &config::Config, section: &str) -> Option<serde_json::Value> {
    match section {
        "chains" => serde_json::to_value(&cfg.chains).ok(),
        "trader" => serde_json::to_value(&cfg.trader).ok(),
        "copy_trading" => serde_json::to_value(&cfg.copy_trading).ok(),
        "positions" => serde_json::to_value(&cfg.positions).ok(),
        "filtering" => serde_json::to_value(&cfg.filtering).ok(),
        "tokens" => serde_json::to_value(&cfg.tokens).ok(),
        "network" => serde_json::to_value(&cfg.network).ok(),
        "events" => serde_json::to_value(&cfg.events).ok(),
        "services" => serde_json::to_value(&cfg.services).ok(),
        "monitoring" => serde_json::to_value(&cfg.monitoring).ok(),
        "ohlcv" => serde_json::to_value(&cfg.ohlcv).ok(),
        "gui" => serde_json::to_value(&cfg.gui).ok(),
        "telegram" => serde_json::to_value(&cfg.telegram).ok(),
        "llm" => serde_json::to_value(&cfg.llm).ok(),
        "llm_analysis" => serde_json::to_value(&cfg.llm_analysis).ok(),
        "assistant" => serde_json::to_value(&cfg.assistant).ok(),
        "agent_control" => serde_json::to_value(&cfg.agent_control).ok(),
        _ => None,
    }
}

/// What the legacy relocation of an import did.
#[derive(Default)]
struct LegacyRelocation {
    /// The top-level sections the relocation created: they hold only the moved
    /// fields, never a whole section, so they always merge into the current
    /// values whatever the requested mode.
    created_sections: Vec<String>,
    /// Every relocation whose legacy node the import carried, as (legacy path,
    /// canonical path).
    moved: Vec<(&'static [&'static str], &'static [&'static str])>,
}

/// Move what an older export still carries at its legacy paths (the `rpc`,
/// `swaps` and `sol_price` sections, the exit-rule leaves of `positions`) to
/// its canonical place before any section is chosen. A malformed legacy value
/// refuses the whole import, and so does a strict name (the time-override unit, the
/// DCA basis) the schema does not know: an import is a write, and the relocation would
/// otherwise read that name as its fallback the way it tolerates a stored config file.
fn relocate_legacy_sections(
    imported: &mut serde_json::Value,
) -> std::result::Result<LegacyRelocation, Response> {
    if let Some(reason) = config::refused_strict_name(imported) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_CONFIG_IMPORT_VALIDATION_FAILED,
        )
        .details(reason)
        .into_response());
    }
    let sections_before: Vec<String> = imported
        .as_object()
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();
    let moved: Vec<_> = config::relocation_destinations()
        .filter(|(from, _)| json_node_at(imported, from).is_some())
        .collect();
    let relocated = config::relocate_legacy_layout(imported).map_err(|e| {
        ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_CONFIG_IMPORT_VALIDATION_FAILED,
        )
        .details(e.to_string())
        .into_response()
    })?;
    if !relocated {
        return Ok(LegacyRelocation::default());
    }
    let created_sections = imported
        .as_object()
        .map(|obj| {
            obj.keys()
                .filter(|key| !sections_before.contains(key))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    Ok(LegacyRelocation {
        created_sections,
        moved,
    })
}

/// The node of `value` at `path`; `None` when a segment is absent.
fn json_node_at<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    path.iter()
        .try_fold(value, |node, segment| node.as_object()?.get(*segment))
}

/// Store `node` in `value` at `path`, creating every missing object on the way;
/// a non-object on the way leaves `value` unchanged.
fn insert_json_node(value: &mut serde_json::Value, path: &[&str], node: serde_json::Value) {
    let Some((leaf, parents)) = path.split_last() else {
        return;
    };
    let mut current = value;
    for segment in parents {
        let Some(object) = current.as_object_mut() else {
            return;
        };
        current = object
            .entry(*segment)
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    }
    if let Some(object) = current.as_object_mut() {
        object.insert((*leaf).to_owned(), node);
    }
}

/// A replaced section keeps the current value of every relocation destination
/// the imported value lacks: an export older than the relocation carried that
/// value at its legacy path, if at all, and the section's defaults must not
/// overwrite what the current config holds.
fn keep_absent_relocation_destinations(
    current: &serde_json::Value,
    section: &str,
    imported: &mut serde_json::Value,
) {
    for (_, to) in config::relocation_destinations() {
        let Some((_, path)) = to.split_first().filter(|(head, _)| **head == section) else {
            continue;
        };
        if json_node_at(imported, path).is_some() {
            continue;
        }
        if let Some(kept) = json_node_at(current, path) {
            insert_json_node(imported, path, kept.clone());
        }
    }
}

/// The values a selected legacy section of an older export feeds into
/// sections the selection leaves out, keyed by destination section. Selecting
/// the section that carried a value imports it at its canonical path, merged
/// into the current values.
fn legacy_selection_values(
    selected: &[String],
    imported: &serde_json::Value,
    relocation: &LegacyRelocation,
    sections_to_import: &[String],
) -> Vec<(String, serde_json::Value)> {
    let mut fed = serde_json::Map::new();
    for (from, to) in &relocation.moved {
        let Some((section, path)) = to.split_first() else {
            continue;
        };
        if !selected.iter().any(|name| name == from[0])
            || sections_to_import.iter().any(|name| name == section)
            || !CONFIG_SECTIONS.contains(section)
        {
            continue;
        }
        let Some(node) = json_node_at(imported, to) else {
            continue;
        };
        let target = fed
            .entry(*section)
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        insert_json_node(target, path, node.clone());
    }
    fed.into_iter().collect()
}

/// The value an import writes for `section`: the imported value over the
/// current one when merging, the imported value alone when replacing, with
/// the relocation destinations it lacks kept from the current config.
fn section_import_value(
    cfg: &config::Config,
    section: &str,
    mut imported: serde_json::Value,
    merge: bool,
) -> serde_json::Value {
    let Some(mut current) = section_value(cfg, section) else {
        return imported;
    };
    if merge {
        config::merge_document(&mut current, &imported);
        current
    } else {
        keep_absent_relocation_destinations(&current, section, &mut imported);
        imported
    }
}

/// Replace one importable section of `cfg` from a value. The single list of
/// importable sections a value can be written to; `section_value` is its read side.
fn apply_section_to_config(
    cfg: &mut config::Config,
    section: &str,
    value: serde_json::Value,
) -> Result<()> {
    match section {
        "chains" => {
            cfg.chains = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid ChainsConfig: {e}"),
            })?;
        }
        "trader" => {
            cfg.trader = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid TraderConfig: {e}"),
            })?;
        }
        "copy_trading" => {
            let copy: config::CopyTradingConfig =
                serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                    detail: format!("Invalid CopyTradingConfig: {e}"),
                })?;
            copy.validate()?;
            cfg.copy_trading = copy;
        }
        "positions" => {
            cfg.positions = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid PositionsConfig: {e}"),
            })?;
        }
        "filtering" => {
            cfg.filtering = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid FilteringConfig: {e}"),
            })?;
        }
        "tokens" => {
            cfg.tokens = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid TokensConfig: {e}"),
            })?;
        }
        "network" => {
            cfg.network = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid NetworkConfig: {e}"),
            })?;
        }
        "events" => {
            cfg.events = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid EventsConfig: {e}"),
            })?;
        }
        "services" => {
            cfg.services = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid ServicesConfig: {e}"),
            })?;
        }
        "monitoring" => {
            cfg.monitoring = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid MonitoringConfig: {e}"),
            })?;
        }
        "ohlcv" => {
            cfg.ohlcv = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid OhlcvConfig: {e}"),
            })?;
        }
        "gui" => {
            cfg.gui = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid GuiConfig: {e}"),
            })?;
        }
        "telegram" => {
            cfg.telegram = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid TelegramConfig: {e}"),
            })?;
        }
        "llm" => {
            cfg.llm = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid LlmConfig: {e}"),
            })?;
        }
        "llm_analysis" => {
            cfg.llm_analysis = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid LlmAnalysisConfig: {e}"),
            })?;
        }
        "assistant" => {
            cfg.assistant = serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                detail: format!("Invalid AssistantConfig: {e}"),
            })?;
        }
        "agent_control" => {
            cfg.agent_control =
                serde_json::from_value(value).map_err(|e| Error::InvalidImport {
                    detail: format!("Invalid AgentControlConfig: {e}"),
                })?;
        }
        _ => {
            return Err(Error::UnknownConfigKey {
                key: section.to_owned(),
            })
        }
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
            let section_value = section_value(cfg, section);

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
    let mut imported = request.config;
    let relocated_sections = match relocate_legacy_sections(&mut imported) {
        Ok(relocation) => relocation.created_sections,
        Err(response) => return response,
    };
    let mut sections = Vec::new();
    let mut warnings = Vec::new();
    let mut total_changes = 0;

    let imported_obj = match imported.as_object() {
        Some(obj) => obj,
        None => {
            return ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_CONFIG_IMPORT_NOT_OBJECT,
            )
            .into_response();
        }
    };

    // Check for unknown sections
    for key in imported_obj.keys() {
        if key != "timestamp" && !CONFIG_SECTIONS.contains(&key.as_str()) {
            warnings.push(
                UiText::new(ids::SYSTEM_CONFIG_IMPORT_WARNING_UNKNOWN_SECTION)
                    .arg("section", UiArg::Text(key.clone())),
            );
        }
    }

    // Check for security-sensitive fields being imported
    for (section, fields) in SENSITIVE_FIELDS {
        if let Some(section_val) = imported_obj.get(*section) {
            for field_path in *fields {
                if has_nested_field(section_val, field_path) {
                    warnings.push(
                        UiText::new(ids::SYSTEM_CONFIG_IMPORT_WARNING_SENSITIVE_FIELD)
                            .arg("field", UiArg::Text(format!("{section}.{field_path}"))),
                    );
                }
            }
        }
    }

    // Analyze each known section
    let scratch = config::get_config_clone();
    for section in CONFIG_SECTIONS {
        let imported_section = imported_obj.get(*section);
        let present = imported_section.is_some();

        if !present {
            sections.push(SectionPreview {
                name: section.to_string(),
                present: false,
                valid: true,
                field_count: 0,
                error: None,
                changes: Vec::new(),
            });
            continue;
        }

        let value = imported_section.unwrap();
        let field_count = count_fields(value);

        // Validate by applying the section to a scratch copy of the config;
        // a section built by legacy relocation is validated merged, as it is imported
        let current_value = section_value(&scratch, section);
        let validation_result: Result<()> = if current_value.is_some() {
            let merge = relocated_sections.iter().any(|s| s == section);
            let applied = section_import_value(&scratch, section, value.clone(), merge);
            apply_section_to_config(&mut scratch.clone(), section, applied)
        } else {
            Ok(())
        };

        let changes = if let Some(curr) = current_value {
            compare_values(&curr, value, "")
        } else {
            Vec::new()
        };

        total_changes += changes.len();

        sections.push(SectionPreview {
            name: section.to_string(),
            present: true,
            valid: validation_result.is_ok(),
            field_count,
            error: validation_result.err().map(|e| {
                let detail = match e {
                    Error::InvalidImport { detail } => detail,
                    other => other.to_string(),
                };
                UiText::new(ids::SYSTEM_CONFIG_IMPORT_SECTION_ERROR)
                    .arg("detail", UiArg::Text(detail))
            }),
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
    let mut imported = request.config;
    let relocation = match relocate_legacy_sections(&mut imported) {
        Ok(relocation) => relocation,
        Err(response) => return response,
    };

    let imported_obj = match imported.as_object() {
        Some(obj) => obj,
        None => {
            return ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_CONFIG_IMPORT_NOT_OBJECT,
            )
            .into_response();
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

    // Sections chosen by name import as a whole; the values a selected legacy
    // section feeds into a section left out merge into the current values.
    let mut section_values: Vec<(String, serde_json::Value, bool)> = sections_to_import
        .iter()
        .filter_map(|section| {
            let merge = request.merge || relocation.created_sections.contains(section);
            imported_obj
                .get(section)
                .map(|value| (section.clone(), value.clone(), merge))
        })
        .collect();
    if let Some(selected) = request.sections.as_deref().filter(|s| !s.is_empty()) {
        section_values.extend(
            legacy_selection_values(selected, &imported, &relocation, &sections_to_import)
                .into_iter()
                .map(|(section, value)| (section, value, true)),
        );
    }

    if section_values.is_empty() {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_CONFIG_IMPORT_NO_SECTIONS,
        )
        .into_response();
    }

    // PHASE 1: Build a candidate config by cloning current and applying all changes
    // This allows us to validate BEFORE modifying the live config
    let mut candidate_config = config::get_config_clone();
    let mut imported_sections = Vec::new();
    let mut errors = Vec::new();

    for (section, value, merge) in section_values {
        // Imported values override current ones when merging
        let final_value = section_import_value(&candidate_config, &section, value, merge);

        // Apply to candidate config
        if let Err(e) = apply_section_to_config(&mut candidate_config, &section, final_value) {
            errors.push(format!("{section}: {e}"));
        } else {
            imported_sections.push(section);
        }
    }

    // PHASE 2: Validate the full candidate config BEFORE committing
    // This catches cross-field validation errors (e.g., DCA settings require valid thresholds)
    if !imported_sections.is_empty() {
        if let Err(validation_error) = config::validate_config(&candidate_config) {
            // Validation failed - don't commit anything
            return ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_CONFIG_IMPORT_VALIDATION_FAILED,
            )
            .details(validation_error.to_string())
            .into_response();
        }
    }

    // PHASE 3: Validation passed - commit the changes atomically
    if !imported_sections.is_empty() {
        let mut applied: Result<()> = Ok(());
        let committed = config::update_config_section(
            |cfg| {
                // Apply all validated sections to a staged copy, then swap it in
                let mut staged = cfg.clone();
                applied = imported_sections.iter().try_for_each(|section| {
                    match section_value(&candidate_config, section) {
                        Some(value) => apply_section_to_config(&mut staged, section, value),
                        None => Ok(()),
                    }
                });
                if applied.is_ok() {
                    *cfg = staged;
                }
            },
            false, // Don't save to disk yet
        )
        .map_err(Error::from)
        .and(applied);
        if let Err(e) = committed {
            return ApiError::new(
                ApiErrorCode::ConfigError,
                ids::ERRORS_CONFIG_IMPORT_COMMIT_FAILED,
            )
            .details(e.to_string())
            .into_response();
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
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_CONFIG_IMPORT_FAILED)
            .details(errors.join(", "))
            .into_response();
    }

    let count = UiArg::Count(imported_sections.len() as i64);
    let text = if errors.is_empty() {
        UiText::new(ids::SYSTEM_RESULT_CONFIG_IMPORTED).arg("count", count)
    } else {
        UiText::new(ids::SYSTEM_RESULT_CONFIG_IMPORTED_WITH_WARNINGS)
            .arg("count", count)
            .arg("warnings", UiArg::Count(errors.len() as i64))
            .arg("details", UiArg::Text(errors.join(", ")))
    };

    success_response(ImportConfigResponse {
        success: true,
        text,
        imported_sections,
        saved_to_disk,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::{apply_section_to_config, section_value, CONFIG_SECTIONS};

    #[test]
    fn every_listed_section_exports_and_imports() {
        let source = crate::config::Config::default();
        let mut target = crate::config::Config::default();
        for section in CONFIG_SECTIONS {
            let value = section_value(&source, section)
                .unwrap_or_else(|| panic!("section `{section}` has no export arm"));
            apply_section_to_config(&mut target, section, value.clone())
                .unwrap_or_else(|e| panic!("section `{section}` does not import: {e}"));
            assert_eq!(section_value(&target, section), Some(value), "{section}");
        }
    }
}
