import { on, off } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";

/**
 * TradeActionDialog - Modern modal for buy/add/sell actions
 *
 * Features:
 * - Quick-action preset buttons with visual feedback
 * - Custom amount input with live validation
 * - Promise-based return value
 * - Keyboard navigation (Enter, Escape)
 * - Focus management and ARIA compliance
 * - Loading states and transaction feedback
 */

const ACTION_CONFIG = {
  buy: {
    icon: "shopping-cart",
    title: "Buy Token",
    subtitle: "Enter amount in SOL",
    confirmLabel: "Execute Buy",
    inputLabel: "Custom Amount",
    inputPlaceholder: "Enter SOL amount",
    inputHint: "Leave empty for config default",
    colorClass: "action-buy",
    presets: [
      { label: "0.005", sublabel: "SOL", value: 0.005, type: "amount" },
      { label: "0.01", sublabel: "SOL", value: 0.01, type: "amount" },
      { label: "0.02", sublabel: "SOL", value: 0.02, type: "amount" },
      { label: "0.05", sublabel: "SOL", value: 0.05, type: "amount" },
    ],
  },
  sell: {
    icon: "trending-down",
    title: "Sell Position",
    subtitle: "Select sell percentage",
    confirmLabel: "Execute Sell",
    inputLabel: "Custom Percentage",
    inputPlaceholder: "1-100",
    inputHint: "Enter value between 1-100",
    colorClass: "action-sell",
    presets: [
      { label: "25%", sublabel: "Partial", value: 25, type: "percentage" },
      { label: "50%", sublabel: "Half", value: 50, type: "percentage" },
      { label: "75%", sublabel: "Most", value: 75, type: "percentage" },
      { label: "100%", sublabel: "Full Exit", value: 100, type: "percentage", default: true },
    ],
  },
  add: {
    icon: "plus-circle",
    title: "Add to Position",
    subtitle: "DCA into existing position",
    confirmLabel: "Add Position",
    inputLabel: "Custom Amount",
    inputPlaceholder: "Enter SOL amount",
    inputHint: "Leave empty for 50% of original entry",
    colorClass: "action-add",
    presets: [], // Dynamic, built from context
  },
};

export class TradeActionDialog {
  constructor() {
    this.root = null;
    this.dialog = null;
    this.titleEl = null;
    this.subtitleEl = null;
    this.contextEl = null;
    this.presetContainers = [];
    this.inputField = null;
    this.errorEl = null;
    this.confirmBtn = null;
    this.cancelBtn = null;
    this.closeBtn = null;
    this._presetButtons = []; // Track preset buttons for cleanup

    this._isOpen = false;
    this._isLoading = false;
    this._previousActiveElement = null;
    this._resolveOpen = null;
    this._settingInputProgrammatically = false; // Prevent input handler during programmatic changes

    this.currentAction = null;
    this.currentContext = null;
    this._selectedPreset = null;

    // Quick trade mode state
    this._quickMode = false;
    this._quickStep = "mint"; // "mint" or "trade"
    this._quickMintStepEl = null;
    this._quickMintInputEl = null;
    this._quickPasteBtnEl = null;
    this._quickContinueBtnEl = null;
    this._quickCancelBtnEl = null;
    this._quickTokenPreviewEl = null;
    this._quickErrorEl = null;
    this._fetchedTokenData = null;
    this._currentSymbol = null;

    this._overlayListener = this._handleOverlayClick.bind(this);
    this._closeListener = this._handleCloseClick.bind(this);
    this._cancelListener = this._handleCancelClick.bind(this);
    this._confirmListener = this._handleConfirmClick.bind(this);
    this._keyListener = this._handleKeyDown.bind(this);
    this._presetClickListener = this._handlePresetClick.bind(this);
    this._inputChangeListener = this._handleInputChange.bind(this);
    this._focusTrap = null;

    // Quick trade mode bound listeners
    this._quickPasteListener = this._handleQuickPaste.bind(this);
    this._quickContinueListener = this._handleQuickContinue.bind(this);
    this._quickMintInputListener = this._handleQuickMintInput.bind(this);
    this._quoteRefreshListener = this._handleQuoteRefresh.bind(this);

    // Quote preview state
    this._quoteData = null;
    this._quoteLoading = false;
    this._quoteError = null;
    this._quoteTimestamp = null;
    this._quoteRefreshTimer = null;
    this._fetchQuoteDebounced = this._debounce(this._fetchQuote.bind(this), 400);

    this._ensureElements();
  }

