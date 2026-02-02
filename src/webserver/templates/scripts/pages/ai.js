import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { playToggleOn, playError } from "../core/sounds.js";

// Constants
const DEFAULT_TAB = "stats";

// Provider names mapping
const PROVIDER_NAMES = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  google: "Google AI",
  groq: "Groq",
  deepseek: "DeepSeek",
  together: "Together AI",
  fireworks: "Fireworks AI",
  openrouter: "OpenRouter",
  ollama: "Ollama",
  custom: "Custom",
};

function createLifecycle() {
  // Component references
  let statusPoller = null;
  let providersPoller = null;
  let cachePoller = null;

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
    // Setup sidebar navigation click handlers
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
  function updateSidebarNavigation(activeTab) {
    const navItems = $$(".ai-nav-item");
    navItems.forEach((item) => {
      const isActive = item.dataset.tab === activeTab;
      item.classList.toggle("active", isActive);
    });
  }

  /**
   * Switch to a tab
   */
  function switchTab(tabId) {
    // Stop all pollers to prevent memory leaks
    if (statusPoller) statusPoller.stop();
    if (providersPoller) providersPoller.stop();
    if (cachePoller) cachePoller.stop();

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

    // Update sidebar status indicator
    updateSidebarStatus(tabId);

    // Load data for the tab and start appropriate poller
    if (tabId === "stats") {
      loadAiStatus();
      if (statusPoller) statusPoller.start();
    } else if (tabId === "providers") {
      loadProviders();
      if (providersPoller) providersPoller.start();
    } else if (tabId === "settings") {
      loadConfig();
      loadCacheStats();
      if (cachePoller) cachePoller.start();
    } else if (tabId === "instructions") {
      loadInstructions();
      loadTemplates();
    } else if (tabId === "history") {
      loadHistory(1);
    }
  }

  /**
   * Update sidebar status indicator based on AI state
   */
  function updateSidebarStatus() {
    const indicator = $(".ai-status-indicator");
    const statusText = $(".ai-status-indicator .status-text");
    
    if (!indicator || !statusText) return;

    // Update status based on AI state
    if (state.aiStatus) {
      if (state.aiStatus.enabled) {
        indicator.classList.remove("error");
        statusText.textContent = "Active";
      } else {
        indicator.classList.add("error");
        statusText.textContent = "Disabled";
      }
    } else {
      indicator.classList.remove("error");
      statusText.textContent = "Ready";
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
      updateSidebarStatus(); // Update sidebar status indicator
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
      statusText.textContent = enabled ? "AI Analysis Active" : "AI Analysis Disabled";
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
      const latency = metrics.avg_latency_ms || 0;
      avgLatency.textContent = `${Math.round(latency)}ms`;
    }

    // Providers
    const providers = $("#metric-providers");
    if (providers) {
      const configured = metrics.configured_providers || 0;
      providers.textContent = `${configured} / 10`;
    }
  }

  /**
   * Update recent decisions list
   */
  function updateRecentDecisions(decisions) {
    const container = $("#recent-decisions-container");
    if (!container) return;

    if (decisions.length === 0) {
      container.innerHTML =
        '<div style="padding: 1rem; text-align: center; color: var(--text-muted);">No recent decisions</div>';
      return;
    }

    container.innerHTML = decisions
      .map((decision) => {
        const isPass = decision.decision === "pass";
        const iconClass = isPass ? "pass" : "reject";
        const icon = isPass ? "check" : "x";
        const confidence = Math.round(decision.confidence * 100);

        return `
        <div class="decision-item">
          <div class="decision-icon ${iconClass}">
            <i class="icon-${icon}"></i>
          </div>
          <div class="decision-content">
            <div class="decision-mint">${decision.mint}</div>
            <div class="decision-details">
              ${decision.risk_level || "N/A"} risk · ${decision.latency_ms || 0}ms
            </div>
          </div>
          <div class="decision-confidence">${confidence}%</div>
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

      if (!response.ok) throw new Error("Failed to update AI config");

      playToggleOn();
      Utils.showToast({
        type: "success",
        title: "AI Updated",
        message: `AI analysis ${enabled ? "enabled" : "disabled"}`,
      });

      // Reload status
      await loadAiStatus();
    } catch (error) {
      console.error("[AI] Failed to toggle AI:", error);
      playError();
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to update AI state",
      });

      // Revert toggle
      const toggle = $("#stats-ai-toggle");
      if (toggle) toggle.checked = !enabled;
    }
  }

  // ============================================================================
  // Providers Tab
  // ============================================================================

  /**
   * Load and render providers
   */
  async function loadProviders() {
    try {
      const response = await fetch("/api/ai/providers");
      if (!response.ok) throw new Error("Failed to fetch providers");

      const data = await response.json();
      state.providers = data.providers || [];

      renderProviders(state.providers);
    } catch (error) {
      console.error("[AI] Failed to load providers:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to load providers",
      });
    }
  }

  /**
   * Render provider cards
   */
  function renderProviders(providers) {
    const grid = $("#providers-grid");
    if (!grid) return;

    // Get all provider IDs
    const allProviderIds = Object.keys(PROVIDER_NAMES);

    grid.innerHTML = allProviderIds
      .map((id) => {
        const provider = providers.find((p) => p.id === id) || {
          id,
          enabled: false,
          api_key: "",
          model: "",
        };

        const name = PROVIDER_NAMES[id];
        const enabled = provider.enabled || false;
        const hasApiKey = provider.api_key && provider.api_key.length > 0;
        const disabledClass = !enabled ? "disabled" : "";

        return `
        <div class="provider-card ${disabledClass}" data-provider-id="${id}">
          <div class="provider-header">
            <span class="provider-name">${name}</span>
            <div class="provider-toggle">
              <label>
                <input type="checkbox" ${enabled ? "checked" : ""} 
                       data-provider-id="${id}" class="provider-enable-toggle" />
                Enable
              </label>
            </div>
          </div>
          <div class="provider-body">
            <div class="provider-field">
              <label>API Key</label>
              <input type="password" 
                     placeholder="Enter API key..." 
                     value="${provider.api_key || ""}"
                     data-provider-id="${id}"
                     class="provider-api-key" />
            </div>
            <div class="provider-field">
              <label>Model</label>
              <input type="text" 
                     placeholder="e.g., gpt-4, claude-3-opus..." 
                     value="${provider.model || ""}"
                     data-provider-id="${id}"
                     class="provider-model" />
            </div>
            <div class="provider-actions">
              <button type="button" class="btn btn-sm btn-secondary provider-test-btn" 
                      data-provider-id="${id}"
                      ${!hasApiKey ? "disabled" : ""}>
                <i class="icon-zap"></i> Test
              </button>
              <button type="button" class="btn btn-sm btn-primary provider-save-btn" 
                      data-provider-id="${id}">
                <i class="icon-save"></i> Save
              </button>
            </div>
            <div class="provider-status-container" data-provider-id="${id}"></div>
          </div>
        </div>
      `;
      })
      .join("");

    // Attach event handlers
    setupProviderHandlers();
  }

  /**
   * Setup provider event handlers
   */
  function setupProviderHandlers() {
    // Enable/disable toggles
    $$(".provider-enable-toggle").forEach((toggle) => {
      addTrackedListener(toggle, "change", async (e) => {
        const providerId = e.target.dataset.providerId;
        const enabled = e.target.checked;
        await updateProviderField(providerId, "enabled", enabled);
      });
    });

    // Test buttons
    $$(".provider-test-btn").forEach((btn) => {
      addTrackedListener(btn, "click", async (e) => {
        const providerId = e.currentTarget.dataset.providerId;
        await testProvider(providerId);
      });
    });

    // Save buttons
    $$(".provider-save-btn").forEach((btn) => {
      addTrackedListener(btn, "click", async (e) => {
        const providerId = e.currentTarget.dataset.providerId;
        await saveProvider(providerId);
      });
    });
  }

  /**
   * Save provider configuration
   */
  async function saveProvider(providerId) {
    const apiKeyInput = $(`.provider-api-key[data-provider-id="${providerId}"]`);
    const modelInput = $(`.provider-model[data-provider-id="${providerId}"]`);
    const enableToggle = $(`.provider-enable-toggle[data-provider-id="${providerId}"]`);
    const saveBtn = $(`.provider-save-btn[data-provider-id="${providerId}"]`);

    if (!apiKeyInput || !modelInput || !enableToggle) return;

    const config = {
      enabled: enableToggle.checked,
      api_key: apiKeyInput.value.trim(),
      model: modelInput.value.trim(),
    };

    // Validation
    if (config.enabled && !config.api_key) {
      Utils.showToast({
        type: "warning",
        title: "Missing API Key",
        message: "Please enter an API key to enable this provider",
      });
      return;
    }

    if (config.enabled && !config.model) {
      Utils.showToast({
        type: "warning",
        title: "Missing Model",
        message: "Please enter a model name",
      });
      return;
    }

    // Show loading state
    const originalHTML = saveBtn ? saveBtn.innerHTML : "";
    if (saveBtn) {
      saveBtn.disabled = true;
      saveBtn.innerHTML = '<i class="icon-loader"></i> Saving...';
    }

    try {
      const response = await fetch(`/api/ai/providers/${providerId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(config),
      });

      if (!response.ok) throw new Error("Failed to save provider");

      Utils.showToast({
        type: "success",
        title: "Provider Saved",
        message: `${PROVIDER_NAMES[providerId]} configuration saved`,
      });

      // Update card state
      const card = $(`.provider-card[data-provider-id="${providerId}"]`);
      if (card) {
        if (config.enabled) {
          card.classList.remove("disabled");
        } else {
          card.classList.add("disabled");
        }
      }
    } catch (error) {
      console.error(`[AI] Failed to save provider ${providerId}:`, error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: `Failed to save ${PROVIDER_NAMES[providerId]}`,
      });
    } finally {
      // Restore button state
      if (saveBtn) {
        saveBtn.disabled = false;
        saveBtn.innerHTML = originalHTML;
      }
    }
  }

  /**
   * Update a single provider field
   */
  async function updateProviderField(providerId, field, value) {
    try {
      const response = await fetch(`/api/ai/providers/${providerId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ [field]: value }),
      });

      if (!response.ok) throw new Error("Failed to update provider");

      // Update card state for enabled toggle
      if (field === "enabled") {
        const card = $(`.provider-card[data-provider-id="${providerId}"]`);
        if (card) {
          if (value) {
            card.classList.remove("disabled");
          } else {
            card.classList.add("disabled");
          }
        }
      }
    } catch (error) {
      console.error(`[AI] Failed to update provider ${providerId}:`, error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: `Failed to update ${PROVIDER_NAMES[providerId]}`,
      });
    }
  }

  /**
   * Test provider connection
   */
  async function testProvider(providerId) {
    const btn = $(`.provider-test-btn[data-provider-id="${providerId}"]`);
    const statusContainer = $(`.provider-status-container[data-provider-id="${providerId}"]`);

    if (!btn || !statusContainer) return;

    // Show loading state
    btn.disabled = true;
    btn.innerHTML = '<i class="icon-loader"></i> Testing...';
    statusContainer.innerHTML = "";

    try {
      const response = await fetch(`/api/ai/providers/${providerId}/test`, {
        method: "POST",
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || errorData.message || `HTTP ${response.status}`);
      }

      const result = await response.json();

      if (result.success) {
        statusContainer.innerHTML = `
          <div class="provider-status success">
            <i class="icon-check-circle"></i> Connection successful
          </div>
        `;
        Utils.showToast({
          type: "success",
          title: "Test Successful",
          message: `${PROVIDER_NAMES[providerId]} is working correctly`,
        });
      } else {
        throw new Error(result.error || result.message || "Test failed");
      }
    } catch (error) {
      console.error(`[AI] Test failed for provider ${providerId}:`, error);
      statusContainer.innerHTML = `
        <div class="provider-status error">
          <i class="icon-x-circle"></i> ${error.message}
        </div>
      `;
      Utils.showToast({
        type: "error",
        title: "Test Failed",
        message: error.message,
      });
    } finally {
      // Restore button state
      btn.disabled = false;
      btn.innerHTML = '<i class="icon-zap"></i> Test';
    }
  }

  // ============================================================================
  // Settings Tab
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
      minConfidence.value = value;
      minConfidenceValue.textContent = value;
    }

    const fallbackPass = $("#setting-fallback-pass");
    if (fallbackPass) fallbackPass.checked = config.filtering?.fallback_pass || false;

    // Trading
    const entryAnalysis = $("#setting-entry-analysis");
    if (entryAnalysis) entryAnalysis.checked = config.trading?.entry_analysis || false;

    const exitAnalysis = $("#setting-exit-analysis");
    if (exitAnalysis) exitAnalysis.checked = config.trading?.exit_analysis || false;

    const trailingStop = $("#setting-trailing-stop");
    if (trailingStop) trailingStop.checked = config.trading?.trailing_stop || false;

    // Auto Blacklist
    const autoBlacklistEnabled = $("#setting-auto-blacklist-enabled");
    if (autoBlacklistEnabled)
      autoBlacklistEnabled.checked = config.auto_blacklist?.enabled || false;

    const blacklistMinConfidence = $("#setting-blacklist-min-confidence");
    const blacklistMinConfidenceValue = $("#slider-value-blacklist-min-confidence");
    if (blacklistMinConfidence && blacklistMinConfidenceValue) {
      const value = Math.round((config.auto_blacklist?.min_confidence || 0.3) * 100);
      blacklistMinConfidence.value = value;
      blacklistMinConfidenceValue.textContent = value;
    }

    // Performance
    const cacheTtl = $("#setting-cache-ttl");
    const cacheTtlValue = $("#slider-value-cache-ttl");
    if (cacheTtl && cacheTtlValue) {
      const value = config.performance?.cache_ttl || 300;
      cacheTtl.value = value;
      cacheTtlValue.textContent = value;
    }

    const maxEvaluations = $("#setting-max-evaluations");
    const maxEvaluationsValue = $("#slider-value-max-evaluations");
    if (maxEvaluations && maxEvaluationsValue) {
      const value = config.performance?.max_evaluations || 5;
      maxEvaluations.value = value;
      maxEvaluationsValue.textContent = value;
    }
  }

  /**
   * Setup settings event handlers
   */
  function setupSettingsHandlers() {
    // Master Control
    const enabledToggle = $("#setting-enabled");
    if (enabledToggle) {
      addTrackedListener(enabledToggle, "change", async (e) => {
        await updateConfig("enabled", e.target.checked);
      });
    }

    const defaultProvider = $("#setting-default-provider");
    if (defaultProvider) {
      addTrackedListener(defaultProvider, "change", async (e) => {
        await updateConfig("default_provider", e.target.value);
      });
    }

    // Filtering
    const filteringEnabled = $("#setting-filtering-enabled");
    if (filteringEnabled) {
      addTrackedListener(filteringEnabled, "change", async (e) => {
        await updateConfig("filtering.enabled", e.target.checked);
      });
    }

    const minConfidence = $("#setting-min-confidence");
    const minConfidenceValue = $("#slider-value-min-confidence");
    if (minConfidence && minConfidenceValue) {
      addTrackedListener(minConfidence, "input", (e) => {
        minConfidenceValue.textContent = e.target.value;
      });
      addTrackedListener(minConfidence, "change", async (e) => {
        await updateConfig("filtering.min_confidence", parseFloat(e.target.value) / 100);
      });
    }

    const fallbackPass = $("#setting-fallback-pass");
    if (fallbackPass) {
      addTrackedListener(fallbackPass, "change", async (e) => {
        await updateConfig("filtering.fallback_pass", e.target.checked);
      });
    }

    // Trading
    const entryAnalysis = $("#setting-entry-analysis");
    if (entryAnalysis) {
      addTrackedListener(entryAnalysis, "change", async (e) => {
        await updateConfig("trading.entry_analysis", e.target.checked);
      });
    }

    const exitAnalysis = $("#setting-exit-analysis");
    if (exitAnalysis) {
      addTrackedListener(exitAnalysis, "change", async (e) => {
        await updateConfig("trading.exit_analysis", e.target.checked);
      });
    }

    const trailingStop = $("#setting-trailing-stop");
    if (trailingStop) {
      addTrackedListener(trailingStop, "change", async (e) => {
        await updateConfig("trading.trailing_stop", e.target.checked);
      });
    }

    // Auto Blacklist
    const autoBlacklistEnabled = $("#setting-auto-blacklist-enabled");
    if (autoBlacklistEnabled) {
      addTrackedListener(autoBlacklistEnabled, "change", async (e) => {
        await updateConfig("auto_blacklist.enabled", e.target.checked);
      });
    }

    const blacklistMinConfidence = $("#setting-blacklist-min-confidence");
    const blacklistMinConfidenceValue = $("#slider-value-blacklist-min-confidence");
    if (blacklistMinConfidence && blacklistMinConfidenceValue) {
      addTrackedListener(blacklistMinConfidence, "input", (e) => {
        blacklistMinConfidenceValue.textContent = e.target.value;
      });
      addTrackedListener(blacklistMinConfidence, "change", async (e) => {
        await updateConfig("auto_blacklist.min_confidence", parseFloat(e.target.value) / 100);
      });
    }

    // Performance
    const cacheTtl = $("#setting-cache-ttl");
    const cacheTtlValue = $("#slider-value-cache-ttl");
    if (cacheTtl && cacheTtlValue) {
      addTrackedListener(cacheTtl, "input", (e) => {
        cacheTtlValue.textContent = e.target.value;
      });
      addTrackedListener(cacheTtl, "change", async (e) => {
        await updateConfig("performance.cache_ttl", parseInt(e.target.value));
      });
    }

    const maxEvaluations = $("#setting-max-evaluations");
    const maxEvaluationsValue = $("#slider-value-max-evaluations");
    if (maxEvaluations && maxEvaluationsValue) {
      addTrackedListener(maxEvaluations, "input", (e) => {
        maxEvaluationsValue.textContent = e.target.value;
      });
      addTrackedListener(maxEvaluations, "change", async (e) => {
        await updateConfig("performance.max_evaluations", parseInt(e.target.value));
      });
    }

    // Clear Cache
    const clearCacheBtn = $("#clear-cache-btn");
    if (clearCacheBtn) {
      addTrackedListener(clearCacheBtn, "click", async () => {
        await clearCache();
      });
    }
  }

  /**
   * Update AI configuration
   */
  async function updateConfig(field, value) {
    try {
      // Build nested object for API
      const config = {};
      const parts = field.split(".");

      if (parts.length === 1) {
        config[field] = value;
      } else {
        let current = config;
        for (let i = 0; i < parts.length - 1; i++) {
          current[parts[i]] = {};
          current = current[parts[i]];
        }
        current[parts[parts.length - 1]] = value;
      }

      const response = await fetch("/api/ai/config", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(config),
      });

      if (!response.ok) throw new Error("Failed to update config");

      Utils.showToast({
        type: "success",
        title: "Settings Updated",
        message: "AI configuration saved",
      });

      // Reload config
      await loadConfig();
    } catch (error) {
      console.error("[AI] Failed to update config:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to update configuration",
      });
    }
  }

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
   * Update cache statistics display
   */
  function updateCacheStats(stats) {
    const cacheSize = $("#cache-size");
    if (cacheSize) {
      cacheSize.textContent = formatNumber(stats.size || 0);
    }

    const cacheMemory = $("#cache-memory");
    if (cacheMemory) {
      cacheMemory.textContent = formatBytes(stats.memory_bytes || 0);
    }
  }

  /**
   * Clear AI cache
   */
  async function clearCache() {
    const confirmed = await ConfirmationDialog.show({
      title: "Clear Cache",
      message:
        "Are you sure you want to clear the AI cache? This will remove all cached evaluations.",
      confirmText: "Clear Cache",
      confirmClass: "btn-danger",
    });

    if (!confirmed) return;

    try {
      const response = await fetch("/api/ai/cache/clear", {
        method: "POST",
      });

      if (!response.ok) throw new Error("Failed to clear cache");

      Utils.showToast({
        type: "success",
        title: "Cache Cleared",
        message: "AI cache has been cleared successfully",
      });

      // Reload cache stats
      await loadCacheStats();
    } catch (error) {
      console.error("[AI] Failed to clear cache:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to clear cache",
      });
    }
  }

  // ============================================================================
  // Testing Tab
  // ============================================================================

  /**
   * Setup testing event handlers
   */
  function setupTestingHandlers() {
    const evaluateBtn = $("#evaluate-btn");
    if (evaluateBtn) {
      addTrackedListener(evaluateBtn, "click", async () => {
        await testEvaluate();
      });
    }
  }

  /**
   * Test AI evaluation on a mint
   */
  async function testEvaluate() {
    const mintInput = $("#test-mint-address");
    const prioritySelect = $("#test-priority");
    const resultsDiv = $("#testing-results");
    const resultsContent = $("#testing-results-content");
    const evaluateBtn = $("#evaluate-btn");

    if (!mintInput || !prioritySelect || !resultsDiv || !resultsContent || !evaluateBtn) return;

    const mint = mintInput.value.trim();
    if (!mint) {
      Utils.showToast({
        type: "warning",
        title: "Missing Input",
        message: "Please enter a mint address",
      });
      return;
    }

    // Show loading state
    evaluateBtn.disabled = true;
    evaluateBtn.innerHTML = '<i class="icon-loader"></i> Evaluating...';
    resultsDiv.style.display = "none";

    try {
      const response = await fetch("/api/ai/test/evaluate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          mint,
          priority: prioritySelect.value,
        }),
      });

      if (!response.ok) throw new Error("Evaluation failed");

      const result = await response.json();

      // Display results
      displayTestResults(result);
      resultsDiv.style.display = "block";
    } catch (error) {
      console.error("[AI] Test evaluation failed:", error);
      Utils.showToast({
        type: "error",
        title: "Evaluation Failed",
        message: error.message,
      });
    } finally {
      // Restore button
      evaluateBtn.disabled = false;
      evaluateBtn.innerHTML = '<i class="icon-play"></i> Evaluate';
    }
  }

  /**
   * Display test evaluation results
   */
  function displayTestResults(result) {
    const resultsContent = $("#testing-results-content");
    if (!resultsContent) return;

    const isPass = result.decision === "pass";
    const decisionClass = isPass ? "decision-pass" : "decision-reject";
    const confidence = Math.round(result.confidence * 100);

    const factorsHtml =
      result.factors && result.factors.length > 0
        ? `
        <div class="result-item">
          <div class="result-label">Factors</div>
          <div class="result-factors">
            ${result.factors
              .map((f) => {
                const impactClass =
                  f.impact === "positive"
                    ? "factor-positive"
                    : f.impact === "negative"
                      ? "factor-negative"
                      : "factor-neutral";
                return `<span class="factor-badge ${impactClass}">${f.name} (${Math.round(f.weight * 100)}%)</span>`;
              })
              .join("")}
          </div>
        </div>
      `
        : "";

    resultsContent.innerHTML = `
      <div class="result-item">
        <div class="result-label">Decision</div>
        <div class="result-value ${decisionClass}">${result.decision.toUpperCase()}</div>
      </div>
      <div class="result-item">
        <div class="result-label">Confidence</div>
        <div class="result-value">${confidence}%</div>
      </div>
      <div class="result-item">
        <div class="result-label">Risk Level</div>
        <div class="result-value">${result.risk_level || "N/A"}</div>
      </div>
      <div class="result-item">
        <div class="result-label">Latency</div>
        <div class="result-value">${result.latency_ms || 0}ms</div>
      </div>
      ${factorsHtml}
      <div class="result-item">
        <div class="result-label">Reasoning</div>
        <div class="result-reasoning">${result.reasoning || "No reasoning provided"}</div>
      </div>
    `;
  }

  // ============================================================================
  // Instructions Tab
  // ============================================================================

  /**
   * Load instructions list
   */
  async function loadInstructions() {
    try {
      const response = await fetch("/api/ai/instructions");
      if (!response.ok) throw new Error("Failed to load instructions");
      const data = await response.json();
      state.instructions = data.instructions || [];
      renderInstructionsList(state.instructions);
    } catch (error) {
      console.error("[AI] Error loading instructions:", error);
      const container = $("#instructions-list");
      if (container) {
        container.innerHTML = '<div class="empty-state">Failed to load instructions</div>';
      }
    }
  }

  /**
   * Load templates
   */
  async function loadTemplates() {
    try {
      const response = await fetch("/api/ai/templates");
      if (!response.ok) throw new Error("Failed to load templates");
      const data = await response.json();
      state.templates = data.templates || [];
      renderTemplatesList(data.templates || []);
    } catch (error) {
      console.error("[AI] Error loading templates:", error);
    }
  }

  /**
   * Render instructions list
   */
  function renderInstructionsList(instructions) {
    const container = $("#instructions-list");
    if (!container) return;

    if (instructions.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <i class="icon-edit"></i>
          <p>No custom instructions yet</p>
          <small>Create instructions to customize AI behavior</small>
        </div>`;
      return;
    }

    container.innerHTML = instructions
      .map(
        (inst) => `
      <div class="instruction-card ${inst.enabled ? "" : "disabled"}" 
           data-id="${inst.id}" 
           draggable="true">
        <div class="instruction-header">
          <span class="drag-handle" title="Drag to reorder"><i class="icon-menu"></i></span>
          <span class="instruction-priority">#${inst.priority}</span>
          <span class="instruction-name">${Utils.escapeHtml(inst.name)}</span>
          <span class="instruction-category badge-${inst.category}">${getCategoryLabel(inst.category)}</span>
        </div>
        <div class="instruction-content" onclick="window.aiPage.toggleInstructionExpanded(${inst.id})">
          ${Utils.escapeHtml(inst.content.substring(0, 150))}${inst.content.length > 150 ? "..." : ""}
        </div>
        <div class="instruction-full-content" style="display: none;">
          ${Utils.escapeHtml(inst.content)}
        </div>
        <div class="instruction-meta">
          <small class="instruction-timestamp">
            ${inst.created_at ? `Created ${formatTimestamp(inst.created_at)}` : ""}
          </small>
        </div>
        <div class="instruction-actions">
          <label class="toggle-small">
            <input type="checkbox" ${inst.enabled ? "checked" : ""} onchange="window.aiPage.toggleInstruction(${inst.id}, this.checked)">
            <span class="toggle-track"></span>
          </label>
          <button class="btn-icon" onclick="window.aiPage.duplicateInstruction(${inst.id})" title="Duplicate">
            <i class="icon-copy"></i>
          </button>
          <button class="btn-icon" onclick="window.aiPage.editInstruction(${inst.id})" title="Edit">
            <i class="icon-edit"></i>
          </button>
          <button class="btn-icon btn-danger" onclick="window.aiPage.deleteInstruction(${inst.id})" title="Delete">
            <i class="icon-trash"></i>
          </button>
        </div>
      </div>
    `
      )
      .join("");

    // Setup drag and drop
    setupDragAndDrop();
  }

  /**
   * Get category label with icon
   */
  function getCategoryLabel(category) {
    const labels = {
      filtering: '<i class="icon-filter"></i> Filtering',
      trading: '<i class="icon-trending-up"></i> Trading',
      analysis: '<i class="icon-bar-chart"></i> Analysis',
      general: '<i class="icon-info"></i> General',
    };
    return labels[category] || category;
  }

  /**
   * Format timestamp
   */
  function formatTimestamp(timestamp) {
    if (!timestamp) return "";
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return "just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  }

  /**
   * Toggle instruction expanded state
   */
  function toggleInstructionExpanded(id) {
    const card = document.querySelector(`.instruction-card[data-id="${id}"]`);
    if (!card) return;

    const shortContent = card.querySelector(".instruction-content");
    const fullContent = card.querySelector(".instruction-full-content");

    if (fullContent.style.display === "none") {
      shortContent.style.display = "none";
      fullContent.style.display = "block";
      card.classList.add("instruction-expanded");
    } else {
      shortContent.style.display = "block";
      fullContent.style.display = "none";
      card.classList.remove("instruction-expanded");
    }
  }

  /**
   * Setup drag and drop for instructions
   */
  function setupDragAndDrop() {
    const cards = $$(".instruction-card");

    cards.forEach((card) => {
      // Drag start
      card.addEventListener("dragstart", (e) => {
        state.draggedItem = card;
        card.classList.add("dragging");
        e.dataTransfer.effectAllowed = "move";
      });

      // Drag end
      card.addEventListener("dragend", () => {
        card.classList.remove("dragging");
        state.draggedItem = null;
        // Remove all drag-over classes
        cards.forEach((c) => c.classList.remove("drag-over"));
      });

      // Drag over
      card.addEventListener("dragover", (e) => {
        e.preventDefault();
        if (state.draggedItem === card) return;
        card.classList.add("drag-over");
      });

      // Drag leave
      card.addEventListener("dragleave", () => {
        card.classList.remove("drag-over");
      });

      // Drop
      card.addEventListener("drop", async (e) => {
        e.preventDefault();
        card.classList.remove("drag-over");

        if (!state.draggedItem || state.draggedItem === card) return;

        // Get all card IDs in current order
        const container = $("#instructions-list");
        const allCards = Array.from(container.querySelectorAll(".instruction-card"));
        const draggedIndex = allCards.indexOf(state.draggedItem);
        const targetIndex = allCards.indexOf(card);

        // Reorder in DOM
        if (draggedIndex < targetIndex) {
          card.after(state.draggedItem);
        } else {
          card.before(state.draggedItem);
        }

        // Get new order
        const newOrder = Array.from(container.querySelectorAll(".instruction-card")).map((c) =>
          parseInt(c.dataset.id)
        );

        // Save new order to backend
        await reorderInstructions(newOrder);
      });
    });
  }

  /**
   * Save instruction order to backend
   */
  async function reorderInstructions(order) {
    try {
      const response = await fetch("/api/ai/instructions/reorder", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ order }),
      });

      if (!response.ok) throw new Error("Failed to reorder instructions");

      Utils.showToast({
        type: "success",
        title: "Reordered",
        message: "Instructions reordered successfully",
      });

      // Reload to get updated priorities
      await loadInstructions();
    } catch (error) {
      console.error("[AI] Error reordering instructions:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to reorder instructions",
      });
      // Reload to restore original order
      await loadInstructions();
    }
  }

  /**
   * Render templates
   */
  function renderTemplatesList(templates) {
    const container = $("#templates-list");
    if (!container) return;

    container.innerHTML = templates
      .map(
        (t) => `
      <div class="template-card" data-id="${t.id}">
        <div class="template-header">
          <span class="template-name">${Utils.escapeHtml(t.name)}</span>
          <span class="template-category badge-${t.category}">${getCategoryLabel(t.category)}</span>
        </div>
        <div class="template-tags">${t.tags.map((tag) => `<span class="tag">${tag}</span>`).join("")}</div>
        <div class="template-actions">
          <button class="btn btn-small btn-secondary" onclick="window.aiPage.previewTemplate('${t.id}')">
            <i class="icon-eye"></i> Preview
          </button>
          <button class="btn btn-small btn-primary" onclick="window.aiPage.customizeTemplate('${t.id}')">
            <i class="icon-edit"></i> Customize & Add
          </button>
        </div>
      </div>
    `
      )
      .join("");
  }

  /**
   * Preview template content
   */
  function previewTemplate(templateId) {
    const template = state.templates.find((t) => t.id === templateId);
    if (!template) return;

    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
      <div class="modal instruction-modal template-preview-modal">
        <div class="modal-header">
          <h3><i class="icon-eye"></i> Template Preview: ${Utils.escapeHtml(template.name)}</h3>
          <button class="modal-close" onclick="this.closest('.modal-overlay').remove()">×</button>
        </div>
        <div class="modal-body">
          <div class="template-preview-info">
            <div class="preview-meta">
              <span class="template-category badge-${template.category}">${getCategoryLabel(template.category)}</span>
              <div class="template-tags">${template.tags.map((tag) => `<span class="tag">${tag}</span>`).join("")}</div>
            </div>
          </div>
          <div class="template-preview-content">
            <h4>Content:</h4>
            <pre class="template-content-display">${Utils.escapeHtml(template.content)}</pre>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Close</button>
          <button class="btn btn-primary" onclick="window.aiPage.customizeTemplate('${template.id}'); this.closest('.modal-overlay').remove();">
            <i class="icon-edit"></i> Customize & Add
          </button>
        </div>
      </div>
    `;
    document.body.appendChild(modal);
  }

  /**
   * Customize template before adding
   */
  function customizeTemplate(templateId) {
    const template = state.templates.find((t) => t.id === templateId);
    if (!template) return;

    // Show modal pre-filled with template data
    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
      <div class="modal instruction-modal">
        <div class="modal-header">
          <h3><i class="icon-edit"></i> Customize Template</h3>
          <button class="modal-close" onclick="this.closest('.modal-overlay').remove()">×</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>Name</label>
            <input type="text" id="inst-name" value="${Utils.escapeHtml(template.name)}" placeholder="e.g., Liquidity Guard">
          </div>
          <div class="form-group">
            <label>Category</label>
            <select id="inst-category">
              <option value="filtering" ${template.category === "filtering" ? "selected" : ""}>Filtering</option>
              <option value="trading" ${template.category === "trading" ? "selected" : ""}>Trading</option>
              <option value="analysis" ${template.category === "analysis" ? "selected" : ""}>Analysis</option>
              <option value="general" ${template.category === "general" ? "selected" : ""}>General</option>
            </select>
            <small class="form-hint">${getCategoryHint(template.category)}</small>
          </div>
          <div class="form-group">
            <label>Content</label>
            <textarea id="inst-content" rows="12" class="instruction-editor" placeholder="Enter your instruction...">${Utils.escapeHtml(template.content)}</textarea>
            <div class="char-count">
              <span id="char-counter">${template.content.length}</span> characters
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
          <button class="btn btn-primary" onclick="window.aiPage.saveNewInstruction()">
            <i class="icon-plus"></i> Create
          </button>
        </div>
      </div>
    `;
    document.body.appendChild(modal);

    // Add character counter
    const textarea = $("#inst-content");
    const counter = $("#char-counter");
    if (textarea && counter) {
      textarea.addEventListener("input", () => {
        counter.textContent = textarea.value.length;
      });
    }
  }

  /**
   * Get category hint text
   */
  function getCategoryHint(category) {
    const hints = {
      filtering:
        "Instructions for token filtering decisions - helps AI determine which tokens to skip",
      trading: "Instructions for entry/exit analysis - guides AI on trading decisions",
      analysis: "General market analysis guidelines - shapes how AI analyzes market conditions",
      general: "Other instructions - miscellaneous AI behavior customizations",
    };
    return hints[category] || "";
  }

  /**
   * Create instruction (with modal)
   */
  async function createInstruction() {
    // Show modal with form
    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
      <div class="modal instruction-modal">
        <div class="modal-header">
          <h3><i class="icon-plus"></i> Create Instruction</h3>
          <button class="modal-close" onclick="this.closest('.modal-overlay').remove()">×</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>Name</label>
            <input type="text" id="inst-name" placeholder="e.g., Liquidity Guard">
          </div>
          <div class="form-group">
            <label>Category</label>
            <select id="inst-category">
              <option value="filtering">Filtering</option>
              <option value="trading">Trading</option>
              <option value="analysis">Analysis</option>
              <option value="general">General</option>
            </select>
            <small class="form-hint" id="category-hint">Instructions for token filtering decisions</small>
          </div>
          <div class="form-group">
            <label>Content</label>
            <textarea id="inst-content" rows="12" class="instruction-editor" placeholder="Enter your instruction..."></textarea>
            <div class="char-count">
              <span id="char-counter">0</span> characters
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
          <button class="btn btn-primary" onclick="window.aiPage.saveNewInstruction()">
            <i class="icon-plus"></i> Create
          </button>
        </div>
      </div>
    `;
    document.body.appendChild(modal);

    // Setup category hint updater
    const categorySelect = $("#inst-category");
    const hintEl = $("#category-hint");
    if (categorySelect && hintEl) {
      categorySelect.addEventListener("change", () => {
        hintEl.textContent = getCategoryHint(categorySelect.value);
      });
    }

    // Setup character counter
    const textarea = $("#inst-content");
    const counter = $("#char-counter");
    if (textarea && counter) {
      textarea.addEventListener("input", () => {
        counter.textContent = textarea.value.length;
      });
    }
  }

  /**
   * Save new instruction
   */
  async function saveNewInstruction() {
    const name = $("#inst-name")?.value;
    const category = $("#inst-category")?.value || "general";
    const content = $("#inst-content")?.value;

    if (!name || !content) {
      Utils.showToast({
        type: "warning",
        title: "Missing Fields",
        message: "Name and content are required",
      });
      return;
    }

    try {
      const response = await fetch("/api/ai/instructions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, category, content }),
      });

      if (!response.ok) throw new Error("Failed to create instruction");

      document.querySelector(".modal-overlay")?.remove();
      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: "Created",
        message: "Instruction created successfully",
      });
    } catch (error) {
      console.error("[AI] Error creating instruction:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to create instruction",
      });
    }
  }

  /**
   * Toggle instruction enabled state
   */
  async function toggleInstruction(id, enabled) {
    try {
      await fetch(`/api/ai/instructions/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
      });
    } catch (error) {
      console.error("[AI] Error toggling instruction:", error);
    }
  }

  /**
   * Edit instruction
   */
  async function editInstruction(id) {
    try {
      // Fetch instruction data
      const response = await fetch(`/api/ai/instructions/${id}`);
      if (!response.ok) throw new Error("Failed to load instruction");
      const inst = await response.json();

      // Show modal pre-filled with data
      const modal = document.createElement("div");
      modal.className = "modal-overlay";
      modal.innerHTML = `
        <div class="modal instruction-modal instruction-edit-modal">
          <div class="modal-header">
            <h3><i class="icon-edit"></i> Edit Instruction</h3>
            <button class="modal-close" onclick="this.closest('.modal-overlay').remove()">×</button>
          </div>
          <div class="modal-body">
            <div class="form-group">
              <label>Name</label>
              <input type="text" id="edit-inst-name" value="${Utils.escapeHtml(inst.name)}" placeholder="e.g., Liquidity Guard">
            </div>
            <div class="form-group">
              <label>Category</label>
              <select id="edit-inst-category">
                <option value="filtering" ${inst.category === "filtering" ? "selected" : ""}>Filtering</option>
                <option value="trading" ${inst.category === "trading" ? "selected" : ""}>Trading</option>
                <option value="analysis" ${inst.category === "analysis" ? "selected" : ""}>Analysis</option>
                <option value="general" ${inst.category === "general" ? "selected" : ""}>General</option>
              </select>
              <small class="form-hint" id="edit-category-hint">${getCategoryHint(inst.category)}</small>
            </div>
            <div class="form-group">
              <label>Content</label>
              <textarea id="edit-inst-content" rows="12" class="instruction-editor" placeholder="Enter your instruction...">${Utils.escapeHtml(inst.content)}</textarea>
              <div class="char-count">
                <span id="edit-char-counter">${inst.content.length}</span> characters
              </div>
            </div>
            <div class="instruction-preview-section">
              <h4><i class="icon-eye"></i> Preview</h4>
              <div class="instruction-preview">
                <div class="preview-header">
                  <span class="preview-name">${Utils.escapeHtml(inst.name)}</span>
                  <span class="preview-category badge-${inst.category}">${getCategoryLabel(inst.category)}</span>
                </div>
                <div class="preview-content">${Utils.escapeHtml(inst.content)}</div>
              </div>
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
            <button class="btn btn-primary" onclick="window.aiPage.saveEditedInstruction(${id})">
              <i class="icon-save"></i> Save Changes
            </button>
          </div>
        </div>
      `;
      document.body.appendChild(modal);

      // Setup live preview updater
      const nameInput = $("#edit-inst-name");
      const categorySelect = $("#edit-inst-category");
      const contentTextarea = $("#edit-inst-content");
      const previewName = modal.querySelector(".preview-name");
      const previewCategory = modal.querySelector(".preview-category");
      const previewContent = modal.querySelector(".preview-content");
      const hintEl = $("#edit-category-hint");
      const counter = $("#edit-char-counter");

      function updatePreview() {
        if (nameInput && previewName) {
          previewName.textContent = nameInput.value || "Untitled";
        }
        if (categorySelect && previewCategory) {
          const cat = categorySelect.value;
          previewCategory.className = `preview-category badge-${cat}`;
          previewCategory.innerHTML = getCategoryLabel(cat);
        }
        if (contentTextarea && previewContent) {
          previewContent.textContent = contentTextarea.value;
        }
      }

      if (nameInput) {
        nameInput.addEventListener("input", updatePreview);
      }
      if (categorySelect) {
        categorySelect.addEventListener("change", () => {
          updatePreview();
          if (hintEl) {
            hintEl.textContent = getCategoryHint(categorySelect.value);
          }
        });
      }
      if (contentTextarea) {
        contentTextarea.addEventListener("input", () => {
          updatePreview();
          if (counter) {
            counter.textContent = contentTextarea.value.length;
          }
        });
      }
    } catch (error) {
      console.error("[AI] Error loading instruction for edit:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to load instruction data",
      });
    }
  }

  /**
   * Save edited instruction
   */
  async function saveEditedInstruction(id) {
    const name = $("#edit-inst-name")?.value;
    const category = $("#edit-inst-category")?.value || "general";
    const content = $("#edit-inst-content")?.value;

    if (!name || !content) {
      Utils.showToast({
        type: "warning",
        title: "Missing Fields",
        message: "Name and content are required",
      });
      return;
    }

    try {
      const response = await fetch(`/api/ai/instructions/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, category, content }),
      });

      if (!response.ok) throw new Error("Failed to update instruction");

      document.querySelector(".modal-overlay")?.remove();
      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: "Updated",
        message: "Instruction updated successfully",
      });
    } catch (error) {
      console.error("[AI] Error updating instruction:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to update instruction",
      });
    }
  }

  /**
   * Delete instruction
   */
  async function deleteInstruction(id) {
    const confirmed = await ConfirmationDialog.show({
      title: "Delete Instruction",
      message: "Are you sure you want to delete this instruction?",
      confirmText: "Delete",
      cancelText: "Cancel",
      type: "danger",
    });

    if (!confirmed) return;

    try {
      await fetch(`/api/ai/instructions/${id}`, { method: "DELETE" });
      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: "Deleted",
        message: "Instruction deleted successfully",
      });
    } catch (error) {
      console.error("[AI] Error deleting instruction:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to delete instruction",
      });
    }
  }

  /**
   * Duplicate instruction
   */
  async function duplicateInstruction(id) {
    try {
      // Fetch the instruction to duplicate
      const response = await fetch(`/api/ai/instructions/${id}`);
      if (!response.ok) throw new Error("Failed to load instruction");
      const inst = await response.json();

      // Create a copy with modified name
      const copyName = `${inst.name} (Copy)`;

      const createResponse = await fetch("/api/ai/instructions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: copyName,
          category: inst.category,
          content: inst.content,
        }),
      });

      if (!createResponse.ok) throw new Error("Failed to duplicate instruction");

      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: "Duplicated",
        message: "Instruction duplicated successfully",
      });
    } catch (error) {
      console.error("[AI] Error duplicating instruction:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to duplicate instruction",
      });
    }
  }

  /**
   * Use template to create instruction
   */
  async function useTemplate(templateId) {
    const template = state.templates.find((t) => t.id === templateId);
    if (!template) return;

    try {
      await fetch("/api/ai/instructions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: template.name,
          category: template.category,
          content: template.content,
        }),
      });
      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: "Created",
        message: `Instruction created from template: ${template.name}`,
      });
    } catch (error) {
      console.error("[AI] Error using template:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to create instruction from template",
      });
    }
  }

  // ============================================================================
  // History Tab
  // ============================================================================

  /**
   * Load history
   */
  async function loadHistory(page = 1) {
    try {
      const response = await fetch(`/api/ai/history?page=${page}&per_page=20`);
      if (!response.ok) throw new Error("Failed to load history");
      const data = await response.json();
      state.historyPage = page;
      state.historyTotal = data.total;
      renderHistoryList(data.decisions || [], data.total, page);
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
  function renderHistoryList(decisions, total, page) {
    const container = $("#history-list");
    if (!container) return;

    if (decisions.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <i class="icon-clock"></i>
          <p>No AI decisions yet</p>
          <small>Decisions will appear here as AI evaluates tokens</small>
        </div>`;
      return;
    }

    const rows = decisions
      .map(
        (d) => `
      <tr class="decision-row ${d.decision}">
        <td class="decision-time">${new Date(d.created_at).toLocaleString()}</td>
        <td class="decision-token">
          <span class="token-symbol">${Utils.escapeHtml(d.symbol || "Unknown")}</span>
          <span class="token-mint" title="${d.mint}">${d.mint.slice(0, 8)}...</span>
        </td>
        <td class="decision-result">
          <span class="badge badge-${d.decision === "pass" ? "success" : "danger"}">${d.decision}</span>
        </td>
        <td class="decision-confidence">${d.confidence}%</td>
        <td class="decision-provider">${d.provider}</td>
        <td class="decision-latency">${d.latency_ms.toFixed(0)}ms</td>
        <td class="decision-cached">${d.cached ? '<i class="icon-zap" title="Cached"></i>' : "-"}</td>
      </tr>
    `
      )
      .join("");

    container.innerHTML = `
      <table class="history-table">
        <thead>
          <tr>
            <th>Time</th>
            <th>Token</th>
            <th>Decision</th>
            <th>Confidence</th>
            <th>Provider</th>
            <th>Latency</th>
            <th>Cached</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
      ${
        total > 20
          ? `
        <div class="pagination">
          <button class="btn btn-small" ${page <= 1 ? "disabled" : ""} onclick="window.aiPage.loadHistory(${page - 1})">Previous</button>
          <span>Page ${page} of ${Math.ceil(total / 20)}</span>
          <button class="btn btn-small" ${page >= Math.ceil(total / 20) ? "disabled" : ""} onclick="window.aiPage.loadHistory(${page + 1})">Next</button>
        </div>
      `
          : ""
      }
    `;
  }

  // ============================================================================
  // API Export for inline event handlers
  // ============================================================================

  // Assign functions to API object for external access
  api.createInstruction = createInstruction;
  api.saveNewInstruction = saveNewInstruction;
  api.toggleInstruction = toggleInstruction;
  api.editInstruction = editInstruction;
  api.saveEditedInstruction = saveEditedInstruction;
  api.deleteInstruction = deleteInstruction;
  api.duplicateInstruction = duplicateInstruction;
  api.toggleInstructionExpanded = toggleInstructionExpanded;
  api.useTemplate = useTemplate;
  api.previewTemplate = previewTemplate;
  api.customizeTemplate = customizeTemplate;
  api.loadHistory = loadHistory;

  // ============================================================================
  // Lifecycle Hooks
  // ============================================================================

  return {
    /**
     * Init - called once when page is first loaded
     */
    async init(_ctx) {
      console.log("[AI] Initializing");

      // Initialize sidebar navigation
      initSubTabs();

      // Set initial active state
      updateSidebarNavigation(DEFAULT_TAB);

      // Show the initial tab content
      switchTab(state.currentTab);

      // Setup event handlers
      setupSettingsHandlers();
      setupTestingHandlers();

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
          { label: "AI Status", intervalMs: 5000 }
        )
      );

      providersPoller = ctx.managePoller(
        new Poller(
          async () => {
            if (state.currentTab === "providers") {
              await loadProviders();
            }
          },
          { label: "AI Providers", intervalMs: 10000 }
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

      // Load initial data immediately and start appropriate poller
      if (state.currentTab === "stats") {
        await loadAiStatus();
        statusPoller.start();
      } else if (state.currentTab === "providers") {
        await loadProviders();
        providersPoller.start();
      } else if (state.currentTab === "settings") {
        await loadConfig();
        await loadCacheStats();
        cachePoller.start();
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

// Expose API functions globally for inline event handlers
window.aiPage = lifecycle.api;

// Register the page
registerPage("ai", lifecycle);
