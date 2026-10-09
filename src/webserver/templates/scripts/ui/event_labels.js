// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for an event's category, subtype and severity, shared by the
 * Events page and the event details dialog.
 */
import { escapeHtml } from "../core/utils.js";

// `EventCategory` string ids (src/events/types.rs), plus the legacy `entry` and
// `learner` categories older rows and the category filter still carry.
export const EVENT_CATEGORY_LABELS = Object.freeze({
  swap: "events-category-swap",
  transaction: "events-category-transaction",
  pool: "events-category-pool",
  position: "events-category-position",
  token: "events-category-token",
  wallet: "events-category-wallet",
  trader: "events-category-trader",
  entry: "events-category-entry",
  system: "events-category-system",
  ohlcv: "events-category-ohlcv",
  rpc: "events-category-rpc",
  api: "events-category-api",
  security: "events-category-security",
  connectivity: "events-category-connectivity",
  filtering: "events-category-filtering",
  scheduled_task: "events-category-scheduled-task",
  learner: "events-category-learner",
  other: "events-category-other",
});

// `Severity` string ids (src/events/types.rs); `critical` is a stored legacy level.
export const EVENT_SEVERITY_LABELS = Object.freeze({
  info: "common-severity-info",
  warn: "common-severity-warning",
  error: "common-severity-error",
  critical: "common-severity-critical",
  debug: "common-severity-debug",
});

/**
 * Message key of each subtype code the event producers record (the `subtype` argument
 * of the recorders in src/events/recorders/, the pool service's `Event` rows, the
 * scheduled-task outcomes and the emergency-stop rows). `event_subtype_labels.test.mjs`
 * keeps this map complete against the Rust sources.
 */
