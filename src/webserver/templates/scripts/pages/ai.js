import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { playToggleOn, playError } from "../core/sounds.js";
import { ChatWidget } from "../core/chat_widget.js";

// Import tab modules
import { createProvidersTab } from "./ai/providers_tab.js";
import { createInstructionsTab } from "./ai/instructions_tab.js";
import { createAutomationTab } from "./ai/automation_tab.js";

// Constants
const DEFAULT_TAB = "chat";

// Provider names mapping
const PROVIDER_NAMES = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  groq: "Groq",
  deepseek: "DeepSeek",
  gemini: "Google Gemini",
  ollama: "Ollama",
  together: "Together AI",
  openrouter: "OpenRouter",
  mistral: "Mistral AI",
  Assistant: "an LLM provider",
};

function createLifecycle() {
  // Component references
  let statusPoller = null;
  let providersPoller = null;
  let cachePoller = null;
  let chatPoller = null;
  let automationPoller = null;
  let _chatWidget = null;

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
    AssistantAuth: {
      authenticated: false,
      hasGithubToken: false,
    },
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

  /**
   * Format number with commas
   */
  function formatNumber(num) {
    return num.toLocaleString();
  }

  /**
   * Format bytes to human-readable size
   */
  function formatBytes(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  }

  // ============================================================================
  // Tab Management
  // ============================================================================

  /**
   * Initialize sidebar navigation
   */
  function initSubTabs() {
    const navItems = $$(".ai-nav-item");
    navItems.forEach((item) => {
      addTrackedListener(item, "click", () => {
        const tabId = item.dataset.tab;
        if (tabId && tabId !== state.currentTab) {
          console.log("[AI] Sidebar navigation to:", tabId);
          state.currentTab = tabId;
          updateSidebarNavigation(tabId);
          switchTab(tabId);
        }
      });
    });
  }

  /**
   * Update sidebar navigation active state
   */
  function updateSidebarNavigation(tabId) {
    $$(".ai-nav-item").forEach((item) => {
      const isActive = item.dataset.tab === tabId;
      item.classList.toggle("active", isActive);
    });
  }

  /**
   * Switch between main tabs
   */
  function switchTab(tabId) {
    // Stop all pollers to prevent memory leaks
    if (statusPoller) statusPoller.stop();
    if (providersPoller) providersPoller.stop();
    if (cachePoller) cachePoller.stop();
    if (chatPoller) chatPoller.stop();
    if (automationPoller) automationPoller.stop();

    // Hide all panels
    const allPanels = $$(".ai-panel-content");
    allPanels.forEach((panel) => {
      panel.classList.remove("active");
    });

    // Show the selected panel
    const selectedPanel = $(`#${tabId}-panel`);
    if (selectedPanel) {
      selectedPanel.classList.add("active");
    }

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
   * Load AI status and update UI
   */
  async function loadAiStatus() {
    try {
      const response = await fetch("/api/ai/status");
      if (!response.ok) throw new Error("Failed to fetch AI status");

      const data = await response.json();
      state.aiStatus = data;

      updateStatusBar(data);
      updateMetrics(data);
      updateRecentDecisions(data.recent_decisions || []);
    } catch (error) {
      console.error("[AI] Failed to load AI status:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to load AI status",
      });
    }
  }

  /**
   * Update status bar
   */
  function updateStatusBar(data) {
    const statusBar = $("#ai-status-bar");
    const statusText = $("#ai-status-text");
    const toggle = $("#stats-ai-toggle");
    const toggleLabel = $("#stats-toggle-label");

    if (!statusBar || !toggle) return;

    const enabled = data.enabled || false;

    // Update status bar state
    statusBar.setAttribute("data-status", enabled ? "enabled" : "disabled");

    // Update status text
    if (statusText) {
      statusText.textContent = enabled ? "Assistant Active" : "Assistant Disabled";
    }

    // Update toggle
    toggle.checked = enabled;
    toggle.disabled = false;
    if (toggleLabel) {
      toggleLabel.textContent = enabled ? "ON" : "OFF";
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
      totalEval.textContent = formatNumber(metrics.total_evaluations || 0);
    }

    // Cache Hit Rate
    const cacheHitRate = $("#metric-cache-hit-rate");
    if (cacheHitRate) {
      const rate = metrics.cache_hit_rate || 0;
      cacheHitRate.textContent = `${Math.round(rate * 100)}%`;
    }

    // Avg Latency
    const avgLatency = $("#metric-avg-latency");
    if (avgLatency) {
      avgLatency.textContent = `${Math.round(metrics.avg_response_time_ms || 0)}ms`;
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
    const container = $("#recent-decisions-container");
    if (!container) return;

    if (!decisions || decisions.length === 0) {
      container.innerHTML =
        '<div class="empty-state">No recent decisions</div>';
      return;
    }

    container.innerHTML = decisions
      .map((d) => {
        const decisionClass =
          d.decision === "allow"
            ? "decision-allow"
            : d.decision === "reject"
              ? "decision-reject"
              : "";
        const icon =
          d.decision === "allow" ? "check-circle" : d.decision === "reject" ? "x-circle" : "help-circle";
        const time = d.timestamp ? Utils.formatTimeAgo(new Date(d.timestamp)) : "";

        return `
          <div class="decision-card ${decisionClass}">
            <div class="decision-header">
              <i class="icon-${icon}"></i>
              <span class="decision-type">${Utils.escapeHtml(d.context || "Unknown")}</span>
              <span class="decision-time">${time}</span>
            </div>
            <div class="decision-body">
              <div class="decision-token">${Utils.escapeHtml(d.token || "N/A")}</div>
              <div class="decision-result">${Utils.escapeHtml(d.decision.toUpperCase())}</div>
            </div>
            <div class="decision-meta">
              <span class="meta-item"><i class="icon-zap"></i> ${Math.round(d.latency_ms || 0)}ms</span>
              <span class="meta-item"><i class="icon-activity"></i> ${Math.round((d.confidence || 0) * 100)}%</span>
            </div>
          </div>
        `;
      })
      .join("");
  }

  /**
   * Toggle AI enabled state
   */
  async function toggleAiEnabled(enabled) {
    try {
      const response = await fetch("/api/ai/config", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
      });

      if (!response.ok) throw new Error("Failed to update AI status");

      playToggleOn();
      Utils.showToast({
        type: "success",
        title: enabled ? "Assistant Enabled" : "Assistant Disabled",
        message: enabled ? "AI Assistant is now active" : "AI Assistant has been disabled",
      });

      await loadAiStatus();
    } catch (error) {
      console.error("[AI] Failed to toggle AI:", error);
      playError();
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to update AI status",
      });

      // Revert toggle
      const toggle = $("#stats-ai-toggle");
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
      const response = await fetch("/api/ai/config");
      if (!response.ok) throw new Error("Failed to fetch AI config");

      const data = await response.json();
      state.config = data;

      updateConfigForm(data);
    } catch (error) {
      console.error("[AI] Failed to load config:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to load AI configuration",
      });
    }
  }

  /**
   * Update configuration form
   */
  function updateConfigForm(config) {
    // Master Control
    const enabledToggle = $("#setting-enabled");
    if (enabledToggle) enabledToggle.checked = config.enabled || false;

    const defaultProvider = $("#setting-default-provider");
    if (defaultProvider) {
      // Populate provider options
      defaultProvider.innerHTML =
        '<option value="">Select Provider...</option>' +
        Object.keys(PROVIDER_NAMES)
          .map((id) => `<option value="${id}">${PROVIDER_NAMES[id]}</option>`)
          .join("");
      defaultProvider.value = config.default_provider || "";
    }

    // Filtering
    const filteringEnabled = $("#setting-filtering-enabled");
    if (filteringEnabled) filteringEnabled.checked = config.filtering?.enabled || false;

    const minConfidence = $("#setting-min-confidence");
    const minConfidenceValue = $("#slider-value-min-confidence");
    if (minConfidence && minConfidenceValue) {
      const value = Math.round((config.filtering?.min_confidence || 0.7) * 100);
      minConfidence.value = value / 100;
      minConfidenceValue.textContent = value + "%";
    }
  }

  // ============================================================================
  // Settings Tab - Cache Stats Only (rest delegated to settings module)
  // ============================================================================

  /**
   * Load cache statistics
   */
  async function loadCacheStats() {
    try {
      const response = await fetch("/api/ai/cache/stats");
      if (!response.ok) throw new Error("Failed to fetch cache stats");

      const data = await response.json();
      state.cacheStats = data;

      updateCacheStats(data);
    } catch (error) {
      console.error("[AI] Failed to load cache stats:", error);
    }
  }

  /**
   * Update cache stats display
   */
  function updateCacheStats(stats) {
    const cacheSize = $("#cache-size");
    const cacheMemory = $("#cache-memory");

    if (cacheSize) cacheSize.textContent = formatNumber(stats.total_entries || 0);
    if (cacheMemory) cacheMemory.textContent = formatBytes(stats.total_size_bytes || 0);
  }

  /**
   * Clear AI cache
   */
  async function clearCache() {
    const confirmed = await ConfirmationDialog.show({
      title: "Clear Cache",
      message: "Are you sure you want to clear the AI cache? This will remove all cached AI responses.",
      confirmText: "Clear Cache",
      confirmClass: "danger",
    });

    if (!confirmed) return;

    try {
      const response = await fetch("/api/ai/cache/clear", { method: "POST" });
      if (!response.ok) throw new Error("Failed to clear cache");

      playToggleOn();
      Utils.showToast({
        type: "success",
        title: "Cache Cleared",
        message: "AI cache has been cleared successfully",
      });

      await loadCacheStats();
    } catch (error) {
      console.error("[AI] Failed to clear cache:", error);
      playError();
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to clear cache",
      });
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
        // Collect form data
        const config = {
          enabled: $("#setting-enabled")?.checked || false,
          default_provider: $("#setting-default-provider")?.value || "",
          filtering: {
            enabled: $("#setting-filtering-enabled")?.checked || false,
            min_confidence: parseFloat($("#setting-min-confidence")?.value) || 0.7,
          },
          entry_analysis: {
            enabled: $("#setting-entry-enabled")?.checked || false,
            min_confidence: parseFloat($("#setting-entry-min-confidence")?.value) || 0.7,
          },
          exit_analysis: {
            enabled: $("#setting-exit-enabled")?.checked || false,
            min_confidence: parseFloat($("#setting-exit-min-confidence")?.value) || 0.7,
          },
        };

        try {
          const response = await fetch("/api/ai/config", {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(config),
          });

          if (!response.ok) throw new Error("Failed to save configuration");

          playToggleOn();
          Utils.showToast({
            type: "success",
            title: "Saved",
            message: "Configuration saved successfully",
          });

          await loadConfig();
        } catch (error) {
          console.error("[AI] Failed to save config:", error);
          playError();
          Utils.showToast({
            type: "error",
            title: "Error",
            message: "Failed to save configuration",
          });
        }
      });
    }

    // Setup sliders
    setupSliders();
  }

  /**
   * Setup slider value displays
   */
  function setupSliders() {
    const sliders = [
      { slider: "setting-min-confidence", display: "slider-value-min-confidence" },
      { slider: "setting-entry-min-confidence", display: "slider-value-entry-min-confidence" },
      { slider: "setting-exit-min-confidence", display: "slider-value-exit-min-confidence" },
    ];

    sliders.forEach(({ slider, display }) => {
      const sliderEl = $(`#${slider}`);
      const displayEl = $(`#${display}`);
      if (sliderEl && displayEl) {
        addTrackedListener(sliderEl, "input", () => {
          displayEl.textContent = Math.round(parseFloat(sliderEl.value) * 100) + "%";
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
      const response = await fetch(`/api/ai/history?page=${page}&limit=20`);
      if (!response.ok) throw new Error("Failed to load history");

      const data = await response.json();
      state.historyPage = page;
      state.historyTotal = data.total || 0;

      renderHistory(data.history || [], page, data.total || 0);
    } catch (error) {
      console.error("[AI] Error loading history:", error);
      const container = $("#history-list");
      if (container) {
        container.innerHTML = '<div class="empty-state">Failed to load history</div>';
      }
    }
  }

  /**
   * Render history list
   */
  function renderHistory(history, page, total) {
    const container = $("#history-list");
    if (!container) return;

    if (!history || history.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <span class="empty-icon">📋</span>
          <p class="empty-text">No AI requests yet</p>
        </div>
      `;
      return;
    }

    const pageSize = 20;
    const totalPages = Math.ceil(total / pageSize);

    container.innerHTML = `
      <div class="history-items">
        ${history
          .map((item) => {
            const statusClass = item.success ? "success" : "failed";
            const statusIcon = item.success ? "check-circle" : "x-circle";
            const time = item.timestamp ? new Date(item.timestamp).toLocaleString() : "";

            return `
            <div class="history-item ${statusClass}">
              <i class="icon-${statusIcon} history-status-icon"></i>
              <div class="history-info">
                <div class="history-context">${Utils.escapeHtml(item.context || "Unknown")}</div>
                <div class="history-details">
                  <span>${time}</span>
                  <span>${Math.round(item.latency_ms || 0)}ms</span>
                  ${item.model ? `<span>${Utils.escapeHtml(item.model)}</span>` : ""}
                </div>
              </div>
            </div>
          `;
          })
          .join("")}
      </div>
      ${
        totalPages > 1
          ? `
        <div class="history-pagination">
          <button class="btn btn-sm btn-secondary" ${page <= 1 ? "disabled" : ""} 
                  onclick="window.aiPage.loadHistory(${page - 1})">
            <i class="icon-chevron-left"></i> Previous
          </button>
          <span class="pagination-info">Page ${page} of ${totalPages}</span>
          <button class="btn btn-sm btn-secondary" ${page >= totalPages ? "disabled" : ""}
                  onclick="window.aiPage.loadHistory(${page + 1})">
            Next <i class="icon-chevron-right"></i>
          </button>
        </div>
      `
          : ""
      }
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
    _chatWidget = new ChatWidget(container, { showSidebar: true });
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
   * Summarize chat session
   */
  async function summarizeSession(sessionId) {
    if (_chatWidget) {
      await _chatWidget.summarizeSession(sessionId);
    }
  }

  /**
   * Generate session title
   */
  async function generateSessionTitle(sessionId) {
    if (_chatWidget) {
      await _chatWidget.generateTitle(sessionId);
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
  api.checkAssistantAuthStatus = providersTab.checkAssistantAuthStatus;
  api.disconnectAssistant = providersTab.disconnectAssistant;

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
  api.toggleAutomationTask = automationTab.toggleAutomationTask;
  api.runAutomationTask = automationTab.runAutomationTask;
  api.deleteAutomationTask = automationTab.deleteAutomationTask;
  api.editAutomationTask = automationTab.editAutomationTask;
  api.viewAutomationRun = automationTab.viewAutomationRun;
  api.viewAutomationTaskRuns = automationTab.viewAutomationTaskRuns;

  // History Tab API
  api.loadHistory = loadHistory;

  // Chat Tab API
  api.createSession = createSession;
  api.selectSession = selectSession;
  api.deleteSession = deleteSession;
  api.summarizeSession = summarizeSession;
  api.generateSessionTitle = generateSessionTitle;
  api.cancelRequest = cancelRequest;

  // ============================================================================
  // Lifecycle Hooks
  // ============================================================================

  return {
    /**
     * Init - called once when page is first loaded
     */
    async init(_ctx) {
      console.log("[AI] Initializing");

      // Check Assistant auth status
      await providersTab.checkAssistantAuthStatus();

      // Initialize sidebar navigation
      initSubTabs();

      // Set initial active state
      updateSidebarNavigation(DEFAULT_TAB);

      // Show the initial tab content
      switchTab(state.currentTab);

      // Setup event handlers
      setupSettingsHandlers();
      setupTestingHandlers();
      setupInstructionHandlers();
      setupChatHandlers();
      automationTab.setupAutomationHandlers();

      // Setup stats toggle
      const statsToggle = $("#stats-ai-toggle");
      if (statsToggle) {
        addTrackedListener(statsToggle, "change", async (e) => {
          await toggleAiEnabled(e.target.checked);
        });
      }
    },

    /**
     * Activate the page (start pollers)
     */
    async activate(ctx) {
      console.log("[AI] Activating page");

      // Create pollers
      statusPoller = ctx.managePoller(
        new Poller(
          async () => {
            if (state.currentTab === "stats") {
              await loadAiStatus();
            }
          },
          { label: "Assistant Status", intervalMs: 5000 }
        )
      );

      providersPoller = ctx.managePoller(
        new Poller(
          async () => {
            if (state.currentTab === "providers") {
              await providersTab.loadProviders();
            }
          },
          { label: "Assistant Providers", intervalMs: 10000 }
        )
      );

      cachePoller = ctx.managePoller(
        new Poller(
          async () => {
            if (state.currentTab === "settings") {
              await loadCacheStats();
            }
          },
          { label: "Cache Stats", intervalMs: 5000 }
        )
      );

      chatPoller = ctx.managePoller(
        new Poller(
          async () => {
            if (state.currentTab === "chat") {
              await loadSessions();
            }
          },
          { label: "Chat Sessions", intervalMs: 3000 }
        )
      );

      automationPoller = ctx.managePoller(
        new Poller(
          async () => {
            if (state.currentTab === "automation" && !document.hidden) {
              await automationTab.loadAutomationTasks();
              await automationTab.loadAutomationRuns();
              await automationTab.loadAutomationStats();
            }
          },
          { label: "Automation Tasks", intervalMs: 10000 }
        )
      );

      // Load initial data immediately and start appropriate poller
      if (state.currentTab === "stats") {
        await loadAiStatus();
        statusPoller.start();
      } else if (state.currentTab === "providers") {
        await providersTab.loadProviders();
        providersPoller.start();
      } else if (state.currentTab === "settings") {
        await loadConfig();
        await loadCacheStats();
        cachePoller.start();
      } else if (state.currentTab === "chat") {
        await loadSessions();
        chatPoller.start();
      } else if (state.currentTab === "automation") {
        await automationTab.loadAutomationTasks();
        await automationTab.loadAutomationRuns();
        await automationTab.loadAutomationStats();
        automationPoller.start();
      }
    },

    /**
     * Deactivate the page (stop pollers)
     */
    async deactivate() {
      console.log("[AI] Deactivating page");
      // Pollers are auto-stopped by lifecycle
    },

    /**
     * Dispose - cleanup when page is destroyed
     */
    async dispose() {
      console.log("[AI] Disposing page");

      // Destroy chat widget
      if (_chatWidget) {
        _chatWidget.destroy();
        _chatWidget = null;
      }

      // Clean up event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;
    },

    // Expose API for external access
    api,
  };
}

// Create lifecycle instance
const lifecycle = createLifecycle();

// Expose API functions globally for dynamically-rendered inline event handlers
// (used in provider cards, instruction cards, modals, etc.)
window.aiPage = lifecycle.api;

// Register the page
registerPage("ai", lifecycle);
