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

  const state = {
    filters: { ...DEFAULT_FILTERS },
    search: "",
  };

  const buildBaseParams = (limit = PAGE_LIMIT) => {
    const params = new URLSearchParams({ limit: String(limit) });
    if (state.filters.category && state.filters.category !== "all") {
      params.set("category", state.filters.category);
    }
    if (state.filters.severity && state.filters.severity !== "all") {
      params.set("severity", state.filters.severity);
    }
    if (state.search) {
      params.set("search", state.search);
    }
    return params;
  };

  const updateToolbar = () => {
    if (!table) {
      return;
    }

    const rows = table.getData();
    const total = rows.length;
    table.updateToolbarSummary([
      {
        id: "events-total",
        label: "Total",
        value: Utils.formatNumber(total),
      },
    ]);

    table.updateToolbarMeta([
      {
        id: "events-last-update",
        text: `Last update ${new Date().toLocaleTimeString()}`,
      },
    ]);
  };

  const loadEventsPage = async ({ direction, cursor, reason, signal }) => {
    const existingRows = table?.getData?.() ?? [];
    const existingIds = new Set(
      existingRows
        .map((row) => row?.id)
        .filter((id) => typeof id === "number" || typeof id === "string")
    );

    const fetchJson = async (url) => {
      const response = await fetch(url, {
        headers: { "X-Requested-With": "fetch" },
        cache: "no-store",
        signal,
      });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      return response.json();
    };

    const normaliseEvents = (events, order = "desc") => {
      const list = Array.isArray(events) ? [...events] : [];
      if (order === "asc") {
        list.sort((a, b) => (b?.id ?? 0) - (a?.id ?? 0));
      }
      const deduped = [];
      for (const event of list) {
        const id = event?.id;
        if (id === undefined || id === null) {
          continue;
        }
        if (existingIds.has(id)) {
          continue;
        }
        existingIds.add(id);
        deduped.push(event);
      }
      return deduped;
    };

    if (direction === "prev") {
      const maxId = cursor ?? existingRows[0]?.id ?? null;
      if (!maxId) {
        return loadEventsPage({ direction: "initial", cursor: null, reason, signal });
      }

      const params = buildBaseParams();
      params.set("after_id", String(maxId));
      const data = await fetchJson(`/api/events/since?${params.toString()}`);

      const fresh = normaliseEvents(data?.events, "asc");
      const nextCursor = fresh.length > 0 ? fresh[0].id : maxId;
      const hasMorePrev = fresh.length > 0 && (data?.events?.length ?? 0) >= PAGE_LIMIT;

      return {
        rows: fresh,
        cursorPrev: nextCursor ?? maxId,
        hasMorePrev,
      };
    }

    if (direction === "next") {
      const minId = cursor ?? existingRows[existingRows.length - 1]?.id ?? null;
      if (!minId) {
        return { rows: [], hasMoreNext: false };
      }
      const params = buildBaseParams();
      params.set("before_id", String(minId));
      const data = await fetchJson(`/api/events/before?${params.toString()}`);
      const fresh = normaliseEvents(data?.events, "desc");
      const nextCursor = fresh.length > 0 ? fresh[fresh.length - 1].id : null;

      return {
        rows: fresh,
        cursorNext: nextCursor,
        hasMoreNext: (data?.events?.length ?? 0) >= PAGE_LIMIT,
      };
    }

    const params = buildBaseParams();
    const data = await fetchJson(`/api/events/head?${params.toString()}`);
    const fresh = normaliseEvents(data?.events, "desc");
    const cursorPrev = fresh.length > 0 ? fresh[0].id : cursor ?? null;
    const cursorNext =
      fresh.length > 0 ? fresh[fresh.length - 1].id : cursor ?? null;

    return {
      rows: fresh,
      cursorPrev,
      cursorNext,
      hasMoreNext: (data?.events?.length ?? 0) >= PAGE_LIMIT,
      hasMorePrev: true,
    };
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
    state.search = "";
    if (table) {
      table.setToolbarFilterValue("category", DEFAULT_FILTERS.category, {
        apply: false,
      });
      table.setToolbarFilterValue("severity", DEFAULT_FILTERS.severity, {
        apply: false,
      });
      table.setToolbarSearchValue("", { apply: false });
    }
    requestReload("reset", { silent: false, resetScroll: true }).catch(() => {});
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
          column: "event_time",
          direction: "desc",
        },
        pagination: {
          threshold: 240,
          maxRows: 1500,
          loadPage: loadEventsPage,
          dedupeKey: (row) => row?.id ?? null,
          rowIdField: "id",
          onPageLoaded: handlePageLoaded,
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
              state.search = (value || "").trim();
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
                state.filters.category = value === "all" ? "all" : value;
                requestReload("filters", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
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
                state.filters.severity = value === "all" ? "all" : value;
                requestReload("filters", {
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

      window.eventsTable = table;
      table.setToolbarFilterValue("category", DEFAULT_FILTERS.category, {
        apply: false,
      });
      table.setToolbarFilterValue("severity", DEFAULT_FILTERS.severity, {
        apply: false,
      });
      table.setToolbarSearchValue("", { apply: false });
      updateToolbar();
    },

    activate(ctx) {
      ctxRef = ctx;

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(
            () =>
              requestReload("poll", { silent: true, preserveScroll: true }),
            { label: "Events" }
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
      state.search = "";
      window.eventsTable = null;
    },
  };
}

registerPage("events", createLifecycle());
