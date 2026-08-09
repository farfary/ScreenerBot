/**
 * Filtering Configuration Page
 *
 * Provides UI for:
 * - Viewing/editing filtering configuration
 * - Monitoring filtering statistics
 * - Manual snapshot refresh
 */

import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import {
  FILTER_TABS,
  TIME_RANGE_PRESETS,
  setConfigValue,
  setSourceEnabled,
  setCategoryEnabled,
  deepMerge,
  configsEqual,
  getStatusMessage,
} from "./filtering/config_metadata.js";
import { createFilteringRenderers } from "./filtering/renderers.js";

// ============================================================================
// STATE
// ============================================================================

const state = {
  metadata: null,
  config: null,
  draft: null,
  stats: null,
  rejectionStats: null,
  analytics: null,
  hasChanges: false,
  isSaving: false,
  isRefreshing: false,
  isLoadingAnalytics: false, // Loading state for analytics section
  analyticsRequestId: 0, // Request ID to prevent race conditions
  lastSaved: null,
  searchQuery: AppState.load("filtering_searchQuery", ""),
  activeTab: AppState.load("filtering_activeTab", "status"),
  initialized: false,
  // Time range filter for analytics - persist all values
  timeRange: {
    preset: AppState.load("filtering_timeRangePreset", "all"),
    startTime: AppState.load("filtering_timeRangeStart", null),
    endTime: AppState.load("filtering_timeRangeEnd", null),
  },
};

const TABBAR_STATE_KEY = "filtering.tab";

let tabBar = null;
const eventCleanups = [];

// Helper to track event listeners
function addTrackedListener(element, event, handler) {
  if (!element) return;
  element.addEventListener(event, handler);
  eventCleanups.push(() => element.removeEventListener(event, handler));
}

// Time range filter functions
async function setTimeRangePreset(preset) {
  const now = Math.floor(Date.now() / 1000);
  state.timeRange.preset = preset;

  if (preset === "all" || preset === "custom") {
    state.timeRange.startTime = null;
    state.timeRange.endTime = null;
  } else if (TIME_RANGE_PRESETS[preset]) {
    state.timeRange.startTime = now - TIME_RANGE_PRESETS[preset].seconds;
    state.timeRange.endTime = now;
  }

  // Persist all time range state
  AppState.save("filtering_timeRangePreset", preset);
  AppState.save("filtering_timeRangeStart", state.timeRange.startTime);
  AppState.save("filtering_timeRangeEnd", state.timeRange.endTime);

  // Force a fresh analytics render when the time range changes
  _lastAnalyticsKey = null;

  // Show loading state immediately
  state.isLoadingAnalytics = true;
  render();

  try {
    await loadAnalytics();
  } finally {
    // Always clear loading state, even on error
    state.isLoadingAnalytics = false;
    render();
  }
}

// ============================================================================
async function fetchConfig() {
  return await requestManager.fetch("/api/config/filtering", {
    priority: "high",
  });
}

async function fetchConfigMetadata() {
  return await requestManager.fetch("/api/config/metadata", {
    priority: "high",
  });
}

async function saveConfig(config) {
  return await requestManager.fetch("/api/config/filtering", {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(config),
    priority: "high",
  });
}

async function refreshSnapshot() {
  return await requestManager.fetch("/api/filtering/refresh", {
    method: "POST",
    priority: "high",
  });
}

async function fetchStats() {
  return await requestManager.fetch("/api/filtering/stats", {
    priority: "normal",
  });
}

async function fetchRejectionStats() {
  return await requestManager.fetch("/api/filtering/rejection-stats", {
    priority: "normal",
  });
}

async function fetchAnalytics() {
  // Build URL with time range parameters if set
  let url = "/api/filtering/analytics";
  const params = new URLSearchParams();

  if (state.timeRange.startTime) {
    params.set("start_time", state.timeRange.startTime.toString());
  }
  if (state.timeRange.endTime) {
    params.set("end_time", state.timeRange.endTime.toString());
  }
  if (state.timeRange.preset && state.timeRange.preset !== "all") {
    params.set("preset", state.timeRange.preset);
  }

  const queryString = params.toString();
  if (queryString) {
    url += "?" + queryString;
  }

  return await requestManager.fetch(url, {
    priority: "normal",
  });
}

// ============================================================================
// RENDERERS
// ============================================================================

// Renderers instance - initialized in createLifecycle
let renderers = null;

// ============================================================================
// DOM UPDATE HELPERS
// ============================================================================

