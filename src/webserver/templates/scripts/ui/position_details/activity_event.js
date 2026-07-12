/**
 * Activity event card — one event in a token's all-time history, rendered from the merged
 * event the backend serves (`/api/positions/{key}/activity`).
 *
 * A POSITION SWAP already carries BOTH halves: the position record (booked amount, price,
 * SOL, exit percentage) and the on-chain transaction (status, fee, router, slot, transfers),
 * plus the running state of its position after it settled. Nothing is joined here.
 *
 * A WALLET EVENT is a transaction that touched the mint without belonging to any position —
 * a transfer, an airdrop, a swap made in another app. It has no price, no cost basis and no
 * position, and it must never look like one.
 *
 * Token amounts arrive as whole tokens (the server scaled them by the mint's decimals).
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
  buy: "Wallet Buy",
  sell: "Wallet Sell",
  transfer: "Transfer",
  ata: "Token Account",
  other: "Transaction",
};

const KIND_ICON = {
  entry: "icon-circle-arrow-down",
  dca: "icon-circle-arrow-down",
  partial_exit: "icon-circle-arrow-up",
  exit: "icon-circle-arrow-up",
  buy: "icon-circle-arrow-down",
  sell: "icon-circle-arrow-up",
  transfer: "icon-arrow-right-left",
  ata: "icon-wallet",
  other: "icon-activity",
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
  const base = KIND_LABEL[event.kind] || "Transaction";
  return sideCount > 1 ? `${base} #${event.sequence}` : base;
}

/**
 * Which round of trading this swap belongs to. Only shown once the token has been traded
 * more than once — with a single position it would be noise on every card.
 */
function positionChip(event, ctx) {
  if (event.side === "wallet") {
    return '<span class="pdd-act-chip-pos is-wallet">Wallet</span>';
  }
  if (ctx.positionCount < 2 || event.position_index == null) return "";
  const isCurrent = event.position_id === ctx.currentPositionId;
  return `<span class="pdd-act-chip-pos${isCurrent ? " is-current" : ""}">Position ${event.position_index}</span>`;
}

/**
 * THE label/value cell — the only one on this card, deliberately.
 *
 * The card used to present the same label→value pair four different ways: big stacked
 * "flow" cells, an inline chip row, the inline "after" strip, and a third, smaller stacked
 * grid inside the expanded panel. Four typographic systems on one card is what made it read
 * as unstyled. Everything now goes through this cell; `key` only raises the emphasis of the
 * numbers a trader reads first (tokens, SOL, price, realized P&L).
 */
function metric(label, value, { key = false, tone = "" } = {}) {
  if (value === null || value === undefined || value === "") return "";
  return `
    <div class="pdd-act-metric${key ? " is-key" : ""}">
      <span class="pdd-act-metric-label">${label}</span>
      <span class="pdd-act-metric-value ${tone}">${value}</span>
    </div>`;
}

const sol = (value, decimals = 4) => `${Utils.formatSol(value, { decimals, suffix: "" })} SOL`;

/**
 * Everything the swap did, in ONE grid: the headline numbers first, then what it cost and
 * where it routed. Folding the old chip row in here is what lets the grid fill the dialog's
 * width — three stretched cells used to leave craters between them — and makes every card
 * line up column-for-column with the one above it.
 */
function renderMetrics(event, ctx) {
  const isExit = event.side === "exit";
  const isWallet = event.side === "wallet";
  const cells = [];

  const amount = event.token_amount;
  if (amount != null && amount !== 0) {
    // A wallet event's direction is whatever the chain did, not a position side.
    const outgoing = isExit || (isWallet && event.kind === "sell");
    // A swap still confirming has NOT been booked: the registry knows what was submitted,
    // the position does not. Say so rather than printing it as a settled amount.
    const label = !isWallet && !event.recorded ? "Tokens (expected)" : "Tokens";
    cells.push(
      metric(
        label,
        `${outgoing ? "-" : "+"}${Utils.formatCompactNumber(amount)} ${Utils.escapeHtml(ctx.symbol)}`,
        { key: true, tone: outgoing ? "negative" : "positive" }
      )
    );
  }

  if (event.sol_amount != null) {
    cells.push(
      metric(
        isExit ? "SOL Received" : "SOL Spent",
        `${isExit ? "+" : "-"}${sol(event.sol_amount)}`,
        { key: true, tone: isExit ? "positive" : "negative" }
      )
    );
  }

  // A wallet event has no position record, so its SOL is the wallet's net delta (fee
  // included) rather than a booked entry/exit amount — a different number, labelled as one.
  if (isWallet && event.sol_change != null && event.sol_change !== 0) {
    cells.push(
      metric("Wallet SOL", `${event.sol_change >= 0 ? "+" : ""}${sol(event.sol_change, 6)}`, {
        key: true,
        tone: event.sol_change >= 0 ? "positive" : "negative",
      })
    );
  }

  if (event.price != null) {
    cells.push(
      metric(isExit ? "Exit Price" : "Entry Price", `${ctx.formatPrice(event.price)} SOL`, {
        key: true,
      })
    );
  }

  if (isExit && event.realized_pnl != null) {
    const pnl = event.realized_pnl;
    const sign = pnl >= 0 ? "+" : "";
    const pct =
      event.realized_pnl_percent != null
        ? `<small>${sign}${Utils.formatNumber(event.realized_pnl_percent, 2)}%</small>`
        : "";
    cells.push(
      metric("Realized P&L", `${sign}${sol(pnl)} ${pct}`, {
        key: true,
        tone: pnl >= 0 ? "positive" : "negative",
      })
    );
  }

  if (isExit && event.cost_basis != null) {
    cells.push(metric("Cost Basis", sol(event.cost_basis)));
  }

  if (event.sol_amount != null && ctx.solPriceUsd) {
    cells.push(metric("Value", Utils.formatCurrencyUSD(event.sol_amount * ctx.solPriceUsd)));
  }

  const fee = event.fee_sol ?? event.record_fee_sol;
  if (fee != null && fee > 0) {
    cells.push(metric("Network Fee", sol(fee, 6)));
  }

  if (event.router) {
    cells.push(metric("Router", Utils.escapeHtml(event.router)));
  }

  if (event.slot != null) {
    cells.push(metric("Slot", Utils.formatNumber(event.slot, 0)));
  }

  const rendered = cells.filter(Boolean).join("");
  return rendered ? `<div class="pdd-act-metrics">${rendered}</div>` : "";
}

