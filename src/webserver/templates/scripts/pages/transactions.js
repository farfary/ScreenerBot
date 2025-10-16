import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";

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
    Pending: { text: "⏳ Pending", variant: "warning" },
    Confirmed: { text: "✓ Confirmed", variant: "success" },
    Finalized: { text: "✓✓ Finalized", variant: "success" },
    Failed: { text: "✗ Failed", variant: "error" },
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

  const state = {
    filters: { ...DEFAULT_FILTERS },
    signature: "",
    totalEstimate: null,
  };

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

    const rows = table.getData();
    const loaded = rows.length;
    let successCount = 0;
    let failedCount = 0;

    for (const row of rows) {
      if (row?.success) {
        successCount += 1;
      } else if (row?.success === false || row?.status === "Failed") {
        failedCount += 1;
      }
    }

    const totalEstimate =
      state.totalEstimate === null || state.totalEstimate === undefined
        ? null
        : state.totalEstimate;

    table.updateToolbarSummary([
      {
        id: "tx-loaded",
        label: "Loaded",
        value: Utils.formatNumber(loaded, { decimals: 0 }),
      },
      {
        id: "tx-estimate",
        label: "Estimate",
        value:
          totalEstimate === null
            ? "—"
            : Utils.formatNumber(totalEstimate, { decimals: 0 }),
      },
      {
        id: "tx-success",
        label: "Success",
        value: Utils.formatNumber(successCount, { decimals: 0 }),
        variant: successCount > 0 ? "success" : "secondary",
      },
      {
        id: "tx-failed",
        label: "Failed",
        value: Utils.formatNumber(failedCount, { decimals: 0 }),
        variant: failedCount > 0 ? "warning" : "success",
      },
    ]);

    table.updateToolbarMeta([
      {
        id: "tx-last-update",
        text: `Last update ${new Date().toLocaleTimeString()}`,
      },
    ]);
  };

  const loadTransactionsPage = async ({
    direction,
    cursor,
    reason,
    signal,
  }) => {
    const payloadCursor = direction === "prev" ? null : cursor ?? null;
    const payload = buildRequestPayload(payloadCursor);

    try {
      const response = await fetch("/api/transactions/list", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Requested-With": "fetch",
        },
        body: JSON.stringify(payload),
        cache: "no-store",
        signal,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      if (
        data?.total_estimate !== undefined &&
        data.total_estimate !== null &&
        Number.isFinite(data.total_estimate)
      ) {
        state.totalEstimate = data.total_estimate;
      }

      const existingRows = table?.getData?.() ?? [];
      const existingKeys = new Set(
        existingRows
          .map((row) => row?.signature)
          .filter((signature) => typeof signature === "string")
      );

      if (direction === "prev") {
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
          const nextResponse = await fetch("/api/transactions/list", {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "X-Requested-With": "fetch",
            },
            body: JSON.stringify(nextPayload),
            cache: "no-store",
            signal,
          });

          if (!nextResponse.ok) {
            throw new Error(
              `HTTP ${nextResponse.status}: ${nextResponse.statusText}`
            );
          }

          const nextData = await nextResponse.json();
          const nextItems = Array.isArray(nextData?.items)
            ? nextData.items
            : [];

          processBatch(nextItems);
          nextCursor = nextData?.next_cursor ?? null;
        }

        const hasMorePrev = !hitDuplicate && Boolean(nextCursor);

        return {
          rows: aggregated,
          hasMorePrev,
        };
      }

      const items = Array.isArray(data?.items) ? data.items : [];
      const fresh = [];
      for (const row of items) {
        const signature = row?.signature;
        if (!signature) {
          continue;
        }
        if (existingKeys.has(signature)) {
          continue;
        }
        existingKeys.add(signature);
        fresh.push(row);
      }

      return {
        rows: fresh,
        cursorNext: data?.next_cursor ?? null,
        hasMoreNext: Boolean(data?.next_cursor),
      };
    } catch (error) {
      if (error?.name === "AbortError") {
        throw error;
      }
      console.error("[Transactions] Failed to fetch:", error);
      if (reason !== "scroll") {
        Utils.showToast("⚠️ Failed to refresh transactions", "warning");
      }
      throw error;
    }
  };

  const handlePageLoaded = () => {
    updateToolbar();
  };

  const requestReload = (reason = "manual", options = {}) => {
    if (!table) {
      return Promise.resolve(null);
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
      const columns = [
        {
          id: "timestamp",
          label: "Time",
          minWidth: 160,
          sortable: true,
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
          sortable: true,
          render: (value) =>
            Utils.formatPnL(value, { decimals: 6, fallback: "—" }),
        },
        {
          id: "fee_sol",
          label: "Fees (SOL)",
          minWidth: 130,
          sortable: true,
          render: (value) =>
            Utils.formatSol(value, { decimals: 6, fallback: "—" }),
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
          sortable: true,
          render: (value) =>
            Utils.formatNumber(value, { decimals: 0, fallback: "—" }),
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
        sorting: {
          column: "timestamp",
          direction: "desc",
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
          title: {
            icon: "💸",
            text: "Transactions",
            meta: [{ id: "tx-last-update", text: "Last update —" }],
          },
          summary: [
            { id: "tx-loaded", label: "Loaded", value: "0" },
            { id: "tx-estimate", label: "Estimate", value: "—" },
            { id: "tx-success", label: "Success", value: "0", variant: "secondary" },
            { id: "tx-failed", label: "Failed", value: "0", variant: "success" },
          ],
          search: {
            enabled: true,
            placeholder: "Search by signature…",
            onChange: (value) => {
              state.signature = (value || "").trim();
            },
            onSubmit: () => {
              requestReload("search", {
                silent: false,
                resetScroll: true,
              }).catch(() => {});
            },
          },
          filters: [
            {
              id: "type",
              label: "Type",
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
              onChange: (value) => {
                state.filters.type = value || "all";
                requestReload("filter", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
            {
              id: "direction",
              label: "Direction",
              defaultValue: state.filters.direction,
              autoApply: false,
              options: [
                { value: "all", label: "All Directions" },
                { value: "Incoming", label: "Incoming" },
                { value: "Outgoing", label: "Outgoing" },
                { value: "Internal", label: "Internal" },
                { value: "Unknown", label: "Unknown" },
              ],
              onChange: (value) => {
                state.filters.direction = value || "all";
                requestReload("filter", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
            {
              id: "status",
              label: "Status",
              defaultValue: state.filters.status,
              autoApply: false,
              options: [
                { value: "all", label: "All Statuses" },
                { value: "Pending", label: "Pending" },
                { value: "Confirmed", label: "Confirmed" },
                { value: "Finalized", label: "Finalized" },
                { value: "Failed", label: "Failed" },
              ],
              onChange: (value) => {
                state.filters.status = value || "all";
                requestReload("filter", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
          ],
          buttons: [
            {
              id: "refresh",
              label: "Refresh",
              variant: "primary",
              onClick: () => {
                requestReload("manual", {
                  silent: false,
                  preserveScroll: false,
                }).catch(() => {});
              },
            },
            {
              id: "reset",
              label: "Reset",
              onClick: () => resetFilters(),
            },
          ],
        },
      });

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
    },

    activate(ctx) {
      ctxRef = ctx;
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(
            () =>
              requestReload("poll", { silent: true, preserveScroll: true }),
            { label: "Transactions" }
          )
        );
      }
      poller.start();
      if ((table?.getData?.() ?? []).length === 0) {
        requestReload("initial", {
          silent: false,
          resetScroll: true,
        }).catch(() => {});
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
      ctxRef = null;
      state.filters = { ...DEFAULT_FILTERS };
      state.signature = "";
      state.totalEstimate = null;
    },
  };
}

registerPage("transactions", createLifecycle());
