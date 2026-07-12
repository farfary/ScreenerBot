/**
 * Activity event card — one swap of a position, rendered from the merged event the
 * backend serves (`/api/positions/{key}/details` → `activity`).
 *
 * Each event already carries BOTH halves of the swap: the position record (booked amount,
 * price, SOL, exit percentage) and the on-chain transaction (status, fee, router, slot,
 * transfers), plus the running position state after it settled. Nothing is joined here.
 *
 * Pure render functions — no dialog state, no listeners. The tab wires interaction with a
 * single delegated handler.
 */
import * as Utils from "../../core/utils.js";

const KIND_LABEL = {
  entry: "Entry",
  dca: "DCA Entry",
  partial_exit: "Partial Exit",
  exit: "Exit",
};

const STATE_META = {
  confirmed: { label: "Confirmed", icon: "icon-circle-check" },
  pending: { label: "Pending", icon: "icon-clock" },
  failed: { label: "Failed", icon: "icon-circle-x" },
  synthetic: { label: "Synthetic", icon: "icon-triangle-alert" },
};

/**
 * Stable identity for an event. A swap still confirming has no record id, and a synthetic
 * close has no signature at all, so fall back to its slot in the timeline.
 */
export function activityEventKey(event) {
  return event.signature || `${event.kind}:${event.side}:${event.sequence}`;
}

/** Human label, numbered once a side has more than one event ("DCA Entry #2"). */
function eventLabel(event, sideCount) {
  const base = KIND_LABEL[event.kind] || "Swap";
  return sideCount > 1 ? `${base} #${event.sequence}` : base;
}

function statChip(label, value, extraClass = "") {
  if (value === null || value === undefined || value === "") return "";
  return `
    <span class="pdd-act-chip ${extraClass}">
      <span class="pdd-act-chip-label">${label}</span>
      <span class="pdd-act-chip-value">${value}</span>
    </span>`;
}

function detailCell(label, value) {
  if (value === null || value === undefined || value === "") return "";
  return `
    <div class="pdd-act-detail">
      <span class="pdd-act-detail-label">${label}</span>
      <span class="pdd-act-detail-value">${value}</span>
    </div>`;
}

/** The token / SOL / price / P&L row — the numbers a trader reads first. */
function renderFlow(event, ctx) {
  const isExit = event.side === "exit";
  const cells = [];

  const amount = event.token_amount;
  if (amount != null) {
    const ui = Utils.formatCompactNumber(ctx.toUi(amount));
    // A swap still confirming has NOT been booked: the registry knows what was submitted,
    // the position does not. Say so rather than printing it as a settled amount.
    const label = event.recorded ? "Tokens" : "Tokens (expected)";
    cells.push(`
      <div class="pdd-act-flow-cell">
        <span class="pdd-act-flow-label">${label}</span>
        <span class="pdd-act-flow-value ${isExit ? "negative" : "positive"}">
          ${isExit ? "-" : "+"}${ui} ${Utils.escapeHtml(ctx.symbol)}
        </span>
      </div>`);
  }

  if (event.sol_amount != null) {
    cells.push(`
      <div class="pdd-act-flow-cell">
        <span class="pdd-act-flow-label">${isExit ? "SOL Received" : "SOL Spent"}</span>
        <span class="pdd-act-flow-value ${isExit ? "positive" : "negative"}">
          ${isExit ? "+" : "-"}${Utils.formatSol(event.sol_amount, { decimals: 4, suffix: "" })} SOL
        </span>
      </div>`);
  }

  if (event.price != null) {
    cells.push(`
      <div class="pdd-act-flow-cell">
        <span class="pdd-act-flow-label">${isExit ? "Exit Price" : "Entry Price"}</span>
        <span class="pdd-act-flow-value">${ctx.formatPrice(event.price)} SOL</span>
      </div>`);
  }

  if (isExit && event.realized_pnl != null) {
    const pnl = event.realized_pnl;
    const cls = pnl >= 0 ? "positive" : "negative";
    const sign = pnl >= 0 ? "+" : "";
    const pct =
      event.realized_pnl_percent != null
        ? `<small>${sign}${Utils.formatNumber(event.realized_pnl_percent, 2)}%</small>`
        : "";
    cells.push(`
      <div class="pdd-act-flow-cell">
        <span class="pdd-act-flow-label">Realized P&amp;L</span>
        <span class="pdd-act-flow-value ${cls}">
          ${sign}${Utils.formatSol(pnl, { decimals: 4, suffix: "" })} SOL ${pct}
        </span>
      </div>`);
  }

  return cells.length ? `<div class="pdd-act-flow">${cells.join("")}</div>` : "";
}