  _ensureElements() {
    if (this.root) {
      return;
    }

    const overlay = document.createElement("div");
    overlay.className = "trade-action-dialog-overlay";
    overlay.setAttribute("role", "presentation");
    overlay.setAttribute("aria-hidden", "true");

    overlay.innerHTML = `
      <div class="trade-action-dialog" role="dialog" aria-modal="true" aria-labelledby="trade-action-title" tabindex="-1">
        <header class="trade-action-header">
          <div class="trade-action-header-content">
            <div class="trade-action-icon-wrapper">
              <svg class="trade-action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 2L2 7l10 5 10-5-10-5z"/>
              </svg>
            </div>
            <div class="trade-action-title-group">
              <h2 id="trade-action-title" class="trade-action-title"></h2>
              <p class="trade-action-subtitle"></p>
            </div>
          </div>
          <button type="button" class="trade-action-close" data-action="close" aria-label="Close dialog">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M18 6L6 18M6 6l12 12"/>
            </svg>
          </button>
        </header>
        <div class="trade-action-body">
          <div class="trade-action-context"></div>
          <div class="trade-action-presets"></div>
          <div class="trade-action-input-section">
            <label class="trade-action-input-label" for="trade-action-input"></label>
            <div class="trade-action-input-wrapper">
              <input type="number" id="trade-action-input" class="trade-action-input" step="any" min="0" />
              <span class="trade-action-input-suffix">SOL</span>
            </div>
            <div class="trade-action-input-hint"></div>
            <div class="trade-action-error-msg" data-visible="false">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 8v4M12 16h.01"/>
              </svg>
              <span class="trade-action-error-text"></span>
            </div>
          </div>
          <div class="trade-action-quote-section" data-state="idle">
            <div class="trade-action-quote-header">
              <span class="trade-action-quote-title">Quote Preview</span>
              <button type="button" class="trade-action-quote-refresh" aria-label="Refresh quote">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/>
                </svg>
                <span class="quote-age"></span>
              </button>
            </div>
            <div class="trade-action-quote-idle">
              <span>Select an amount to see quote</span>
            </div>
            <div class="trade-action-quote-loading">
              <div class="trade-action-quote-spinner"></div>
              <span>Fetching quote...</span>
            </div>
            <div class="trade-action-quote-error">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 8v4M12 16h.01"/>
              </svg>
              <span class="quote-error-text"></span>
            </div>
            <div class="trade-action-quote-content">
              <div class="quote-row quote-row-main">
                <span class="quote-label">You Receive</span>
                <span class="quote-value quote-output"></span>
              </div>
              <div class="quote-row">
                <span class="quote-label">Price Impact</span>
                <span class="quote-value quote-impact"></span>
              </div>
              <div class="quote-row">
                <span class="quote-label">Platform Fee</span>
                <span class="quote-value quote-platform-fee"></span>
              </div>
              <div class="quote-row">
                <span class="quote-label">Network Fee</span>
                <span class="quote-value quote-network-fee"></span>
              </div>
              <div class="quote-row">
                <span class="quote-label">Route</span>
                <span class="quote-value quote-route"></span>
              </div>
              <div class="quote-row">
                <span class="quote-label">Slippage</span>
                <span class="quote-value quote-slippage"></span>
              </div>
            </div>
          </div>
        </div>
        <footer class="trade-action-footer">
          <button type="button" class="trade-action-btn trade-action-btn-cancel" data-action="cancel">
            Cancel
          </button>
          <button type="button" class="trade-action-btn trade-action-btn-confirm" data-action="confirm" disabled>
            <span class="btn-text">Confirm</span>
            <span class="btn-loader"></span>
          </button>
        </footer>
        
        <!-- Quick Trade Mint Input Step -->
        <div class="quick-trade-mint-step" data-visible="false">
          <div class="quick-trade-mint-content">
            <label class="quick-trade-mint-label">Enter Token Mint Address</label>
            <div class="quick-trade-mint-input-wrapper">
              <input type="text" class="quick-trade-mint-input" placeholder="Paste Solana mint address..." autocomplete="off" spellcheck="false" />
              <button type="button" class="quick-trade-paste-btn" aria-label="Paste from clipboard">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                  <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/>
                </svg>
              </button>
            </div>
            <div class="quick-trade-error" data-visible="false">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 8v4M12 16h.01"/>
              </svg>
              <span class="quick-trade-error-text"></span>
            </div>
            <div class="quick-trade-token-preview" data-visible="false">
              <div class="quick-trade-token-loading">
                <div class="quick-trade-spinner"></div>
                <span>Fetching token info...</span>
              </div>
              <div class="quick-trade-token-info">
                <div class="quick-trade-token-symbol"></div>
                <div class="quick-trade-token-name"></div>
                <div class="quick-trade-token-price"></div>
              </div>
            </div>
          </div>
          <div class="quick-trade-mint-footer">
            <button type="button" class="trade-action-btn trade-action-btn-cancel quick-trade-cancel-btn">
              Cancel
            </button>
            <button type="button" class="trade-action-btn trade-action-btn-confirm quick-trade-continue-btn" disabled>
              <span class="btn-text">Continue</span>
              <span class="btn-loader"></span>
            </button>
          </div>
        </div>
      </div>
    `;

    // Inject quick trade styles
    this._injectQuickTradeStyles();

    document.body.appendChild(overlay);

    this.root = overlay;
    this.dialog = overlay.querySelector(".trade-action-dialog");
    this.titleEl = overlay.querySelector(".trade-action-title");
    this.subtitleEl = overlay.querySelector(".trade-action-subtitle");
    this.iconWrapper = overlay.querySelector(".trade-action-icon-wrapper");
    this.contextEl = overlay.querySelector(".trade-action-context");
    this.presetsContainer = overlay.querySelector(".trade-action-presets");
    this.inputField = overlay.querySelector(".trade-action-input");
    this.inputSuffix = overlay.querySelector(".trade-action-input-suffix");
    this.inputLabelEl = overlay.querySelector(".trade-action-input-label");
    this.inputHintEl = overlay.querySelector(".trade-action-input-hint");
    this.errorEl = overlay.querySelector(".trade-action-error-msg");
    this.errorTextEl = overlay.querySelector(".trade-action-error-text");
    this.confirmBtn = overlay.querySelector('[data-action="confirm"]');
    this.cancelBtn = overlay.querySelector('[data-action="cancel"]');
    this.closeBtn = overlay.querySelector('[data-action="close"]');

    // Quote preview elements
    this.quoteSection = overlay.querySelector(".trade-action-quote-section");
    this.quoteRefreshBtn = overlay.querySelector(".trade-action-quote-refresh");
    this.quoteAgeEl = overlay.querySelector(".quote-age");
    this.quoteOutputEl = overlay.querySelector(".quote-output");
    this.quoteImpactEl = overlay.querySelector(".quote-impact");
    this.quotePlatformFeeEl = overlay.querySelector(".quote-platform-fee");
    this.quoteNetworkFeeEl = overlay.querySelector(".quote-network-fee");
    this.quoteRouteEl = overlay.querySelector(".quote-route");
    this.quoteSlippageEl = overlay.querySelector(".quote-slippage");
    this.quoteErrorTextEl = overlay.querySelector(".quote-error-text");

    // Quick trade step elements
    this._quickMintStepEl = overlay.querySelector(".quick-trade-mint-step");
    this._quickMintInputEl = overlay.querySelector(".quick-trade-mint-input");
    this._quickPasteBtnEl = overlay.querySelector(".quick-trade-paste-btn");
    this._quickContinueBtnEl = overlay.querySelector(".quick-trade-continue-btn");
    this._quickCancelBtnEl = overlay.querySelector(".quick-trade-cancel-btn");
    this._quickTokenPreviewEl = overlay.querySelector(".quick-trade-token-preview");
    this._quickErrorEl = overlay.querySelector(".quick-trade-error");
    this._quickErrorTextEl = overlay.querySelector(".quick-trade-error-text");
    this._quickTokenSymbolEl = overlay.querySelector(".quick-trade-token-symbol");
    this._quickTokenNameEl = overlay.querySelector(".quick-trade-token-name");
    this._quickTokenPriceEl = overlay.querySelector(".quick-trade-token-price");
    this._quickTokenLoadingEl = overlay.querySelector(".quick-trade-token-loading");
    this._quickTokenInfoEl = overlay.querySelector(".quick-trade-token-info");
    this._tradeBodyEl = overlay.querySelector(".trade-action-body");
    this._tradeFooterEl = overlay.querySelector(".trade-action-footer");

    on(overlay, "click", this._overlayListener);
    on(this.closeBtn, "click", this._closeListener);
    on(this.cancelBtn, "click", this._cancelListener);
    on(this.confirmBtn, "click", this._confirmListener);
    on(this.inputField, "input", this._inputChangeListener);
    on(this.quoteRefreshBtn, "click", this._quoteRefreshListener);

    // Quick trade step listeners
    on(this._quickPasteBtnEl, "click", this._quickPasteListener);
    on(this._quickContinueBtnEl, "click", this._quickContinueListener);
    on(this._quickCancelBtnEl, "click", this._cancelListener);
    on(this._quickMintInputEl, "input", this._quickMintInputListener);
  }

