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
  let ctxRef = null;
  let inflightRequest = null;
  let abortController = null;
  let nextRefreshReason = "poll";
  let nextRefreshOptions = {};
  let nextCursor = null;
  let loadedRows = [];
  let filterSelections = { ...DEFAULT_FILTERS };
  let signatureQuery = "";
  let scrollContainer = null;
  let scrollListener = null;
  let scrollFrame = null;

  const getAbortController = () => {
    if (ctxRef && typeof ctxRef.createAbortController === "function") {
      return ctxRef.createAbortController();
    }
    if (typeof window !== "undefined" && window.AbortController) {
      return new window.AbortController();
    }
    return { abort() {}, signal: undefined };
  };

  const buildFiltersPayload = () => {
    const filters = {};
    const typeValue = filterSelections.type;
    const directionValue = filterSelections.direction;
    const statusValue = filterSelections.status;

    if (signatureQuery) {
      filters.signature = signatureQuery;
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

  const updateToolbar = (responseMeta) => {
    if (!table) {
      return;
    }

    const totalEstimate = Number.isFinite(responseMeta?.total_estimate)
      ? responseMeta.total_estimate
      : null;
    const loaded = loadedRows.length;
    const successCount = loadedRows.reduce(
      (acc, row) => (row?.success ? acc + 1 : acc),
      0
    );
    const failedCount = loadedRows.reduce((acc, row) => {
      if (row?.success === false || row?.status === "Failed") {
        return acc + 1;
      }
      return acc;
    }, 0);

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
          totalEstimate === null || totalEstimate === undefined
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

  const detachScrollListener = () => {
    if (scrollContainer && scrollListener) {
      scrollContainer.removeEventListener("scroll", scrollListener);
    }
    if (scrollFrame !== null && typeof window !== "undefined") {
      window.cancelAnimationFrame(scrollFrame);
    }
    scrollContainer = null;
    scrollListener = null;
    scrollFrame = null;
  };

  const ensureScrollListener = () => {
    detachScrollListener();
    const container = document.querySelector(
      "#transactions-root .data-table-scroll-container"
    );
    if (!container) {
      return;
    }
    scrollContainer = container;
    scrollListener = () => {
      if (scrollFrame !== null) {
        return;
      }
      scrollFrame = window.requestAnimationFrame(() => {
        scrollFrame = null;
        if (!scrollContainer || !nextCursor || inflightRequest) {
          return;
        }
        const remaining =
          scrollContainer.scrollHeight -
          scrollContainer.scrollTop -
          scrollContainer.clientHeight;
        if (remaining < 320) {
          fetchTransactions("append", { append: true }).catch(() => {});
        }
      });
    };
    scrollContainer.addEventListener("scroll", scrollListener, {
      passive: true,
    });
  };

  const fetchTransactions = async (reason = "poll", options = {}) => {
    const { force = false, append = false, showToast = false } = options;

    if (append && !nextCursor) {
      return Promise.resolve(null);
    }

    if (inflightRequest) {
      if (append || (!force && reason === "poll")) {
        return inflightRequest;
      }
      if (abortController) {
        try {
          abortController.abort();
        } catch (error) {
          console.warn("[Transactions] abort failed", error);
        }
        abortController = null;
      }
    }

    const controller = getAbortController();
    abortController = controller;

    const request = (async () => {
      try {
        const payload = buildRequestPayload(append ? nextCursor : null);
        const response = await fetch("/api/transactions/list", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Requested-With": "fetch",
          },
          body: JSON.stringify(payload),
          cache: "no-store",
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const data = await response.json();
        const items = Array.isArray(data?.items) ? data.items : [];

        if (append) {
          loadedRows = loadedRows.concat(items);
        } else {
          loadedRows = items;
        }

        nextCursor = data?.next_cursor ?? null;

        if (table) {
          table.setData(loadedRows);
          updateToolbar({ total_estimate: data?.total_estimate });
          if (!append && scrollContainer) {
            scrollContainer.scrollTop = 0;
          }
        }

        return data;
      } catch (error) {
        if (error?.name === "AbortError") {
          return null;
        }
        console.error("[Transactions] Failed to fetch:", error);
        if (showToast) {
          Utils.showToast("⚠️ Failed to refresh transactions", "warning");
        }
        throw error;
      } finally {
        if (abortController === controller) {
          abortController = null;
        }
        inflightRequest = null;
      }
    })();

    inflightRequest = request;
    return request;
  };

  const triggerRefresh = (reason = "poll", options = {}) => {
    nextRefreshReason = reason;
    nextRefreshOptions = options;
    if (!options.append) {
      nextCursor = null;
    }
    if (table) {
      return table.refresh();
    }
    return Promise.resolve(null);
  };

  const resetFilters = () => {
    filterSelections = { ...DEFAULT_FILTERS };
    signatureQuery = "";
    if (table) {
      table.setToolbarSearchValue("", { apply: false });
      table.setToolbarFilterValue("type", filterSelections.type, {
        apply: false,
      });
      table.setToolbarFilterValue("direction", filterSelections.direction, {
        apply: false,
      });
      table.setToolbarFilterValue("status", filterSelections.status, {
        apply: false,
      });
    }
    triggerRefresh("reset", { force: true, showToast: true }).catch(() => {});
  };

  return {
    init(ctx) {
      ctxRef = ctx;

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
        stateKey: "transactions-table",
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        sorting: {
          defaultColumn: "timestamp",
          defaultDirection: "desc",
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
              signatureQuery = (value || "").trim();
            },
            onSubmit: () => {
              triggerRefresh("search", { force: true }).catch(() => {});
            },
          },
          filters: [
            {
              id: "type",
              label: "Type",
              defaultValue: filterSelections.type,
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
                filterSelections.type = value || "all";
                triggerRefresh("filter", { force: true }).catch(() => {});
              },
            },
            {
              id: "direction",
              label: "Direction",
              defaultValue: filterSelections.direction,
              autoApply: false,
              options: [
                { value: "all", label: "All Directions" },
                { value: "Incoming", label: "Incoming" },
                { value: "Outgoing", label: "Outgoing" },
                { value: "Internal", label: "Internal" },
                { value: "Unknown", label: "Unknown" },
              ],
              onChange: (value) => {
                filterSelections.direction = value || "all";
                triggerRefresh("filter", { force: true }).catch(() => {});
              },
            },
            {
              id: "status",
              label: "Status",
              defaultValue: filterSelections.status,
              autoApply: false,
              options: [
                { value: "all", label: "All Statuses" },
                { value: "Pending", label: "Pending" },
                { value: "Confirmed", label: "Confirmed" },
                { value: "Finalized", label: "Finalized" },
                { value: "Failed", label: "Failed" },
              ],
              onChange: (value) => {
                filterSelections.status = value || "all";
                triggerRefresh("filter", { force: true }).catch(() => {});
              },
            },
          ],
          buttons: [
            {
              id: "refresh",
              label: "Refresh",
              variant: "primary",
              onClick: () => {
                triggerRefresh("manual", { force: true, showToast: true }).catch(
                  () => {}
                );
              },
            },
            {
              id: "reset",
              label: "Reset",
              onClick: () => resetFilters(),
            },
          ],
        },
        onRefresh: () => {
          const reason = nextRefreshReason;
          const options = nextRefreshOptions;
          nextRefreshReason = "poll";
          nextRefreshOptions = {};
          return fetchTransactions(reason, options);
        },
      });

      table.setToolbarSearchValue(signatureQuery, { apply: false });
      table.setToolbarFilterValue("type", filterSelections.type, {
        apply: false,
      });
      table.setToolbarFilterValue("direction", filterSelections.direction, {
        apply: false,
      });
      table.setToolbarFilterValue("status", filterSelections.status, {
        apply: false,
      });

      if (typeof window !== "undefined") {
        window.requestAnimationFrame(() => ensureScrollListener());
      }
    },

    activate(ctx) {
      ctxRef = ctx;
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => triggerRefresh("poll"), { label: "Transactions" })
        );
      }
      poller.start();
      triggerRefresh("initial", { force: true, showToast: true }).catch(() => {});
    },

    deactivate() {
      if (abortController) {
        try {
          abortController.abort();
        } catch (error) {
          console.warn("[Transactions] abort on deactivate failed", error);
        }
        abortController = null;
      }
      inflightRequest = null;
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
      detachScrollListener();
      ctxRef = null;
      inflightRequest = null;
      loadedRows = [];
      nextCursor = null;
      if (abortController) {
        try {
          abortController.abort();
        } catch (error) {
          console.warn("[Transactions] abort on dispose failed", error);
        }
        abortController = null;
      }
    },
  };
}

registerPage("transactions", createLifecycle());
