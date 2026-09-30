import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { playToggleOn, playToggleOff, playSuccess, playError } from "../core/sounds.js";
import { ChatWidget } from "../core/chat_widget.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { formatLatencyMs, formatPercentValue } from "../core/format.js";
import { LLM_PROVIDER_LABELS } from "../ui/llm_provider.js";

// Import tab modules
import { createProvidersTab } from "./assistant/providers_tab.js";
import { createInstructionsTab } from "./assistant/instructions_tab.js";
import { createAutomationTab } from "./assistant/automation_tab.js";
import {
  ANALYSIS_CONFIG_FIELDS,
  SLIDER_SUFFIX,
  readAnalysisConfigForm,
  applyAnalysisConfigForm,
  sliderLabelId,
} from "./assistant/config_contract.js";

// Constants
const DEFAULT_TAB = "chat";
const ASSISTANT_STATE_KEY = "assistant.activeTab";
const ASSISTANT_TAB_IDS = new Set([
  "chat",
  "stats",
  "providers",
  "instructions",
  "automation",
  "history",
  "testing",
  "settings",
]);

// Message key of each tab label.
const ASSISTANT_TAB_LABELS = Object.freeze({
  chat: "assistant-tab-chat",
  stats: "assistant-tab-overview",
  providers: "assistant-tab-providers",
  instructions: "assistant-tab-instructions",
  automation: "assistant-tab-automation",
  history: "assistant-tab-history",
  testing: "assistant-tab-testing",
  settings: "assistant-tab-settings",
});

// Verdict ids written by src/llm_analysis/engine.rs; `allow` is the alias the
// overview feed may carry.
const ANALYSIS_DECISION_LABELS = Object.freeze({
  allow: "assistant-decision-allow",
  pass: "assistant-decision-pass",
  reject: "assistant-decision-reject",
  buy: "assistant-decision-buy",
  sell: "assistant-decision-sell",
  hold: "assistant-decision-hold",
});

// `RiskLevel` ids of src/llm_analysis/types.rs.
const ANALYSIS_RISK_LABELS = Object.freeze({
  low: "assistant-risk-low",
  medium: "assistant-risk-medium",
  high: "assistant-risk-high",
  critical: "assistant-risk-critical",
});

function buildAssistantTabs() {
  return [...ASSISTANT_TAB_IDS].map((id) => ({
    id,
    label: I18n.label(ASSISTANT_TAB_LABELS, id),
  }));
}

/** Label for an id the model or history API reported; empty values show a dash. */
function analysisLabel(map, value) {
  if (!value) return "—";
  const id = String(value).toLowerCase();
  return I18n.label(map, Object.hasOwn(map, id) ? id : value);
}

function percentText(value, fallback) {
  return formatPercentValue(value, { decimals: 0, includeSign: false, fallback });
}

// Decisions per History page. Sent as `per_page` and used to compute the page
// count, so one constant keeps the request and the pager agreeing.
const HISTORY_PAGE_SIZE = 20;

