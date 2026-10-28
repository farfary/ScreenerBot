// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Strategies roster - the strategy list of the Strategies sub-tab: one DataTable with an All, Entry and Exit view of the server's status and figures of each strategy, the switch that turns it on, its mode action and the trader-level block stated once in the toolbar.

import { DataTable } from "../../ui/data_table.js";
import { STRATEGY_TYPE_ICONS, strategyTypeLabel, strategyTypeMarker } from "../../ui/strategy_type.js";
import { tradingModeLabel, tradingModeMarker } from "../../ui/trading_mode.js";
import { setupRequired, setupRequiredReason } from "../../ui/setup_gate.js";
import { SetupDialog } from "../../ui/setup_dialog.js";
import { setAutoTraderEnabled } from "../../ui/auto_trader_control.js";
import { requestManager } from "../../core/request_manager.js";
import * as Utils from "../../core/utils.js";
import { setStrategyEnabled } from "./enable.js";
import { modeAction } from "./strategy_mode.js";

const OVERVIEW_URL = "/api/strategies/overview";

/**
 * The views in toolbar order: every strategy, then one per kind. `type` is the API kind a
 * view holds, null for every kind.
 */
const VIEWS = Object.freeze([
  { value: "all", type: null, icon: "icon-layers" },
  { value: "entry", type: "ENTRY", icon: STRATEGY_TYPE_ICONS.ENTRY },
  { value: "exit", type: "EXIT", icon: STRATEGY_TYPE_ICONS.EXIT },
]);

/** Message key of each `StrategyRunStatus` (src/strategies/overview.rs). */
const STATUS_LABELS = Object.freeze({
  running: "strategies-status-running",
  not_running: "strategies-status-not-running",
  off: "strategies-status-off",
  setup_required: "strategies-status-setup-required",
  live_paused: "strategies-status-live-paused",
  paper_cash_short: "strategies-status-paper-cash",
  slots_full: "strategies-status-slots-full",
  entry_monitor_off: "strategies-status-entry-monitor-off",
  exit_monitor_off: "strategies-status-exit-monitor-off",
});

/** Message key of each trader-level `RunNotice` (src/strategies/overview.rs). */
const NOTICE_LABELS = Object.freeze({
  setup_required: "strategies-notice-setup-required",
  force_stopped: "strategies-notice-force-stopped",
  trader_off: "strategies-notice-trader-off",
  waiting: "strategies-notice-waiting",
  monitors_off: "strategies-notice-monitors-off",
});

/** Message key of each `RuleExitKind`: the name of its Auto Trader sub-tab. */
const RULE_EXIT_LABELS = Object.freeze({
  stop_loss: "trader-tab-stop-loss",
  trailing_stop: "trader-tab-trailing-stop",
  take_profit: "trader-tab-take-profit",
  time_exit: "trader-tab-time-rules",
});

/** Message key of each readiness check id (src/trader/paper/readiness.rs, src/strategies/readiness.rs). */
const READINESS_CHECK_LABELS = Object.freeze({
  history: "paper-readiness-history",
  profit: "paper-readiness-profit",
  priced: "strategies-readiness-priced",
  exits: "paper-readiness-exits",
  runtime: "strategies-readiness-runtime",
});

const ABSENT = "—";
const esc = Utils.escapeHtml;

/** A strategy's name, or the unnamed label. */
export const strategyName = (row) => (row?.name ? String(row.name) : I18n.t("strategies-unnamed"));

/** One roster row of an overview entry: the list item's fields beside the summary's. */
function rowOf({ item, ...summary }) {
  return {
    ...summary,
    id: item.id,
    name: item.name,
    description: item.description ?? null,
    type: item.strategy_type,
    enabled: Boolean(item.enabled),
    priority: item.priority,
    mode: item.mode,
    entry_strategy_ids: item.entry_strategy_ids ?? [],
  };
}

/** A count, or the absent mark when the server could not attribute one. */
const countText = (value) =>
  value === null || value === undefined ? ABSENT : Utils.formatNumber(value, { decimals: 0 });

