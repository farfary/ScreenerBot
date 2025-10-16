import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";

const DEFAULT_FILTERS = {
  category: "all",
  severity: "all",
};

const PAGE_LIMIT = 200;

function formatSeverityBadge(value) {
  const key = (value || "").toLowerCase();
  const badges = {
    info: '<span class="badge">ℹ️ Info</span>',
    warn: '<span class="badge warning">⚠️ Warning</span>',
    warning: '<span class="badge warning">⚠️ Warning</span>',
    error: '<span class="badge error">❌ Error</span>',
    critical: '<span class="badge error">🔴 Critical</span>',
    debug: '<span class="badge secondary">🐞 Debug</span>',
  };
  return badges[key] || `<span class="badge">${value || "—"}</span>`;
}

function formatMint(mint) {
  if (!mint) {
    return "—";
  }
  const trimmed = mint.trim();
  if (!trimmed) {
    return "—";
  }
  const short = `${trimmed.slice(0, 4)}...${trimmed.slice(-4)}`;
  return `<span class="mono-text" title="${trimmed}">${short}</span>`;
}

function createLifecycle() {
  let table = null;
  let poller = null;
  let ctxRef = null;
  let inflightRequest = null;
  let abortController = null;
  let nextRefreshReason = "poll";
  let nextRefreshOptions = {};
  let currentFilters = { ...DEFAULT_FILTERS };
  let searchTerm = "";

  const getAbortController = () => {
    if (ctxRef && typeof ctxRef.createAbortController === "function") {
      return ctxRef.createAbortController();
    }
    if (typeof window !== "undefined" && window.AbortController) {
      return new window.AbortController();
    }
    return {
      abort() {},
      signal: undefined,
    };
  };

  const buildQueryParams = () => {
    const params = new URLSearchParams({ limit: String(PAGE_LIMIT) });
    if (currentFilters.category && currentFilters.category !== "all") {
      params.set("category", currentFilters.category);
    }
    if (currentFilters.severity && currentFilters.severity !== "all") {
      params.set("severity", currentFilters.severity);
    }
    if (searchTerm) {
      params.set("search", searchTerm);
    }
    return params;
  };

  const applyDataToTable = (data) => {
    if (!table) {
      return;
    }
    const events = Array.isArray(data?.events) ? data.events : [];
    table.setData(events);

    const total = Number.isFinite(data?.count) ? data.count : events.length;
    table.updateToolbarSummary([
      {
        id: "events-total",
        label: "Total",
        value: Utils.formatNumber(total),
      },
    ]);

    const timestamp = data?.timestamp || new Date().toISOString();
    table.updateToolbarMeta([
      {
        id: "events-last-update",
        text: `Last update ${new Date(timestamp).toLocaleTimeString()}`,
      },
    ]);
  };

  const fetchEvents = async (reason = "poll", options = {}) => {
    const { force = false, showToast = false } = options;

    if (inflightRequest) {
      if (!force && reason === "poll") {
        return inflightRequest;
      }
      if (abortController) {
        try {
          abortController.abort();
        } catch (error) {
          console.warn("[Events] Failed to abort in-flight request", error);
        }
        abortController = null;
      }
    }

    const controller = getAbortController();
    abortController = controller;

    const request = (async () => {
      try {
        const params = buildQueryParams();
        const response = await fetch(`/api/events/head?${params.toString()}`, {
          headers: { "X-Requested-With": "fetch" },
          cache: "no-store",
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const data = await response.json();
        applyDataToTable(data);
        return data;
      } catch (error) {
        if (error?.name === "AbortError") {
          return null;
        }
        console.error("[Events] Failed to fetch:", error);
        if (showToast) {
          Utils.showToast("⚠️ Failed to refresh events", "warning");
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
    if (table) {
      return table.refresh();
    }
    return Promise.resolve(null);
  };

  const resetFilters = () => {
    currentFilters = { ...DEFAULT_FILTERS };
    searchTerm = "";
    if (table) {
      table.setToolbarFilterValue("category", DEFAULT_FILTERS.category, {
        apply: false,
      });
      table.setToolbarFilterValue("severity", DEFAULT_FILTERS.severity, {
        apply: false,
      });
      table.setToolbarSearchValue("", { apply: false });
    }
    triggerRefresh("reset", { force: true, showToast: true }).catch(() => {});
  };

  return {
    init(ctx) {
      ctxRef = ctx;

      const columns = [
        {
          id: "event_time",
          label: "Time",
          minWidth: 150,
          sortable: true,
          render: (value) => {
            if (!value) {
              return "—";
            }
            const date = new Date(value);
            return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`;
          },
        },
        {
          id: "category",
          label: "Category",
          minWidth: 110,
          sortable: true,
          render: (value) => value || "—",
        },
        {
          id: "subtype",
          label: "Type",
          minWidth: 120,
          sortable: true,
          render: (value) => value || "—",
        },
        {
          id: "severity",
          label: "Severity",
          minWidth: 110,
          sortable: true,
          render: (value) => formatSeverityBadge(value),
        },
        {
          id: "message",
          label: "Message",
          minWidth: 320,
          render: (value) => value || "—",
        },
        {
          id: "mint",
          label: "Token",
          minWidth: 140,
          render: (value) => formatMint(value),
        },
        {
          id: "payload",
          label: "Details",
          minWidth: 200,
          render: (value) => {
            if (!value || typeof value !== "object") {
              return "—";
            }
            const keys = Object.keys(value);
            if (!keys.length) {
              return "—";
            }
            const snippet = JSON.stringify(value);
            const preview = snippet.length > 120 ? `${snippet.slice(0, 120)}…` : snippet;
            return `<code style="font-size:0.85em;">${Utils.escapeHtml(preview)}</code>`;
          },
        },
      ];

      table = new DataTable({
        container: "#events-root",
        columns,
        rowIdField: "id",
        stateKey: "events-table",
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        autoSizeColumns: true,
        sorting: {
          defaultColumn: "event_time",
          defaultDirection: "desc",
        },
        toolbar: {
          title: {
            icon: "📡",
            text: "Events",
            meta: [{ id: "events-last-update", text: "Last update —" }],
          },
          summary: [{ id: "events-total", label: "Total", value: "0" }],
          search: {
            enabled: true,
            placeholder: "Search events...",
            onChange: (value) => {
              searchTerm = (value || "").trim();
            },
            onSubmit: () => {
              triggerRefresh("search", { force: true }).catch(() => {});
            },
          },
          filters: [
            {
              id: "category",
              label: "Category",
              defaultValue: DEFAULT_FILTERS.category,
              autoApply: false,
              filterFn: () => true,
              options: [
                { value: "all", label: "All Categories" },
                { value: "swap", label: "Swap" },
                { value: "transaction", label: "Transaction" },
                { value: "pool", label: "Pool" },
                { value: "position", label: "Position" },
                { value: "token", label: "Token" },
                { value: "wallet", label: "Wallet" },
                { value: "entry", label: "Entry" },
                { value: "system", label: "System" },
                { value: "ohlcv", label: "OHLCV" },
                { value: "rpc", label: "RPC" },
                { value: "security", label: "Security" },
                { value: "learner", label: "Learner" },
                { value: "other", label: "Other" },
              ],
              onChange: (value) => {
                currentFilters.category = value === "all" ? "all" : value;
                triggerRefresh("filters", { force: true }).catch(() => {});
              },
            },
            {
              id: "severity",
              label: "Severity",
              defaultValue: DEFAULT_FILTERS.severity,
              autoApply: false,
              filterFn: () => true,
              options: [
                { value: "all", label: "All Severities" },
                { value: "info", label: "Info" },
                { value: "warn", label: "Warning" },
                { value: "error", label: "Error" },
                { value: "debug", label: "Debug" },
              ],
              onChange: (value) => {
                currentFilters.severity = value === "all" ? "all" : value;
                triggerRefresh("filters", { force: true }).catch(() => {});
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
          return fetchEvents(reason, options);
        },
      });

      window.eventsTable = table;
      table.setToolbarFilterValue("category", DEFAULT_FILTERS.category, {
        apply: false,
      });
      table.setToolbarFilterValue("severity", DEFAULT_FILTERS.severity, {
        apply: false,
      });
      table.setToolbarSearchValue("", { apply: false });
    },

    activate(ctx) {
      ctxRef = ctx;

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => triggerRefresh("poll"), { label: "Events" })
        );
      }

      poller.start();
      triggerRefresh("initial", { force: true }).catch(() => {});
    },

    deactivate() {
      if (abortController) {
        try {
          abortController.abort();
        } catch (error) {
          console.warn("[Events] Failed to abort request on deactivate", error);
        }
        abortController = null;
      }
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
      if (abortController) {
        try {
          abortController.abort();
        } catch (error) {
          console.warn("[Events] Failed to abort request on dispose", error);
        }
        abortController = null;
      }
      inflightRequest = null;
      ctxRef = null;
      nextRefreshReason = "poll";
      nextRefreshOptions = {};
      currentFilters = { ...DEFAULT_FILTERS };
      searchTerm = "";
      window.eventsTable = null;
    },
  };
}

registerPage("events", createLifecycle());
