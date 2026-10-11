// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * One trade action as a row of the Actions drawer, and the day groups the rows sit in.
 *
 * A row states what happened in one line: status, verb and token, who started it, what
 * the swap spent and received (with the round's P&L on a closing sell), and when. Its
 * detail lists the step timeline, the transaction, price, fee, route and duration, and
 * the ways to the token, the position and the transaction. Everything here reads the
 * action as `/api/actions/history` sends it, including `outcome`, the position record
 * booked under the swap's signature; a value the backend did not record is left out.
 */

import {
  formatCompactNumber,
  formatDate,
  formatDuration,
  formatPercentValue,
  formatSignedSol,
  formatPriceSubscript,
  formatSol,
  formatTimeAgo,
  formatTimestamp,
  formatUptime,
  signedTone,
  withSolUnit,
} from "../core/format.js";
import { escapeHtml } from "../core/utils.js";
import { apiErrorMessage } from "../core/request_manager.js";
import { routerName, stepMessage, symbolOf, outcomeMessage } from "../core/action_message.js";
import { stepLabel } from "./action_step.js";
import { closeReasonText } from "./trade_reason.js";
import { getIdentity, renderSignature, renderTokenLogo } from "./token_identity.js";

const HOUR_MS = 60 * 60 * 1000;

// Action type ids serialized by `ActionType` (src/actions/types.rs).
const ACTION_TYPE_LABELS = Object.freeze({
  swap_buy: "notifications-action-swap-buy",
  swap_sell: "notifications-action-swap-sell",
  position_open: "notifications-action-position-open",
  position_close: "notifications-action-position-close",
  position_dca: "notifications-action-position-dca",
  position_partial_exit: "notifications-action-position-partial-exit",
  manual_order: "notifications-action-manual-order",
  unknown: "notifications-action-unknown",
});

// Values returned by `actionSource`.
const SOURCE_LABELS = Object.freeze({
  auto: "notifications-source-auto",
  copy: "notifications-source-copy",
  manual: "notifications-source-manual",
});

// Status ids of `ActionState` (src/actions/types.rs) and the glyph each shows.
const STATUS_ICONS = Object.freeze({
  in_progress: "icon-loader",
  completed: "icon-circle-check",
  failed: "icon-circle-x",
  cancelled: "icon-ban",
});

// Status ids of `StepStatus` (src/actions/types.rs) and the glyph each shows.
const STEP_ICONS = Object.freeze({
  pending: "icon-circle",
  in_progress: "icon-loader",
  completed: "icon-circle-check",
  failed: "icon-circle-x",
  skipped: "icon-circle-minus",
});

const SELL_TYPES = new Set(["swap_sell", "position_close", "position_partial_exit"]);

export function actionStatus(action) {
  return action?.state?.status || "";
}

/** Who started the action: a copy-trading task, the auto trader, or the user. */
export function actionSource(action) {
  const metadata = action?.metadata || {};
  if (metadata.origin === "copy") return "copy";
  const operation = String(metadata.operation || "").toLowerCase();
  if (operation.startsWith("auto")) return "auto";
  if (operation.startsWith("manual")) return "manual";
  return "";
}

/** The moment a row is listed under: when it finished, else when it started. */
export function actionTime(action) {
  return action?.completed_at || action?.started_at || action?.timestamp || "";
}

function toDate(value) {
  const date = value instanceof Date ? value : new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

function dayKey(date) {
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
}

/** "Today", "Yesterday", else the weekday and date (with the year outside the current one). */
export function dayHeading(value, now = new Date()) {
  const date = toDate(value);
  if (!date) return "";
  const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
  if (dayKey(date) === dayKey(now)) return I18n.t("notifications-day-today");
  if (dayKey(date) === dayKey(yesterday)) return I18n.t("notifications-day-yesterday");
  return formatDate(date, {
    weekday: true,
    includeYear: date.getFullYear() !== now.getFullYear(),
  });
}

/** Actions in their given order, split into runs of one local calendar day. */
export function groupByDay(actions, now = new Date()) {
  const groups = [];
  for (const action of actions) {
    const date = toDate(actionTime(action));
    const key = date ? dayKey(date) : "";
    const last = groups[groups.length - 1];
    if (last && last.key === key) {
      last.actions.push(action);
    } else {
      groups.push({ key, label: date ? dayHeading(date, now) : "", actions: [action] });
    }
  }
  return groups;
}

/** Relative within the last hour ("5m ago"), the time of day after that ("14:05"). */
export function timeLabel(value, now = new Date()) {
  const date = toDate(value);
  if (!date) return "";
  if (now.getTime() - date.getTime() < HOUR_MS) return formatTimeAgo(date);
  return formatTimestamp(date, { includeDate: false, includeSeconds: false });
}

/** Elapsed time since `value`, for a running action. */
export function elapsedLabel(value, now = new Date()) {
  const date = toDate(value);
  if (!date) return "";
  const seconds = Math.max(0, Math.round((now.getTime() - date.getTime()) / 1000));
  return formatUptime(seconds, { style: "trimmed" });
}

function durationLabel(action) {
  const start = toDate(action?.started_at);
  const end = toDate(action?.completed_at);
  if (!start || !end) return "";
  const ms = end.getTime() - start.getTime();
  if (!(ms > 0)) return "";
  const duration_ns = ms * 1_000_000;
  return ms < 60_000 ? formatDuration(duration_ns) : formatUptime(ms / 1000, { style: "trimmed" });
}

function isPartialSell(action) {
  const outcomeKind = action?.outcome?.kind;
  if (outcomeKind) return outcomeKind === "partial_exit";
  const percentage = Number(action?.metadata?.percentage);
  return action?.action_type === "swap_sell" && Number.isFinite(percentage) && percentage < 100;
}

function verbLabel(action) {
  if (isPartialSell(action)) return I18n.label(ACTION_TYPE_LABELS, "position_partial_exit");
  return I18n.label(ACTION_TYPE_LABELS, action?.action_type || "unknown");
}

function tokenIdentity(action) {
  const mint = action?.entity_id || action?.metadata?.mint || "";
  const identity = getIdentity(mint);
  return { ...identity, symbol: identity.symbol || symbolOf(action) || null };
}

function failureText(action) {
  const failedStep = (action?.steps || []).find((step) => step?.status === "failed");
  return apiErrorMessage(
    { error: action?.state?.error || failedStep?.error },
    I18n.t("actions-failure-unknown")
  );
}

/** The amount of the traded token, "12.3K WIF", or "" when it is not known. */
function tokenAmount(amount, symbol) {
  if (amount === null || amount === undefined || !Number.isFinite(Number(amount))) return "";
  return `${formatCompactNumber(amount)} ${symbol || ""}`.trim();
}

function flow(from, to) {
  if (!from || !to) return escapeHtml(from || to || "");
  return `<span class="action-row__flow"><span dir="ltr">${escapeHtml(from)}</span><i class="icon-arrow-right" aria-hidden="true"></i><span dir="ltr">${escapeHtml(to)}</span></span>`;
}

/** The round's P&L, amount and percent in one tone (`signedTone` of the amount). */
function pnlMarkup(outcome, className) {
  if (outcome?.realized_pnl === null || outcome?.realized_pnl === undefined) return "";
  const tone = signedTone(outcome.realized_pnl);
  const percent =
    outcome.realized_pnl_percent === null || outcome.realized_pnl_percent === undefined
      ? ""
      : ` ${escapeHtml(formatPercentValue(outcome.realized_pnl_percent, { decimals: 1 }))}`;
  return `<span class="${className} pnl-${tone}">${escapeHtml(formatSignedSol(outcome.realized_pnl))}${percent}</span>`;
}

/** The one-line result: what the swap spent and received, or why it stopped. */
function resultMarkup(action, symbol, now) {
  const status = actionStatus(action);
  if (status === "in_progress") {
    const step = stepMessage(action) || "";
    return `<span class="action-row__step">${escapeHtml(step)}</span><span class="action-row__elapsed" data-elapsed-from="${escapeHtml(action.started_at || "")}">${escapeHtml(elapsedLabel(action.started_at, now))}</span>`;
  }
  if (status === "failed") {
    return `<span class="action-row__error">${escapeHtml(failureText(action))}</span>`;
  }
  if (status === "cancelled") {
    return `<span class="action-row__muted">${escapeHtml(I18n.t("notifications-cancelled"))}</span>`;
  }

  const outcome = action?.outcome;
  if (outcome) {
    const sol = formatSol(outcome.native_amount);
    const tokens = tokenAmount(outcome.token_amount, symbol);
    const isExit = outcome.kind === "exit" || outcome.kind === "partial_exit";
    return isExit ? flow(tokens, sol) : flow(sol, tokens);
  }
  const requested = outcomeMessage(action);
  const requestedHtml = requested
    ? `<span class="action-row__muted">${escapeHtml(requested)}</span>`
    : "";
  if (action?.unrecorded) {
    return `${requestedHtml}<span class="action-row__warning">${escapeHtml(I18n.t("notifications-unrecorded"))}</span>`;
  }
  return requestedHtml;
}

/** One label/value pair of the detail; `hint` explains the label in the shared tooltip. */
function fact(label, valueHtml, hint = "") {
  if (!valueHtml) return "";
  const tooltip = hint ? ` data-tooltip="${escapeHtml(hint)}"` : "";
  return `<div class="action-detail__fact"><dt${tooltip}>${escapeHtml(label)}</dt><dd>${valueHtml}</dd></div>`;
}

function stepTime(step) {
  const date = toDate(step?.completed_at || step?.started_at);
  return date ? formatTimestamp(date, { includeDate: false }) : "";
}

function stepsMarkup(action) {
  const steps = Array.isArray(action?.steps) ? action.steps : [];
  if (steps.length === 0) return "";
  const items = steps
    .map((step) => {
      const status = STEP_ICONS[step?.status] ? step.status : "pending";
      const error =
        status === "failed" && step.error
          ? `<span class="action-step__error">${escapeHtml(apiErrorMessage({ error: step.error }, ""))}</span>`
          : "";
      const time = stepTime(step);
      return `<li class="action-step action-step--${status}">
        <i class="action-step__icon ${STEP_ICONS[status]}" aria-hidden="true"></i>
        <span class="action-step__label">${escapeHtml(stepLabel(step?.name || "unknown"))}</span>
        ${time ? `<time class="action-step__time" dir="ltr">${escapeHtml(time)}</time>` : ""}
        ${error}
      </li>`;
    })
    .join("");
  return `<ol class="action-detail__steps" aria-label="${escapeHtml(I18n.t("notifications-steps"))}">${items}</ol>`;
}

function positionIdOf(action) {
  const id = action?.outcome?.position_id ?? action?.metadata?.position_id;
  const numeric = Number(id);
  return id !== null && id !== undefined && id !== "" && Number.isFinite(numeric) ? numeric : null;
}

function swapSignature(action) {
  if (action?.outcome?.signature) return action.outcome.signature;
  const steps = Array.isArray(action?.steps) ? action.steps : [];
  for (let index = steps.length - 1; index >= 0; index -= 1) {
    const signature = steps[index]?.metadata?.signature;
    // The trackers write "unknown" for a swap that reported no signature.
    if (typeof signature === "string" && signature && signature !== "unknown") return signature;
  }
  return "";
}

function detailMarkup(action, identity, detailId, expanded) {
  const outcome = action?.outcome || null;
  const metadata = action?.metadata || {};
  const signature = swapSignature(action);
  const positionId = positionIdOf(action);
  const status = actionStatus(action);

  const pnl = outcome ? pnlMarkup(outcome, "action-detail__pnl") : "";
  const facts = [
    fact(
      I18n.t("notifications-detail-transaction"),
      signature ? `<span class="action-detail__signature">${renderSignature(signature)}</span>` : ""
    ),
    fact(
      I18n.t("notifications-detail-fill-price"),
      outcome ? escapeHtml(withSolUnit(formatPriceSubscript(outcome.price))) : "",
      I18n.attr("notifications-detail-fill-price", "title")
    ),
    fact(
      I18n.t("notifications-detail-fee"),
      outcome && outcome.fees_native !== null && outcome.fees_native !== undefined
        ? escapeHtml(formatSol(outcome.fees_native, { decimals: 6 }))
        : ""
    ),
    fact(I18n.t("notifications-detail-pnl"), pnl),
    fact(I18n.t("notifications-detail-route"), escapeHtml(routerName(action))),
    fact(I18n.t("notifications-detail-duration"), escapeHtml(durationLabel(action))),
    fact(
      I18n.t("notifications-detail-reason"),
      typeof metadata.reason === "string" && metadata.reason
        ? escapeHtml(closeReasonText(metadata.reason))
        : ""
    ),
  ].join("");

  const error =
    status === "failed"
      ? `<p class="action-detail__error" role="note">${escapeHtml(failureText(action))}</p>`
      : action?.unrecorded
        ? `<p class="action-detail__warning" role="note">${escapeHtml(I18n.t("notifications-unrecorded-detail"))}</p>`
        : "";

  const mint = identity.mint || "";
  const links = [
    mint
      ? `<button type="button" class="action-detail__link" data-open-token="${escapeHtml(mint)}">${escapeHtml(I18n.t("notifications-view-token"))}</button>`
      : "",
    positionId !== null
      ? `<button type="button" class="action-detail__link" data-open-position="${escapeHtml(String(positionId))}">${escapeHtml(I18n.t("notifications-view-position"))}</button>`
      : "",
    signature
      ? `<button type="button" class="action-detail__link" data-open-transaction="${escapeHtml(signature)}">${escapeHtml(I18n.t("notifications-view-transaction"))}</button>`
      : "",
  ].join("");

  return `<div class="action-detail" id="${detailId}"${expanded ? "" : " hidden"}>
    ${error}
    ${stepsMarkup(action)}
    ${facts ? `<dl class="action-detail__facts">${facts}</dl>` : ""}
    ${links ? `<div class="action-detail__links">${links}</div>` : ""}
  </div>`;
}

/** One action as a compact row; `expanded` shows its detail. */
export function renderActionRow(action, { expanded = false, now = new Date() } = {}) {
  const status = actionStatus(action);
  const identity = tokenIdentity(action);
  const symbol = identity.symbol || "";
  const source = actionSource(action);
  const time = actionTime(action);
  const date = toDate(time);
  const id = escapeHtml(action?.id || "");
  const detailId = `action-detail-${id}`;
  const sell = SELL_TYPES.has(action?.action_type);

  const sourceHtml = source
    ? `<span class="action-row__source">${escapeHtml(I18n.label(SOURCE_LABELS, source))}</span>`
    : "";
  const pnlHtml =
    status === "completed" && action?.outcome ? pnlMarkup(action.outcome, "action-row__pnl") : "";
  const timeHtml = date
    ? `<time class="action-row__time" datetime="${escapeHtml(date.toISOString())}" data-relative-time="${escapeHtml(date.toISOString())}" title="${escapeHtml(formatTimestamp(date, { includeSeconds: false }))}">${escapeHtml(timeLabel(date, now))}</time>`
    : "";

  return `<article class="action-row action-row--${escapeHtml(status || "unknown")}${sell ? " action-row--sell" : ""}${action?.read ? "" : " is-unread"}${expanded ? " is-open" : ""}${action?.unrecorded ? " is-unrecorded" : ""}" data-id="${id}">
    <button type="button" class="action-row__summary" aria-expanded="${expanded ? "true" : "false"}" aria-controls="${detailId}">
      <i class="action-row__status ${action?.unrecorded ? "icon-triangle-alert" : STATUS_ICONS[status] || "icon-circle"}" aria-hidden="true"></i>
      <span class="action-row__main">
        <span class="action-row__title">
          <span class="action-row__verb">${escapeHtml(verbLabel(action))}</span>
          <span class="action-row__token"><span class="action-row__logo" aria-hidden="true">${renderTokenLogo(identity, { size: "xs" })}</span><span class="action-row__symbol token-symbol-type">${escapeHtml(symbol || I18n.t("format-unknown"))}</span></span>
          ${sourceHtml}
        </span>
        <span class="action-row__result">${resultMarkup(action, symbol, now)}</span>
      </span>
      <span class="action-row__aside">${timeHtml}${pnlHtml}</span>
      <i class="action-row__chevron icon-chevron-down" aria-hidden="true"></i>
    </button>
    ${detailMarkup(action, identity, detailId, expanded)}
  </article>`;
}

/** Every action under its day heading. `expanded` holds the ids whose detail is open. */
export function renderActionList(actions, { expanded = new Set(), now = new Date() } = {}) {
  return groupByDay(actions, now)
    .map(
      (group) => `<section class="action-day" aria-label="${escapeHtml(group.label)}">
        <h4 class="action-day__heading">${escapeHtml(group.label)}</h4>
        ${group.actions.map((action) => renderActionRow(action, { expanded: expanded.has(action.id), now })).join("")}
      </section>`
    )
    .join("");
}
