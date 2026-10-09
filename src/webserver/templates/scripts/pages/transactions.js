// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Transactions page - infinite-scroll DataTable over /api/transactions with server filters, a wallet selector and a TransactionDetailsDialog row dialog.

import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { withApprox } from "../core/format.js";
import { DataTable } from "../ui/data_table.js";
import { requestManager } from "../core/request_manager.js";
import { TransactionDetailsDialog } from "../ui/transaction_details_dialog.js";
import {
  TYPE_FILTER_OPTIONS,
  typeKind,
  typeLabel,
  typeShortLabel,
  typeVariant,
} from "../ui/transaction_type.js";
import {
  DIRECTION_FILTER_VALUES,
  directionBadge,
  directionLabel,
} from "../ui/transaction_direction.js";
import { listStatusBadge, statusLabel } from "../ui/transaction_status.js";
import {
  renderSignature,
  renderTokenCell,
  resolveTokenCells,
  TOKEN_CELL_MIN_WIDTH,
} from "../ui/token_identity.js";
import { venueLabel } from "../ui/venue.js";
import { renderSetupGate, setupRequired } from "../ui/setup_gate.js";

const PAGE_LIMIT = 100;
const DEFAULT_FILTERS = {
  type: "all",
  direction: "all",
  status: "all",
};

/**
 * A failed transaction carries the type "failed" because its effects were never applied,
 * and the Status column already states the failure, so the Type cell stays absent.
 */
function formatTypeBadge(value) {
  if (!value || typeKind(value) === "failed") return "—";
  // The short label fits the column; the full label is the badge's tooltip.
  const full = typeLabel(value);
  const short = typeShortLabel(value);
  const title = short === full ? "" : ` title="${Utils.escapeHtml(full)}"`;
  return `<span class="badge ${typeVariant(value)}"${title}>${Utils.escapeHtml(short)}</span>`;
}

function formatStatusBadge(status, success) {
  if (!status) return "—";
  const badge = listStatusBadge(status);
  if (badge) return badge;
  if (success === true) {
    return `<span class="badge success">${Utils.escapeHtml(status)}</span>`;
  }
  if (success === false) {
    return `<span class="badge error">${Utils.escapeHtml(status)}</span>`;
  }
  return Utils.escapeHtml(status);
}

