//! Agent-facing copy-trading tools: inspect tasks, their paper/live books and
//! decisions; create, tune, pause, delete and arm tasks. Transports over
//! `trader::copy::control`, the same owner the dashboard Copy Trading tab uses.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::config::with_config;
use crate::trader::copy::workspace::{self, ActivityFilter};
use crate::trader::copy::{
    control, CopyMode, CopyTaskInput, ExitMode, SizingMode, LIVE_ARM_CONFIRMATION,
};
use crate::trader::policy::ExitPolicyOverrides;

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

const EXIT_SEMANTICS: &str = "exit_mode: buy_only never copies the target's sells; mirror sells \
the same fraction of the holding the target sold; hybrid mirrors target sells AND lets the auto \
trader's exit policy manage the position. exit_policy_overrides (stop_loss, trailing, roi, time) \
tune that policy for this task. The policy applies in both books: live positions through the \
auto trader's exit monitor (which needs the trader running), paper holdings through the copy \
service's own sweep at pool price; paper exit activity carries exit_rule. mirror tasks exit only \
on target sells in both modes.";

/// Schema of every editable task field, shared by create and update so the two
/// can never describe the same field differently.
fn task_field_schema() -> serde_json::Value {
    json!({
        "target_address": { "type": "string", "description": "Wallet to copy" },
        "label": { "type": ["string", "null"], "description": "Display name" },
        "enabled": { "type": "boolean", "description": "Whether the task copies new trades" },
        "sizing": {
            "type": "object",
            "description": "{\"kind\":\"fixed\",\"sol\":0.05} spends a fixed SOL per copied buy; {\"kind\":\"ratio_of_target\",\"pct\":10} spends that percent of the target's SOL input",
            "properties": {
                "kind": { "type": "string", "enum": ["fixed", "ratio_of_target"] },
                "sol": { "type": "number" },
                "pct": { "type": "number" }
            },
            "required": ["kind"]
        },
        "exit_mode": { "type": "string", "enum": ["buy_only", "mirror", "hybrid"] },
        "exit_policy_overrides": {
            "type": "object",
            "description": "Per-task exit policy overrides; omitted fields inherit the auto-trader config",
            "properties": {
                "stop_loss": { "type": "object", "properties": {
                    "enabled": { "type": "boolean" }, "threshold_pct": { "type": "number" },
                    "min_hold_seconds": { "type": "integer" }, "allow_partial": { "type": "boolean" },
                    "partial_exit_default_pct": { "type": "number" } } },
                "trailing": { "type": "object", "properties": {
                    "enabled": { "type": "boolean" }, "activation_pct": { "type": "number" },
                    "distance_pct": { "type": "number" } } },
                "roi": { "type": "object", "properties": {
                    "enabled": { "type": "boolean" }, "target_profit_pct": { "type": "number" } } },
                "time": { "type": "object", "properties": {
                    "enabled": { "type": "boolean" }, "loss_threshold_pct": { "type": "number" },
                    "duration_seconds": { "type": "number" } } }
            }
        },
        "max_sol_per_trade": { "type": "number", "description": "Cap per copied buy; <= max_sol_per_token" },
        "max_sol_per_token": { "type": "number", "description": "Cap per token; <= total_budget_sol" },
        "total_budget_sol": { "type": "number", "description": "Lifetime spend cap of the task" },
        "min_target_trade_sol": { "type": ["number", "null"], "description": "Ignore target buys smaller than this (filters noise)" },
        "max_target_trade_sol": { "type": ["number", "null"], "description": "Ignore target buys larger than this" },
        "buy_once_per_token": { "type": "boolean", "description": "Copy only the first buy of each token" },
        "slippage_pct": { "type": "number", "description": "Slippage for copied trades in percent" },
        "require_filter_pass": { "type": ["boolean", "null"], "description": "Per-task override of copy_trading.require_filter_pass (only copy tokens the filtering pipeline passed); null inherits the global setting" }
    })
}

// ============================================================================
// GetCopyTradingOverviewTool
// ============================================================================

pub struct GetCopyTradingOverviewTool;

#[derive(Deserialize)]
struct OverviewParams {
    #[serde(default = "default_activity")]
    activity_limit: usize,
}

fn default_activity() -> usize {
    20
}

