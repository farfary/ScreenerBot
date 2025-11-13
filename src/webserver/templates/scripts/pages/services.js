import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
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
    healthy: '<span class="badge success"><i class="icon-check"></i> Healthy</span>',
    starting: '<span class="badge warning"><i class="icon-loader"></i> Starting</span>',
    degraded: '<span class="badge warning"><i class="icon-alert-triangle"></i> Degraded</span>',
    unhealthy: '<span class="badge error"><i class="icon-x"></i> Unhealthy</span>',
  };
  return (
    badges[status] || `<span class="badge secondary"><i class="icon-pause"></i> ${status}</span>`
  );
}

function getActivityBar(metrics) {
  const total = (metrics.total_poll_duration_ns || 0) + (metrics.total_idle_duration_ns || 0);
  const activity = total > 0 ? ((metrics.total_poll_duration_ns || 0) / total) * 100 : 0;
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

function createLifecycle() {
  let table = null;
  let poller = null;

  const state = {
    summary: null,
  };

  const updateToolbar = () => {
    if (!table) {
      return;
    }

    const rows = table.getData();
    const summary = state.summary;

    const healthy =
      summary?.healthy_services ?? rows.filter((row) => row.health?.status === "healthy").length;
    const degraded =
      summary?.degraded_services ?? rows.filter((row) => row.health?.status === "degraded").length;
    const unhealthy =
      summary?.unhealthy_services ??
      rows.filter((row) => row.health?.status === "unhealthy").length;
    const total = summary?.total_services ?? rows.length;
    const alerts = degraded + unhealthy;

    table.updateToolbarSummary([
      {
        id: "services-total",
        label: "Total",
        value: Utils.formatNumber(total, 0),
      },
      {
        id: "services-healthy",
        label: "Healthy",
        value: Utils.formatNumber(healthy, 0),
        variant: "success",
      },
      {
        id: "services-alerts",
        label: "Alerts",
        value: Utils.formatNumber(alerts, 0),
        variant: alerts > 0 ? "warning" : "success",
        tooltip: `${Utils.formatNumber(degraded, 0)} degraded / ${Utils.formatNumber(unhealthy, 0)} unhealthy`,
      },
    ]);

    table.updateToolbarMeta([
      {
        id: "services-last-update",
        text: `Last update ${new Date().toLocaleTimeString()}`,
      },
    ]);
  };

  const loadServicesPage = async ({ reason, signal }) => {
    try {
      const response = await fetch("/api/services/overview", {
        headers: { "X-Requested-With": "fetch" },
        cache: "no-store",
        signal,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      const services = Array.isArray(data?.services) ? data.services : [];
      state.summary = data?.summary ?? null;

      return {
        rows: services,
        cursorNext: null,
        cursorPrev: null,
        hasMoreNext: false,
        hasMorePrev: false,
        total: services.length,
        meta: data?.summary ? { summary: data.summary } : {},
        preserveScroll: reason === "poll",
      };
    } catch (error) {
      if (error?.name === "AbortError") {
        throw error;
      }
      console.error("[Services] Failed to fetch:", error);
      if (reason !== "poll") {
        Utils.showToast("Failed to refresh services", "warning");
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

  return {
    init(_ctx) {
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
          sortFn: (a, b) => healthRank(a.health?.status) - healthRank(b.health?.status),
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
          render: (v, row) => Utils.formatUptime(row.uptime_seconds, { style: "compact" }),
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
                (metrics.total_poll_duration_ns || 0) + (metrics.total_idle_duration_ns || 0);
              return total > 0 ? (metrics.total_poll_duration_ns || 0) / total : 0;
            };
            return calcActivity(a.metrics || {}) - calcActivity(b.metrics || {});
          },
        },
        {
          id: "lastCycle",
          label: "Last Cycle",
          sortable: true,
          minWidth: 96,
          render: (v, row) => Utils.formatDuration(row.metrics?.last_cycle_duration_ns || 0),
          sortFn: (a, b) =>
            (a.metrics?.last_cycle_duration_ns || 0) - (b.metrics?.last_cycle_duration_ns || 0),
        },
        {
          id: "avgCycle",
          label: "Avg Cycle",
          sortable: true,
          minWidth: 96,
          render: (v, row) => Utils.formatDuration(row.metrics?.avg_cycle_duration_ns || 0),
          sortFn: (a, b) =>
            (a.metrics?.avg_cycle_duration_ns || 0) - (b.metrics?.avg_cycle_duration_ns || 0),
        },
        {
          id: "avgPoll",
          label: "Avg Poll",
          sortable: true,
          minWidth: 96,
          render: (v, row) => Utils.formatDuration(row.metrics?.mean_poll_duration_ns || 0),
          sortFn: (a, b) =>
            (a.metrics?.mean_poll_duration_ns || 0) - (b.metrics?.mean_poll_duration_ns || 0),
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
            (a.metrics?.cycles_per_second || 0) - (b.metrics?.cycles_per_second || 0),
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
          sortFn: (a, b) => (a.metrics?.task_count || 0) - (b.metrics?.task_count || 0),
        },
        {
          id: "ops",
          label: "Ops/sec",
          sortable: true,
          minWidth: 90,
          render: (v, row) => (row.metrics?.operations_per_second || 0).toFixed(2),
          sortFn: (a, b) =>
            (a.metrics?.operations_per_second || 0) - (b.metrics?.operations_per_second || 0),
        },
        {
          id: "errors",
          label: "Errors",
          sortable: true,
          minWidth: 80,
          render: (v, row) => row.metrics?.errors_total || 0,
          sortFn: (a, b) => (a.metrics?.errors_total || 0) - (b.metrics?.errors_total || 0),
        },
        {
          id: "dependencies",
          label: "Dependencies",
          sortable: false,
          minWidth: 160,
          render: (v, row) => {
            const deps = Array.isArray(row.dependencies) ? row.dependencies : [];
            return deps.length > 0
              ? deps.map((dep) => `<span class="dependency-badge">${dep}</span>`).join(" ")
              : '<span class="detail-value">None</span>';
          },
        },
      ];

      table = new DataTable({
        container: "#services-root",
        columns,
        rowIdField: "name",
        stateKey: "services-table",
        enableLogging: false,
        sorting: {
          column: "priority",
          direction: "asc",
        },
        compact: true, // Enable compact mode for denser display
        stickyHeader: true,
        zebra: true,
        fitToContainer: true, // Auto-fit columns to container width
        pagination: {
          threshold: 160,
          maxRows: 1000,
          loadPage: loadServicesPage,
          dedupeKey: (row) => row?.name ?? null,
          rowIdField: "name",
          onPageLoaded: handlePageLoaded,
        },
        toolbar: {
          title: {
            icon: "icon-settings",
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
              filterFn: (row, value) => value === "all" || row.health?.status === value,
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
        },
      });

      window.servicesTable = table;
      updateToolbar();
    },

    activate(ctx) {
      // Create and start poller
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => requestReload("poll", { silent: true, preserveScroll: true }), {
            label: "Services",
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
    },

    dispose() {
      if (table) {
        table.destroy();
        table = null;
      }
      poller = null;
      state.summary = null;
      window.servicesTable = null;
    },
  };
}

registerPage("services", createLifecycle());