export const EVENT_SUBTYPE_LABELS = Object.freeze({
  task_completed: "events-subtype-task-completed",
  task_failed: "events-subtype-task-failed",
  task_timed_out: "events-subtype-task-timed-out",
  auto_trading_started: "events-subtype-auto-trading-started",
  auto_trading_stopped: "events-subtype-auto-trading-stopped",
  auto_trading_error: "events-subtype-auto-trading-error",
  service_initialized: "events-subtype-service-initialized",
  service_start: "events-subtype-service-start",
  service_started: "events-subtype-service-started",
  service_stop: "events-subtype-service-stop",
  service_stopped: "events-subtype-service-stopped",
  trading_enabled: "events-subtype-trading-enabled",
  trading_disabled: "events-subtype-trading-disabled",
  entry_monitor_started: "events-subtype-entry-monitor-started",
  entry_monitor_error: "events-subtype-entry-monitor-error",
  exit_monitor_started: "events-subtype-exit-monitor-started",
  exit_monitor_error: "events-subtype-exit-monitor-error",
  strategy_evaluation_timeout: "events-subtype-strategy-evaluation-timeout",
  ForceStop: "events-subtype-force-stop",
  ForceStopCleared: "events-subtype-force-stop-cleared",
  entry_signal_generated: "events-subtype-entry-signal-generated",
  entry_capacity_guard: "events-subtype-entry-capacity-guard",
  entry_executed: "events-subtype-entry-executed",
  entry_execution_error: "events-subtype-entry-execution-error",
  entry_failed: "events-subtype-entry-failed",
  entry_submitted_unconfirmed: "events-subtype-entry-submitted-unconfirmed",
  entry_verified: "events-subtype-entry-verified",
  entry_not_landed: "events-subtype-entry-not-landed",
  entry_booked_as_add: "events-subtype-entry-booked-as-add",
  entry_settlement_parked: "events-subtype-entry-settlement-parked",
  opened: "events-subtype-opened",
  open_blocked: "events-subtype-open-blocked",
  open_blocked_by_open_round: "events-subtype-open-blocked-by-open-round",
  open_persist_failed: "events-subtype-open-persist-failed",
  pending_open_set: "events-subtype-pending-open-set",
  pending_open_cleared: "events-subtype-pending-open-cleared",
  dca_initiated: "events-subtype-dca-initiated",
  dca_submitted: "events-subtype-dca-submitted",
  dca_verified: "events-subtype-dca-verified",
  dca_failed: "events-subtype-dca-failed",
  exit_signal_roi_target: "events-subtype-exit-signal-roi-target",
  exit_signal_stop_loss: "events-subtype-exit-signal-stop-loss",
  exit_signal_trailing_stop: "events-subtype-exit-signal-trailing-stop",
  exit_signal_time_override: "events-subtype-exit-signal-time-override",
  exit_signal_strategy: "events-subtype-exit-signal-strategy",
  exit_signal_llm_analysis: "events-subtype-exit-signal-llm-analysis",
  closing_submitted: "events-subtype-closing-submitted",
  exit_blocked: "events-subtype-exit-blocked",
  exit_blocked_pending_sig: "events-subtype-exit-blocked-pending-sig",
  exit_verified: "events-subtype-exit-verified",
  exit_retry_cleared: "events-subtype-exit-retry-cleared",
  exit_residual_detected: "events-subtype-exit-residual-detected",
  exit_residual_unattributed: "events-subtype-exit-residual-unattributed",
  partial_exit_initiated: "events-subtype-partial-exit-initiated",
  partial_exit_submitted: "events-subtype-partial-exit-submitted",
  partial_exit_verified: "events-subtype-partial-exit-verified",
  partial_exit_failed: "events-subtype-partial-exit-failed",
  partial_exit_swap_failed: "events-subtype-partial-exit-swap-failed",
  closed_externally: "events-subtype-closed-externally",
  fill_after_force_close: "events-subtype-fill-after-force-close",
  late_fill_handed_over: "events-subtype-late-fill-handed-over",
  confirmed_swap_unbooked: "events-subtype-confirmed-swap-unbooked",
  verification_started: "events-subtype-verification-started",
  verification_finished: "events-subtype-verification-finished",
  verification_abandoned: "events-subtype-verification-abandoned",
  verification_worker_summary: "events-subtype-verification-worker-summary",
  copy_trade: "events-subtype-copy-trade",
  copy_failed: "events-subtype-copy-failed",
  copy_task_paused: "events-subtype-copy-task-paused",
  processed: "events-subtype-processed",
  healthy: "events-subtype-healthy",
  degraded: "events-subtype-degraded",
  unhealthy: "events-subtype-unhealthy",
  critical_endpoints_unhealthy: "events-subtype-critical-endpoints-unhealthy",
  snapshot_compute_start: "events-subtype-snapshot-compute-start",
  snapshot_compute_complete: "events-subtype-snapshot-compute-complete",
  snapshot_empty_store: "events-subtype-snapshot-empty-store",
  snapshot_refreshed: "events-subtype-snapshot-refreshed",
  token_passed: "events-subtype-token-passed",
  token_rejected: "events-subtype-token-rejected",
  token_discovered: "events-subtype-token-discovered",
  token_blacklisted: "events-subtype-token-blacklisted",
  discovery_run_complete: "events-subtype-discovery-run-complete",
  discovery_run_failed: "events-subtype-discovery-run-failed",
  force_update_complete: "events-subtype-force-update-complete",
  market_data_updated: "events-subtype-market-data-updated",
  market_data_update_failed: "events-subtype-market-data-update-failed",
  pool_canonical_changed: "events-subtype-pool-canonical-changed",
  pool_prefetch_failed: "events-subtype-pool-prefetch-failed",
  pool_snapshot_fallback: "events-subtype-pool-snapshot-fallback",
  pool_snapshot_fetch_error: "events-subtype-pool-snapshot-fetch-error",
  pool_snapshot_updated: "events-subtype-pool-snapshot-updated",
  pool_source_fetch_failed: "events-subtype-pool-source-fetch-failed",
  pool_sources_failed: "events-subtype-pool-sources-failed",
  pool_sources_unconfigured: "events-subtype-pool-sources-unconfigured",
  rugcheck_analysis: "events-subtype-rugcheck-analysis",
  initialization: "events-subtype-initialization",
  backfill_scheduled: "events-subtype-backfill-scheduled",
  cache_cleanup_failed: "events-subtype-cache-cleanup-failed",
  cache_expired: "events-subtype-cache-expired",
  cache_hit: "events-subtype-cache-hit",
  default_pool_changed: "events-subtype-default-pool-changed",
  empty_fetch: "events-subtype-empty-fetch",
  fetch_aggregate_attempt: "events-subtype-fetch-aggregate-attempt",
  fetch_aggregate_complete: "events-subtype-fetch-aggregate-complete",
  fetch_aggregate_error: "events-subtype-fetch-aggregate-error",
  fetch_attempt: "events-subtype-fetch-attempt",
  fetch_complete: "events-subtype-fetch-complete",
  fetch_success: "events-subtype-fetch-success",
  fetch_error: "events-subtype-fetch-error",
  fetch_failed: "events-subtype-fetch-failed",
  fetch_immediate_complete: "events-subtype-fetch-immediate-complete",
  fallback: "events-subtype-fallback",
  fallback_skipped_non_native_pool: "events-subtype-fallback-skipped-non-native-pool",
  gap_cleanup_failed: "events-subtype-gap-cleanup-failed",
  gap_detection_start: "events-subtype-gap-detection-start",
  gap_detection_complete: "events-subtype-gap-detection-complete",
  gap_detection_failed: "events-subtype-gap-detection-failed",
  gap_fill_complete: "events-subtype-gap-fill-complete",
  gap_fill_partial: "events-subtype-gap-fill-partial",
  gap_fill_inconclusive: "events-subtype-gap-fill-inconclusive",
  gap_fill_confirmed_no_trade: "events-subtype-gap-fill-confirmed-no-trade",
  gap_fill_failed: "events-subtype-gap-fill-failed",
  large_aggregation: "events-subtype-large-aggregation",
  pool_discovery_start: "events-subtype-pool-discovery-start",
  pool_discovery_complete: "events-subtype-pool-discovery-complete",
  pool_discovery_success: "events-subtype-pool-discovery-success",
  pool_discovery_empty: "events-subtype-pool-discovery-empty",
  pool_discovery_error: "events-subtype-pool-discovery-error",
  pool_discovery_failed: "events-subtype-pool-discovery-failed",
  pool_failure: "events-subtype-pool-failure",
  pool_registered: "events-subtype-pool-registered",
  pool_unavailable: "events-subtype-pool-unavailable",
  process_token_error: "events-subtype-process-token-error",
  rate_limit_hit: "events-subtype-rate-limit-hit",
  retention_backfill_failed: "events-subtype-retention-backfill-failed",
  token_missing: "events-subtype-token-missing",
  account_blacklisted_after_threshold: "events-subtype-account-blacklisted-after-threshold",
  accounts_not_found: "events-subtype-accounts-not-found",
  decoder_failed: "events-subtype-decoder-failed",
  discovery_tick_started: "events-subtype-discovery-tick-started",
  discovery_tick_completed: "events-subtype-discovery-tick-completed",
  get_multiple_accounts_success: "events-subtype-get-multiple-accounts-success",
  get_multiple_accounts_failed: "events-subtype-get-multiple-accounts-failed",
  incomplete_account_bundle: "events-subtype-incomplete-account-bundle",
  invalid_reserve_accounts: "events-subtype-invalid-reserve-accounts",
  pool_account_fetch_failed: "events-subtype-pool-account-fetch-failed",
  pool_blacklisted_missing_accounts: "events-subtype-pool-blacklisted-missing-accounts",
  pool_components_init_start: "events-subtype-pool-components-init-start",
  pool_components_initialized: "events-subtype-pool-components-initialized",
  pool_selection_changed: "events-subtype-pool-selection-changed",
  pool_service_start_attempt: "events-subtype-pool-service-start-attempt",
  pool_service_already_running: "events-subtype-pool-service-already-running",
  pool_service_component_init_failed: "events-subtype-pool-service-component-init-failed",
  pool_service_db_init_failed: "events-subtype-pool-service-db-init-failed",
  pool_service_not_running: "events-subtype-pool-service-not-running",
  pool_service_stop_attempt: "events-subtype-pool-service-stop-attempt",
  pool_service_stopped: "events-subtype-pool-service-stopped",
  price_calculation_started: "events-subtype-price-calculation-started",
  price_calculation_success: "events-subtype-price-calculation-success",
  price_calculation_failed: "events-subtype-price-calculation-failed",
  rpc_batch_started: "events-subtype-rpc-batch-started",
  rpc_batch_completed: "events-subtype-rpc-batch-completed",
  rpc_batch_failed: "events-subtype-rpc-batch-failed",
  unsupported_program: "events-subtype-unsupported-program",
});

