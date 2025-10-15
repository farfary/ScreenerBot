import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $ } from "../core/dom.js";
import * as Utils from "../core/utils.js";

const state = {
  servicesData: null,
  sortKey: "priority",
  sortDir: "asc",
  hasInitialSnapshot: false,
  isFetching: false,
  abortController: null,
  lastErrorMessage: null,
  isActive: false,
};

function currentFilters() {
  const q = ($("#serviceSearch")?.value || "").toLowerCase();
  const status = $("#statusFilter")?.value || "all";
  const enabledOnly = $("#enabledOnly")?.checked || false;
  return { q, status, enabledOnly };
}

function healthRank(status) {
  const ranks = { healthy: 3, degraded: 2, starting: 1, unhealthy: 0 };
  return ranks[status] ?? -1;
}

function getHealthStatus(health) {
  const statuses = {
    healthy: "✅ Healthy",
    starting: "⏳ Starting",
    degraded: "⚠️ Degraded",
    unhealthy: "❌ Unhealthy",
  };
  return statuses[health?.status] || `⏸️ ${health?.status || "unknown"}`;
}

function getHealthBadgeClass(health) {
  const classes = {
    healthy: "success",
    starting: "warning",
    degraded: "warning",
    unhealthy: "error",
  };
  return classes[health?.status] || "secondary";
}

function filteredAndSortedServices() {
  if (!state.servicesData) return [];
  const { q, status, enabledOnly } = currentFilters();
  const items = Array.isArray(state.servicesData.services)
    ? state.servicesData.services.slice()
    : [];

  const filtered = items.filter((service) => {
    const name = (service.name || "").toLowerCase();
    const matchesText = !q || name.includes(q);
    const matchesStatus = status === "all" || service.health?.status === status;
    const matchesEnabled = !enabledOnly || !!service.enabled;
    return matchesText && matchesStatus && matchesEnabled;
  });

  const sortDir = state.sortDir === "asc" ? 1 : -1;

  const getKey = (service) => {
    const metrics = service.metrics || {};
    const keyMap = {
      name: service.name || "",
      health: healthRank(service.health?.status),
      priority: service.priority || 0,
      enabled: service.enabled ? 1 : 0,
      uptime: service.uptime_seconds || 0,
      activity: (() => {
        const total =
          (metrics.total_poll_duration_ns || 0) +
          (metrics.total_idle_duration_ns || 0);
        return total > 0 ? (metrics.total_poll_duration_ns || 0) / total : 0;
      })(),
      lastCycle: metrics.last_cycle_duration_ns || 0,
      avgCycle: metrics.avg_cycle_duration_ns || 0,
      avgPoll: metrics.mean_poll_duration_ns || 0,
      cycleRate: metrics.cycles_per_second || 0,
      tasks: metrics.task_count || 0,
      ops: metrics.operations_per_second || 0,
      errors: metrics.errors_total || 0,
    };
    return keyMap[state.sortKey] ?? (service.priority || 0);
  };

  filtered.sort((a, b) => {
    const aKey = getKey(a);
    const bKey = getKey(b);
    if (aKey < bKey) return -1 * sortDir;
    if (aKey > bKey) return 1 * sortDir;
    return 0;
  });

  return filtered;
}

function setLoading(message = "Loading services...") {
  const tbody = $("#servicesTableBody");
  if (!tbody) return;
  tbody.innerHTML = `
    <tr>
      <td colspan="14" style="text-align:center; padding: 20px; color: var(--text-muted);">
        ${message}
      </td>
    </tr>
  `;
}