  /**
   * Open the dialog and return a promise that resolves with the selected value or null
   * @param {Object} options - Dialog options
   * @param {string} options.action - 'buy' | 'add' | 'sell'
   * @param {string} options.mode - 'normal' (default) or 'quick' (shows mint input first)
   * @param {string} options.symbol - Token symbol for display
   * @param {Object} options.context - Contextual data (balance, entrySize, entrySizes, holdings)
   * @returns {Promise<Object|null>} - Resolves with { amount: number } or { percentage: number } or null if cancelled
   */
  async open({ action, mode = "normal", symbol, context = {} }) {
    if (!action || !ACTION_CONFIG[action]) {
      throw new Error(`Invalid action: ${action}`);
    }

    // Quick mode only supports buy and sell (not add)
    if (mode === "quick" && action === "add") {
      throw new Error("Quick mode is not supported for 'add' action");
    }

    // Guard against multiple simultaneous opens
    if (this._isOpen) {
      console.warn("[TradeActionDialog] Dialog already open, ignoring duplicate request");
      return null;
    }

    this._ensureElements();

    this._previousActiveElement =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;

    this.currentAction = action;
    this.currentContext = context;
    this._selectedPreset = null;
    this._isLoading = false;
    this._currentSymbol = symbol || null;

    // Quick mode state
    this._quickMode = mode === "quick";
    this._quickStep = this._quickMode ? "mint" : "trade";
    this._fetchedTokenData = null;

    // Reset quote state
    this._quoteData = null;
    this._quoteError = null;
    this._quoteTimestamp = null;
    this._stopQuoteRefreshTimer();

    // Create promise before rendering
    const resultPromise = new Promise((resolve) => {
      this._resolveOpen = resolve;
    });

    // Render based on mode
    if (this._quickMode) {
      this._renderQuickMintStep(action);
    } else {
      this._render(action, symbol, context);
    }

    // Show dialog with animation
    this.root.classList.add("is-visible");
    this.root.setAttribute("aria-hidden", "false");
    document.body.classList.add("trade-action-dialog-open");
    this._isOpen = true;

    document.addEventListener("keydown", this._keyListener, true);

    // Activate focus trap
    this._focusTrap = createFocusTrap(this.dialog);
    this._focusTrap.activate();

    requestAnimationFrame(() => {
      if (!this._isOpen) {
        return;
      }

      if (this._quickMode) {
        // Focus mint input for quick mode
        this._quickMintInputEl?.focus();
      } else {
        this.dialog?.focus();

        // Auto-select default preset if exists
        const defaultPreset = this.presetsContainer.querySelector(
          ".trade-action-preset-btn[data-default='true']"
        );
        if (defaultPreset) {
          defaultPreset.click();
        }
      }
    });

    return resultPromise;
  }

  close({ restoreFocus = true } = {}) {
    if (!this._isOpen) {
      return;
    }

    // Clear quote state
    this._stopQuoteRefreshTimer();
    this._setQuoteState("idle");

    // Reset quick mode state
    this._quickMode = false;
    this._quickStep = "mint";
    this._fetchedTokenData = null;
    if (this._quickMintStepEl) {
      this._quickMintStepEl.setAttribute("data-visible", "false");
      this._quickMintStepEl.classList.remove("slide-out");
    }
    if (this._tradeBodyEl) {
      this._tradeBodyEl.style.display = "";
      this._tradeBodyEl.classList.remove("slide-in");
    }
    if (this._tradeFooterEl) {
      this._tradeFooterEl.style.display = "";
      this._tradeFooterEl.classList.remove("slide-in");
    }

    this.root.classList.remove("is-visible");
    this.root.setAttribute("aria-hidden", "true");
    document.body.classList.remove("trade-action-dialog-open");
    this._isOpen = false;

    document.removeEventListener("keydown", this._keyListener, true);

    // Deactivate focus trap
    if (this._focusTrap) {
      this._focusTrap.deactivate();
      this._focusTrap = null;
    }

    if (
      restoreFocus &&
      this._previousActiveElement &&
      typeof this._previousActiveElement.focus === "function"
    ) {
      try {
        this._previousActiveElement.focus();
      } catch {
        // Ignore focus errors
      }
    }
    this._previousActiveElement = null;
  }

  destroy() {
    this.close({ restoreFocus: false });

    // Resolve pending promise to prevent hanging
    if (this._resolveOpen) {
      this._resolveOpen(null);
      this._resolveOpen = null;
    }

    if (!this.root) {
      return;
    }

    // Clean up preset button listeners
    this._presetButtons.forEach((btn) => {
      off(btn, "click", this._presetClickListener);
    });
    this._presetButtons = [];

    off(this.root, "click", this._overlayListener);
    off(this.closeBtn, "click", this._closeListener);
    off(this.cancelBtn, "click", this._cancelListener);
    off(this.confirmBtn, "click", this._confirmListener);
    off(this.inputField, "input", this._inputChangeListener);
    off(this.quoteRefreshBtn, "click", this._quoteRefreshListener);

    // Clean up quick trade mode listeners
    off(this._quickPasteBtnEl, "click", this._quickPasteListener);
    off(this._quickContinueBtnEl, "click", this._quickContinueListener);
    off(this._quickCancelBtnEl, "click", this._cancelListener);
    off(this._quickMintInputEl, "input", this._quickMintInputListener);

    if (this.root.parentNode) {
      this.root.parentNode.removeChild(this.root);
    }

    this.root = null;
    this.dialog = null;
    this.titleEl = null;
    this.contextEl = null;
    this.presetsContainer = null;
    this.inputField = null;
    this.errorEl = null;
    this.confirmBtn = null;
    this.cancelBtn = null;
    this.closeBtn = null;
  }