/**
 * The position AFTER this swap settled — held tokens, cost basis still on the books, and
 * the average entry price those two imply. Only a BOOKED position swap moves these, so a
 * pending swap and a wallet event have none.
 */
function renderAfter(event, ctx) {
  if (event.tokens_after == null || event.invested_after == null) return "";

  const held = event.tokens_after;
  const avgEntry = held > 0 ? event.invested_after / held : null;

  return `
    <div class="pdd-act-after">
      <span class="pdd-act-after-title">Position after</span>
      <span class="pdd-act-after-item">
        <span class="pdd-act-after-label">Held</span>
        <span class="pdd-act-after-value">${Utils.formatCompactNumber(held)} ${Utils.escapeHtml(ctx.symbol)}</span>
      </span>
      <span class="pdd-act-after-item">
        <span class="pdd-act-after-label">Invested</span>
        <span class="pdd-act-after-value">${sol(event.invested_after)}</span>
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
      <span class="pdd-act-subhead">Token Transfers</span>
      <table class="pdd-act-xfer-table">
        <thead>
          <tr><th>Amount</th><th>Mint</th><th>From</th><th>To</th></tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    </div>`;
}

/** The full on-chain record, revealed on demand — same metric cell as the card above it. */
function renderDetails(event) {
  const cells = [
    metric("Status", event.status ? Utils.escapeHtml(event.status) : null),
    metric(
      "Transaction Type",
      event.transaction_type ? Utils.escapeHtml(event.transaction_type) : null
    ),
    metric("Direction", event.direction ? Utils.escapeHtml(event.direction) : null),
    metric("Block Time", event.block_time ? Utils.formatTimestamp(event.block_time) : null),
    metric(
      "Wallet SOL Change",
      event.sol_change != null
        ? `${event.sol_change >= 0 ? "+" : ""}${sol(event.sol_change, 6)}`
        : null
    ),
    metric("Instructions", event.instructions_count ?? null),
    metric(
      "Compute Units",
      event.compute_units != null ? Utils.formatNumber(event.compute_units, 0) : null
    ),
    metric("Accounts", event.accounts_count ?? null),
    metric("Record ID", event.record_id ?? null),
  ]
    .filter(Boolean)
    .join("");

  const note = event.notes
    ? `<div class="pdd-act-note ${event.state === "failed" ? "is-error" : ""}">
         <i class="icon-info"></i>${Utils.escapeHtml(event.notes)}
       </div>`
    : "";

  const grid = cells ? `<div class="pdd-act-metrics is-compact">${cells}</div>` : "";

  return `<div class="pdd-act-details">${note}${grid}${renderTransfers(event)}</div>`;
}

/**
 * Render one activity card.
 * @param {Object} event - merged activity event from the API
 * @param {Object} ctx - { symbol, solPriceUsd, expanded:Set, formatPrice(fn), sideCounts,
 *                         positionCount, currentPositionId }
 */
export function renderActivityCard(event, ctx) {
  const key = activityEventKey(event);
  const isExit = event.side === "exit";
  const sideCount = ctx.sideCounts[event.side] ?? 0;
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
          <i class="${KIND_ICON[event.kind] || "icon-activity"}"></i>
        </span>
        <span class="pdd-act-label">${eventLabel(event, sideCount)}</span>
        ${pctTag}
        ${positionChip(event, ctx)}
        <span class="pdd-act-head-right">
          <span class="pdd-act-time" title="${Utils.formatTimestamp(event.timestamp)}">${Utils.formatTimeAgo(event.timestamp)}</span>
          <span class="pdd-act-state pdd-act-state-${event.state}">
            <i class="${stateMeta.icon}"></i>${stateMeta.label}
          </span>
        </span>
      </div>
      ${renderMetrics(event, ctx)}
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