/** Secondary chips: what it cost and where it routed. */
function renderChips(event, ctx) {
  const chips = [];

  const fee = event.fee_sol ?? event.record_fee_sol;
  if (fee != null && fee > 0) {
    chips.push(statChip("Fee", `${Utils.formatSol(fee, { decimals: 6, suffix: "" })} SOL`));
  }
  if (event.side === "exit" && event.cost_basis != null) {
    chips.push(
      statChip(
        "Cost Basis",
        `${Utils.formatSol(event.cost_basis, { decimals: 4, suffix: "" })} SOL`
      )
    );
  }
  if (event.sol_amount != null && ctx.solPriceUsd) {
    chips.push(statChip("Value", Utils.formatCurrencyUSD(event.sol_amount * ctx.solPriceUsd)));
  }
  if (event.router) {
    chips.push(statChip("Router", Utils.escapeHtml(event.router), "pdd-act-chip-router"));
  }
  if (event.slot != null) {
    chips.push(statChip("Slot", Utils.formatNumber(event.slot, 0)));
  }

  return chips.length ? `<div class="pdd-act-chips">${chips.join("")}</div>` : "";
}

/**
 * The position AFTER this swap settled — held tokens, cost basis still on the books, and
 * the average entry price those two imply. Only a BOOKED swap moves these, which is why a
 * pending event has none.
 */
function renderAfter(event, ctx) {
  if (event.tokens_after == null || event.invested_after == null) return "";

  const heldUi = ctx.toUi(event.tokens_after);
  const avgEntry = heldUi > 0 ? event.invested_after / heldUi : null;

  return `
    <div class="pdd-act-after">
      <span class="pdd-act-after-title">After</span>
      <span class="pdd-act-after-item">
        <span class="pdd-act-after-label">Held</span>
        <span class="pdd-act-after-value">${Utils.formatCompactNumber(heldUi)} ${Utils.escapeHtml(ctx.symbol)}</span>
      </span>
      <span class="pdd-act-after-item">
        <span class="pdd-act-after-label">Invested</span>
        <span class="pdd-act-after-value">${Utils.formatSol(event.invested_after, { decimals: 4, suffix: "" })} SOL</span>
      </span>
      ${
        avgEntry
          ? `<span class="pdd-act-after-item">
               <span class="pdd-act-after-label">Avg Entry</span>
               <span class="pdd-act-after-value">${ctx.formatPrice(avgEntry)} SOL</span>
             </span>`
          : ""
      }
    </div>`;
}