  _render(action, symbol, context) {
    const config = ACTION_CONFIG[action];

    // Set action-specific class on dialog
    this.dialog.className = `trade-action-dialog ${config.colorClass}`;

    // Update icon based on action
    const iconSvg = this._getActionIcon(config.icon);
    this.iconWrapper.innerHTML = iconSvg;

    // Set title and subtitle
    this.titleEl.textContent = config.title;
    this.subtitleEl.textContent = config.subtitle;

    // Render context info
    this._renderContext(action, symbol, context);

    // Build and render presets
    const presets = this._buildPresets(action, context);
    this._renderPresets(presets, action);

    // Set input labels and state
    this.inputLabelEl.textContent = config.inputLabel;
    this.inputHintEl.textContent = config.inputHint;
    this.inputField.value = "";
    this.inputField.placeholder = config.inputPlaceholder;

    // Update input suffix based on action
    this.inputSuffix.textContent = action === "sell" ? "%" : "SOL";
    this.inputSuffix.style.display = "block";

    // Set confirm button label and reset loading state
    const btnText = this.confirmBtn.querySelector(".btn-text");
    if (btnText) btnText.textContent = config.confirmLabel;
    this.confirmBtn.disabled = true;
    this.confirmBtn.classList.remove("loading");

    // Clear error
    this._clearError();
  }

  _getActionIcon(iconName) {
    const icons = {
      "shopping-cart": `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="9" cy="21" r="1"/><circle cx="20" cy="21" r="1"/>
        <path d="M1 1h4l2.68 13.39a2 2 0 0 0 2 1.61h9.72a2 2 0 0 0 2-1.61L23 6H6"/>
      </svg>`,
      "trending-down": `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="23 18 13.5 8.5 8.5 13.5 1 6"/><polyline points="17 18 23 18 23 12"/>
      </svg>`,
      "plus-circle": `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="16"/><line x1="8" y1="12" x2="16" y2="12"/>
      </svg>`,
    };
    return icons[iconName] || icons["shopping-cart"];
  }

  _renderContext(action, symbol, context) {
    const rows = [];

    // Store mint for quote fetching
    if (context.mint) {
      this.currentContext.mint = context.mint;
    }

    // Token info with symbol highlight
    rows.push(`
      <div class="trade-action-context-item">
        <span class="trade-action-context-label">Token</span>
        <span class="trade-action-context-value trade-action-symbol">${Utils.escapeHtml(symbol || "Unknown")}</span>
      </div>
    `);

    if (action === "buy" || action === "add") {
      const balance = context.balance != null ? context.balance.toFixed(4) : "—";
      const balanceClass = context.balance != null && context.balance < 0.01 ? "low-balance" : "";
      rows.push(`
        <div class="trade-action-context-item">
          <span class="trade-action-context-label">Available</span>
          <span class="trade-action-context-value ${balanceClass}">
            <span class="trade-action-balance-amount">${Utils.escapeHtml(balance)}</span>
            <span class="trade-action-balance-unit">SOL</span>
          </span>
        </div>
      `);
    }

    if (action === "add" && context.currentSize != null) {
      rows.push(`
        <div class="trade-action-context-item">
          <span class="trade-action-context-label">Current Position</span>
          <span class="trade-action-context-value">
            <span class="trade-action-balance-amount">${context.currentSize.toFixed(4)}</span>
            <span class="trade-action-balance-unit">SOL</span>
          </span>
        </div>
      `);
    }

    if (action === "sell" && context.holdings != null) {
      const formatted = Utils.formatCompactNumber(context.holdings, 2);
      rows.push(`
        <div class="trade-action-context-item">
          <span class="trade-action-context-label">Holdings</span>
          <span class="trade-action-context-value">${Utils.escapeHtml(formatted)} tokens</span>
        </div>
      `);
    }

    this.contextEl.innerHTML = `<div class="trade-action-context-grid">${rows.join("")}</div>`;
  }

  _buildPresets(action, context) {
    const config = ACTION_CONFIG[action];

    if (action === "buy" || action === "sell") {
      return config.presets;
    }

    if (action === "add") {
      const presets = [];

      // Multipliers based on entry size
      if (context.entrySize != null && context.entrySize > 0) {
        presets.push(
          {
            label: "1.0×",
            sublabel: `${context.entrySize.toFixed(3)} SOL`,
            value: context.entrySize,
            type: "amount",
            group: "multiplier",
          },
          {
            label: "1.5×",
            sublabel: `${(context.entrySize * 1.5).toFixed(3)} SOL`,
            value: context.entrySize * 1.5,
            type: "amount",
            group: "multiplier",
          },
          {
            label: "2.0×",
            sublabel: `${(context.entrySize * 2.0).toFixed(3)} SOL`,
            value: context.entrySize * 2.0,
            type: "amount",
            group: "multiplier",
          }
        );
      }

      // Entry sizes from config
      if (Array.isArray(context.entrySizes) && context.entrySizes.length > 0) {
        context.entrySizes.forEach((size) => {
          presets.push({
            label: `${size}`,
            sublabel: "SOL",
            value: size,
            type: "amount",
            group: "entry",
          });
        });
      }

      return presets;
    }

    return [];
  }

  _renderPresets(presets, action) {
    if (!presets || presets.length === 0) {
      this.presetsContainer.innerHTML = "";
      return;
    }

    // Group presets if needed
    const groups = {};
    presets.forEach((preset) => {
      const group = preset.group || "default";
      if (!groups[group]) {
        groups[group] = [];
      }
      groups[group].push(preset);
    });

    const sections = [];

    Object.entries(groups).forEach(([groupName, groupPresets]) => {
      const label =
        groupName === "multiplier"
          ? "Match Entry"
          : groupName === "entry"
            ? "Fixed Amount"
            : action === "sell"
              ? "Quick Sell"
              : "Quick Amount";

      const buttons = groupPresets
        .map(
          (preset) => `
        <button 
          type="button" 
          class="trade-action-preset-btn" 
          data-value="${preset.value}"
          data-type="${preset.type}"
          ${preset.default ? 'data-default="true"' : ""}
          aria-label="Select ${Utils.escapeHtml(preset.label)}"
        >
          <span class="preset-label">${Utils.escapeHtml(preset.label)}</span>
          ${preset.sublabel ? `<span class="preset-sublabel">${Utils.escapeHtml(preset.sublabel)}</span>` : ""}
        </button>
      `
        )
        .join("");

      sections.push(`
        <div class="trade-action-preset-section">
          <div class="trade-action-section-label">${label}</div>
          <div class="trade-action-preset-grid">
            ${buttons}
          </div>
        </div>
      `);
    });

    this.presetsContainer.innerHTML = sections.join("");

    // Clear old preset button references
    this._presetButtons.forEach((btn) => {
      off(btn, "click", this._presetClickListener);
    });
    this._presetButtons = [];

    // Attach click listeners to all preset buttons and track them
    this.presetsContainer.querySelectorAll(".trade-action-preset-btn").forEach((btn) => {
      on(btn, "click", this._presetClickListener);
      this._presetButtons.push(btn);
    });
  }

