// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Services page - polling table of service health, status badges and poll-activity metrics from /api/services/overview.

import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { formatFixed } from "../core/format.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";
import { requestManager } from "../core/request_manager.js";

// Service ids from `Service::name()` (src/services/implementations/*.rs); a Rust test pins each to the catalog.
const SERVICE_NAME_LABELS = Object.freeze({
  account: "services-name-account",
  assistant_scheduled_tasks: "services-name-assistant-scheduled-tasks",
  ata_cleanup: "services-name-ata-cleanup",
  connectivity: "services-name-connectivity",
  copy_trading: "services-name-copy-trading",
  events: "services-name-events",
  filtering: "services-name-filtering",
  llm_analysis: "services-name-llm-analysis",
  ohlcv: "services-name-ohlcv",
  pool_pricing: "services-name-pool-pricing",
  pools: "services-name-pools",
  positions: "services-name-positions",
  referral: "services-name-referral",
  rpc_stats: "services-name-rpc-stats",
  sol_price: "services-name-sol-price",
  telegram: "services-name-telegram",
  tokens: "services-name-tokens",
  trader: "services-name-trader",
  transactions: "services-name-transactions",
  update_check: "services-name-update-check",
  wallet: "services-name-wallet",
  wallet_watch: "services-name-wallet-watch",
  webserver: "services-name-webserver",
});

// `ServiceHealth` status tags (src/services/health.rs); `unknown` covers a row without health.
const HEALTH_STATUS_LABELS = Object.freeze({
  healthy: "services-status-healthy",
  starting: "services-status-starting",
  degraded: "services-status-degraded",
  unhealthy: "services-status-unhealthy",
  stopping: "services-status-stopping",
  disabled: "services-status-disabled",
  unknown: "services-status-unknown",
});

const HEALTH_STYLES = {
  healthy: { variant: "success", icon: "icon-check" },
  starting: { variant: "warning", icon: "icon-loader" },
  degraded: { variant: "warning", icon: "icon-triangle-alert" },
  unhealthy: { variant: "error", icon: "icon-x" },
};
const HEALTH_STYLE_FALLBACK = { variant: "secondary", icon: "icon-pause" };

const serviceName = (id) => I18n.label(SERVICE_NAME_LABELS, id);

const ABSENT = "—";

/**
 * The API sends `metrics: null` for a disabled or not yet sampled service. Poll-derived
 * readings (activity, cycle and poll durations, cycle rate) additionally need an
 * instrumented task; without one they are absent, not zero.
 */
const pollMetrics = (row) => (row.metrics?.task_count > 0 ? row.metrics : null);

/** Sort value for a reading that may be absent: absent sorts below every measured value. */
const sortReading = (value) => (Number.isFinite(value) ? value : -1);

// Helper functions
function healthRank(status) {
  const ranks = { healthy: 4, disabled: 3, degraded: 2, starting: 1, unhealthy: 0 };
  return ranks[status] ?? -1;
}

function getHealthBadge(health) {
  const status = health?.status || "unknown";
  const { variant, icon } = HEALTH_STYLES[status] ?? HEALTH_STYLE_FALLBACK;
  // Degraded and unhealthy services carry the reason as a UiText.
  const reason = health?.message ? I18n.text(health.message) : "";
  const title = reason ? ` title="${Utils.escapeHtml(reason)}"` : "";
  return `<span class="badge ${variant}"${title}><i class="${icon}"></i> ${Utils.escapeHtml(I18n.label(HEALTH_STATUS_LABELS, status))}</span>`;
}

/**
 * Share of a service's sampled time spent polling, as a percentage; null when the service
 * reports no poll readings.
 */
function activityPercent(row) {
  const metrics = pollMetrics(row);
  if (!metrics) return null;
  const total = metrics.total_poll_duration_ns + metrics.total_idle_duration_ns;
  return total > 0 ? (metrics.total_poll_duration_ns / total) * 100 : null;
}

function activityTier(activity) {
  if (activity > 80) return "busy";
  if (activity > 50) return "active";
  if (activity > 20) return "moderate";
  return "idle";
}

function getActivityBar(row) {
  const activity = activityPercent(row);
  if (activity === null) return ABSENT;
  const percent = Utils.formatPercentValue(activity, { decimals: 1, includeSign: false });
  const fillWidth = activity.toFixed(2);
  return `
    <div class="activity-cell" title="${Utils.escapeHtml(I18n.t("services-activity-busy", { percent }))}">
      <div class="activity-track">
        <div class="activity-fill activity-fill--${activityTier(activity)}" style="inline-size:${fillWidth}%;"></div>
      </div>
      <div class="activity-meta">
        <span>${percent}</span>
        <span>${Utils.escapeHtml(I18n.t("services-activity-polls", { count: pollMetrics(row).total_polls }))}</span>
      </div>
    </div>
  `;
}