#[async_trait]
impl Tool for GetCopyTradingOverviewTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_copy_trading_overview".to_owned(),
            description: "Copy-trading state: system status (enabled, entry blocks, defaults), \
                          every task with its effective state, budget use and book stats \
                          (decisions, fills, open/closed positions, realized/unrealized P&L in \
                          SOL, latency), plus the most recent decisions across tasks."
                .to_owned(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {
                    "activity_limit": { "type": "integer", "minimum": 1, "maximum": 500, "description": "Recent decisions to include (default 20)" }
                },
                "required": []
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: OverviewParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        respond(control::overview(params.activity_limit.clamp(1, 500)).await)
    }
}

// ============================================================================
// GetCopyTaskTool
// ============================================================================

pub struct GetCopyTaskTool;

#[derive(Deserialize)]
struct TaskParams {
    task_id: i64,
}

#[async_trait]
impl Tool for GetCopyTaskTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_copy_task".to_owned(),
            description: "One copy task in depth: full settings, stats (fills, target sells vs \
                          policy exits, wins/losses), budget use, pause reason, the effective \
                          exit rules next to the inherited Trader defaults, every paper holding \
                          marked at the live pool price with entry, peak and the prices its exit \
                          rules act at, the live-readiness checklist and the 20 newest decisions."
                .to_owned(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "integer" }
                },
                "required": ["task_id"]
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: TaskParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        respond(workspace::task_workspace(params.task_id).await)
    }
}

// ============================================================================
// GetCopyActivityTool
// ============================================================================

pub struct GetCopyActivityTool;

#[derive(Deserialize)]
struct ActivityParams {
    #[serde(default)]
    task_id: Option<i64>,
    #[serde(flatten)]
    filter: ActivityFilter,
}

#[async_trait]
impl Tool for GetCopyActivityTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_copy_activity".to_owned(),
            description:
                "Copy decisions newest first: each observed target trade with the \
                          outcome (paper fill, live submit, or the typed skip reason such as \
                          budget_exhausted, already_bought, target_below_minimum, filter_required). \
                          Pages with `before` = the returned next_before."
                    .to_owned(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "integer", "description": "Only this task (default: all tasks)" },
                    "filter": { "type": "string", "enum": ["all", "fills", "exits", "skips", "errors"], "description": "Decision category (default all)" },
                    "mint": { "type": "string", "description": "Only decisions on this token" },
                    "before": { "type": "integer", "description": "Page cursor: rows older than this activity id" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500, "description": "Rows to return (default 50)" }
                },
                "required": []
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: ActivityParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        respond(workspace::activity_page(params.task_id, params.filter).await)
    }
}

// ============================================================================
// CreateCopyTaskTool
// ============================================================================

pub struct CreateCopyTaskTool;

/// Creation input: the task fields with the dashboard form's defaults for the
/// optional ones. Mode is not an input -- every task is born in paper.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateParams {
    target_address: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default = "enabled_default")]
    enabled: bool,
    sizing: SizingMode,
    exit_mode: ExitMode,
    #[serde(default)]
    exit_policy_overrides: ExitPolicyOverrides,
    max_sol_per_trade: f64,
    max_sol_per_token: f64,
    total_budget_sol: f64,
    #[serde(default)]
    min_target_trade_sol: Option<f64>,
    #[serde(default)]
    max_target_trade_sol: Option<f64>,
    #[serde(default = "enabled_default")]
    buy_once_per_token: bool,
    #[serde(default)]
    slippage_pct: Option<f64>,
    #[serde(default)]
    require_filter_pass: Option<bool>,
}

fn enabled_default() -> bool {
    true
}

#[async_trait]
impl Tool for CreateCopyTaskTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = task_field_schema();
        if let Some(slippage) = properties.get_mut("slippage_pct") {
            slippage["description"] = json!(
                "Slippage for copied trades in percent (default copy_trading.default_slippage_pct)"
            );
        }
        ToolDefinition {
            name: "create_copy_task".to_owned(),
            description: format!(
                "Create a copy-trading task for a wallet. Every task starts in PAPER mode \
                 (simulated fills, no money moves); arming live is set_copy_task_mode. \
                 Defaults: enabled=true, buy_once_per_token=true, no overrides. {EXIT_SEMANTICS}"
            ),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": properties,
                "required": ["target_address", "sizing", "exit_mode", "max_sol_per_trade", "max_sol_per_token", "total_budget_sol"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let p: CreateParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        let input = CopyTaskInput {
            target_address: p.target_address,
            label: p.label,
            enabled: p.enabled,
            mode: CopyMode::Paper,
            sizing: p.sizing,
            exit_mode: p.exit_mode,
            exit_policy_overrides: p.exit_policy_overrides,
            max_sol_per_trade: p.max_sol_per_trade,
            max_sol_per_token: p.max_sol_per_token,
            total_budget_sol: p.total_budget_sol,
            min_target_trade_sol: p.min_target_trade_sol,
            max_target_trade_sol: p.max_target_trade_sol,
            buy_once_per_token: p.buy_once_per_token,
            slippage_pct: p
                .slippage_pct
                .unwrap_or_else(|| with_config(|cfg| cfg.copy_trading.default_slippage_pct)),
            require_filter_pass: p.require_filter_pass,
        };
        respond(
            control::create_task(input)
                .await
                .map(|task| json!({ "task": task })),
        )
    }
}

