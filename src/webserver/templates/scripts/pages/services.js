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
    <div class="activity-cell" title="${Utils.escapeHtml(I18n.t("services-activity-busy", { percent: Utils.formatPercentValue(activity, { decimals: 1, includeSign: false }) }))}">
      <div class="activity-track">
        <div class="activity-fill" style="width:${activity.toFixed(
          1
        )}%; background:${color};"></div>
      </div>
      <div class="activity-meta">
        <span>${Utils.formatPercentValue(activity, { decimals: 1, includeSign: false })}</span>
        <span>${Utils.escapeHtml(I18n.t("services-activity-polls", { count: metrics.total_polls || 0 }))}</span>
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
          render: (v) => `<strong>${v ? Utils.escapeHtml(serviceName(v)) : "-"}</strong>`,
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
          render: (v) => v ?? "-",
        },
        {
          id: "enabled",
          label: I18n.t("common-state-enabled"),
          sortable: true,
          minWidth: 72,
          render: (v) =>
            v
              ? '<i class="icon-circle-check" style="color: var(--success);"></i>'
              : '<i class="icon-circle-x" style="color: var(--error);"></i>',
        },
        {
          id: "uptime",
          label: I18n.t("services-col-uptime"),
          type: "number",
          sortable: true,
          minWidth: 96,
          render: (v, row) => Utils.formatUptime(row.uptime_seconds, { style: "compact" }),
          sortFn: (a, b) => (a.uptime_seconds || 0) - (b.uptime_seconds || 0),
        },
        {
          id: "activity",
          label: I18n.t("services-col-activity"),
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
          label: I18n.t("services-col-last-cycle"),
          type: "number",
          sortable: true,
          minWidth: 96,
          render: (v, row) => Utils.formatDuration(row.metrics?.last_cycle_duration_ns || 0),
          sortFn: (a, b) =>
            (a.metrics?.last_cycle_duration_ns || 0) - (b.metrics?.last_cycle_duration_ns || 0),
        },
        {
          id: "avgCycle",
          label: I18n.t("services-col-avg-cycle"),
          type: "number",
          sortable: true,
          minWidth: 96,
          render: (v, row) => Utils.formatDuration(row.metrics?.avg_cycle_duration_ns || 0),
          sortFn: (a, b) =>
            (a.metrics?.avg_cycle_duration_ns || 0) - (b.metrics?.avg_cycle_duration_ns || 0),
        },
        {
          id: "avgPoll",
          label: I18n.t("services-col-avg-poll"),
          type: "number",
          sortable: true,
          minWidth: 96,
          render: (v, row) => Utils.formatDuration(row.metrics?.mean_poll_duration_ns || 0),
          sortFn: (a, b) =>
            (a.metrics?.mean_poll_duration_ns || 0) - (b.metrics?.mean_poll_duration_ns || 0),
        },
        {
          id: "cycleRate",
          label: I18n.t("services-col-cycle-rate"),
          type: "number",
          sortable: true,
          minWidth: 90,
          render: (v, row) => {
            const rate = row.metrics?.cycles_per_second;
            return formatFixed(Number.isFinite(rate) ? rate : 0);
          },
          sortFn: (a, b) =>
            (a.metrics?.cycles_per_second || 0) - (b.metrics?.cycles_per_second || 0),
        },
        {
          id: "tasks",
          label: I18n.t("services-col-tasks"),
          type: "number",
          sortable: true,
          minWidth: 90,
          render: (v, row) => {
            const m = row.metrics || {};
            const taskInfo =
              m.task_count > 0
                ? I18n.t("services-tasks-tooltip", {
                    count: m.task_count,
                    last: Utils.formatDuration(m.last_cycle_duration_ns),
                    avg: Utils.formatDuration(m.avg_cycle_duration_ns),
                    poll: Utils.formatDuration(m.mean_poll_duration_ns),
                    idle: Utils.formatDuration(m.mean_idle_duration_ns),
                    polls: m.total_polls || 0,
                  })
                : I18n.t("services-tasks-none");
            return `<span title="${Utils.escapeHtml(taskInfo)}">${m.task_count || 0}</span>`;
          },
          sortFn: (a, b) => (a.metrics?.task_count || 0) - (b.metrics?.task_count || 0),
        },
        {
          id: "ops",
          label: I18n.t("services-col-ops"),
          type: "number",
          sortable: true,
          minWidth: 90,
          render: (v, row) => formatFixed(row.metrics?.operations_per_second || 0),
          sortFn: (a, b) =>
            (a.metrics?.operations_per_second || 0) - (b.metrics?.operations_per_second || 0),
        },
        {
          id: "errors",
          label: I18n.t("services-col-errors"),
          type: "number",
          sortable: true,
          minWidth: 80,
          render: (v, row) => row.metrics?.errors_total || 0,
          sortFn: (a, b) => (a.metrics?.errors_total || 0) - (b.metrics?.errors_total || 0),
        },
        {
          id: "dependencies",
          label: I18n.t("services-col-dependencies"),
          sortable: false,
          minWidth: 160,
          render: (v, row) => {
            const deps = Array.isArray(row.dependencies) ? row.dependencies : [];
            if (deps.length === 0) {
              return `<span class="dependency-badge dependency-badge--none">${Utils.escapeHtml(I18n.t("services-dependencies-none"))}</span>`;
            }
            // Clamp the badge list to a couple of rows; full list on hover.
            const badges = deps
              .map((dep) => `<span class="dependency-badge">${Utils.escapeHtml(serviceName(String(dep)))}</span>`)
              .join(" ");
            const full = deps.map((dep) => serviceName(String(dep))).join("\n");
            return `<div class="dt-longtext" data-tooltip="${Utils.escapeHtml(full)}" data-tooltip-truncated>${badges}</div>`;
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