/**
 * Codes a candle feed records behind its own label (`<feed>_fetch_success`,
 * src/ohlcvs/fetcher.rs and monitor.rs); the feed is named in the event payload.
 */
export const EVENT_FEED_SUBTYPE_CODES = Object.freeze([
  "fetch_attempt",
  "fetch_complete",
  "fetch_error",
  "fetch_failed",
  "fetch_success",
  "fallback",
]);

const SEVERITY_STYLES = {
  info: { variant: "", icon: "icon-info" },
  warn: { variant: " warning", icon: "icon-triangle-alert" },
  error: { variant: " error", icon: "icon-x" },
  critical: { variant: " error", icon: "icon-circle-alert" },
  debug: { variant: " secondary", icon: "icon-bug" },
};

export function eventCategoryLabel(category) {
  return I18n.label(EVENT_CATEGORY_LABELS, category);
}

/**
 * Text of an event subtype: the label of a known code or of a candle feed's code, else
 * the stored value (an API endpoint name, or a code an older build recorded).
 */
export function eventSubtypeLabel(subtype) {
  const code = String(subtype);
  if (Object.hasOwn(EVENT_SUBTYPE_LABELS, code)) return I18n.label(EVENT_SUBTYPE_LABELS, code);
  const feedCode = EVENT_FEED_SUBTYPE_CODES.find((suffix) => code.endsWith(`_${suffix}`));
  return feedCode ? I18n.label(EVENT_SUBTYPE_LABELS, feedCode) : code;
}

/** Severity badge markup, or an empty string when the event has no severity. */
export function severityBadge(severity) {
  if (!severity) return "";
  const lowered = String(severity).toLowerCase();
  const key = lowered === "warning" ? "warn" : lowered;
  const style = SEVERITY_STYLES[key];
  if (!style) return `<span class="badge">${escapeHtml(String(severity))}</span>`;
  return `<span class="badge${style.variant}"><i class="${style.icon}"></i> ${escapeHtml(I18n.label(EVENT_SEVERITY_LABELS, key))}</span>`;
}
