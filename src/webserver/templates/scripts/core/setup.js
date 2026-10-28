// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Full-screen setup controller.
//
// Four steps: the trading wallet (create one and save its key, or import one), the
// connection (the ScreenerBot service or your own RPC), verification, and ready. The
// verification step sends one immutable snapshot to the backend, receives a
// short-lived receipt, and saves that exact snapshot without repeating the network
// tests. After the restart, a wallet without SOL is offered the funding step.

(function () {
  "use strict";

  const {
    mountBackupPanel,
    mountRpcField,
    mountWalletField,
    mountWalletSource,
    requestJson,
    summarizeServiceValidation,
    summarizeValidation,
    waitForScreenerBotRestart,
  } = window.SetupRuntime;

  // The query this script was served with, so a module imported at run time is the
  // same build as the page.
  const ASSET_QUERY = document.currentScript ? new URL(document.currentScript.src).search : "";

  const STEP = Object.freeze({ wallet: 1, connection: 2, verify: 3, ready: 4 });

  // Service states the error bar answers with a second action: use your own RPC.
  const OWN_RPC_REMEDY_STATES = new Set(["busy", "daily_limit"]);
  // Service states that clear by trying again, as opposed to by changing the account.
  const RETRY_STATES = new Set(["busy", "unreachable", "checking"]);
  // Service states the account panel on the connection step resolves.
  const ACCOUNT_STATES = new Set(["sign_in_required", "reauthorization_required"]);

  class SetupControllerClass {
    constructor() {
      this.currentStep = STEP.wallet;
      this.initialized = false;
      this.verificationBusy = false;
      this.verificationRun = 0;
      this.verificationAbort = null;
      this.restartAbort = null;
      this.snapshot = null;
      this.previousInstanceId = null;
      this.walletSource = null;
      this.walletDetail = false;
      this.source = null;
      this.backup = null;
      this.walletField = null;
      this.rpcField = null;
      this.accountPanel = null;
      this.accountStatus = null;
      this.gatewayBusy = false;
      this.failedServiceState = null;
      this.fundingAddress = null;
      this.stopFundingWatch = null;
    }

    init() {
      if (this.initialized) return;

      const byId = (id) => document.getElementById(id);
      this.screen = byId("setupScreen");
      this.stepContents = document.querySelectorAll(".setup-step-content[data-step]");
      this.stepIndicators = document.querySelectorAll(".setup-step[data-step]");
      this.footer = document.querySelector(".setup-footer");
      this.backBtn = byId("setup-back");
      this.nextBtn = byId("setup-next");
      this.exploreBtn = byId("setup-explore");
      this.retryBtn = byId("setup-retry");
      this.useOwnRpcBtn = byId("setup-use-own-rpc");
      this.reconnectBtn = byId("setup-reconnect");
      this.reloadBtn = byId("setup-reload");
      this.errorEl = byId("setup-error");
      this.errorMessages = byId("setup-error-messages");
      this.sourceEl = byId("setup-wallet-source");
      this.backupEl = byId("setup-wallet-backup");
      this.importEl = byId("setup-wallet-import");
      this.connectionInputs = [...document.querySelectorAll('input[name="setup-connection"]')];
      this.connectionRequirement = byId("setup-connection-requirement");
      this.ownFieldsEl = byId("setup-connection-own-fields");
      this.accountRequirement = byId("setup-account-requirement");
      this.accountPanelEl = byId("setupAccountPanel");
      this.gatewayOptionEl = byId("setup-gateway-option");
      this.gatewayCheckboxEl = byId("setup-use-gateway");
      this.verificationSummary = byId("verification-summary");
      this.completeState = byId("setup-complete-state");
      this.completeIcon = this.completeState?.querySelector(".setup-complete-icon");
      this.completeTitle = byId("complete-title");
      this.completeText = byId("setup-complete-text");
      this.restartIndicator = byId("setup-restart-indicator");
      this.servicesStatus = byId("services-status");
      this.completeActions = byId("setup-complete-actions");
      this.fundEl = byId("setup-fund");

      if (!this.screen || !this.stepContents.length || !this.sourceEl || !this.backupEl) {
        console.warn("[Setup] Required setup elements were not found");
        return;
      }

      this.source = mountWalletSource(this.sourceEl, "setup-", {
        onChange: (value) => this.onWalletSourceChanged(value),
      });
      this.backup = mountBackupPanel(this.backupEl, "setup-", {
        onChange: () => this.updateButtons(),
      });
      this.walletField = mountWalletField(this.importEl, "");
      this.rpcField = mountRpcField(document.getElementById("setup-rpc-field"), "");
      this.bindEvents();
      this.loadVersion();
      this.mountAccountPanel();
      this.attachGatewayHandler();
      this.resetVerificationStates();
      this.showWalletView(false);
      this.setStep(STEP.wallet);
      this.initialized = true;
    }

    bindEvents() {
      this.backBtn?.addEventListener("click", () => this.goBack());
      this.nextBtn?.addEventListener("click", () => this.goNext());
      this.exploreBtn?.addEventListener("click", () => this.enterExploreMode());
      this.retryBtn?.addEventListener("click", () => this.onRetry());
      this.useOwnRpcBtn?.addEventListener("click", () => this.switchToOwnRpc());
      this.reconnectBtn?.addEventListener("click", () => this.startRestartWait());
      this.reloadBtn?.addEventListener("click", () => window.location.reload());
      this.connectionInputs.forEach((input) =>
        input.addEventListener("change", () => this.onConnectionChanged())
      );
      document
        .getElementById("setup-fund-open")
        ?.addEventListener("click", () => this.openDashboard());
    }

    async loadVersion() {
      try {
        const result = await requestJson("/api/version");
        const versionEl = document.getElementById("setup-version");
        if (versionEl && result?.version)
          versionEl.textContent = I18n.t("shell-version", { version: result.version });
      } catch {
        // Version decoration is optional and must never block setup.
      }
    }

    setStep(step, options = {}) {
      this.currentStep = step;
      this.screen.dataset.step = String(step);

      this.stepIndicators.forEach((indicator) => {
        const indicatorStep = Number(indicator.dataset.step);
        indicator.classList.toggle("active", indicatorStep === step);
        indicator.classList.toggle("completed", indicatorStep < step);
        delete indicator.dataset.tone;
        if (indicatorStep === step) indicator.setAttribute("aria-current", "step");
        else indicator.removeAttribute("aria-current");
      });

      this.stepContents.forEach((content) => {
        const active = Number(content.dataset.step) === step;
        content.classList.toggle("active", active);
        content.hidden = !active;
        content.setAttribute("aria-hidden", String(!active));
        if (active) content.removeAttribute("inert");
        else content.setAttribute("inert", "");
      });

      this.updateButtons();

      if (options.focusHeading) {
        const heading = this.screen.querySelector(`.setup-step-content[data-step="${step}"] h1`);
        if (heading) {
          heading.tabIndex = -1;
          window.requestAnimationFrame(() => heading.focus({ preventScroll: true }));
        }
      }
    }

    /** The chosen connection, or null while none is chosen. */
    connection() {
      return this.connectionInputs.find((input) => input.checked)?.value ?? null;
    }

    /** Why the ScreenerBot service cannot be chosen yet, or null when it can. */
    serviceBlocker() {
      const status = this.accountStatus;
      if (!status?.signed_in) return "sign_in";
      if (!status.scopes?.includes("chain:read")) return "reauthorize";
      return null;
    }

    /** Whether the step's own requirement lets Continue proceed. */
    canContinue() {
      if (this.currentStep === STEP.wallet) {
        if (!this.walletSource) return false;
        if (!this.walletDetail) return true;
        return this.walletSource === "import" || this.backup.ready();
      }
      if (this.currentStep === STEP.connection) {
        const connection = this.connection();
        if (connection === "service") return !this.serviceBlocker();
        return connection === "own_rpc";
      }
      return false;
    }

    updateButtons() {
      if (this.footer) this.footer.hidden = this.currentStep === STEP.ready;

      if (this.backBtn) {
        this.backBtn.hidden = this.currentStep > STEP.verify;
        this.backBtn.disabled = this.verificationBusy;
      }
      if (this.exploreBtn) {
        this.exploreBtn.hidden = this.currentStep !== STEP.wallet;
        this.exploreBtn.disabled = this.verificationBusy;
      }
      if (this.nextBtn) {
        this.nextBtn.hidden = this.currentStep > STEP.connection;
        this.nextBtn.disabled = this.verificationBusy || !this.canContinue();
      }
    }

    onWalletSourceChanged(value) {
      this.walletSource = value;
      // A new wallet is created as soon as it is chosen; choosing to import drops it.
      if (value === "create") this.backup.ensureGenerated();
      else this.backup.clear();
      this.hideError();
      this.updateButtons();
    }

    /** Step 1 shows either the source choice or the chosen source's detail. */
    showWalletView(detail) {
      this.walletDetail = detail;
      this.sourceEl.hidden = detail;
      this.backupEl.hidden = !(detail && this.walletSource === "create");
      this.importEl.hidden = !(detail && this.walletSource === "import");
      if (detail && this.walletSource === "import") {
        window.requestAnimationFrame(() => this.walletField.walletInput.focus());
      } else if (!detail) {
        window.requestAnimationFrame(() => this.source.focus());
      }
      this.updateButtons();
    }

    onConnectionChanged() {
      const connection = this.connection();
      this.ownFieldsEl.hidden = connection !== "own_rpc";
      this.renderServiceRequirement();
      this.hideError();
      this.updateButtons();
    }

    renderServiceRequirement() {
      const service = this.connection() === "service";
      const blocker = service ? this.serviceBlocker() : null;
      if (this.connectionRequirement) {
        this.connectionRequirement.hidden = !blocker;
        let requirement = "";
        if (blocker === "sign_in") requirement = I18n.t("setup-connection-signin-required");
        else if (blocker === "reauthorize") requirement = I18n.t("setup-connection-reauthorize");
        this.connectionRequirement.textContent = requirement;
      }
      if (this.accountRequirement) {
        this.accountRequirement.textContent = service
          ? I18n.t("setup-wizard-account-required")
          : I18n.t("setup-wizard-account-optional");
      }
      // The data-access summary calls signing in optional, which holds only with an own RPC.
      this.accountPanel?.setDataAccessVisible(!service);
    }

    goNext() {
      if (this.verificationBusy || !this.canContinue()) return;
      this.hideError();

      if (this.currentStep === STEP.wallet) {
        if (!this.walletDetail) {
          this.showWalletView(true);
          return;
        }
        if (this.walletSource === "import" && !this.walletField.validate()) return;
        this.setStep(STEP.connection, { focusHeading: true });
        return;
      }

      if (this.currentStep === STEP.connection) {
        if (this.connection() === "own_rpc" && !this.rpcField.validate()) return;
        this.validateAndProceed();
      }
    }

    goBack() {
      if (this.verificationBusy) return;
      this.hideError();

      if (this.currentStep === STEP.wallet) {
        if (this.walletDetail) {
          // The created key stays: going back only reopens the choice.
          this.showWalletView(false);
          return;
        }
        this.leaveWithoutWallet();
        window.OnboardingController?.showFromSetup();
        return;
      }

      if (this.currentStep === STEP.connection) {
        this.setStep(STEP.wallet, { focusHeading: true });
        return;
      }

      if (this.currentStep !== STEP.verify) return;
      this.cancelVerification();
      this.resetVerificationStates();
      this.setStep(STEP.connection, { focusHeading: true });
    }

    /** Leaving setup without finishing it drops a created key and a typed one. */
    leaveWithoutWallet() {
      this.backup?.clear();
      this.walletField?.clear();
    }

    onRetry() {
      if (this.verificationBusy) return;
      if (this.currentStep === STEP.verify && RETRY_STATES.has(this.failedServiceState)) {
        this.validateAndProceed();
        return;
      }
      // A service state no retry or sign-in resolves (the daily limit, an old version)
      // is not a credential problem: the bar closes and the card keeps the reason.
      if (
        this.currentStep === STEP.verify &&
        this.failedServiceState &&
        !ACCOUNT_STATES.has(this.failedServiceState)
      ) {
        this.hideError();
        return;
      }
      this.reviewCredentials();
    }

    /** Returns from a failed verification to the step its cause belongs to. */
    reviewCredentials() {
      // A refused key goes back to the wallet; anything else is the connection's.
      const walletFailed =
        document.getElementById("wallet-verification-card")?.dataset.state === "error";
      const step =
        this.currentStep === STEP.verify && !walletFailed ? STEP.connection : STEP.wallet;
      this.hideError();
      this.resetVerificationStates();
      this.setStep(step, { focusHeading: true });
    }

    switchToOwnRpc() {
      if (this.verificationBusy) return;
      const own = this.connectionInputs.find((input) => input.value === "own_rpc");
      if (own) own.checked = true;
      this.onConnectionChanged();
      this.resetVerificationStates();
      this.setStep(STEP.connection, { focusHeading: true });
      window.requestAnimationFrame(() => this.rpcField.rpcInput.focus());
    }

    cancelVerification() {
      this.verificationRun += 1;
      this.verificationAbort?.abort();
      this.verificationAbort = null;
      this.verificationBusy = false;
      this.snapshot = null;
      this.updateButtons();
    }

    walletPrivateKey() {
      return this.walletSource === "create" ? this.backup.privateKey() : this.walletField.value();
    }

    async validateAndProceed() {
      this.hideError();
      const access = this.connection();
      const walletPrivateKey = this.walletPrivateKey();
      if (!access || !walletPrivateKey) return;

      this.snapshot = Object.freeze({
        walletPrivateKey,
        rpcUrls: Object.freeze(access === "own_rpc" ? [...this.rpcField.rpcUrls()] : []),
        access,
      });
      this.failedServiceState = null;
      this.verificationBusy = true;
      this.verificationRun += 1;
      const run = this.verificationRun;
      this.verificationAbort?.abort();
      this.verificationAbort = new AbortController();
      this.resetVerificationStates();
      this.setStep(STEP.verify, { focusHeading: true });
      this.updateButtons();

      await this.runVerification(run, this.verificationAbort.signal, this.snapshot);
    }

    isCurrentRun(run, signal) {
      return run === this.verificationRun && !signal.aborted;
    }

    async runVerification(run, signal, snapshot) {
      const service = snapshot.access === "service";
      this.setVerificationState(
        "wallet",
        "running",
        I18n.t("setup-verify-wallet-parsing"),
        I18n.t("setup-verify-wallet-parsing-detail")
      );
      if (service) {
        this.setVerificationState(
          "connection",
          "running",
          I18n.t("setup-verify-service-checking"),
          ""
        );
      } else {
        this.setVerificationState(
          "connection",
          "running",
          I18n.t("setup-verify-rpc-testing"),
          I18n.t("setup-verify-rpc-testing-detail", { count: snapshot.rpcUrls.length })
        );
      }
      this.setVerificationState("save", "pending", I18n.t("setup-verify-save-waiting"), "");
      if (this.verificationSummary) {
        this.verificationSummary.textContent = I18n.t("setup-verify-summary-running");
      }

      try {
        const validation = await requestJson("/api/initialization/validate", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            wallet_private_key: snapshot.walletPrivateKey,
            rpc_urls: snapshot.rpcUrls,
            access: snapshot.access,
          }),
          signal,
        });
        if (!this.isCurrentRun(run, signal)) return;

        const serviceFailure = this.renderValidationResult(validation, service);
        if (!validation?.valid || !validation?.validation_id) {
          this.failedServiceState = serviceFailure;
          // The cards above already name each failure in the user's language; the
          // server's own sentences are diagnostics and stay out of the error bar.
          throw new Error(
            serviceFailure
              ? I18n.t("setup-error-verify-failed")
              : I18n.t("setup-error-credentials-failed")
          );
        }

        this.setVerificationState(
          "save",
          "running",
          I18n.t("setup-verify-save-running"),
          I18n.t("setup-verify-save-running-detail")
        );
        if (this.verificationSummary) {
          this.verificationSummary.textContent = I18n.t("setup-verify-summary-saving");
        }

        const completed = await requestJson("/api/initialization/complete", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            validation_id: validation.validation_id,
            wallet_private_key: snapshot.walletPrivateKey,
            rpc_urls: snapshot.rpcUrls,
            access: snapshot.access,
          }),
          signal,
        });
        if (!this.isCurrentRun(run, signal)) return;
        if (!completed?.success) {
          throw new Error(completed?.errors?.join(" ") || I18n.t("setup-error-save-failed"));
        }

        this.setVerificationState(
          "save",
          "success",
          I18n.t("setup-verify-save-done"),
          service
            ? I18n.t("setup-verify-save-done-service-detail")
            : I18n.t("setup-verify-save-done-detail")
        );
        // The key is stored encrypted now; nothing on this page keeps it.
        this.leaveWithoutWallet();
        this.fundingAddress = validation.wallet_address || null;
        this.snapshot = null;
        this.verificationBusy = false;
        this.previousInstanceId = completed.instance_id;
        this.setStep(STEP.ready, { focusHeading: true });
        this.startRestartWait();
      } catch (error) {
        if (error?.name === "AbortError" || !this.isCurrentRun(run, signal)) return;

        if (document.getElementById("save-verification-card")?.dataset.state === "running") {
          this.setVerificationState(
            "save",
            "error",
            I18n.t("setup-verify-save-failed"),
            error.message
          );
        } else if (!document.querySelector('.setup-verification-item[data-state="error"]')) {
          const requestFailed = I18n.t("setup-verify-request-failed");
          this.setVerificationState("wallet", "error", requestFailed, "");
          this.setVerificationState("connection", "error", requestFailed, "");
          this.setVerificationState("save", "pending", I18n.t("setup-verify-save-skipped"), "");
        }
        if (document.getElementById("save-verification-card")?.dataset.state === "pending") {
          this.setVerificationState("save", "pending", I18n.t("setup-verify-save-skipped"), "");
        }

        if (this.verificationSummary) {
          this.verificationSummary.textContent = I18n.t("setup-verify-summary-failed");
        }
        this.showError(error.message || I18n.t("setup-error-verify-failed"), true);
        this.verificationBusy = false;
        this.updateButtons();
      }
    }

    /**
     * Paints the wallet and connection cards from a validation; returns the service
     * state that stopped a service validation, or null.
     */
    renderValidationResult(validation, service) {
      const summary = summarizeValidation(validation);
      this.setVerificationState(
        "wallet",
        summary.wallet.state,
        summary.wallet.label,
        summary.wallet.details
      );
      if (summary.wallet.address && this.walletSource === "import") {
        this.walletField.showWalletPreview(summary.wallet.address);
      }
      if (!service) {
        this.setVerificationState(
          "connection",
          summary.rpc.state,
          summary.rpc.label,
          summary.rpc.details
        );
        return null;
      }
      const connection = summarizeServiceValidation(validation);
      this.setVerificationState(
        "connection",
        connection.state,
        connection.label,
        connection.details
      );
      return connection.state === "success" ? null : connection.serviceState;
    }

    setVerificationState(name, state, label, details) {
      const card = document.getElementById(`${name}-verification-card`);
      const status = document.getElementById(`${name}-status`);
      const detail = document.getElementById(`${name}-details`);
      if (!card || !status || !detail) return;

      card.dataset.state = state;
      const labelElement = status.querySelector("span:last-child");
      if (labelElement) labelElement.textContent = label;
      detail.textContent = details || "";
      detail.hidden = !details;
    }

    resetVerificationStates() {
      this.setVerificationState("wallet", "pending", I18n.t("setup-verify-wallet-waiting"), "");
      this.setVerificationState("connection", "pending", I18n.t("setup-verify-rpc-waiting"), "");
      this.setVerificationState("save", "pending", I18n.t("setup-verify-save-waiting"), "");
      if (this.verificationSummary) {
        this.verificationSummary.textContent = I18n.t("setup-verify-summary-checking");
      }
    }

    async enterExploreMode() {
      if (this.verificationBusy) return;
      this.hideError();
      this.exploreBtn.disabled = true;
      // Only the label changes, so the button keeps its glyph through a failure.
      const label = this.exploreBtn.querySelector("span");
      const originalLabel = label.textContent;
      label.textContent = I18n.t("setup-explore-opening");

      try {
        const result = await requestJson("/api/initialization/explore", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: "{}",
        });
        if (!result?.success) {
          throw new Error(result?.errors?.join(" ") || I18n.t("setup-error-explore-failed"));
        }
        this.leaveWithoutWallet();
        window.location.assign("/tokens");
      } catch (error) {
        this.exploreBtn.disabled = false;
        label.textContent = originalLabel;
        this.showError(error.message || I18n.t("setup-error-explore-failed"), true);
      }
    }

    /** Marks the current step's indicator with `tone` (a stalled restart), or clears it. */
    setCurrentStepTone(tone) {
      this.stepIndicators.forEach((indicator) => {
        if (tone && Number(indicator.dataset.step) === this.currentStep) {
          indicator.dataset.tone = tone;
        } else {
          delete indicator.dataset.tone;
        }
      });
    }

    startRestartWait() {
      if (!this.previousInstanceId) return;
      this.restartAbort?.abort();
      this.restartAbort = new AbortController();
      this.completeState?.setAttribute("data-state", "waiting");
      this.setCurrentStepTone(null);
      this.completeState.hidden = false;
      if (this.fundEl) this.fundEl.hidden = true;
      if (this.completeIcon) {
        this.completeIcon.className = "setup-complete-icon icon-circle-check";
      }
      if (this.completeText) {
        this.completeText.textContent = I18n.t("setup-complete-restarting");
      }
      if (this.servicesStatus) this.servicesStatus.textContent = I18n.t("setup-complete-finishing");
      if (this.restartIndicator) this.restartIndicator.hidden = false;
      if (this.completeActions) this.completeActions.hidden = true;

      waitForScreenerBotRestart(this.previousInstanceId, {
        target: null,
        signal: this.restartAbort.signal,
        onReady: () => {
          if (this.servicesStatus) {
            this.servicesStatus.textContent = I18n.t("setup-complete-ready");
          }
        },
      })
        .then(() => this.afterRestart())
        .catch((error) => {
          if (error?.name === "AbortError") return;
          this.completeState?.setAttribute("data-state", "error");
          this.setCurrentStepTone("warning");
          if (this.completeIcon) {
            this.completeIcon.className = "setup-complete-icon icon-triangle-alert";
          }
          if (this.completeText) {
            this.completeText.textContent = I18n.t("setup-complete-stored");
          }
          if (this.servicesStatus) this.servicesStatus.textContent = error.message;
          if (this.restartIndicator) this.restartIndicator.hidden = true;
          if (this.completeActions) this.completeActions.hidden = false;
        });
    }

    /**
     * The new instance answers: a funded wallet opens the dashboard; a wallet with no
     * SOL, or one whose balance cannot be read yet, is offered the funding step first.
     */
    async afterRestart() {
      const funding = await import(`/scripts/ui/funding_notice.js${ASSET_QUERY}`);
      const balance = await funding.fetchMainWalletBalance();
      if (balance !== null && balance > 0) {
        this.openDashboard();
        return;
      }
      this.showFunding(funding, balance);
    }

    async showFunding(funding, balance) {
      this.completeState?.classList.add("is-funding");
      if (this.completeTitle) this.completeTitle.textContent = I18n.t("setup-ready-title");
      if (this.completeText) this.completeText.hidden = true;
      if (this.restartIndicator) this.restartIndicator.hidden = true;
      if (this.servicesStatus) this.servicesStatus.hidden = true;
      if (!this.fundEl) {
        this.openDashboard();
        return;
      }
      this.fundEl.hidden = false;

      const address = this.fundingAddress || (await funding.fetchMainWalletAddress());
      if (address) {
        const { renderAddress } = await import(`/scripts/ui/token_identity.js${ASSET_QUERY}`);
        document.getElementById("setup-fund-address").innerHTML = renderAddress(address, {
          explorer: "account",
        });
        this.loadFundingQr(funding, address);
      }
      this.renderFundingBalance(balance);
      this.stopFundingWatch = funding.watchMainWalletBalance((next) =>
        this.renderFundingBalance(next)
      );
    }

    async loadFundingQr(funding, address) {
      const image = document.getElementById("setup-fund-qr-image");
      const status = document.getElementById("setup-fund-qr-status");
      try {
        image.src = await funding.fetchWalletQr(address);
        image.hidden = false;
        status.hidden = true;
      } catch {
        status.textContent = I18n.t("home-wallet-qr-unavailable");
      }
    }

    renderFundingBalance(balance) {
      const text = document.getElementById("setup-fund-status-text");
      const icon = this.fundEl?.querySelector(".setup-fund-status-icon");
      if (!text || !icon) return;
      const status = text.closest(".setup-fund-status");
      // "This wallet holds no SOL" holds only while the balance is known to be zero.
      const message = document.getElementById("setup-fund-message");
      if (message) message.hidden = balance !== 0;
      if (balance !== null && balance > 0) {
        this.stopFundingWatch?.();
        this.stopFundingWatch = null;
        status.dataset.state = "received";
        icon.className = "setup-fund-status-icon icon-circle-check";
        text.textContent = I18n.t("setup-fund-received", {
          amount: window.Utils.formatSol(balance, { decimals: 4, suffix: "" }),
        });
        return;
      }
      status.dataset.state = balance === null ? "unavailable" : "waiting";
      icon.className = `setup-fund-status-icon ${balance === null ? "icon-info" : "icon-loader"}`;
      text.textContent =
        balance === null ? I18n.t("setup-fund-balance-unavailable") : I18n.t("setup-fund-waiting");
    }

    openDashboard() {
      this.stopFundingWatch?.();
      this.stopFundingWatch = null;
      window.location.replace("/home");
    }

    mountAccountPanel() {
      if (!this.accountPanelEl || !window.AccountPanel) return;
      this.accountPanel = window.AccountPanel.mount(this.accountPanelEl, {
        onChange: (status) => this.onAccountChanged(status),
      });
    }

    onAccountChanged(status) {
      this.accountStatus = status || null;
      this.renderServiceRequirement();
      this.updateButtons();
      if (!this.gatewayOptionEl || !this.gatewayCheckboxEl) return;
      const canUseGateway = Boolean(status?.signed_in) && status?.scopes?.includes("rpc:submit");
      this.gatewayOptionEl.hidden = !canUseGateway;
      this.gatewayCheckboxEl.checked = Boolean(status?.use_gateway_rpc);
    }

    attachGatewayHandler() {
      if (!this.gatewayCheckboxEl) return;

      this.gatewayCheckboxEl.addEventListener("change", async () => {
        if (this.gatewayBusy) return;
        const requested = this.gatewayCheckboxEl.checked;
        this.gatewayBusy = true;
        this.gatewayCheckboxEl.disabled = true;

        try {
          const status = await requestJson("/api/account/gateway", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ enabled: requested }),
          });
          this.onAccountChanged(status);
        } catch (error) {
          this.gatewayCheckboxEl.checked = !requested;
          this.showError(error.message || I18n.t("setup-error-gateway-failed"), false);
        } finally {
          this.gatewayBusy = false;
          this.gatewayCheckboxEl.disabled = false;
        }
      });
    }

    showError(message, focus) {
      if (!this.errorEl || !this.errorMessages) return;
      this.errorMessages.textContent = message;
      const serviceState = this.currentStep === STEP.verify ? this.failedServiceState : null;
      if (this.retryBtn) {
        let label = I18n.t("common-action-dismiss");
        if (RETRY_STATES.has(serviceState)) label = I18n.t("setup-action-try-again");
        else if (ACCOUNT_STATES.has(serviceState)) label = I18n.t("setup-action-sign-in-again");
        else if (this.currentStep === STEP.verify && !serviceState) {
          label = I18n.t("setup-action-review-credentials");
        }
        this.retryBtn.textContent = label;
      }
      if (this.useOwnRpcBtn) this.useOwnRpcBtn.hidden = !OWN_RPC_REMEDY_STATES.has(serviceState);
      this.errorEl.hidden = false;
      if (focus) window.requestAnimationFrame(() => this.errorEl.focus({ preventScroll: true }));
    }

    hideError() {
      if (this.errorEl) this.errorEl.hidden = true;
      if (this.errorMessages) this.errorMessages.textContent = "";
      if (this.useOwnRpcBtn) this.useOwnRpcBtn.hidden = true;
    }

    dispose() {
      this.leaveWithoutWallet();
      this.source?.dispose();
      this.backup?.dispose();
      this.walletField?.dispose();
      this.rpcField?.dispose();
      this.cancelVerification();
      this.restartAbort?.abort();
      this.restartAbort = null;
      this.stopFundingWatch?.();
      this.stopFundingWatch = null;
      this.accountPanel?.destroy?.();
      this.accountPanel = null;
      this.initialized = false;
    }
  }

  window.SetupController = new SetupControllerClass();
})();
