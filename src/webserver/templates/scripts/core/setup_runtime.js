// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Stateless setup validation, requests, and restart polling shared by the
// full-screen controller and the Explore Mode setup dialog.
(function () {
  "use strict";

  const PUBLIC_SOLANA_RPC = "api.mainnet-beta.solana.com";

  function isPrivateHostname(hostname) {
    const host = hostname.toLowerCase().replace(/^\[|\]$/g, "");
    if (
      host === "localhost" ||
      host.endsWith(".localhost") ||
      host.endsWith(".local") ||
      host.endsWith(".internal") ||
      host === "::1"
    ) {
      return true;
    }

    const octets = host.split(".").map(Number);
    if (octets.length !== 4 || octets.some((octet) => !Number.isInteger(octet))) return false;
    return (
      octets[0] === 0 ||
      octets[0] === 10 ||
      octets[0] === 127 ||
      (octets[0] === 169 && octets[1] === 254) ||
      (octets[0] === 172 && octets[1] >= 16 && octets[1] <= 31) ||
      (octets[0] === 192 && octets[1] === 168)
    );
  }

  function parseRpcUrls(value) {
    return String(value || "")
      .split(/\r?\n/)
      .map((url) => url.trim())
      .filter(Boolean);
  }

  function validateWalletValue(value, required) {
    const key = String(value || "").trim();
    if (!key) {
      return {
        valid: false,
        state: required ? "error" : "",
        message: required ? I18n.t("setup-wallet-required") : "",
      };
    }

    if (key.startsWith("[") && key.endsWith("]")) {
      try {
        const bytes = JSON.parse(key);
        const validBytes =
          Array.isArray(bytes) &&
          bytes.length === 64 &&
          bytes.every((byte) => Number.isInteger(byte) && byte >= 0 && byte <= 255);
        if (!validBytes) throw new Error("invalid bytes");
        return {
          valid: true,
          state: "success",
          message: I18n.t("setup-wallet-json-recognized"),
        };
      } catch {
        return {
          valid: false,
          state: "error",
          message: I18n.t("setup-wallet-json-invalid"),
        };
      }
    }

    if (!/^[1-9A-HJ-NP-Za-km-z]{80,90}$/.test(key)) {
      return {
        valid: false,
        state: "error",
        message: I18n.t("setup-wallet-format-invalid"),
      };
    }

    return {
      valid: true,
      state: "success",
      message: I18n.t("setup-wallet-base58-recognized"),
    };
  }

  function validateRpcValue(value, required) {
    const urls = parseRpcUrls(value);
    const fail = (message) => ({ valid: false, state: "error", message, urls });

    if (!urls.length) {
      return {
        valid: !required,
        state: required ? "error" : "",
        message: required ? I18n.t("setup-rpc-required") : "",
        urls,
      };
    }
    if (urls.length > 10) return fail(I18n.t("setup-rpc-too-many"));

    const normalized = new Set();
    for (const value of urls) {
      let parsed;
      try {
        parsed = new URL(value);
      } catch {
        return fail(I18n.t("setup-rpc-url-invalid"));
      }

      if (parsed.protocol !== "https:" || !parsed.hostname) {
        return fail(I18n.t("setup-rpc-url-invalid"));
      }
      if (parsed.username || parsed.password) {
        return fail(I18n.t("setup-rpc-url-credentials"));
      }
      if (parsed.hash) return fail(I18n.t("setup-rpc-url-fragment"));
      if (parsed.hostname.toLowerCase() === PUBLIC_SOLANA_RPC) {
        return fail(I18n.t("setup-rpc-public-endpoint"));
      }
      if (isPrivateHostname(parsed.hostname)) {
        return fail(I18n.t("setup-rpc-private-host"));
      }

      const key = parsed.href.replace(/\/$/, "").toLowerCase();
      if (normalized.has(key)) return fail(I18n.t("setup-rpc-duplicate"));
      normalized.add(key);
    }

    return {
      valid: true,
      state: "success",
      message: I18n.t("setup-rpc-ready", { count: urls.length }),
      urls,
    };
  }

  function summarizeValidation(validation) {
    const walletValid = Boolean(validation?.wallet_address);
    const wallet = {
      state: walletValid ? "success" : "error",
      label: walletValid ? I18n.t("setup-wallet-verified") : I18n.t("setup-wallet-unverified"),
      details: walletValid
        ? I18n.t("setup-wallet-address-detail", { address: validation.wallet_address })
        : I18n.t("setup-wallet-format-hint"),
      address: walletValid ? validation.wallet_address : null,
    };

    const results = Array.isArray(validation?.rpc_test_results) ? validation.rpc_test_results : [];
    const working = results.filter((result) => result.success);
    const failed = results.filter((result) => !result.success);
    let rpc;

    if (!working.length) {
      rpc = {
        state: "error",
        label: I18n.t("setup-rpc-none-working"),
        details:
          failed[0]?.error ||
          validation?.errors?.find((message) => /rpc|endpoint|https|mainnet/i.test(message)) ||
          I18n.t("setup-rpc-health-failed"),
      };
    } else {
      const fastest = working.reduce((best, current) =>
        current.latency_ms < best.latency_ms ? current : best
      );
      const warning = Boolean(failed.length || validation?.warnings?.length);
      rpc = {
        state: warning ? "warning" : "success",
        label: warning
          ? I18n.t("setup-rpc-partial", { working: working.length, failed: failed.length })
          : I18n.t("setup-rpc-verified", { count: working.length }),
        details: I18n.t("setup-rpc-fastest", {
          url: fastest.display_url,
          latency: fastest.latency_ms,
        }),
      };
    }

    return { rpc, wallet };
  }

  /**
   * The wallet and RPC credential fields: one markup for the onboarding step and the
   * Explore Mode setup dialog, so both ask with the same labels, placeholders, hints
   * and controls. `prefix` keeps the element ids unique, since the onboarding screen
   * stays in every page's document while the dialog is open.
   */
  function credentialFieldsHtml(prefix, rpcOptional) {
    return `
      <section class="setup-card setup-card-wallet">
        <label class="setup-card-header" for="${prefix}wallet-private-key">
          <i class="setup-card-icon icon-key" aria-hidden="true"></i>
          <span class="setup-card-title" data-l10n-id="setup-dialog-wallet-label"></span>
        </label>
        <div class="setup-field">
          <div class="setup-input-wrapper" dir="ltr">
            <textarea dir="ltr" class="setup-input" id="${prefix}wallet-private-key"
              data-l10n-id="setup-dialog-wallet-input" spellcheck="false" autocomplete="off"
              autocapitalize="none" rows="3" aria-required="true"
              aria-describedby="${prefix}wallet-security-note ${prefix}wallet-validation"
              required></textarea>
            <button class="setup-input-toggle" type="button" aria-pressed="false">
              <i class="toggle-icon icon-eye" aria-hidden="true"></i>
            </button>
          </div>
          <p class="setup-security-note" id="${prefix}wallet-security-note"
            data-l10n-id="setup-wizard-wallet-security-note"></p>
          <div class="setup-validation" id="${prefix}wallet-validation" role="status"
            aria-live="polite"></div>
          <div class="setup-wallet-preview" hidden>
            <i class="icon-wallet" aria-hidden="true"></i>
            <span class="ti-address ti-address-plain wallet-address-text"><span
              class="ti-address-value" dir="ltr" translate="no"></span></span>
            <button type="button" class="wallet-copy-btn" data-l10n-id="setup-wallet-copy">
              <i class="icon-copy" aria-hidden="true"></i>
            </button>
          </div>
        </div>
      </section>
      <section class="setup-card setup-card-rpc">
        <label class="setup-card-header" for="${prefix}rpc-urls">
          <i class="setup-card-icon icon-globe" aria-hidden="true"></i>
          <span class="setup-card-title" data-l10n-id="setup-wizard-rpc-title"></span>
        </label>
        <div class="setup-field">
          <div class="setup-input-wrapper" dir="ltr">
            <textarea dir="ltr" class="setup-input" id="${prefix}rpc-urls"
              data-l10n-id="setup-wizard-rpc-input" spellcheck="false" autocomplete="off"
              autocapitalize="none" rows="4" aria-required="${!rpcOptional}"
              aria-describedby="${prefix}rpc-guidance ${prefix}rpc-validation"
              ${rpcOptional ? "" : "required"}></textarea>
          </div>
          ${
            rpcOptional
              ? `<p class="setup-security-note" id="${prefix}rpc-optional-note"
            data-l10n-id="setup-dialog-rpc-optional-note"></p>`
              : ""
          }
          <div class="setup-rpc-guidance" id="${prefix}rpc-guidance">
            <span data-l10n-id="setup-wizard-rpc-guidance"></span>
            <span class="setup-provider-links">
              <a href="https://www.helius.dev/solana-rpc-nodes" target="_blank"
                rel="noopener noreferrer"><span data-l10n-id="setup-provider-helius"></span>
                <strong data-l10n-id="setup-wizard-provider-recommended"></strong></a>
              <a href="https://www.quicknode.com" target="_blank" rel="noopener noreferrer"
                data-l10n-id="setup-provider-quicknode"></a>
              <a href="https://www.alchemy.com" target="_blank" rel="noopener noreferrer"
                data-l10n-id="setup-provider-alchemy"></a>
            </span>
          </div>
          <div class="setup-validation" id="${prefix}rpc-validation" role="status"
            aria-live="polite"></div>
        </div>
      </section>`;
  }

  /**
   * Renders the credential fields into `container` and binds them: the key mask and
   * its toggle, inline validation while typing, the derived address preview and its
   * copy action. The returned controller is what a surface reads and validates. With
   * `rpcOptional` an empty RPC field passes: the server then tests and keeps the
   * configured endpoints.
   */
  function mountCredentialFields(container, prefix = "", { rpcOptional = false } = {}) {
    container.innerHTML = credentialFieldsHtml(prefix, rpcOptional);
    I18n.localizeTree(container);

    const walletInput = container.querySelector(`#${prefix}wallet-private-key`);
    const rpcInput = container.querySelector(`#${prefix}rpc-urls`);
    const walletValidation = container.querySelector(`#${prefix}wallet-validation`);
    const rpcValidation = container.querySelector(`#${prefix}rpc-validation`);
    const toggleBtn = container.querySelector(".setup-input-toggle");
    const preview = container.querySelector(".setup-wallet-preview");
    const previewText = preview.querySelector(".wallet-address-text");
    const copyBtn = preview.querySelector(".wallet-copy-btn");
    const timers = { wallet: null, rpc: null };
    let copyReset = null;

    const setInline = (element, input, state, message) => {
      element.className = `setup-validation${state ? ` ${state}` : ""}`;
      element.textContent = message || "";
      element.hidden = !message;
      if (state === "error") input.setAttribute("aria-invalid", "true");
      else input.removeAttribute("aria-invalid");
    };

    const maskWallet = (masked) => {
      walletInput.style.webkitTextSecurity = masked ? "disc" : "none";
      walletInput.style.textSecurity = masked ? "disc" : "none";
      toggleBtn.setAttribute("aria-pressed", String(!masked));
      const action = masked ? I18n.t("setup-wallet-show-key") : I18n.t("setup-wallet-hide-key");
      toggleBtn.setAttribute("aria-label", action);
      toggleBtn.title = action;
      toggleBtn.querySelector(".toggle-icon").className =
        `toggle-icon ${masked ? "icon-eye" : "icon-eye-off"}`;
    };

    const hideWalletPreview = () => {
      previewText.querySelector(".ti-address-value").textContent = "";
      preview.hidden = true;
    };

    const showWalletPreview = (address) => {
      // `.ti-address` (token_identity.css) fits the full address to this line.
      previewText.style.setProperty("--ti-address-chars", String(address.length));
      previewText.querySelector(".ti-address-value").textContent = address;
      preview.hidden = false;
    };

    const validateWallet = (required) => {
      const result = validateWalletValue(walletInput.value, required);
      setInline(walletValidation, walletInput, result.state, result.message); // api-body-ok: local validator result, not a server body
      if (!result.valid) hideWalletPreview();
      return result.valid;
    };

    const validateRpc = (required) => {
      const result = validateRpcValue(rpcInput.value, required);
      setInline(rpcValidation, rpcInput, result.state, result.message); // api-body-ok: local validator result, not a server body
      return result.valid;
    };

    const setCopyLabel = (label, title) => {
      copyBtn.setAttribute("aria-label", label);
      copyBtn.title = title;
    };

    walletInput.addEventListener("input", () => {
      window.clearTimeout(timers.wallet);
      timers.wallet = window.setTimeout(() => validateWallet(false), 250);
    });
    rpcInput.addEventListener("input", () => {
      window.clearTimeout(timers.rpc);
      timers.rpc = window.setTimeout(() => validateRpc(false), 250);
    });
    toggleBtn.addEventListener("click", () =>
      maskWallet(walletInput.style.webkitTextSecurity === "none")
    );
    copyBtn.addEventListener("click", async () => {
      const address = previewText.textContent.trim();
      if (!address) return;
      window.clearTimeout(copyReset);
      try {
        await navigator.clipboard.writeText(address);
        setCopyLabel(
          I18n.attr("setup-wallet-copy-done", "aria-label"),
          I18n.attr("setup-wallet-copy-done", "title")
        );
        copyReset = window.setTimeout(
          () =>
            setCopyLabel(
              I18n.attr("setup-wallet-copy", "aria-label"),
              I18n.attr("setup-wallet-copy", "title")
            ),
          1500
        );
      } catch {
        setCopyLabel(
          I18n.attr("setup-wallet-copy-failed", "aria-label"),
          I18n.attr("setup-wallet-copy-failed", "title")
        );
      }
    });
    maskWallet(true);

    return {
      walletInput,
      rpcInput,
      rpcField: rpcInput.closest(".setup-field"),
      validateWallet,
      validateRpc,
      rpcUrls: () => parseRpcUrls(rpcInput.value),
      /** Both fields checked, the RPC field as required unless it is optional; focuses the first one that fails. */
      validateAll() {
        window.clearTimeout(timers.wallet);
        window.clearTimeout(timers.rpc);
        const walletValid = validateWallet(true);
        const rpcValid = validateRpc(!rpcOptional);
        if (!walletValid || !rpcValid) {
          (walletValid ? rpcInput : walletInput).focus({ preventScroll: true });
        }
        return walletValid && rpcValid;
      },
      showWalletPreview,
      hideWalletPreview,
      dispose() {
        window.clearTimeout(timers.wallet);
        window.clearTimeout(timers.rpc);
        window.clearTimeout(copyReset);
      },
    };
  }

  async function requestJson(path, options = {}) {
    const response = await fetch(path, options);
    let body = null;
    try {
      body = await response.json();
    } catch {
      body = null;
    }

    if (!response.ok) {
      throw new Error(
        window.RequestManagerErrors.apiErrorMessage(
          body,
          I18n.t("setup-error-request-failed", { status: response.status })
        )
      );
    }
    return body?.data ?? body;
  }

  function delay(milliseconds, signal) {
    return new Promise((resolve, reject) => {
      if (signal?.aborted) {
        reject(new DOMException("Aborted", "AbortError"));
        return;
      }

      const onAbort = () => {
        window.clearTimeout(timeout);
        reject(new DOMException("Aborted", "AbortError"));
      };
      const timeout = window.setTimeout(() => {
        signal?.removeEventListener("abort", onAbort);
        resolve();
      }, milliseconds);
      signal?.addEventListener("abort", onAbort, { once: true });
    });
  }

  async function waitForScreenerBotRestart(previousInstanceId, options = {}) {
    const target = options.target || "/home";
    const timeoutMs = options.timeoutMs || 120000;
    const startedAt = Date.now();

    while (Date.now() - startedAt < timeoutMs) {
      if (options.signal?.aborted) throw new DOMException("Aborted", "AbortError");

      try {
        const response = await fetch(`/api/health?restart_check=${Date.now()}`, {
          cache: "no-store",
          signal: options.signal,
        });
        if (response.ok) {
          const health = await response.json();
          if (health.instance_id && health.instance_id !== previousInstanceId) {
            options.onReady?.();
            window.location.replace(target);
            return;
          }
        }
      } catch (error) {
        if (error?.name === "AbortError") throw error;
      }

      options.onWaiting?.(Date.now() - startedAt);
      await delay(500, options.signal);
    }

    throw new Error(I18n.t("setup-error-restart-timeout"));
  }

  window.SetupRuntime = {
    mountCredentialFields,
    parseRpcUrls,
    requestJson,
    summarizeValidation,
    validateRpcValue,
    validateWalletValue,
    waitForScreenerBotRestart,
  };
  window.waitForScreenerBotRestart = waitForScreenerBotRestart;
})();
