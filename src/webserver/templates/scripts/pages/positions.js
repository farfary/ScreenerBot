// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Positions page - table, tabs and dialogs for open and closed positions.

import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { apiErrorMessage, requestManager } from "../core/request_manager.js";
import * as Utils from "../core/utils.js";
import { formatFixed, withPercentUnit } from "../core/format.js";
import * as AppState from "../core/app_state.js";
import { DataTable } from "../ui/data_table.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { stepShortLabel } from "../ui/action_step.js";
import { POSITION_EMPTY_LABELS, POSITION_STATUS_LABELS } from "../ui/position_status.js";
import { manualTrade } from "../ui/manual_trade.js";
import { PositionDetailsDialog } from "../ui/position_details_dialog.js";
import { PositionRemoveDialog } from "../ui/position_remove_dialog.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { openCopyTask } from "../ui/copy_handoff.js";
import { notificationManager } from "../core/notifications.js";
import { renderTokenRowCell } from "../ui/token_identity.js";

// Origin kinds that get a chip in the token cell. `external` is derived from this
// wallet's own on-chain history rather than traded by the bot.
const ORIGIN_LABELS = Object.freeze({
  copy: "positions-origin-copy",
  manual: "positions-origin-manual",
  external: "positions-origin-wallet",
});
const ORIGIN_CLASSES = Object.freeze({ copy: "copy", manual: "manual", external: "wallet" });

const esc = (text) => Utils.escapeHtml(text);

// Sub-tab labels: an icon followed by the view name.
const subTabs = () => [
  {
    id: "open",
    label: `<i class="icon-trending-up"></i> ${esc(I18n.label(POSITION_STATUS_LABELS, "open"))}`,
  },
  {
    id: "closed",
    label: `<i class="icon-trending-down"></i> ${esc(I18n.label(POSITION_STATUS_LABELS, "closed"))}`,
  },
  {
    id: "archived",
    label: `<i class="icon-archive"></i> ${esc(I18n.label(POSITION_STATUS_LABELS, "archived"))}`,
  },
];

// Title and aria-label attributes of an icon-only row action.
const actionAttrs = (title, ariaLabel) => `title="${esc(title)}" aria-label="${esc(ariaLabel)}"`;
const addAttrs = () =>
  actionAttrs(
    I18n.attr("positions-action-add", "title"),
    I18n.attr("positions-action-add", "aria-label")
  );
const removeAttrs = () =>
  actionAttrs(
    I18n.attr("positions-action-remove", "title"),
    I18n.attr("positions-action-remove", "aria-label")
  );
const restoreAttrs = () =>
  actionAttrs(
    I18n.attr("positions-action-restore", "title"),
    I18n.attr("positions-action-restore", "aria-label")
  );
const deleteAttrs = () =>
  actionAttrs(
    I18n.attr("positions-action-delete", "title"),
    I18n.attr("positions-action-delete", "aria-label")
  );

// Live-action wiring: the actions system streams every in-flight buy/sell as an
// Action (SSE -> notificationManager) long before the on-chain position is
// created/closed. We surface those as transient "pending" rows / state badges in
// the Open list so a freshly clicked buy is visible immediately, and a selling
// position keeps showing until it is fully closed.
const ACTION_BUY_TYPES = new Set(["swap_buy", "position_open"]);
const ACTION_SELL_TYPES = new Set(["swap_sell", "position_close", "position_partial_exit"]);
// A DCA ("add to position") acts on a position that ALREADY exists, so it is neither a
// pending-buy row nor a sell. It had no state at all here, which left the add invisible:
// the row's Total Invested and Holdings only move once the DCA is VERIFIED on chain
// (`total_size_native += sol_spent`, `remaining_token_amount += tokens_bought`), several
// seconds later — so the table just sat there showing the pre-DCA numbers with no hint
// that anything was happening.
const ACTION_DCA_TYPES = new Set(["position_dca"]);

// Keep showing a just-finished buy as pending for a short grace window so the row
// doesn't flicker out between "swap done" and the position appearing on next poll.
const BUY_GRACE_MS = 10000;
// How long a failed buy lingers as a red row before it disappears.
const FAILED_LINGER_MS = 8000;
// A closed position keeps its arrival highlight for this long after exit.
const JUST_CLOSED_MS = 12000;

// Short label for the state caption of a backend step code.
const shortStep = (step) => stepShortLabel(step || "unknown");

const actionMint = (n) => n?.entity_id || n?.metadata?.mint || "";
const parseTs = (v) => {
  const t = v ? Date.parse(v) : NaN;
  return Number.isFinite(t) ? t : NaN;
};

const getPositionsTableStateKey = (view) => `positions-table.${view}`;
const normalizeSortDirection = (direction) => (direction === "desc" ? "desc" : "asc");

const loadPersistedSort = (stateKey) => {
  if (!stateKey) return null;
  const saved = AppState.load(stateKey);
  if (saved && typeof saved === "object" && saved.sortColumn) {
    return {
      column: saved.sortColumn,
      direction: normalizeSortDirection(saved.sortDirection),
    };
  }
  return null;
};

const getInitialSortForView = (view) => {
  const fallbackColumn =
    view === "archived" ? "archived_at" : view === "closed" ? "exit_time" : "entry_time";
  const persisted = loadPersistedSort(getPositionsTableStateKey(view));
  if (persisted?.column) {
    return { column: persisted.column, direction: persisted.direction || "desc" };
  }
  return { column: fallbackColumn, direction: "desc" };
};