function showError(message) {
  const tbody = $("#servicesTableBody");
  if (!tbody) return;
  tbody.innerHTML = `
    <tr>
      <td colspan="14" style="text-align:center; padding: 20px; color: #ef4444;">
        ${message}
      </td>
    </tr>
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
  const metrics = firstService?.metrics || {};
  if (cpuEl) {
    const cpu = Number.isFinite(metrics.process_cpu_percent)
      ? metrics.process_cpu_percent.toFixed(1) + "%"
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

function renderRow(service) {
  const metrics = service.metrics || {};
  const dependencies = Array.isArray(service.dependencies)
    ? service.dependencies.map(
        (dep) => `<span class="dependency-badge">${dep}</span>`
      )
    : [];

  const total =
    (metrics.total_poll_duration_ns || 0) +
    (metrics.total_idle_duration_ns || 0);
  const activity =
    total > 0 ? ((metrics.total_poll_duration_ns || 0) / total) * 100 : 0;
  const activityColor =
    activity > 80
      ? "#10b981"
      : activity > 50
      ? "#3b82f6"
      : activity > 20
      ? "#f59e0b"
      : activity > 5
      ? "#6b7280"
      : "#9ca3af";

  const lastCycle = Utils.formatDuration(metrics.last_cycle_duration_ns || 0);
  const avgCycle = Utils.formatDuration(metrics.avg_cycle_duration_ns || 0);
  const avgPoll = Utils.formatDuration(metrics.mean_poll_duration_ns || 0);
  const cycleRate = Number.isFinite(metrics.cycles_per_second)
    ? metrics.cycles_per_second.toFixed(2)
    : "0.00";

  const taskInfo =
    metrics.task_count > 0
      ? `${
          metrics.task_count
        } tasks\nLast cycle: ${lastCycle}\nAvg cycle: ${avgCycle}\nPoll: ${Utils.formatDuration(
          metrics.mean_poll_duration_ns
        )}\nIdle: ${Utils.formatDuration(
          metrics.mean_idle_duration_ns
        )}\nTotal Polls: ${metrics.total_polls || 0}`
      : "No instrumented tasks";

  return `
    <tr>
      <td style="font-weight:600;">${service.name || "-"}</td>
      <td>
        <span class="badge ${getHealthBadgeClass(service.health)}">
          ${getHealthStatus(service.health)}
        </span>
      </td>
      <td>${service.priority ?? "-"}</td>
      <td>${service.enabled ? "✅" : "❌"}</td>
      <td>${Utils.formatUptime(service.uptime_seconds, {
        style: "compact",
      })}</td>
      <td class="activity-cell" title="${activity.toFixed(1)}% busy">
        <div class="activity-track">
          <div class="activity-fill" style="width:${activity.toFixed(
            1
          )}%; background:${activityColor};"></div>
        </div>
        <div class="activity-meta">
          <span>${activity.toFixed(1)}%</span>
          <span>${metrics.total_polls || 0} polls</span>
        </div>
      </td>
      <td title="Last full loop duration">${lastCycle}</td>
      <td title="Average full loop duration">${avgCycle}</td>
      <td title="Average duration per poll">${avgPoll}</td>
      <td title="Cycles per second">${cycleRate}</td>
      <td title="${taskInfo}">${metrics.task_count || 0}</td>
      <td title="Operations per second">${(
        metrics.operations_per_second || 0
      ).toFixed(2)}</td>
      <td title="Total errors">${metrics.errors_total || 0}</td>
      <td>${
        dependencies.length > 0
          ? dependencies.join(" ")
          : '<span class="detail-value">None</span>'
      }</td>
    </tr>
  `;
}

function renderServicesTable() {
  if (!state.servicesData) return;

  updateSummary(state.servicesData.summary);
  if (Array.isArray(state.servicesData.services)) {
    updateProcessMetrics(state.servicesData.services[0]);
  }

  const tbody = $("#servicesTableBody");
  if (!tbody) return;

  const rows = filteredAndSortedServices()
    .map((service) => renderRow(service))
    .join("");

  if (!rows) {
    tbody.innerHTML = `
      <tr>
        <td colspan="14" style="text-align:center; padding: 20px; color: var(--text-muted);">
          No services match your filters
        </td>
      </tr>
    `;
  } else {
    tbody.innerHTML = rows;
  }
}

async function fetchServices(
  ctx,
  { reason = "poll", showSpinner = false } = {}
) {
  if (!state.isActive && reason !== "initial") return;

  if (state.isFetching) {
    if (reason === "poll") return;
    if (state.abortController) {
      state.abortController.abort();
      state.abortController = null;
    }
  }

  if (showSpinner) setLoading("Loading services...");

  const controller = ctx.createAbortController();
  state.abortController = controller;
  state.isFetching = true;

  try {
    const response = await fetch("/api/services/overview", {
      signal: controller.signal,
      headers: { "X-Requested-With": "fetch" },
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const data = await response.json();
    state.servicesData = data;
    state.hasInitialSnapshot = true;
    state.lastErrorMessage = null;
    renderServicesTable();
  } catch (error) {
    if (error?.name === "AbortError") return;

    console.error("[Services] Failed to fetch services:", error);

    if (!state.hasInitialSnapshot) {
      state.lastErrorMessage = `Failed to load services: ${error.message}`;
      showError(state.lastErrorMessage);
    } else {
      state.lastErrorMessage = error.message;
      Utils.showToast("⚠️ Failed to refresh services", "warning");
    }
  } finally {
    if (state.abortController === controller) {
      state.abortController = null;
    }
    state.isFetching = false;
  }
}

function handleManualRefresh(poller) {
  if (!state.isActive) return;

  if (poller?.stop) poller.stop();

  if (state.abortController) {
    state.abortController.abort();
    state.abortController = null;
  }

  state.hasInitialSnapshot = false;
  state.servicesData = null;
  setLoading("Refreshing services...");

  fetchServices({ reason: "manual", showSpinner: true }).finally(() => {
    if (state.isActive && poller?.start) {
      poller.start({ silent: true });
    }
  });
}

function bindToolbarEvents() {
  const search = $("#serviceSearch");
  const status = $("#statusFilter");
  const enabledOnly = $("#enabledOnly");

  if (search) search.addEventListener("input", renderServicesTable);
  if (status) status.addEventListener("change", renderServicesTable);
  if (enabledOnly) enabledOnly.addEventListener("change", renderServicesTable);
}

function bindSortHandlers() {
  document
    .querySelectorAll("#servicesTable thead th[data-sort]")
    .forEach((th) => {
      th.addEventListener("click", () => {
        const key = th.getAttribute("data-sort");
        if (!key) return;
        if (state.sortKey === key) {
          state.sortDir = state.sortDir === "asc" ? "desc" : "asc";
        } else {
          state.sortKey = key;
          state.sortDir = "asc";
        }
        document
          .querySelectorAll("#servicesTable thead th[data-sort]")
          .forEach((header) => {
            header.classList.remove("asc", "desc");
          });
        th.classList.add(state.sortDir);
        renderServicesTable();
      });
    });
}

function createLifecycle() {
  let poller = null;

  return {
    init(ctx) {
      console.log("[Services] Lifecycle init");
      bindToolbarEvents();
      bindSortHandlers();

      const exportBtn = $("#servicesExportBtn");
      if (exportBtn) {
        exportBtn.addEventListener("click", () =>
          console.warn("[Services] Export not yet migrated")
        );
      }
    },

    activate(ctx) {
      console.log("[Services] Lifecycle activate");
      state.isActive = true;

      if (!state.hasInitialSnapshot) {
        setLoading("Loading services...");
      }

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => fetchServices(ctx, { reason: "poll" }), {
            label: "Services",
          })
        );
      }

      poller.start();

      if (!state.hasInitialSnapshot) {
        fetchServices(ctx, { reason: "initial" });
      } else if (state.servicesData) {
        renderServicesTable();
      }

      window.refreshServices = () => handleManualRefresh(poller);
    },

    deactivate() {
      console.log("[Services] Lifecycle deactivate");
      state.isActive = false;
      if (state.abortController) {
        state.abortController.abort();
        state.abortController = null;
      }
    },

    dispose() {
      console.log("[Services] Lifecycle dispose");
      state.isActive = false;
      poller = null;
      state.servicesData = null;
      state.hasInitialSnapshot = false;
      state.lastErrorMessage = null;
      delete window.refreshServices;
    },
  };
}

registerPage("services", createLifecycle());
