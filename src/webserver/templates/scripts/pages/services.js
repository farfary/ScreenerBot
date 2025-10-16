import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";

// Helper functions
function healthRank(status) {
  const ranks = { healthy: 3, degraded: 2, starting: 1, unhealthy: 0 };
  return ranks[status] ?? -1;
}

function getHealthBadge(health) {
  const status = health?.status || "unknown";
  const badges = {
    healthy: '<span class="badge success">✅ Healthy</span>',
    starting: '<span class="badge warning">⏳ Starting</span>',
    degraded: '<span class="badge warning">⚠️ Degraded</span>',
    unhealthy: '<span class="badge error">❌ Unhealthy</span>',
  };
  return badges[status] || `<span class="badge secondary">⏸️ ${status}</span>`;
}

function getActivityBar(metrics) {
  const total =
    (metrics.total_poll_duration_ns || 0) +
    (metrics.total_idle_duration_ns || 0);
  const activity =
    total > 0 ? ((metrics.total_poll_duration_ns || 0) / total) * 100 : 0;
  const color =
    activity > 80
      ? "#10b981"
      : activity > 50
      ? "#3b82f6"
      : activity > 20
      ? "#f59e0b"
      : activity > 5
      ? "#6b7280"
      : "#9ca3af";

  return `
    <div class="activity-cell" title="${activity.toFixed(1)}% busy">
      <div class="activity-track">
        <div class="activity-fill" style="width:${activity.toFixed(
          1
        )}%; background:${color};"></div>
      </div>
      <div class="activity-meta">
        <span>${activity.toFixed(1)}%</span>
        <span>${metrics.total_polls || 0} polls</span>
      </div>
    </div>
  `;
}

function updateSummary(summary) {
  const total = $("#totalServices");
  const healthy = $("#healthyServices");
  const starting = $("#startingServices");
  const unhealthy = $("#unhealthyServices");
  if (total) total.textContent = summary?.total_services ?? "-";
  if (healthy) healthy.textContent = summary?.healthy_services ?? "-";
  if (starting) starting.textContent = summary?.starting_services ?? "-";
  if (unhealthy) {
    const degraded = summary?.degraded_services || 0;
    const unhealthyCount = summary?.unhealthy_services || 0;
    unhealthy.textContent = summary ? unhealthyCount + degraded : "-";
  }
}

function updateProcessMetrics(firstService) {
  const cpuEl = $("#processCpu");
  const memEl = $("#processMemory");

  if (!cpuEl && !memEl) {
    return;
  }

  const metrics = firstService?.metrics;
  if (!metrics) {
    if (cpuEl) cpuEl.textContent = "-";
    if (memEl) memEl.textContent = "-";
    return;
  }

  if (cpuEl) {
    const cpu = Number.isFinite(metrics.process_cpu_percent)
      ? `${metrics.process_cpu_percent.toFixed(1)}%`
      : "-";
    cpuEl.textContent = cpu;
  }

  if (memEl) {
    const mem = metrics.process_memory_bytes
      ? Utils.formatBytes(metrics.process_memory_bytes)
      : "-";
    memEl.textContent = mem;
  }
}

