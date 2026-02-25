//! Action query and retrieval operations.
//!
//! Read-only database operations: single action lookup, filtered listing,
//! paginated history, startup sync, and cleanup of old entries.

use super::ActionFilters;
use crate::actions::types::{Action, ActionState, ActionStep, ActionType, StepStatus};
use crate::logger::{self, LogTag};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

use super::ActionsDatabase;

impl ActionsDatabase {
    /// Get a single action by ID
    pub async fn get_action(&self, action_id: &str) -> Result<Option<Action>, String> {
        let conn = self.get_read_connection()?;

        let action_row: Option<(
            String,         // id
            String,         // action_type
            String,         // entity_id
            String,         // state
            String,         // state_data
            String,         // started_at
            Option<String>, // completed_at
            String,         // metadata
            String,         // updated_at
        )> = conn
            .query_row(
                r#"
                SELECT id, action_type, entity_id, state, state_data,
                       started_at, completed_at, metadata, updated_at
                FROM actions
                WHERE id = ?1
                "#,
                params![action_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("Failed to query action: {e}"))?;

        let Some((
            id,
            action_type_str,
            entity_id,
            _state_str,
            state_data,
            started_at_str,
            completed_at_str,
            metadata_str,
            _updated_at,
        )) = action_row
        else {
            return Ok(None);
        };

        // Parse action type
        let action_type = self.parse_action_type(&action_type_str)?;

        // Parse state
        let state: ActionState = serde_json::from_str(&state_data)
            .map_err(|e| format!("Failed to parse state: {e}"))?;

        // Parse timestamps
        let started_at = DateTime::parse_from_rfc3339(&started_at_str)
            .map_err(|e| format!("Failed to parse started_at: {e}"))?
            .with_timezone(&Utc);

        let completed_at = if let Some(s) = completed_at_str {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        } else {
            None
        };

        // Parse metadata
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
            .map_err(|e| format!("Failed to parse metadata: {e}"))?;

        // Get steps
        let mut stmt = conn
            .prepare(
                r#"
                SELECT step_index, step_id, name, status, started_at, completed_at, error, metadata
                FROM action_steps
                WHERE action_id = ?1
                ORDER BY step_index ASC
                "#,
            )
            .map_err(|e| format!("Failed to prepare step query: {e}"))?;

        let steps = stmt
            .query_map(params![action_id], |row| {
                let status_str: String = row.get(3)?;
                let status = self.parse_step_status(&status_str);

                let started_at_str: Option<String> = row.get(4)?;
                let started_at = started_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let completed_at_str: Option<String> = row.get(5)?;
                let completed_at = completed_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let metadata_str: Option<String> = row.get(7)?;
                let metadata = metadata_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(serde_json::Value::Null);

                Ok(ActionStep {
                    step_id: row.get(1)?,
                    name: row.get(2)?,
                    status,
                    started_at,
                    completed_at,
                    error: row.get(6)?,
                    metadata,
                })
            })
            .map_err(|e| format!("Failed to query steps: {e}"))?
            .collect::<Result<Vec<ActionStep>, _>>()
            .map_err(|e| format!("Failed to collect steps: {e}"))?;

        let current_step_index = match &state {
            ActionState::InProgress {
                current_step_index, ..
            } => *current_step_index,
            _ => 0,
        };

        Ok(Some(Action {
            id,
            action_type,
            entity_id,
            state,
            steps,
            current_step_index,
            started_at,
            completed_at,
            metadata,
        }))
    }

    /// Get actions with filters (optimized with batch fetching)
    pub async fn get_actions(&self, filters: &ActionFilters) -> Result<Vec<Action>, String> {
        let conn = self.get_read_connection()?;

        // Build query for action IDs
        let mut query = String::from(
            r#"
            SELECT id, action_type, entity_id, state, state_data,
                   started_at, completed_at, metadata, updated_at
            FROM actions
            WHERE 1=1
            "#,
        );

        let mut params: Vec<String> = Vec::new();

        if let Some(action_type) = filters.action_type {
            query.push_str(" AND action_type = ?");
            params.push(format!("{:?}", action_type).to_lowercase());
        }

        if let Some(ref entity_id) = filters.entity_id {
            query.push_str(" AND entity_id = ?");
            params.push(entity_id.clone());
        }

        if let Some(ref states) = filters.state {
            if !states.is_empty() {
                let placeholders = states.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND state IN ({placeholders})"));
                for state in states {
                    params.push(state.clone());
                }
            }
        }

        if let Some(started_after) = filters.started_after {
            query.push_str(" AND started_at >= ?");
            params.push(started_after.to_rfc3339());
        }

        if let Some(started_before) = filters.started_before {
            query.push_str(" AND started_at <= ?");
            params.push(started_before.to_rfc3339());
        }

        query.push_str(" ORDER BY started_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {limit}"));
        }

        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET {offset}"));
        }

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare query: {e}"))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();

        // Fetch all actions in one query
        let actions_data: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            String,
        )> = stmt
            .query_map(&params_refs[..], |row| {
                Ok((
                    row.get(0)?, // id
                    row.get(1)?, // action_type
                    row.get(2)?, // entity_id
                    row.get(3)?, // state
                    row.get(4)?, // state_data
                    row.get(5)?, // started_at
                    row.get(6)?, // completed_at
                    row.get(7)?, // metadata
                ))
            })
            .map_err(|e| format!("Failed to query actions: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect actions: {e}"))?;

        if actions_data.is_empty() {
            return Ok(Vec::new());
        }

        // Collect action IDs for batch step fetch
        let action_ids: Vec<String> = actions_data.iter().map(|(id, ..)| id.clone()).collect();

        // Batch fetch all steps for these actions in ONE query
        let placeholders = action_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let steps_query = format!(
            r#"
            SELECT action_id, step_index, step_id, name, status, 
                   started_at, completed_at, error, metadata
            FROM action_steps
            WHERE action_id IN ({})
            ORDER BY action_id, step_index ASC
            "#,
            placeholders
        );

        let mut steps_stmt = conn
            .prepare(&steps_query)
            .map_err(|e| format!("Failed to prepare steps query: {e}"))?;

        let action_id_refs: Vec<&dyn rusqlite::ToSql> = action_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let steps_rows = steps_stmt
            .query_map(&action_id_refs[..], |row| {
                let action_id: String = row.get(0)?;
                let status_str: String = row.get(4)?;
                let status = self.parse_step_status(&status_str);

                let started_at_str: Option<String> = row.get(5)?;
                let started_at = started_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let completed_at_str: Option<String> = row.get(6)?;
                let completed_at = completed_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let metadata_str: Option<String> = row.get(8)?;
                let metadata = metadata_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(serde_json::Value::Null);

                Ok((
                    action_id,
                    ActionStep {
                        step_id: row.get(2)?,
                        name: row.get(3)?,
                        status,
                        started_at,
                        completed_at,
                        error: row.get(7)?,
                        metadata,
                    },
                ))
            })
            .map_err(|e| format!("Failed to query steps: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect steps: {e}"))?;

        // Build a map of action_id -> Vec<ActionStep>
        let mut steps_map: HashMap<String, Vec<ActionStep>> = HashMap::new();
        for (action_id, step) in steps_rows {
            steps_map
                .entry(action_id)
                .or_default()
                .push(step);
        }

        // Assemble actions with their steps
        let mut actions = Vec::new();
        for (
            id,
            action_type_str,
            entity_id,
            _state_str,
            state_data,
            started_at_str,
            completed_at_str,
            metadata_str,
        ) in actions_data
        {
            let action_type = self.parse_action_type(&action_type_str)?;

            let state: ActionState = serde_json::from_str(&state_data)
                .map_err(|e| format!("Failed to parse state for action {id}: {e}"))?;

            let started_at = DateTime::parse_from_rfc3339(&started_at_str)
                .map_err(|e| format!("Failed to parse started_at for action {id}: {e}"))?
                .with_timezone(&Utc);

            let completed_at = if let Some(s) = completed_at_str {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            } else {
                None
            };

            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
                .map_err(|e| format!("Failed to parse metadata for action {id}: {e}"))?;

            let steps = steps_map.remove(&id).unwrap_or_default();

            let current_step_index = match &state {
                ActionState::InProgress {
                    current_step_index, ..
                } => *current_step_index,
                _ => 0,
            };

            actions.push(Action {
                id,
                action_type,
                entity_id,
                state,
                steps,
                current_step_index,
                started_at,
                completed_at,
                metadata,
            });
        }

        Ok(actions)
    }

    /// Get action history with pagination
    pub async fn get_action_history(
        &self,
        limit: usize,
        offset: usize,
        filters: &ActionFilters,
    ) -> Result<(Vec<Action>, usize), String> {
        // Get total count in a scope to drop conn and params early
        let total = {
            let conn = self.get_read_connection()?;

            let mut count_query = "SELECT COUNT(*) FROM actions WHERE 1=1".to_owned();
            let mut params: Vec<String> = Vec::new();

            if let Some(action_type) = filters.action_type {
                count_query.push_str(" AND action_type = ?");
                params.push(format!("{:?}", action_type).to_lowercase());
            }

            if let Some(ref entity_id) = filters.entity_id {
                count_query.push_str(" AND entity_id = ?");
                params.push(entity_id.clone());
            }

            if let Some(ref states) = filters.state {
                if !states.is_empty() {
                    let placeholders = states.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                    count_query.push_str(&format!(" AND state IN ({placeholders})"));
                    for state in states {
                        params.push(state.clone());
                    }
                }
            }

            if let Some(started_after) = filters.started_after.as_ref() {
                count_query.push_str(" AND started_at >= ?");
                params.push(started_after.to_rfc3339());
            }

            if let Some(started_before) = filters.started_before.as_ref() {
                count_query.push_str(" AND started_at <= ?");
                params.push(started_before.to_rfc3339());
            }

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();

            let total: i64 = conn
                .query_row(&count_query, &params_refs[..], |row| row.get(0))
                .map_err(|e| format!("Failed to count actions: {e}"))?;

            total as usize
        };

        // Get actions (conn and params are now dropped)
        let mut filters_with_pagination = filters.clone();
        filters_with_pagination.limit = Some(limit);
        filters_with_pagination.offset = Some(offset);

        let actions = self.get_actions(&filters_with_pagination).await?;

        Ok((actions, total))
    }

    /// Cleanup old actions
    pub async fn cleanup_old_actions(&self, days: i64) -> Result<usize, String> {
        let mut conn = self.get_write_connection()?;

        let cutoff = Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();

        // Use transaction to ensure both deletes succeed or both roll back
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        let deleted = tx
            .execute(
                "DELETE FROM actions WHERE completed_at < ?1 AND completed_at IS NOT NULL",
                params![cutoff_str],
            )
            .map_err(|e| format!("Failed to cleanup old actions: {e}"))?;

        // Cleanup orphaned steps
        tx.execute(
            "DELETE FROM action_steps WHERE action_id NOT IN (SELECT id FROM actions)",
            [],
        )
        .map_err(|e| format!("Failed to cleanup orphaned steps: {e}"))?;

        tx.commit()
            .map_err(|e| format!("Failed to commit cleanup transaction: {e}"))?;

        if deleted > 0 {
            logger::info(
                LogTag::System,
                &format!(
                    "Cleaned up {} old actions (older than {} days)",
                    deleted, days
                ),
            );
        }

        Ok(deleted)
    }

    /// Get recent incomplete actions for startup sync
    pub async fn get_recent_incomplete_actions(&self) -> Result<Vec<Action>, String> {
        // NOTE: We intentionally do not time-box this query. Any action that is still
        // running must be restored after a restart so the in-memory cache and SSE
        // stream remain consistent with the database.
        let filters = ActionFilters {
            state: Some(vec!["in_progress".to_owned()]),
            limit: Some(500),
            ..Default::default()
        };

        self.get_actions(&filters).await
    }

    /// Parse action type from string
    fn parse_action_type(&self, s: &str) -> Result<ActionType, String> {
        match s {
            "swapbuy" => Ok(ActionType::SwapBuy),
            "swapsell" => Ok(ActionType::SwapSell),
            "positionopen" => Ok(ActionType::PositionOpen),
            "positionclose" => Ok(ActionType::PositionClose),
            "positiondca" => Ok(ActionType::PositionDca),
            "positionpartialexit" => Ok(ActionType::PositionPartialExit),
            "manualorder" => Ok(ActionType::ManualOrder),
            _ => Err(format!("Unknown action type: {s}")),
        }
    }

    /// Parse step status from string
    fn parse_step_status(&self, s: &str) -> StepStatus {
        match s {
            "pending" => StepStatus::Pending,
            "inprogress" => StepStatus::InProgress,
            "completed" => StepStatus::Completed,
            "failed" => StepStatus::Failed,
            "skipped" => StepStatus::Skipped,
            _ => StepStatus::Pending,
        }
    }
}