function createLifecycle() {
  let table = null;
  let poller = null;
  let tabBar = null;
  let positionDetailsDialog = null;
  let liveUnsub = null;
  let liveDebounce = null;

  const state = {
    view: "open", // 'open' | 'closed'
    total: 0,
    // Most recent server-side rows (already token-mapped) for the active view.
    // Live action updates re-merge against this without a network round-trip.
    lastServerRows: [],
    sort: getInitialSortForView("open"),
  };

  // Compact caption describing a row's live state (buying / selling / failed).
  // Open rows stay clean; an in-flight or failed row adds a tint beside this caption.
  const stateCaption = (row) => {
    const st = row?._state;
    if (!st || st === "open") return "";
    const step = row?._stepLabel ? String(row._stepLabel) : "";
    let label;
    let icon;
    if (st === "buying") {
      label = step
        ? I18n.t("positions-caption-buying-step", { step })
        : I18n.t("positions-caption-buying");
      icon = '<span class="pos-state-spinner" aria-hidden="true"></span>';
    } else if (st === "selling") {
      label = step
        ? I18n.t("positions-caption-selling-step", { step })
        : I18n.t("positions-caption-selling");
      icon = '<span class="pos-state-spinner" aria-hidden="true"></span>';
    } else if (st === "closing") {
      label = I18n.t("positions-caption-closing");
      icon = '<span class="pos-state-spinner" aria-hidden="true"></span>';
    } else if (st === "failed") {
      label = row?._error
        ? I18n.t("positions-caption-failed-detail", { error: String(row._error) })
        : I18n.t("positions-caption-failed");
      icon = '<i class="icon-triangle-alert" aria-hidden="true"></i>';
    } else {
      return "";
    }
    return `<div class="pos-state-caption pos-state-${Utils.escapeHtml(st)}" title="${Utils.escapeHtml(
      label
    )}">${icon}<span class="pos-state-text">${Utils.escapeHtml(label)}</span></div>`;
  };

  const positionOriginChip = (row) => {
    const kind = row?.origin?.kind;
    return Object.hasOwn(ORIGIN_LABELS, kind)
      ? { text: I18n.label(ORIGIN_LABELS, kind), cls: ORIGIN_CLASSES[kind] }
      : null;
  };

  // A wallet-derived round can be missing its cost basis (an airdrop, a USD-quoted
  // fill, a token->token swap with no SOL leg) or fail to reconcile with the chain.
  // Those rows show "—" instead of a number: a fabricated basis produces a plausible
  // and permanently wrong P&L, which is worse than admitting we do not know.
  const basisUnknownTitle = () => I18n.t("positions-unknown-basis");
  const historyUnknownTitle = () => I18n.t("positions-unknown-history");

  const basisUnknown = (row) => row?.basis_complete === false;
  const pnlUnknown = (row) => row?.basis_complete === false || row?.history_complete === false;
  const unknownCell = (title) =>
    `<span class="position-unknown" title="${Utils.escapeHtml(title)}">—</span>`;
  const basisCell = (row, render) =>
    basisUnknown(row) ? unknownCell(basisUnknownTitle()) : render();
  const pnlGuardedCell = (row, render) =>
    pnlUnknown(row)
      ? unknownCell(basisUnknown(row) ? basisUnknownTitle() : historyUnknownTitle())
      : render();

  // Origin and holding tags ride after the token name.
  const positionTags = (row) => {
    const originChip = positionOriginChip(row);
    const origin = originChip
      ? row?.origin?.kind === "copy" && row.origin.task_id != null
        ? `<button type="button" class="position-origin-label position-origin-copy position-origin-link" data-copy-task="${Utils.escapeHtml(String(row.origin.task_id))}" title="${esc(I18n.attr("positions-origin-copy-link", "title"))}">${esc(originChip.text)}</button>`
        : `<span class="position-origin-label position-origin-${originChip.cls}">${esc(originChip.text)}</span>`
      : "";
    const frozen =
      row?.holding_state === "frozen"
        ? `<span class="position-origin-label position-frozen" title="${esc(I18n.attr("positions-holding-frozen", "title"))}">${esc(I18n.t("positions-holding-frozen"))}</span>`
        : "";
    return origin + frozen;
  };

  const tokenCell = (row, actions = "", actionCount = 0) =>
    renderTokenRowCell(row.mint, {
      symbol: row.symbol,
      name: row.name,
      logoUrl: row.logo_url || row.image_url,
      tags: positionTags(row),
      caption: stateCaption(row),
      actions,
      actionCount,
    });

  // Significant digits in the cell; the exact price on hover.
  const priceCell = (value) =>
    value == null
      ? "—"
      : `<span title="${esc(formatFixed(value, { decimals: 12, trim: true }))}">${esc(
          Utils.formatPriceSubscript(value, { fallback: "—" })
        )}</span>`;
  const solCell = (v) => Utils.formatSol(v, { decimals: 4 });
  const pnlCell = (v) => Utils.formatPnL(v, { decimals: 4 });
  const percentCell = (v) => Utils.formatPercent(v, { style: "pnl", decimals: 2, fallback: "—" });
  const timeCell = (v) => Utils.formatTimeFromSeconds(v, { includeSeconds: false });

  const dcaCell = (count) => {
    if (!count || count === 0) return "—";
    return `<span class="chip">${esc(I18n.t("positions-dca-count", { count }))}</span>`;
  };

  const partialExitsCell = (count) => {
    if (!count || count === 0) return "—";
    return `<span class="chip warning">${esc(I18n.t("positions-exit-count", { count }))}</span>`;
  };

  // How much of the position is still held.
  //
  // The denominator is NOT `token_amount`: that field is the tokens bought at ENTRY and
  // never grows on a DCA, while `remaining_token_amount` does — so a DCA'd position
  // reported >100% held. Tokens ever acquired = what is left + what has been exited,
  // which is correct for DCA and partial exits alike.
  const currentSizeCell = (remaining, totalExited) => {
    if (remaining == null) return "—";
    const acquired = remaining + (totalExited || 0);
    if (acquired <= 0) return "—";
    const pct = Math.round((remaining / acquired) * 100);
    const cls = pct === 100 ? "success" : pct >= 50 ? "warning" : "danger";
    return `<span class="chip ${cls}">${withPercentUnit(pct)}</span>`;
  };

  // Compact "Remove" (archive/delete) button for closed rows.
  const removeActionCell = (row) => {
    const id = row?.id;
    if (id == null) return "";
    return `<button class="btn row-action" data-action="remove" data-id="${Utils.escapeHtml(
      String(id)
    )}" data-mint="${Utils.escapeHtml(row?.mint || "")}" ${removeAttrs()}><i class="icon-trash-2"></i></button>`;
  };

  // Restore + permanent-delete buttons for archived rows.
  const archivedActionCell = (row) => {
    const id = row?.id;
    if (id == null) return "";
    const idAttr = Utils.escapeHtml(String(id));
    const mintAttr = Utils.escapeHtml(row?.mint || "");
    return `<button class="btn row-action" data-action="restore" data-id="${idAttr}" data-mint="${mintAttr}" ${restoreAttrs()}><i class="icon-rotate-ccw"></i></button>
      <button class="btn row-action row-action--danger" data-action="delete" data-id="${idAttr}" data-mint="${mintAttr}" ${deleteAttrs()}><i class="icon-trash-2"></i></button>`;
  };

  const openActionCell = (row) => {
    const mint = row?.mint || "";
    const isOpen = !row?.transaction_exit_verified;
    if (!mint || !isOpen) return "";

    if (row?._pending) {
      return `<span class="row-actions-busy">${esc(I18n.t("positions-action-in-progress"))}</span>`;
    }

    const busy = row?._state === "selling" || row?._state === "closing";
    const dis = busy ? " disabled" : "";
    // A frozen token account cannot be transferred, so every sell would fail on-chain.
    // The button is disabled and says why; the row itself stays exactly where it is —
    // archiving an unsellable holding is the user's call, never automatic.
    const frozen = row?.holding_state === "frozen";
    const sellDis = busy || frozen ? " disabled" : "";
    const sellTitle = frozen
      ? I18n.t("positions-action-sell-frozen")
      : I18n.attr("positions-action-sell", "title");
    const mintAttr = Utils.escapeHtml(mint);
    const idAttr = Utils.escapeHtml(String(row?.id ?? ""));
    return `
      <button class="btn row-action" data-action="add" data-mint="${mintAttr}" ${addAttrs()}${dis}><i class="icon-circle-plus"></i></button>
      <button class="btn row-action" data-action="sell" data-mint="${mintAttr}" title="${Utils.escapeHtml(
        sellTitle
      )}" aria-label="${esc(I18n.attr("positions-action-sell", "aria-label"))}"${sellDis}><i class="icon-trending-down"></i></button>
      <button class="btn row-action" data-action="remove" data-id="${idAttr}" data-mint="${mintAttr}" ${removeAttrs()}><i class="icon-trash-2"></i></button>
    `;
  };

  const buildArchivedColumns = () => [
    {
      id: "token",
      label: I18n.t("positions-column-token"),
      sortable: true,
      floating: true,
      minWidth: 260,
      wrap: false,
      render: (_v, r) => tokenCell(r, archivedActionCell(r), 2),
    },
    {
      id: "archived_at",
      label: I18n.t("positions-column-archived-at"),
      sortable: true,
      minWidth: 140,
      render: (v, r) => timeCell(v ?? r.exit_time ?? r.entry_time),
    },
    {
      id: "entry_time",
      label: I18n.t("positions-column-entry-time"),
      sortable: true,
      minWidth: 140,
      render: (v) => timeCell(v),
    },
    {
      id: "average_entry_price",
      label: I18n.t("positions-column-avg-entry"),
      type: "price",
      sortable: true,
      minWidth: 140,
      render: (v, r) => basisCell(r, () => priceCell(v || r.entry_price)),
    },
    {
      id: "total_size_native",
      label: I18n.t("positions-column-total-invested"),
      type: "sol",
      sortable: true,
      minWidth: 120,
      render: (v, r) => basisCell(r, () => solCell(v)),
    },
    {
      id: "sol_received",
      label: I18n.t("positions-column-proceeds"),
      type: "sol",
      sortable: true,
      minWidth: 110,
      render: (v) => (v == null ? "—" : solCell(v)),
    },
    {
      id: "pnl",
      label: I18n.t("positions-column-pnl"),
      type: "sol",
      sortable: true,
      minWidth: 110,
      render: (v, r) => pnlGuardedCell(r, () => pnlCell(v)),
    },
    {
      id: "pnl_percent",
      label: I18n.t("positions-column-pnl-percent"),
      type: "percent",
      sortable: true,
      minWidth: 100,
      render: (v, r) => pnlGuardedCell(r, () => percentCell(v)),
    },
  ];

  /**
   * Build columns array based on current view (open/closed/archived)
   * Different views show different columns (open has unrealized PnL, closed has exit data)
   */
  const buildColumns = () => {
    if (state.view === "open") {
      return [
        {
          id: "token",
          label: I18n.t("positions-column-token"),
          sortable: true,
          floating: true,
          minWidth: 260,
          wrap: false,
          render: (_v, r) => tokenCell(r, openActionCell(r), 3),
        },
        {
          id: "entry_time",
          label: I18n.t("positions-column-entry-time"),
          sortable: true,
          minWidth: 140,
          render: (v) => timeCell(v),
        },
        {
          id: "dca_count",
          label: I18n.t("positions-column-dca"),
          type: "number",
          sortable: true,
          minWidth: 80,
          render: (v) => dcaCell(v),
        },
        {
          id: "average_entry_price",
          label: I18n.t("positions-column-avg-entry"),
          type: "price",
          sortable: true,
          minWidth: 140,
          render: (v, r) => basisCell(r, () => priceCell(v)),
        },
        {
          id: "current_price",
          label: I18n.t("positions-column-current-price"),
          type: "price",
          sortable: true,
          minWidth: 140,
          render: (v) => (v == null ? "—" : priceCell(v)),
        },
        {
          id: "total_size_native",
          label: I18n.t("positions-column-total-invested"),
          type: "sol",
          sortable: true,
          minWidth: 120,
          render: (v, r) => basisCell(r, () => solCell(v)),
        },
        {
          id: "current_size",
          label: I18n.t("positions-column-size"),
          type: "percent",
          sortable: true,
          minWidth: 80,
          render: (_v, r) => currentSizeCell(r.remaining_token_amount, r.total_exited_amount),
        },
        {
          id: "partial_exit_count",
          label: I18n.t("positions-column-exits"),
          type: "number",
          sortable: true,
          minWidth: 90,
          render: (v) => partialExitsCell(v),
        },
        {
          id: "unrealized_pnl",
          label: I18n.t("positions-column-unrealized-pnl"),
          type: "sol",
          sortable: true,
          minWidth: 130,
          render: (v, r) => pnlGuardedCell(r, () => pnlCell(v)),
        },
        {
          id: "unrealized_pnl_percent",
          label: I18n.t("positions-column-unrealized-percent"),
          type: "percent",
          sortable: true,
          minWidth: 110,
          render: (v, r) => pnlGuardedCell(r, () => percentCell(v)),
        },
      ];
    } else if (state.view === "archived") {
      return buildArchivedColumns();
    } else {
      // closed view
      return [
        {
          id: "token",
          label: I18n.t("positions-column-token"),
          sortable: true,
          floating: true,
          minWidth: 260,
          wrap: false,
          render: (_v, r) => tokenCell(r, removeActionCell(r), 1),
        },
        {
          id: "exit_time",
          label: I18n.t("positions-column-exit-time"),
          sortable: true,
          minWidth: 140,
          render: (v) => (v == null ? "—" : timeCell(v)),
        },
        {
          id: "dca_count",
          label: I18n.t("positions-column-dca"),
          type: "number",
          sortable: true,
          minWidth: 80,
          render: (v) => dcaCell(v),
        },
        {
          id: "average_entry_price",
          label: I18n.t("positions-column-avg-entry"),
          type: "price",
          sortable: true,
          minWidth: 140,
          render: (v, r) => basisCell(r, () => priceCell(v || r.entry_price)),
        },
        {
          id: "average_exit_price",
          label: I18n.t("positions-column-avg-exit"),
          type: "price",
          sortable: true,
          minWidth: 140,
          render: (v, r) => (v == null ? priceCell(r.exit_price) : priceCell(v)),
        },
        {
          id: "total_size_native",
          label: I18n.t("positions-column-total-invested"),
          type: "sol",
          sortable: true,
          minWidth: 120,
          render: (v, r) => basisCell(r, () => solCell(v)),
        },
        {
          id: "partial_exit_count",
          label: I18n.t("positions-column-exits"),
          type: "number",
          sortable: true,
          minWidth: 90,
          render: (v) => partialExitsCell(v),
        },
        {
          id: "sol_received",
          label: I18n.t("positions-column-proceeds"),
          type: "sol",
          sortable: true,
          minWidth: 110,
          render: (v) => (v == null ? "—" : solCell(v)),
        },
        {
          id: "pnl",
          label: I18n.t("positions-column-pnl"),
          type: "sol",
          sortable: true,
          minWidth: 110,
          render: (v, r) => pnlGuardedCell(r, () => pnlCell(v)),
        },
        {
          id: "pnl_percent",
          label: I18n.t("positions-column-pnl-percent"),
          type: "percent",
          sortable: true,
          minWidth: 100,
          render: (v, r) => pnlGuardedCell(r, () => percentCell(v)),
        },
      ];
    }
  };

  const updateToolbar = () => {
    if (!table) return;

    const viewLabel = I18n.label(POSITION_STATUS_LABELS, state.view);
    table.updateToolbarSummary([
      {
        id: "positions-total",
        label: viewLabel,
        value: Utils.formatNumber(state.total, 0),
      },
    ]);
  };

  // Pull in-flight buy/sell actions from the live notification stream and index
  // them by mint. Buys are shown for their whole lifecycle (in-progress, plus a
  // short grace window after completion until the position appears, plus a brief
  // linger on failure). Active sells flag their position as "selling".
  const deriveLive = () => {
    const now = Date.now();
    const buyByMint = new Map();
    const sellByMint = new Map();
    const dcaByMint = new Map();

    const considerBuy = (n, kind) => {
      if (!ACTION_BUY_TYPES.has(n?.action_type)) return;
      const mint = actionMint(n);
      if (!mint) return;
      if (kind === "completed") {
        const ts = parseTs(n.completed_at);
        if (!Number.isFinite(ts) || now - ts > BUY_GRACE_MS) return;
      } else if (kind === "failed") {
        const ts = parseTs(n.completed_at);
        if (!Number.isFinite(ts) || now - ts > FAILED_LINGER_MS) return;
      }
      const rank = kind === "active" ? 3 : kind === "completed" ? 2 : 1;
      const existing = buyByMint.get(mint);
      if (existing && existing._rank >= rank) return;
      const symbol =
        n.metadata?.symbol && n.metadata.symbol !== "Unknown" ? n.metadata.symbol : null;
      const failedStep = Array.isArray(n.steps)
        ? n.steps.find((s) => s?.status === "failed")
        : null;
      buyByMint.set(mint, {
        actionId: n.id,
        mint,
        symbol,
        size: Number(n.metadata?.size_sol) || null,
        step: shortStep(n.state?.current_step),
        kind,
        _rank: rank,
        error:
          kind === "failed"
            ? apiErrorMessage({ error: n.state?.error || failedStep?.error }, null)
            : null,
      });
    };

    // Keep the NEWEST action for a mint, not whichever one the active list happened to
    // yield last. A trade that never finished (a hung swap leaves its action in progress
    // forever) otherwise stays in the active set and can overwrite the entry for the trade
    // the user is actually watching — the row then reports a step the live trade passed
    // minutes ago.
    const keepNewest = (map, n, mint, value) => {
      const startedAt = parseTs(n.started_at);
      const existing = map.get(mint);
      if (existing && Number.isFinite(existing._startedAt)) {
        if (!Number.isFinite(startedAt) || startedAt < existing._startedAt) return;
      }
      map.set(mint, { ...value, _startedAt: startedAt });
    };

    const considerSell = (n) => {
      if (!ACTION_SELL_TYPES.has(n?.action_type)) return;
      const mint = actionMint(n);
      if (!mint) return;
      keepNewest(sellByMint, n, mint, { step: shortStep(n.state?.current_step) });
    };

    const considerDca = (n) => {
      if (!ACTION_DCA_TYPES.has(n?.action_type)) return;
      const mint = actionMint(n);
      if (!mint) return;
      keepNewest(dcaByMint, n, mint, { step: shortStep(n.state?.current_step) });
    };

    try {
      notificationManager.getActive().forEach((n) => {
        considerBuy(n, "active");
        considerSell(n);
        considerDca(n);
      });
      notificationManager.getCompleted({ includeDismissed: false }).forEach((n) => {
        considerBuy(n, "completed");
      });
      notificationManager.getFailed({ includeDismissed: false }).forEach((n) => {
        considerBuy(n, "failed");
      });
    } catch (err) {
      console.warn("[Positions] deriveLive failed:", err);
    }

    return { buyByMint, sellByMint, dcaByMint };
  };

  // Merge live action state into the server rows: annotate real rows with their
  // live state (selling/closing), prepend synthetic pending-buy rows for mints
  // that have no position yet, and tag freshly-closed rows for the arrival glow.
  const mergeLiveIntoRows = (rows) => {
    const base = Array.isArray(rows) ? rows : [];

    if (state.view !== "open") {
      const now = Date.now();
      return base.map((r) => {
        const exitMs = r.exit_time ? r.exit_time * 1000 : NaN;
        return Number.isFinite(exitMs) && now - exitMs < JUST_CLOSED_MS
          ? { ...r, _justClosed: true }
          : r;
      });
    }

    const { buyByMint, sellByMint, dcaByMint } = deriveLive();
    const presentMints = new Set(base.map((r) => r.mint));

    const annotated = base.map((r) => {
      let st = "open";
      let step = null;
      const sell = sellByMint.get(r.mint);
      const dca = dcaByMint.get(r.mint);
      if (sell) {
        st = "selling";
        step = sell.step;
      } else if (r.exit_transaction_signature && !r.transaction_exit_verified) {
        st = "closing";
      } else if (dca) {
        // An add is in flight. Its SOL and tokens only land on the row once the DCA is
        // VERIFIED, so without this the row looks untouched and the user assumes the add
        // was lost.
        st = "buying";
        step = dca.step || I18n.t("positions-step-adding");
      }
      return { ...r, _state: st, _stepLabel: step };
    });

    const nowSec = Math.floor(Date.now() / 1000);
    const pending = [];
    buyByMint.forEach((info, mint) => {
      if (presentMints.has(mint)) return; // real position exists -> it supersedes
      const failed = info.kind === "failed";
      const symbol = info.symbol || I18n.t("format-unknown");
      pending.push({
        id: `pending:${info.actionId}`,
        mint,
        symbol,
        name: failed ? I18n.t("positions-pending-buy-failed") : I18n.t("positions-pending-buying"),
        token: `${symbol} ${mint}`,
        _pending: true,
        _state: failed ? "failed" : "buying",
        _stepLabel: failed ? null : info.step,
        _error: info.error || null,
        entry_time: nowSec,
        total_size_native: info.size,
        average_entry_price: null,
        current_price: null,
        dca_count: 0,
        partial_exit_count: 0,
        unrealized_pnl: null,
        unrealized_pnl_percent: null,
        transaction_exit_verified: false,
      });
    });

    return [...pending, ...annotated];
  };

  // Re-merge live state into the cached server rows and push to the table with no
  // network call. Driven by notification-stream events between poll ticks.
  const applyLiveUpdate = () => {
    if (!table || state.view !== "open") return;
    const merged = mergeLiveIntoRows(state.lastServerRows);
    state.total = merged.length;
    table.setData(merged, { preserveScroll: true });
    updateToolbar();
  };

  const scheduleLiveUpdate = () => {
    if (liveDebounce) return;
    liveDebounce = window.setTimeout(() => {
      liveDebounce = null;
      applyLiveUpdate();
    }, 120);
  };

  const loadPositionsPage = async ({ reason, signal }) => {
    const status = state.view;
    // limit=0 is the whole tab. A capped page would quietly hide the oldest rounds of a
    // wallet with a long history, and this table is the record of that history.
    const url = `/api/positions?status=${encodeURIComponent(status)}&limit=0`;
    try {
      const rows = await requestManager.fetch(url, {
        priority: "normal",
        signal,
      });

      const mapped = (Array.isArray(rows) ? rows : []).map((row) => ({
        ...row,
        token: `${row.symbol} ${row.mint}`,
      }));
      state.lastServerRows = mapped;

      const finalRows = mergeLiveIntoRows(mapped);
      state.total = finalRows.length;

      return {
        rows: finalRows,
        cursorNext: null,
        cursorPrev: null,
        hasMoreNext: false,
        hasMorePrev: false,
        total: finalRows.length,
        preserveScroll: reason === "poll",
      };
    } catch (err) {
      if (err?.name !== "AbortError") {
        console.error("[Positions] fetch failed:", err);
        if (reason !== "poll") {
          Utils.showToast({
            key: "positions-load",
            type: "warning",
            title: I18n.t("positions-load-failed"),
          });
        }
      }
      throw err;
    }
  };

  // Handle remove (archive/delete) / restore / delete actions on a position row.
  const handleLifecycleAction = async (action, row, btn) => {
    if (!row || row.id == null) {
      Utils.showToast(I18n.t("positions-toast-not-found"), "error");
      return;
    }
    const id = row.id;
    const symbol = row.symbol || "?";

    try {
      if (action === "remove") {
        const isOpen = !row.transaction_exit_verified;
        const choice = await PositionRemoveDialog.show({ symbol, mint: row.mint, isOpen });
        if (!choice?.confirmed) return;

        if (btn) btn.disabled = true;
        if (choice.mode === "delete") {
          await requestManager.fetch(`/api/positions/${id}`, {
            method: "DELETE",
            priority: "high",
          });
          Utils.showToast(I18n.t("positions-toast-deleted"), "success");
        } else {
          await requestManager.fetch(`/api/positions/${id}/archive`, {
            method: "POST",
            priority: "high",
          });
          Utils.showToast(I18n.t("positions-toast-archived"), "success");
        }
      } else if (action === "restore") {
        if (btn) btn.disabled = true;
        await requestManager.fetch(`/api/positions/${id}/unarchive`, {
          method: "POST",
          priority: "high",
        });
        Utils.showToast(I18n.t("positions-toast-restored"), "success");
      } else if (action === "delete") {
        const confirmed = await ConfirmationDialog.show({
          title: I18n.t("positions-delete-title"),
          message: I18n.t("positions-delete-message", { symbol }),
          confirmLabel: I18n.t("positions-delete-confirm"),
          cancelLabel: I18n.t("common-action-cancel"),
          variant: "danger",
        });
        if (!confirmed?.confirmed) return;
        if (btn) btn.disabled = true;
        await requestManager.fetch(`/api/positions/${id}`, { method: "DELETE", priority: "high" });
        Utils.showToast(I18n.t("positions-toast-deleted"), "success");
      }
      table.refresh({ reason: "manual", preserveScroll: true });
    } catch (err) {
      if (btn) btn.disabled = false;
      Utils.showToast(err?.message || I18n.t("positions-action-failed"), "error");
    }
  };

  // Bulk permanent delete of every archived position.
  const handleDeleteAllArchived = async () => {
    const count = state.total;
    const confirmed = await ConfirmationDialog.show({
      title: I18n.t("positions-delete-all-title"),
      message:
        count > 0
          ? I18n.t("positions-delete-all-message", { count })
          : I18n.t("positions-delete-all-message-empty"),
      confirmLabel: I18n.t("positions-delete-all-confirm"),
      cancelLabel: I18n.t("common-action-cancel"),
      variant: "danger",
    });
    if (!confirmed?.confirmed) return;
    try {
      const res = await requestManager.fetch("/api/positions/archived", {
        method: "DELETE",
        priority: "high",
      });
      Utils.showToast(I18n.t("positions-delete-all-done", { count: res?.deleted ?? 0 }), "success");
      table.refresh({ reason: "manual", preserveScroll: true });
    } catch (err) {
      Utils.showToast(err?.message || I18n.t("positions-delete-all-failed"), "error");
    }
  };

  // Show the "Delete all archived" toolbar button only in the archived view.
  // "Delete all" only applies to the Archived view — the toolbar owns the item's
  // visibility, so no page-level markup or selector is involved.
  const syncArchivedTools = () => {
    table?.setToolbarItem("delete-all-archived", { hidden: state.view !== "archived" });
  };

  const switchView = (view) => {
    if (view !== "open" && view !== "closed" && view !== "archived") return;
    state.view = view;
    state.sort = getInitialSortForView(view);
    // Drop the previous view's cached rows so a live tick can't merge stale data.
    state.lastServerRows = [];
    if (table) {
      const nextStateKey = getPositionsTableStateKey(view);
      table.setStateKey(nextStateKey, { render: false });

      // Update columns for the new view using setColumns (systematic approach)
      const newColumns = buildColumns();
      table.setColumns(newColumns, {
        preserveData: false, // Clear data to force reload
        preserveScroll: false, // Reset scroll when switching views
        resetState: false, // Keep column widths/visibility preferences within each view
      });

      // Update sorting for the new view (defer render — the reload below renders)
      table.setSortState(state.sort.column, state.sort.direction, { render: false });

      // Load the new view's data immediately. setColumns(preserveData:false) already
      // cleared the previous view's rows, so this replaces the (now empty) table with
      // fresh data instead of leaving stale items until the next poll tick.
      table.reload({ reason: "view-change", resetScroll: true }).catch(() => {});
    }
    updateToolbar();
    syncArchivedTools();
  };

  return {
    init(ctx) {
      // Initialize dialogs
      positionDetailsDialog = new PositionDetailsDialog();

      // Sub-tabs
      tabBar = new TabBar({
        container: "#subTabsContainer",
        tabs: subTabs(),
        defaultTab: state.view,
        stateKey: "positions.activeTab",
        pageName: "positions",
        onChange: (tabId) => switchView(tabId),
      });
      TabBarManager.register("positions", tabBar);
      ctx.manageTabBar(tabBar);
      tabBar.show();

      // Sync state.view with TabBar's restored state (from server or URL)
      state.view = tabBar.getActiveTab() || state.view;
      state.sort = getInitialSortForView(state.view);

      // Build columns based on current view
      const columns = buildColumns();

      table = new DataTable({
        container: "#positions-root",
        columns,
        // Unique per position. mint is NOT unique — the same token can have many
        // positions (multiple closed trades, plus an open + closed of the same mint),
        // so keying rows by mint caused duplicate rows and cross-sub-tab row mixing.
        rowIdField: "id",
        stateKey: getPositionsTableStateKey(state.view),
        emptyTitle: () => I18n.label(POSITION_EMPTY_LABELS, state.view),
        emptyMessage: () => I18n.labelAttr(POSITION_EMPTY_LABELS, state.view, "message"),
        enableLogging: false,
        sorting: {
          mode: "client",
          column: state.sort.column,
          direction: state.sort.direction,
        },
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        uniformRowHeight: 2,
        // State-driven row styling: a tint for pending/selling/closing/failed rows,
        // and a brief arrival glow for just-closed rows.
        rowClass: (row) => {
          const classes = [];
          if (row?._state) classes.push(`pos-row-${row._state}`);
          else if (row?._justClosed) classes.push("pos-row-just-closed");
          return classes.join(" ");
        },
        rowAttributes: (row) => ({
          "data-management": row?.management || "auto_trader",
          "data-origin-kind": row?.origin?.kind || "auto",
        }),
        // Animate rows leaving the Open list as they finish closing or a pending
        // buy resolves into a real position.
        rowExitAnimation: (tr) =>
          tr.classList.contains("pos-row-selling") ||
          tr.classList.contains("pos-row-closing") ||
          tr.classList.contains("pos-row-buying"),
        pagination: {
          threshold: 160,
          maxRows: 5000,
          loadPage: loadPositionsPage,
          dedupeKey: (row) => row?.id ?? null,
          rowIdField: "id",
          onPageLoaded: () => updateToolbar(),
        },
        toolbar: {
          summary: [
            {
              id: "positions-total",
              label: I18n.t("positions-toolbar-total"),
              value: "0",
              variant: "secondary",
            },
          ],
          search: {
            enabled: true,
            mode: "client",
            placeholder: I18n.t("positions-search-placeholder"),
          },
          filters: [
            {
              id: "origin",
              label: I18n.t("positions-filter-origin"),
              options: [
                { value: "all", label: I18n.t("positions-filter-origin-all") },
                { value: "auto", label: I18n.t("positions-filter-origin-auto") },
                { value: "copy", label: I18n.t("positions-filter-origin-copy") },
                { value: "manual", label: I18n.t("positions-origin-manual") },
                { value: "external", label: I18n.t("positions-origin-wallet") },
              ],
              filterFn: (row, value) => value === "all" || (row?.origin?.kind || "auto") === value,
            },
          ],
          buttons: [
            {
              id: "delete-all-archived",
              label: I18n.t("positions-toolbar-delete-all"),
              icon: "icon-trash-2",
              variant: "danger",
              tooltip: I18n.t("positions-delete-all-tooltip"),
              onClick: () => handleDeleteAllArchived(),
            },
          ],
        },
      });

      updateToolbar();
      // The delete-all button only applies to the Archived view.
      syncArchivedTools();

      // Row actions: delegate clicks on the table container
      const containerEl = document.querySelector("#positions-root");
      const handleRowActionClick = async (e) => {
        const btn = e.target?.closest?.(".row-action");
        if (!btn) return;
        const action = btn.getAttribute("data-action");
        const mint = btn.getAttribute("data-mint");
        if (!action) return;

        // ID-based lifecycle actions (mint is not unique — a token can have many
        // positions, so these must target the exact row by id).
        if (action === "remove" || action === "restore" || action === "delete") {
          const id = btn.getAttribute("data-id");
          if (!id) return;
          const target = table.getData().find((r) => String(r.id) === id);
          await handleLifecycleAction(action, target, btn);
          return;
        }

        if (!mint) return;

        // Find row data
        const row = table.getData().find((r) => r.mint === mint);
        if (!row) {
          Utils.showToast(I18n.t("positions-toast-not-found"), "error");
          return;
        }

        // Both actions go through the shared manual-trade flow (ui/manual_trade.js),
        // which owns the dialog, the payload, the toasts AND resolves everything the
        // dialog shows — position (holdings/decimals/size), wallet balance and the
        // configured DCA presets — so this passes none of them by hand.
        const placed = await manualTrade({
          action,
          mint,
          symbol: row.symbol,
          btn,
        });

        if (placed) table.refresh({ reason: "manual", preserveScroll: true });
      };

      if (containerEl) {
        containerEl.addEventListener("click", handleRowActionClick);

        // A copied position's origin label opens the copy task that bought it.
        const handleCopyOriginClick = (e) => {
          const origin = e.target?.closest?.("[data-copy-task]");
          if (!origin) return;
          e.stopPropagation();
          openCopyTask(Number(origin.dataset.copyTask));
        };
        containerEl.addEventListener("click", handleCopyOriginClick);
        ctx.onDispose(() => containerEl.removeEventListener("click", handleCopyOriginClick));
        ctx.onDispose(() => containerEl.removeEventListener("click", handleRowActionClick));

        // Row click handler for position details dialog
        const handleRowClick = (e) => {
          // Skip if clicking on action buttons, links, or buttons
          if (
            e.target.closest(".row-action") ||
            e.target.closest("a") ||
            e.target.closest("button")
          )
            return;

          const row = e.target.closest("tr[data-row-id]");
          if (!row) return;

          // data-row-id is now the unique position id (see rowIdField above).
          const rowId = row.dataset.rowId;
          const position = table?.getData()?.find((p) => String(p.id) === rowId);
          // Pending (in-flight buy) rows have no real position record yet.
          if (position && !position._pending && positionDetailsDialog) {
            positionDetailsDialog.show(position);
          }
        };

        containerEl.addEventListener("click", handleRowClick);
        ctx.onDispose(() => containerEl.removeEventListener("click", handleRowClick));
      }

      // Update the row in place after manual management is toggled (from the row context
      // menu or the position details dialog). Updating the cached row re-applies the
      // row class immediately (the backend already persisted it), so the Manual ribbon
      // flips without waiting on a poll/refetch round-trip.
      const handleManagementChanged = (e) => {
        const { id, management } = e?.detail || {};
        if (id == null) return;
        table?.updateRow(id, { management });
      };
      window.addEventListener("screenerbot:position-management-changed", handleManagementChanged);
      ctx.onDispose(() =>
        window.removeEventListener(
          "screenerbot:position-management-changed",
          handleManagementChanged
        )
      );
    },

    activate(ctx) {
      // Re-register deactivate cleanup (cleanups are cleared after each deactivate)
      // and force-show tab bar to handle race conditions with TabBarManager
      if (tabBar) {
        ctx.manageTabBar(tabBar);
        tabBar.show({ force: true });
      }

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => table?.refresh({ reason: "poll", preserveScroll: true, silent: true }), {
            label: "Positions", // l10n-ignore: internal poller name
          })
        );
      }
      poller.start();
      if ((table?.getData?.() ?? []).length === 0) {
        table.refresh({ reason: "initial" });
      }

      // Subscribe to the live action stream so in-flight buys/sells reflect on the
      // Open list between poll ticks (instant pending row on click, live step text).
      if (!liveUnsub) {
        liveUnsub = notificationManager.subscribe(() => scheduleLiveUpdate());
      }
    },

    deactivate() {
      table?.cancelPendingLoad?.();
      if (liveUnsub) {
        liveUnsub();
        liveUnsub = null;
      }
      if (liveDebounce) {
        window.clearTimeout(liveDebounce);
        liveDebounce = null;
      }
    },

    dispose() {
      if (liveUnsub) {
        liveUnsub();
        liveUnsub = null;
      }
      if (liveDebounce) {
        window.clearTimeout(liveDebounce);
        liveDebounce = null;
      }
      if (positionDetailsDialog) {
        positionDetailsDialog.destroy();
        positionDetailsDialog = null;
      }
      if (table) {
        table.destroy();
        table = null;
      }
      poller = null;
      tabBar = null;
      TabBarManager.unregister("positions");
      state.view = "open";
      state.total = 0;
      state.lastServerRows = [];
    },
  };
}

registerPage("positions", createLifecycle());