function createLifecycle() {
  let table = null;
  let poller = null;
  let ctxRef = null;
  let inflightRequest = null;
  let abortController = null;
  let nextRefreshReason = "poll";
  let nextRefreshOptions = {};

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

  const fetchServices = async (reason = "poll", options = {}) => {
    const { force = false, showToast = false } = options;

    if (inflightRequest) {
      if (!force && reason === "poll") {
        return inflightRequest;
      }
      if (abortController) {
        try {
          abortController.abort();
        } catch (error) {
          console.warn("[Services] Failed to abort in-flight request", error);
        }
        abortController = null;
      }
    }

    const controller = getAbortController();

    abortController = controller;

    const request = (async () => {
      try {
        const response = await fetch("/api/services/overview", {
          headers: { "X-Requested-With": "fetch" },
          cache: "no-store",
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(
            `HTTP ${response.status}: ${response.statusText}`
          );
        }

        const data = await response.json();

        updateSummary(data.summary);
        if (Array.isArray(data.services) && data.services.length > 0) {
          updateProcessMetrics(data.services[0]);
        } else {
          updateProcessMetrics(null);
        }

        if (table) {
          table.setData(Array.isArray(data.services) ? data.services : []);
          if (data.summary) {
            const healthy = data.summary.healthy_services || 0;
            const degraded = data.summary.degraded_services || 0;
            const unhealthy = data.summary.unhealthy_services || 0;
            const alerts = degraded + unhealthy;
            table.updateToolbarSummary([
              {
                id: "services-total",
                label: "Total",
                value: Utils.formatNumber(
                  data.summary.total_services ?? healthy + alerts
                ),
              },
              {
                id: "services-healthy",
                label: "Healthy",
                value: Utils.formatNumber(healthy),
                variant: "success",
              },
              {
                id: "services-alerts",
                label: "Alerts",
                value: Utils.formatNumber(alerts),
                variant: alerts > 0 ? "warning" : "success",
                tooltip: `${Utils.formatNumber(degraded)} degraded / ${Utils.formatNumber(unhealthy)} unhealthy`,
              },
            ]);
            table.updateToolbarMeta([
              {
                id: "services-last-update",
                text: `Last update ${new Date().toLocaleTimeString()}`,
              },
            ]);
          }
        }

        return data;
      } catch (error) {
        if (error?.name === "AbortError") {
          return null;
        }

        console.error("[Services] Failed to fetch:", error);
        if (showToast) {
          Utils.showToast("⚠️ Failed to refresh services", "warning");
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

  return {
    init(ctx) {
      console.log("[Services] Lifecycle init");
      ctxRef = ctx;

      // Define table columns with custom renderers
      const columns = [
        {
          id: "name",
          label: "Service",
          sortable: true,
          minWidth: 140,
          render: (v) => `<strong>${v || "-"}</strong>`,
        },
        {
          id: "health",
          label: "Health",
          sortable: true,
          minWidth: 120,
          render: (v, row) => getHealthBadge(row.health),
          sortFn: (a, b) =>
            healthRank(a.health?.status) - healthRank(b.health?.status),
        },
        {
          id: "priority",
          label: "Priority",
          sortable: true,
          minWidth: 72,
          render: (v) => v ?? "-",
        },
        {
          id: "enabled",
          label: "Enabled",
          sortable: true,
          minWidth: 72,
          render: (v) => (v ? "✅" : "❌"),
        },
        {
          id: "uptime",
          label: "Uptime",
          sortable: true,
          minWidth: 96,
          render: (v, row) =>
            Utils.formatUptime(row.uptime_seconds, { style: "compact" }),
          sortFn: (a, b) => (a.uptime_seconds || 0) - (b.uptime_seconds || 0),
        },
        {
          id: "activity",
          label: "Activity",
          sortable: true,
          minWidth: 200,
          render: (v, row) => getActivityBar(row.metrics || {}),
          sortFn: (a, b) => {
            const calcActivity = (metrics) => {
              const total =
                (metrics.total_poll_duration_ns || 0) +
                (metrics.total_idle_duration_ns || 0);
              return total > 0
                ? (metrics.total_poll_duration_ns || 0) / total
                : 0;
            };
            return (
              calcActivity(a.metrics || {}) - calcActivity(b.metrics || {})
            );
          },
        },
        {
          id: "lastCycle",
          label: "Last Cycle",
          sortable: true,
          minWidth: 96,
          render: (v, row) =>
            Utils.formatDuration(row.metrics?.last_cycle_duration_ns || 0),
          sortFn: (a, b) =>
            (a.metrics?.last_cycle_duration_ns || 0) -
            (b.metrics?.last_cycle_duration_ns || 0),
        },
        {
          id: "avgCycle",
          label: "Avg Cycle",
          sortable: true,
          minWidth: 96,
          render: (v, row) =>
            Utils.formatDuration(row.metrics?.avg_cycle_duration_ns || 0),
          sortFn: (a, b) =>
            (a.metrics?.avg_cycle_duration_ns || 0) -
            (b.metrics?.avg_cycle_duration_ns || 0),
        },
        {
          id: "avgPoll",
          label: "Avg Poll",
          sortable: true,
          minWidth: 96,
          render: (v, row) =>
            Utils.formatDuration(row.metrics?.mean_poll_duration_ns || 0),
          sortFn: (a, b) =>
            (a.metrics?.mean_poll_duration_ns || 0) -
            (b.metrics?.mean_poll_duration_ns || 0),
        },
        {
          id: "cycleRate",
          label: "Cycle Rate",
          sortable: true,
          minWidth: 90,
          render: (v, row) => {
            const rate = row.metrics?.cycles_per_second;
            return Number.isFinite(rate) ? rate.toFixed(2) : "0.00";
          },
          sortFn: (a, b) =>
            (a.metrics?.cycles_per_second || 0) -
            (b.metrics?.cycles_per_second || 0),
        },
        {
          id: "tasks",
          label: "Tasks",
          sortable: true,
          minWidth: 90,
          render: (v, row) => {
            const m = row.metrics || {};
            const taskInfo =
              m.task_count > 0
                ? `${m.task_count} tasks\nLast: ${Utils.formatDuration(
                    m.last_cycle_duration_ns
                  )}\nAvg: ${Utils.formatDuration(
                    m.avg_cycle_duration_ns
                  )}\nPoll: ${Utils.formatDuration(
                    m.mean_poll_duration_ns
                  )}\nIdle: ${Utils.formatDuration(
                    m.mean_idle_duration_ns
                  )}\nTotal Polls: ${m.total_polls || 0}`
                : "No instrumented tasks";
            return `<span title="${taskInfo}">${m.task_count || 0}</span>`;
          },
          sortFn: (a, b) =>
            (a.metrics?.task_count || 0) - (b.metrics?.task_count || 0),
        },
        {
          id: "ops",
          label: "Ops/sec",
          sortable: true,
          minWidth: 90,
          render: (v, row) =>
            (row.metrics?.operations_per_second || 0).toFixed(2),
          sortFn: (a, b) =>
            (a.metrics?.operations_per_second || 0) -
            (b.metrics?.operations_per_second || 0),
        },
        {
          id: "errors",
          label: "Errors",
          sortable: true,
          minWidth: 80,
          render: (v, row) => row.metrics?.errors_total || 0,
          sortFn: (a, b) =>
            (a.metrics?.errors_total || 0) - (b.metrics?.errors_total || 0),
        },
        {
          id: "dependencies",
          label: "Dependencies",
          sortable: false,
          minWidth: 160,
          render: (v, row) => {
            const deps = Array.isArray(row.dependencies)
              ? row.dependencies
              : [];
            return deps.length > 0
              ? deps
                  .map((dep) => `<span class="dependency-badge">${dep}</span>`)
                  .join(" ")
              : '<span class="detail-value">None</span>';
          },
        },
      ];

      // Create DataTable instance
      table = new DataTable({
        container: "#services-root",
        columns,
        stateKey: "services-table",
        enableLogging: false,
        sorting: {
          defaultColumn: "priority",
          defaultDirection: "asc",
        },
        compact: true, // Enable compact mode for denser display
        stickyHeader: true,
        zebra: true,
        fitToContainer: true, // Auto-fit columns to container width
        toolbar: {
          title: {
            icon: "🔧",
            text: "Services",
            meta: [{ id: "services-last-update", text: "Last update —" }],
          },
          summary: [
            { id: "services-total", label: "Total", value: "0" },
            {
              id: "services-healthy",
              label: "Healthy",
              value: "0",
              variant: "success",
            },
            {
              id: "services-alerts",
              label: "Alerts",
              value: "0",
              variant: "warning",
            },
          ],
          search: {
            enabled: true,
            placeholder: "Search services...",
          },
          filters: [
            {
              id: "status",
              label: "Status",
              options: [
                { value: "all", label: "All Statuses" },
                { value: "healthy", label: "Healthy" },
                { value: "starting", label: "Starting" },
                { value: "degraded", label: "Degraded" },
                { value: "unhealthy", label: "Unhealthy" },
              ],
              filterFn: (row, value) =>
                value === "all" || row.health?.status === value,
            },
            {
              id: "enabled",
              label: "Enabled",
              options: [
                { value: "all", label: "All Services" },
                { value: "enabled", label: "Enabled Only" },
                { value: "disabled", label: "Disabled Only" },
              ],
              filterFn: (row, value) => {
                if (value === "all") return true;
                if (value === "enabled") return !!row.enabled;
                if (value === "disabled") return !row.enabled;
                return true;
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
          ],
        },
        onRefresh: () => {
          const reason = nextRefreshReason;
          const options = nextRefreshOptions;
          nextRefreshReason = "poll";
          nextRefreshOptions = {};
          return fetchServices(reason, options);
        },
      });
    },

    activate(ctx) {
      console.log("[Services] Lifecycle activate");
      ctxRef = ctx;

      // Create and start poller
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => triggerRefresh("poll"), { label: "Services" })
        );
      }

      poller.start();

      // Initial data load
      triggerRefresh("initial", { force: true, showToast: true }).catch(
        () => {}
      );
    },

    deactivate() {
      console.log("[Services] Lifecycle deactivate");
      // Poller auto-paused by lifecycle context
      if (abortController) {
        try {
          abortController.abort();
        } catch (error) {
          console.warn("[Services] Failed to abort request on deactivate", error);
        }
        abortController = null;
      }
      inflightRequest = null;
    },

    dispose() {
      console.log("[Services] Lifecycle dispose");
      if (table) {
        table.destroy();
        table = null;
      }
      poller = null;
      ctxRef = null;
      inflightRequest = null;
      nextRefreshReason = "poll";
      nextRefreshOptions = {};
      if (abortController) {
        try {
          abortController.abort();
        } catch (error) {
          console.warn("[Services] Failed to abort request on dispose", error);
        }
        abortController = null;
      }
    },
  };
}

registerPage("services", createLifecycle());
