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
                          stop_loss, trailing_stop, take_profit, time_override, manual), skip \
                          reasons, arrival-latency histogram and fill slippage against the \
                          target's price. Without task_id: every task side by side."
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
            description: "Start a paper task's evaluation over: removes its paper holdings, \
                          paper spend and the paper fills, sells and skips that built them. \
                          Refused for a live task. Use after changing a task's rules so the \
                          results measure the new rules only."
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
}

#[async_trait]
impl Tool for CloseCopyPaperHoldingTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "close_copy_paper_holding".to_owned(),
            description: "Sell one open paper holding of a copy task at the live pool price, \
                          with the task's slippage and fees, booked as a manual exit. A holding \
                          with no pool price is written off at zero proceeds. Simulated only: \
                          no money moves."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "integer" },
                    "mint": { "type": "string", "description": "Token of the paper holding" }
                },
                "required": ["task_id", "mint"]
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
        respond(workspace::close_paper_holding(params.task_id, params.mint.trim()).await)
    }
}
