import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { DataTable } from "../ui/data_table.js";
import { EventDetailsDialog } from "../ui/events_dialog.js";
import { requestManager } from "../core/request_manager.js";

const DEFAULT_FILTERS = {
  category: "all",
  severity: "all",
};

const PAGE_LIMIT = 100;

function formatSeverityBadge(value) {
  const key = (value || "").toLowerCase();
  const badges = {
    info: '<span class="badge"><i class="icon-info"></i> Info</span>',
    warn: '<span class="badge warning"><i class="icon-triangle-alert"></i> Warning</span>',
    warning: '<span class="badge warning"><i class="icon-triangle-alert"></i> Warning</span>',
    error: '<span class="badge error"><i class="icon-x"></i> Error</span>',
    critical: '<span class="badge error"><i class="icon-circle-alert"></i> Critical</span>',
    debug: '<span class="badge secondary"><i class="icon-bug"></i> Debug</span>',
  };
  if (badges[key]) {
    return badges[key];
  }
  const label = value ? Utils.escapeHtml(String(value)) : "—";
  return `<span class="badge">${label}</span>`;
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
  return `<span class="mono-text" title="${Utils.escapeHtml(trimmed)}">${Utils.escapeHtml(
    short
  )}</span>`;
}

function formatMessagePreview(value) {
  if (!value) {
    return "—";
  }
  const text = String(value);
  const preview = text.length > 160 ? `${text.slice(0, 160)}...` : text;
  return `<span title="${Utils.escapeHtml(text)}">${Utils.escapeHtml(preview)}</span>`;
}

function formatPayloadPreview(value) {
  if (!value || typeof value !== "object") {
    return "—";
  }

  const entries = Array.isArray(value)
    ? value.map((item, index) => [index, item])
    : Object.entries(value);
  if (entries.length === 0) {
    return "—";
  }

  const previewParts = entries.slice(0, 3).map(([key, raw]) => {
    const val =
      raw === null || raw === undefined ? "null" : typeof raw === "object" ? "{...}" : String(raw);
    const safeKey = Utils.escapeHtml(String(key));
    const safeVal = Utils.escapeHtml(val.length > 32 ? `${val.slice(0, 32)}...` : val);
    return `${safeKey}: ${safeVal}`;
  });

  const remaining = entries.length - previewParts.length;
  const preview = previewParts.join(", ") + (remaining > 0 ? `, +${remaining} more` : "");
  const fullJson = Utils.escapeHtml(JSON.stringify(value, null, 2));

  return `<span class="mono-text" title="${fullJson}">${preview}</span>`;
}

function createLifecycle() {
  let table = null;
  let poller = null;
  let ctxRef = null;
  let detailsDialog = null;

  const state = {
    filters: { ...DEFAULT_FILTERS },
    search: "",
    totalCount: null,
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
    const total = state.totalCount !== null ? state.totalCount : rows.length;
    table.updateToolbarSummary([
      {
        id: "events-total",
        label: "Total",
        value: Utils.formatNumber(total, 0),
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
        .map((id) => String(id))
    );

    const fetchJson = async (url) => {
      return await requestManager.fetch(url, {
        headers: { "X-Requested-With": "fetch" },
        cache: "no-store",
        priority: "normal",
      });
    };

    const normaliseEvents = (events, order = "desc", dedupeExisting = true) => {
      const list = Array.isArray(events) ? [...events] : [];
      if (order === "asc") {
        list.sort((a, b) => (b?.id ?? 0) - (a?.id ?? 0));
      }
      const deduped = [];
      const seenIds = new Set();
      for (const event of list) {
        const id = event?.id;
        if (id === undefined || id === null) {
          continue;
        }
        const key = String(id);
        if (seenIds.has(key)) {
          continue;
        }
        if (dedupeExisting && existingIds.has(key)) {
          continue;
        }
        seenIds.add(key);
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
    const fresh = normaliseEvents(data?.events, "desc", false);
    const cursorPrev = fresh.length > 0 ? fresh[0].id : (cursor ?? null);
    const cursorNext = fresh.length > 0 ? fresh[fresh.length - 1].id : (cursor ?? null);

    // Store total count from API response
    if (typeof data?.total_count === "number") {
      state.totalCount = data.total_count;
    }

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

      if (!detailsDialog) {
        detailsDialog = new EventDetailsDialog();
      }

      const columns = [
        {
          id: "event_time",
          label: "Time",
          minWidth: 165,
          sortable: true,
          wrap: false,
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
          wrap: false,
          render: (value) => value || "—",
        },
        {
          id: "subtype",
          label: "Type",
          minWidth: 120,
          sortable: true,
          wrap: false,
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
          wrap: false,
          render: (value) => formatMessagePreview(value),
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
          wrap: false,
          render: (value) => formatPayloadPreview(value),
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
        autoSizeColumns: false,
        uniformRowHeight: 2, // enforce consistent row height (2 lines)
        onRowClick: (row, event) => {
          if (!row) {
            return;
          }
          if (event?.defaultPrevented) {
            return;
          }
          detailsDialog?.open(row);
        },
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
            icon: "icon-radio",
            text: "Events",
            meta: [{ id: "events-last-update", text: "Last update —" }],
          },
          summary: [{ id: "events-total", label: "Total", value: "0" }],
          search: {
            enabled: true,
            mode: "server",
            placeholder: "Search events...",
            onChange: (value, el, options) => {
              state.search = (value || "").trim();
              // Skip if this is state restoration
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
          filters: [
            {
              id: "category",
              label: "Category",
              mode: "server",
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
                { value: "connectivity", label: "Connectivity" },
                { value: "learner", label: "Learner" },
                { value: "other", label: "Other" },
              ],
              onChange: (value, el, options) => {
                state.filters.category = value === "all" ? "all" : value;
                // Skip reload if this is state restoration
                if (options?.restored) {
                  return;
                }
                requestReload("filters", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
            {
              id: "severity",
              label: "Severity",
              mode: "server",
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
              onChange: (value, el, options) => {
                state.filters.severity = value === "all" ? "all" : value;
                // Skip reload if this is state restoration
                if (options?.restored) {
                  return;
                }
                requestReload("filters", {
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
              onClick: () => resetFilters(),
            },
          ],
        },
      });

      // Sync state from DataTable's restored server state
      const serverState = table.getServerState();
      if (serverState.searchQuery) {
        state.search = serverState.searchQuery;
      }
      if (serverState.filters.category) {
        state.filters.category = serverState.filters.category;
      }
      if (serverState.filters.severity) {
        state.filters.severity = serverState.filters.severity;
      }

      window.eventsTable = table;
      table.setToolbarFilterValue("category", state.filters.category, {
        apply: false,
      });
      table.setToolbarFilterValue("severity", state.filters.severity, {
        apply: false,
      });
      table.setToolbarSearchValue(state.search, { apply: false });
      updateToolbar();
    },

    activate(ctx) {
      ctxRef = ctx;

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => requestReload("poll", { silent: true, preserveScroll: true }), {
            label: "Events",
          })
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
      detailsDialog?.close();
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
      if (detailsDialog) {
        detailsDialog.destroy();
        detailsDialog = null;
      }
      ctxRef = null;
      state.filters = { ...DEFAULT_FILTERS };
      state.search = "";
      window.eventsTable = null;
    },
  };
}

registerPage("events", createLifecycle());
