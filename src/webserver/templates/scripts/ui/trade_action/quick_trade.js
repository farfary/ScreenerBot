import * as Utils from "../../core/utils.js";

/**
 * Quick Trade Mode Mixin for TradeActionDialog
 * Adds quick buy/sell functionality with token search and mint input
 */
export function applyQuickTradeMixin(TradeActionDialog) {
  const proto = TradeActionDialog.prototype;

  /**
   * Validate if a string is a valid Solana mint address
   * @param {string} value - The string to validate
   * @returns {boolean}
   */
  proto._isValidMint = function (value) {
    return /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(value);
  };

  /**
   * Render the quick trade mint input step
   * @param {string} action - 'buy' or 'sell'
   */
  proto._renderQuickMintStep = function (action) {
    // Get ACTION_CONFIG from the class
    const ACTION_CONFIG = this.constructor.ACTION_CONFIG || {
      buy: { icon: "shopping-cart", colorClass: "action-buy" },
      sell: { icon: "trending-down", colorClass: "action-sell" },
    };
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

    // Reset search state
    this._hideSearchResults();
    this._clearSearchDebounce();

    // Render recent trades
    this._renderRecentTrades();
  };

  /**
   * Handle paste button click in quick trade mode
   */
  proto._handleQuickPaste = async function () {
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
  };

  /**
   * Handle mint input changes in quick trade mode
   * Detects if input is a mint address or a search query
   */
  proto._handleQuickMintInput = function () {
    const value = this._quickMintInputEl.value.trim();

    // Hide error when typing
    this._quickErrorEl.setAttribute("data-visible", "false");
    this._clearSearchDebounce();

    if (!value) {
      this._quickContinueBtnEl.disabled = true;
      this._quickTokenPreviewEl.setAttribute("data-visible", "false");
      this._hideSearchResults();
      this._fetchedTokenData = null;
      return;
    }

    // Check if it looks like a mint address (base58, 32-44 chars)
    if (this._isValidMint(value)) {
      // It's a mint address - hide search, fetch token info directly
      this._hideSearchResults();
      this._fetchQuickTokenInfo(value);
      return;
    }

    // It's a search query - hide token preview, show search results
    this._quickTokenPreviewEl.setAttribute("data-visible", "false");
    this._quickContinueBtnEl.disabled = true;
    this._fetchedTokenData = null;

    // Only search if query is at least 2 characters
    if (value.length >= 2) {
      this._searchDebounceTimer = setTimeout(() => {
        this._searchTokens(value);
      }, 300);
    } else {
      this._hideSearchResults();
    }
  };

  /**
   * Clear search debounce timer
   */
  proto._clearSearchDebounce = function () {
    if (this._searchDebounceTimer) {
      clearTimeout(this._searchDebounceTimer);
      this._searchDebounceTimer = null;
    }
  };

  /**
   * Search tokens by symbol/name
   * @param {string} query - Search query
   */
  proto._searchTokens = async function (query) {
    if (this._isSearching) return;
    this._isSearching = true;

    try {
      const response = await fetch(`/api/tokens/search?q=${encodeURIComponent(query)}&limit=5`);

      if (!this._isOpen || this._quickMintInputEl.value.trim() !== query) {
        return; // Dialog closed or query changed
      }

      if (!response.ok) {
        this._hideSearchResults();
        return;
      }

      const data = await response.json();

      if (!data.success || !data.data || data.data.length === 0) {
        this._hideSearchResults();
        return;
      }

      this._searchResults = data.data;
      this._searchSelectedIndex = -1;
      this._renderSearchResults();
    } catch {
      this._hideSearchResults();
    } finally {
      this._isSearching = false;
    }
  };

  /**
   * Render search results dropdown
   */
  proto._renderSearchResults = function () {
    if (!this._searchDropdownEl || this._searchResults.length === 0) {
      this._hideSearchResults();
      return;
    }

    this._searchDropdownEl.innerHTML = this._searchResults
      .map((token, index) => {
        const mintShort = token.mint ? `${token.mint.slice(0, 4)}...${token.mint.slice(-4)}` : "";
        return `
          <div class="search-result-item" data-index="${index}" data-mint="${Utils.escapeHtml(token.mint)}">
            <span class="search-result-symbol token-symbol-type">${Utils.escapeHtml(token.symbol || "???")} </span>
            <span class="search-result-name token-name-type">${Utils.escapeHtml(token.name || "Unknown")}</span>
            <span class="search-result-mint">${mintShort}</span>
          </div>
        `;
      })
      .join("");

    this._searchDropdownEl.setAttribute("data-visible", "true");

    // Attach click listeners
    this._searchDropdownEl.querySelectorAll(".search-result-item").forEach((item) => {
      item.addEventListener("click", (e) => {
        const mint = e.currentTarget.getAttribute("data-mint");
        if (mint) {
          this._selectSearchResult(mint);
        }
      });
    });
  };

  /**
   * Hide search results dropdown
   */
  proto._hideSearchResults = function () {
    if (this._searchDropdownEl) {
      this._searchDropdownEl.setAttribute("data-visible", "false");
      this._searchDropdownEl.innerHTML = "";
    }
    this._searchResults = [];
    this._searchSelectedIndex = -1;
  };

  /**
   * Select a search result and populate the mint input
   * @param {string} mint - The mint address to select
   */
  proto._selectSearchResult = function (mint) {
    this._quickMintInputEl.value = mint;
    this._hideSearchResults();
    this._fetchQuickTokenInfo(mint);
  };

  /**
   * Handle keyboard navigation in search results
   * @param {KeyboardEvent} e
   */
  proto._handleSearchKeyDown = function (e) {
    if (!this._searchDropdownEl || this._searchResults.length === 0) {
      return;
    }

    const isVisible = this._searchDropdownEl.getAttribute("data-visible") === "true";
    if (!isVisible) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      this._searchSelectedIndex = Math.min(
        this._searchSelectedIndex + 1,
        this._searchResults.length - 1
      );
      this._highlightSearchResult();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      this._searchSelectedIndex = Math.max(this._searchSelectedIndex - 1, 0);
      this._highlightSearchResult();
    } else if (e.key === "Enter" && this._searchSelectedIndex >= 0) {
      e.preventDefault();
      e.stopPropagation();
      const selected = this._searchResults[this._searchSelectedIndex];
      if (selected?.mint) {
        this._selectSearchResult(selected.mint);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      this._hideSearchResults();
    }
  };

  /**
   * Highlight the currently selected search result
   */
  proto._highlightSearchResult = function () {
    const items = this._searchDropdownEl.querySelectorAll(".search-result-item");
    items.forEach((item, idx) => {
      item.classList.toggle("selected", idx === this._searchSelectedIndex);
    });
  };

  /**
   * Fetch token info for quick trade mode
   * @param {string} mint - The mint address
   */
  proto._fetchQuickTokenInfo = async function (mint) {
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

      // /api/tokens/{mint} returns the TokenDetailResponse RAW — success_response is
      // a plain Json(...) with no {success,data} envelope. This used to test
      // `data.success && data.data`, which are both always undefined, so quick trade
      // threw "Token not found in database" for EVERY mint and the mode never worked.
      const token = await response.json();

      if (!token?.mint) {
        throw new Error("Token not found in database");
      }

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
  };

  /**
   * Handle continue button click in quick trade mode
   */
  proto._handleQuickContinue = async function () {
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

        // /details returns PositionDetailResponse directly; `position` flattens the
        // summary fields onto itself (no {success,data} envelope).
        const pos = data?.position;
        if (!pos) {
          this._quickContinueBtnEl.classList.remove("loading");
          this._showQuickError("No position found for this token");
          return;
        }

        const holdings = pos.remaining_token_amount ?? pos.token_amount ?? 0;

        if (holdings <= 0) {
          this._quickContinueBtnEl.classList.remove("loading");
          this._showQuickError("Position has no remaining tokens");
          return;
        }

        // Update context with the position. `hasPosition` + decimals are what make
        // the dialog render the Held badge and scale holdings to whole tokens.
        this.currentContext.holdings = holdings;
        this.currentContext.mint = mint;
        this.currentContext.hasPosition = true;
        this.currentContext.decimals = pos.token_decimals ?? null;
        this.currentContext.currentSize = pos.total_size_sol ?? pos.entry_size_sol ?? null;

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

    // Identity for the dialog's token strip, so quick mode shows the same
    // logo/name/symbol header as every other entry point.
    this.currentContext.symbol = this._currentSymbol;
    this.currentContext.name = token.name || null;
    this.currentContext.logo = token.logo_url || null;
    this.currentContext.decimals = token.decimals ?? this.currentContext.decimals ?? null;

    // Transition to trade step
    this._transitionToTradeStep();
  };

  /**
   * Transition from mint input step to trade step with animation
   */
  proto._transitionToTradeStep = function () {
    this._quickStep = "trade";

    // Get ACTION_CONFIG from the class
    const ACTION_CONFIG = this.constructor.ACTION_CONFIG || {
      buy: { title: "Buy Token", subtitle: "Enter amount in SOL" },
      sell: { title: "Sell Position", subtitle: "Select sell percentage" },
    };

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
  };

  /**
   * Show error message in quick trade mode
   * @param {string} message - The error message
   */
  proto._showQuickError = function (message) {
    this._quickErrorTextEl.textContent = message;
    this._quickErrorEl.setAttribute("data-visible", "true");
    this._quickContinueBtnEl.disabled = true;
  };

  /**
   * Inject CSS styles for quick trade mode
   */
  proto._injectQuickTradeStyles = function () {
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
      
      /* Quick Trade Search Results */
      .quick-trade-search-results {
        display: none;
        flex-direction: column;
        background: var(--color-surface);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        max-height: 200px;
        overflow-y: auto;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
      }
      .quick-trade-search-results[data-visible="true"] {
        display: flex;
      }
      
      .search-result-item {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1rem;
        cursor: pointer;
        transition: background-color 0.1s;
        border-bottom: 1px solid var(--color-border);
      }
      .search-result-item:last-child {
        border-bottom: none;
      }
      .search-result-item:hover,
      .search-result-item.selected {
        background: var(--color-surface-hover);
      }
      .search-result-item.selected {
        background: rgba(99, 102, 241, 0.15);
      }
      
      .search-result-symbol {
        font-weight: 600;
        font-size: 0.9375rem;
        color: var(--color-text-primary);
        min-width: 60px;
      }
      
      .search-result-name {
        flex: 1;
        font-size: 0.8125rem;
        color: var(--color-text-secondary);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      
      .search-result-mint {
        font-family: var(--font-mono);
        font-size: 0.75rem;
        color: var(--color-text-muted);
        flex-shrink: 0;
      }
      
      /* Quick Trade Recent Tokens */
      .quick-trade-recent {
        display: none;
        align-items: center;
        gap: 0.5rem;
        flex-wrap: wrap;
        margin-top: 0.25rem;
      }
      .quick-trade-recent[data-visible="true"] {
        display: flex;
      }
      .quick-trade-recent-label {
        font-size: 0.75rem;
        color: var(--color-text-muted);
        text-transform: uppercase;
        letter-spacing: 0.04em;
        flex-shrink: 0;
      }
      .quick-trade-recent-list {
        display: flex;
        gap: 0.375rem;
        flex-wrap: wrap;
      }
      .quick-trade-recent-chip {
        display: inline-flex;
        align-items: center;
        padding: 0.25rem 0.625rem;
        font-family: var(--font-mono);
        font-size: 0.75rem;
        font-weight: 500;
        color: var(--color-text-secondary);
        background: var(--color-surface);
        border: 1px solid var(--color-border);
        border-radius: 9999px;
        cursor: pointer;
        transition: all 0.15s;
        max-width: 80px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .quick-trade-recent-chip:hover {
        background: var(--color-surface-hover);
        border-color: var(--color-primary);
        color: var(--color-text-primary);
      }
      .quick-trade-recent-chip:active {
        transform: scale(0.95);
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
  };
}