/** A column header carrying its one-line hint as tooltip; DataTable inserts it as markup. */
const hinted = (label, hint) => `<span title="${esc(hint)}">${esc(label)}</span>`;

/**
 * Create the roster inside `container`. `onOpen(id)` opens a strategy in the editor,
 * `onCreate(type)` starts a new one of that kind, `onImport()` reads an export file,
 * `onDelete(row)` deletes one, and `strategyMode` carries the arm and return flows.
 */
export function createRoster({ container, onOpen, onCreate, onImport, onDelete, strategyMode }) {
  let table = null;
  let view = VIEWS[0].value;
  let overview = null;
  let rows = [];
  let lastKey = null;
  let loadFailed = false;
  let loadedAt = null;
  const busy = new Set();

  const viewOf = (value) => VIEWS.find((entry) => entry.value === value) ?? VIEWS[0];
  /** A kind's place in the switch, so All lists entries before exits. */
  const kindOrder = (row) => VIEWS.findIndex((entry) => entry.type === row.type);
  /** The rows of the current view in the server's order; All groups them by kind. */
  const rowsOfView = () => {
    const { type } = viewOf(view);
    if (type) return rows.filter((row) => row.type === type);
    return [...rows].sort((a, b) => kindOrder(a) - kindOrder(b));
  };
  const stateKey = () => `strategies-roster-${view}`;

  // ---------------------------------------------------------------------------
  // Cells
  // ---------------------------------------------------------------------------

  function switchCell(row) {
    const gated = setupRequired();
    const disabled = gated || busy.has(row.id) ? " disabled" : "";
    const title = gated ? ` title="${esc(setupRequiredReason())}"` : "";
    return `<label class="toggle strategy-roster-switch"${title}><input type="checkbox" data-enable-strategy="${esc(
      String(row.id)
    )}" aria-label="${esc(strategyName(row))}"${row.enabled ? " checked" : ""}${disabled} /><span class="toggle-track"></span></label>`;
  }

  function nameCell(row) {
    const name = strategyName(row);
    const description = String(row.description ?? "").trim();
    return `<span class="strategy-roster-name">${esc(name)}</span>${
      description
        ? `<span class="strategy-roster-description" title="${esc(description)}">${esc(description)}</span>`
        : ""
    }`;
  }

  /**
   * How far a paper strategy is from arming live: ready, still counting rounds, or
   * held by a check other than the round count.
   */
  function readinessState(readiness) {
    if (readiness.ready) return "ready";
    return readiness.failing && readiness.failing !== "history" ? "blocked" : "counting";
  }

  /**
   * The readiness tooltip: the counted rounds with the rules change they count from,
   * then the check that is not passing.
   */
  function readinessTitle(readiness) {
    const count = { count: readiness.closed, needed: readiness.needed };
    const rounds = readiness.since_rules_changed
      ? I18n.t("strategies-readiness-history-since-short", {
          ...count,
          changed: Utils.formatTimestamp(readiness.since_rules_changed, {
            includeSeconds: false,
            fallback: ABSENT,
          }),
        })
      : I18n.t("paper-readiness-history-short", count);
    if (readinessState(readiness) !== "blocked") return rounds;
    return `${rounds}\n${I18n.t("strategies-readiness-failing", {
      check: I18n.label(READINESS_CHECK_LABELS, readiness.failing),
    })}`;
  }

  /**
   * The mode marker, and under a paper strategy one line of how far it is from arming
   * live; the tooltip carries the rules change and the check that holds it.
   */
  function modeCell(row) {
    const marker = tradingModeMarker(row.mode, esc);
    const readiness = row.mode === "paper" ? row.readiness : null;
    if (!readiness) return marker;
    const text = readiness.ready
      ? I18n.t("strategies-readiness-ready")
      : I18n.t("strategies-readiness-progress", {
          closed: readiness.closed,
          needed: readiness.needed,
        });
    return `${marker}<span class="strategy-roster-detail strategy-roster-readiness" data-readiness="${readinessState(
      readiness
    )}" title="${esc(readinessTitle(readiness))}">${esc(text)}</span>`;
  }

  function statusCell(row) {
    return `<span class="strategy-roster-status" data-status="${esc(String(row.status))}">${esc(
      I18n.label(STATUS_LABELS, row.status)
    )}</span>`;
  }

  /** Open positions in the strategy's book, then those still open in its other book. */
  function openCell(row) {
    const other = Number(row.open_other_book) || 0;
    if (other === 0) return countText(row.open);
    const book = row.book === "paper" ? "live" : "paper";
    return `${countText(row.open)}<span class="strategy-roster-detail">${esc(
      I18n.t("strategies-open-other-book", { count: other, book: tradingModeLabel(book) })
    )}</span>`;
  }

  /** The entry strategies an exit strategy manages: every position, or their names. */
  function managesCell(row) {
    const ids = row.entry_strategy_ids ?? [];
    if (ids.length === 0) return esc(I18n.t("strategies-manages-all"));
    const names = ids.map((id) => {
      const entry = rows.find((candidate) => candidate.id === id);
      return entry ? strategyName(entry) : I18n.t("strategies-editor-entry-binding-deleted", { id });
    });
    const text =
      names.length === 1
        ? names[0]
        : I18n.t("strategies-manages-some", { first: names[0], more: names.length - 1 });
    return `<span title="${esc(Utils.formatList(names))}">${esc(text)}</span>`;
  }

  const winRateCell = (row) =>
    Utils.formatPercentValue(row.rounds?.win_rate_pct, {
      decimals: 1,
      includeSign: false,
      fallback: ABSENT,
    });

  const realizedCell = (row) =>
    Utils.formatPnL(row.rounds?.realized_native, { fallback: ABSENT, unit: false });

  function lastSignalCell(row) {
    if (!row.last_signal_at) return ABSENT;
    return `<span title="${esc(Utils.formatTimestamp(row.last_signal_at))}">${esc(
      Utils.formatTimeAgo(row.last_signal_at, { fallback: ABSENT })
    )}</span>`;
  }

  /**
   * The mode action while the strategy is on, then Edit and Delete, each named by the
   * strategy. The column is end-aligned, so the icon pair lines up on every row.
   */
  function actionButtons(row) {
    const name = strategyName(row);
    const buttons = [];
    const mode = modeAction(row);
    if (mode) {
      buttons.push({
        id: mode.id,
        label: esc(mode.label),
        ariaLabel: `${mode.label}: ${name}`,
        tooltip: setupRequired() ? setupRequiredReason() : null,
        disabled: setupRequired() || busy.has(row.id),
        onClick: () => void changeMode(row, mode.id),
      });
    }
    buttons.push({
      id: "edit",
      icon: '<i class="icon-square-pen" aria-hidden="true"></i>',
      tooltip: I18n.t("common-action-edit"),
      ariaLabel: I18n.t("strategies-action-edit", { name }),
      onClick: () => onOpen(row.id),
    });
    buttons.push({
      id: "delete",
      icon: '<i class="icon-trash-2" aria-hidden="true"></i>',
      tooltip: I18n.t("common-action-delete"),
      ariaLabel: I18n.t("strategies-action-delete", { name }),
      variant: "danger",
      onClick: () => onDelete(row),
    });
    return buttons;
  }

  // ---------------------------------------------------------------------------
  // Columns per view: All shows what every kind shares, a kind view adds its own
  // ---------------------------------------------------------------------------

  function columns() {
    const on = {
      id: "on",
      label: esc(I18n.t("strategies-col-on")),
      minWidth: "content",
      floating: true,
      render: (_value, row) => switchCell(row),
    };
    const strategy = {
      id: "name",
      label: esc(I18n.t("strategies-col-strategy")),
      grow: true,
      floating: true,
      sortable: true,
      wrap: false,
      minWidth: 160,
      sortFn: (a, b) => strategyName(a).localeCompare(strategyName(b)),
      searchValue: (row) => `${strategyName(row)} ${row.description ?? ""}`,
      render: (_value, row) => nameCell(row),
    };
    const mode = {
      id: "mode",
      label: esc(I18n.t("strategies-col-mode")),
      wrap: false,
      minWidth: "content",
      render: (_value, row) => modeCell(row),
    };
    const status = {
      id: "status",
      label: esc(I18n.t("strategies-col-status")),
      minWidth: "content",
      render: (_value, row) => statusCell(row),
    };
    const rounds = {
      id: "rounds",
      label: esc(I18n.t("strategies-col-rounds")),
      type: "number",
      render: (_value, row) => countText(row.rounds?.closed),
    };
    const winRate = {
      id: "win_rate",
      label: esc(I18n.t("strategies-col-win-rate")),
      type: "percent",
      render: (_value, row) => winRateCell(row),
    };
    const realized = {
      id: "realized",
      label: I18n.t("strategies-col-realized"),
      type: "sol",
      render: (_value, row) => realizedCell(row),
    };
    const lastSignal = {
      id: "last_signal",
      label: esc(I18n.t("strategies-col-last-signal")),
      render: (_value, row) => lastSignalCell(row),
    };
    const actions = {
      id: "actions",
      label: "",
      type: "actions",
      resizable: false,
      actions: { buttons: actionButtons },
    };

    if (view === "all") {
      return [
        on,
        strategy,
        {
          id: "kind",
          label: esc(I18n.t("strategies-col-kind")),
          sortable: true,
          minWidth: "content",
          sortFn: (a, b) => strategyTypeLabel(a.type).localeCompare(strategyTypeLabel(b.type)),
          searchValue: (row) => strategyTypeLabel(row.type),
          render: (_value, row) => strategyTypeMarker(row.type, esc),
        },
        mode,
        status,
        rounds,
        winRate,
        realized,
        lastSignal,
        actions,
      ];
    }
    if (view === "exit") {
      return [
        on,
        strategy,
        {
          id: "manages",
          label: esc(I18n.t("strategies-col-manages")),
          wrap: false,
          minWidth: "content",
          render: (_value, row) => managesCell(row),
        },
        mode,
        status,
        {
          id: "watching",
          label: esc(I18n.t("strategies-col-watching")),
          type: "number",
          render: (_value, row) => countText(row.watching),
        },
        {
          id: "closed",
          label: esc(I18n.t("strategies-col-closed")),
          type: "number",
          render: (_value, row) => countText(row.rounds?.closed),
        },
        winRate,
        realized,
        lastSignal,
        actions,
      ];
    }
    return [
      on,
      strategy,
      {
        id: "order",
        label: hinted(I18n.t("strategies-col-rank"), I18n.attr("strategies-col-rank", "title")),
        type: "number",
        render: (_value, row) => countText(row.order),
      },
      mode,
      status,
      {
        id: "open",
        label: esc(I18n.t("strategies-col-open")),
        type: "number",
        render: (_value, row) => openCell(row),
      },
      rounds,
      winRate,
      realized,
      lastSignal,
      actions,
    ];
  }

  // ---------------------------------------------------------------------------
  // Toolbar
  // ---------------------------------------------------------------------------

  function viewOptions() {
    return VIEWS.map((entry) => ({
      value: entry.value,
      label: entry.type ? strategyTypeLabel(entry.type) : I18n.t("strategies-view-all"),
      icon: entry.icon,
    }));
  }

  /** The counts on the view switch: strategies on, of all in that view. */
  function syncCounts() {
    const counts = overview?.counts;
    if (!counts) return;
    table?.setToolbarSegmentCounts("kind", {
      all: `${counts.entry.enabled + counts.exit.enabled}/${counts.entry.total + counts.exit.total}`,
      entry: `${counts.entry.enabled}/${counts.entry.total}`,
      exit: `${counts.exit.enabled}/${counts.exit.total}`,
    });
  }

  /**
   * The trader-level block, stated once: why no enabled strategy runs and, where the
   * dashboard can lift it, the action that does.
   */
  function syncNotice() {
    const notice = overview?.notice ?? null;
    table?.updateToolbarSummary([
      {
        id: "run-notice",
        value: notice ? I18n.label(NOTICE_LABELS, notice) : ABSENT,
        variant: "warning",
        hidden: !notice,
      },
    ]);
    const action =
      notice === "trader_off"
        ? I18n.t("strategies-notice-start")
        : notice === "setup_required"
          ? I18n.t("shell-explore-action")
          : null;
    table?.setToolbarItem("run-notice-action", action ? { hidden: false, label: action } : { hidden: true });
  }

  /** A view holding exit strategies names the rule exits that decide before any of them. */
  function syncRuleExits() {
    const names = (overview?.rule_exits ?? [])
      .filter((rule) => rule.enabled)
      .map((rule) => I18n.label(RULE_EXIT_LABELS, rule.kind));
    table?.updateToolbarSummary([
      {
        id: "rule-exits",
        value: names.length > 0 ? Utils.formatList(names) : ABSENT,
        hidden: view === "entry" || names.length === 0,
      },
    ]);
  }

  async function runNoticeAction() {
    const notice = overview?.notice;
    if (notice === "setup_required") {
      SetupDialog.show();
      return;
    }
    if (notice !== "trader_off") return;
    table?.setToolbarItem("run-notice-action", { busy: true });
    try {
      const result = await setAutoTraderEnabled(true);
      if (!result.cancelled) await load();
    } finally {
      table?.setToolbarItem("run-notice-action", { busy: false });
    }
  }

  /**
   * Two rows: the title, the trader-level state and the actions, then the kind switch
   * and the search.
   */
  function toolbar() {
    return {
      layout: "query-row",
      identity: { icon: "icon-puzzle", title: I18n.t("trader-tab-strategies") },
      summary: [
        {
          id: "run-notice",
          label: I18n.t("strategies-col-status"),
          value: ABSENT,
          format: "text",
          variant: "warning",
          hidden: true,
        },
        {
          id: "rule-exits",
          label: I18n.t("strategies-exit-before"),
          value: ABSENT,
          format: "text",
          tooltip: I18n.t("strategies-exit-hint"),
          hidden: true,
        },
        {
          id: "stale",
          label: I18n.t("strategies-roster-stale"),
          value: ABSENT,
          variant: "warning",
          hidden: true,
        },
      ],
      views: {
        id: "kind",
        value: view,
        ariaLabel: I18n.t("strategies-view-kind"),
        options: viewOptions(),
        onChange: (value) => switchView(value),
      },
      search: { enabled: true, mode: "client", placeholder: I18n.t("strategies-search-placeholder") },
      buttons: [
        {
          id: "run-notice-action",
          label: I18n.t("strategies-notice-start"),
          hidden: true,
          onClick: () => void runNoticeAction(),
        },
        {
          id: "import-strategy",
          label: I18n.attr("strategies-import", "title"),
          icon: "icon-download",
          onClick: () => onImport(),
        },
        {
          id: "create-strategy",
          label: I18n.t("strategies-new"),
          icon: "icon-plus",
          variant: "primary",
          onClick: () => onCreate(viewOf(view).type),
        },
      ],
    };
  }

  // ---------------------------------------------------------------------------
  // Rendering
  // ---------------------------------------------------------------------------

  /**
   * Put the current view's rows in the table. A control focused before the render is
   * focused again after it, so a refresh never drops the keyboard user's place. `force`
   * repaints rows the table holds already: the empty state reads `loadFailed`, and the
   * table skips a render whose rows did not change.
   */
  function render({ force = false } = {}) {
    if (!table) return;
    const active = container.ownerDocument.activeElement;
    const enableId = active?.dataset?.enableStrategy ?? null;
    table.setData(rowsOfView(), { forceRender: force });
    if (enableId !== null && !active.isConnected) {
      container
        .querySelector(`input[data-enable-strategy="${CSS.escape(enableId)}"]`)
        ?.focus({ preventScroll: true });
    }
  }

  function switchView(value) {
    view = viewOf(value).value;
    table.setStateKey(stateKey(), { render: false });
    table.setColumns(columns(), { preserveData: false, preserveScroll: false, resetState: false });
    syncRuleExits();
    render();
  }

  function mount() {
    if (table) return;
    table = new DataTable({
      container,
      columns: columns(),
      rowIdField: "id",
      stateKey: stateKey(),
      sorting: { mode: "client", column: null, direction: "asc" },
      compact: true,
      stickyHeader: true,
      zebra: true,
      fitToContainer: true,
      uniformRowHeight: 2,
      emptyTitle: () =>
        loadFailed ? I18n.t("strategies-toast-load-failed") : I18n.t("strategies-list-empty-title"),
      emptyMessage: () =>
        loadFailed
          ? I18n.attr("strategies-toast-load-failed", "message")
          : I18n.t("strategies-list-empty-hint"),
      emptyAction: () =>
        loadFailed
          ? { label: I18n.t("common-action-retry"), onClick: () => void load() }
          : { label: I18n.t("strategies-new"), onClick: () => onCreate(viewOf(view).type) },
      onRowClick: (row) => onOpen(row.id),
      toolbar: toolbar(),
    });
    container.addEventListener("change", onSwitchChange);
  }

  // ---------------------------------------------------------------------------
  // Loading
  // ---------------------------------------------------------------------------

  /**
   * Read the overview and render it when it changed. A failed read keeps the rows already
   * shown and marks them stale; only a first read that fails shows the error with Retry.
   */
  async function load() {
    if (!table) return;
    if (overview === null && !loadFailed) {
      table.showBlockingState({ variant: "loading", title: I18n.t("table-loading-message") });
    }
    try {
      const data = await requestManager.fetch(OVERVIEW_URL, { priority: "normal" });
      const recovered = loadFailed;
      loadFailed = false;
      loadedAt = Date.now();
      table.hideBlockingState();
      table.updateToolbarSummary([{ id: "stale", value: ABSENT, hidden: true }]);
      const content = data ?? {};
      // The response time changes on every read; the change key leaves it out.
      const key = JSON.stringify({ ...content, timestamp: undefined });
      if (key === lastKey) return;
      lastKey = key;
      overview = content;
      rows = (content.strategies ?? []).map(rowOf);
      syncCounts();
      syncNotice();
      syncRuleExits();
      render({ force: recovered });
    } catch (error) {
      console.error("[Strategies] Failed to load the strategies overview:", error);
      table.hideBlockingState();
      if (overview !== null) {
        table.updateToolbarSummary([
          {
            id: "stale",
            value: Utils.formatTimeAgo(loadedAt, { fallback: ABSENT }),
            variant: "warning",
            hidden: false,
          },
        ]);
        return;
      }
      loadFailed = true;
      table.setData([], { forceRender: true });
    }
  }

  /** Read the overview again even when nothing seems to have changed. */
  function reload() {
    lastKey = null;
    return load();
  }

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  /** The switch moved: the row follows at once and returns if the server refuses. */
  async function onSwitchChange(event) {
    const input = event.target.closest?.("input[data-enable-strategy]");
    if (!input) return;
    const row = rows.find((candidate) => String(candidate.id) === input.dataset.enableStrategy);
    if (!row) return;
    const enabled = input.checked;
    busy.add(row.id);
    input.disabled = true;
    const changed = await setStrategyEnabled(row.id, enabled);
    busy.delete(row.id);
    if (!changed) {
      input.checked = !enabled;
      input.disabled = false;
      return;
    }
    // A new row rather than an edit of the one the table holds: the table compares the next
    // rows with its own, so an edited row reads as unchanged and the mode action the switch
    // brings never appears. The render also releases the switch if the reload fails.
    rows = rows.map((candidate) => (candidate === row ? { ...row, enabled } : candidate));
    render();
    await reload();
  }

  async function changeMode(row, actionId) {
    busy.add(row.id);
    table?.repaintRows();
    const act = actionId === "arm" ? strategyMode.armLive : strategyMode.returnToPaper;
    const changed = await act(row);
    busy.delete(row.id);
    if (changed) await reload();
    else table?.repaintRows();
  }

  return {
    mount,
    load,
    reload,
    /** The roster row of strategy `id`, as last read. */
    row: (id) => rows.find((row) => row.id === id) ?? null,
    /** The entry strategies an exit strategy can be bound to, in roster order. */
    entries: () => rows.filter((row) => row.type === "ENTRY"),
    destroy() {
      container.removeEventListener("change", onSwitchChange);
      table?.destroy();
      table = null;
      overview = null;
      rows = [];
      lastKey = null;
      loadFailed = false;
    },
  };
}
