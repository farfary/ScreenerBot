// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Setup validation, the credential fields, the wallet source choice and the new
// wallet's backup panel, requests, and restart polling shared by the full-screen
// controller and the setup dialogs.
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

  /** Message key of each service state a service validation answers with. */
  const SERVICE_STATE_LABELS = Object.freeze({
    checking: "setup-verify-service-checking",
    ready: "setup-verify-service-ready",
    sign_in_required: "setup-verify-service-signin",
    reauthorization_required: "setup-connection-reauthorize",
    busy: "setup-verify-service-busy",
    daily_limit: "setup-verify-service-daily-limit",
    unreachable: "setup-verify-service-unreachable",
    version_unsupported: "setup-verify-service-version",
  });

  /**
   * The connection card of a service validation: ready, or the state that stops it
   * with its sentence. `serviceState` is the state key the error actions follow.
   */
  function summarizeServiceValidation(validation) {
    const serviceState = validation?.connection?.state || "unreachable";
    const ready = serviceState === "ready";
    return {
      state: ready ? "success" : "error",
      label: I18n.label(SERVICE_STATE_LABELS, serviceState),
      details: "",
      serviceState,
    };
  }

  /** The wallet key field: the onboarding import step and the setup dialogs share it. */
  function walletFieldHtml(prefix) {
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
      </section>`;
  }

  /** The RPC endpoints field: the onboarding connection step and the setup dialogs share it. */
  function rpcFieldHtml(prefix, rpcOptional) {
    return `
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

  function setInline(element, input, state, message) {
    element.className = `setup-validation${state ? ` ${state}` : ""}`;
    element.textContent = message || "";
    element.hidden = !message;
    if (state === "error") input.setAttribute("aria-invalid", "true");
    else input.removeAttribute("aria-invalid");
  }

  /**
   * Renders the wallet key field into `container` and binds it: the key mask and its
   * toggle, inline validation while typing, the derived address preview and its copy
   * action. `prefix` keeps the element ids unique, since the onboarding screen stays
   * in every page's document while a setup dialog is open.
   */
  function mountWalletField(container, prefix = "") {
    container.innerHTML = walletFieldHtml(prefix);
    I18n.localizeTree(container);

    const walletInput = container.querySelector(`#${prefix}wallet-private-key`);
    const walletValidation = container.querySelector(`#${prefix}wallet-validation`);
    const toggleBtn = container.querySelector(".setup-input-toggle");
    const preview = container.querySelector(".setup-wallet-preview");
    const previewText = preview.querySelector(".wallet-address-text");
    const copyBtn = preview.querySelector(".wallet-copy-btn");
    let timer = null;
    let copyReset = null;

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

    const setCopyLabel = (label, title) => {
      copyBtn.setAttribute("aria-label", label);
      copyBtn.title = title;
    };

    walletInput.addEventListener("input", () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => validateWallet(false), 250);
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
      validateWallet,
      /** The field checked as required; focuses it when it fails. */
      validate() {
        window.clearTimeout(timer);
        const valid = validateWallet(true);
        if (!valid) walletInput.focus({ preventScroll: true });
        return valid;
      },
      value: () => walletInput.value.trim(),
      showWalletPreview,
      hideWalletPreview,
      /** Empties the field, its message and its preview. */
      clear() {
        window.clearTimeout(timer);
        walletInput.value = "";
        setInline(walletValidation, walletInput, "", "");
        hideWalletPreview();
      },
      dispose() {
        window.clearTimeout(timer);
        window.clearTimeout(copyReset);
      },
    };
  }

  /**
   * Renders the RPC endpoints field into `container` and binds its inline validation.
   * With `rpcOptional` an empty field passes: the server then tests and keeps the
   * configured endpoints.
   */
  function mountRpcField(container, prefix = "", { rpcOptional = false } = {}) {
    container.innerHTML = rpcFieldHtml(prefix, rpcOptional);
    I18n.localizeTree(container);

    const rpcInput = container.querySelector(`#${prefix}rpc-urls`);
    const rpcValidation = container.querySelector(`#${prefix}rpc-validation`);
    let timer = null;

    const validateRpc = (required) => {
      const result = validateRpcValue(rpcInput.value, required);
      setInline(rpcValidation, rpcInput, result.state, result.message); // api-body-ok: local validator result, not a server body
      return result.valid;
    };

    rpcInput.addEventListener("input", () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => validateRpc(false), 250);
    });

    return {
      rpcInput,
      rpcField: rpcInput.closest(".setup-field"),
      validateRpc,
      rpcUrls: () => parseRpcUrls(rpcInput.value),
      /** The field checked, as required unless it is optional; focuses it when it fails. */
      validate() {
        window.clearTimeout(timer);
        const valid = validateRpc(!rpcOptional);
        if (!valid) rpcInput.focus({ preventScroll: true });
        return valid;
      },
      dispose() {
        window.clearTimeout(timer);
      },
    };
  }

  /**
   * Renders the wallet source choice into `container`: create a new wallet or import
   * one, as native radios with nothing preselected. `onChange` receives the chosen
   * source. The onboarding step and the setup dialogs share it.
   */
  function mountWalletSource(container, prefix = "", { onChange } = {}) {
    container.innerHTML = `
      <div class="setup-choice setup-choice-wallet" role="radiogroup"
        data-l10n-id="setup-wallet-source">
        <label class="setup-choice-card" for="${prefix}wallet-source-create">
          <input type="radio" class="setup-choice-input" id="${prefix}wallet-source-create"
            name="${prefix}wallet-source" value="create" />
          <span class="setup-choice-copy">
            <span class="setup-choice-title">
              <i class="icon-sparkles" aria-hidden="true"></i>
              <span data-l10n-id="setup-wallet-create-title"></span>
            </span>
            <span class="setup-choice-detail" data-l10n-id="setup-wallet-create-detail"></span>
          </span>
        </label>
        <label class="setup-choice-card" for="${prefix}wallet-source-import">
          <input type="radio" class="setup-choice-input" id="${prefix}wallet-source-import"
            name="${prefix}wallet-source" value="import" />
          <span class="setup-choice-copy">
            <span class="setup-choice-title">
              <i class="icon-import" aria-hidden="true"></i>
              <span data-l10n-id="setup-wallet-import-title"></span>
            </span>
            <span class="setup-choice-detail" data-l10n-id="setup-wallet-import-detail"></span>
          </span>
        </label>
      </div>`;
    I18n.localizeTree(container);

    const inputs = [...container.querySelectorAll(".setup-choice-input")];
    const onInput = (event) => onChange?.(event.target.value);
    inputs.forEach((input) => input.addEventListener("change", onInput));

    return {
      value: () => inputs.find((input) => input.checked)?.value ?? null,
      focus: () => (inputs.find((input) => input.checked) ?? inputs[0])?.focus(),
      dispose() {
        inputs.forEach((input) => input.removeEventListener("change", onInput));
      },
    };
  }

  const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

  /** The bytes a base58 string encodes, or null when it holds a character outside the alphabet. */
  function base58ToBytes(value) {
    const bytes = [];
    for (const character of value) {
      let carry = BASE58_ALPHABET.indexOf(character);
      if (carry < 0) return null;
      for (let index = 0; index < bytes.length; index += 1) {
        carry += bytes[index] * 58;
        bytes[index] = carry & 0xff;
        carry >>= 8;
      }
      while (carry > 0) {
        bytes.push(carry & 0xff);
        carry >>= 8;
      }
    }
    for (const character of value) {
      if (character !== "1") break;
      bytes.push(0);
    }
    return bytes.reverse();
  }

  /** The Solana CLI keypair file name for `address`, whole so two wallets never share a name. */
  function keypairFileName(address) {
    return `screenerbot-wallet-${address}.json`;
  }

  const KEY_MASK = "•".repeat(44);

  /**
   * Renders the backup panel of a new wallet into `container`: it creates the wallet
   * when `generate()` runs, shows the address and the masked private key with show,
   * copy and download, the machine warning, and three acknowledgements. The secret
   * lives only in this closure: it is never written to storage, a URL, the history, a
   * DOM attribute or a log, and `clear()` drops it. `onChange` runs whenever
   * `ready()` may have changed.
   */
  function mountBackupPanel(container, prefix = "", { onChange } = {}) {
    container.innerHTML = `
      <section class="setup-backup" aria-labelledby="${prefix}backup-title">
        <header class="setup-backup-header">
          <h2 class="setup-backup-title" id="${prefix}backup-title">
            <i class="icon-key-round" aria-hidden="true"></i>
            <span data-l10n-id="setup-backup-title"></span>
          </h2>
          <p class="setup-backup-description" data-l10n-id="setup-backup-description"></p>
        </header>
        <div class="setup-backup-progress" role="status" aria-live="polite">
          <span class="setup-backup-progress-text"></span>
          <button type="button" class="setup-btn setup-btn-secondary" data-action="retry"
            data-l10n-id="setup-action-try-again" hidden></button>
        </div>
        <div class="setup-backup-body" hidden>
          <dl class="setup-backup-values">
            <div class="setup-backup-row">
              <dt data-l10n-id="setup-backup-address"></dt>
              <dd class="setup-backup-value" dir="ltr" translate="no" data-value="address"></dd>
              <dd class="setup-backup-actions">
                <button type="button" class="setup-backup-action" data-action="copy-address">
                  <i class="icon-copy" aria-hidden="true"></i>
                  <span data-l10n-id="setup-backup-copy-address"></span>
                </button>
              </dd>
            </div>
            <div class="setup-backup-row">
              <dt data-l10n-id="setup-backup-key"></dt>
              <dd class="setup-backup-value setup-backup-key" dir="ltr" translate="no"
                data-value="key"></dd>
              <dd class="setup-backup-actions">
                <button type="button" class="setup-backup-action" data-action="toggle-key"
                  aria-pressed="false">
                  <i class="icon-eye" aria-hidden="true"></i>
                  <span></span>
                </button>
                <button type="button" class="setup-backup-action" data-action="copy-key">
                  <i class="icon-copy" aria-hidden="true"></i>
                  <span data-l10n-id="common-action-copy"></span>
                </button>
                <button type="button" class="setup-backup-action" data-action="download-key">
                  <i class="icon-download" aria-hidden="true"></i>
                  <span data-l10n-id="setup-backup-download"></span>
                </button>
              </dd>
            </div>
          </dl>
          <p class="setup-backup-feedback" role="status" aria-live="polite"></p>
          <div data-slot="machine-warning"></div>
          <div class="setup-backup-acks">
            <label class="setup-backup-ack">
              <input type="checkbox" data-ack="saved" />
              <span data-l10n-id="setup-backup-ack-saved"></span>
            </label>
            <label class="setup-backup-ack">
              <input type="checkbox" data-ack="control" />
              <span data-l10n-id="setup-backup-ack-control"></span>
            </label>
            <label class="setup-backup-ack">
              <input type="checkbox" data-ack="recovery" />
              <span data-l10n-id="setup-backup-ack-recovery"></span>
            </label>
          </div>
          <div class="setup-backup-another">
            <button type="button" class="setup-btn setup-btn-secondary"
              data-action="create-another">
              <i class="icon-refresh-cw" aria-hidden="true"></i>
              <span data-l10n-id="setup-backup-create-another"></span>
            </button>
            <div class="setup-backup-confirm" hidden>
              <p data-l10n-id="setup-backup-replace-confirm"></p>
              <div class="setup-backup-confirm-actions">
                <button type="button" class="setup-btn setup-btn-secondary"
                  data-action="keep-key" data-l10n-id="common-action-cancel"></button>
                <button type="button" class="setup-btn setup-btn-primary"
                  data-action="confirm-another" data-l10n-id="setup-backup-create-another"></button>
              </div>
            </div>
          </div>
        </div>
      </section>`;
    I18n.localizeTree(container);

    const find = (selector) => container.querySelector(selector);
    const progress = find(".setup-backup-progress");
    const progressText = find(".setup-backup-progress-text");
    const retryBtn = find('[data-action="retry"]');
    const body = find(".setup-backup-body");
    const addressEl = find('[data-value="address"]');
    const keyEl = find('[data-value="key"]');
    const toggleBtn = find('[data-action="toggle-key"]');
    const feedback = find(".setup-backup-feedback");
    const acks = [...container.querySelectorAll("[data-ack]")];
    const anotherBtn = find('[data-action="create-another"]');
    const confirmEl = find(".setup-backup-confirm");

    // The warning is the shared notice, whose owner is a module this classic script loads.
    import("/scripts/ui/notice.js")
      .then(({ renderNotice }) => {
        find('[data-slot="machine-warning"]').innerHTML = renderNotice({
          variant: "warning",
          message: I18n.t("setup-backup-machine-warning"),
        });
      })
      .catch((error) => console.error("[Setup] The backup warning failed to load:", error));

    let secret = null;
    let address = null;
    let revealed = false;
    let run = 0;
    let feedbackReset = null;

    const notify = () => onChange?.();

    const showFeedback = (message, tone = "success") => {
      window.clearTimeout(feedbackReset);
      feedback.dataset.tone = tone;
      feedback.textContent = message;
      feedbackReset = window.setTimeout(() => {
        feedback.textContent = "";
      }, 6000);
    };

    const paintKey = () => {
      keyEl.textContent = secret && revealed ? secret : secret ? KEY_MASK : "";
      keyEl.classList.toggle("is-revealed", Boolean(secret && revealed));
      const action = revealed ? I18n.t("setup-wallet-hide-key") : I18n.t("setup-wallet-show-key");
      toggleBtn.setAttribute("aria-pressed", String(revealed));
      toggleBtn.querySelector("span").textContent = action;
      toggleBtn.querySelector("i").className = revealed ? "icon-eye-off" : "icon-eye";
    };

    const reset = () => {
      run += 1;
      secret = null;
      address = null;
      revealed = false;
      addressEl.textContent = "";
      paintKey();
      acks.forEach((ack) => {
        ack.checked = false;
      });
      window.clearTimeout(feedbackReset);
      feedback.textContent = "";
      confirmEl.hidden = true;
      anotherBtn.hidden = false;
      body.hidden = true;
      progress.hidden = true;
      retryBtn.hidden = true;
      progressText.textContent = "";
    };

    const generate = async () => {
      reset();
      const current = run;
      progress.hidden = false;
      progressText.textContent = I18n.t("setup-wallet-creating");
      notify();
      try {
        const created = await requestJson("/api/initialization/wallet/generate", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: "{}",
          cache: "no-store",
        });
        if (current !== run) return;
        if (!created?.address || !created?.private_key) throw new Error("incomplete");
        secret = created.private_key;
        address = created.address;
        addressEl.textContent = address;
        paintKey();
        progress.hidden = true;
        body.hidden = false;
      } catch {
        if (current !== run) return;
        progressText.textContent = I18n.t("setup-wallet-create-failed");
        retryBtn.hidden = false;
      }
      notify();
    };

    const copy = async (value, message) => {
      try {
        await navigator.clipboard.writeText(value);
        showFeedback(message);
      } catch {
        showFeedback(I18n.attr("setup-wallet-copy-failed", "title"), "error");
      }
    };

    const download = async () => {
      const bytes = secret ? base58ToBytes(secret) : null;
      if (!bytes || bytes.length !== 64) return;
      const { downloadBlob } = await import("/scripts/core/download.js");
      downloadBlob(
        keypairFileName(address),
        new Blob([JSON.stringify(bytes)], { type: "application/json" })
      );
    };

    const onClick = (event) => {
      const action = event.target.closest("[data-action]")?.dataset.action;
      if (!action) return;
      if (action === "retry") void generate();
      else if (action === "copy-address" && address) {
        void copy(address, I18n.attr("setup-wallet-copy-done", "title"));
      } else if (action === "toggle-key" && secret) {
        revealed = !revealed;
        paintKey();
      } else if (action === "copy-key" && secret) {
        void copy(secret, I18n.t("setup-backup-copied"));
      } else if (action === "download-key") {
        void download();
      } else if (action === "create-another") {
        // A key the reader has acknowledged saving is discarded only after a confirmation.
        if (acks.some((ack) => ack.checked)) {
          confirmEl.hidden = false;
          anotherBtn.hidden = true;
          find('[data-action="keep-key"]').focus();
        } else {
          void generate();
        }
      } else if (action === "keep-key") {
        confirmEl.hidden = true;
        anotherBtn.hidden = false;
        anotherBtn.focus();
      } else if (action === "confirm-another") {
        void generate();
      }
    };
    const onAck = () => notify();

    container.addEventListener("click", onClick);
    acks.forEach((ack) => ack.addEventListener("change", onAck));
    reset();

    return {
      /** Creates the wallet unless one is already created or being created. */
      ensureGenerated() {
        if (!secret && progress.hidden) void generate();
      },
      /** True once a wallet exists and all three acknowledgements are ticked. */
      ready: () => Boolean(secret) && acks.every((ack) => ack.checked),
      privateKey: () => secret,
      address: () => address,
      /** Drops the secret and everything shown about it. */
      clear() {
        reset();
        notify();
      },
      dispose() {
        reset();
        container.removeEventListener("click", onClick);
        acks.forEach((ack) => ack.removeEventListener("change", onAck));
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
    // A null target leaves navigation to the caller once the new instance answers.
    const target = options.target === undefined ? "/home" : options.target;
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
            if (target) window.location.replace(target);
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
    mountBackupPanel,
    mountRpcField,
    mountWalletField,
    mountWalletSource,
    parseRpcUrls,
    requestJson,
    summarizeServiceValidation,
    summarizeValidation,
    validateRpcValue,
    validateWalletValue,
    waitForScreenerBotRestart,
  };
  window.waitForScreenerBotRestart = waitForScreenerBotRestart;
})();