let globalHandlersBound = false;

function bindGlobalHandlers() {
  if (globalHandlersBound) return;

  const saveBtn = $("#save-config-btn");
  const resetBtn = $("#reset-config-btn");
  const refreshBtn = $("#refresh-snapshot-btn");
  const exportBtn = $("#export-config-btn");
  const importBtn = $("#import-config-btn");

  if (saveBtn) addTrackedListener(saveBtn, "click", handleSaveConfig);
  if (resetBtn) addTrackedListener(resetBtn, "click", handleResetConfig);
  if (refreshBtn) addTrackedListener(refreshBtn, "click", handleRefreshSnapshot);
  if (exportBtn) addTrackedListener(exportBtn, "click", handleExportConfig);
  if (importBtn) addTrackedListener(importBtn, "click", handleImportConfig);

  globalHandlersBound = true;
}

function bindSourceToggleHandlers() {
  $$("[data-source-toggle]").forEach((input) => {
    addTrackedListener(input, "change", handleSourceToggle);
  });
}

function bindConfigHandlers() {
  const container = $("#filtering-config-panels");
  if (!container) return;

  container.querySelectorAll("[data-field]").forEach((input) => {
    addTrackedListener(input, "input", handleFieldChange);
    if (input.type === "checkbox") {
      addTrackedListener(input, "change", handleFieldChange);
    }
  });

  container.querySelectorAll("[data-category-toggle]").forEach((input) => {
    addTrackedListener(input, "change", handleCategoryToggle);
  });
}

function updateInfoBar() {
  const key = JSON.stringify(state.stats);
  if (key === _lastInfoBarKey) return;
  _lastInfoBarKey = key;
  const container = $("#filtering-info-bar");
  if (container) {
    container.innerHTML = renderers.renderInfoBar();
  }
}

function updateSearchBar() {
  const container = $("#filtering-search-bar");
  if (!container) return;

  const html = renderers.renderSearchBar();
  container.innerHTML = html;

  // Hide the container if no content
  container.style.display = html ? "" : "none";

  if (html) {
    bindSearchHandler();
    bindSourceToggleHandlers();
  }
}

function bindSearchHandler() {
  const searchInput = $("#filtering-search");
  if (searchInput) {
    addTrackedListener(searchInput, "input", (event) => {
      state.searchQuery = (event.target.value || "").toLowerCase();
      AppState.save("filtering_searchQuery", state.searchQuery);
      updateConfigPanels({ scrollTop: 0 });
    });
  }
}

function updateConfigPanels(options = {}) {
  const container = $("#filtering-config-panels");
  if (!container) return;

  // Scroll lives on the inner .config-scroll-area (config tabs), .analytics-scroll-area
  // (analytics tab), or the two independent status columns (.status-metrics-section /
  // .status-rejection-section). Save them all before the innerHTML replacement.
  const prevScrollEl =
    container.querySelector(".config-scroll-area, .analytics-scroll-area") || container;
  const previousScroll = prevScrollEl.scrollTop;
  const prevLeftScroll = container.querySelector(".status-metrics-section")?.scrollTop ?? 0;
  const prevRightScroll = container.querySelector(".status-rejection-section")?.scrollTop ?? 0;

  container.innerHTML = renderers.renderConfigPanels();
  bindConfigHandlers();

  const nextScrollEl =
    container.querySelector(".config-scroll-area, .analytics-scroll-area") || container;
  if (options.scrollTop === 0) {
    nextScrollEl.scrollTo({ top: 0, behavior: options.smooth ? "smooth" : "auto" });
  } else if (options.preserveScroll) {
    nextScrollEl.scrollTop = previousScroll;
    const leftEl = container.querySelector(".status-metrics-section");
    const rightEl = container.querySelector(".status-rejection-section");
    if (leftEl) leftEl.scrollTop = prevLeftScroll;
    if (rightEl) rightEl.scrollTop = prevRightScroll;
  }
}

function updateStatusMessage() {
  const statusEl = $("#filtering-status-message");
  if (statusEl) {
    const statusMsg = getStatusMessage({
      isSaving: state.isSaving,
      isRefreshing: state.isRefreshing,
      hasChanges: state.hasChanges,
      lastSaved: state.lastSaved,
      Utils,
    });
    statusEl.textContent = statusMsg;
  }
}

