// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Activity timeline event rendering for the Position Details dialog.
 *
 * The collapsed row says what happened in plain language. Accounting and chain-level fields
 * stay behind Details, so a long token history remains easy to scan.
 */
import * as Utils from "../../core/utils.js";
import { withSolUnit } from "../../core/format.js";
import { directionLabel } from "../transaction_direction.js";
import { TRANSACTION_STATUS_LABELS } from "../transaction_status.js";

const KIND_ICONS = Object.freeze({
  entry: "icon-circle-arrow-down",
  dca: "icon-circle-arrow-down",
  partial_exit: "icon-circle-arrow-up",
  exit: "icon-circle-arrow-up",
  buy: "icon-circle-arrow-down",
  sell: "icon-circle-arrow-up",
  transfer: "icon-arrow-right-left",
  ata: "icon-wallet",
  other: "icon-activity",
});

// Kinds are the `ActivityEvent.kind` values of the activity endpoint.
const EVENT_KIND_LABELS = Object.freeze({
  entry: "positions-event-kind-entry",
  dca: "positions-event-kind-dca",
  partial_exit: "positions-event-kind-partial-exit",
  exit: "positions-event-kind-exit",
  buy: "positions-event-kind-buy",
  sell: "positions-event-kind-sell",
  transfer: "positions-event-kind-transfer",
  ata: "positions-event-kind-ata",
  other: "positions-event-kind-other",
});

const EVENT_STATE_LABELS = Object.freeze({
  pending: "positions-event-state-pending",
  failed: "positions-event-state-failed",
  synthetic: "positions-event-state-synthetic",
});

// Plain chain statuses; a failed status carries the error after "Failed:".
const PLAIN_CHAIN_STATUSES = ["Pending", "Confirmed", "Finalized"];
const FAILED_STATUS_PREFIX = "Failed:";

const esc = (text) => Utils.escapeHtml(text);

function chainStatusText(status) {
  if (PLAIN_CHAIN_STATUSES.includes(status)) return I18n.label(TRANSACTION_STATUS_LABELS, status);
  if (status.startsWith(FAILED_STATUS_PREFIX)) {
    return I18n.t("positions-chain-status-failed-detail", {
      error: status.slice(FAILED_STATUS_PREFIX.length).trim(),
    });
  }
  return I18n.label(TRANSACTION_STATUS_LABELS, "Failed");
}

export function activityEventKey(event) {
  return event.signature || `${event.kind}:${event.side}:${event.position_index}:${event.sequence}`;
}

function metric(label, value, tone = "") {
  if (value === null || value === undefined || value === "") return "";
  return `
    <div class="pdd-act-detail-metric">
      <span>${esc(label)}</span>
      <strong class="${tone}">${value}</strong>
    </div>`;
}

/** Plain text of what happened; the caller escapes it once. */
function eventDescription(event, ctx) {
  const amount = event.token_amount;
  const submitted = event.side !== "wallet" && !event.recorded;
  const amountText =
    amount != null && amount !== 0
      ? `${Utils.formatCompactNumber(amount)} ${ctx.symbol}`
      : I18n.t("positions-event-tokens-fallback");
  const solText = event.sol_amount != null ? ctx.formatSol(event.sol_amount) : null;
  const args = { amount: amountText, sol: solText };

  switch (event.kind) {
    case "entry":
      if (submitted) return I18n.t("positions-event-entry-submitted", args);
      return solText
        ? I18n.t("positions-event-entry-for", args)
        : I18n.t("positions-event-entry", args);
    case "dca":
      if (submitted) return I18n.t("positions-event-dca-submitted", args);
      return solText
        ? I18n.t("positions-event-dca-for", args)
        : I18n.t("positions-event-dca", args);
    case "partial_exit": {
      const percent =
        event.exit_percentage != null
          ? Utils.formatPercentValue(event.exit_percentage, { decimals: 0, includeSign: false })
          : null;
      if (submitted) {
        return percent
          ? I18n.t("positions-event-partial-exit-submitted-percent", { ...args, percent })
          : I18n.t("positions-event-partial-exit-submitted", args);
      }
      if (percent) {
        return solText
          ? I18n.t("positions-event-sold-percent-for", { ...args, percent })
          : I18n.t("positions-event-sold-percent", { ...args, percent });
      }
      return solText
        ? I18n.t("positions-event-sold-for", args)
        : I18n.t("positions-event-sold", args);
    }
    case "exit":
      if (submitted) return I18n.t("positions-event-exit-submitted");
      return solText
        ? I18n.t("positions-event-exit-for", args)
        : I18n.t("positions-event-exit-closed");
    case "buy":
      return I18n.t("positions-event-wallet-bought", args);
    case "sell":
      return I18n.t("positions-event-wallet-sold", args);
    case "transfer":
      if (event.direction === "Incoming") return I18n.t("positions-event-received", args);
      if (event.direction === "Outgoing") return I18n.t("positions-event-sent", args);
      return I18n.t("positions-event-transferred", args);
    case "ata":
      return I18n.t("positions-event-ata");
    default:
      return I18n.t("positions-event-wallet-transaction", args);
  }
}