// ============================================================================
// UpdateCopyTaskTool
// ============================================================================

pub struct UpdateCopyTaskTool;

#[async_trait]
impl Tool for UpdateCopyTaskTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = task_field_schema();
        properties["task_id"] = json!({ "type": "integer", "description": "Task to update" });
        ToolDefinition {
            name: "update_copy_task".to_owned(),
            description: format!(
                "Change any fields of a copy task (only the fields given change; set \
                 enabled=false to pause, true to resume). exit_policy_overrides replaces the \
                 whole override object. Mode is changed only by set_copy_task_mode; the \
                 target wallet cannot change (clone_copy_task copies another wallet). \
                 {EXIT_SEMANTICS}"
            ),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": properties,
                "required": ["task_id"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let serde_json::Value::Object(mut fields) = params else {
            return ToolResult::error("Invalid parameters: expected an object".to_owned());
        };
        let task_id = match fields.remove("task_id") {
            Some(value) => match value.as_i64() {
                Some(id) => id,
                None => {
                    return ToolResult::error(
                        "Invalid parameters: task_id must be an integer".to_owned(),
                    )
                }
            },
            None => return ToolResult::error("Invalid parameters: task_id is required".to_owned()),
        };
        if fields.contains_key("mode") {
            return ToolResult::error(
                "mode cannot be changed here; use set_copy_task_mode".to_owned(),
            );
        }
        if fields.is_empty() {
            return ToolResult::error("Nothing to update: pass at least one field".to_owned());
        }
        respond(
            control::update_task(task_id, serde_json::Value::Object(fields))
                .await
                .map(|task| json!({ "task": task })),
        )
    }
}

// ============================================================================
// DeleteCopyTaskTool
// ============================================================================

pub struct DeleteCopyTaskTool;

#[derive(Deserialize)]
struct DeleteParams {
    task_id: i64,
}

#[async_trait]
impl Tool for DeleteCopyTaskTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "delete_copy_task".to_owned(),
            description: "Delete a copy task with its decisions, spend and paper book. Refused \
                          for an enabled live task (pause it first) and while the task still \
                          owns open live positions."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": { "task_id": { "type": "integer" } },
                "required": ["task_id"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: DeleteParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        respond(
            control::delete_task(params.task_id)
                .await
                .map(|()| json!({ "deleted": params.task_id })),
        )
    }
}

// ============================================================================
// SetCopyTaskModeTool
// ============================================================================

pub struct SetCopyTaskModeTool;

#[derive(Deserialize)]
struct ModeParams {
    task_id: i64,
    mode: CopyMode,
    #[serde(default)]
    confirmation: Option<String>,
}

#[async_trait]
impl Tool for SetCopyTaskModeTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "set_copy_task_mode".to_owned(),
            description: format!(
                "Switch a copy task between paper and live. Live copies trades with REAL \
                 money and requires confirmation set to exactly \"{LIVE_ARM_CONFIRMATION}\"; \
                 it is refused (LIVE_UNAVAILABLE) while setup is incomplete, copy trading is \
                 disabled or the emergency stop is on. Returning to paper is always allowed \
                 and needs no confirmation."
            ),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "integer" },
                    "mode": { "type": "string", "enum": ["paper", "live"] },
                    "confirmation": { "type": "string", "description": "Required to arm live" }
                },
                "required": ["task_id", "mode"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: ModeParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        respond(
            control::set_task_mode(params.task_id, params.mode, params.confirmation)
                .await
                .map(|task| json!({ "task": task })),
        )
    }
}