function render() {
  const root = $("#filtering-root");
  if (!root) return;

  if (!state.initialized) {
    root.innerHTML = renderers.renderShell();
    state.initialized = true;
    bindGlobalHandlers();
  }

  updateInfoBar();
  updateSearchBar();
  updateConfigPanels({ preserveScroll: true });
  updateStatusMessage();
  updateActionButtons();
}
// ============================================================================
// EVENT HANDLERS
// ============================================================================

function handleFieldChange(event) {
  const input = event.target;
  const fieldKey = input.dataset.field;
  const source = input.dataset.source;

  if (!fieldKey || !source) return;

  let value;
  if (input.type === "checkbox") {
    value = input.checked;
  } else if (input.type === "number") {
    value = parseFloat(input.value);
    if (isNaN(value)) return;
  } else {
    value = input.value;
  }

  setConfigValue(state.draft, source, fieldKey, value);
  checkForChanges();
  updateActionButtons();
  updateStatusMessage();
}

function handleSourceToggle(event) {
  const input = event.target;
  const source = input.dataset.sourceToggle;
  const enabled = input.checked;

  setSourceEnabled(state.draft, source, enabled);
  checkForChanges();
  render(); // Need full re-render to update disabled states
}

function handleCategoryToggle(event) {
  const input = event.target;
  const source = input.dataset.categoryToggle;
  const enableKey = input.dataset.enableKey;
  const enabled = input.checked;

  setCategoryEnabled(state.draft, source, enableKey, enabled);
  checkForChanges();
  render(); // Need full re-render to update disabled states
}

function updateActionButtons() {
  const saveBtn = $("#save-config-btn");
  const resetBtn = $("#reset-config-btn");
  const refreshBtn = $("#refresh-snapshot-btn");

  if (saveBtn) {
    saveBtn.disabled = !state.hasChanges || state.isSaving;
  }
  if (resetBtn) {
    resetBtn.disabled = !state.hasChanges;
  }
  if (refreshBtn) {
    refreshBtn.disabled = state.isRefreshing;
  }
}

function checkForChanges() {
  if (!state.config || !state.draft) {
    state.hasChanges = false;
    return;
  }

  state.hasChanges = !configsEqual(state.config, state.draft);
}

async function handleSaveConfig() {
  if (!state.hasChanges || state.isSaving) return;

  state.isSaving = true;
  updateActionButtons();
  updateStatusMessage();

  try {
    await saveConfig(state.draft);
    state.config = JSON.parse(JSON.stringify(state.draft)); // Deep clone
    state.hasChanges = false;
    state.lastSaved = new Date();

    // Trigger filtering refresh after config save so changes take effect immediately
    try {
      await refreshSnapshot();
      // Reload stats to show updated results
      setTimeout(() => loadStats(), 500);
    } catch (refreshError) {
      console.warn("Auto-refresh after save failed:", refreshError);
    }

    Utils.showToast({
      type: "success",
      title: "Configuration Saved",
      message: "Filtering settings saved and snapshot refreshed",
    });
  } catch (error) {
    console.error("Failed to save config:", error);
    Utils.showToast({
      type: "error",
      title: "Save Failed",
      message: error.message || "Failed to save filtering configuration",
    });
  } finally {
    state.isSaving = false;
    updateActionButtons();
    updateStatusMessage();
  }
}

function handleResetConfig() {
  if (!state.hasChanges) return;

  state.draft = JSON.parse(JSON.stringify(state.config)); // Deep clone
  state.hasChanges = false;
  render(); // Need full render to restore original values
  Utils.showToast({
    type: "info",
    title: "Changes Reset",
    message: "Configuration restored to last saved state",
  });
}

async function handleRefreshSnapshot() {
  if (state.isRefreshing) return;

  state.isRefreshing = true;
  updateActionButtons();
  updateStatusMessage();

  try {
    await refreshSnapshot();
    // Reload stats after refresh
    setTimeout(() => loadStats(), 1000);
  } catch (error) {
    console.error("Failed to refresh snapshot:", error);
    Utils.showToast({
      type: "error",
      title: "Refresh Failed",
      message: error.message || "Failed to refresh filtering snapshot",
    });
  } finally {
    state.isRefreshing = false;
    updateActionButtons();
    updateStatusMessage();
  }
}

