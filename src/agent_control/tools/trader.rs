//! Agent-facing auto-trader tools: state, performance, start/stop, monitors,
//! loss limit and exit templates. Transports over `trader::controller`,
//! `trader::stats` and `trader::templates` -- the same owners the dashboard uses.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::config::with_config;
use crate::trader::{self, Monitor};

fn parse<T: for<'de> Deserialize<'de>>(params: serde_json::Value) -> Result<T, ToolResult> {
    serde_json::from_value(params)
        .map_err(|e| ToolResult::error(format!("Invalid parameters: {e}")))
}

fn no_params() -> serde_json::Value {
    json!({ "type": "object", "properties": {}, "required": [] })
}

// ============================================================================
// GetTraderStatusTool
// ============================================================================

pub struct GetTraderStatusTool;

#[async_trait]
impl Tool for GetTraderStatusTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_trader_status".to_owned(),
            description: "Full auto-trader state in one call: running/available, emergency stop, \
                          entry/exit monitors, loss limit progress, open-slot usage and the \
                          effective sizing and exit settings (stop loss, trailing stop, ROI, \
                          time override, DCA)."
                .to_owned(),
            category: ToolCategory::System,
            parameters: no_params(),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> ToolResult {
        let open_positions = crate::positions::get_open_positions_count().await;
        let settings = with_config(|cfg| {
            json!({
                "trade_size_sol": cfg.trader.trade_size_sol,
                "max_open_positions": cfg.trader.max_open_positions,
                "open_positions": open_positions,
                "stop_loss": {
                    "enabled": cfg.trader.stop_loss_enabled,
                    "threshold_pct": cfg.trader.stop_loss_threshold_pct,
                },
                "trailing_stop": {
                    "enabled": cfg.positions.trailing_stop_enabled,
                    "activation_pct": cfg.positions.trailing_stop_activation_pct,
                    "distance_pct": cfg.positions.trailing_stop_distance_pct,
                },
                "roi_exit": {
                    "enabled": cfg.trader.roi_exit_enabled,
                    "target_pct": cfg.trader.roi_target_percent,
                },
                "time_override": {
                    "enabled": cfg.trader.time_override_enabled,
                    "duration": cfg.trader.time_override_duration,
                    "unit": cfg.trader.time_override_unit,
                    "loss_threshold_pct": cfg.trader.time_override_loss_threshold_percent,
                },
                "dca": {
                    "enabled": cfg.trader.dca_enabled,
                    "size_percentage": cfg.trader.dca_size_percentage,
                },
            })
        });
        ToolResult::success(json!({
            "trader": trader::trader_status(),
            "force_stop": crate::global::get_force_stop_status(),
            "monitors": trader::monitors_status(),
            "loss_limit": trader::loss_limit_snapshot(),
            "entries_blocked_by_loss_limit":
                trader::safety::loss_limit::is_entry_blocked_by_loss_limit(),
            "settings": settings,
        }))
    }
}

// ============================================================================
// GetTraderStatsTool
// ============================================================================

pub struct GetTraderStatsTool;

#[derive(Deserialize)]
struct StatsParams {
    #[serde(default)]
    days: Option<u32>,
    #[serde(default)]
    include_daily: bool,
}

#[async_trait]
impl Tool for GetTraderStatsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_trader_stats".to_owned(),
            description: "Realized trading performance over a window: win rate, total/gross \
                          P&L in SOL, profit factor, expectancy, max drawdown, average \
                          win/loss, hold times, best/worst trade and a per-exit-reason \
                          breakdown -- the exact figures of the dashboard Stats tab."
                .to_owned(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {
                    "days": { "type": "integer", "minimum": 1, "maximum": 365, "description": "Window in days (default 30)" },
                    "include_daily": { "type": "boolean", "description": "Include the per-day P&L series (default false)" }
                },
                "required": []
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: StatsParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        let mut stats = trader::stats::trader_stats(params.days.unwrap_or(30)).await;
        if !params.include_daily {
            stats.daily_pnl.clear();
        }
        ToolResult::success(json!(stats))
    }
}

// ============================================================================
// SetTraderEnabledTool
// ============================================================================

pub struct SetTraderEnabledTool;

#[derive(Deserialize)]
struct EnabledParams {
    enabled: bool,
}