/** Token transfers exactly as they moved on chain. */
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
    <div class="pdd-act-xfers">
      <span class="pdd-act-detail-title">Token Transfers</span>
      <table class="pdd-act-xfer-table">
        <thead>
          <tr><th>Amount</th><th>Mint</th><th>From</th><th>To</th></tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    </div>`;
}

/** The full on-chain record, revealed on demand. */
function renderDetails(event) {
  const cells = [
    detailCell("Status", event.status ? Utils.escapeHtml(event.status) : null),
    detailCell(
      "Transaction Type",
      event.transaction_type ? Utils.escapeHtml(event.transaction_type) : null
    ),
    detailCell("Direction", event.direction ? Utils.escapeHtml(event.direction) : null),
    detailCell("Slot", event.slot != null ? Utils.formatNumber(event.slot, 0) : null),
    detailCell("Block Time", event.block_time ? Utils.formatTimestamp(event.block_time) : null),
    detailCell(
      "Wallet SOL Change",
      event.sol_change != null
        ? `${event.sol_change >= 0 ? "+" : ""}${Utils.formatSol(event.sol_change, { decimals: 6, suffix: "" })} SOL`
        : null
    ),
    detailCell(
      "Network Fee",
      event.fee_sol != null
        ? `${Utils.formatSol(event.fee_sol, { decimals: 6, suffix: "" })} SOL`
        : null
    ),
    detailCell("Instructions", event.instructions_count ?? null),
    detailCell(
      "Compute Units",
      event.compute_units != null ? Utils.formatNumber(event.compute_units, 0) : null
    ),
    detailCell("Accounts", event.accounts_count ?? null),
    detailCell("Record ID", event.record_id ?? null),
  ]
    .filter(Boolean)
    .join("");

  const note = event.notes
    ? `<div class="pdd-act-note ${event.state === "failed" ? "is-error" : ""}">
         <i class="icon-info"></i>${Utils.escapeHtml(event.notes)}
       </div>`
    : "";

  const grid = cells ? `<div class="pdd-act-detail-grid">${cells}</div>` : "";

  return `<div class="pdd-act-details">${note}${grid}${renderTransfers(event)}</div>`;
}

/**
 * Render one activity card.
 * @param {Object} event - merged activity event from the API
 * @param {Object} ctx - { symbol, solPriceUsd, expanded:Set, toUi(fn), formatPrice(fn), sideCounts }
 */
export function renderActivityCard(event, ctx) {
  const key = activityEventKey(event);
  const isExit = event.side === "exit";
  const sideCount = isExit ? ctx.sideCounts.exit : ctx.sideCounts.entry;
  const stateMeta = STATE_META[event.state] || STATE_META.pending;
  const expanded = ctx.expanded.has(key);

  const pctTag =
    isExit && event.exit_percentage != null
      ? `<span class="pdd-act-tag">${Utils.formatNumber(event.exit_percentage, 0)}%</span>`
      : "";

  const signature = event.signature || "";
  const sigGroup = signature
    ? `<span class="pdd-act-sig" data-copy="${signature}" title="Click to copy">${Utils.formatSignatureCompact(signature, { start: 8, end: 8 })}</span>
       <button type="button" class="pdd-act-sig-copy" data-copy="${signature}" title="Copy signature"><i class="icon-copy"></i></button>
       <a href="${Utils.solscanTxUrl(signature)}" target="_blank" rel="noopener" class="pdd-act-sig-link" title="View on Solscan"><i class="icon-external-link"></i><span>Solscan</span></a>`
    : '<span class="pdd-act-sig-na">No signature</span>';

  return `
    <article class="pdd-act-card${expanded ? " is-open" : ""}" data-side="${event.side}" data-state="${event.state}" data-key="${Utils.escapeHtml(key)}">
      <div class="pdd-act-head">
        <span class="pdd-act-icon">
          <i class="${isExit ? "icon-circle-arrow-up" : "icon-circle-arrow-down"}"></i>
        </span>
        <span class="pdd-act-label">${eventLabel(event, sideCount)}</span>
        ${pctTag}
        <span class="pdd-act-head-right">
          <span class="pdd-act-time" title="${Utils.formatTimestamp(event.timestamp)}">${Utils.formatTimeAgo(event.timestamp)}</span>
          <span class="pdd-act-state pdd-act-state-${event.state}">
            <i class="${stateMeta.icon}"></i>${stateMeta.label}
          </span>
        </span>
      </div>
      ${renderFlow(event, ctx)}
      ${renderChips(event, ctx)}
      ${renderAfter(event, ctx)}
      <div class="pdd-act-foot">
        <span class="pdd-act-sig-group">${sigGroup}</span>
        <button type="button" class="pdd-act-expand" data-expand="${Utils.escapeHtml(key)}" aria-expanded="${expanded}">
          <span>${expanded ? "Hide" : "Details"}</span><i class="icon-chevron-down"></i>
        </button>
      </div>
      ${renderDetails(event)}
    </article>`;
}
