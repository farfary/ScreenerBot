// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Agent-facing copy-trading analysis and paper-book tools: task analytics and
//! comparison, the wallet profile, clone, paper reset and closing a paper
//! holding. Transports over `trader::copy::workspace`, the same owner the
//! dashboard Copy Trading page uses.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::trader::copy::workspace::{self, CloneRequest};
use crate::trader::copy::InsightRange;

fn parse<T: for<'de> Deserialize<'de>>(params: serde_json::Value) -> Result<T, ToolResult> {
    serde_json::from_value(params)
        .map_err(|e| ToolResult::error(format!("Invalid parameters: {e}")))
}

fn respond<T: serde::Serialize>(result: crate::trader::Result<T>) -> ToolResult {
    match result {
        Ok(value) => ToolResult::success(json!(value)),
        Err(e) => ToolResult::error(e.to_string()),
    }
}

fn task_id_schema(description: &str) -> serde_json::Value {
    json!({
        "type": "object",
        "properties": { "task_id": { "type": "integer", "description": description } },
        "required": ["task_id"]
    })
}

#[derive(Deserialize)]
struct TaskIdParams {
    task_id: i64,
}

// ============================================================================
// GetCopyInsightsTool
// ============================================================================

pub struct GetCopyInsightsTool;

#[derive(Deserialize)]
struct InsightParams {
    #[serde(default)]
    task_id: Option<i64>,
    #[serde(default)]
    from: Option<DateTime<Utc>>,
    #[serde(default)]
    to: Option<DateTime<Utc>>,
}

#[async_trait]
impl Tool for GetCopyInsightsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_copy_insights".to_owned(),
            description: "Copy-trading performance analytics. With task_id: that task's closed \
                          rounds (paper rounds replayed from its decisions, or its real \
                          positions when live), win rate, average win/loss, profit factor, \
                          average hold, cumulative P&L curve, P&L by exit (target_sell, \
                          stop_loss, trailing_stop, break_even, take_profit, time_override, \
                          time_stagnation, manual), skip \
                          reasons, arrival-latency histogram and the fill's price \
                          difference against the target's price; legacy estimates are \
                          excluded from current paper performance. Without task_id: every task side by side."
                .to_owned(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "integer", "description": "One task (default: compare all tasks)" },
                    "from": { "type": "string", "description": "RFC 3339 start of the window (default: all history)" },
                    "to": { "type": "string", "description": "RFC 3339 end of the window" }
                },
                "required": []
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: InsightParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        let range = InsightRange {
            from: params.from,
            to: params.to,
        };
        match params.task_id {
            Some(id) => respond(workspace::task_insights(id, range).await),
            None => respond(
                workspace::compare_tasks(range)
                    .await
                    .map(|tasks| json!({ "tasks": tasks })),
            ),
        }
    }
}

// ============================================================================
// GetCopyWalletProfileTool
// ============================================================================

pub struct GetCopyWalletProfileTool;

#[derive(Deserialize)]
struct WalletParams {
    address: String,
}

#[async_trait]
impl Tool for GetCopyWalletProfileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_copy_wallet_profile".to_owned(),
            description: "What this bot knows about a wallet before or while copying it: \
                          whether it is one of the user's own wallets, its watch status, the \
                          target swaps the copy service observed (buys, sells, tokens, first \
                          and last seen) and the results of every task copying it."
                .to_owned(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": { "address": { "type": "string", "description": "Wallet address" } },
                "required": ["address"]
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: WalletParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        respond(workspace::wallet_profile(params.address.trim()).await)
    }
}

// ============================================================================
// CloneCopyTaskTool
// ============================================================================

pub struct CloneCopyTaskTool;

#[derive(Deserialize)]
struct CloneParams {
    task_id: i64,
    #[serde(flatten)]
    request: CloneRequest,
}

#[async_trait]
impl Tool for CloneCopyTaskTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "clone_copy_task".to_owned(),
            description: "Create a new PAPER task with the same sizing, filters and exit rules \
                          as an existing task -- for another wallet (target_address) or the same \
                          wallet with different settings to A/B test. Starts paused unless \
                          enabled=true."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "integer", "description": "Task to copy the rules of" },
                    "target_address": { "type": "string", "description": "Wallet for the new task (default: the same wallet)" },
                    "label": { "type": "string", "description": "Name of the new task (default: \"<name> (copy)\")" },
                    "enabled": { "type": "boolean", "description": "Start copying immediately (default false)" }
                },
                "required": ["task_id"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: CloneParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        respond(
            workspace::clone_task(params.task_id, params.request)
                .await
                .map(|task| json!({ "task": task })),
        )
    }
}