#[async_trait]
impl Tool for SetTraderEnabledTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "set_trader_enabled".to_owned(),
            description: "Start or stop the auto trader. Starting lets it open REAL positions \
                          with trader.trade_size_sol each; refused while setup is incomplete or \
                          the emergency stop is active. Stopping leaves open positions in place. \
                          Idempotent: a trader already in the requested state returns \
                          changed=false."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": { "enabled": { "type": "boolean", "description": "true to start, false to stop" } },
                "required": ["enabled"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: EnabledParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        let result = if params.enabled {
            trader::start_trader_checked().await
        } else {
            trader::stop_trader_checked().await
        };
        match result {
            Ok(status) => ToolResult::success(json!({ "trader": status, "changed": true })),
            // Already in the requested state: the outcome the caller asked for holds.
            Err(trader::Error::AlreadyRunning | trader::Error::AlreadyStopped) => {
                ToolResult::success(json!({ "trader": trader::trader_status(), "changed": false }))
            }
            Err(e) => ToolResult::error(e.to_string()),
        }
    }
}

// ============================================================================
// SetTraderMonitorTool
// ============================================================================

pub struct SetTraderMonitorTool;

#[derive(Deserialize)]
struct MonitorParams {
    monitor: Monitor,
    enabled: bool,
}

#[async_trait]
impl Tool for SetTraderMonitorTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "set_trader_monitor".to_owned(),
            description: "Enable or disable one auto-trader monitor. entry: whether the trader \
                          opens new positions. exit: whether it manages open ones (stop loss, \
                          trailing, ROI, time). Disabling exit leaves positions unprotected."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "monitor": { "type": "string", "enum": ["entry", "exit"] },
                    "enabled": { "type": "boolean" }
                },
                "required": ["monitor", "enabled"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: MonitorParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        match trader::set_monitor_enabled(params.monitor, params.enabled) {
            Ok(()) => ToolResult::success(json!({ "monitors": trader::monitors_status() })),
            Err(e) => ToolResult::error(e.to_string()),
        }
    }
}

// ============================================================================
// ManageLossLimitTool
// ============================================================================

pub struct ManageLossLimitTool;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum LossLimitAction {
    Resume,
    Reset,
}

#[derive(Deserialize)]
struct LossLimitParams {
    action: LossLimitAction,
}

#[async_trait]
impl Tool for ManageLossLimitTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "manage_loss_limit".to_owned(),
            description: "Act on the period loss limit that pauses new entries. resume: lift the \
                          current pause. reset: zero the accumulated loss and start a new period."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": { "action": { "type": "string", "enum": ["resume", "reset"] } },
                "required": ["action"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: LossLimitParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        match params.action {
            LossLimitAction::Resume => trader::safety::loss_limit::resume_from_loss_limit(),
            LossLimitAction::Reset => trader::safety::loss_limit::reset_loss_limit_state(),
        }
        ToolResult::success(json!({ "loss_limit": trader::loss_limit_snapshot() }))
    }
}

// ============================================================================
// Templates
// ============================================================================

pub struct ListTraderTemplatesTool;

#[async_trait]
impl Tool for ListTraderTemplatesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_trader_templates".to_owned(),
            description: "List the preset exit templates (trailing stop, ROI target, time \
                          override bundles) that apply_trader_template can apply."
                .to_owned(),
            category: ToolCategory::Config,
            parameters: no_params(),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> ToolResult {
        ToolResult::success(json!({ "templates": trader::templates::all_templates() }))
    }
}

pub struct ApplyTraderTemplateTool;

#[derive(Deserialize)]
struct TemplateParams {
    template_id: String,
}

#[async_trait]
impl Tool for ApplyTraderTemplateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "apply_trader_template".to_owned(),
            description: "Apply a preset exit template to the auto-trader config (trailing stop, \
                          ROI exit, time override) and save it. See list_trader_templates."
                .to_owned(),
            category: ToolCategory::Config,
            parameters: json!({
                "type": "object",
                "properties": { "template_id": { "type": "string", "description": "Template id, e.g. conservative, balanced, aggressive, day_trade" } },
                "required": ["template_id"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: TemplateParams = match parse(params) {
            Ok(p) => p,
            Err(e) => return e,
        };
        match trader::templates::apply_template(&params.template_id) {
            Ok(template) => ToolResult::success(json!({ "applied": template })),
            Err(e) => ToolResult::error(e.to_string()),
        }
    }
}