  _handlePresetClick(e) {
    const btn = e.target.closest(".trade-action-preset-btn");
    if (!btn) {
      return;
    }

    const value = parseFloat(btn.getAttribute("data-value"));
    const type = btn.getAttribute("data-type");

    // Clear all selections
    this.presetsContainer.querySelectorAll(".trade-action-preset-btn").forEach((b) => {
      b.classList.remove("selected");
    });

    // Select this one
    btn.classList.add("selected");
    this._selectedPreset = { value, type };

    // Fill input - use flag to prevent input handler from clearing preset
    this._settingInputProgrammatically = true;
    this.inputField.value = value;
    this._settingInputProgrammatically = false;

    // Clear error and validate
    this._clearError();
    this._updateConfirmButton();

    // Fetch quote for new amount
    this._fetchQuoteDebounced();
  }

  _handleInputChange() {
    // Skip if value was set programmatically (e.g., by clicking preset)
    if (this._settingInputProgrammatically) {
      return;
    }
    // Clear preset selection when user types
    this.presetsContainer.querySelectorAll(".trade-action-preset-btn").forEach((b) => {
      b.classList.remove("selected");
    });
    this._selectedPreset = null;

    // Clear error and validate
    this._clearError();
    this._updateConfirmButton();

    // Fetch quote when input changes
    this._fetchQuoteDebounced();
  }

  _updateConfirmButton() {
    const value = this._getInputValue();

    if (value === null || value === "") {
      // Empty input - allow if default behavior is acceptable
      if (this.currentAction === "buy" || this.currentAction === "add") {
        this.confirmBtn.disabled = false; // Allow empty for default
      } else {
        this.confirmBtn.disabled = true;
      }
      return;
    }

    const error = this._validateInput(this.currentAction, value, this.currentContext);
    this.confirmBtn.disabled = error !== null;
  }

  _getInputValue() {
    const raw = this.inputField.value.trim();
    if (raw === "") {
      return "";
    }
    const num = parseFloat(raw);
    return Number.isFinite(num) ? num : null;
  }

  _validateInput(action, value, context) {
    if (value === "") {
      return null; // Allow empty for default behavior
    }

    if (value === null) {
      return "Invalid number";
    }

    if (action === "sell") {
      if (value <= 0 || value > 100) {
        return "Percentage must be between 1 and 100";
      }
    }

    if (action === "buy" || action === "add") {
      if (value <= 0) {
        return "Amount must be greater than 0";
      }
      if (value < 0.001) {
        return "Minimum: 0.001 SOL";
      }
      if (context.balance != null && value > context.balance) {
        return `Insufficient balance (need ${value.toFixed(4)}, have ${context.balance.toFixed(4)})`;
      }
    }

    return null;
  }

  _showError(message) {
    if (this.errorTextEl) {
      this.errorTextEl.textContent = message;
    }
    this.errorEl.setAttribute("data-visible", "true");
    this.inputField.classList.add("error");
    this.confirmBtn.disabled = true;
  }

  _clearError() {
    this.errorEl.setAttribute("data-visible", "false");
    this.inputField.classList.remove("error");
  }

  _setLoading(loading) {
    this._isLoading = loading;
    if (loading) {
      this.confirmBtn.classList.add("loading");
      this.confirmBtn.disabled = true;
      this.cancelBtn.disabled = true;
      this.inputField.disabled = true;
      this._presetButtons.forEach((btn) => (btn.disabled = true));
    } else {
      this.confirmBtn.classList.remove("loading");
      this.cancelBtn.disabled = false;
      this.inputField.disabled = false;
      this._presetButtons.forEach((btn) => (btn.disabled = false));
      this._updateConfirmButton();
    }
  }

  async _handleConfirmClick() {
    if (this._isLoading) return;

    const value = this._getInputValue();

    // Validate
    if (value !== "" && value !== null) {
      const error = this._validateInput(this.currentAction, value, this.currentContext);
      if (error) {
        this._showError(error);
        return;
      }
    }

    // Slippage warning check (price impact > 5%)
    if (this._quoteData && this._quoteData.price_impact_pct > 5) {
      const confirmed = await this._showSlippageWarning(this._quoteData.price_impact_pct);
      if (!confirmed) {
        return; // User cancelled
      }
    }

    // Token balance verification for sell
    if (this.currentAction === "sell" && this.currentContext?.mint) {
      const verifyResult = await this._verifyTokenBalance();
      if (!verifyResult.ok) {
        this._showError(verifyResult.error);
        return;
      }
    }

    // Build result
    let result;
    if (this.currentAction === "sell") {
      result = {
        percentage: value === "" ? 100 : value,
      };
    } else {
      // buy or add
      result = value === "" ? {} : { amount: value };
    }

    // Resolve promise
    if (this._resolveOpen) {
      this._resolveOpen(result);
      this._resolveOpen = null;
    }

    this.close({ restoreFocus: true });
  }

  /**
   * Show slippage warning dialog for high price impact trades
   * @param {number} impactPct - Price impact percentage
   * @returns {Promise<boolean>} True if user confirms, false if cancelled
   */
  _showSlippageWarning(impactPct) {
    return new Promise((resolve) => {
      const overlay = document.createElement("div");
      overlay.className = "trade-slippage-warning-overlay";
      overlay.innerHTML = `
        <div class="trade-slippage-warning">
          <div class="slippage-warning-icon"><i class="icon-triangle-alert"></i></div>
          <div class="slippage-warning-title">High Price Impact Warning</div>
          <div class="slippage-warning-text">
            This trade has a price impact of <strong>${impactPct.toFixed(2)}%</strong>, 
            which is higher than recommended (5%). You may receive significantly less 
            than expected.
          </div>
          <div class="slippage-warning-buttons">
            <button class="slippage-warning-btn cancel">Cancel</button>
            <button class="slippage-warning-btn confirm">Proceed Anyway</button>
          </div>
        </div>
      `;

      overlay.querySelector(".cancel").onclick = () => {
        overlay.remove();
        resolve(false);
      };
      overlay.querySelector(".confirm").onclick = () => {
        overlay.remove();
        resolve(true);
      };

      document.body.appendChild(overlay);
    });
  }

