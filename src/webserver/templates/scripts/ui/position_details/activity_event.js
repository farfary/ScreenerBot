/**
 * Compact activity timeline event rendering for the Position Details dialog.
 *
 * The collapsed row explains what happened in plain language. Accounting and chain-level
 * fields stay available behind Details, so a long token history remains easy to scan.
 */
import * as Utils from "../../core/utils.js";

const KIND_META = {
  entry: { label: "Entry", icon: "icon-circle-arrow-down" },
  dca: { label: "Add", icon: "icon-circle-arrow-down" },
  partial_exit: { label: "Partial exit", icon: "icon-circle-arrow-up" },
  exit: { label: "Exit", icon: "icon-circle-arrow-up" },
  buy: { label: "Wallet buy", icon: "icon-circle-arrow-down" },
  sell: { label: "Wallet sell", icon: "icon-circle-arrow-up" },
  transfer: { label: "Transfer", icon: "icon-arrow-right-left" },
  ata: { label: "Token account", icon: "icon-wallet" },
  other: { label: "Transaction", icon: "icon-activity" },
};

const STATE_META = {
  pending: { label: "Pending", icon: "icon-clock" },
  failed: { label: "Failed", icon: "icon-circle-x" },
  synthetic: { label: "Synthetic", icon: "icon-triangle-alert" },
};

export function activityEventKey(event) {
  return event.signature || `${event.kind}:${event.side}:${event.position_index}:${event.sequence}`;
}

const sol = (value, decimals = 4) => `${Utils.formatSol(value || 0, { decimals, suffix: "" })} SOL`;

function metric(label, value, tone = "") {
  if (value === null || value === undefined || value === "") return "";
  return `
    <div class="pdd-act-detail-metric">
      <span>${label}</span>
      <strong class="${tone}">${value}</strong>
    </div>`;
}

function eventDescription(event, ctx) {
  const amount = event.token_amount;
  const submitted = event.side !== "wallet" && !event.recorded;
  const amountText =
    amount != null && amount !== 0
      ? `${Utils.formatCompactNumber(amount)} ${Utils.escapeHtml(ctx.symbol)}`
      : "tokens";
  const solText = event.sol_amount != null ? sol(event.sol_amount) : null;

  switch (event.kind) {
    case "entry":
      if (submitted) return `Submitted buy for ${amountText}`;
      return solText ? `Bought ${amountText} for ${solText}` : `Bought ${amountText}`;
    case "dca":
      if (submitted) return `Submitted add for ${amountText}`;
      return solText ? `Added ${amountText} for ${solText}` : `Added ${amountText}`;
    case "partial_exit": {
      const pctValue =
        event.exit_percentage != null ? `${Utils.formatNumber(event.exit_percentage, 0)}% ` : "";
      const pct =
        event.exit_percentage != null ? ` (${Utils.formatNumber(event.exit_percentage, 0)}%)` : "";
      if (submitted) return `Submitted ${pctValue}partial exit for ${amountText}`;
      return solText ? `Sold ${amountText}${pct} for ${solText}` : `Sold ${amountText}${pct}`;
    }
    case "exit":
      if (submitted) return "Submitted full position exit";
      return solText ? `Closed with ${amountText} sold for ${solText}` : "Closed the position";
    case "buy":
      return `Wallet acquired ${amountText} outside ScreenerBot`;
    case "sell":
      return `Wallet sold ${amountText} outside ScreenerBot`;
    case "transfer":
      if (event.direction === "Incoming") return `Received ${amountText}`;
      if (event.direction === "Outgoing") return `Sent ${amountText}`;
      return `Transferred ${amountText}`;
    case "ata":
      return "Token account activity";
    default:
      return `Wallet transaction involving ${amountText}`;
  }
}

function eventOutcome(event, ctx) {
  if (event.side === "exit" && event.realized_pnl != null) {
    const pnl = event.realized_pnl;
    const sign = pnl >= 0 ? "+" : "";
    const pct =
      event.realized_pnl_percent != null
        ? ` (${sign}${Utils.formatNumber(event.realized_pnl_percent, 2)}%)`
        : "";
    return `<span class="pdd-act-outcome ${pnl >= 0 ? "is-positive" : "is-negative"}">${sign}${sol(pnl)}${pct}</span>`;
  }

  if (event.price != null) {
    return `<span class="pdd-act-outcome">${ctx.formatPrice(event.price)} SOL / token</span>`;
  }

  if (event.side === "wallet" && event.sol_change != null && event.sol_change !== 0) {
    const sign = event.sol_change >= 0 ? "+" : "";
    return `<span class="pdd-act-outcome">${sign}${sol(event.sol_change, 6)} wallet change</span>`;
  }

  return "";
}