function handleExportConfig() {
  const json = JSON.stringify(state.draft, null, 2);
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `filtering-config-${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
  Utils.showToast({
    type: "success",
    title: "Configuration Exported",
    message: "Filtering settings saved to file",
  });
}

function handleImportConfig() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".json";
  input.addEventListener("change", async (e) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      const text = await file.text();
      const imported = JSON.parse(text);
      const current = JSON.parse(JSON.stringify(state.draft ?? {}));
      state.draft = deepMerge(current, imported);
      checkForChanges();
      render();
      Utils.showToast({
        type: "success",
        title: "Configuration Imported",
        message: "Filtering settings loaded from file",
      });
    } catch (error) {
      console.error("Failed to import config:", error);
      Utils.showToast({
        type: "error",
        title: "Import Failed",
        message: error.message || "Failed to import configuration - invalid file format",
      });
    }
  });
  input.click();
}

// ============================================================================
// DATA LOADING
// ============================================================================

async function loadConfig() {
  try {
    const [response, metadataResponse] = await Promise.all([fetchConfig(), fetchConfigMetadata()]);
    state.config = response.data || response;
    state.metadata = (metadataResponse.data || metadataResponse).filtering || {};
    state.draft = JSON.parse(JSON.stringify(state.config)); // Deep clone for nested structures
    state.hasChanges = false;
  } catch (error) {
    console.error("Failed to load config:", error);
    Utils.showToast({
      type: "error",
      title: "Load Failed",
      message: error.message || "Failed to load filtering configuration",
    });
  }
}

async function loadStats() {
  try {
    const [statsResponse, rejectionStatsResponse] = await Promise.all([
      fetchStats(),
      fetchRejectionStats(),
    ]);
    state.stats = statsResponse.data || statsResponse;
    state.rejectionStats = rejectionStatsResponse.data || rejectionStatsResponse;

    // Also load analytics if on analytics tab
    if (state.activeTab === "analytics") {
      await loadAnalytics();
    }
  } catch (error) {
    console.error("Failed to load stats:", error);
    // Don't show toast for stats errors (non-critical)
  }
}

async function loadAnalytics() {
  // Increment request ID to track this request and prevent race conditions
  const thisRequestId = ++state.analyticsRequestId;

  try {
    const response = await fetchAnalytics();

    // Check if this is still the latest request (prevent stale data from overwriting)
    if (thisRequestId !== state.analyticsRequestId) {
      return; // A newer request was made, discard this response
    }

    const key = JSON.stringify(response);
    if (key === _lastAnalyticsKey) return; // data unchanged — skip redraw, scroll preserved
    _lastAnalyticsKey = key;

    state.analytics = response.data || response;
    // Preserve scroll position on incremental updates; only reset on explicit user action
    updateConfigPanels({ preserveScroll: true });
  } catch (error) {
    // Only log error if this is still the active request
    if (thisRequestId === state.analyticsRequestId) {
      console.error("Failed to load analytics:", error);
      state.analytics = null;
    }
  }
}

async function loadData() {
  await Promise.all([loadConfig(), loadStats()]);

  // If active tab is analytics or explorer, load analytics data
  if (state.activeTab === "analytics" || state.activeTab === "explorer") {
    await loadAnalytics();
  }

  render();
}

// ============================================================================
// LIFECYCLE
// ============================================================================

let poller = null;
// Hash guards — prevent DOM rebuilds when polled data is unchanged
let _lastInfoBarKey = null;
let _lastAnalyticsKey = null;
let _lastStatusKey = null;

export function createLifecycle() {
  return {
    async init() {
      console.log("[Filtering] Initializing");

      // Initialize renderers with state and dependencies
      renderers = createFilteringRenderers({ state, $, Utils, requestManager });

      // Validate time range state consistency - if preset is "custom" but times are null, reset to "all"
      if (
        state.timeRange.preset === "custom" &&
        (!state.timeRange.startTime || !state.timeRange.endTime)
      ) {
        console.warn("[Filtering] Inconsistent time range state detected, resetting to 'all'");
        state.timeRange.preset = "all";
        state.timeRange.startTime = null;
        state.timeRange.endTime = null;
        AppState.save("filtering_timeRangePreset", "all");
        AppState.save("filtering_timeRangeStart", null);
        AppState.save("filtering_timeRangeEnd", null);
      }

      await loadData();
    },

    activate(ctx) {
      console.log("[Filtering] Activating");

      // Initialize tab bar with global container
      if (!tabBar) {
        tabBar = new TabBar({
          container: "#subTabsContainer",
          tabs: FILTER_TABS,
          defaultTab: state.activeTab,
          stateKey: TABBAR_STATE_KEY,
          pageName: "filtering",
          onChange: async (tabId) => {
            state.activeTab = tabId;
            AppState.save("filtering_activeTab", tabId);
            updateSearchBar();

            // Load analytics data if switching to analytics or explorer tab
            if ((tabId === "analytics" || tabId === "explorer") && !state.analytics) {
              await loadAnalytics();
            }

            updateConfigPanels({ scrollTop: 0 });
          },
        });
        TabBarManager.register("filtering", tabBar);
        ctx.manageTabBar(tabBar);
        tabBar.show();

        // Sync state.activeTab with TabBar's restored state (from server or URL)
        const active = tabBar.getActiveTab();
        if (active && active !== state.activeTab) {
          state.activeTab = active;
          AppState.save("filtering_activeTab", active);
        }
      } else {
        // Re-register deactivate cleanup (cleanups are cleared after each deactivate)
        // and force-show tab bar to handle race conditions with TabBarManager
        ctx.manageTabBar(tabBar);
        // Ensure registry is up to date in case of page reload/HMR
        TabBarManager.register("filtering", tabBar);

        // Force show in next frame to ensure DOM is ready and override any race conditions
        requestAnimationFrame(() => {
          if (tabBar) tabBar.show({ force: true });
        });
      }

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(
            async () => {
              await loadStats();
              updateInfoBar();
              // Keep the status tab live — only re-render when data actually changed
              if (state.activeTab === "status") {
                const statusKey = JSON.stringify({ s: state.stats, r: state.rejectionStats });
                if (statusKey !== _lastStatusKey) {
                  _lastStatusKey = statusKey;
                  updateConfigPanels({ preserveScroll: true });
                }
              }
            },
            { label: "Filtering Stats", intervalMs: 5000 }
          )
        );
      }

      poller.start();
    },

    deactivate() {
      console.log("[Filtering] Deactivating");
      if (poller) {
        poller.stop();
      }
    },

    dispose() {
      console.log("[Filtering] Disposing");

      // Clean up all tracked event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

      if (poller) {
        poller.stop();
        poller = null;
      }
      tabBar = null; // Cleaned up automatically by ctx.manageTabBar
      TabBarManager.unregister("filtering");
      state.initialized = false;
      globalHandlersBound = false;
      _lastInfoBarKey = null;
      _lastAnalyticsKey = null;
      _lastStatusKey = null;
    },
  };
}

// Expose for inline handlers
window.filteringPage = {
  explorerPage: 0,
  explorerLimit: 50,
  currentReason: null,
  currentReasonLabel: null,
  tokenSearchQuery: "",

  debouncedFilterTokens: null,

  // Time range filter functions
  setTimeRangePreset: (preset) => {
    setTimeRangePreset(preset);
  },

  toggleCustomRange: () => {
    if (state.timeRange.preset === "custom") {
      // Already in custom mode, toggle off to all
      setTimeRangePreset("all");
    } else {
      // Switch to custom mode
      state.timeRange.preset = "custom";
      AppState.save("filtering_timeRangePreset", "custom");
      render();
    }
  },

  updateCustomRange: () => {
    // Just update state without triggering refresh yet
    const startInput = document.getElementById("time-range-start");
    const endInput = document.getElementById("time-range-end");
    if (startInput && startInput.value) {
      state.timeRange.startTime = Math.floor(new Date(startInput.value).getTime() / 1000);
    }
    if (endInput && endInput.value) {
      state.timeRange.endTime = Math.floor(new Date(endInput.value).getTime() / 1000);
    }
  },

  applyCustomRange: async () => {
    const startInput = document.getElementById("time-range-start");
    const endInput = document.getElementById("time-range-end");

    // Parse dates
    let startTime = null;
    let endTime = null;

    if (startInput && startInput.value) {
      startTime = Math.floor(new Date(startInput.value).getTime() / 1000);
    }
    if (endInput && endInput.value) {
      endTime = Math.floor(new Date(endInput.value).getTime() / 1000);
    }

    // Validation: ensure both dates are provided
    if (!startTime || !endTime) {
      Utils.showToast("Please select both start and end dates", "error");
      return;
    }

    // Validation: start must be before end
    if (startTime >= endTime) {
      Utils.showToast("Start time must be before end time", "error");
      return;
    }

    // Validation: end cannot be in the future (with 1 minute tolerance)
    const now = Math.floor(Date.now() / 1000);
    if (endTime > now + 60) {
      Utils.showToast("End time cannot be in the future", "error");
      return;
    }

    state.timeRange.startTime = startTime;
    state.timeRange.endTime = endTime;
    state.timeRange.preset = "custom";

    // Persist all state
    AppState.save("filtering_timeRangePreset", "custom");
    AppState.save("filtering_timeRangeStart", startTime);
    AppState.save("filtering_timeRangeEnd", endTime);

    // Show loading state
    state.isLoadingAnalytics = true;
    render();

    try {
      await loadAnalytics();
    } finally {
      // Always clear loading state, even on error
      state.isLoadingAnalytics = false;
      render();
    }
  },

  refreshAnalytics: async () => {
    // Simply load analytics and re-render - no inline button spinner needed
    // The footer refresh button and isLoadingAnalytics state handle UX
    state.isLoadingAnalytics = true;
    render();

    try {
      await loadAnalytics();
      if (window.filteringPage.currentReason) {
        window.filteringPage.loadExplorer(window.filteringPage.explorerPage);
      } else {
        const container = document.getElementById("explorer-detail-view");
        if (container && state.analytics) {
          container.innerHTML = renderers.renderExplorerDashboard(state.analytics);
        }
      }
    } finally {
      state.isLoadingAnalytics = false;
      render();
    }
  },

  debouncedFilterExplorerTree: null,

  filterExplorerTree: (query) => {
    if (!window.filteringPage.debouncedFilterExplorerTree) {
      window.filteringPage.debouncedFilterExplorerTree = Utils.debounce((q) => {
        const lowerQ = q.toLowerCase();
        let anyVisible = false;
        document.querySelectorAll(".tree-category").forEach((cat) => {
          let hasVisibleReason = false;
          const reasons = cat.querySelectorAll(".tree-reason");
          reasons.forEach((r) => {
            const label = r.getAttribute("data-label") || "";
            const visible = lowerQ === "" || label.includes(lowerQ);
            r.style.display = visible ? "flex" : "none";
            if (visible) hasVisibleReason = true;
          });

          cat.style.display = hasVisibleReason ? "block" : "none";
          if (hasVisibleReason) anyVisible = true;

          const reasonsList = cat.querySelector(".tree-reasons");
          const toggle = cat.querySelector(".tree-toggle");
          if (lowerQ !== "" && hasVisibleReason) {
            if (reasonsList) reasonsList.style.display = "block";
            if (toggle) toggle.style.transform = "rotate(180deg)";
          } else if (lowerQ === "" && reasonsList) {
            // Re-collapse on clear (preserve last explicit toggle state isn't tracked, so collapse all)
            reasonsList.style.display = "none";
            if (toggle) toggle.style.transform = "rotate(0deg)";
          }
        });
        const emptyEl = document.getElementById("explorer-tree-empty");
        if (emptyEl) emptyEl.style.display = lowerQ !== "" && !anyVisible ? "block" : "none";
      }, 150);
    }
    window.filteringPage.debouncedFilterExplorerTree(query);
  },

  selectSummary: () => {
    window.filteringPage.currentReason = null;
    window.filteringPage.currentReasonLabel = null;

    document.querySelectorAll(".tree-reason").forEach((el) => el.classList.remove("active"));
    const navOverview = document.querySelector(".explorer-nav-overview");
    if (navOverview) navOverview.classList.add("active");

    const container = document.getElementById("explorer-detail-view");
    if (container && state.analytics) {
      container.innerHTML = renderers.renderExplorerDashboard(state.analytics);
    }
  },

  toggleCategory: (category) => {
    const el = document.getElementById(`reasons-${category}`);
    const toggle = document.getElementById(`toggle-${category}`);
    if (el) {
      const isHidden = el.style.display === "none";
      el.style.display = isHidden ? "block" : "none";
      if (toggle) {
        toggle.style.transform = isHidden ? "rotate(180deg)" : "rotate(0deg)";
      }
    }
  },

  selectReason: (reason, label) => {
    // Toggle deselect: clicking the active reason returns to overview
    if (window.filteringPage.currentReason === reason) {
      window.filteringPage.selectSummary();
      return;
    }

    document.querySelectorAll(".tree-reason").forEach((el) => el.classList.remove("active"));
    const navOverview = document.querySelector(".explorer-nav-overview");
    if (navOverview) navOverview.classList.remove("active");

    const activeEl = document.getElementById(`reason-${reason}`);
    if (activeEl) {
      activeEl.classList.add("active");
      // Ensure category is expanded
      const category = activeEl.closest(".tree-reasons");
      if (category && category.style.display === "none") {
        category.style.display = "block";
        const catId = category.id.replace("reasons-", "");
        const toggle = document.getElementById(`toggle-${catId}`);
        if (toggle) toggle.style.transform = "rotate(180deg)";
      }
    }

    window.filteringPage.currentReason = reason;
    window.filteringPage.currentReasonLabel = label;
    window.filteringPage.tokenSearchQuery = "";
    window.filteringPage.loadExplorer(0);
  },

  filterTokens: (query) => {
    if (!window.filteringPage.debouncedFilterTokens) {
      window.filteringPage.debouncedFilterTokens = Utils.debounce((q) => {
        window.filteringPage.tokenSearchQuery = q;
        window.filteringPage.loadExplorer(0);
      }, 300);
    }
    window.filteringPage.debouncedFilterTokens(query);
  },

  firstPage: () => {
    if (window.filteringPage.explorerPage > 0) {
      window.filteringPage.loadExplorer(0);
    }
  },

  prevPage: () => {
    if (window.filteringPage.explorerPage > 0) {
      window.filteringPage.loadExplorer(window.filteringPage.explorerPage - 1);
    }
  },

  nextPage: () => {
    window.filteringPage.loadExplorer(window.filteringPage.explorerPage + 1);
  },

  lastPage: async () => {
    // Go forward in large steps until we hit the end
    const limit = window.filteringPage.explorerLimit;
    const reason = window.filteringPage.currentReason;
    if (!reason) return;

    // Estimate last page by fetching with large offset
    let testPage = window.filteringPage.explorerPage + 100;
    try {
      const url = `/api/filtering/rejected-tokens?limit=${limit}&offset=${testPage * limit}&reason=${encodeURIComponent(reason)}`;
      const response = await fetch(url);
      if (!response.ok) throw new Error("Failed to fetch tokens");
      const tokens = await response.json();

      if (tokens.length === 0) {
        // Binary search for actual last page
        let low = window.filteringPage.explorerPage;
        let high = testPage;
        while (low < high - 1) {
          const mid = Math.floor((low + high) / 2);
          const checkUrl = `/api/filtering/rejected-tokens?limit=${limit}&offset=${mid * limit}&reason=${encodeURIComponent(reason)}`;
          const checkRes = await fetch(checkUrl);
          if (!checkRes.ok) throw new Error("Failed to fetch tokens");
          const checkTokens = await checkRes.json();
          if (checkTokens.length === 0) {
            high = mid;
          } else if (checkTokens.length < limit) {
            // This is the last page
            window.filteringPage.loadExplorer(mid);
            return;
          } else {
            low = mid;
          }
        }
        window.filteringPage.loadExplorer(low);
      } else if (tokens.length < limit) {
        // testPage is the last page
        window.filteringPage.loadExplorer(testPage);
      } else {
        // Need to go further - just load testPage for now
        window.filteringPage.loadExplorer(testPage);
      }
    } catch {
      // Fallback: just go forward 10 pages
      window.filteringPage.loadExplorer(window.filteringPage.explorerPage + 10);
    }
  },

  exportCsv: () => {
    const reason = window.filteringPage.currentReason;
    if (!reason) return;

    let url = `/api/filtering/export-rejected-tokens?reason=${encodeURIComponent(reason)}`;
    window.open(url, "_blank");
  },

  loadExplorer: async (page) => {
    window.filteringPage.explorerPage = page;
    const container = document.getElementById("explorer-detail-view");
    const reason = window.filteringPage.currentReason;
    const searchQuery = window.filteringPage.tokenSearchQuery;

    if (!container || !reason) return;

    // Initial render — just table + pagination, no header
    if (page === 0 && !container.querySelector(".explorer-table-wrapper")) {
      container.innerHTML = `
        <div class="explorer-table-wrapper">
          <div class="explorer-empty-state">Loading...</div>
        </div>
        <div class="pagination-controls">
          <button class="page-btn" onclick="window.filteringPage.firstPage()" disabled title="First"><i class="icon-chevrons-left"></i></button>
          <button class="page-btn" onclick="window.filteringPage.prevPage()" disabled title="Previous"><i class="icon-chevron-left"></i></button>
          <span class="page-info">Page ${page + 1}</span>
          <button class="page-btn" onclick="window.filteringPage.nextPage()" disabled title="Next"><i class="icon-chevron-right"></i></button>
          <button class="page-btn" onclick="window.filteringPage.lastPage()" disabled title="Last"><i class="icon-chevrons-right"></i></button>
        </div>
      `;
    } else {
      const wrapper = container.querySelector(".explorer-table-wrapper");
      if (wrapper)
        wrapper.innerHTML = '<div class="loading-spinner small explorer-loading-full"></div>';
    }

    try {
      let url = `/api/filtering/rejected-tokens?limit=${window.filteringPage.explorerLimit}&offset=${page * window.filteringPage.explorerLimit}&reason=${encodeURIComponent(reason)}`;
      if (searchQuery) {
        url += `&search=${encodeURIComponent(searchQuery)}`;
      }

      const response = await fetch(url);
      if (!response.ok) throw new Error("Failed to fetch tokens");

      const tokens = await response.json();
      const wrapper = container.querySelector(".explorer-table-wrapper");

      if (!wrapper) return;

      if (tokens.length === 0) {
        wrapper.innerHTML = `
          <div class="explorer-empty-state">
            No tokens found${searchQuery ? " matching filter" : ""}
          </div>`;
        // Update pagination
        const pagination = container.querySelector(".pagination-controls");
        if (pagination) {
          pagination.innerHTML = `
            <button class="page-btn" disabled><i class="icon-chevrons-left"></i></button>
            <button class="page-btn" disabled><i class="icon-chevron-left"></i></button>
            <span class="page-info">No results</span>
            <button class="page-btn" disabled><i class="icon-chevron-right"></i></button>
            <button class="page-btn" disabled><i class="icon-chevrons-right"></i></button>
          `;
        }
        return;
      }

      let html = `
        <table class="reasons-table">
          <thead>
            <tr>
              <th>Token</th>
              <th>Source</th>
              <th class="text-end">Time</th>
            </tr>
          </thead>
          <tbody>
      `;

      html += tokens
        .map((t) => {
          const src = t.image_url;
          const logo = src
            ? `<img class="token-logo" alt="" src="${Utils.escapeHtml(src)}" loading="lazy" />`
            : '<div class="token-logo token-logo-placeholder">?</div>';
          const sym = Utils.escapeHtml(t.symbol || "—");
          const name = Utils.escapeHtml(t.name || "Unknown");

          return `
        <tr>
          <td>
            <div class="token-info-cell">
              <div class="token-logo-wrapper">
                ${logo}
              </div>
              <div class="token-details">
                <div class="token-symbol">${sym}</div>
                <div class="token-name">${name}</div>
              </div>
              <div class="token-actions">
                <button class="btn-icon small" onclick="Utils.copyToClipboard('${t.mint}')" title="Copy Mint">
                  <i class="icon-copy"></i>
                </button>
                <a href="https://dexscreener.com/solana/${t.mint}" target="_blank" class="btn-icon small" title="DexScreener">
                  <i class="icon-external-link"></i>
                </a>
              </div>
            </div>
          </td>
          <td>
            <span class="source-badge ${t.source.toLowerCase()}">${Utils.escapeHtml(t.source)}</span>
          </td>
          <td class="table-time-cell">
            ${Utils.formatTimeAgo(new Date(t.rejected_at))}
          </td>
        </tr>
      `;
        })
        .join("");

      html += "</tbody></table>";
      wrapper.innerHTML = html;

      // Update pagination
      const pagination = container.querySelector(".pagination-controls");
      if (pagination) {
        const hasMore = tokens.length >= window.filteringPage.explorerLimit;
        pagination.innerHTML = `
          <button class="page-btn" onclick="window.filteringPage.firstPage()" ${page === 0 ? "disabled" : ""} title="First"><i class="icon-chevrons-left"></i></button>
          <button class="page-btn" onclick="window.filteringPage.prevPage()" ${page === 0 ? "disabled" : ""} title="Previous"><i class="icon-chevron-left"></i></button>
          <span class="page-info">Page ${page + 1}</span>
          <button class="page-btn" onclick="window.filteringPage.nextPage()" ${!hasMore ? "disabled" : ""} title="Next"><i class="icon-chevron-right"></i></button>
          <button class="page-btn" onclick="window.filteringPage.lastPage()" ${!hasMore ? "disabled" : ""} title="Last"><i class="icon-chevrons-right"></i></button>
        `;
      }
    } catch (err) {
      console.error("Failed to load explorer:", err);
      const wrapper = container.querySelector(".explorer-table-wrapper");
      if (wrapper) {
        wrapper.innerHTML = `<div class="error-message p-lg">Failed to load tokens: ${err.message}</div>`;
      }
    }
  },
};

// Register the page with the lifecycle system
registerPage("filtering", createLifecycle());
