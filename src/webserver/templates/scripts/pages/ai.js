import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { playToggleOn, playError } from "../core/sounds.js";

// Sub-tabs configuration
const SUB_TABS = [
  { id: "stats", label: '<i class="icon-chart-bar"></i> Stats' },
  { id: "providers", label: '<i class="icon-server"></i> Providers' },
  { id: "settings", label: '<i class="icon-settings"></i> Settings' },
  { id: "testing", label: '<i class="icon-flask"></i> Testing' },
];

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
  let tabBar = null;
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
  };

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
   * Initialize sub-tabs
   */
  function initSubTabs() {
    const container = $("#subTabsContainer");
    if (!container) return;

    tabBar = new TabBar({
      container,
      tabs: SUB_TABS,
      defaultTab: DEFAULT_TAB,
      onTabChange: (tabId) => {
        if (tabId !== state.currentTab) {
          state.currentTab = tabId;
          switchTab(tabId);
        }
      },
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

    // Hide all tabs
    const allTabs = $$(".ai-tab-content");
    allTabs.forEach((tab) => {
      tab.style.display = "none";
    });

    // Show the selected tab
    const selectedTab = $(`#${tabId}-tab`);
    if (selectedTab) {
      selectedTab.style.display = "block";
    }

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
  // Lifecycle Hooks
  // ============================================================================

  return {
    /**
     * Before mount - called before page HTML is inserted
     */
    async beforeMount() {
      console.log("[AI] Before mount");
    },

    /**
     * Mount - called after HTML is inserted
     */
    async mounted(ctx) {
      console.log("[AI] Mounted");

      // Initialize sub-tabs
      initSubTabs();

      // Integrate with lifecycle for auto-cleanup
      if (tabBar) {
        ctx.manageTabBar(tabBar);
        tabBar.show();
      }

      // Sync state with tab bar's restored state
      const activeTab = tabBar ? tabBar.getActiveTab() : DEFAULT_TAB;
      if (activeTab) {
        state.currentTab = activeTab;
      }

      // Show the active tab content
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

      // Re-register tab bar
      if (tabBar) {
        ctx.manageTabBar(tabBar);
        tabBar.show({ force: true });
      }

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
     * Before unmount - cleanup
     */
    async beforeUnmount() {
      console.log("[AI] Before unmount");

      // Clean up event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

      // Cleanup tab bar
      if (tabBar) {
        TabBarManager.unregister("ai");
        tabBar = null;
      }
    },
  };
}

// Register the page
registerPage("ai", createLifecycle);