function renderPositionAfter(event, ctx) {
  if (event.tokens_after == null || event.invested_after == null) return "";
  const avgEntry = event.tokens_after > 0 ? event.invested_after / event.tokens_after : null;

  return `
    <section class="pdd-act-position-after">
      <h4>Position after this event</h4>
      <div class="pdd-act-detail-grid">
        ${metric(
          "Holding",
          `${Utils.formatCompactNumber(event.tokens_after)} ${Utils.escapeHtml(ctx.symbol)}`
        )}
        ${metric("Capital invested", sol(event.invested_after))}
        ${metric("Average entry", avgEntry ? `${ctx.formatPrice(avgEntry)} SOL` : null)}
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
        <td>${Utils.formatAddressCompact(transfer.mint)}</td>
        <td>${Utils.formatAddressCompact(transfer.from)}</td>
        <td>${Utils.formatAddressCompact(transfer.to)}</td>
      </tr>`
    )
    .join("");

  return `
    <section class="pdd-act-xfers">
      <h4>Token transfers</h4>
      <table class="pdd-act-xfer-table">
        <thead><tr><th>Amount</th><th>Mint</th><th>From</th><th>To</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </section>`;
}

function renderSignature(event) {
  if (!event.signature) return '<span class="pdd-act-sig-na">No on-chain signature</span>';
  const signature = event.signature;
  return `
    <div class="pdd-act-signature">
      <span class="pdd-act-sig" data-copy="${signature}" title="Click to copy">${Utils.formatSignatureCompact(signature, { start: 10, end: 10 })}</span>
      <button type="button" class="pdd-act-sig-copy" data-copy="${signature}" title="Copy signature"><i class="icon-copy"></i></button>
      <a href="${Utils.solscanTxUrl(signature)}" target="_blank" rel="noopener" class="pdd-act-sig-link"><i class="icon-external-link"></i>Solscan</a>
    </div>`;
}

function renderDetails(event, ctx) {
  const fee = event.fee_sol ?? event.record_fee_sol;
  const details = [
    metric(
      "Token amount",
      event.token_amount != null ? Utils.formatNumber(event.token_amount) : null
    ),
    metric("Trade price", event.price != null ? `${ctx.formatPrice(event.price)} SOL` : null),
    metric("SOL amount", event.sol_amount != null ? sol(event.sol_amount) : null),
    metric("Cost basis", event.cost_basis != null ? sol(event.cost_basis) : null),
    metric(
      "USD value",
      event.sol_amount != null && ctx.solPriceUsd
        ? Utils.formatCurrencyUSD(event.sol_amount * ctx.solPriceUsd)
        : null
    ),
    metric("Network fee", fee != null && fee > 0 ? sol(fee, 6) : null),
    metric("Router", event.router ? Utils.escapeHtml(event.router) : null),
    metric("Slot", event.slot != null ? Utils.formatNumber(event.slot, 0) : null),
    metric("Chain status", event.status ? Utils.escapeHtml(event.status) : null),
    metric(
      "Transaction type",
      event.transaction_type ? Utils.escapeHtml(event.transaction_type) : null
    ),
    metric("Direction", event.direction ? Utils.escapeHtml(event.direction) : null),
    metric(
      "Wallet SOL change",
      event.sol_change != null
        ? `${event.sol_change >= 0 ? "+" : ""}${sol(event.sol_change, 6)}`
        : null
    ),
    metric("Instructions", event.instructions_count ?? null),
    metric(
      "Compute units",
      event.compute_units != null ? Utils.formatNumber(event.compute_units, 0) : null
    ),
    metric("Accounts", event.accounts_count ?? null),
    metric("Record ID", event.record_id ?? null),
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
  const key = activityEventKey(event);
  const meta = KIND_META[event.kind] || KIND_META.other;
  const stateMeta = STATE_META[event.state];
  const expanded = ctx.expanded.has(key);
  const time = event.timestamp ? Utils.formatTimestamp(event.timestamp) : "Time unavailable";
  const relative = event.timestamp ? Utils.formatTimeAgo(event.timestamp) : "";

  return `
    <article class="pdd-act-card${expanded ? " is-open" : ""}" data-side="${event.side}" data-state="${event.state}" data-key="${Utils.escapeHtml(key)}">
      <span class="pdd-act-node"><i class="${meta.icon}"></i></span>
      <button type="button" class="pdd-act-expand" data-expand="${Utils.escapeHtml(key)}" aria-expanded="${expanded}">
        <span class="pdd-act-main">
          <span class="pdd-act-event-topline">
            <strong>${meta.label}</strong>
            ${stateMeta ? `<span class="pdd-act-state is-${event.state}"><i class="${stateMeta.icon}"></i>${stateMeta.label}</span>` : ""}
          </span>
          <span class="pdd-act-description">${eventDescription(event, ctx)}</span>
          <span class="pdd-act-event-time" title="${time}">${time}${relative ? ` · ${relative}` : ""}</span>
        </span>
        <span class="pdd-act-event-side">
          ${eventOutcome(event, ctx)}
          <span class="pdd-act-details-label">${expanded ? "Hide details" : "Details"}<i class="icon-chevron-down"></i></span>
        </span>
      </button>
      ${renderDetails(event, ctx)}
    </article>`;
}