function createLifecycle() {
  let table = null;
  let poller = null;
  let txDialog = null;

  const state = {
    subject: "",
    filters: { ...DEFAULT_FILTERS },
    signature: "",
    totalEstimate: null,
    summary: null,
  };

  let lastUserReloadAt = 0;
  const isScrolledAwayFromTop = () => (table?.elements?.scrollContainer?.scrollTop ?? 0) > 1;

  const buildFiltersPayload = () => {
    const filters = {};
    const typeValue = state.filters.type;
    const directionValue = state.filters.direction;
    const statusValue = state.filters.status;

    if (state.signature) {
      filters.signature = state.signature;
    }
    if (typeValue && typeValue !== "all") {
      filters.types = [typeValue.toLowerCase()];
    }
    if (directionValue && directionValue !== "all") {
      filters.direction = directionValue;
    }
    if (statusValue && statusValue !== "all") {
      filters.status = statusValue;
    }

    return filters;
  };

  const buildRequestPayload = (cursor = null) => ({
    subject: state.subject || null,
    filters: buildFiltersPayload(),
    pagination: {
      cursor,
      limit: PAGE_LIMIT,
    },
  });

  const updateToolbar = () => {
    if (!table) {
      return;
    }

    // The exact summary total; until it arrives, the list's row estimate marked approximate.
    const totalFromSummary = state.summary?.total;
    const totalEstimate = state.totalEstimate ?? null;
    let totalText = "—";
    if (typeof totalFromSummary === "number") {
      totalText = Utils.formatNumber(totalFromSummary, { decimals: 0 });
    } else if (totalEstimate !== null) {
      totalText = withApprox(Utils.formatNumber(totalEstimate, { decimals: 0 }));
    }

    const successCountGlobal =
      typeof state.summary?.success_count === "number" ? state.summary.success_count : null;
    const failedCountGlobal =
      typeof state.summary?.failed_count === "number" ? state.summary.failed_count : null;

    table.updateToolbarSummary([
      {
        id: "tx-total",
        label: I18n.t("transactions-summary-total"),
        value: totalText,
      },
      {
        id: "tx-success",
        label: I18n.t("transactions-summary-success"),
        value:
          successCountGlobal === null
            ? "—"
            : Utils.formatNumber(successCountGlobal, { decimals: 0 }),
        variant: Utils.countTone(successCountGlobal, "success") || "secondary",
      },
      {
        id: "tx-failed",
        label: I18n.t("transactions-summary-failed"),
        value:
          failedCountGlobal === null ? "—" : Utils.formatNumber(failedCountGlobal, { decimals: 0 }),
        variant: Utils.countTone(failedCountGlobal, "error") || "secondary",
      },
    ]);
  };

  const fetchSummary = async ({ signal: _signal } = {}) => {
    try {
      const query = state.subject ? `?subject=${encodeURIComponent(state.subject)}` : "";
      const data = await requestManager.fetch(`/api/transactions/summary${query}`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Requested-With": "fetch",
        },
        cache: "no-store",
        priority: "normal",
        skipDedup: true,
      });

      state.summary = data ?? null;
      updateToolbar();
    } catch (error) {
      if (error?.name === "AbortError") {
        throw error;
      }
      // Silent failure for summary to avoid noisy toasts
      console.warn("[Transactions] Failed to fetch summary:", error);
    }
  };

  // Logos arrive after the rows: the token cells repaint once their identities resolve.
  const loadTransactionsPage = async ({ direction, cursor, reason, signal: _signal }) => {
    const payloadCursor = direction === "prev" ? null : (cursor ?? null);
    const payload = buildRequestPayload(payloadCursor);

    try {
      const data = await requestManager.fetch("/api/transactions/list", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Requested-With": "fetch",
        },
        body: JSON.stringify(payload),
        cache: "no-store",
        priority: "normal",
        skipDedup: true,
      });
      if (
        data?.total_estimate !== undefined &&
        data.total_estimate !== null &&
        Number.isFinite(data.total_estimate)
      ) {
        state.totalEstimate = data.total_estimate;
      }

      // Close the race between the poll guard and the response: the user may
      // start scrolling while the first-page request is in flight. Keeping the
      // accumulated rows turns that late poll into a value-only no-op.
      if (reason === "poll" && isScrolledAwayFromTop()) {
        return { rows: table?.getData?.() ?? [] };
      }

      // For initial/reload direction, return all items without dedup.
      // _replaceData._isDataUnchanged() handles skip-if-same optimization.
      // Dedup is only needed for prev direction (prepending new transactions).
      if (direction !== "prev") {
        const items = Array.isArray(data?.items) ? data.items : [];
        resolveTokenCells(
          items.map((row) => row?.token_mint),
          () => table?.repaintRows()
        );
        return {
          rows: items,
          cursorNext: data?.next_cursor ?? null,
          hasMoreNext: Boolean(data?.next_cursor),
        };
      }

      const existingRows = table?.getData?.() ?? [];
      const existingKeys = new Set(
        existingRows
          .map((row) => row?.signature)
          .filter((signature) => typeof signature === "string")
      );

      const aggregated = [];
      let hitDuplicate = false;
      const processBatch = (batch) => {
        for (const row of batch) {
          const signature = row?.signature;
          if (!signature) {
            continue;
          }
          if (existingKeys.has(signature)) {
            hitDuplicate = true;
            return false;
          }
          existingKeys.add(signature);
          aggregated.push(row);
        }
        return true;
      };

      const firstItems = Array.isArray(data?.items) ? data.items : [];
      processBatch(firstItems);

      let nextCursor = data?.next_cursor ?? null;
      let guard = 0;
      const MAX_EXTRA_BATCHES = 5;

      while (nextCursor && guard < MAX_EXTRA_BATCHES && !hitDuplicate) {
        guard += 1;
        const nextPayload = buildRequestPayload(nextCursor);
        const nextData = await requestManager.fetch("/api/transactions/list", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Requested-With": "fetch",
          },
          body: JSON.stringify(nextPayload),
          cache: "no-store",
          priority: "normal",
          skipDedup: true,
        });

        const nextItems = Array.isArray(nextData?.items) ? nextData.items : [];

        processBatch(nextItems);
        nextCursor = nextData?.next_cursor ?? null;
      }

      const hasMorePrev = !hitDuplicate && Boolean(nextCursor);
      resolveTokenCells(
        aggregated.map((row) => row?.token_mint),
        () => table?.repaintRows()
      );

      return {
        rows: aggregated,
        hasMorePrev,
      };
    } catch (error) {
      if (error?.name === "AbortError") {
        throw error;
      }
      console.error("[Transactions] Failed to fetch:", error);
      if (reason !== "scroll") {
        Utils.showToast({
          key: "transactions-load",
          type: "warning",
          title: I18n.t("transactions-load-failed"),
        });
      }
      throw error;
    }
  };

  const handlePageLoaded = () => {
    updateToolbar();
  };

  const shouldSkipPollReload = () => {
    if (!table) return false;

    // If table is already loading, skip poll to avoid reload loops
    if (table.state?.isLoading) return true;

    // Skip polls shortly after user-triggered reload
    if (lastUserReloadAt && Date.now() - lastUserReloadAt < 2500) return true;

    const paginationState =
      typeof table.getPaginationState === "function" ? table.getPaginationState() : null;
    if (
      paginationState?.loadingNext ||
      paginationState?.loadingPrev ||
      paginationState?.loadingInitial
    ) {
      return true;
    }

    // Skip only while a control inside the table holds user state a reload would destroy (a
    // focused text input or select). Hovering and focused BUTTONS must NOT skip the poll: this
    // guard gates the FETCH, so they stopped the table receiving data at all — and a clicked
    // button stays focused in Chromium, which froze the table indefinitely after any click.
    // DataTable already updates values in place under the cursor without moving anything.
    const container = table?.elements?.container;
    if (container) {
      // A poll reload replaces the accumulated infinite-scroll window with the
      // first server page. Only do that at the top; otherwise the shorter row set
      // clamps scrollTop onto unrelated records and the visible history jumps.
      if (isScrolledAwayFromTop()) return true;

      const focusedElement = document.activeElement;
      if (focusedElement && container.contains(focusedElement)) {
        const tagName = focusedElement.tagName?.toLowerCase();
        if (tagName === "input" || tagName === "select" || tagName === "textarea") return true;
      }
    }

    return false;
  };

  const requestReload = (reason = "manual", options = {}) => {
    if (!table) return Promise.resolve(null);
    if (reason === "poll" && shouldSkipPollReload()) return Promise.resolve(null);

    if (reason !== "poll") {
      lastUserReloadAt = Date.now();
    }

    return table.reload({
      reason,
      silent: options.silent ?? false,
      preserveScroll: options.preserveScroll ?? false,
      resetScroll: options.resetScroll ?? false,
    });
  };

  const resetFilters = () => {
    state.filters = { ...DEFAULT_FILTERS };
    state.signature = "";
    if (table) {
      table.setToolbarSearchValue("", { apply: false });
      table.setToolbarFilterValue("type", state.filters.type, {
        apply: false,
      });
      table.setToolbarFilterValue("direction", state.filters.direction, {
        apply: false,
      });
      table.setToolbarFilterValue("status", state.filters.status, {
        apply: false,
      });
    }
    return requestReload("reset", {
      silent: false,
      resetScroll: true,
    }).catch(() => {});
  };

  return {
    init(_ctx) {
      // Explore Mode has no main wallet: the page is the setup notice alone, with no
      // table, no transaction reads and no poller.
      if (setupRequired()) {
        $("#transactions-root").hidden = true;
        const gate = $("#transactions-setup-gate");
        gate.hidden = false;
        renderSetupGate(gate, I18n.t("transactions-setup-gate-title"));
        return;
      }

      const columns = [
        {
          id: "timestamp",
          label: I18n.t("transactions-col-time"),
          minWidth: 130,
          floating: true,
          wrap: false,
          render: (value) =>
            Utils.formatTimestamp(value, {
              includeYear: false,
              includeSeconds: false,
              fallback: "—",
            }),
        },
        {
          id: "transaction_type",
          label: I18n.t("transactions-col-type"),
          // The widest label on screen sets the floor, so a badge never runs under
          // the next column; short labels keep that floor small enough for 1200px.
          minWidth: "content",
          render: (value) => formatTypeBadge(value),
        },
        {
          id: "direction",
          label: I18n.t("transactions-col-direction"),
          minWidth: "content",
          render: (value) => directionBadge(value, "—"),
        },
        {
          id: "status",
          label: I18n.t("transactions-col-status"),
          minWidth: "content",
          render: (value, row) => formatStatusBadge(value, row?.success),
        },
        {
          id: "native_delta",
          label: I18n.t("transactions-col-native-delta"),
          type: "sol",
          minWidth: 110,
          render: (value) => Utils.formatPnL(value, { fallback: "—", unit: false, trim: false }),
        },
        {
          id: "fee_sol",
          label: I18n.t("transactions-col-fees"),
          type: "sol",
          minWidth: 100,
          // The one exception to significant-digit SOL figures: network fees sit near
          // 0.000005, so the column keeps 6 fixed decimals and never reads as zero.
          render: (value) =>
            Utils.formatSol(value, { decimals: 6, fallback: "—", suffix: "", trim: false }),
        },
        {
          id: "token_mint",
          label: I18n.t("transactions-col-token"),
          grow: true,
          minWidth: TOKEN_CELL_MIN_WIDTH,
          render: (value, row) =>
            renderTokenCell(row?.token_mint?.trim(), { symbol: row?.token_symbol?.trim() }),
        },
        {
          id: "router",
          label: I18n.t("transactions-col-router"),
          minWidth: 90,
          render: (value) =>
            value === null || value === undefined ? "—" : Utils.escapeHtml(venueLabel(value)),
        },
        {
          id: "instructions_count",
          label: I18n.t("transactions-col-instructions"),
          type: "number",
          minWidth: 64,
          // Every transaction has at least one instruction; 0 is a row stored before the
          // count was recorded, so it reads as unknown.
          render: (value) =>
            Utils.formatNumber(value > 0 ? value : null, { decimals: 0, fallback: "—" }),
        },
        {
          id: "signature",
          label: I18n.t("transactions-col-signature"),
          minWidth: 130,
          render: (value) => renderSignature(value),
        },
      ];

      table = new DataTable({
        container: "#transactions-root",
        columns,
        rowIdField: "signature",
        stateKey: "transactions-table",
        emptyTitle: I18n.t("transactions-empty"),
        emptyMessage: I18n.attr("transactions-empty", "message"),
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        onRowClick: (row) => {
          if (row && row.signature) {
            if (!txDialog) {
              txDialog = new TransactionDetailsDialog();
            }
            txDialog.show({ ...row, subject: state.subject });
          }
        },
        pagination: {
          threshold: 320,
          maxRows: 1200,
          loadPage: loadTransactionsPage,
          dedupeKey: (row) => row?.signature ?? null,
          rowIdField: "signature",
          onPageLoaded: handlePageLoaded,
        },
        toolbar: {
          layout: "query-row",
          identity: {
            icon: "icon-arrow-left-right",
            title: I18n.t("transactions-toolbar-title"),
          },
          summary: [
            { id: "tx-total", label: I18n.t("transactions-summary-total"), value: "—" },
            {
              id: "tx-success",
              label: I18n.t("transactions-summary-success"),
              value: "—",
              variant: "secondary",
            },
            {
              id: "tx-failed",
              label: I18n.t("transactions-summary-failed"),
              value: "—",
              variant: "secondary",
            },
          ],
          controls: [
            {
              id: "search",
              type: "search",
              mode: "server",
              placeholder: I18n.attr("transactions-search", "placeholder"),
              ariaLabel: I18n.attr("transactions-search", "aria-label"),
              onChange: (value, el, options) => {
                state.signature = (value || "").trim();
                if (options?.restored) {
                  return;
                }
              },
              onSubmit: () => {
                requestReload("search", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
            // Which wallet's history is shown. Options are filled in by
            // `setupSubjectSelector()` and stay first in the filter group.
            {
              id: "subject",
              type: "select",
              label: I18n.t("transactions-filter-wallet"),
              mode: "server",
              autoApply: false,
              minWidth: "170px",
              options: [{ value: "", label: I18n.t("transactions-wallet-main") }],
              onChange: (value, el, options) => {
                state.subject = value || "";
                state.summary = null;
                state.totalEstimate = null;
                // State restoration only syncs `state.subject`; init loads the data.
                if (options?.restored) {
                  return;
                }
                Promise.all([
                  fetchSummary({}),
                  requestReload("subject", { silent: false, resetScroll: true }),
                ]).catch(() => {});
              },
            },
            {
              id: "type",
              type: "select",
              label: I18n.t("transactions-filter-type"),
              mode: "server",
              defaultValue: state.filters.type,
              autoApply: false,
              options: TYPE_FILTER_OPTIONS,
              onChange: (value, el, options) => {
                state.filters.type = value || "all";
                // Skip reload if this is state restoration
                if (options?.restored) {
                  return;
                }
                requestReload("filter", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
            {
              id: "direction",
              type: "select",
              label: I18n.t("transactions-filter-direction"),
              mode: "server",
              defaultValue: state.filters.direction,
              autoApply: false,
              options: [
                { value: "all", label: I18n.t("transactions-filter-all-directions") },
                ...DIRECTION_FILTER_VALUES.map((value) => ({
                  value,
                  label: directionLabel(value),
                })),
              ],
              onChange: (value, el, options) => {
                state.filters.direction = value || "all";
                // Skip reload if this is state restoration
                if (options?.restored) {
                  return;
                }
                requestReload("filter", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
            {
              id: "status",
              type: "select",
              label: I18n.t("transactions-filter-status"),
              mode: "server",
              defaultValue: state.filters.status,
              autoApply: false,
              options: [
                { value: "all", label: I18n.t("transactions-filter-all-statuses") },
                { value: "Pending", label: statusLabel("Pending") },
                { value: "Confirmed", label: statusLabel("Confirmed") },
                { value: "Finalized", label: statusLabel("Finalized") },
                { value: "Failed", label: statusLabel("Failed") },
              ],
              onChange: (value, el, options) => {
                state.filters.status = value || "all";
                // Skip reload if this is state restoration
                if (options?.restored) {
                  return;
                }
                requestReload("filter", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
          ],
          buttons: [
            {
              id: "reset",
              label: I18n.t("common-action-reset"),
              icon: "icon-rotate-ccw",
              onClick: () => resetFilters(),
            },
          ],
        },
      });

      // The cursor API is strictly timestamp-descending. Client-sorting only the
      // loaded window makes later pages reorder above the viewport and presents a
      // false global sort, so retain the server's stable cursor order.
      table.setSortState(null, "desc", { render: true });

      // Sync state from DataTable's restored server state
      const serverState = table.getServerState();
      if (serverState.searchQuery) {
        state.signature = serverState.searchQuery;
      }
      if (serverState.filters.type) {
        state.filters.type = serverState.filters.type;
      }
      // A saved direction the filter no longer offers (an id from before the
      // direction named its subject) falls back to every direction.
      if (DIRECTION_FILTER_VALUES.includes(serverState.filters.direction)) {
        state.filters.direction = serverState.filters.direction;
      }
      if (serverState.filters.status) {
        state.filters.status = serverState.filters.status;
      }
      if (serverState.filters.subject) {
        state.subject = serverState.filters.subject;
      }

      table.setToolbarSearchValue(state.signature, { apply: false });
      table.setToolbarFilterValue("type", state.filters.type, {
        apply: false,
      });
      table.setToolbarFilterValue("direction", state.filters.direction, {
        apply: false,
      });
      table.setToolbarFilterValue("status", state.filters.status, {
        apply: false,
      });
      updateToolbar();
      // The main table is already usable with the main-wallet option. Watched
      // subjects enhance the selector when their independent request returns.
      void setupSubjectSelector();
    },

    activate(ctx) {
      if (setupRequired()) return;
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(
            () => {
              fetchSummary({});
              requestReload("poll", { silent: true, preserveScroll: true });
            },
            { label: "Transactions" } // l10n-ignore: poller log name
          )
        );
      }
      poller.start();
      if ((table?.getData?.() ?? []).length === 0) {
        Promise.all([
          fetchSummary({}),
          requestReload("initial", {
            silent: false,
            resetScroll: true,
          }),
        ]).catch(() => {});
      }
    },

    deactivate() {
      table?.cancelPendingLoad();
    },

    dispose() {
      if (poller) {
        poller.stop({ silent: true });
        poller = null;
      }
      if (table) {
        table.destroy();
        table = null;
      }
      if (txDialog) {
        txDialog.close();
        txDialog = null;
      }
      state.filters = { ...DEFAULT_FILTERS };
      state.subject = "";
      state.signature = "";
      state.totalEstimate = null;
      state.summary = null;
    },
  };

  // Fills the toolbar's subject selector with the watched wallets. The change
  // handler is declared with the control itself in the toolbar config.
  async function setupSubjectSelector() {
    if (!table) return;

    const options = [{ value: "", label: I18n.t("transactions-wallet-main") }];
    let targetsLoaded = false;
    try {
      const data = await requestManager.fetch("/api/wallets/watch", { priority: "normal" });
      for (const target of data.targets || []) {
        // A labelled wallet is named by its label; an unlabelled one by its full address.
        options.push({ value: target.address, label: target.label || target.address });
      }
      targetsLoaded = true;
    } catch (error) {
      console.warn("[Transactions] Failed to load watched-wallet subjects:", error);
    }
    if (!table) return;

    // A persisted subject can outlive its watch target. The API rejects an unwatched
    // subject with 403 SUBJECT_NOT_WATCHED on the list, summary and detail routes, and
    // the selector cannot display the stale value, so the page would silently freeze
    // while showing "Main wallet". Fall back to the main wallet and persist that.
    const staleSubject =
      targetsLoaded && state.subject && !options.some((option) => option.value === state.subject);
    if (staleSubject) {
      console.warn(
        "[Transactions] Saved wallet filter is no longer watched; using main wallet:",
        state.subject
      );
      state.subject = "";
      state.summary = null;
      state.totalEstimate = null;
    }

    table.setToolbarSelectOptions("subject", options, state.subject);

    if (staleSubject) {
      table.setToolbarFilterValue("subject", "", { apply: false });
      Promise.all([
        fetchSummary({}),
        requestReload("subject", { silent: false, resetScroll: true }),
      ]).catch(() => {});
    }
  }
}

registerPage("transactions", createLifecycle());