function createLifecycle() {
  // Component references
  let statusPoller = null;
  let providersPoller = null;
  let cachePoller = null;
  let chatPoller = null;
  let automationPoller = null;
  let _chatWidget = null;
  let subTabBar = null;

  // Hash guards — skip re-render when polled data is unchanged
  let _lastDecisionsKey = null;

  // Event cleanup tracking
  const eventCleanups = [];

  // Page state
  const state = {
    currentTab: DEFAULT_TAB,
    aiStatus: null,
    providers: [],
    config: null,
    cacheStats: null,
    templates: [],
    historyPage: 1,
    historyTotal: 0,
    instructions: [], // Store instructions for drag-drop
    draggedItem: null, // Track dragged instruction
    automationTasks: [],
    automationRuns: [],
    automationStats: null,
  };

  // Store API functions for external access
  const api = {};

  // ============================================================================
  // Helper Functions
  // ============================================================================

  /**
   * Add tracked event listener for cleanup
   */
  function addTrackedListener(element, event, handler) {
    if (!element) return;
    element.addEventListener(event, handler);
    eventCleanups.push(() => element.removeEventListener(event, handler));
  }

  // ============================================================================
  // Tab Management
  // ============================================================================

  /**
   * Switch between main tabs
   */
  function switchTab(tabId, { load = true } = {}) {
    // Stop all pollers to prevent memory leaks
    if (statusPoller) statusPoller.stop();
    if (providersPoller) providersPoller.stop();
    if (cachePoller) cachePoller.stop();
    if (chatPoller) chatPoller.stop();
    if (automationPoller) automationPoller.stop();

    // Hide all panels
    const allPanels = $$(".assistant-panel-content");
    allPanels.forEach((panel) => {
      panel.classList.remove("active");
    });

    // Show the selected panel
    const selectedPanel = $(`#${tabId}-panel`);
    if (selectedPanel) {
      selectedPanel.classList.add("active");
    }

    // Initialization uses this function to paint the restored panel before the
    // page is active. Its data load is started once activate() owns the pollers.
    if (!load) return;

    // Load data for the tab and start appropriate poller
    if (tabId === "stats") {
      loadAiStatus();
      if (statusPoller) statusPoller.start();
    } else if (tabId === "providers") {
      providersTab.loadProviders();
      if (providersPoller) providersPoller.start();
    } else if (tabId === "settings") {
      loadConfig();
      loadCacheStats();
      if (cachePoller) cachePoller.start();
    } else if (tabId === "instructions") {
      instructionsTab.loadInstructions();
      instructionsTab.loadTemplates();
    } else if (tabId === "history") {
      loadHistory(1);
    } else if (tabId === "chat") {
      loadSessions();
      if (chatPoller) chatPoller.start();
    } else if (tabId === "automation") {
      automationTab.loadAutomationTasks();
      automationTab.loadAutomationRuns();
      automationTab.loadAutomationStats();
      if (automationPoller) automationPoller.start();
    }
  }

  // ============================================================================
  // Stats Tab
  // ============================================================================

  /**
   * Load model-feature status and update the UI
   */
  async function loadAiStatus() {
    try {
      const response = await fetch("/api/llm-analysis/status");
      if (!response.ok) throw new Error("Failed to fetch model-feature status");

      const data = await response.json();
      state.aiStatus = data;

      updateStatusBar(data);
      updateMetrics(data);
      updateRecentDecisions(data.recent_decisions || []);
    } catch (error) {
      console.error("[Assistant] Failed to load model-feature status:", error);
      Utils.showToast({
        key: "assistant-load",
        type: "error",
        title: I18n.t("assistant-status-load-failed"),
      });
    }
  }

  /**
   * Update status bar
   */
  function updateStatusBar(data) {
    const statusBar = $("#assistant-status-bar");
    const statusText = $("#assistant-status-text");
    const toggle = $("#stats-assistant-toggle");
    const toggleLabel = $("#stats-toggle-label");

    if (!statusBar || !toggle) return;

    const enabled = data.enabled || false;

    // Update status bar state
    statusBar.setAttribute("data-status", enabled ? "enabled" : "disabled");

    // Update status text
    if (statusText) {
      statusText.textContent = enabled
        ? I18n.t("assistant-status-active")
        : I18n.t("assistant-status-disabled");
    }

    // Update toggle
    toggle.checked = enabled;
    toggle.disabled = false;
    if (toggleLabel) {
      toggleLabel.textContent = enabled
        ? I18n.t("assistant-toggle-on")
        : I18n.t("assistant-toggle-off");
    }
  }

  /**
   * Update metrics cards
   */
  function updateMetrics(data) {
    const metrics = data.metrics || {};

    // Total Evaluations
    const totalEval = $("#metric-total-evaluations");
    if (totalEval) {
      totalEval.textContent = Utils.formatNumber(metrics.total_evaluations || 0, 0);
    }

    // Cache Hit Rate
    const cacheHitRate = $("#metric-cache-hit-rate");
    if (cacheHitRate) {
      const rate = metrics.cache_hit_rate || 0;
      cacheHitRate.textContent = percentText(Math.round(rate * 100));
    }

    // Avg Latency
    const avgLatency = $("#metric-avg-latency");
    if (avgLatency) {
      avgLatency.textContent = formatLatencyMs(Math.round(metrics.avg_response_time_ms || 0));
    }

    // Active Providers
    const providers = $("#metric-providers");
    if (providers) {
      const active = metrics.active_providers || 0;
      const total = metrics.total_providers || 10;
      providers.textContent = `${active} / ${total}`;
    }
  }

  /**
   * Update recent decisions feed
   */
  function updateRecentDecisions(decisions) {
    const key = JSON.stringify(decisions);
    if (key === _lastDecisionsKey) return;
    _lastDecisionsKey = key;

    const container = $("#recent-decisions-container");
    if (!container) return;

    if (!decisions || decisions.length === 0) {
      container.innerHTML = `<div class="empty-state">${Utils.escapeHtml(I18n.t("assistant-decisions-empty"))}</div>`;
      return;
    }

    container.innerHTML = decisions
      .map((d) => {
        const decision = (d.decision || "").toLowerCase();
        const state = decision === "allow" ? "allow" : decision === "reject" ? "reject" : "neutral";
        const icon = state === "allow" ? "circle-check" : state === "reject" ? "circle-x" : "info";
        const time = d.timestamp
          ? Utils.formatTimeAgo(new Date(d.timestamp))
          : I18n.t("format-just-now");
        const confidence = percentText(Math.round((d.confidence || 0) * 100));
        const latency = formatLatencyMs(Math.round(d.latency_ms || 0));
        const latencyTitle = Utils.escapeHtml(I18n.attr("assistant-decision-latency", "title"));
        const confidenceTitle = Utils.escapeHtml(
          I18n.attr("assistant-decision-confidence", "title")
        );

        return `
          <div class="decision-card" data-decision="${state}">
            <div class="decision-icon"><i class="icon-${icon}"></i></div>
            <div class="decision-main">
              <div class="decision-top">
                <span class="decision-token">${Utils.escapeHtml(d.token || I18n.t("format-not-available"))}</span>
                ${d.context ? `<span class="decision-context">${Utils.escapeHtml(d.context)}</span>` : ""}
              </div>
              <div class="decision-time"><i class="icon-clock"></i> ${Utils.escapeHtml(time)}</div>
            </div>
            <div class="decision-side">
              <span class="decision-result">${Utils.escapeHtml(analysisLabel(ANALYSIS_DECISION_LABELS, d.decision))}</span>
              <div class="decision-stats">
                <span title="${latencyTitle}"><i class="icon-zap"></i> ${latency}</span>
                <span title="${confidenceTitle}"><i class="icon-activity"></i> ${confidence}</span>
              </div>
            </div>
          </div>
        `;
      })
      .join("");
  }

  /**
   * Toggle the shared LLM master switch
   */
  async function toggleAiEnabled(enabled) {
    try {
      // The master enable switch is owned by /api/llm/config, not the analysis endpoint.
      const response = await fetch("/api/llm/config", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
      });

      if (!response.ok) throw new Error("Failed to update model-feature status");

      enabled ? playToggleOn() : playToggleOff();
      Utils.showToast({
        type: "success",
        title: enabled
          ? I18n.t("assistant-toggle-enabled-title")
          : I18n.t("assistant-toggle-disabled-title"),
        message: enabled
          ? I18n.t("assistant-toggle-enabled-message")
          : I18n.t("assistant-toggle-disabled-message"),
      });

      await loadAiStatus();
    } catch (error) {
      console.error("[Assistant] Failed to toggle model features:", error);
      playError();
      Utils.showToast({ type: "error", title: I18n.t("assistant-toggle-failed") });

      // Revert toggle
      const toggle = $("#stats-assistant-toggle");
      if (toggle) toggle.checked = !enabled;
    }
  }

  // ============================================================================
  // Settings Tab - Config Management
  // ============================================================================

  /**
   * Load configuration
   */
  async function loadConfig() {
    try {
      // Master fields (enabled / default_provider) are owned by /api/llm/config;
      // analysis behaviour by /api/llm-analysis/config. Fetch both and merge.
      const [analysisRes, masterRes] = await Promise.all([
        fetch("/api/llm-analysis/config"),
        fetch("/api/llm/config"),
      ]);
      if (!analysisRes.ok) throw new Error("Failed to fetch analysis config");
      if (!masterRes.ok) throw new Error("Failed to fetch LLM config");

      const analysis = await analysisRes.json();
      const master = await masterRes.json();
      const data = {
        ...analysis,
        enabled: master.enabled,
        default_provider: master.default_provider,
      };
      state.config = data;

      updateConfigForm(data);
    } catch (error) {
      console.error("[Assistant] Failed to load config:", error);
      Utils.showToast({
        key: "assistant-load",
        type: "error",
        title: I18n.t("assistant-config-load-failed"),
      });
    }
  }

  /**
   * Update configuration form.
   *
   * `enabled` / `default_provider` come from `/api/llm/config`; every other
   * field is the flat `AnalysisConfigResponse` from `/api/llm-analysis/config`
   * and is applied through the shared wire contract so the form can never drift
   * from the endpoint's key names or scalar shapes.
   */
  function updateConfigForm(config) {
    // Master Control
    const enabledToggle = $("#setting-enabled");
    if (enabledToggle) enabledToggle.checked = config.enabled || false;

    const defaultProvider = $("#setting-default-provider");
    if (defaultProvider) {
      // Populate provider options
      defaultProvider.replaceChildren(
        new window.Option(I18n.t("assistant-provider-select"), ""),
        ...Object.keys(LLM_PROVIDER_LABELS).map(
          (id) => new window.Option(I18n.label(LLM_PROVIDER_LABELS, id), id)
        )
      );
      defaultProvider.value = config.default_provider || "";
    }

    // Analysis behaviour — flat contract, one control per wire key.
    applyAnalysisConfigForm(
      config,
      (id) => $(`#${id}`),
      (id, text) => {
        const label = $(`#${sliderLabelId(id)}`);
        if (label) label.textContent = text;
      }
    );
  }

  // ============================================================================
  // Settings Tab - Cache Stats Only (rest delegated to settings module)
  // ============================================================================

  /**
   * Load cache statistics
   */
  async function loadCacheStats() {
    try {
      const response = await fetch("/api/llm-analysis/cache/stats");
      if (!response.ok) throw new Error("Failed to fetch cache stats");

      const data = await response.json();
      state.cacheStats = data;

      updateCacheStats(data);
    } catch (error) {
      console.error("[Assistant] Failed to load cache stats:", error);
    }
  }

  /**
   * Update cache stats display
   */
  function updateCacheStats(stats) {
    const cacheSize = $("#cache-size");
    const cacheMemory = $("#cache-memory");

    if (cacheSize) cacheSize.textContent = Utils.formatNumber(stats.total_entries || 0, 0);
    if (cacheMemory) cacheMemory.textContent = Utils.formatBytes(stats.total_size_bytes || 0);
  }

  /**
   * Clear the model-analysis cache
   */
  async function clearCache() {
    const confirmed = await ConfirmationDialog.show({
      title: I18n.t("assistant-cache-clear"),
      message: I18n.t("assistant-cache-clear-message"),
      confirmText: I18n.t("assistant-cache-clear"),
      confirmClass: "danger",
    });

    if (!confirmed) return;

    try {
      const response = await fetch("/api/llm-analysis/cache/clear", { method: "POST" });
      if (!response.ok) throw new Error("Failed to clear cache");

      playSuccess();
      Utils.showToast({
        type: "success",
        title: I18n.t("assistant-cache-cleared-title"),
        message: I18n.t("assistant-cache-cleared-message"),
      });

      await loadCacheStats();
    } catch (error) {
      console.error("[Assistant] Failed to clear cache:", error);
      playError();
      Utils.showToast({ type: "error", title: I18n.t("assistant-cache-clear-failed") });
    }
  }

  /**
   * Setup settings tab handlers
   */
  function setupSettingsHandlers() {
    const clearCacheBtn = $("#clear-cache-btn");
    if (clearCacheBtn) {
      addTrackedListener(clearCacheBtn, "click", clearCache);
    }

    const saveConfigBtn = $("#save-config-btn");
    if (saveConfigBtn) {
      addTrackedListener(saveConfigBtn, "click", async () => {
        // Master enable + default provider are owned by /api/llm/config.
        const master = {
          enabled: $("#setting-enabled")?.checked || false,
          default_provider: $("#setting-default-provider")?.value || "",
        };
        // Everything else is analysis behaviour, owned by /api/llm-analysis/config:
        // a flat `UpdateAnalysisConfigRequest` carrying only the backed controls.
        const analysis = readAnalysisConfigForm((id) => $(`#${id}`));

        try {
          const [masterRes, analysisRes] = await Promise.all([
            fetch("/api/llm/config", {
              method: "PATCH",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify(master),
            }),
            fetch("/api/llm-analysis/config", {
              method: "PATCH",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify(analysis),
            }),
          ]);

          if (!masterRes.ok || !analysisRes.ok) throw new Error("Failed to save configuration");

          playSuccess();
          Utils.showToast({
            type: "success",
            title: I18n.t("assistant-config-saved-title"),
            message: I18n.t("assistant-config-saved-message"),
          });

          await loadConfig();
        } catch (error) {
          console.error("[Assistant] Failed to save config:", error);
          playError();
          Utils.showToast({ type: "error", title: I18n.t("assistant-config-save-failed") });
        }
      });
    }

    // Setup sliders
    setupSliders();
  }

  /**
   * Setup slider value displays.
   *
   * Every range control in the contract carries its integer value straight to
   * the wire, so the readout is the raw value plus the field's unit suffix — no
   * 0-1 rescaling.
   */
  function setupSliders() {
    ANALYSIS_CONFIG_FIELDS.filter(([, , kind]) => kind !== "bool").forEach(([, id, kind]) => {
      const sliderEl = $(`#${id}`);
      const displayEl = $(`#${sliderLabelId(id)}`);
      if (sliderEl && displayEl) {
        addTrackedListener(sliderEl, "input", () => {
          displayEl.textContent = `${sliderEl.value}${SLIDER_SUFFIX[kind]}`;
        });
      }
    });
  }

  /**
   * Setup testing tab handlers
   */
  function setupTestingHandlers() {
    // Testing tab uses inline handlers, no setup needed
  }

  /**
   * Setup instruction handlers
   */
  function setupInstructionHandlers() {
    const newBtn = $("#new-instruction-btn");
    if (newBtn) {
      addTrackedListener(newBtn, "click", () => instructionsTab.createInstruction());
    }

    const emptyBtn = $("#empty-add-instruction-btn");
    if (emptyBtn) {
      addTrackedListener(emptyBtn, "click", () => instructionsTab.createInstruction());
    }
  }

  // ============================================================================
  // History Tab
  // ============================================================================

  /**
   * Load history with pagination
   */
  async function loadHistory(page = 1) {
    try {
      // `per_page`, not `limit`: HistoryQuery has no `limit` field, so the old
      // parameter was silently dropped and every page asked for the default 50.
      const response = await fetch(
        `/api/llm-analysis/history?page=${page}&per_page=${HISTORY_PAGE_SIZE}`
      );
      if (!response.ok) throw new Error("Failed to load history");

      const data = await response.json();
      state.historyPage = page;
      state.historyTotal = data.total || 0;

      // The API returns `decisions`; reading `history` here meant the tab
      // rendered its empty state no matter how many evaluations existed.
      renderHistory(data.decisions || [], page, data.total || 0);
    } catch (error) {
      console.error("[Assistant] Error loading history:", error);
      const container = $("#history-list");
      if (container) {
        container.innerHTML = `<div class="empty-state">${Utils.escapeHtml(I18n.t("assistant-history-load-failed"))}</div>`;
      }
    }
  }

  /**
   * Render history list
   */
  function renderHistory(decisions, page, total) {
    const container = $("#history-list");
    if (!container) return;

    if (!decisions || decisions.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <p class="empty-text">${Utils.escapeHtml(I18n.t("assistant-history-empty"))}</p>
        </div>
      `;
      return;
    }

    const totalPages = Math.ceil(total / HISTORY_PAGE_SIZE);

    container.innerHTML = `
      <table class="history-table">
        <thead>
          <tr>
            <th>${Utils.escapeHtml(I18n.t("assistant-history-column-token"))}</th>
            <th>${Utils.escapeHtml(I18n.t("assistant-history-column-decision"))}</th>
            <th>${Utils.escapeHtml(I18n.t("assistant-history-column-confidence"))}</th>
            <th>${Utils.escapeHtml(I18n.t("assistant-history-column-risk"))}</th>
            <th>${Utils.escapeHtml(I18n.t("assistant-history-column-reasoning"))}</th>
            <th>${Utils.escapeHtml(I18n.t("assistant-history-column-model"))}</th>
            <th>${Utils.escapeHtml(I18n.t("assistant-history-column-latency"))}</th>
            <th>${Utils.escapeHtml(I18n.t("assistant-history-column-when"))}</th>
          </tr>
        </thead>
        <tbody>
          ${decisions.map(renderDecisionRow).join("")}
        </tbody>
      </table>
      ${
        totalPages > 1
          ? `
        <div class="pagination">
          <button class="btn btn-sm btn-secondary" ${page <= 1 ? "disabled" : ""}
                  onclick="window.assistantPage.loadHistory(${page - 1})">
            <i class="icon-chevron-left"></i> ${Utils.escapeHtml(I18n.t("assistant-history-previous"))}
          </button>
          <span class="pagination-info">${Utils.escapeHtml(I18n.t("assistant-history-page", { page, total: totalPages }))}</span>
          <button class="btn btn-sm btn-secondary" ${page >= totalPages ? "disabled" : ""}
                  onclick="window.assistantPage.loadHistory(${page + 1})">
            ${Utils.escapeHtml(I18n.t("common-action-next"))} <i class="icon-chevron-right"></i>
          </button>
        </div>
      `
          : ""
      }
    `;
  }

  /**
   * One decision row. `decision` is the verdict the assistant reached, which is
   * what `.decision-row.pass/.reject` colours the row by — not whether the
   * request succeeded, since a failed request produces no record at all.
   */
  function renderDecisionRow(item) {
    const allowed = item.decision === "allow" || item.decision === "pass";
    const when = item.created_at ? Utils.formatTimeAgo(new Date(item.created_at)) : "—";
    const reasoning = item.reasoning || "";

    return `
      <tr class="decision-row ${allowed ? "pass" : "reject"}">
        <td>
          <span class="token-symbol">${Utils.escapeHtml(item.symbol || I18n.t("format-unknown"))}</span>
          <span class="token-mint" dir="ltr">${Utils.formatAddressCompact(item.mint)}</span>
        </td>
        <td><span class="badge ${allowed ? "success" : "error"}">${Utils.escapeHtml(analysisLabel(ANALYSIS_DECISION_LABELS, item.decision))}</span></td>
        <td>${percentText(item.confidence, "—")}</td>
        <td>${Utils.escapeHtml(analysisLabel(ANALYSIS_RISK_LABELS, item.risk_level))}</td>
        <td class="decision-reasoning">
          <span title="${Utils.escapeHtml(reasoning)}">${Utils.escapeHtml(reasoning)}</span>
        </td>
        <td class="decision-model">${Utils.escapeHtml(item.model || item.provider || "—")}${item.cached ? ` <span class="badge secondary">${Utils.escapeHtml(I18n.t("assistant-history-cached"))}</span>` : ""}</td>
        <td class="decision-latency">${formatLatencyMs(Math.round(item.latency_ms || 0))}</td>
        <td class="decision-when">${when}</td>
      </tr>
    `;
  }

  // ============================================================================
  // Chat Tab (delegated to ChatWidget)
  // ============================================================================

  /**
   * Initialize chat widget
   */
  function initChatWidget() {
    if (_chatWidget) return;

    const container = $("#chat-panel");
    if (!container) return;

    // Clear the static HTML - ChatWidget builds its own
    container.innerHTML = "";
    _chatWidget = new ChatWidget(container, { layout: "page" });
  }

  /**
   * Load chat sessions
   */
  async function loadSessions() {
    if (!_chatWidget) initChatWidget();
    if (_chatWidget) await _chatWidget.loadSessions();
  }

  /**
   * Create new chat session
   */
  async function createSession() {
    if (!_chatWidget) initChatWidget();
    if (_chatWidget) {
      await _chatWidget.createSession();
    }
  }

  /**
   * Select chat session
   */
  async function selectSession(sessionId) {
    if (_chatWidget) {
      await _chatWidget.selectSession(sessionId);
    }
  }

  /**
   * Delete chat session
   */
  async function deleteSession(sessionId) {
    if (_chatWidget) {
      await _chatWidget.deleteSession(sessionId);
    }
  }

  /**
   * Generate session title
   */
  async function generateSessionTitle(sessionId) {
    if (_chatWidget) {
      await _chatWidget.generateSessionTitle(sessionId);
    }
  }

  /**
   * Cancel ongoing request
   */
  function cancelRequest() {
    if (_chatWidget) {
      _chatWidget.cancelRequest();
    }
  }

  function setupChatHandlers() {
    initChatWidget();
  }

  // ============================================================================
  // Tab Module Initialization
  // ============================================================================

  // Create tab module instances with shared dependencies
  const deps = { state, eventCleanups, addTrackedListener, loadConfig };
  const providersTab = createProvidersTab(deps);
  const instructionsTab = createInstructionsTab({ state, eventCleanups });
  const automationTab = createAutomationTab(deps);

  // ============================================================================
  // API Export for inline event handlers
  // ============================================================================

  // Providers Tab API
  api.setDefaultProvider = providersTab.setDefaultProvider;
  api.testProviderFromList = providersTab.testProviderFromList;
  api.configureProvider = providersTab.configureProvider;

  // Instructions Tab API
  api.createInstruction = instructionsTab.createInstruction;
  api.saveNewInstruction = instructionsTab.saveNewInstruction;
  api.toggleInstruction = instructionsTab.toggleInstruction;
  api.editInstruction = instructionsTab.editInstruction;
  api.saveEditedInstruction = instructionsTab.saveEditedInstruction;
  api.deleteInstruction = instructionsTab.deleteInstruction;
  api.duplicateInstruction = instructionsTab.duplicateInstruction;
  api.showInstructionMenu = instructionsTab.showInstructionMenu;
  api.useTemplate = instructionsTab.useTemplate;
  api.previewTemplate = instructionsTab.previewTemplate;
  api.customizeTemplate = instructionsTab.customizeTemplate;

  // Automation Tab API
  api.createAutomationTask = automationTab.createAutomationTask;
  api.saveNewAutomationTask = automationTab.saveNewAutomationTask;
  api.saveEditedAutomationTask = automationTab.saveEditedAutomationTask;
  api.updateScheduleHint = automationTab.updateScheduleHint;
  api.toggleAutomationTask = automationTab.toggleAutomationTask;
  api.runAutomationTask = automationTab.runAutomationTask;
  api.deleteAutomationTask = automationTab.deleteAutomationTask;
  api.editAutomationTask = automationTab.editAutomationTask;
  api.viewAutomationRun = automationTab.viewAutomationRun;
  api.viewAutomationTaskRuns = automationTab.viewAutomationTaskRuns;
  api.showAutomationMenu = automationTab.showAutomationMenu;

  // History Tab API
  api.loadHistory = loadHistory;

  // Chat Tab API
  api.createSession = createSession;
  api.selectSession = selectSession;
  api.deleteSession = deleteSession;
  api.generateSessionTitle = generateSessionTitle;
  api.cancelRequest = cancelRequest;

  // ============================================================================
  // Lifecycle Hooks
  // ============================================================================

  return {
    /**
     * Init - called once when page is first loaded
     */
    init(_ctx) {
      console.log("[Assistant] Initializing");

      const hashTab = window.location.hash.slice(1);
      const savedTab = AppState.load(ASSISTANT_STATE_KEY, DEFAULT_TAB);
      state.currentTab = ASSISTANT_TAB_IDS.has(hashTab)
        ? hashTab
        : ASSISTANT_TAB_IDS.has(savedTab)
          ? savedTab
          : DEFAULT_TAB;
      window.history.replaceState(
        { page: "assistant", subtab: state.currentTab },
        "",
        `#${state.currentTab}`
      );

      // Show the initial tab content
      switchTab(state.currentTab, { load: false });

      // Setup event handlers
      setupSettingsHandlers();
      setupTestingHandlers();
      setupInstructionHandlers();
      setupChatHandlers();
      automationTab.setupAutomationHandlers();

      addTrackedListener(window, "popstate", () => {
        const tabId = window.location.hash.slice(1);
        if (!ASSISTANT_TAB_IDS.has(tabId) || tabId === state.currentTab) return;
        state.currentTab = tabId;
        AppState.save(ASSISTANT_STATE_KEY, tabId);
        subTabBar?.setActive(tabId, {
          silent: true,
          skipValidation: true,
          historyMode: "replace",
          playSound: false,
        });
        switchTab(tabId);
      });

      // Setup stats toggle
      const statsToggle = $("#stats-assistant-toggle");
      if (statsToggle) {
        addTrackedListener(statsToggle, "change", async (e) => {
          await toggleAiEnabled(e.target.checked);
        });
      }
    },

    /**
     * Activate the page (start pollers)
     */
    activate(ctx) {
      console.log("[Assistant] Activating page");

      if (!subTabBar) {
        subTabBar = new TabBar({
          container: "#subTabsContainer",
          tabs: buildAssistantTabs(),
          defaultTab: state.currentTab,
          stateKey: ASSISTANT_STATE_KEY,
          pageName: "assistant",
          onChange: (tabId) => {
            state.currentTab = tabId;
            switchTab(tabId);
          },
        });
        TabBarManager.register("assistant", subTabBar);
      }
      ctx.manageTabBar(subTabBar);
      TabBarManager.register("assistant", subTabBar);
      subTabBar.show({ force: true });

      const restoredTab = subTabBar.getActiveTab();
      if (restoredTab && restoredTab !== state.currentTab) {
        state.currentTab = restoredTab;
        switchTab(restoredTab, { load: false });
      }

      // Create pollers once, then re-register the same instances with the
      // lifecycle when a cached page is revisited.
      if (!statusPoller) {
        statusPoller = new Poller(
          async () => {
            if (state.currentTab === "stats") {
              await loadAiStatus();
            }
          },
          { label: "Assistant Status", intervalMs: 5000 } // l10n-ignore: poller name used in logs, never displayed
        );
      }

      if (!providersPoller) {
        providersPoller = new Poller(
          async () => {
            if (state.currentTab === "providers") {
              await providersTab.loadProviders();
            }
          },
          { label: "Assistant Providers", intervalMs: 10000 } // l10n-ignore: poller name used in logs, never displayed
        );
      }

      if (!cachePoller) {
        cachePoller = new Poller(
          async () => {
            if (state.currentTab === "settings") {
              await loadCacheStats();
            }
          },
          { label: "Cache Stats", intervalMs: 5000 } // l10n-ignore: poller name used in logs, never displayed
        );
      }

      if (!chatPoller) {
        chatPoller = new Poller(
          async () => {
            if (state.currentTab === "chat") {
              await loadSessions();
            }
          },
          { label: "Chat Sessions", intervalMs: 3000 } // l10n-ignore: poller name used in logs, never displayed
        );
      }

      if (!automationPoller) {
        automationPoller = new Poller(
          async () => {
            if (state.currentTab === "automation" && !document.hidden) {
              await automationTab.loadAutomationTasks();
              await automationTab.loadAutomationRuns();
              await automationTab.loadAutomationStats();
            }
          },
          { label: "Automation Tasks", intervalMs: 10000 } // l10n-ignore: poller name used in logs, never displayed
        );
      }

      ctx.managePoller(statusPoller);
      ctx.managePoller(providersPoller);
      ctx.managePoller(cachePoller);
      ctx.managePoller(chatPoller);
      ctx.managePoller(automationPoller);

      // switchTab paints first and starts the selected tab's independent loads
      // without holding the router's navigation transaction open.
      switchTab(state.currentTab);
    },

    /**
     * Deactivate the page (stop pollers)
     */
    async deactivate() {
      console.log("[Assistant] Deactivating page");
      // Pollers are auto-stopped by lifecycle
    },

    /**
     * Dispose - cleanup when page is destroyed
     */
    async dispose() {
      console.log("[Assistant] Disposing page");

      // Destroy chat widget
      if (_chatWidget) {
        _chatWidget.destroy();
        _chatWidget = null;
      }
      subTabBar = null;
      statusPoller = null;
      providersPoller = null;
      cachePoller = null;
      chatPoller = null;
      automationPoller = null;
      TabBarManager.unregister("assistant");

      // Clean up event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;
      _lastDecisionsKey = null;
    },

    // Expose API for external access
    api,
  };
}

// Create lifecycle instance
const lifecycle = createLifecycle();

// Expose API functions globally for dynamically-rendered inline event handlers
// (used in provider cards, instruction cards, modals, etc.)
window.assistantPage = lifecycle.api;

// Register the page
registerPage("assistant", lifecycle);
