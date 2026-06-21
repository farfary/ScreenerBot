import { $ } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import { ConfirmationDialog } from "../../ui/confirmation_dialog.js";
import { playToggleOn, playError } from "../../core/sounds.js";

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

export function createProvidersTab({ state, _eventCleanups, loadConfig }) {
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

      // Store default_provider from API response for use in rendering
      if (data.default_provider) {
        state.defaultProvider = data.default_provider;
      }

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
   * Render provider list view
   */
  function renderProviders(providers) {
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

        // Handle Assistant specially (OAuth-based)
        if (providerId === "Assistant") {
          const isAuthenticated = state.AssistantAuth.authenticated;
          const isConfigured = isAuthenticated && provider.enabled && provider.model;

          return `
          <div class="provider-item ${isDefault ? "is-default" : ""} ${isConfigured ? "is-configured" : ""}" data-provider="${providerId}">
            <div class="provider-select" onclick="window.aiPage.setDefaultProvider('${providerId}')" title="Set as default">
              <div class="provider-radio"></div>
            </div>
            
            <div class="provider-logo">
              <img src="/assets/providers/${providerId}.png" alt="${name}" onerror="this.style.display='none'; this.nextElementSibling.style.display='flex';">
              <div class="provider-logo-fallback" style="display: none;">${name.charAt(0)}</div>
            </div>
            
            <div class="provider-info">
              <div class="provider-name">${name}</div>
              <div class="provider-model">${provider.model || "Not configured"}</div>
            </div>
            
            <div class="provider-status">
              ${isConfigured ? '<span class="status-badge configured"><i class="icon-check"></i> Ready</span>' : isAuthenticated ? '<span class="status-badge not-configured">Not Set Up</span>' : '<span class="status-badge not-configured">Not Connected</span>'}
              ${isDefault ? '<span class="status-badge default">Default</span>' : ""}
            </div>
            
            <div class="provider-actions">
              <button class="provider-btn ${isAuthenticated ? "" : "primary"}" onclick="window.aiPage.configureProvider('${providerId}')">
                <i class="icon-${isAuthenticated ? "settings" : "github"}"></i> ${isAuthenticated ? "Configure" : "Login with GitHub"}
              </button>
            </div>
          </div>
        `;
        }

        // Regular providers (API key-based)
        const isConfigured = provider.enabled && provider.api_key && provider.model;

        return `
          <div class="provider-item ${isDefault ? "is-default" : ""} ${isConfigured ? "is-configured" : ""}" data-provider="${providerId}">
            <div class="provider-select" onclick="window.aiPage.setDefaultProvider('${providerId}')" title="Set as default">
              <div class="provider-radio"></div>
            </div>
            
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
              ${isConfigured ? `<button class="provider-btn test-btn" onclick="window.aiPage.testProviderFromList('${providerId}')"><i class="icon-zap"></i> Test</button>` : ""}
              <button class="provider-btn ${isConfigured ? "" : "primary"}" onclick="window.aiPage.configureProvider('${providerId}')">
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
      const response = await fetch("/api/ai/config", {
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
      console.error("[AI] Error setting default provider:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to set default provider",
      });
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

      const response = await fetch(`/api/ai/providers/${providerId}/test`, {
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
      console.error(`[AI] Provider test failed for ${providerId}:`, error);
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
    // Handle Assistant OAuth separately
    if (providerId === "Assistant") {
      configureAssistant();
      return;
    }

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
          const saveRes = await fetch(`/api/ai/providers/${providerId}`, {
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
        const response = await fetch(`/api/ai/providers/${providerId}/test`, {
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
          playToggleOn();
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

        const response = await fetch(`/api/ai/providers/${providerId}`, {
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
        console.error(`[AI] Failed to save provider ${providerId}:`, error);
        Utils.showToast({
          type: "error",
          title: "Error",
          message: "Failed to save provider configuration",
        });
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
      Assistant: "gpt-4o",
    };
    return defaults[providerId] || "gpt-4";
  }

  // ============================================================================
  // Assistant OAuth Functions
  // ============================================================================

  /**
   * Check an LLM provider authentication status
   */
  async function checkAssistantAuthStatus() {
    try {
      const response = await fetch("/api/ai/Assistant/auth/status");
      const data = await response.json();
      state.AssistantAuth = {
        authenticated: data.authenticated || false,
        hasGithubToken: data.has_github_token || false,
      };
      return data.authenticated;
    } catch (error) {
      console.error("[AI] Failed to check Assistant auth:", error);
      return false;
    }
  }

  /**
   * Start an LLM provider device code flow
   */
  async function startAssistantAuth(modal) {
    try {
      const statusDiv = modal.querySelector(".Assistant-auth-status");
      statusDiv.innerHTML = '<div class="loading-spinner"><i class="icon-loader spin"></i> Starting authentication...</div>';

      const response = await fetch("/api/ai/Assistant/auth/start", {
        method: "POST",
      });

      if (!response.ok) throw new Error("Failed to start authentication");

      const data = await response.json();

      // Show user code and verification URL
      statusDiv.innerHTML = `
        <div class="Assistant-device-flow">
          <div class="device-flow-header">
            <i class="icon-info"></i>
            <span>Authentication Required</span>
          </div>
          <div class="device-flow-body">
            <p>Enter this code on GitHub:</p>
            <div class="user-code">${data.user_code}</div>
            <a href="${data.verification_uri}" target="_blank" class="btn btn-primary">
              <i class="icon-external-link"></i>
              Open GitHub
            </a>
            <p class="help-text">Waiting for authorization...</p>
          </div>
        </div>
      `;

      // Auto-open verification URL
      window.open(data.verification_uri, "_blank");

      // Start polling
      pollAssistantAuth(data.device_code, data.interval, modal);
    } catch (error) {
      console.error("[AI] Failed to start Assistant auth:", error);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: error.message || "Failed to start authentication",
      });
    }
  }

  /**
   * Poll for an LLM provider authentication completion
   */
  async function pollAssistantAuth(deviceCode, interval, modal) {
    const maxAttempts = 60; // 5 minutes max
    let attempts = 0;

    const poll = async () => {
      if (attempts >= maxAttempts) {
        const statusDiv = modal.querySelector(".Assistant-auth-status");
        statusDiv.innerHTML = `
          <div class="auth-error">
            <i class="icon-circle-x"></i>
            Authentication timed out. Please try again.
          </div>
        `;
        Utils.showToast({
          type: "error",
          title: "Timeout",
          message: "Authentication timed out",
        });
        return;
      }

      try {
        const response = await fetch("/api/ai/Assistant/auth/poll", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ device_code: deviceCode }),
        });

        const data = await response.json();

        if (data.success) {
          state.AssistantAuth = {
            authenticated: true,
            hasGithubToken: true,
          };

          const statusDiv = modal.querySelector(".Assistant-auth-status");
          statusDiv.innerHTML = `
            <div class="auth-success">
              <i class="icon-circle-check"></i>
              Successfully connected to an LLM provider!
            </div>
          `;

          playToggleOn();
          Utils.showToast({
            type: "success",
            title: "Success",
            message: "an LLM provider connected!",
          });

          // Refresh providers list
          await loadProviders();

          // Close modal after 1.5 seconds
          setTimeout(() => {
            modal.remove();
          }, 1500);

          return;
        }

        if (data.pending) {
          attempts++;
          setTimeout(poll, interval * 1000);
        } else {
          const errorMsg = typeof data.error === 'string' ? data.error : 
            (data.error?.message || JSON.stringify(data.error) || "Authentication failed");
          throw new Error(errorMsg);
        }
      } catch (error) {
        console.error("[AI] Poll error:", error);
        const errorMessage = error.message || String(error) || "Authentication failed";
        const statusDiv = modal.querySelector(".Assistant-auth-status");
        statusDiv.innerHTML = `
          <div class="auth-error">
            <i class="icon-circle-x"></i>
            ${errorMessage}
          </div>
        `;
        playError();
        Utils.showToast({
          type: "error",
          title: "Error",
          message: errorMessage,
        });
      }
    };

    poll();
  }

  /**
   * Disconnect an LLM provider
   */
  async function disconnectAssistant() {
    try {
      const response = await fetch("/api/ai/Assistant/auth/logout", {
        method: "POST",
      });

      if (!response.ok) throw new Error("Failed to disconnect");

      state.AssistantAuth = {
        authenticated: false,
        hasGithubToken: false,
      };

      playToggleOn();
      Utils.showToast({
        type: "success",
        title: "Disconnected",
        message: "an LLM provider disconnected",
      });

      // Refresh providers list
      await loadProviders();

      // Close modal if open
      document.querySelector(".modal-overlay")?.remove();
    } catch (error) {
      console.error("[AI] Failed to disconnect Assistant:", error);
      playError();
      Utils.showToast({
        type: "error",
        title: "Error",
        message: error.message || "Failed to disconnect",
      });
    }
  }

  /**
   * Test an LLM provider connection
   */
  async function testAssistantConnection(modal) {
    const testBtn = modal.querySelector("#Assistant-test-btn");
    const testResult = modal.querySelector("#Assistant-test-result");

    if (!testBtn || !testResult) {
      console.error("[AI] Assistant test elements not found in modal");
      return;
    }

    try {
      testBtn.disabled = true;
      testBtn.innerHTML = '<i class="icon-loader spin"></i> Testing...';
      testResult.className = "test-connection-result";

      console.log("[AI] Testing Assistant connection...");

      const response = await fetch("/api/ai/providers/Assistant/test", {
        method: "POST",
      });

      const data = await response.json();
      console.log("[AI] Assistant test response:", data);

      if (response.ok && data.success) {
        testResult.className = "test-connection-result visible success";
        testResult.innerHTML = `
          <span class="test-result-message">
            <i class="icon-circle-check"></i> Connection successful!
          </span>
          <dl class="test-result-details">
            <dt>Model:</dt><dd>${Utils.escapeHtml(String(data.model || "N/A"))}</dd>
            <dt>Latency:</dt><dd>${Math.round(data.latency_ms || 0)}ms</dd>
            <dt>Tokens:</dt><dd>${parseInt(data.tokens_used) || 0}</dd>
          </dl>
        `;
        playToggleOn();
      } else {
        throw new Error(data.error?.message || data.error || "Test failed");
      }
    } catch (error) {
      console.error("[AI] Assistant test failed:", error);
      testResult.className = "test-connection-result visible error";
      testResult.innerHTML = `
        <span class="test-result-message">
          <i class="icon-circle-x"></i> ${Utils.escapeHtml(String(error.message))}
        </span>
      `;
      playError();
    } finally {
      testBtn.disabled = false;
      testBtn.innerHTML = '<i class="icon-zap"></i> Test Connection';
    }
  }

  /**
   * Open an LLM provider configuration modal
   */
  function configureAssistant() {
    const isAuthenticated = state.AssistantAuth.authenticated;
    const provider = state.providers.find((p) => p.id === "Assistant") || {
      id: "Assistant",
      enabled: false,
      model: "",
    };

    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
      <div class="modal-dialog provider-config-modal Assistant-modal">
        <div class="modal-header">
          <h3>
            <span class="provider-icon"><i class="icon-github"></i></span>
            an LLM provider Configuration
          </h3>
          <button class="modal-close" onclick="this.closest('.modal-overlay').remove()">
            <i class="icon-x"></i>
          </button>
        </div>
        <div class="modal-body">
          <!-- Authentication Status -->
          <div class="Assistant-auth-status">
            ${
              isAuthenticated
                ? `
              <div class="auth-success">
                <i class="icon-circle-check"></i>
                Connected to an LLM provider
              </div>
            `
                : `
              <div class="auth-info">
                <i class="icon-info"></i>
                Not connected to GitHub
              </div>
            `
            }
          </div>
          
          ${
            !isAuthenticated
              ? `
            <!-- Login Section -->
            <div class="form-group">
              <label>Authentication</label>
              <button type="button" class="btn btn-primary" id="Assistant-login-btn" style="width: 100%;">
                <i class="icon-github"></i>
                Login with GitHub
              </button>
              <small class="form-help">Sign in with your GitHub account to use Assistant</small>
            </div>
          `
              : `
            <!-- Model Section -->
            <div class="form-group model-section">
              <label for="Assistant-modal-model">Model</label>
              <div class="model-input-wrapper">
                <input type="text" id="Assistant-modal-model" class="form-control" 
                       placeholder="e.g., gpt-4o, gpt-4..." value="${provider.model || "gpt-4o"}">
              </div>
              <small class="form-help">The model to use for Assistant requests</small>
            </div>
            
            <!-- Enable Checkbox -->
            <div class="form-group checkbox-group">
              <label class="checkbox-label">
                <input type="checkbox" id="Assistant-modal-enabled" ${provider.enabled ? "checked" : ""}>
                <span>Enable an LLM provider</span>
              </label>
              <small class="form-help">When enabled, Assistant will be available for Assistant analysis</small>
            </div>
            
            <!-- Test Connection Section -->
            <div class="test-connection-section">
              <div class="test-connection-header">
                <span class="test-connection-title">Connection Test</span>
                <button type="button" class="btn btn-sm btn-secondary" id="Assistant-test-btn">
                  <i class="icon-zap"></i>
                  Test Connection
                </button>
              </div>
              <div class="test-connection-result" id="Assistant-test-result">
                <span class="test-result-message"></span>
                <dl class="test-result-details"></dl>
              </div>
            </div>
          `
          }
        </div>
        <div class="modal-footer modal-footer-split">
          <div class="footer-left">
            ${isAuthenticated ? '<button class="btn btn-danger" id="Assistant-disconnect-btn"><i class="icon-log-out"></i> Disconnect</button>' : ""}
          </div>
          <div class="footer-right">
            ${
              isAuthenticated
                ? `
              <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
              <button class="btn btn-primary" id="Assistant-save-btn">
                <i class="icon-save"></i>
                Save Configuration
              </button>
            `
                : '<button class="btn btn-secondary" onclick="this.closest(\'.modal-overlay\').remove()">Close</button>'
            }
          </div>
        </div>
      </div>
    `;

    document.body.appendChild(modal);

    // Login handler
    const loginBtn = modal.querySelector("#Assistant-login-btn");
    if (loginBtn) {
      loginBtn.addEventListener("click", () => {
        startAssistantAuth(modal);
      });
    }

    // Test connection handler
    const testBtn = modal.querySelector("#Assistant-test-btn");
    if (testBtn) {
      testBtn.addEventListener("click", () => {
        testAssistantConnection(modal);
      });
    }

    // Disconnect handler
    const disconnectBtn = modal.querySelector("#Assistant-disconnect-btn");
    if (disconnectBtn) {
      disconnectBtn.addEventListener("click", async () => {
        const confirmed = await ConfirmationDialog.show({
          title: "Disconnect an LLM provider",
          message: "Are you sure you want to disconnect an LLM provider?",
          confirmText: "Disconnect",
          confirmClass: "danger",
        });

        if (confirmed) {
          await disconnectAssistant();
        }
      });
    }

    // Save handler
    const saveBtn = modal.querySelector("#Assistant-save-btn");
    if (saveBtn) {
      saveBtn.addEventListener("click", async () => {
        const model = modal.querySelector("#Assistant-modal-model").value.trim();
        const enabled = modal.querySelector("#Assistant-modal-enabled").checked;

        if (!model) {
          Utils.showToast({
            type: "warning",
            title: "Missing Model",
            message: "Please enter a model name",
          });
          return;
        }

        saveBtn.disabled = true;
        saveBtn.innerHTML = '<i class="icon-loader spin"></i> Saving...';

        try {
          const response = await fetch("/api/ai/providers/Assistant", {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              enabled,
              model,
            }),
          });

          if (!response.ok) throw new Error("Failed to save configuration");

          playToggleOn();
          Utils.showToast({
            type: "success",
            title: "Saved",
            message: "an LLM provider configuration saved",
          });

          modal.remove();
          await loadProviders();
        } catch (error) {
          console.error("[AI] Failed to save Assistant config:", error);
          playError();
          Utils.showToast({
            type: "error",
            title: "Error",
            message: "Failed to save configuration",
          });
          saveBtn.disabled = false;
          saveBtn.innerHTML = '<i class="icon-save"></i> Save Configuration';
        }
      });
    }
  }

  // Return public API
  return {
    loadProviders,
    renderProviders,
    setDefaultProvider,
    testProviderFromList,
    configureProvider,
    checkAssistantAuthStatus,
    disconnectAssistant,
  };
}
