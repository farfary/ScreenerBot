// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for the agent tools, in one place.
 *
 * Ids are the tool names registered by `create_tool_registry`
 * (src/agent_control/tools/mod.rs). The message key is the id with hyphens.
 * Tool ids stay the wire identifier; only the label is localized.
 */

/** Message key of each agent tool label. */
export const AGENT_TOOL_LABELS = Object.freeze({
  analyze_token: "assistant-tool-analyze-token",
  get_market_data: "assistant-tool-get-market-data",
  check_security: "assistant-tool-check-security",
  get_positions: "assistant-tool-get-positions",
  get_position: "assistant-tool-get-position",
  get_balance: "assistant-tool-get-balance",
  get_pnl: "assistant-tool-get-pnl",
  buy_token: "assistant-tool-buy-token",
  add_to_position: "assistant-tool-add-to-position",
  sell_token: "assistant-tool-sell-token",
  close_position: "assistant-tool-close-position",
  get_trade_status: "assistant-tool-get-trade-status",
  get_config: "assistant-tool-get-config",
  describe_config: "assistant-tool-describe-config",
  update_config: "assistant-tool-update-config",
  get_status: "assistant-tool-get-status",
  get_events: "assistant-tool-get-events",
  force_stop: "assistant-tool-force-stop",
  clear_force_stop: "assistant-tool-clear-force-stop",
  get_trader_status: "assistant-tool-get-trader-status",
  get_trader_stats: "assistant-tool-get-trader-stats",
  set_trader_enabled: "assistant-tool-set-trader-enabled",
  set_trader_monitor: "assistant-tool-set-trader-monitor",
  manage_loss_limit: "assistant-tool-manage-loss-limit",
  list_trader_templates: "assistant-tool-list-trader-templates",
  apply_trader_template: "assistant-tool-apply-trader-template",
  get_copy_trading_overview: "assistant-tool-get-copy-trading-overview",
  get_copy_task: "assistant-tool-get-copy-task",
  get_copy_activity: "assistant-tool-get-copy-activity",
  create_copy_task: "assistant-tool-create-copy-task",
  update_copy_task: "assistant-tool-update-copy-task",
  delete_copy_task: "assistant-tool-delete-copy-task",
  set_copy_task_mode: "assistant-tool-set-copy-task-mode",
  get_copy_insights: "assistant-tool-get-copy-insights",
  get_copy_wallet_profile: "assistant-tool-get-copy-wallet-profile",
  clone_copy_task: "assistant-tool-clone-copy-task",
  reset_copy_paper_book: "assistant-tool-reset-copy-paper-book",
  close_copy_paper_holding: "assistant-tool-close-copy-paper-holding",
});
