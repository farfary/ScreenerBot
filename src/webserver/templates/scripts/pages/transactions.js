import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";
import { requestManager } from "../core/request_manager.js";
import { TransactionDetailsDialog } from "../ui/transaction_details_dialog.js";

const PAGE_LIMIT = 100;
const DEFAULT_FILTERS = {
  type: "all",
  direction: "all",
  status: "all",
};

function formatTimestamp(value) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "—";
  return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`;
}

function formatSignatureLink(signature) {
  if (!signature) return "—";
  const safe = Utils.escapeHtml(signature);
  return `<a class="mono-text" href="https://solscan.io/tx/${safe}" target="_blank" rel="noopener">${safe}</a>`;
}

function formatTypeBadge(value) {
  if (!value) return "—";
  const key = String(value).toLowerCase();
  const types = {
    buy: { label: "Buy", variant: "success" },
    sell: { label: "Sell", variant: "error" },
    swap: { label: "Swap", variant: "info" },
    transfer: { label: "Transfer", variant: "secondary" },
    ata: { label: "ATA", variant: "secondary" },
    failed: { label: "Failed", variant: "error" },
    unknown: { label: "Unknown", variant: "secondary" },
  };
  const info = types[key];
  if (!info) {
    return Utils.escapeHtml(value);
  }
  return `<span class="badge ${info.variant}">${info.label}</span>`;
}

function formatDirectionBadge(value) {
  if (!value) return "—";
  const map = {
    Incoming: { text: "↓ Incoming", variant: "success" },
    Outgoing: { text: "↑ Outgoing", variant: "error" },
    Internal: { text: "⟲ Internal", variant: "secondary" },
    Unknown: { text: "? Unknown", variant: "secondary" },
  };
  const info = map[value] ?? null;
  if (!info) {
    return Utils.escapeHtml(value);
  }
  return `<span class="badge ${info.variant}">${info.text}</span>`;
}

function formatStatusBadge(status, success) {
  if (!status) return "—";
  const map = {
    Pending: { text: '<i class="icon-loader"></i> Pending', variant: "warning" },
    Confirmed: { text: '<i class="icon-check"></i> Confirmed', variant: "success" },
    Finalized: { text: '<i class="icon-check-check"></i> Finalized', variant: "success" },
    Failed: { text: '<i class="icon-x"></i> Failed', variant: "error" },
  };
  const info = map[status];
  if (!info) {
    if (success === true) {
      return `<span class="badge success">${Utils.escapeHtml(status)}</span>`;
    }
    if (success === false) {
      return `<span class="badge error">${Utils.escapeHtml(status)}</span>`;
    }
    return Utils.escapeHtml(status);
  }
  return `<span class="badge ${info.variant}">${info.text}</span>`;
}

function formatTokenDisplay(row) {
  const symbol = row?.token_symbol?.trim();
  if (symbol) {
    return Utils.escapeHtml(symbol);
  }
  const mint = row?.token_mint?.trim();
  if (!mint) {
    return "—";
  }
  if (mint.length <= 8) {
    return Utils.escapeHtml(mint);
  }
  const short = `${mint.slice(0, 4)}…${mint.slice(-4)}`;
  return `<span class="mono-text" title="${Utils.escapeHtml(mint)}">${Utils.escapeHtml(short)}</span>`;
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
  const isScrolledAwayFromTop = () =>
    (table?.elements?.scrollContainer?.scrollTop ?? 0) > 1;

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

    // Prefer exact totals from summary; fall back to estimate if unavailable
    const totalFromSummary = state.summary?.total;
    const totalEstimate =
      state.totalEstimate === null || state.totalEstimate === undefined
        ? null
        : state.totalEstimate;
    const totalValue =
      typeof totalFromSummary === "number" ? totalFromSummary : (totalEstimate ?? null);

    const successCountGlobal =
      typeof state.summary?.success_count === "number" ? state.summary.success_count : null;
    const failedCountGlobal =
      typeof state.summary?.failed_count === "number" ? state.summary.failed_count : null;

    table.updateToolbarSummary([
      {
        id: "tx-total",
        label: "Total",
        value: totalValue === null ? "—" : Utils.formatNumber(totalValue, { decimals: 0 }),
      },
      {
        id: "tx-estimate",
        label: "Estimate",
        value: totalEstimate === null ? "—" : Utils.formatNumber(totalEstimate, { decimals: 0 }),
      },
      {
        id: "tx-success",
        label: "Success",
        value:
          successCountGlobal === null
            ? "—"
            : Utils.formatNumber(successCountGlobal, { decimals: 0 }),
        variant:
          typeof successCountGlobal === "number" && successCountGlobal > 0
            ? "success"
            : "secondary",
      },
      {
        id: "tx-failed",
        label: "Failed",
        value:
          failedCountGlobal === null ? "—" : Utils.formatNumber(failedCountGlobal, { decimals: 0 }),
        variant:
          typeof failedCountGlobal === "number" && failedCountGlobal > 0 ? "warning" : "success",
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
        Utils.showToast("Failed to refresh transactions", "warning");
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
    async init(_ctx) {
      const columns = [
        {
          id: "timestamp",
          label: "Time",
          minWidth: 160,
          floating: true,
          render: (value) => formatTimestamp(value),
        },
        {
          id: "signature",
          label: "Signature",
          minWidth: 300,
          render: (value) => formatSignatureLink(value),
        },
        {
          id: "transaction_type",
          label: "Type",
          minWidth: 120,
          render: (value) => formatTypeBadge(value),
        },
        {
          id: "direction",
          label: "Direction",
          minWidth: 130,
          render: (value) => formatDirectionBadge(value),
        },
        {
          id: "status",
          label: "Status",
          minWidth: 120,
          render: (value, row) => formatStatusBadge(value, row?.success),
        },
        {
          id: "sol_delta",
          label: "Δ SOL",
          minWidth: 140,
          render: (value) => Utils.formatPnL(value, { decimals: 6, fallback: "—" }),
        },
        {
          id: "fee_sol",
          label: "Fees (SOL)",
          minWidth: 130,
          render: (value) => Utils.formatSol(value, { decimals: 6, fallback: "—" }),
        },
        {
          id: "token_mint",
          label: "Token",
          minWidth: 140,
          render: (value, row) => formatTokenDisplay(row),
        },
        {
          id: "router",
          label: "Router",
          minWidth: 140,
          render: (value) => value ?? "—",
        },
        {
          id: "instructions_count",
          label: "Instr.",
          minWidth: 90,
          render: (value) => Utils.formatNumber(value, { decimals: 0, fallback: "—" }),
        },
      ];

      table = new DataTable({
        container: "#transactions-root",
        columns,
        rowIdField: "signature",
        stateKey: "transactions-table",
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
            title: "Transaction history",
          },
          summary: [
            { id: "tx-total", label: "Total", value: "—" },
            { id: "tx-estimate", label: "Estimate", value: "—" },
            { id: "tx-success", label: "Success", value: "—", variant: "secondary" },
            { id: "tx-failed", label: "Failed", value: "—", variant: "success" },
          ],
          controls: [
            {
              id: "search",
              type: "search",
              mode: "server",
              placeholder: "Search signatures…",
              ariaLabel: "Search transaction signatures",
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
              label: "Wallet",
              mode: "server",
              autoApply: false,
              minWidth: "170px",
              options: [{ value: "", label: "Main wallet" }],
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
              label: "Type",
              mode: "server",
              defaultValue: state.filters.type,
              autoApply: false,
              options: [
                { value: "all", label: "All Types" },
                { value: "buy", label: "Buy" },
                { value: "sell", label: "Sell" },
                { value: "swap", label: "Swap" },
                { value: "transfer", label: "Transfer" },
                { value: "ata", label: "ATA" },
                { value: "failed", label: "Failed" },
                { value: "unknown", label: "Unknown" },
              ],
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
              label: "Direction",
              mode: "server",
              defaultValue: state.filters.direction,
              autoApply: false,
              options: [
                { value: "all", label: "All Directions" },
                { value: "Incoming", label: "Incoming" },
                { value: "Outgoing", label: "Outgoing" },
                { value: "Internal", label: "Internal" },
                { value: "Unknown", label: "Unknown" },
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
              label: "Status",
              mode: "server",
              defaultValue: state.filters.status,
              autoApply: false,
              options: [
                { value: "all", label: "All Statuses" },
                { value: "Pending", label: "Pending" },
                { value: "Confirmed", label: "Confirmed" },
                { value: "Finalized", label: "Finalized" },
                { value: "Failed", label: "Failed" },
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
              label: "Reset",
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
      if (serverState.filters.direction) {
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
      await setupSubjectSelector();
    },

    activate(ctx) {
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(
            () => {
              fetchSummary({});
              requestReload("poll", { silent: true, preserveScroll: true });
            },
            { label: "Transactions" }
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

    const options = [{ value: "", label: "Main wallet" }];
    try {
      const data = await requestManager.fetch("/api/wallets/watch", { priority: "normal" });
      for (const target of data.targets || []) {
        const short = `${target.address.slice(0, 6)}…${target.address.slice(-4)}`;
        options.push({
          value: target.address,
          label: target.label ? `${target.label} · ${short}` : short,
        });
      }
    } catch (error) {
      console.warn("[Transactions] Failed to load watched-wallet subjects:", error);
    }

    table.setToolbarSelectOptions("subject", options, state.subject);
  }
}

registerPage("transactions", createLifecycle());
