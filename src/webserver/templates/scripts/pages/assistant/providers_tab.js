import { $ } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import { playSuccess, playError } from "../../core/sounds.js";

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
};

export function createProvidersTab({ state, _eventCleanups, loadConfig }) {
  // Hash guard — skip re-render when polled providers data is unchanged
  let _lastProvidersKey = null;

  // ============================================================================
  // Providers Tab
  // ============================================================================

  /**
   * Load and render providers
   */
  async function loadProviders() {
    try {
      const response = await fetch("/api/llm/providers");
      if (!response.ok) throw new Error("Failed to fetch providers");

      const data = await response.json();
      state.providers = data.providers || [];

      // Store default_provider from API response for use in rendering
      if (data.default_provider) {
        state.defaultProvider = data.default_provider;
      }

      renderProviders(state.providers);
    } catch (error) {
      console.error("[Assistant] Failed to load providers:", error);
      Utils.showToast({ key: "assistant-load", type: "error", title: "Could not load providers" });
    }
  }

  /**
   * Render provider list view
   */
  function renderProviders(providers) {
    const key = JSON.stringify({ providers, default: state.defaultProvider });
    if (key === _lastProvidersKey) return;
    _lastProvidersKey = key;

    const container = $("#providers-list");
    if (!container) return;

    // Get default provider from state (loaded from API) or fallback to config
    const defaultProvider = state.defaultProvider || state.config?.default_provider || "";

    // Get all provider IDs
    const allProviderIds = Object.keys(PROVIDER_NAMES);

    container.innerHTML = allProviderIds
      .map((providerId) => {
        const provider = providers.find((p) => p.id === providerId) || {
          id: providerId,
          enabled: false,
          api_key: "",
          model: "",
        };

        const isDefault = providerId === defaultProvider;
        const name = PROVIDER_NAMES[providerId];

        // Regular providers (API key-based)
        const isConfigured = provider.enabled && provider.api_key && provider.model;

        return `
          <div class="provider-item ${isDefault ? "is-default" : ""}" data-provider="${providerId}">
            <label class="provider-select" title="Set as default">
              <input type="radio" name="default-provider" value="${providerId}"
                     aria-label="Use ${name} as the default provider"
                     ${isDefault ? "checked" : ""}
                     onchange="window.assistantPage.setDefaultProvider('${providerId}')">
            </label>
            
            <div class="provider-logo">
              <img src="/assets/providers/${providerId}.png" alt="${name}" onerror="this.style.display='none'; this.nextElementSibling.style.display='flex';">
              <div class="provider-logo-fallback" style="display: none;">${name.charAt(0)}</div>
            </div>
            
            <div class="provider-info">
              <div class="provider-name">${name}</div>
              <div class="provider-model">${provider.model || "Not configured"}</div>
            </div>
            
            <div class="provider-status">
              ${isConfigured ? '<span class="status-badge configured"><i class="icon-check"></i> Ready</span>' : '<span class="status-badge not-configured">Not Set Up</span>'}
              ${isDefault ? '<span class="status-badge default">Default</span>' : ""}
            </div>
            
            <div class="provider-actions">
              ${isConfigured ? `<button class="provider-btn test-btn" onclick="window.assistantPage.testProviderFromList('${providerId}')"><i class="icon-zap"></i> Test</button>` : ""}
              <button class="provider-btn ${isConfigured ? "" : "primary"}" onclick="window.assistantPage.configureProvider('${providerId}')">
                <i class="icon-${isConfigured ? "settings" : "plus"}"></i> ${isConfigured ? "Edit" : "Configure"}
              </button>
            </div>
          </div>
        `;
      })
      .join("");
  }

  /**
   * Set default provider
   */
  async function setDefaultProvider(providerId) {
    try {
      // Default provider is a master LLM field, owned by /api/llm/config.
      const response = await fetch("/api/llm/config", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ default_provider: providerId }),
      });

      if (!response.ok) throw new Error("Failed to set default provider");

      Utils.showToast({
        type: "success",
        title: "Default Provider Set",
        message: `${PROVIDER_NAMES[providerId]} is now the default provider`,
      });

      // Reload config and providers to update UI
      await loadConfig();
      await loadProviders();
    } catch (error) {
      console.error("[Assistant] Error setting default provider:", error);
      // A native radio updates immediately. Restore the persisted selection if
      // the mutation fails instead of leaving the failed choice checked.
      _lastProvidersKey = null;
      renderProviders(state.providers);
      Utils.showToast({ type: "error", title: "Failed to set default provider" });
    }
  }

  /**
   * Test provider from list view
   */
  async function testProviderFromList(providerId) {
    try {
      Utils.showToast({
        type: "info",
        title: "Testing Provider",
        message: `Testing ${PROVIDER_NAMES[providerId]}...`,
      });

      const response = await fetch(`/api/llm/providers/${providerId}/test`, {
        method: "POST",
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || errorData.message || `HTTP ${response.status}`);
      }

      const result = await response.json();

      if (result.success) {
        Utils.showToast({
          type: "success",
          title: "Connection Successful",
          message: `${PROVIDER_NAMES[providerId]} is working correctly`,
        });
      } else {
        throw new Error(result.error || "Test failed");
      }
    } catch (error) {
      console.error(`[Assistant] Provider test failed for ${providerId}:`, error);
      Utils.showToast({
        type: "error",
        title: "Test Failed",
        message: error.message,
      });
    }
  }

  /**
   * Open provider configuration modal
   */
  function configureProvider(providerId) {
    const provider = state.providers.find((p) => p.id === providerId) || {
      id: providerId,
      enabled: false,
      has_api_key: false,
      model: "",
    };

    const name = PROVIDER_NAMES[providerId];
    const hasApiKey = provider.has_api_key || false;

    // Create and show modal
    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
      <div class="modal-dialog provider-config-modal">
        <div class="modal-header">
          <h3>
            <span class="provider-icon"><i class="icon-bot"></i></span>
            ${name} Configuration
          </h3>
          <button class="modal-close" onclick="this.closest('.modal-overlay').remove()">
            <i class="icon-x"></i>
          </button>
        </div>
        <div class="modal-body">
          <!-- API Key Section -->
          <div class="form-group">
            <label for="modal-api-key">
              API Key
              ${hasApiKey ? '<span class="key-status key-saved"><i class="icon-circle-check"></i> Key saved</span>' : '<span class="key-status key-missing"><i class="icon-circle-alert"></i> No key set</span>'}
            </label>
            <div class="api-key-input-wrapper">
              <input type="password" id="modal-api-key" class="form-control" 
                     placeholder="${hasApiKey ? "Enter new key to update..." : "Enter API key..."}" 
                     value="">
              <button type="button" class="api-key-toggle" id="toggle-api-key" title="Show/Hide">
                <i class="icon-eye"></i>
              </button>
            </div>
            <small class="form-help">${hasApiKey ? "Leave empty to keep current key, or enter a new key to update" : "Your API key is stored securely and never shared"}</small>
          </div>
          
          <!-- Model Section -->
          <div class="form-group model-section">
            <label for="modal-model">Model</label>
            <div class="model-input-wrapper">
              <input type="text" id="modal-model" class="form-control" 
                     placeholder="e.g., gpt-4, claude-3-opus..." value="${provider.model || ""}">
            </div>
            <small class="form-help">The model to use for Assistant analysis requests</small>
          </div>
          
          <!-- Enable Checkbox -->
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" id="modal-enabled" ${provider.enabled ? "checked" : ""}>
              <span>Enable this provider</span>
            </label>
            <small class="form-help">When enabled, this provider will be available for Assistant analysis</small>
          </div>
          
          <!-- Test Connection Section -->
          <div class="test-connection-section">
            <div class="test-connection-header">
              <span class="test-connection-title">Connection Test</span>
              <button type="button" class="btn btn-sm btn-secondary" id="test-connection-btn">
                <i class="icon-zap"></i>
                Test Connection
              </button>
            </div>
            <div class="test-connection-result" id="test-result">
              <span class="test-result-message"></span>
              <dl class="test-result-details"></dl>
            </div>
          </div>
        </div>
        <div class="modal-footer modal-footer-split">
          <div class="footer-left">
            <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
          </div>
          <div class="footer-right">
            <button class="btn btn-primary" id="modal-save-btn">
              <i class="icon-save"></i>
              Save Configuration
            </button>
          </div>
        </div>
      </div>
    `;

    document.body.appendChild(modal);

    // API Key visibility toggle
    const apiKeyInput = modal.querySelector("#modal-api-key");
    const toggleBtn = modal.querySelector("#toggle-api-key");
    toggleBtn.addEventListener("click", () => {
      const isPassword = apiKeyInput.type === "password";
      apiKeyInput.type = isPassword ? "text" : "password";
      toggleBtn.querySelector("i").className = isPassword ? "icon-eye-off" : "icon-eye";
    });

    // Test connection handler
    const testBtn = modal.querySelector("#test-connection-btn");
    const testResult = modal.querySelector("#test-result");
    const testMessage = testResult.querySelector(".test-result-message");
    const testDetails = testResult.querySelector(".test-result-details");
    const hasExistingKey = provider.has_api_key || false;

    testBtn.addEventListener("click", async () => {
      const apiKey = apiKeyInput.value.trim();
      const model = modal.querySelector("#modal-model").value.trim();

      // Allow testing with existing key if no new key entered
      if (!apiKey && !hasExistingKey) {
        Utils.showToast({
          type: "warning",
          title: "Missing API Key",
          message: "Please enter an API key first",
        });
        return;
      }

      // First save the config temporarily
      testBtn.disabled = true;
      testBtn.innerHTML = '<i class="icon-loader spin"></i> Testing...';
      testResult.className = "test-connection-result";

      try {
        // Only save new key if entered, otherwise test with existing
        if (apiKey) {
          const saveRes = await fetch(`/api/llm/providers/${providerId}`, {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              enabled: true,
              api_key: apiKey,
              model: model || getDefaultModel(providerId),
            }),
          });

          if (!saveRes.ok) throw new Error("Failed to save config for testing");
        }

        // Now test the provider
        const response = await fetch(`/api/llm/providers/${providerId}/test`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
        });

        const data = await response.json();

        if (response.ok && data.success) {
          testResult.className = "test-connection-result visible success";
          testMessage.innerHTML = '<i class="icon-circle-check"></i> Connection successful!';
          testDetails.innerHTML = `
            <dt>Model:</dt><dd>${Utils.escapeHtml(String(data.model || "N/A"))}</dd>
            <dt>Latency:</dt><dd>${Math.round(data.latency_ms || 0)}ms</dd>
            <dt>Tokens:</dt><dd>${parseInt(data.tokens_used) || 0}</dd>
          `;
          playSuccess();
        } else {
          throw new Error(data.error?.message || "Test failed");
        }
      } catch (error) {
        testResult.className = "test-connection-result visible error";
        testMessage.innerHTML = `<i class="icon-circle-x"></i> ${Utils.escapeHtml(String(error.message))}`;
        testDetails.innerHTML = "";
        playError();
      } finally {
        testBtn.disabled = false;
        testBtn.innerHTML = '<i class="icon-zap"></i> Test Connection';
      }
    });

    // Add save handler
    const saveBtn = modal.querySelector("#modal-save-btn");
    saveBtn.addEventListener("click", async () => {
      const apiKey = modal.querySelector("#modal-api-key").value.trim();
      const model = modal.querySelector("#modal-model").value.trim();
      const enabled = modal.querySelector("#modal-enabled").checked;
      const hasExistingKey = provider.has_api_key || false;

      // Only require new API key if enabling and no existing key
      if (enabled && !apiKey && !hasExistingKey) {
        Utils.showToast({
          type: "warning",
          title: "Missing API Key",
          message: "Please enter an API key to enable this provider",
        });
        return;
      }

      if (enabled && !model) {
        Utils.showToast({
          type: "warning",
          title: "Missing Model",
          message: "Please enter a model name",
        });
        return;
      }

      try {
        saveBtn.disabled = true;
        saveBtn.innerHTML = '<i class="icon-loader spin"></i> Saving...';

        // Only include api_key if user entered a new one
        const payload = { enabled, model };
        if (apiKey) {
          payload.api_key = apiKey;
        }

        const response = await fetch(`/api/llm/providers/${providerId}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });

        if (!response.ok) throw new Error("Failed to save provider");

        Utils.showToast({
          type: "success",
          title: "Provider Saved",
          message: `${name} configuration saved`,
        });

        modal.remove();
        await loadProviders();
      } catch (error) {
        console.error(`[Assistant] Failed to save provider ${providerId}:`, error);
        Utils.showToast({ type: "error", title: "Failed to save provider configuration" });
        saveBtn.disabled = false;
        saveBtn.innerHTML = '<i class="icon-save"></i> Save Configuration';
      }
    });
  }

  /**
   * Get default model for provider
   */
  function getDefaultModel(providerId) {
    const defaults = {
      openai: "gpt-4",
      anthropic: "claude-3-5-sonnet-20241022",
      groq: "llama-3.1-70b-versatile",
      deepseek: "deepseek-chat",
      gemini: "gemini-pro",
      ollama: "llama3.2",
      together: "meta-llama/Llama-3-70b-chat-hf",
      openrouter: "openai/gpt-4",
      mistral: "mistral-large-latest",
    };
    return defaults[providerId] || "gpt-4";
  }

  // Return public API
  return {
    loadProviders,
    renderProviders,
    setDefaultProvider,
    testProviderFromList,
    configureProvider,
  };
}