  /**
   * Verify token balance before sell to prevent stale data trades
   * @returns {Promise<{ok: boolean, error?: string}>}
   */
  async _verifyTokenBalance() {
    try {
      const res = await fetch(
        `/api/positions/${encodeURIComponent(this.currentContext.mint)}/details`
      );
      if (!res.ok) {
        return { ok: false, error: "Could not verify token balance" };
      }
      const data = await res.json();
      if (!data.success || !data.data?.position?.summary) {
        return { ok: false, error: "Position not found - it may have been closed" };
      }

      const pos = data.data.position.summary;
      const currentHoldings = pos.remaining_token_amount ?? pos.token_amount ?? 0;
      const expectedHoldings = this.currentContext.holdings || 0;

      // Allow 1% variance for rounding
      const variance = Math.abs(currentHoldings - expectedHoldings) / expectedHoldings;
      if (variance > 0.01 && expectedHoldings > 0) {
        return {
          ok: false,
          error: `Token balance changed. Expected ${expectedHoldings.toFixed(2)}, now ${currentHoldings.toFixed(2)}. Please refresh.`,
        };
      }

      return { ok: true };
    } catch {
      return { ok: false, error: "Network error verifying balance" };
    }
  }

  _handleCancelClick() {
    if (this._resolveOpen) {
      this._resolveOpen(null); // null = cancelled
      this._resolveOpen = null;
    }
    this.close({ restoreFocus: true });
  }

  _handleCloseClick() {
    this._handleCancelClick();
  }

  _handleOverlayClick(event) {
    if (event.target === this.root) {
      this._handleCancelClick();
    }
  }

