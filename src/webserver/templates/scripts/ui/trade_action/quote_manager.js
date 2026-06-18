import * as Utils from "../../core/utils.js";

/**
 * Quote Manager Mixin for TradeActionDialog
 * Handles recent trades tracking and quote preview functionality
 */
export function applyQuoteManagerMixin(TradeActionDialog) {
  const proto = TradeActionDialog.prototype;

  // ============================================
  // Recent Trades Methods
  // ============================================

  /**
   * Load recent trades from localStorage
   * @returns {Array<{mint: string, symbol: string, timestamp: number}>}
   */
  proto._loadRecentTrades = function () {
    try {
      const stored = localStorage.getItem("screenerbot_recent_trades");
      return stored ? JSON.parse(stored) : [];
    } catch {
      return [];
    }
  };

  /**
   * Save a recent trade to localStorage
   * @param {string} mint - Token mint address
   * @param {string} symbol - Token symbol
   */
  proto._saveRecentTrade = function (mint, symbol) {
    if (!mint) return;
    try {
      let recent = this._loadRecentTrades();
      // Remove if already exists
      recent = recent.filter((r) => r.mint !== mint);
      // Add to front
      recent.unshift({ mint, symbol: symbol || "Unknown", timestamp: Date.now() });
      // Keep max 10
      recent = recent.slice(0, 10);
      localStorage.setItem("screenerbot_recent_trades", JSON.stringify(recent));
    } catch {
      // Ignore localStorage errors
    }
  };

  /**
   * Render recent trades chips in quick trade mint step
   */
  proto._renderRecentTrades = function () {
    const container = this._quickMintStepEl?.querySelector(".quick-trade-recent-list");
    if (!container) return;

    const recent = this._loadRecentTrades();
    if (recent.length === 0) {
      container.closest(".quick-trade-recent")?.setAttribute("data-visible", "false");
      return;
    }

    container.closest(".quick-trade-recent")?.setAttribute("data-visible", "true");
    container.innerHTML = recent
      .map(
        (r) => `
        <button type="button" class="quick-trade-recent-chip" data-mint="${Utils.escapeHtml(r.mint)}" title="${Utils.escapeHtml(r.mint)}">
          ${Utils.escapeHtml(r.symbol)}
        </button>
      `
      )
      .join("");

    // Attach click listeners
    container.querySelectorAll(".quick-trade-recent-chip").forEach((chip) => {
      chip.addEventListener("click", (e) => {
        const mint = e.currentTarget.getAttribute("data-mint");
        if (mint) {
          this._quickMintInputEl.value = mint;
          this._handleQuickMintInput();
        }
      });
    });
  };

  // ============================================
  // Quote Preview Methods
  // ============================================

  /**
   * Fetch quote preview from API
   */
  proto._fetchQuote = async function () {
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

      // The /api/trader/quote endpoint returns a FLAT object (success + quote
      // fields at the top level), not a {data:{...}} wrapper. _renderQuote and
      // the confirm-path slippage check both read these flat fields directly.
      if (data.success) {
        this._quoteData = data;
        this._quoteError = null;
        this._quoteTimestamp = Date.now();
        this._renderQuote(data);
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
  };

  /**
   * Render quote data in the UI
   * @param {Object} quote - Quote data from API
   */
  proto._renderQuote = function (quote) {
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
  };

  /**
   * Set quote section state (idle, loading, loaded, error)
   * @param {string} state - One of: "idle", "loading", "loaded", "error"
   */
  proto._setQuoteState = function (state) {
    if (this.quoteSection) {
      this.quoteSection.dataset.state = state;
    }
  };

  /**
   * Get currently selected amount (from preset or input)
   * @returns {number|null}
   */
  proto._getSelectedAmount = function () {
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
  };

  /**
   * Start quote refresh timer to update age and auto-refresh
   */
  proto._startQuoteRefreshTimer = function () {
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
  };

  /**
   * Stop quote refresh timer
   */
  proto._stopQuoteRefreshTimer = function () {
    if (this._quoteRefreshTimer) {
      clearInterval(this._quoteRefreshTimer);
      this._quoteRefreshTimer = null;
    }
  };

  /**
   * Handle manual quote refresh button click
   * @param {Event} e - Click event
   */
  proto._handleQuoteRefresh = function (e) {
    e.preventDefault();
    e.stopPropagation();
    this._fetchQuote();
  };
}
