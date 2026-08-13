import { on, off } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { pushEscapeHandler } from "../core/escape_stack.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "./hint_popover.js";
import { applyQuickTradeMixin } from "./trade_action/quick_trade.js";
import { applyQuoteManagerMixin } from "./trade_action/quote_manager.js";

// Hard ceiling on a manual slippage override. MUST match the backend's
// trader::constants::MAX_MANUAL_SLIPPAGE_PCT — the route rejects anything above it.
const MAX_SLIPPAGE_PCT = 50;

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
    // The backend's default add is trade_size_sol x dca_size_percentage — NOT "50% of
    // the original entry", which is what this used to claim.
    inputHint: "Leave empty for the configured DCA size",
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
    this._releaseEscape = null;

    // Quick trade mode bound listeners
    this._quickPasteListener = this._handleQuickPaste.bind(this);
    this._quickContinueListener = this._handleQuickContinue.bind(this);
    this._quickMintInputListener = this._handleQuickMintInput.bind(this);
    this._quoteRefreshListener = this._handleQuoteRefresh.bind(this);

    // Search state
    this._searchResults = [];
    this._searchSelectedIndex = -1;
    this._searchDropdownEl = null;
    this._searchDebounceTimer = null;
    this._isSearching = false;
    this._searchKeyListener = this._handleSearchKeyDown.bind(this);

    // Slippage state. null = "Auto" = follow the configured slippage (what the
    // auto-trader always does); a number is a per-trade manual override.
    this._slippagePct = null;
    this._configuredSlippage = null;
    this._slippageBtnListener = this._handleSlippageBtnClick.bind(this);
    this._slippageInputListener = this._handleSlippageInputChange.bind(this);

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
          <div class="trade-action-grid">
          <div class="trade-action-pane trade-action-pane-form">
          <div class="trade-action-context"></div>
          <div class="trade-action-presets"></div>
          <div class="trade-action-input-section">
            <label class="trade-action-input-label" for="trade-action-input"></label>
            <div class="trade-action-input-wrapper">
              <!-- data-stepper="off": MAX and the percentage slider already own
                   this field's gutter, and an amount is chosen, not incremented. -->
              <input type="number" id="trade-action-input" class="trade-action-input" step="any" min="0" inputmode="decimal" data-stepper="off" />
              <span class="trade-action-input-suffix">SOL</span>
              <button type="button" class="trade-action-input-max" data-action="max" aria-label="Use maximum">MAX</button>
            </div>
            <div class="trade-action-slider-row" data-visible="false">
              <div class="trade-action-slider-wrap">
                <input type="range" class="trade-action-slider" min="0" max="100" step="1" value="0" aria-label="Amount slider" />
                <div class="trade-action-slider-ticks" aria-hidden="true"></div>
              </div>
              <span class="trade-action-slider-readout"></span>
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
          <!--
            Slippage. "Auto" follows the configured slippage (swaps.slippage.*) —
            which is what the auto-trader ALWAYS uses. Everything else is a per-trade
            manual override, for forcing a fill on an illiquid token.
          -->
          <div class="trade-action-slippage">
            <div class="trade-action-slippage-head">
              <span class="trade-action-slippage-label">Slippage</span>
              <span class="trade-action-slippage-note"></span>
            </div>
            <div class="trade-action-slippage-options">
              <div class="trade-action-slippage-presets" role="group" aria-label="Slippage preset">
                <button type="button" class="trade-action-slippage-btn" data-slippage="auto">Auto</button>
                <button type="button" class="trade-action-slippage-btn" data-slippage="1">1%</button>
                <button type="button" class="trade-action-slippage-btn" data-slippage="5">5%</button>
                <button type="button" class="trade-action-slippage-btn" data-slippage="15">15%</button>
              </div>
              <input
                type="number"
                class="trade-action-slippage-input"
                placeholder="Custom"
                min="0.1"
                max="50"
                step="0.1"
                inputmode="decimal"
                aria-label="Custom slippage percent"
              />
            </div>
            <div class="trade-action-slippage-warning" data-visible="false"></div>
          </div>

          <!-- Manual management (buy only): keep this position off the auto-trader. -->
          <label class="trade-action-manage-row" data-visible="false">
            <input type="checkbox" class="trade-action-manage-checkbox" checked />
            <span class="trade-action-manage-copy">
              <span class="trade-action-manage-title">
                Manual management
                <span class="trade-action-manage-hint"></span>
              </span>
              <span class="trade-action-manage-sub">Auto-trader won't sell or DCA this position. Uncheck to let it manage exits.</span>
            </span>
          </label>
          </div>
          <div class="trade-action-pane trade-action-pane-preview">
          <div class="trade-action-quote-section" data-state="idle" data-refreshing="false">
            <div class="trade-action-quote-refresh-bar"></div>
            <div class="trade-action-quote-header">
              <span class="trade-action-quote-title">Swap Preview</span>
              <button type="button" class="trade-action-quote-refresh" aria-label="Refresh quote" title="Refresh quote">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/>
                </svg>
                <span class="quote-age"></span>
              </button>
            </div>
            <div class="trade-action-quote-idle">
              <span>Choose an amount to preview your swap</span>
            </div>
            <div class="trade-action-quote-loading">
              <div class="trade-action-quote-spinner"></div>
              <span>Finding the best route…</span>
            </div>
            <div class="trade-action-quote-error">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 8v4M12 16h.01"/>
              </svg>
              <div class="quote-error-body">
                <span class="quote-error-text"></span>
                <span class="quote-error-detail"></span>
                <button type="button" class="quote-error-retry">Try again</button>
              </div>
            </div>
            <div class="trade-action-quote-content">
              <div class="quote-hero">
                <span class="quote-hero-label">You receive (estimated)</span>
                <span class="quote-hero-value quote-output">—</span>
                <span class="quote-hero-sub quote-unit-price"></span>
              </div>
              <div class="quote-rows">
                <div class="quote-row" data-min="minimal">
                  <span class="quote-label">Price impact</span>
                  <span class="quote-value quote-impact"></span>
                </div>
                <div class="quote-row" data-min="minimal">
                  <span class="quote-label">Minimum received <span class="quote-info" title="The least you will get after max slippage. The swap reverts if it cannot fill at this amount.">?</span></span>
                  <span class="quote-value quote-min-received"></span>
                </div>
                <div class="quote-row" data-min="normal">
                  <span class="quote-label">Platform fee <span class="quote-info" title="0.5% — supports development. Built into the quote.">?</span></span>
                  <span class="quote-value quote-platform-fee"></span>
                </div>
                <div class="quote-row" data-min="normal">
                  <span class="quote-label">Network fee</span>
                  <span class="quote-value quote-network-fee"></span>
                </div>
                <div class="quote-row" data-min="normal">
                  <span class="quote-label">Max slippage</span>
                  <span class="quote-value quote-slippage"></span>
                </div>
                <div class="quote-row" data-min="normal">
                  <span class="quote-label">Provider</span>
                  <span class="quote-value quote-route"></span>
                </div>
                <div class="quote-row" data-min="advanced">
                  <span class="quote-label">You pay</span>
                  <span class="quote-value quote-you-pay"></span>
                </div>
                <div class="quote-row" data-min="advanced">
                  <span class="quote-label">Route</span>
                  <span class="quote-value quote-route-path"></span>
                </div>
                <div class="quote-row" data-min="advanced">
                  <span class="quote-label">Auto-refresh</span>
                  <span class="quote-value quote-expiry"></span>
                </div>
              </div>
              <div class="quote-disclaimer" data-min="advanced">
                Estimates update with live on-chain prices. The transaction reverts if it can't
                fill within your max slippage, so you never spend more than shown.
              </div>
            </div>
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
              <input type="text" class="quick-trade-mint-input" placeholder="Enter mint address or search by symbol..." autocomplete="off" spellcheck="false" />
              <button type="button" class="quick-trade-paste-btn" aria-label="Paste from clipboard">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                  <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/>
                </svg>
              </button>
            </div>
            <div class="quick-trade-search-results" data-visible="false"></div>
            <div class="quick-trade-recent" data-visible="false">
              <span class="quick-trade-recent-label">Recent:</span>
              <div class="quick-trade-recent-list"></div>
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
                <div class="quick-trade-token-symbol token-symbol-type"></div>
                <div class="quick-trade-token-name token-name-type"></div>
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
    this.manageRowEl = overlay.querySelector(".trade-action-manage-row");
    this.manageCheckboxEl = overlay.querySelector(".trade-action-manage-checkbox");
    this.manageHintEl = overlay.querySelector(".trade-action-manage-hint");
    this.slippageNoteEl = overlay.querySelector(".trade-action-slippage-note");
    this.slippageInputEl = overlay.querySelector(".trade-action-slippage-input");
    this.slippageWarningEl = overlay.querySelector(".trade-action-slippage-warning");
    this.slippageBtns = Array.from(overlay.querySelectorAll(".trade-action-slippage-btn"));
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
    this.quoteErrorDetailEl = overlay.querySelector(".quote-error-detail");
    this.quoteErrorRetryBtn = overlay.querySelector(".quote-error-retry");
    this.quoteUnitPriceEl = overlay.querySelector(".quote-unit-price");
    this.quoteMinReceivedEl = overlay.querySelector(".quote-min-received");
    this.quoteYouPayEl = overlay.querySelector(".quote-you-pay");
    this.quoteRoutePathEl = overlay.querySelector(".quote-route-path");
    this.quoteExpiryEl = overlay.querySelector(".quote-expiry");
    this.quoteContentEl = overlay.querySelector(".trade-action-quote-content");

    // Amount controls (slider + MAX)
    this.sliderRow = overlay.querySelector(".trade-action-slider-row");
    this.sliderEl = overlay.querySelector(".trade-action-slider");
    this.sliderReadoutEl = overlay.querySelector(".trade-action-slider-readout");
    this.sliderTicksEl = overlay.querySelector(".trade-action-slider-ticks");
    this.maxBtn = overlay.querySelector(".trade-action-input-max");

    // Quick trade step elements
    this._quickMintStepEl = overlay.querySelector(".quick-trade-mint-step");
    this._quickMintInputEl = overlay.querySelector(".quick-trade-mint-input");
    this._quickPasteBtnEl = overlay.querySelector(".quick-trade-paste-btn");
    this._searchDropdownEl = overlay.querySelector(".quick-trade-search-results");
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
    on(this.quoteErrorRetryBtn, "click", this._quoteRefreshListener);

    // Amount slider + MAX
    this._sliderListener = this._handleSliderInput.bind(this);
    this._maxListener = this._handleMaxClick.bind(this);
    on(this.sliderEl, "input", this._sliderListener);
    this.slippageBtns.forEach((btn) => on(btn, "click", this._slippageBtnListener));
    on(this.slippageInputEl, "input", this._slippageInputListener);
    on(this.maxBtn, "click", this._maxListener);

    // Quick trade step listeners
    on(this._quickPasteBtnEl, "click", this._quickPasteListener);
    on(this._quickContinueBtnEl, "click", this._quickContinueListener);
    on(this._quickCancelBtnEl, "click", this._cancelListener);
    on(this._quickMintInputEl, "input", this._quickMintInputListener);
    on(this._quickMintInputEl, "keydown", this._searchKeyListener);
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
    this._releaseEscape = pushEscapeHandler(() => this._handleCancelClick());

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

        // `context.preselect` picks a specific preset (Close Position opens the sell
        // dialog at 100%). It was read by NO code before, so close-position never
        // pre-selected anything — and its caller then only submitted when the user
        // happened to choose exactly 100%, silently doing nothing otherwise.
        const preselected =
          context.preselect != null
            ? this.presetsContainer.querySelector(
                `.trade-action-preset-btn[data-value="${context.preselect}"]`
              )
            : null;

        const target =
          preselected ||
          this.presetsContainer.querySelector(".trade-action-preset-btn[data-default='true']");

        if (target) target.click();
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
    this._releaseEscape?.();
    this._releaseEscape = null;

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
    off(this.quoteErrorRetryBtn, "click", this._quoteRefreshListener);
    off(this.sliderEl, "input", this._sliderListener);
    this.slippageBtns.forEach((btn) => off(btn, "click", this._slippageBtnListener));
    off(this.slippageInputEl, "input", this._slippageInputListener);
    off(this.maxBtn, "click", this._maxListener);

    // Clean up quick trade mode listeners
    off(this._quickPasteBtnEl, "click", this._quickPasteListener);
    off(this._quickContinueBtnEl, "click", this._quickContinueListener);
    off(this._quickCancelBtnEl, "click", this._cancelListener);
    off(this._quickMintInputEl, "input", this._quickMintInputListener);
    off(this._quickMintInputEl, "keydown", this._searchKeyListener);
    this._clearSearchDebounce();

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

    // Set title and subtitle (header is minimal — no icon)
    this.titleEl.textContent = config.title;
    this.subtitleEl.textContent = config.subtitle;

    // Render context info
    this._renderContext(action, symbol, context);

    // Build and render presets
    const presets = this._buildPresets(action, context);
    this._renderPresets(presets, action);

    // Configure the amount slider + MAX for this action/context
    this._configureAmountControls(action, context);

    // Set input labels and state
    this.inputLabelEl.textContent = config.inputLabel;
    this.inputHintEl.textContent = config.inputHint;
    this.inputField.value = "";
    this.inputField.placeholder = config.inputPlaceholder;

    // Update input suffix based on action
    this.inputSuffix.textContent = action === "sell" ? "%" : "SOL";
    this.inputSuffix.style.display = "block";

    // Manual-management choice is only meaningful when opening a position (buy).
    this._renderManageOption(action);

    // Slippage resets to Auto on every open — an override is per-trade.
    this._renderSlippage();

    // Set confirm button label and reset loading state
    const btnText = this.confirmBtn.querySelector(".btn-text");
    if (btnText) btnText.textContent = config.confirmLabel;
    this.confirmBtn.disabled = true;
    this.confirmBtn.classList.remove("loading");

    // Clear error
    this._clearError();
  }

  /**
   * Show/hide and reset the "Manual management" checkbox. Only the buy action opens a
   * new position, so it's the only place the choice applies. Defaults to checked
   * (protected) so the auto-trader won't sell a token the user bought on purpose.
   */
  _renderManageOption(action) {
    if (!this.manageRowEl) return;
    const isBuy = action === "buy";
    this.manageRowEl.setAttribute("data-visible", isBuy ? "true" : "false");
    if (!isBuy) return;

    this.manageCheckboxEl.checked = true;

    // Inject the hint trigger once (idempotent) and wire the global popover handler.
    if (this.manageHintEl && !this.manageHintEl.firstChild) {
      const hint = Hints.getHint("positions.positionManagement");
      if (hint) {
        this.manageHintEl.innerHTML = HintTrigger.render(hint, "positions.positionManagement", {
          size: "sm",
          position: "bottom",
        });
        HintTrigger.initAll();
      }
    }
  }

  /**
   * The configured slippage (swaps.slippage.quote_default_pct) — what "Auto" means,
   * and what the auto-trader always uses. Fetched once and cached; the dialog still
   * works without it (Auto just sends no override and the backend applies the same
   * config value server-side).
   */
  async _loadConfiguredSlippage() {
    if (this._configuredSlippage != null) return this._configuredSlippage;

    try {
      const res = await fetch("/api/config/swaps");
      if (res.ok) {
        const body = await res.json();
        const pct = Number(body?.data?.slippage?.quote_default_pct);
        if (Number.isFinite(pct)) this._configuredSlippage = pct;
      }
    } catch {
      // Leave it null — Auto still works, it just cannot show the number.
    }

    return this._configuredSlippage;
  }

  /**
   * Reset the slippage control to Auto (config-driven) for a freshly opened dialog.
   * A previous trade's override must never leak into the next one.
   */
  _renderSlippage() {
    this._slippagePct = null; // null = Auto = follow config
    if (this.slippageInputEl) this.slippageInputEl.value = "";
    this._syncSlippageUi();

    this._loadConfiguredSlippage().then(() => this._syncSlippageUi());
  }

  /**
   * Paint the active state, the "Auto = N%" note, and the high-slippage warning.
   */
  _syncSlippageUi() {
    const active = this._slippagePct;
    let matchedPreset = false;

    this.slippageBtns.forEach((btn) => {
      const value = btn.dataset.slippage;
      const isActive =
        value === "auto" ? active == null : active != null && Number(value) === active;
      if (isActive) matchedPreset = true;
      btn.classList.toggle("active", isActive);
    });

    // A value no preset covers lives in the custom field — mark the field itself,
    // so the control always shows exactly one active choice.
    if (this.slippageInputEl) {
      this.slippageInputEl.classList.toggle("active", active != null && !matchedPreset);
    }

    if (this.slippageNoteEl) {
      this.slippageNoteEl.textContent =
        active == null
          ? this._configuredSlippage != null
            ? `Auto (${this._configuredSlippage}% from settings)`
            : "Auto (from settings)"
          : `Override: ${active}%`;
    }

    // A large override is a real way to lose money to sandwich bots — say so.
    if (this.slippageWarningEl) {
      const risky = active != null && active >= 10;
      this.slippageWarningEl.setAttribute("data-visible", risky ? "true" : "false");
      if (risky) {
        this.slippageWarningEl.textContent = `High slippage: you may receive up to ${active}% less than quoted.`;
      }
    }
  }

  /**
   * Slippage preset / custom input changed. Re-quotes, because the quote must be
   * priced at the slippage the trade will actually execute with.
   */
  _handleSlippageChange(value) {
    if (value === "auto" || value === "" || value == null) {
      this._slippagePct = null;
      if (this.slippageInputEl) this.slippageInputEl.value = "";
    } else {
      const pct = Number(value);
      if (!Number.isFinite(pct) || pct <= 0 || pct > MAX_SLIPPAGE_PCT) return;
      this._slippagePct = pct;
    }

    this._syncSlippageUi();
    this._fetchQuoteDebounced();
  }

  _handleSlippageBtnClick(e) {
    const btn = e.currentTarget;
    // A preset supersedes whatever was typed: a stale custom number left in the
    // field would claim a slippage the trade is not using.
    if (this.slippageInputEl) this.slippageInputEl.value = "";
    this._handleSlippageChange(btn.dataset.slippage);
  }

  _handleSlippageInputChange() {
    this._handleSlippageChange(this.slippageInputEl.value.trim());
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

  /**
   * Token identity strip: logo, name, symbol — so the user can SEE which token they
   * are about to trade, not just a bare ticker. Replaces a plain "Token: SYM" row.
   *
   * Everything is optional: a token with no logo falls back to its initial, and one
   * with no name shows the symbol alone.
   */
  _renderIdentity(symbol, context) {
    const displaySymbol = (symbol || "?").toUpperCase();
    const logo = Utils.normalizeImageUrl(context.logo);
    const initial = Utils.escapeHtml(displaySymbol.charAt(0));

    const logoHtml = logo
      ? `<img src="${Utils.escapeHtml(logo)}" alt="" class="trade-action-token-logo-img"
           onerror="this.replaceWith(Object.assign(document.createElement('span'),{className:'trade-action-token-initial',textContent:'${initial}'}))"/>`
      : `<span class="trade-action-token-initial">${initial}</span>`;

    // Name is only worth its own line when it differs from the symbol.
    const name = context.name && context.name.toUpperCase() !== displaySymbol ? context.name : "";

    return `
      <div class="trade-action-token">
        <div class="trade-action-token-logo">${logoHtml}</div>
        <div class="trade-action-token-meta">
          <span class="trade-action-token-symbol token-symbol-type">${Utils.escapeHtml(displaySymbol)}</span>
          ${name ? `<span class="trade-action-token-name token-name-type">${Utils.escapeHtml(name)}</span>` : ""}
        </div>
        ${
          context.hasPosition
            ? '<span class="trade-action-position-badge" title="You hold an open position in this token">Held</span>'
            : ""
        }
      </div>
    `;
  }

  _renderContext(action, symbol, context) {
    const rows = [];

    // Store mint for quote fetching
    if (context.mint) {
      this.currentContext.mint = context.mint;
    }

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

    // Invested size, shown for ANY action on a held token — buying more of something
    // you already hold is exactly when you want to see the existing exposure.
    // (This row used to be gated on `action === "add"` AND a `currentSize` that no
    // caller ever passed, so it never rendered at all.)
    if (context.hasPosition && context.currentSize != null) {
      rows.push(`
        <div class="trade-action-context-item">
          <span class="trade-action-context-label">Position Size</span>
          <span class="trade-action-context-value">
            <span class="trade-action-balance-amount">${context.currentSize.toFixed(4)}</span>
            <span class="trade-action-balance-unit">SOL</span>
          </span>
        </div>
      `);
    }

    if (context.hasPosition && context.holdings) {
      rows.push(`
        <div class="trade-action-context-item">
          <span class="trade-action-context-label">Holdings</span>
          <span class="trade-action-context-value">${Utils.escapeHtml(this._formatHoldings(context))} tokens</span>
        </div>
      `);
    }

    this.contextEl.innerHTML = `
      ${this._renderIdentity(symbol, context)}
      <div class="trade-action-context-grid">${rows.join("")}</div>
    `;
  }

  /**
   * Holdings arrive as RAW on-chain units and must be scaled by the token's decimals
   * before display — printing them raw shows "1.2B tokens" for 1,200 tokens at 6
   * decimals. Display only: the percentage-based quote and execution work off the
   * real on-chain balance server-side.
   */
  _formatHoldings(context) {
    const whole =
      context.decimals != null ? context.holdings / 10 ** context.decimals : context.holdings;
    return Utils.formatCompactNumber(whole, 2);
  }

  _buildPresets(action, context) {
    const config = ACTION_CONFIG[action];

    if (action === "buy" || action === "sell") {
      return config.presets;
    }

    if (action === "add") {
      /** @type {Array<Object>} */
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

      // Add presets are built from context, so unlike buy/sell none of them is marked
      // as the default — and `open()` preselects `[data-default="true"]`. With nothing
      // preselected the dialog showed no amount and fetched no quote, so the user
      // confirmed an invisible backend default. Preselect the first preset instead: the
      // amount is then on screen, quoted, before anything is committed.
      if (presets.length > 0 && !presets.some((p) => p.default)) {
        presets[0].default = true;
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

    // Keep slider in sync with the chosen preset
    this._syncSliderToValue(value);

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

    // Keep the slider in sync with typed value
    const typed = parseFloat(this.inputField.value);
    this._syncSliderToValue(Number.isFinite(typed) ? typed : 0);

    // Clear error and validate
    this._clearError();
    this._updateConfirmButton();

    // Fetch quote when input changes
    this._fetchQuoteDebounced();
  }

  // --- Amount controls (slider + MAX) -------------------------------------

  /** Configure the slider range/units for the current action + context. */
  _configureAmountControls(action, context) {
    if (!this.sliderEl) return;
    if (action === "sell") {
      // Percentage 0–100
      this._sliderMax = 100;
      this._sliderStep = 1;
      this._sliderUnit = "%";
      this.sliderEl.min = "0";
      this.sliderEl.max = "100";
      this.sliderEl.step = "1";
      this.maxBtn.textContent = "100%";
      this.sliderRow.dataset.visible = "true";
    } else {
      // Buy/add: SOL from 0 to available balance (fall back to a sane default)
      const bal = typeof context.balance === "number" && context.balance > 0 ? context.balance : 1;
      // Leave a tiny headroom for fees when using the balance as the ceiling.
      const max = action === "buy" ? Math.max(bal - 0.002, 0.001) : Math.max(bal, 0.001);
      this._sliderMax = max;
      this._sliderStep = max > 0.5 ? 0.005 : 0.001;
      this._sliderUnit = "SOL";
      this.sliderEl.min = "0";
      this.sliderEl.max = String(max);
      this.sliderEl.step = String(this._sliderStep);
      this.maxBtn.textContent = "MAX";
      // Only show the slider when we know the balance (otherwise it's misleading).
      this.sliderRow.dataset.visible = typeof context.balance === "number" ? "true" : "false";
    }
    this.sliderEl.value = "0";

    // Build magnetic snap points + visible ticks (presets/round values + ends)
    this._buildSnapPoints(action, context);
    this._renderSliderTicks();

    this._updateSliderFill();
    this._updateSliderReadout(0);
  }

  /** Snap points the slider gently magnetizes to (0, presets, quarters, max). */
  _buildSnapPoints(action, context) {
    const max = this._sliderMax || 0;
    const set = new Set([0, max]);
    if (action === "sell") {
      [25, 50, 75].forEach((p) => set.add(p));
    } else {
      // Config/preset amounts that fit under the balance ceiling …
      const presets = ACTION_CONFIG[action]?.presets || [];
      presets.forEach((p) => {
        if (typeof p.value === "number" && p.value > 0 && p.value <= max) set.add(p.value);
      });
      if (Array.isArray(context.entrySizes)) {
        context.entrySizes.forEach((v) => {
          if (typeof v === "number" && v > 0 && v <= max) set.add(v);
        });
      }
      // … plus quarter steps of the balance for a sense of proportion.
      [0.25, 0.5, 0.75].forEach((f) => set.add(parseFloat((max * f).toFixed(6))));
    }
    this._snapPoints = Array.from(set)
      .filter((v) => v >= 0 && v <= max)
      .sort((a, b) => a - b);
  }

  /** Render tick marks on the track at each snap point. */
  _renderSliderTicks() {
    if (!this.sliderTicksEl) return;
    const max = this._sliderMax || 0;
    if (max <= 0 || !this._snapPoints?.length) {
      this.sliderTicksEl.innerHTML = "";
      return;
    }
    this.sliderTicksEl.innerHTML = this._snapPoints
      .map((v) => {
        const pct = Math.min(Math.max((v / max) * 100, 0), 100);
        const strong = v === 0 || v === max ? " is-strong" : "";
        return `<span class="trade-action-slider-tick${strong}" style="left:${pct}%"></span>`;
      })
      .join("");
  }

  /** Apply a small magnetic pull toward the nearest snap point. */
  _magnetize(value) {
    const max = this._sliderMax || 0;
    if (max <= 0 || !this._snapPoints?.length) return value;
    const threshold = max * 0.03; // ~3% of the range
    let best = value;
    let bestDist = Infinity;
    for (const p of this._snapPoints) {
      const d = Math.abs(p - value);
      if (d < bestDist) {
        bestDist = d;
        best = p;
      }
    }
    return bestDist <= threshold ? best : value;
  }

  _handleSliderInput() {
    const raw = parseFloat(this.sliderEl.value);
    const value = this._magnetize(Number.isFinite(raw) ? raw : 0);
    // Reflect the magnetized value back onto the slider thumb.
    if (this.sliderEl.value !== String(value)) {
      this.sliderEl.value = String(value);
    }
    // Slider acts like typing: clear presets, fill input, fetch.
    this.presetsContainer.querySelectorAll(".trade-action-preset-btn").forEach((b) => {
      b.classList.remove("selected");
    });
    this._selectedPreset = null;
    this._settingInputProgrammatically = true;
    this.inputField.value = value > 0 ? this._trimNumber(value) : "";
    this._settingInputProgrammatically = false;
    this._updateSliderFill();
    this._updateSliderReadout(value);
    this._clearError();
    this._updateConfirmButton();
    if (value > 0) this._fetchQuoteDebounced();
  }

  _handleMaxClick() {
    const max = this._sliderMax || 0;
    if (max <= 0) return;
    this.presetsContainer.querySelectorAll(".trade-action-preset-btn").forEach((b) => {
      b.classList.remove("selected");
    });
    this._selectedPreset = null;
    this._settingInputProgrammatically = true;
    this.inputField.value = this._trimNumber(max);
    this._settingInputProgrammatically = false;
    if (this.sliderEl) this.sliderEl.value = String(max);
    this._updateSliderFill();
    this._updateSliderReadout(max);
    this._clearError();
    this._updateConfirmButton();
    this._fetchQuoteDebounced();
  }

  /** Move the slider to match a value set via input/preset (no event loop). */
  _syncSliderToValue(value) {
    if (!this.sliderEl) return;
    const v = Number.isFinite(value) ? Math.min(Math.max(value, 0), this._sliderMax || 100) : 0;
    this.sliderEl.value = String(v);
    this._updateSliderFill();
    this._updateSliderReadout(value);
  }

  _updateSliderFill() {
    if (!this.sliderEl) return;
    const max = parseFloat(this.sliderEl.max) || 1;
    const val = parseFloat(this.sliderEl.value) || 0;
    const pct = Math.min(Math.max((val / max) * 100, 0), 100);
    this.sliderEl.style.setProperty("--slider-fill", `${pct}%`);
  }

  _updateSliderReadout(value) {
    if (!this.sliderReadoutEl) return;
    if (!value || value <= 0) {
      this.sliderReadoutEl.textContent = "";
      return;
    }
    if (this._sliderUnit === "%") {
      this.sliderReadoutEl.textContent = `${Math.round(value)}%`;
    } else {
      this.sliderReadoutEl.textContent = `${this._trimNumber(value)} SOL`;
    }
  }

  _trimNumber(n) {
    // Compact numeric string without trailing zeros (e.g. 0.0500 -> 0.05).
    return parseFloat(Number(n).toFixed(6)).toString();
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
      // The floor is 1, not 0: the executor clamps the exit percentage to [1, 100], so
      // anything below 1 would be silently rounded UP and sell more than asked.
      if (value < 1 || value > 100) {
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

    // Warn when the quoted PRICE IMPACT exceeds the slippage tolerance actually in
    // force for this trade (the override, else the configured default). This used to
    // be a hardcoded 5%, which both nagged users who had deliberately set a high
    // tolerance and stayed silent for those running a tight one.
    const tolerance = this._slippagePct ?? this._configuredSlippage ?? 5;
    if (this._quoteData && this._quoteData.price_impact_pct > tolerance) {
      const confirmed = await this._showSlippageWarning(
        this._quoteData.price_impact_pct,
        tolerance
      );
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
      // Buy opens a position: carry typed ownership (checked means user-only).
      if (this.currentAction === "buy" && this.manageCheckboxEl) {
        result.management = this.manageCheckboxEl.checked ? "user_only" : "auto_trader";
      }
    }

    // Per-trade slippage override. Only sent when the user picked one — omitting it
    // is what tells the backend to use the configured slippage.
    if (this._slippagePct != null) {
      result.slippage_pct = this._slippagePct;
    }

    // Save to recent trades (for quick mode or any trade with mint)
    if (this.currentContext?.mint) {
      this._saveRecentTrade(this.currentContext.mint, this._currentSymbol);
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
   * @param {number} tolerance - Slippage tolerance in force (override, else config)
   * @returns {Promise<boolean>} True if user confirms, false if cancelled
   */
  _showSlippageWarning(impactPct, tolerance = 5) {
    return new Promise((resolve) => {
      const overlay = document.createElement("div");
      overlay.className = "trade-slippage-warning-overlay";
      overlay.innerHTML = `
        <div class="trade-slippage-warning">
          <div class="slippage-warning-icon"><i class="icon-triangle-alert"></i></div>
          <div class="slippage-warning-title">High Price Impact Warning</div>
          <div class="slippage-warning-text">
            This trade has a price impact of <strong>${impactPct.toFixed(2)}%</strong>,
            which exceeds your slippage tolerance of <strong>${tolerance}%</strong>. You may
            receive significantly less than expected.
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
      // /api/positions/:key/details returns the PositionDetailResponse directly
      // (no {success,data} envelope), and `position` flattens the summary fields
      // (id, mint, token_amount, ...) onto itself via #[serde(flatten)].
      const data = await res.json();
      const pos = data?.position;
      if (!pos) {
        return { ok: false, error: "Position not found - it may have been closed" };
      }

      const currentHoldings = pos.remaining_token_amount ?? pos.token_amount ?? 0;
      const expectedHoldings = this.currentContext.holdings || 0;

      // Allow 1% variance for rounding
      const variance = Math.abs(currentHoldings - expectedHoldings) / expectedHoldings;
      if (variance > 0.01 && expectedHoldings > 0) {
        // Both are RAW on-chain units — print them scaled, or the warning reads
        // "Expected 1200000000.00, now 1150000000.00" for 1,200 vs 1,150 tokens.
        const decimals = this.currentContext.decimals;
        const scale = (raw) =>
          Utils.formatCompactNumber(decimals != null ? raw / 10 ** decimals : raw, 2);

        return {
          ok: false,
          error: `Token balance changed. Expected ${scale(expectedHoldings)}, now ${scale(currentHoldings)}. Please refresh.`,
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
}

// Expose ACTION_CONFIG as a static property for mixins to access
TradeActionDialog.ACTION_CONFIG = ACTION_CONFIG;

// Apply mixins to add quote manager and quick trade functionality
applyQuoteManagerMixin(TradeActionDialog);
applyQuickTradeMixin(TradeActionDialog);