function eventOutcome(event, ctx) {
  if (event.side === "exit" && event.realized_pnl != null) {
    const pnl = event.realized_pnl;
    const tone = pnl > 0 ? "pdd-positive" : pnl < 0 ? "pdd-negative" : "";
    const pct =
      event.realized_pnl_percent != null
        ? ` (${Utils.formatPercentValue(event.realized_pnl_percent, { decimals: 2 })})`
        : "";
    return `<span class="pdd-act-outcome ${tone}">${ctx.formatSol(pnl, { sign: true })}${pct}</span>`;
  }

  if (event.price != null) {
    return `<span class="pdd-act-outcome">${esc(I18n.t("positions-event-price-per-token", { price: ctx.formatPrice(event.price) }))}</span>`;
  }

  if (event.side === "wallet" && event.sol_change != null && event.sol_change !== 0) {
    return `<span class="pdd-act-outcome">${esc(I18n.t("positions-event-wallet-change", { amount: ctx.formatSol(event.sol_change, { sign: true }) }))}</span>`;
  }

  return "";
}

function renderPositionAfter(event, ctx) {
  if (event.tokens_after == null || event.invested_after == null) return "";
  const avgEntry = event.tokens_after > 0 ? event.invested_after / event.tokens_after : null;

  return `
    <section class="pdd-act-position-after">
      <h4>${esc(I18n.t("positions-event-after-title"))}</h4>
      <div class="pdd-act-detail-grid">
        ${metric(
          I18n.t("positions-fact-holding"),
          `${Utils.formatCompactNumber(event.tokens_after)} ${Utils.escapeHtml(ctx.symbol)}`
        )}
        ${metric(I18n.t("positions-event-capital-invested"), ctx.formatSol(event.invested_after))}
        ${metric(
          I18n.t("positions-event-average-entry"),
          avgEntry ? withSolUnit(ctx.formatPrice(avgEntry)) : null
        )}
      </div>
    </section>`;
}

function renderTransfers(event) {
  if (!event.token_transfers?.length) return "";

  const rows = event.token_transfers
    .map(
      (transfer) => `
      <tr>
        <td class="pdd-act-xfer-amount">${Utils.formatCompactNumber(transfer.amount)}</td>
        <td dir="ltr">${Utils.formatAddressCompact(transfer.mint)}</td>
        <td dir="ltr">${Utils.formatAddressCompact(transfer.from)}</td>
        <td dir="ltr">${Utils.formatAddressCompact(transfer.to)}</td>
      </tr>`
    )
    .join("");

  return `
    <section class="pdd-act-xfers">
      <h4>${esc(I18n.t("positions-event-transfers-title"))}</h4>
      <table class="pdd-act-xfer-table">
        <thead><tr><th>${esc(I18n.t("positions-event-transfer-amount"))}</th><th>${esc(I18n.t("positions-event-transfer-mint"))}</th><th>${esc(I18n.t("positions-event-transfer-from"))}</th><th>${esc(I18n.t("positions-event-transfer-to"))}</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </section>`;
}

function renderSignature(event) {
  if (!event.signature) return `<span class="pdd-act-sig-na">${esc(I18n.t("positions-event-no-signature"))}</span>`;
  const signature = Utils.escapeHtml(event.signature);
  return `
    <div class="pdd-act-signature">
      <span class="pdd-act-sig" dir="ltr" data-copy="${signature}" title="${esc(I18n.t("positions-event-click-to-copy"))}">${Utils.formatSignatureCompact(event.signature, { start: 10, end: 10 })}</span>
      <button type="button" class="pdd-act-sig-copy" data-copy="${signature}"><i class="icon-copy"></i>${esc(I18n.t("common-action-copy"))}</button>
      <a href="${Utils.solscanTxUrl(event.signature)}" target="_blank" rel="noopener" class="pdd-act-sig-link"><i class="icon-external-link"></i>${esc(I18n.t("positions-event-solscan"))}</a>
    </div>`;
}

