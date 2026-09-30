import { $ } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import { playSuccess, playError } from "../../core/sounds.js";
import { apiErrorMessage } from "../../core/request_manager.js";
import { formatLatencyMs, formatNumber } from "../../core/format.js";
import { LLM_PROVIDER_LABELS } from "../../ui/llm_provider.js";

const providerName = (providerId) => I18n.label(LLM_PROVIDER_LABELS, providerId);

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
      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      const data = await response.json();
      state.providers = data.providers || [];

      // Store default_provider from API response for use in rendering
      if (data.default_provider) {
        state.defaultProvider = data.default_provider;
      }

      renderProviders(state.providers);
    } catch (error) {
      console.error("[Assistant] Failed to load providers:", error);
      Utils.showToast({
        key: "assistant-load",
        type: "error",
        title: I18n.t("assistant-providers-load-failed"),
      });
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
    const allProviderIds = Object.keys(LLM_PROVIDER_LABELS);

    container.innerHTML = allProviderIds
      .map((providerId) => {
        const provider = providers.find((p) => p.id === providerId) || {
          id: providerId,
          enabled: false,
          api_key: "",
          model: "",
        };

        const isDefault = providerId === defaultProvider;
        const name = Utils.escapeHtml(providerName(providerId));

        // Regular providers (API key-based)
        const isConfigured = provider.enabled && provider.api_key && provider.model;

        return `
          <div class="provider-item ${isDefault ? "is-default" : ""}" data-provider="${providerId}">
            <label class="provider-select" data-l10n-id="assistant-providers-select-default">
              <input type="radio" name="default-provider" value="${providerId}"
                     ${isDefault ? "checked" : ""}
                     onchange="window.assistantPage.setDefaultProvider('${providerId}')">
            </label>
            
            <div class="provider-logo">
              <img src="/assets/providers/${providerId}.png" alt="" onerror="this.style.display='none'; this.nextElementSibling.style.display='flex';">
              <div class="provider-logo-fallback" style="display: none;">${name.charAt(0)}</div>
            </div>
            
            <div class="provider-info">
              <div class="provider-name">${name}</div>
              <div class="provider-model">${provider.model ? Utils.escapeHtml(provider.model) : '<span data-l10n-id="assistant-providers-model-none"></span>'}</div>
            </div>
            
            <div class="provider-status">
              ${isConfigured ? '<span class="status-badge configured"><i class="icon-check"></i> <span data-l10n-id="assistant-providers-status-ready"></span></span>' : '<span class="status-badge not-configured" data-l10n-id="assistant-providers-status-not-set-up"></span>'}
              ${isDefault ? '<span class="status-badge default" data-l10n-id="assistant-providers-status-default"></span>' : ""}
            </div>
            
            <div class="provider-actions">
              ${isConfigured ? `<button class="provider-btn test-btn" onclick="window.assistantPage.testProviderFromList('${providerId}')"><i class="icon-zap"></i> <span data-l10n-id="assistant-providers-test"></span></button>` : ""}
              <button class="provider-btn ${isConfigured ? "" : "primary"}" onclick="window.assistantPage.configureProvider('${providerId}')">
                <i class="icon-${isConfigured ? "settings" : "plus"}"></i> ${isConfigured ? '<span data-l10n-id="common-action-edit"></span>' : '<span data-l10n-id="assistant-providers-configure"></span>'}
              </button>
            </div>
          </div>
        `;
      })
      .join("");
    I18n.localizeTree(container);
    container.querySelectorAll(".provider-item").forEach((row) => {
      const id = row.dataset.provider;
      row
        .querySelector(".provider-select input")
        ?.setAttribute("aria-label", I18n.t("assistant-providers-use-default", { name: providerName(id) }));
      row.querySelector(".provider-logo img")?.setAttribute("alt", providerName(id));
    });
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

      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      Utils.showToast({
        type: "success",
        title: I18n.t("assistant-providers-default-set-title"),
        message: I18n.t("assistant-providers-default-set-message", {
          name: providerName(providerId),
        }),
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
      Utils.showToast({
        type: "error",
        title: I18n.t("assistant-providers-default-set-failed"),
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
        title: I18n.t("assistant-providers-testing-title"),
        message: I18n.t("assistant-providers-testing-message", { name: providerName(providerId) }),
      });

      const response = await fetch(`/api/llm/providers/${providerId}/test`, {
        method: "POST",
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(
          apiErrorMessage(
            errorData,
            I18n.t("assistant-providers-test-http", { status: String(response.status) })
          )
        );
      }

      const result = await response.json();

      if (result.success) {
        Utils.showToast({
          type: "success",
          title: I18n.t("assistant-providers-test-success-title"),
          message: I18n.t("assistant-providers-test-success-message", {
            name: providerName(providerId),
          }),
        });
      } else {
        throw new Error(apiErrorMessage(result, I18n.t("assistant-providers-test-failed")));
      }
    } catch (error) {
      console.error(`[Assistant] Provider test failed for ${providerId}:`, error);
      Utils.showToast({
        type: "error",
        title: I18n.t("assistant-providers-test-failed"),
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

    const name = providerName(providerId);
    const hasApiKey = provider.has_api_key || false;

    // Create and show modal
    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
      <div class="modal-dialog provider-config-modal">
        <div class="modal-header">
          <h3>
            <span class="provider-icon"><i class="icon-bot"></i></span>
            <span class="provider-config-name" data-l10n-id="assistant-providers-config-title"></span>
          </h3>
          <button class="modal-close" data-l10n-id="assistant-modal-close" onclick="this.closest('.modal-overlay').remove()">
            <i class="icon-x"></i>
          </button>
        </div>
        <div class="modal-body">
          <!-- API Key Section -->
          <div class="form-group">
            <label for="modal-api-key">
              <span data-l10n-id="assistant-providers-api-key"></span>
              ${hasApiKey ? '<span class="key-status key-saved"><i class="icon-circle-check"></i> <span data-l10n-id="assistant-providers-key-saved"></span></span>' : '<span class="key-status key-missing"><i class="icon-circle-alert"></i> <span data-l10n-id="assistant-providers-key-missing"></span></span>'}
            </label>
            <div class="api-key-input-wrapper">
              <input type="password" id="modal-api-key" class="form-control" 
                     data-l10n-id="assistant-providers-api-key-enter" 
                     value="">
              <button type="button" class="api-key-toggle" id="toggle-api-key" data-l10n-id="assistant-providers-key-toggle">
                <i class="icon-eye"></i>
              </button>
            </div>
            ${hasApiKey ? '<small class="form-help" data-l10n-id="assistant-providers-key-help-saved"></small>' : '<small class="form-help" data-l10n-id="assistant-providers-key-help-new"></small>'}
          </div>
          
          <!-- Model Section -->
          <div class="form-group model-section">
            <label for="modal-model" data-l10n-id="assistant-providers-model"></label>
            <div class="model-input-wrapper">
              <input type="text" id="modal-model" class="form-control" 
                     data-l10n-id="assistant-providers-model-input" value="${provider.model || ""}">
            </div>
            <small class="form-help" data-l10n-id="assistant-providers-model-help"></small>
          </div>
          
          <!-- Enable Checkbox -->
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" id="modal-enabled" ${provider.enabled ? "checked" : ""}>
              <span data-l10n-id="assistant-providers-enable"></span>
            </label>
            <small class="form-help" data-l10n-id="assistant-providers-enable-help"></small>
          </div>
          
          <!-- Test Connection Section -->
          <div class="test-connection-section">
            <div class="test-connection-header">
              <span class="test-connection-title" data-l10n-id="assistant-providers-connection-test"></span>
              <button type="button" class="btn btn-sm btn-secondary" id="test-connection-btn">
                <i class="icon-zap"></i>
                <span data-l10n-id="assistant-providers-test-connection"></span>
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
            <button class="btn btn-secondary" data-l10n-id="common-action-cancel" onclick="this.closest('.modal-overlay').remove()"></button>
          </div>
          <div class="footer-right">
            <button class="btn btn-primary" id="modal-save-btn">
              <i class="icon-save"></i>
              <span data-l10n-id="assistant-providers-save"></span>
            </button>
          </div>
        </div>
      </div>
    `;
    modal.querySelector(".provider-config-name").dataset.l10nArgs = JSON.stringify({ name });
    if (hasApiKey) {
      modal.querySelector("#modal-api-key").dataset.l10nId = "assistant-providers-api-key-update";
    }
    I18n.localizeTree(modal);
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
          title: I18n.t("assistant-providers-missing-key-title"),
          message: I18n.t("assistant-providers-missing-key-test"),
        });
        return;
      }

      // First save the config temporarily
      testBtn.disabled = true;
      testBtn.innerHTML = `<i class="icon-loader spin"></i> ${Utils.escapeHtml(I18n.t("assistant-providers-testing"))}`;
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

          if (!saveRes.ok) {
            const saveError = await saveRes.json().catch(() => ({}));
            throw new Error(apiErrorMessage(saveError, I18n.t("assistant-providers-test-save-failed")));
          }
        }

        // Now test the provider
        const response = await fetch(`/api/llm/providers/${providerId}/test`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
        });

        const data = await response.json();

        if (response.ok && data.success) {
          testResult.className = "test-connection-result visible success";
          testMessage.innerHTML = `<i class="icon-circle-check"></i> ${Utils.escapeHtml(I18n.t("assistant-providers-test-connected"))}`;
          testDetails.innerHTML = `
            <dt>${Utils.escapeHtml(I18n.t("assistant-providers-detail-model"))}</dt><dd>${data.model ? Utils.escapeHtml(String(data.model)) : Utils.escapeHtml(I18n.t("assistant-providers-detail-none"))}</dd>
            <dt>${Utils.escapeHtml(I18n.t("assistant-providers-detail-latency"))}</dt><dd>${Utils.escapeHtml(formatLatencyMs(Math.round(data.latency_ms || 0)))}</dd>
            <dt>${Utils.escapeHtml(I18n.t("assistant-providers-detail-tokens"))}</dt><dd>${Utils.escapeHtml(formatNumber(parseInt(data.tokens_used) || 0, 0))}</dd>
          `;
          playSuccess();
        } else {
          throw new Error(apiErrorMessage(data, I18n.t("assistant-providers-test-failed")));
        }
      } catch (error) {
        testResult.className = "test-connection-result visible error";
        testMessage.innerHTML = `<i class="icon-circle-x"></i> ${Utils.escapeHtml(String(error.message))}`;
        testDetails.innerHTML = "";
        playError();
      } finally {
        testBtn.disabled = false;
        testBtn.innerHTML = `<i class="icon-zap"></i> ${Utils.escapeHtml(I18n.t("assistant-providers-test-connection"))}`;
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
          title: I18n.t("assistant-providers-missing-key-title"),
          message: I18n.t("assistant-providers-missing-key-enable"),
        });
        return;
      }

      if (enabled && !model) {
        Utils.showToast({
          type: "warning",
          title: I18n.t("assistant-providers-missing-model-title"),
          message: I18n.t("assistant-providers-missing-model-message"),
        });
        return;
      }

      try {
        saveBtn.disabled = true;
        saveBtn.innerHTML = `<i class="icon-loader spin"></i> ${Utils.escapeHtml(I18n.t("assistant-providers-saving"))}`;

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

        if (!response.ok) throw new Error(`HTTP ${response.status}`);

        Utils.showToast({
          type: "success",
          title: I18n.t("assistant-providers-saved-title"),
          message: I18n.t("assistant-providers-saved-message", { name }),
        });

        modal.remove();
        await loadProviders();
      } catch (error) {
        console.error(`[Assistant] Failed to save provider ${providerId}:`, error);
        Utils.showToast({ type: "error", title: I18n.t("assistant-providers-save-failed") });
        saveBtn.disabled = false;
        saveBtn.innerHTML = `<i class="icon-save"></i> ${Utils.escapeHtml(I18n.t("assistant-providers-save"))}`;
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
