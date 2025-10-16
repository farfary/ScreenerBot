import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";

function createLifecycle() {
  let table = null;
  let poller = null;
  let ctxRef = null;
  let inflightRequest = null;
  let abortController = null;
  let nextRefreshReason = "poll";
  let nextRefreshOptions = {};
  let nextCursor = null;

  const getAbortController = () => {
    if (ctxRef && typeof ctxRef.createAbortController === "function") {
      return ctxRef.createAbortController();
    }
    if (typeof window !== "undefined" && window.AbortController) {
      return new window.AbortController();
    }
    return { abort() {}, signal: undefined };
  };

  const buildRequestPayload = (opts = {}) => {
    const payload = {
      limit: opts.limit || 50,
      cursor: opts.cursor || null,
      filters: opts.filters || {},
      search: opts.search || null,
    };
    return payload;
  };

  const fetchTransactions = async (reason = "poll", options = {}) => {
    const { force = false, append = false, showToast = false } = options;

    if (inflightRequest) {
      if (!force && reason === "poll") {
        return inflightRequest;
      }
      if (abortController) {
        try {
          abortController.abort();
        } catch (err) {
          console.warn("[Transactions] abort failed", err);
        }
        abortController = null;
      }
    }

    const controller = getAbortController();
    abortController = controller;

    const request = (async () => {
      try {
        const toolbarState = table ? table.getToolbarState() : {};
        const payload = buildRequestPayload({
          limit: toolbarState.limit || 50,
          cursor: append ? nextCursor : null,
          filters: toolbarState.filters || {},
          search: toolbarState.search || null,
        });

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

        if (table) {
          const items = Array.isArray(data.transactions) ? data.transactions : [];
          if (append) {
            table.appendData(items);
          } else {
            table.setData(items);
          }

          // Update next cursor for infinite scroll
          nextCursor = data.next_cursor || null;

          // Update toolbar summary/meta
          if (data.summary) {
            table.updateToolbarSummary([
              { id: "tx-total", label: "Total", value: Utils.formatNumber(data.summary.total || items.length) },
              { id: "tx-success", label: "Success", value: Utils.formatNumber(data.summary.success || 0), variant: "success" },
              { id: "tx-failed", label: "Failed", value: Utils.formatNumber(data.summary.failed || 0), variant: data.summary.failed > 0 ? "warning" : "success" },
            ]);

            table.updateToolbarMeta([
              { id: "tx-last-update", text: `Last update ${new Date().toLocaleTimeString()}` },
            ]);
          }
        }

        return data;
      } catch (error) {
        if (error?.name === "AbortError") return null;
        console.error("[Transactions] Failed to fetch:", error);
        if (showToast) Utils.showToast("⚠️ Failed to refresh transactions", "warning");
        throw error;
      } finally {
        if (abortController === controller) abortController = null;
        inflightRequest = null;
      }
    })();

    inflightRequest = request;
    return request;
  };

  const triggerRefresh = (reason = "poll", options = {}) => {
    nextRefreshReason = reason;
    nextRefreshOptions = options;
    if (table) return table.refresh();
    return Promise.resolve(null);
  };

  return {
    init(ctx) {
      console.log("[Transactions] Lifecycle init");
      ctxRef = ctx;

      const columns = [
        { id: "ts", label: "Time", minWidth: 140, sortable: true, render: (v, row) => Utils.formatDateTime(row.timestamp || row.created_at) },
        { id: "signature", label: "Signature", minWidth: 300, render: (v) => `<code class=\"mono\">${v || "-"}</code>` },
        { id: "type", label: "Type", minWidth: 120, render: (v) => v || "-" },
        { id: "wallet", label: "Wallet", minWidth: 200, render: (v) => v || "-" },
        { id: "amount", label: "Amount (SOL)", minWidth: 120, render: (v, row) => (typeof row.amount === 'number' ? row.amount.toFixed(6) : "-") },
        { id: "status", label: "Status", minWidth: 90, render: (v) => v || "-" },
        { id: "memo", label: "Memo", minWidth: 220, render: (v) => v || "-" },
      ];

      table = new DataTable({
        container: "#transactions-root",
        columns,
        stateKey: "transactions-table",
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        toolbar: {
          title: { icon: "💸", text: "Transactions", meta: [{ id: "tx-last-update", text: "Last update —" }] },
          summary: [
            { id: "tx-total", label: "Total", value: "0" },
            { id: "tx-success", label: "Success", value: "0", variant: "success" },
            { id: "tx-failed", label: "Failed", value: "0", variant: "warning" },
          ],
          search: { enabled: true, placeholder: "Search transactions (signature, wallet, memo)..." },
          filters: [
            { id: "status", label: "Status", options: [ { value: "all", label: "All" }, { value: "success", label: "Success" }, { value: "failed", label: "Failed" } ], autoApply: true },
            { id: "type", label: "Type", options: [ { value: "all", label: "All Types" } ], autoApply: false },
          ],
          buttons: [
            { id: "refresh", label: "Refresh", variant: "primary", onClick: () => triggerRefresh("manual", { force: true, showToast: true }).catch(() => {}) },
          ],
        },
        onRefresh: () => {
          const reason = nextRefreshReason;
          const options = nextRefreshOptions;
          nextRefreshReason = "poll";
          nextRefreshOptions = {};
          // reset cursor when not appending
          nextCursor = null;
          return fetchTransactions(reason, options);
        },
      });

      // wire search and filters to server-side behavior
      table.onToolbarSearchSubmit = (searchTerm) => {
        // user pressed enter or submitted search
        triggerRefresh("search", { force: true }).catch(() => {});
      };

      table.onToolbarFilterChange = (filterState) => {
        // any filter change should trigger a refresh (server-side filtering)
        triggerRefresh("filter", { force: true }).catch(() => {});
      };

      // infinite scroll handling: when near bottom, append more if nextCursor
      const scrollHandler = Utils.debounce(() => {
        try {
          const el = document.querySelector('#transactions-root .data-table-body');
          if (!el) return;
          const threshold = 300; // px
          if (el.scrollHeight - el.scrollTop - el.clientHeight < threshold) {
            if (nextCursor) {
              // append next page
              fetchTransactions('poll', { append: true }).catch(() => {});
            }
          }
        } catch (e) {
          // ignore
        }
      }, 200);

      // attach scroll listener
      setTimeout(() => {
        const el = document.querySelector('#transactions-root .data-table-body');
        if (el) {
          el.addEventListener('scroll', scrollHandler, { passive: true });
        }
      }, 200);
    },

    activate(ctx) {
      console.log('[Transactions] Lifecycle activate');
      ctxRef = ctx;
      if (!poller) {
        poller = ctx.managePoller(new Poller(() => triggerRefresh('poll'), { label: 'Transactions' }));
      }
      poller.start();
      // initial load
      triggerRefresh('initial', { force: true, showToast: true }).catch(() => {});
    },

    deactivate() {
      console.log('[Transactions] Lifecycle deactivate');
      if (abortController) {
        try { abortController.abort(); } catch (e) { console.warn('[Transactions] abort on deactivate failed', e); }
        abortController = null;
      }
      inflightRequest = null;
    },

    dispose() {
      console.log('[Transactions] Lifecycle dispose');
      if (table) { table.destroy(); table = null; }
      poller = null; ctxRef = null; inflightRequest = null; nextCursor = null;
      if (abortController) { try { abortController.abort(); } catch (e) {} abortController = null; }
    },
  };
}

registerPage('transactions', createLifecycle());