  _handleKeyDown(event) {
    if (!this._isOpen) {
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      this._handleCancelClick();
    }

    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();

      // Handle Enter for quick mode mint step
      if (this._quickMode && this._quickStep === "mint") {
        if (!this._quickContinueBtnEl.disabled) {
          this._handleQuickContinue();
        }
        return;
      }

      // Handle Enter for normal trade step
      if (!this.confirmBtn.disabled) {
        this._handleConfirmClick();
      }
    }
  }

  _debounce(func, wait) {
    let timeout;
    return (...args) => {
      clearTimeout(timeout);
      timeout = setTimeout(() => func.apply(this, args), wait);
    };
  }

  async _fetchQuote() {
    if (!this._isOpen || !this.currentContext?.mint) {
      return;
    }

    const amount = this._getSelectedAmount();
    if (!amount || amount <= 0) {
      this._setQuoteState("idle");
      return;
    }

    this._setQuoteState("loading");
    this._quoteData = null;
    this._quoteError = null;

    const direction = this.currentAction === "sell" ? "sell" : "buy";

    try {
      // Build URL based on direction
      let url;
      if (direction === "sell") {
        // For sell, amount is percentage, calculate token amount from holdings
        const holdings = this.currentContext.holdings || 0;
        if (holdings <= 0) {
          throw new Error("No holdings available to sell");
        }
        const tokenAmount = holdings * (amount / 100);
        url = `/api/trader/quote?mint=${encodeURIComponent(this.currentContext.mint)}&amount_tokens=${tokenAmount}&direction=sell`;
      } else {
        // For buy/add, amount is SOL
        url = `/api/trader/quote?mint=${encodeURIComponent(this.currentContext.mint)}&amount_sol=${amount}&direction=buy`;
      }

      const response = await fetch(url);
      const data = await response.json();

      if (!this._isOpen) return; // Dialog closed during fetch

      if (data.success && data.data) {
        this._quoteData = data.data;
        this._quoteError = null;
        this._quoteTimestamp = Date.now();
        this._renderQuote(data.data);
        this._setQuoteState("loaded");
        this._startQuoteRefreshTimer();
      } else {
        throw new Error(data.error?.message || "Failed to fetch quote");
      }
    } catch (err) {
      if (!this._isOpen) return;
      this._quoteError = err.message;
      this._quoteData = null;
      this.quoteErrorTextEl.textContent = err.message;
      this._setQuoteState("error");
    }
  }

  _renderQuote(quote) {
    // Output amount
    this.quoteOutputEl.textContent = `~${quote.output_formatted}`;

    // Price impact with color
    const impactPct = quote.price_impact_pct.toFixed(2);
    this.quoteImpactEl.textContent = `${impactPct}%`;
    this.quoteImpactEl.className = "quote-value quote-impact";
    if (quote.price_impact_pct > 5) {
      this.quoteImpactEl.classList.add("impact-high");
    } else if (quote.price_impact_pct > 1) {
      this.quoteImpactEl.classList.add("impact-medium");
    } else {
      this.quoteImpactEl.classList.add("impact-low");
    }

    // Fees
    this.quotePlatformFeeEl.textContent = `${quote.platform_fee_pct}% (${quote.platform_fee_sol.toFixed(6)} SOL)`;
    this.quoteNetworkFeeEl.textContent = `~${quote.network_fee_sol.toFixed(6)} SOL`;

    // Route and slippage
    this.quoteRouteEl.textContent = quote.router || "Unknown";
    this.quoteSlippageEl.textContent = `${(quote.slippage_bps / 100).toFixed(1)}%`;
  }

  _setQuoteState(state) {
    if (this.quoteSection) {
      this.quoteSection.dataset.state = state;
    }
  }

  _getSelectedAmount() {
    // First check for selected preset
    if (this._selectedPreset !== null) {
      // For all actions, return the preset value
      // For sell, this is the percentage (25, 50, 75, 100)
      // For buy/add, this is the SOL amount
      return this._selectedPreset.value;
    }
    // Then check input field
    const inputVal = parseFloat(this.inputField?.value);
    if (!isNaN(inputVal) && inputVal > 0) {
      return inputVal;
    }
    return null;
  }

  _startQuoteRefreshTimer() {
    this._stopQuoteRefreshTimer();
    this._quoteRefreshTimer = setInterval(() => {
      if (this._isOpen && this._quoteData) {
        const age = Math.floor((Date.now() - this._quoteTimestamp) / 1000);
        if (this.quoteAgeEl) {
          this.quoteAgeEl.textContent = `${age}s`;
        }
        // Auto-refresh after 15 seconds
        if (age >= 15) {
          this._fetchQuote();
        }
      }
    }, 1000);
  }

  _stopQuoteRefreshTimer() {
    if (this._quoteRefreshTimer) {
      clearInterval(this._quoteRefreshTimer);
      this._quoteRefreshTimer = null;
    }
  }

  _handleQuoteRefresh(e) {
    e.preventDefault();
    e.stopPropagation();
    this._fetchQuote();
  }

  // ============================================
  // Quick Trade Mode Methods
  // ============================================

  /**
   * Validate if a string is a valid Solana mint address
   * @param {string} value - The string to validate
   * @returns {boolean}
   */
  _isValidMint(value) {
    return /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(value);
  }

  /**
   * Render the quick trade mint input step
   * @param {string} action - 'buy' or 'sell'
   */
  _renderQuickMintStep(action) {
    const config = ACTION_CONFIG[action];

    // Set action-specific class on dialog
    this.dialog.className = `trade-action-dialog ${config.colorClass} quick-mode`;

    // Update icon based on action
    const iconSvg = this._getActionIcon(config.icon);
    this.iconWrapper.innerHTML = iconSvg;

    // Set title for quick mode
    const quickTitle = action === "buy" ? "Quick Buy" : "Quick Sell";
    this.titleEl.textContent = quickTitle;
    this.subtitleEl.textContent = "Enter token mint address";

    // Hide trade body/footer, show quick mint step
    this._tradeBodyEl.style.display = "none";
    this._tradeFooterEl.style.display = "none";
    this._quickMintStepEl.setAttribute("data-visible", "true");

    // Reset mint input state
    this._quickMintInputEl.value = "";
    this._quickContinueBtnEl.disabled = true;
    this._quickErrorEl.setAttribute("data-visible", "false");
    this._quickTokenPreviewEl.setAttribute("data-visible", "false");
    this._quickTokenLoadingEl.style.display = "none";
    this._quickTokenInfoEl.style.display = "none";
  }

  /**
   * Handle paste button click in quick trade mode
   */
  async _handleQuickPaste() {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        this._quickMintInputEl.value = text.trim();
        this._handleQuickMintInput();
      }
    } catch {
      // Clipboard access denied - show subtle feedback
      this._quickPasteBtnEl.classList.add("error-flash");
      setTimeout(() => this._quickPasteBtnEl.classList.remove("error-flash"), 300);
    }
  }

  /**
   * Handle mint input changes in quick trade mode
   */
  _handleQuickMintInput() {
    const value = this._quickMintInputEl.value.trim();

    // Hide error when typing
    this._quickErrorEl.setAttribute("data-visible", "false");

    if (!value) {
      this._quickContinueBtnEl.disabled = true;
      this._quickTokenPreviewEl.setAttribute("data-visible", "false");
      this._fetchedTokenData = null;
      return;
    }

    // Validate mint format
    if (!this._isValidMint(value)) {
      this._quickContinueBtnEl.disabled = true;
      this._quickTokenPreviewEl.setAttribute("data-visible", "false");
      this._fetchedTokenData = null;

      // Only show error if they've typed enough characters
      if (value.length >= 32) {
        this._showQuickError("Invalid mint address format");
      }
      return;
    }

    // Valid format - fetch token info
    this._fetchQuickTokenInfo(value);
  }

  /**
   * Fetch token info for quick trade mode
   * @param {string} mint - The mint address
   */
  async _fetchQuickTokenInfo(mint) {
    // Show loading state
    this._quickTokenPreviewEl.setAttribute("data-visible", "true");
    this._quickTokenLoadingEl.style.display = "flex";
    this._quickTokenInfoEl.style.display = "none";
    this._quickContinueBtnEl.disabled = true;
    this._fetchedTokenData = null;

    try {
      const response = await fetch(`/api/tokens/${encodeURIComponent(mint)}`);

      if (!this._isOpen || this._quickMintInputEl.value.trim() !== mint) {
        return; // Dialog closed or mint changed
      }

      if (!response.ok) {
        throw new Error(response.status === 404 ? "Token not found" : "Failed to fetch token");
      }

      const data = await response.json();

      if (!data.success || !data.data) {
        throw new Error("Token not found in database");
      }

      const token = data.data;
      this._fetchedTokenData = token;

      // Update preview
      this._quickTokenSymbolEl.textContent = token.symbol || "Unknown";
      this._quickTokenNameEl.textContent = token.name || mint.slice(0, 8) + "...";

      if (token.price_sol != null && token.price_sol > 0) {
        const priceFormatted =
          token.price_sol < 0.000001
            ? token.price_sol.toExponential(4)
            : token.price_sol.toFixed(9).replace(/\.?0+$/, "");
        this._quickTokenPriceEl.textContent = `${priceFormatted} SOL`;
        this._quickTokenPriceEl.style.display = "block";
      } else {
        this._quickTokenPriceEl.style.display = "none";
      }

      // Show token info, hide loading
      this._quickTokenLoadingEl.style.display = "none";
      this._quickTokenInfoEl.style.display = "flex";
      this._quickContinueBtnEl.disabled = false;
    } catch (err) {
      if (!this._isOpen) return;

      this._quickTokenPreviewEl.setAttribute("data-visible", "false");
      this._showQuickError(err.message || "Failed to fetch token info");
      this._fetchedTokenData = null;
    }
  }

  /**
   * Handle continue button click in quick trade mode
   */
  async _handleQuickContinue() {
    if (!this._fetchedTokenData) return;

    const mint = this._quickMintInputEl.value.trim();
    const token = this._fetchedTokenData;

    // For sell action, verify holdings exist
    if (this.currentAction === "sell") {
      this._quickContinueBtnEl.classList.add("loading");
      this._quickContinueBtnEl.disabled = true;

      try {
        const response = await fetch(`/api/positions/${encodeURIComponent(mint)}/details`);

        if (!this._isOpen) return;

        if (!response.ok || response.status === 404) {
          this._quickContinueBtnEl.classList.remove("loading");
          this._showQuickError("No position found for this token");
          return;
        }

        const data = await response.json();

        if (!data.success || !data.data?.position?.summary) {
          this._quickContinueBtnEl.classList.remove("loading");
          this._showQuickError("No position found for this token");
          return;
        }

        const pos = data.data.position.summary;
        const holdings = pos.remaining_token_amount ?? pos.token_amount ?? 0;

        if (holdings <= 0) {
          this._quickContinueBtnEl.classList.remove("loading");
          this._showQuickError("Position has no remaining tokens");
          return;
        }

        // Update context with holdings
        this.currentContext.holdings = holdings;
        this.currentContext.mint = mint;

        this._quickContinueBtnEl.classList.remove("loading");
      } catch {
        if (!this._isOpen) return;
        this._quickContinueBtnEl.classList.remove("loading");
        this._showQuickError("Failed to fetch position data");
        return;
      }
    } else {
      // For buy, just set the mint
      this.currentContext.mint = mint;
    }

    // Update symbol
    this._currentSymbol = token.symbol || "Unknown";

    // Transition to trade step
    this._transitionToTradeStep();
  }

  /**
   * Transition from mint input step to trade step with animation
   */
  _transitionToTradeStep() {
    this._quickStep = "trade";

    // Animate out mint step
    this._quickMintStepEl.classList.add("slide-out");

    setTimeout(() => {
      // Hide mint step
      this._quickMintStepEl.setAttribute("data-visible", "false");
      this._quickMintStepEl.classList.remove("slide-out");

      // Render and show trade step
      this._render(this.currentAction, this._currentSymbol, this.currentContext);

      // Show trade body/footer with animation
      this._tradeBodyEl.style.display = "";
      this._tradeFooterEl.style.display = "";
      this._tradeBodyEl.classList.add("slide-in");
      this._tradeFooterEl.classList.add("slide-in");

      // Update title back to normal
      const config = ACTION_CONFIG[this.currentAction];
      this.titleEl.textContent = config.title;
      this.subtitleEl.textContent = config.subtitle;

      setTimeout(() => {
        this._tradeBodyEl.classList.remove("slide-in");
        this._tradeFooterEl.classList.remove("slide-in");

        // Focus and auto-select default preset
        this.dialog?.focus();
        const defaultPreset = this.presetsContainer.querySelector(
          ".trade-action-preset-btn[data-default='true']"
        );
        if (defaultPreset) {
          defaultPreset.click();
        }
      }, 200);
    }, 150);
  }

  /**
   * Show error message in quick trade mode
   * @param {string} message - The error message
   */
  _showQuickError(message) {
    this._quickErrorTextEl.textContent = message;
    this._quickErrorEl.setAttribute("data-visible", "true");
    this._quickContinueBtnEl.disabled = true;
  }

  /**
   * Inject CSS styles for quick trade mode
   */
  _injectQuickTradeStyles() {
    if (document.getElementById("quick-trade-styles")) return;

    const style = document.createElement("style");
    style.id = "quick-trade-styles";
    style.textContent = `
      /* Quick Trade Mint Step */
      .quick-trade-mint-step {
        display: none;
        flex-direction: column;
        padding: 1.5rem;
        gap: 1rem;
      }
      .quick-trade-mint-step[data-visible="true"] {
        display: flex;
      }
      .quick-trade-mint-step.slide-out {
        animation: slideOutLeft 0.15s ease-out forwards;
      }
      
      .quick-trade-mint-content {
        display: flex;
        flex-direction: column;
        gap: 1rem;
      }
      
      .quick-trade-mint-label {
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--color-text-secondary);
      }
      
      .quick-trade-mint-input-wrapper {
        display: flex;
        gap: 0.5rem;
        align-items: stretch;
      }
      
      .quick-trade-mint-input {
        flex: 1;
        padding: 0.875rem 1rem;
        font-family: var(--font-mono);
        font-size: 0.9375rem;
        background: var(--color-surface);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        color: var(--color-text-primary);
        transition: border-color 0.15s, box-shadow 0.15s;
      }
      .quick-trade-mint-input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
      }
      .quick-trade-mint-input::placeholder {
        color: var(--color-text-muted);
      }
      
      .quick-trade-paste-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 44px;
        background: var(--color-surface);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        color: var(--color-text-secondary);
        cursor: pointer;
        transition: all 0.15s;
      }
      .quick-trade-paste-btn:hover {
        background: var(--color-surface-hover);
        color: var(--color-text-primary);
        border-color: var(--color-border-hover);
      }
      .quick-trade-paste-btn svg {
        width: 18px;
        height: 18px;
      }
      .quick-trade-paste-btn.error-flash {
        background: rgba(239, 68, 68, 0.15);
        border-color: var(--color-error);
      }
      
      /* Quick Trade Error */
      .quick-trade-error {
        display: none;
        align-items: center;
        gap: 0.5rem;
        padding: 0.625rem 0.875rem;
        font-size: 0.8125rem;
        color: var(--color-error);
        background: rgba(239, 68, 68, 0.1);
        border-radius: var(--radius-sm);
      }
      .quick-trade-error[data-visible="true"] {
        display: flex;
      }
      .quick-trade-error svg {
        width: 16px;
        height: 16px;
        flex-shrink: 0;
      }
      
      /* Quick Trade Token Preview */
      .quick-trade-token-preview {
        display: none;
        padding: 1rem;
        background: var(--color-surface);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
      }
      .quick-trade-token-preview[data-visible="true"] {
        display: block;
      }
      
      .quick-trade-token-loading {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        color: var(--color-text-secondary);
        font-size: 0.875rem;
      }
      
      .quick-trade-spinner {
        width: 18px;
        height: 18px;
        border: 2px solid var(--color-border);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
      }
      
      .quick-trade-token-info {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
      }
      
      .quick-trade-token-symbol {
        font-size: 1.125rem;
        font-weight: 600;
        color: var(--color-text-primary);
      }
      
      .quick-trade-token-name {
        font-size: 0.8125rem;
        color: var(--color-text-secondary);
      }
      
      .quick-trade-token-price {
        font-family: var(--font-mono);
        font-size: 0.875rem;
        color: var(--color-text-muted);
        margin-top: 0.25rem;
      }
      
      /* Quick Trade Footer */
      .quick-trade-mint-footer {
        display: flex;
        gap: 0.75rem;
        justify-content: flex-end;
        padding-top: 0.5rem;
        border-top: 1px solid var(--color-border);
        margin-top: 0.5rem;
      }
      
      /* Animations */
      @keyframes slideOutLeft {
        from {
          opacity: 1;
          transform: translateX(0);
        }
        to {
          opacity: 0;
          transform: translateX(-20px);
        }
      }
      
      .slide-in {
        animation: slideInRight 0.2s ease-out forwards;
      }
      
      @keyframes slideInRight {
        from {
          opacity: 0;
          transform: translateX(20px);
        }
        to {
          opacity: 1;
          transform: translateX(0);
        }
      }
      
      @keyframes spin {
        to { transform: rotate(360deg); }
      }
      
      /* Quick mode dialog adjustments */
      .trade-action-dialog.quick-mode .trade-action-header {
        border-bottom: none;
      }
    `;
    document.head.appendChild(style);
  }
}