function renderDetails(event, ctx) {
  const fee = event.fee_sol ?? event.record_fee_native;
  const details = [
    metric(
      I18n.t("positions-event-token-amount"),
      event.token_amount != null ? Utils.formatNumber(event.token_amount) : null
    ),
    metric(I18n.t("positions-event-trade-price"), event.price != null ? withSolUnit(ctx.formatPrice(event.price)) : null),
    metric(I18n.t("positions-event-native-amount"), event.sol_amount != null ? ctx.formatSol(event.sol_amount) : null),
    metric(I18n.t("positions-event-cost-basis"), event.cost_basis != null ? ctx.formatSol(event.cost_basis) : null),
    metric(
      I18n.t("positions-event-usd-value"),
      event.sol_amount != null && ctx.solPriceUsd
        ? Utils.formatCurrencyUSD(event.sol_amount * ctx.solPriceUsd)
        : null
    ),
    metric(I18n.t("positions-event-network-fee"), fee != null && fee > 0 ? ctx.formatSol(fee) : null),
    metric(I18n.t("positions-event-router"), event.router ? Utils.escapeHtml(event.router) : null),
    metric(I18n.t("positions-event-slot"), event.slot != null ? Utils.formatNumber(event.slot, 0) : null),
    metric(
      I18n.t("positions-event-chain-status"),
      event.status ? esc(chainStatusText(event.status)) : null
    ),
    metric(
      I18n.t("positions-event-transaction-type"),
      event.transaction_type ? Utils.escapeHtml(I18n.text(event.transaction_type)) : null
    ),
    metric(
      I18n.t("positions-event-direction"),
      event.direction ? esc(directionLabel(event.direction)) : null
    ),
    metric(
      I18n.t("positions-event-wallet-native-change"),
      event.sol_change != null ? ctx.formatSol(event.sol_change, { sign: true }) : null
    ),
    metric(I18n.t("positions-event-instructions"), event.instructions_count ?? null),
    metric(
      I18n.t("positions-event-compute-units"),
      event.compute_units != null ? Utils.formatNumber(event.compute_units, 0) : null
    ),
    metric(I18n.t("positions-event-accounts"), event.accounts_count ?? null),
    metric(I18n.t("positions-event-record-id"), event.record_id ?? null),
  ]
    .filter(Boolean)
    .join("");

  const note = event.notes
    ? `<div class="pdd-act-note${event.state === "failed" ? " is-error" : ""}"><i class="icon-info"></i>${Utils.escapeHtml(event.notes)}</div>`
    : "";

  return `
    <div class="pdd-act-details">
      ${note}
      ${renderPositionAfter(event, ctx)}
      ${details ? `<div class="pdd-act-detail-grid">${details}</div>` : ""}
      ${renderSignature(event)}
      ${renderTransfers(event)}
    </div>`;
}

export function renderActivityCard(event, ctx) {
  const key = Utils.escapeHtml(activityEventKey(event));
  const kind = Object.hasOwn(KIND_ICONS, event.kind) ? event.kind : "other";
  const kindLabel = I18n.label(EVENT_KIND_LABELS, kind);
  const stateLabel = Object.hasOwn(EVENT_STATE_LABELS, event.state)
    ? I18n.label(EVENT_STATE_LABELS, event.state)
    : "";
  const expanded = ctx.expanded.has(activityEventKey(event));
  const time = event.timestamp
    ? Utils.formatTimestamp(event.timestamp)
    : I18n.t("positions-event-time-unavailable");
  const relative = event.timestamp ? Utils.formatTimeAgo(event.timestamp) : "";

  return `
    <article class="pdd-act-card${expanded ? " is-open" : ""}" data-side="${Utils.escapeHtml(event.side)}" data-kind="${Utils.escapeHtml(event.kind)}" data-state="${Utils.escapeHtml(event.state)}" data-key="${key}" data-ts="${Number(event.timestamp) || ""}" data-price="${Number(event.price) || ""}">
      <i class="pdd-act-glyph ${KIND_ICONS[kind]}" aria-hidden="true"></i>
      <button type="button" class="pdd-act-expand" data-expand="${key}" aria-expanded="${expanded}">
        <span class="pdd-act-main">
          <span class="pdd-act-event-topline">
            <strong>${esc(kindLabel)}</strong>
            ${stateLabel ? `<span class="pdd-act-state is-${event.state}">${esc(stateLabel)}</span>` : ""}
          </span>
          <span class="pdd-act-description">${esc(eventDescription(event, ctx))}</span>
          <span class="pdd-act-event-time" title="${esc(time)}">${esc(time)}${relative ? ` · ${esc(relative)}` : ""}</span>
        </span>
        <span class="pdd-act-event-side">
          ${eventOutcome(event, ctx)}
          <span class="pdd-act-details-label">${esc(expanded ? I18n.t("positions-event-hide-details") : I18n.t("positions-event-details"))}<i class="icon-chevron-down"></i></span>
        </span>
      </button>
      ${renderDetails(event, ctx)}
    </article>`;
}
