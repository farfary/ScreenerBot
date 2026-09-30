/**
 * Presentation for trade and close reasons, in one place.
 *
 * Ids are the `Debug` names of `TradeReason` (src/trader/types.rs) plus the
 * reasons written by src/positions. `positions.closed_reason` additionally
 * carries the `_pending_verification` suffix (`PENDING_VERIFICATION_SUFFIX`)
 * and `force_closed: <operator text>` (routes/positions/force_close.rs), which
 * `closeReasonText` resolves.
 */

/** Message key of each reason's label; keep in step with `TradeReason`. */
export const TRADE_REASON_LABELS = Object.freeze({
  StrategySignal: "trade-reason-strategy-signal",
  ManualEntry: "trade-reason-manual-entry",
  ForceBuy: "trade-reason-force-buy",
  CopyBuy: "trade-reason-copy-buy",
  DCAScheduled: "trade-reason-dca-scheduled",
  TakeProfit: "trade-reason-take-profit",
  StopLoss: "trade-reason-stop-loss",
  TrailingStop: "trade-reason-trailing-stop",
  TimeOverride: "trade-reason-time-override",
  StrategyExit: "trade-reason-strategy-exit",
  LlmAnalysisExit: "trade-reason-llm-analysis-exit",
  ManualExit: "trade-reason-manual-exit",
  RiskManagement: "trade-reason-risk-management",
  Blacklisted: "trade-reason-blacklisted",
  ForceSell: "trade-reason-force-sell",
  CopySell: "trade-reason-copy-sell",
  closed_externally: "trade-reason-closed-externally",
  wallet_history: "trade-reason-wallet-history",
  exit_retry_pending: "trade-reason-exit-retry-pending",
  synthetic_exit_permanent_failure: "trade-reason-synthetic-exit-permanent-failure",
});

const PENDING_VERIFICATION_SUFFIX = "_pending_verification";
const FORCE_CLOSED_PREFIX = "force_closed:";

/**
 * Plain text for a stored reason. The result is not HTML-safe: it can carry
 * operator text, so the caller escapes it once when it renders.
 */
export function closeReasonText(raw) {
  const value = String(raw ?? "");
  if (value.endsWith(PENDING_VERIFICATION_SUFFIX)) {
    const base = value.slice(0, -PENDING_VERIFICATION_SUFFIX.length);
    return I18n.t("trade-reason-pending-verification", { reason: closeReasonText(base) });
  }
  if (value.startsWith(FORCE_CLOSED_PREFIX)) {
    return I18n.t("trade-reason-force-closed", {
      note: value.slice(FORCE_CLOSED_PREFIX.length).trim(),
    });
  }
  if (Object.hasOwn(TRADE_REASON_LABELS, value)) return I18n.label(TRADE_REASON_LABELS, value);
  return value;
}