// ============================================================================
// ResetCopyPaperBookTool
// ============================================================================

pub struct ResetCopyPaperBookTool;

#[async_trait]
impl Tool for ResetCopyPaperBookTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "reset_copy_paper_book".to_owned(),
            description: "Start a paper task's evaluation over under the quote model: removes \
                          its current paper holdings, paper spend and the quote fills, sells \
                          and skips that built them, while legacy estimates and live data are \
                          preserved. Refused for a live task. Use after changing a task's \
                          rules so the results measure the new rules only."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: task_id_schema("Paper task to reset"),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: TaskIdParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        respond(workspace::reset_paper_book(params.task_id).await)
    }
}

// ============================================================================
// CloseCopyPaperHoldingTool
// ============================================================================

pub struct CloseCopyPaperHoldingTool;

#[derive(Deserialize)]
struct CloseParams {
    task_id: i64,
    mint: String,
    opened_at: DateTime<Utc>,
}

#[async_trait]
impl Tool for CloseCopyPaperHoldingTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "close_copy_paper_holding".to_owned(),
            description: "Sell one open paper holding round of a copy task through the paper \
                          book's FIFO quote queue: a quote for the held amount, the task's \
                          simulated delay, then a second quote that must meet the first \
                          quote's minimum output, with estimated costs counted once, \
                          booked as a manual exit. opened_at \
                          must be the exact paper_holdings.opened_at the workspace returned \
                          for the round; a round that closed or reopened since is refused. A \
                          failed quote leaves the holding unchanged. Simulated only: no money \
                          moves."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "integer" },
                    "mint": { "type": "string", "description": "Token of the paper holding" },
                    "opened_at": {
                        "type": "string",
                        "format": "date-time",
                        "description": "Use the EXACT paper_holdings.opened_at the workspace returned for this holding round"
                    }
                },
                "required": ["task_id", "mint", "opened_at"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: CloseParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        respond(
            workspace::close_paper_holding(params.task_id, params.mint.trim(), params.opened_at)
                .await,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, CloseCopyPaperHoldingTool, CloseParams};
    use crate::agent_control::tools::{Tool, ToolCategory};
    use chrono::Timelike;
    use serde_json::json;

    #[test]
    fn the_close_schema_requires_the_confirmed_round() {
        let definition = CloseCopyPaperHoldingTool.definition();
        assert_eq!(
            definition.parameters["required"],
            json!(["task_id", "mint", "opened_at"])
        );
        assert_eq!(
            definition.parameters["properties"]["opened_at"]["type"],
            "string"
        );
        assert_eq!(
            definition.parameters["properties"]["opened_at"]["format"],
            "date-time"
        );
    }

    #[test]
    fn the_close_capability_flags_are_unchanged() {
        let definition = CloseCopyPaperHoldingTool.definition();
        assert_eq!(definition.name, "close_copy_paper_holding");
        assert!(matches!(definition.category, ToolCategory::Trading));
        assert!(definition.mutating);
        assert!(definition.requires_confirmation);
    }

    #[test]
    fn a_close_without_a_valid_round_fails_before_any_store_call() {
        for params in [
            json!({ "task_id": 1, "mint": "Mint111" }),
            json!({ "task_id": 1, "mint": "Mint111", "opened_at": null }),
            json!({ "task_id": 1, "mint": "Mint111", "opened_at": "yesterday" }),
        ] {
            assert!(parse::<CloseParams>(params.clone()).is_err(), "{params}");
        }
    }

    #[test]
    fn a_close_keeps_the_round_instant_exact() {
        let params: CloseParams = parse(json!({
            "task_id": 1,
            "mint": "Mint111",
            "opened_at": "2026-10-07T03:00:00.123456789Z"
        }))
        .unwrap_or_else(|_| panic!("valid parameters"));
        assert_eq!(params.opened_at.nanosecond(), 123_456_789);
    }
}
