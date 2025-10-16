import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";

function createLifecycle() {
  let table = null;
  let poller = null;
  let currentFilters = {
    category: null,
    severity: null,
  };

  const fetchEvents = async () => {
    try {
      const params = new URLSearchParams({ limit: "500" });
      if (currentFilters.category) {
        params.append("category", currentFilters.category);
      }
      if (currentFilters.severity) {
        params.append("severity", currentFilters.severity);
      }

      const response = await fetch(`/api/events/head?${params.toString()}`, {
        headers: { "X-Requested-With": "fetch" },
        cache: "no-store",
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      return data;
    } catch (error) {
      console.error("[Events] Failed to fetch:", error);
      return null;
    }
  };

  const loadEvents = async () => {
    const data = await fetchEvents();
    if (!data || !table) return;

    const events = Array.isArray(data.events) ? data.events : [];
    table.setData(events);

    // Update toolbar summary
    if (data.count !== undefined) {
      table.updateToolbarSummary([
        {
          id: "events-total",
          label: "Total",
          value: Utils.formatNumber(data.count),
        },
      ]);
    }

    // Update last update time
    table.updateToolbarMeta([
      {
        id: "events-last-update",
        text: `Last update ${new Date().toLocaleTimeString()}`,
      },
    ]);
  };

  return {
    init(_ctx) {
      // Define columns
      const columns = [
        {
          id: "event_time",
          label: "Time",
          minWidth: 140,
          sortable: true,
          render: (val) => {
            if (!val) return "—";
            const date = new Date(val);
            return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`;
          },
        },
        {
          id: "category",
          label: "Category",
          minWidth: 100,
          sortable: true,
          render: (val) => val || "—",
        },
        {
          id: "subtype",
          label: "Type",
          minWidth: 100,
          sortable: true,
          render: (val) => val || "—",
        },
        {
          id: "severity",
          label: "Severity",
          minWidth: 90,
          sortable: true,
          render: (val) => {
            const badges = {
              info: '<span class="badge">ℹ️ Info</span>',
              warning: '<span class="badge warning">⚠️ Warning</span>',
              error: '<span class="badge error">❌ Error</span>',
              critical: '<span class="badge error">🔴 Critical</span>',
            };
            return badges[val?.toLowerCase()] || `<span class="badge">${val || "—"}</span>`;
          },
        },
        {
          id: "message",
          label: "Message",
          minWidth: 300,
          render: (val) => val || "—",
        },
        {
          id: "mint",
          label: "Token",
          minWidth: 120,
          render: (val) => {
            if (!val) return "—";
            const short = `${val.substring(0, 4)}...${val.substring(val.length - 4)}`;
            return `<span class="mono-text" title="${val}">${short}</span>`;
          },
        },
        {
          id: "payload",
          label: "Details",
          minWidth: 150,
          render: (val) => {
            if (!val || typeof val !== "object" || Object.keys(val).length === 0) return "—";
            const str = JSON.stringify(val);
            return `<code style="font-size: 0.85em;">${Utils.escapeHtml(str.substring(0, 100))}${str.length > 100 ? "..." : ""}</code>`;
          },
        },
      ];

      // Create DataTable
      table = new DataTable({
        container: "#events-root",
        columns,
        rowIdField: "id",
        emptyMessage: "No events found",
        loadingMessage: "Loading events...",
        stateKey: "events-table",
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        autoSizeColumns: true,
        sorting: {
          column: "timestamp",
          direction: "desc",
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
          },
          filters: [
            {
              id: "category",
              label: "Category",
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
              ],
              onChange: (value) => {
                currentFilters.category = value === "all" ? null : value;
                loadEvents().catch(() => {});
              },
            },
            {
              id: "severity",
              label: "Severity",
              options: [
                { value: "all", label: "All Severities" },
                { value: "info", label: "Info" },
                { value: "warn", label: "Warning" },
                { value: "error", label: "Error" },
                { value: "debug", label: "Debug" },
              ],
              onChange: (value) => {
                currentFilters.severity = value === "all" ? null : value;
                loadEvents().catch(() => {});
              },
            },
          ],
          buttons: [
            {
              id: "refresh",
              label: "Refresh",
              variant: "primary",
              onClick: () => {
                loadEvents().catch(() => {});
              },
            },
          ],
        },
      });

      window.eventsTable = table;
    },

    activate(ctx) {
      // Initial load
      loadEvents();

      // Start poller
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => loadEvents(), { label: "Events" })
        );
      }
      poller.start();
    },

    deactivate() {
      // Poller auto-paused by lifecycle context
    },

    dispose() {
      if (table) {
        table.destroy();
        table = null;
      }
      poller = null;
      window.eventsTable = null;
    },
  };
}

registerPage("events", createLifecycle());