function createLifecycle() {
  let table = null;
  let poller = null;

  const state = {
    summary: null,
    hasLoadedOnce: false,
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
    const disabled =
      summary?.disabled_services ?? rows.filter((row) => row.health?.status === "disabled").length;
    const total = summary?.total_services ?? rows.length;
    const alerts = degraded + unhealthy;

    table.updateToolbarSummary([
      {
        id: "services-total",
        label: I18n.t("services-summary-total"),
        value: Utils.formatNumber(total, 0),
      },
      {
        id: "services-healthy",
        label: I18n.label(HEALTH_STATUS_LABELS, "healthy"),
        value: Utils.formatNumber(healthy, 0),
        variant: "success",
      },
      {
        id: "services-disabled",
        label: I18n.label(HEALTH_STATUS_LABELS, "disabled"),
        value: Utils.formatNumber(disabled, 0),
      },
      {
        id: "services-alerts",
        label: I18n.t("services-summary-alerts"),
        value: Utils.formatNumber(alerts, 0),
        variant: alerts > 0 ? "warning" : "success",
        tooltip: I18n.t("services-summary-alerts-tooltip", {
          degraded: Utils.formatNumber(degraded, 0),
          unhealthy: Utils.formatNumber(unhealthy, 0),
        }),
      },
    ]);
  };

  const loadServicesPage = async ({ reason, signal }) => {
    if (!state.hasLoadedOnce && reason !== "poll" && table?.showBlockingState) {
      table.showBlockingState({
        variant: "loading",
        title: I18n.t("services-loading"),
      });
    }

    try {
      const data = await requestManager.fetch("/api/services/overview", {
        headers: { "X-Requested-With": "fetch" },
        cache: "no-store",
        priority: "normal",
        signal,
      });

      const services = Array.isArray(data?.services) ? data.services : [];
      state.summary = data?.summary ?? null;

      state.hasLoadedOnce = true;
      table?.hideBlockingState?.();

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
      if (!state.hasLoadedOnce) {
        table?.showBlockingState?.({
          variant: "error",
          title: I18n.t("services-load-failed"),
          description: I18n.t("services-load-failed-description"),
        });
      } else if (reason !== "poll") {
        Utils.showToast({
          key: "services-load",
          type: "warning",
          title: I18n.t("services-refresh-failed"),
        });
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
          label: I18n.t("services-col-service"),
          sortable: true,
          floating: true,
          minWidth: 140,
          wrap: false,
          render: (v) => `<strong>${v ? Utils.escapeHtml(serviceName(v)) : ABSENT}</strong>`,
        },
        {
          id: "health",
          label: I18n.t("services-col-health"),
          sortable: true,
          minWidth: 120,
          render: (v, row) => getHealthBadge(row.health),
          sortFn: (a, b) => healthRank(a.health?.status) - healthRank(b.health?.status),
        },
        {
          id: "priority",
          label: I18n.t("services-col-priority"),
          type: "number",
          sortable: true,
          minWidth: 72,
          render: (v) => v ?? ABSENT,
        },
        {
          id: "enabled",
          label: I18n.t("common-state-enabled"),
          sortable: true,
          minWidth: 72,
          render: (v) =>
            v
              ? '<i class="icon-circle-check services-enabled-icon is-on"></i>'
              : '<i class="icon-circle-x services-enabled-icon is-off"></i>',
        },
        {
          id: "uptime",
          label: I18n.t("services-col-uptime"),
          type: "number",
          sortable: true,
          minWidth: 96,
          render: (v, row) =>
            Utils.formatUptime(row.uptime_seconds, { style: "compact", fallback: ABSENT }),
          sortFn: (a, b) => sortReading(a.uptime_seconds) - sortReading(b.uptime_seconds),
        },
        {
          id: "activity",
          label: I18n.t("services-col-activity"),
          sortable: true,
          minWidth: 200,
          render: (v, row) => getActivityBar(row),
          sortFn: (a, b) => sortReading(activityPercent(a)) - sortReading(activityPercent(b)),
        },
        {
          id: "lastCycle",
          label: I18n.t("services-col-last-cycle"),
          type: "number",
          sortable: true,
          minWidth: 96,
          render: (v, row) =>
            Utils.formatDuration(pollMetrics(row)?.last_cycle_duration_ns, ABSENT),
          sortFn: (a, b) =>
            sortReading(pollMetrics(a)?.last_cycle_duration_ns) -
            sortReading(pollMetrics(b)?.last_cycle_duration_ns),
        },
        {
          id: "avgCycle",
          label: I18n.t("services-col-avg-cycle"),
          type: "number",
          sortable: true,
          minWidth: 96,
          render: (v, row) => Utils.formatDuration(pollMetrics(row)?.avg_cycle_duration_ns, ABSENT),
          sortFn: (a, b) =>
            sortReading(pollMetrics(a)?.avg_cycle_duration_ns) -
            sortReading(pollMetrics(b)?.avg_cycle_duration_ns),
        },
        {
          id: "avgPoll",
          label: I18n.t("services-col-avg-poll"),
          type: "number",
          sortable: true,
          minWidth: 96,
          render: (v, row) => Utils.formatDuration(pollMetrics(row)?.mean_poll_duration_ns, ABSENT),
          sortFn: (a, b) =>
            sortReading(pollMetrics(a)?.mean_poll_duration_ns) -
            sortReading(pollMetrics(b)?.mean_poll_duration_ns),
        },
        {
          id: "cycleRate",
          label: I18n.t("services-col-cycle-rate"),
          type: "number",
          sortable: true,
          minWidth: 90,
          render: (v, row) => formatFixed(pollMetrics(row)?.cycles_per_second),
          sortFn: (a, b) =>
            sortReading(pollMetrics(a)?.cycles_per_second) -
            sortReading(pollMetrics(b)?.cycles_per_second),
        },
        {
          id: "tasks",
          label: I18n.t("services-col-tasks"),
          type: "number",
          sortable: true,
          minWidth: 90,
          render: (v, row) => {
            const m = row.metrics;
            if (!m) return ABSENT;
            const taskInfo =
              m.task_count > 0
                ? I18n.t("services-tasks-tooltip", {
                    count: m.task_count,
                    last: Utils.formatDuration(m.last_cycle_duration_ns),
                    avg: Utils.formatDuration(m.avg_cycle_duration_ns),
                    poll: Utils.formatDuration(m.mean_poll_duration_ns),
                    idle: Utils.formatDuration(m.mean_idle_duration_ns),
                    polls: m.total_polls,
                  })
                : I18n.t("services-tasks-none");
            return `<span title="${Utils.escapeHtml(taskInfo)}">${Utils.formatNumber(m.task_count, 0)}</span>`;
          },
          sortFn: (a, b) => sortReading(a.metrics?.task_count) - sortReading(b.metrics?.task_count),
        },
        {
          id: "ops",
          label: I18n.t("services-col-ops"),
          type: "number",
          sortable: true,
          minWidth: 90,
          render: (v, row) => formatFixed(row.metrics?.operations_per_second),
          sortFn: (a, b) =>
            sortReading(a.metrics?.operations_per_second) -
            sortReading(b.metrics?.operations_per_second),
        },
        {
          id: "errors",
          label: I18n.t("services-col-errors"),
          type: "number",
          sortable: true,
          minWidth: 80,
          render: (v, row) => Utils.formatNumber(row.metrics?.errors_total, 0),
          sortFn: (a, b) =>
            sortReading(a.metrics?.errors_total) - sortReading(b.metrics?.errors_total),
        },
        {
          id: "dependencies",
          label: I18n.t("services-col-dependencies"),
          sortable: false,
          minWidth: 160,
          wrap: false,
          render: (v, row) => {
            const deps = Array.isArray(row.dependencies) ? row.dependencies : [];
            if (deps.length === 0) return ABSENT;
            // One line per row: the column sizes to its widest list.
            const badges = deps
              .map(
                (dep) =>
                  `<span class="dependency-badge">${Utils.escapeHtml(serviceName(String(dep)))}</span>`
              )
              .join("");
            return `<span class="dependency-list">${badges}</span>`;
          },
        },
      ];

      table = new DataTable({
        container: "#services-root",
        columns,
        rowIdField: "name",
        stateKey: "services-table",
        emptyTitle: I18n.t("services-empty"),
        emptyMessage: I18n.attr("services-empty", "message"),
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
          summary: [
            { id: "services-total", label: I18n.t("services-summary-total"), value: "0" },
            {
              id: "services-healthy",
              label: I18n.label(HEALTH_STATUS_LABELS, "healthy"),
              value: "0",
              variant: "success",
            },
            {
              id: "services-disabled",
              label: I18n.label(HEALTH_STATUS_LABELS, "disabled"),
              value: "0",
            },
            {
              id: "services-alerts",
              label: I18n.t("services-summary-alerts"),
              value: "0",
              variant: "warning",
            },
          ],
          search: {
            enabled: true,
            placeholder: I18n.t("services-search-placeholder"),
          },
          filters: [
            {
              id: "status",
              label: I18n.t("services-filter-status"),
              options: [
                { value: "all", label: I18n.t("services-filter-all-statuses") },
                { value: "healthy", label: I18n.label(HEALTH_STATUS_LABELS, "healthy") },
                { value: "starting", label: I18n.label(HEALTH_STATUS_LABELS, "starting") },
                { value: "degraded", label: I18n.label(HEALTH_STATUS_LABELS, "degraded") },
                { value: "unhealthy", label: I18n.label(HEALTH_STATUS_LABELS, "unhealthy") },
              ],
              filterFn: (row, value) => value === "all" || row.health?.status === value,
            },
            {
              id: "enabled",
              label: I18n.t("common-state-enabled"),
              options: [
                { value: "all", label: I18n.t("services-filter-all-services") },
                { value: "enabled", label: I18n.t("services-filter-enabled-only") },
                { value: "disabled", label: I18n.t("services-filter-disabled-only") },
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
            label: "Services", // l10n-ignore: poller log name
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
      state.hasLoadedOnce = false;
      window.servicesTable = null;
    },
  };
}

registerPage("services", createLifecycle());
