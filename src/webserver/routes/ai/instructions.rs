//! AI instructions, templates, and history handlers

use axum::{extract::{Path, Query, State}, http::StatusCode, response::Response, Json};
use std::sync::Arc;

use crate::ai::db;
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// ============================================================================
// INSTRUCTIONS HANDLERS
// ============================================================================

/// GET /api/ai/instructions - List all instructions
pub async fn list_instructions(State(_state): State<Arc<AppState>>) -> Response {
    match db::with_ai_db(|conn| db::list_instructions(conn)) {
        Ok(instructions) => {
            let total = instructions.len();
            let instructions: Vec<InstructionResponse> = instructions
                .into_iter()
                .map(|i| InstructionResponse {
                    id: i.id,
                    name: i.name,
                    content: i.content,
                    category: i.category,
                    priority: i.priority,
                    enabled: i.enabled,
                    created_at: i.created_at,
                    updated_at: i.updated_at,
                })
                .collect();

            success_response(InstructionsListResponse {
                instructions,
                total,
            })
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list instructions: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/instructions/:id - Get single instruction
pub async fn get_instruction(State(_state): State<Arc<AppState>>, Path(id): Path<i64>) -> Response {
    match db::with_ai_db(|conn| db::get_instruction(conn, id)) {
        Ok(Some(i)) => success_response(InstructionResponse {
            id: i.id,
            name: i.name,
            content: i.content,
            category: i.category,
            priority: i.priority,
            enabled: i.enabled,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Instruction {} not found", id),
            None,
        ),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get instruction: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/instructions - Create new instruction
pub async fn create_instruction(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CreateInstructionRequest>,
) -> Response {
    let category = req.category.unwrap_or_else(|| "general".to_string());

    match db::with_ai_db(|conn| db::create_instruction(conn, &req.name, &req.content, &category)) {
        Ok(id) => {
            logger::info(
                LogTag::Api,
                &format!("Created AI instruction: {} ({})", req.name, category),
            );

            // Fetch the created instruction
            match db::with_ai_db(|conn| db::get_instruction(conn, id)) {
                Ok(Some(instruction)) => success_response(InstructionResponse {
                    id: instruction.id,
                    name: instruction.name,
                    content: instruction.content,
                    category: instruction.category,
                    priority: instruction.priority,
                    enabled: instruction.enabled,
                    created_at: instruction.created_at,
                    updated_at: instruction.updated_at,
                }),
                Ok(None) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    "Failed to retrieve created instruction",
                    None,
                ),
                Err(e) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    &format!("Failed to retrieve created instruction: {}", e),
                    None,
                ),
            }
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to create instruction: {}", e),
            None,
        ),
    }
}

/// PATCH /api/ai/instructions/:id - Update instruction
pub async fn update_instruction(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateInstructionRequest>,
) -> Response {
    match db::with_ai_db(|conn| {
        db::update_instruction(
            conn,
            id,
            req.name.as_deref(),
            req.content.as_deref(),
            req.category.as_deref(),
            req.priority,
            req.enabled,
        )
    }) {
        Ok(()) => {
            logger::info(LogTag::Api, &format!("Updated AI instruction: {}", id));

            // Fetch the updated instruction
            match db::with_ai_db(|conn| db::get_instruction(conn, id)) {
                Ok(Some(instruction)) => success_response(InstructionResponse {
                    id: instruction.id,
                    name: instruction.name,
                    content: instruction.content,
                    category: instruction.category,
                    priority: instruction.priority,
                    enabled: instruction.enabled,
                    created_at: instruction.created_at,
                    updated_at: instruction.updated_at,
                }),
                Ok(None) => error_response(
                    StatusCode::NOT_FOUND,
                    "NOT_FOUND",
                    &format!("Instruction {} not found", id),
                    None,
                ),
                Err(e) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    &format!("Failed to retrieve updated instruction: {}", e),
                    None,
                ),
            }
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to update instruction: {}", e),
            None,
        ),
    }
}

/// DELETE /api/ai/instructions/:id - Delete instruction
pub async fn delete_instruction(State(_state): State<Arc<AppState>>, Path(id): Path<i64>) -> Response {
    match db::with_ai_db(|conn| db::delete_instruction(conn, id)) {
        Ok(()) => {
            logger::info(LogTag::Api, &format!("Deleted AI instruction: {}", id));
            success_response(serde_json::json!({
                "message": "Instruction deleted successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to delete instruction: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/instructions/reorder - Reorder instructions
pub async fn reorder_instructions(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ReorderInstructionsRequest>,
) -> Response {
    match db::with_ai_db(|conn| db::reorder_instructions(conn, &req.ids)) {
        Ok(()) => {
            logger::info(
                LogTag::Api,
                &format!("Reordered {} AI instructions", req.ids.len()),
            );
            success_response(serde_json::json!({
                "message": "Instructions reordered successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to reorder instructions: {}", e),
            None,
        ),
    }
}

// ============================================================================
// TEMPLATES HANDLERS
// ============================================================================

/// GET /api/ai/templates - List built-in templates
pub async fn list_templates(State(_state): State<Arc<AppState>>) -> Response {
    let templates = db::get_builtin_templates();
    let templates: Vec<TemplateResponse> = templates
        .into_iter()
        .map(|t| TemplateResponse {
            id: t.id.to_string(),
            name: t.name.to_string(),
            category: t.category.to_string(),
            content: t.content.to_string(),
            tags: t.tags.iter().map(|s| s.to_string()).collect(),
        })
        .collect();

    success_response(TemplatesListResponse { templates })
}

// ============================================================================
// HISTORY HANDLERS
// ============================================================================

/// GET /api/ai/history - List decision history with pagination
pub async fn list_history(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 100);

    // Calculate offset
    let offset = (page - 1) * per_page;

    // Fetch decisions based on whether mint filter is provided
    let result = if let Some(mint) = query.mint {
        // For specific mint, use list_decisions_for_mint_paginated with offset
        db::with_ai_db(|conn| db::list_decisions_for_mint_paginated(conn, &mint, per_page, offset))
    } else {
        // For all decisions, use list_decisions with pagination
        db::with_ai_db(|conn| db::list_decisions(conn, per_page, offset))
    };

    match result {
        Ok(decisions) => {
            // Get total count (simplified - in production, you'd want a separate count query)
            let total = if decisions.len() == per_page {
                // Page is full, there are likely more results
                (page * per_page) + 1
            } else {
                // Last page
                ((page - 1) * per_page) + decisions.len()
            };

            let decisions: Vec<DecisionHistoryResponse> = decisions
                .into_iter()
                .map(|d| DecisionHistoryResponse {
                    id: d.id,
                    mint: d.mint,
                    symbol: d.symbol,
                    decision: d.decision,
                    confidence: d.confidence,
                    reasoning: d.reasoning,
                    risk_level: d.risk_level,
                    provider: d.provider,
                    model: d.model,
                    tokens_used: d.tokens_used,
                    latency_ms: d.latency_ms,
                    cached: d.cached,
                    created_at: d.created_at,
                })
                .collect();

            success_response(HistoryListResponse {
                decisions,
                total,
                page,
                per_page,
            })
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list decision history: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/history/:id - Get single decision details
pub async fn get_history_detail(State(_state): State<Arc<AppState>>, Path(id): Path<i64>) -> Response {
    match db::with_ai_db(|conn| db::get_decision(conn, id)) {
        Ok(Some(d)) => success_response(DecisionHistoryResponse {
            id: d.id,
            mint: d.mint,
            symbol: d.symbol,
            decision: d.decision,
            confidence: d.confidence,
            reasoning: d.reasoning,
            risk_level: d.risk_level,
            provider: d.provider,
            model: d.model,
            tokens_used: d.tokens_used,
            latency_ms: d.latency_ms,
            cached: d.cached,
            created_at: d.created_at,
        }),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Decision {} not found", id),
            None,
        ),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get decision: {}", e),
            None,
        ),
    }
}
