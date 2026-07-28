import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { DataTable } from "../ui/data_table.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { manualTrade } from "../ui/manual_trade.js";
import { PositionDetailsDialog } from "../ui/position_details_dialog.js";
import { PositionRemoveDialog } from "../ui/position_remove_dialog.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { notificationManager } from "../core/notifications.js";

const SUB_TABS = [
  { id: "open", label: '<i class="icon-trending-up"></i> Open' },
  { id: "closed", label: '<i class="icon-trending-down"></i> Closed' },
  { id: "archived", label: '<i class="icon-archive"></i> Archived' },
];

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
// (`total_size_sol += sol_spent`, `remaining_token_amount += tokens_bought`), several
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

// Map a backend step name to a short, user-friendly label for the state caption.
const shortStep = (step) => {
  const s = String(step || "").toLowerCase();
  if (s.includes("valid")) return "Checking";
  if (s.includes("quote")) return "Quote";
  if (s.includes("swap")) return "Swapping";
  if (s.includes("verif")) return "Confirming";
  return step || "Working";
};

const actionMint = (n) => n?.entity_id || n?.metadata?.mint || "";
const actionStatus = (n) => n?.state?.status || "";
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
  // Open rows stay clean — the left status bar + tint carry the state there.
  const stateCaption = (row) => {
    const st = row?._state;
    if (!st || st === "open") return "";
    const step = row?._stepLabel ? Utils.escapeHtml(row._stepLabel) : "";
    let label;
    let icon;
    if (st === "buying") {
      label = step ? `Buying · ${step}` : "Buying";
      icon = '<span class="pos-state-spinner" aria-hidden="true"></span>';
    } else if (st === "selling") {
      label = step ? `Selling · ${step}` : "Selling";
      icon = '<span class="pos-state-spinner" aria-hidden="true"></span>';
    } else if (st === "closing") {
      label = "Closing";
      icon = '<span class="pos-state-spinner" aria-hidden="true"></span>';
    } else if (st === "failed") {
      label = row?._error ? `Failed · ${Utils.escapeHtml(String(row._error))}` : "Failed";
      icon = '<i class="icon-triangle-alert" aria-hidden="true"></i>';
    } else {
      return "";
    }
    return `<div class="pos-state-caption pos-state-${Utils.escapeHtml(st)}" title="${Utils.escapeHtml(
      label
    )}">${icon}<span class="pos-state-text">${label}</span></div>`;
  };

  // Systematic per-position row indicators.
  //
  // These surface persistent per-position status (e.g. "Manual management") as a compact
  // vertical label on the ROW's left edge (the first/actions cell), drawn purely in CSS.
  // The token logo/symbol column is never touched, so logos stay aligned across every row
  // whether or not a row has an indicator.
  //
  // To add a new indicator: append ONE descriptor here AND one CSS rule
  // (`.pos-ind-<id> > td:first-child::after { content: "<Label>"; }`) in positions.css.
  // Indicators are priority-ordered; the first matching one is shown (a single row has
  // room for one vertical label).
  //   { id, test(row)->bool, label }
  const POSITION_INDICATORS = [
    {
      id: "manual",
      test: (row) => Boolean(row?.manual_management),
      label: "Manual management",
    },
  ];

  // The `pos-ind pos-ind-<id>` row class for the highest-priority active indicator (or "").
  const positionIndicatorClass = (row) => {
    const primary = POSITION_INDICATORS.find((d) => {
      try {
        return d.test(row);
      } catch {
        return false;
      }
    });
    return primary ? `pos-ind pos-ind-${primary.id}` : "";
  };

  const tokenCell = (row, actionsHtml = "", actionCount = 0) => {
    const logo = row.logo_url || row.image_url || "";
    const symbol = row.symbol || "?";
    const name = row.name || "";
    const logoHtml = logo
      ? `<img class="token-logo" src="${Utils.escapeHtml(logo)}" alt="${Utils.escapeHtml(
          symbol
        )}"/>`
      : '<i class="token-logo icon-coins"></i>';
    return `<div class="position-token position-token--actions-${actionCount}">
      <div class="position-token__identity">
        ${logoHtml}
        <div class="position-token-meta">
          <div class="token-symbol">${Utils.escapeHtml(symbol)}</div>
          <div class="token-name">${Utils.escapeHtml(name)}</div>
          ${stateCaption(row)}
        </div>
      </div>
      ${actionsHtml}
    </div>`;
  };

  const priceCell = (value) => Utils.formatPriceSol(value, { fallback: "—", decimals: 12 });
  const solCell = (v) => Utils.formatSol(v, { decimals: 4 });
  const pnlCell = (v) => Utils.formatPnL(v, { decimals: 4 });
  const percentCell = (v) => Utils.formatPercent(v, { style: "pnl", decimals: 2, fallback: "—" });
  const timeCell = (v) => Utils.formatTimeFromSeconds(v, { includeSeconds: false });

  const dcaCell = (count) => {
    if (!count || count === 0) return "—";
    return `<span class="chip">${count} DCA${count > 1 ? "s" : ""}</span>`;
  };

  const partialExitsCell = (count) => {
    if (!count || count === 0) return "—";
    return `<span class="chip warning">${count} exit${count > 1 ? "s" : ""}</span>`;
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
    return `<span class="chip ${cls}">${pct}%</span>`;
  };

  // Compact "Remove" (archive/delete) button for closed rows.
  const removeActionCell = (row) => {
    const id = row?.id;
    if (id == null) return "";
    return `<div class="row-actions position-token__actions">
      <button class="btn row-action row-action--icon" data-action="remove" data-id="${Utils.escapeHtml(
        String(id)
      )}" data-mint="${Utils.escapeHtml(
        row?.mint || ""
      )}" title="Remove (archive or delete)" aria-label="Remove position"><i class="icon-trash-2"></i></button>
    </div>`;
  };

  // Restore + permanent-delete buttons for archived rows.
  const archivedActionCell = (row) => {
    const id = row?.id;
    if (id == null) return "";
    const idAttr = Utils.escapeHtml(String(id));
    const mintAttr = Utils.escapeHtml(row?.mint || "");
    return `<div class="row-actions position-token__actions">
      <button class="btn row-action" data-action="restore" data-id="${idAttr}" data-mint="${mintAttr}" title="Restore to Open/Closed" aria-label="Restore position"><i class="icon-rotate-ccw"></i></button>
      <button class="btn row-action row-action--icon row-action--danger" data-action="delete" data-id="${idAttr}" data-mint="${mintAttr}" title="Delete permanently" aria-label="Delete permanently"><i class="icon-trash-2"></i></button>
    </div>`;
  };

  const openActionCell = (row) => {
    const mint = row?.mint || "";
    const isOpen = !row?.transaction_exit_verified;
    if (!mint || !isOpen) return "";

    if (row?._pending) {
      return '<div class="position-token__actions"><span class="row-actions-busy">In progress…</span></div>';
    }

    const busy = row?._state === "selling" || row?._state === "closing";
    const dis = busy ? " disabled" : "";
    const mintAttr = Utils.escapeHtml(mint);
    const idAttr = Utils.escapeHtml(String(row?.id ?? ""));
    return `
      <div class="row-actions position-token__actions">
        <button class="btn row-action" data-action="add" data-mint="${mintAttr}" title="Add to position (DCA)" aria-label="Add to position"${dis}><i class="icon-circle-plus"></i></button>
        <button class="btn row-action" data-action="sell" data-mint="${mintAttr}" title="Sell (full or % partial)" aria-label="Sell position"${dis}><i class="icon-trending-down"></i></button>
        <button class="btn row-action row-action--icon" data-action="remove" data-id="${idAttr}" data-mint="${mintAttr}" title="Remove (archive or delete)" aria-label="Remove position"><i class="icon-trash-2"></i></button>
      </div>
    `;
  };

  const buildArchivedColumns = () => [
    {
      id: "token",
      label: "Token",
      sortable: true,
      floating: true,
      minWidth: 260,
      wrap: false,
      render: (_v, r) => tokenCell(r, archivedActionCell(r), 2),
    },
    {
      id: "archived_at",
      label: "Archived",
      sortable: true,
      minWidth: 140,
      render: (v, r) => timeCell(v ?? r.exit_time ?? r.entry_time),
    },
    {
      id: "entry_time",
      label: "Entry Time",
      sortable: true,
      minWidth: 140,
      render: (v) => timeCell(v),
    },
    {
      id: "average_entry_price",
      label: "Avg Entry (SOL)",
      sortable: true,
      minWidth: 140,
      render: (v, r) => priceCell(v || r.entry_price),
    },
    {
      id: "total_size_sol",
      label: "Total Invested",
      sortable: true,
      minWidth: 120,
      render: (v) => solCell(v),
    },
    {
      id: "sol_received",
      label: "Proceeds",
      sortable: true,
      minWidth: 110,
      render: (v) => (v == null ? "—" : solCell(v)),
    },
    { id: "pnl", label: "PnL", sortable: true, minWidth: 110, render: (v) => pnlCell(v) },
    {
      id: "pnl_percent",
      label: "PnL %",
      sortable: true,
      minWidth: 100,
      render: (v) => percentCell(v),
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
          label: "Token",
          sortable: true,
          floating: true,
          minWidth: 260,
          wrap: false,
          render: (_v, r) => tokenCell(r, openActionCell(r), 3),
        },
        {
          id: "entry_time",
          label: "Entry Time",
          sortable: true,
          minWidth: 140,
          render: (v) => timeCell(v),
        },
        {
          id: "dca_count",
          label: "DCA",
          sortable: true,
          minWidth: 80,
          render: (v) => dcaCell(v),
        },
        {
          id: "average_entry_price",
          label: "Avg Entry (SOL)",
          sortable: true,
          minWidth: 140,
          render: (v) => priceCell(v),
        },
        {
          id: "current_price",
          label: "Current (SOL)",
          sortable: true,
          minWidth: 140,
          render: (v) => (v == null ? "—" : priceCell(v)),
        },
        {
          id: "total_size_sol",
          label: "Total Invested",
          sortable: true,
          minWidth: 120,
          render: (v) => solCell(v),
        },
        {
          id: "current_size",
          label: "Size",
          sortable: true,
          minWidth: 80,
          render: (_v, r) => currentSizeCell(r.remaining_token_amount, r.total_exited_amount),
        },
        {
          id: "partial_exit_count",
          label: "Exits",
          sortable: true,
          minWidth: 90,
          render: (v) => partialExitsCell(v),
        },
        {
          id: "unrealized_pnl",
          label: "Unrealized PnL",
          sortable: true,
          minWidth: 130,
          render: (v) => pnlCell(v),
        },
        {
          id: "unrealized_pnl_percent",
          label: "Unrealized %",
          sortable: true,
          minWidth: 110,
          render: (v) => percentCell(v),
        },
      ];
    } else if (state.view === "archived") {
      return buildArchivedColumns();
    } else {
      // closed view
      return [
        {
          id: "token",
          label: "Token",
          sortable: true,
          floating: true,
          minWidth: 260,
          wrap: false,
          render: (_v, r) => tokenCell(r, removeActionCell(r), 1),
        },
        {
          id: "exit_time",
          label: "Exit Time",
          sortable: true,
          minWidth: 140,
          render: (v) => (v == null ? "—" : timeCell(v)),
        },
        {
          id: "dca_count",
          label: "DCA",
          sortable: true,
          minWidth: 80,
          render: (v) => dcaCell(v),
        },
        {
          id: "average_entry_price",
          label: "Avg Entry (SOL)",
          sortable: true,
          minWidth: 140,
          render: (v, r) => priceCell(v || r.entry_price),
        },
        {
          id: "average_exit_price",
          label: "Avg Exit (SOL)",
          sortable: true,
          minWidth: 140,
          render: (v, r) => (v == null ? priceCell(r.exit_price) : priceCell(v)),
        },
        {
          id: "total_size_sol",
          label: "Total Invested",
          sortable: true,
          minWidth: 120,
          render: (v) => solCell(v),
        },
        {
          id: "partial_exit_count",
          label: "Exits",
          sortable: true,
          minWidth: 90,
          render: (v) => partialExitsCell(v),
        },
        {
          id: "sol_received",
          label: "Proceeds",
          sortable: true,
          minWidth: 110,
          render: (v) => (v == null ? "—" : solCell(v)),
        },
        { id: "pnl", label: "PnL", sortable: true, minWidth: 110, render: (v) => pnlCell(v) },
        {
          id: "pnl_percent",
          label: "PnL %",
          sortable: true,
          minWidth: 100,
          render: (v) => percentCell(v),
        },
      ];
    }
  };

  const updateToolbar = () => {
    if (!table) return;

    const viewLabel =
      state.view === "open" ? "Open" : state.view === "archived" ? "Archived" : "Closed";
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
        error: kind === "failed" ? n.state?.error || failedStep?.error || null : null,
      });
    };

    const considerSell = (n) => {
      if (!ACTION_SELL_TYPES.has(n?.action_type)) return;
      const mint = actionMint(n);
      if (!mint) return;
      sellByMint.set(mint, { step: shortStep(n.state?.current_step) });
    };

    const considerDca = (n) => {
      if (!ACTION_DCA_TYPES.has(n?.action_type)) return;
      const mint = actionMint(n);
      if (!mint) return;
      dcaByMint.set(mint, { step: shortStep(n.state?.current_step) });
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
        step = dca.step || "Adding";
      }
      return { ...r, _state: st, _stepLabel: step };
    });

    const nowSec = Math.floor(Date.now() / 1000);
    const pending = [];
    buyByMint.forEach((info, mint) => {
      if (presentMints.has(mint)) return; // real position exists -> it supersedes
      const failed = info.kind === "failed";
      const shortMint = `${mint.slice(0, 4)}…${mint.slice(-4)}`;
      pending.push({
        id: `pending:${info.actionId}`,
        mint,
        symbol: info.symbol || mint.slice(0, 4),
        name: failed ? "Buy failed" : "Buying…",
        token: `${info.symbol || mint.slice(0, 4)} (${shortMint})`,
        _pending: true,
        _state: failed ? "failed" : "buying",
        _stepLabel: failed ? null : info.step,
        _error: info.error || null,
        entry_time: nowSec,
        total_size_sol: info.size,
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
    const url = `/api/positions?status=${encodeURIComponent(status)}&limit=500`;
    try {
      const rows = await requestManager.fetch(url, {
        priority: "normal",
        signal,
      });

      const mapped = (Array.isArray(rows) ? rows : []).map((row) => ({
        ...row,
        token: `${row.symbol} (${row.mint.slice(0, 4)}…${row.mint.slice(-4)})`,
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
          Utils.showToast("Failed to refresh positions", "warning");
        }
      }
      throw err;
    }
  };

  // Handle remove (archive/delete) / restore / delete actions on a position row.
  const handleLifecycleAction = async (action, row, btn) => {
    if (!row || row.id == null) {
      Utils.showToast("Position data not found", "error");
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
          Utils.showToast("Position deleted", "success");
        } else {
          await requestManager.fetch(`/api/positions/${id}/archive`, {
            method: "POST",
            priority: "high",
          });
          Utils.showToast("Position archived", "success");
        }
      } else if (action === "restore") {
        if (btn) btn.disabled = true;
        await requestManager.fetch(`/api/positions/${id}/unarchive`, {
          method: "POST",
          priority: "high",
        });
        Utils.showToast("Position restored", "success");
      } else if (action === "delete") {
        const confirmed = await ConfirmationDialog.show({
          title: "Delete position permanently",
          message: `Permanently delete ${symbol}? This removes the position and its history from the database and cannot be undone. Your transactions and token data are not affected.`,
          confirmLabel: "Delete permanently",
          cancelLabel: "Cancel",
          variant: "danger",
        });
        if (!confirmed?.confirmed) return;
        if (btn) btn.disabled = true;
        await requestManager.fetch(`/api/positions/${id}`, { method: "DELETE", priority: "high" });
        Utils.showToast("Position deleted", "success");
      }
      table.refresh({ reason: "manual", preserveScroll: true });
    } catch (err) {
      if (btn) btn.disabled = false;
      Utils.showToast(err?.message || "Action failed", "error");
    }
  };

  // Bulk permanent delete of every archived position.
  const handleDeleteAllArchived = async () => {
    const count = state.total;
    const confirmed = await ConfirmationDialog.show({
      title: "Delete all archived positions",
      message:
        count > 0
          ? `Permanently delete all ${count} archived position(s)? This cannot be undone. Transactions and token data are not affected.`
          : "Permanently delete all archived positions? This cannot be undone.",
      confirmLabel: "Delete all",
      cancelLabel: "Cancel",
      variant: "danger",
    });
    if (!confirmed?.confirmed) return;
    try {
      const res = await requestManager.fetch("/api/positions/archived", {
        method: "DELETE",
        priority: "high",
      });
      Utils.showToast(`Deleted ${res?.deleted ?? 0} archived position(s)`, "success");
      table.refresh({ reason: "manual", preserveScroll: true });
    } catch (err) {
      Utils.showToast(err?.message || "Failed to delete archived positions", "error");
    }
  };

  // Show the "Delete all archived" toolbar button only in the archived view.
  const syncArchivedTools = () => {
    const btn = document.querySelector(
      '#positions-root .table-toolbar-btn[data-btn-id="delete-all-archived"]'
    );
    if (btn) btn.hidden = state.view !== "archived";
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
        tabs: SUB_TABS,
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
      const viewLabel = state.view === "open" ? "Open" : "Closed";

      table = new DataTable({
        container: "#positions-root",
        columns,
        // Unique per position. mint is NOT unique — the same token can have many
        // positions (multiple closed trades, plus an open + closed of the same mint),
        // so keying rows by mint caused duplicate rows and cross-sub-tab row mixing.
        rowIdField: "id",
        stateKey: getPositionsTableStateKey(state.view),
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
        // State-driven row styling: left status bar + tint for pending/selling/
        // closing/failed rows, and a brief arrival glow for just-closed rows.
        rowClass: (row) => {
          const classes = [];
          if (row?._state) classes.push(`pos-row-${row._state}`);
          else if (row?._justClosed) classes.push("pos-row-just-closed");
          // Persistent per-position indicators (e.g. manual management) — row-level left
          // edge label, independent of the transient state classes above.
          const indicator = positionIndicatorClass(row);
          if (indicator) classes.push(indicator);
          return classes.join(" ");
        },
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
          summary: [{ id: "positions-total", label: "Total", value: "0", variant: "secondary" }],
          search: {
            enabled: true,
            mode: "client",
            placeholder: "Search by symbol or mint...",
          },
          buttons: [
            {
              id: "delete-all-archived",
              label: "Delete all",
              icon: "icon-trash-2",
              variant: "danger",
              tooltip: "Permanently delete all archived positions",
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
          Utils.showToast("Position data not found", "error");
          return;
        }

        // Both actions go through the shared manual-trade flow (ui/manual_trade.js),
        // which owns the dialog, the payload, the toasts AND resolves the position
        // (holdings/decimals/size) itself — so this no longer passes them by hand.
        const context = {};
        if (action === "add") {
          // DCA presets for the add dialog's size buttons.
          let entrySizes = [0.005, 0.01, 0.02, 0.05];
          try {
            const configData = await requestManager.fetch("/api/config/trader", {
              priority: "normal",
            });
            if (Array.isArray(configData?.data?.entry_sizes)) {
              entrySizes = configData.data.entry_sizes;
            }
          } catch (err) {
            console.warn("Failed to fetch entry_sizes config:", err);
          }
          // The positions API returns NEITHER `entry_sol` NOR `sol_size` — both were dead
          // reads, so this was always undefined and the add dialog's multiplier presets
          // (1.0x / 1.5x / 2.0x) never had a basis. `total_size_sol` is the money actually in
          // the position (entry + every DCA), which is what an add should be sized against.
          context.entrySize = row.total_size_sol ?? row.entry_size_sol;
          context.entrySizes = entrySizes;
        }

        const placed = await manualTrade({
          action,
          mint,
          symbol: row.symbol,
          btn,
          context,
        });

        if (placed) table.refresh({ reason: "manual", preserveScroll: true });
      };

      if (containerEl) {
        containerEl.addEventListener("click", handleRowActionClick);
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
        const { id, enabled } = e?.detail || {};
        if (id == null) return;
        table?.updateRow(id, { manual_management: !!enabled });
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
            label: "Positions",
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
