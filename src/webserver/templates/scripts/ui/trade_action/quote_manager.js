import * as Utils from "../../core/utils.js";

/**
 * Quote Manager Mixin for TradeActionDialog
 * Handles recent trades tracking and quote preview functionality
 */
export function applyQuoteManagerMixin(TradeActionDialog) {
  const proto = TradeActionDialog.prototype;

  // Single source of truth for the quote auto-refresh cadence. The countdown
  // shown in the UI and the actual refresh both use this, so they always agree.
  const QUOTE_REFRESH_SECS = 15;

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
  proto._fetchQuote = async function (opts = {}) {
    if (!this._isOpen || !this.currentContext?.mint) {
      return;
    }

    const amount = this._getSelectedAmount();
    if (!amount || amount <= 0) {
      this._setQuoteState("idle");
      return;
    }

    // Silent refresh: when we already have a quote on screen (auto-refresh,
    // manual refresh, or an amount tweak), update values in place with a subtle
    // pulse instead of blanking the panel and showing the big spinner. The full
    // loading state is only used for the very first fetch.
    const silent = opts.silent === true || !!this._quoteData;
    if (silent) {
      this._setRefreshing(true);
    } else {
      this._setQuoteState("loading");
      this._quoteData = null;
    }
    this._quoteError = null;

    const direction = this.currentAction === "sell" ? "sell" : "buy";

    // Price the quote at the slippage this trade will actually execute with, so the
    // preview the user confirms is the one they get. Omitted when on "Auto" — the
    // backend then applies the configured default itself.
    const slippageParam =
      this._slippagePct != null ? `&slippage_pct=${this._slippagePct}` : "";

    try {
      // Build URL based on direction
      let url;
      if (direction === "sell") {
        // For sell, amount is the percentage of holdings. Send the percentage and
        // let the backend apply it to the REAL on-chain balance — the stale DB
        // holdings (raw units) must never drive the quoted amount.
        url = `/api/trader/quote?mint=${encodeURIComponent(this.currentContext.mint)}&percentage=${amount}&direction=sell${slippageParam}`;
      } else {
        // For buy/add, amount is SOL
        url = `/api/trader/quote?mint=${encodeURIComponent(this.currentContext.mint)}&amount_sol=${amount}&direction=buy${slippageParam}`;
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
        this._setRefreshing(false);
        this._setQuoteState("loaded");
        this._pulseQuote();
        this._startQuoteRefreshTimer();
      } else {
        // Carry the backend's friendly title (message) + actionable hint
        // (details) so the error panel can explain WHY the quote failed.
        const e = new Error(data.error?.message || "Couldn't fetch a quote");
        e.detail = data.error?.details || "";
        e.code = data.error?.code || "";
        throw e;
      }
    } catch (err) {
      if (!this._isOpen) return;
      this._setRefreshing(false);
      // On a silent refresh failure, keep the last good quote on screen instead
      // of flipping to an error panel — the next tick will retry.
      if (silent && this._quoteData) {
        return;
      }
      this._quoteError = err.message;
      this._quoteData = null;
      this._renderQuoteError(err);
      this._setQuoteState("error");
    }
  };

  /**
   * Render the quote error panel: a friendly title plus an optional actionable
   * detail line. Network/parse failures (no structured backend error) fall back
   * to a generic-but-clear message.
   * @param {Error & {detail?: string, code?: string}} err
   */
  proto._renderQuoteError = function (err) {
    const title =
      err?.message && err.message !== "Failed to fetch"
        ? err.message
        : "Couldn't fetch a quote — check your connection and try again";
    if (this.quoteErrorTextEl) this.quoteErrorTextEl.textContent = title;
    if (this.quoteErrorDetailEl) {
      const detail = err?.detail || "";
      this.quoteErrorDetailEl.textContent = detail;
      this.quoteErrorDetailEl.style.display = detail ? "" : "none";
    }
  };

  /**
   * Render quote data in the UI
   * @param {Object} quote - Quote data from API
   */
  proto._renderQuote = function (quote) {
    const isSell = this.currentAction === "sell";
    const outUnit = isSell ? "SOL" : "tokens";

    // Hero: output amount + unit price
    this.quoteOutputEl.textContent = `≈ ${quote.output_formatted}`;
    if (this.quoteUnitPriceEl) {
      const pp = quote.price_per_token_sol;
      this.quoteUnitPriceEl.textContent =
        typeof pp === "number" && pp > 0 ? `1 token ≈ ${trimSol(pp)} SOL` : "";
    }

    // Price impact with color
    const impactPct = (quote.price_impact_pct ?? 0).toFixed(2);
    this.quoteImpactEl.textContent = `${impactPct}%`;
    this.quoteImpactEl.className = "quote-value quote-impact";
    if (quote.price_impact_pct > 5) {
      this.quoteImpactEl.classList.add("impact-high");
    } else if (quote.price_impact_pct > 1) {
      this.quoteImpactEl.classList.add("impact-medium");
    } else {
      this.quoteImpactEl.classList.add("impact-low");
    }

    // Minimum received (after max slippage)
    if (this.quoteMinReceivedEl) {
      const slipFrac = (quote.slippage_bps || 0) / 10000;
      const minOut = (quote.output_amount ?? 0) * (1 - slipFrac);
      this.quoteMinReceivedEl.textContent = `≈ ${formatAmount(minOut, outUnit)}`;
    }

    // Fees
    this.quotePlatformFeeEl.textContent = `${quote.platform_fee_pct}% · ${trimSol(quote.platform_fee_sol)} SOL`;
    this.quoteNetworkFeeEl.textContent = `≈ ${trimSol(quote.network_fee_sol)} SOL`;

    // Slippage + provider + full route path
    this.quoteSlippageEl.textContent = `${(quote.slippage_bps / 100).toFixed(1)}%`;
    this.quoteRouteEl.textContent = quote.router || "Unknown";
    if (this.quoteRoutePathEl) {
      this.quoteRoutePathEl.textContent = quote.route || quote.router || "Direct";
    }

    // You pay (advanced)
    if (this.quoteYouPayEl) {
      this.quoteYouPayEl.textContent = isSell
        ? formatAmount(quote.input_amount, "tokens")
        : `${trimSol(quote.input_amount)} SOL`;
    }

    // Auto-refresh countdown (advanced) — kept live by the refresh timer.
    if (this.quoteExpiryEl) {
      this.quoteExpiryEl.textContent = `in ${QUOTE_REFRESH_SECS}s`;
    }
  };

  /** Compact SOL string without trailing zeros, never scientific notation. */
  function trimSol(n) {
    if (typeof n !== "number" || !isFinite(n)) return "0";
    if (n === 0) return "0";
    if (n < 0.000001) return n.toFixed(9).replace(/0+$/, "").replace(/\.$/, "");
    return parseFloat(n.toFixed(6)).toString();
  }

  /** Compact amount with a unit label, using K/M/B for large token counts. */
  function formatAmount(n, unit) {
    if (typeof n !== "number" || !isFinite(n)) return `0 ${unit}`;
    if (unit === "SOL") return `${trimSol(n)} ${unit}`;
    if (n >= 1e9) return `${(n / 1e9).toFixed(2)}B ${unit}`;
    if (n >= 1e6) return `${(n / 1e6).toFixed(2)}M ${unit}`;
    if (n >= 1e3) return `${(n / 1e3).toFixed(2)}K ${unit}`;
    return `${parseFloat(n.toFixed(4))} ${unit}`;
  }

  /**
   * Set quote section state (idle, loading, loaded, error)
   * @param {string} state - One of: "idle", "loading", "loaded", "error"
   */
  proto._setQuoteState = function (state) {
    if (this.quoteSection) {
      this.quoteSection.dataset.state = state;
    }
  };

  /** Toggle the subtle "refreshing" indicator (thin bar + spinning refresh icon)
   * without hiding the current quote content. */
  proto._setRefreshing = function (on) {
    if (this.quoteSection) {
      this.quoteSection.dataset.refreshing = on ? "true" : "false";
    }
  };

  /** Briefly pulse the quote content so a silent value update is noticeable. */
  proto._pulseQuote = function () {
    const el = this.quoteContentEl;
    if (!el) return;
    el.classList.remove("quote-pulse");
    // Force reflow so the animation can restart on consecutive refreshes.
    void el.offsetWidth;
    el.classList.add("quote-pulse");
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
        const remaining = Math.max(0, QUOTE_REFRESH_SECS - age);
        // Both the header badge and the Auto-refresh row count down to the same
        // moment the quote actually refreshes.
        if (this.quoteAgeEl) {
          this.quoteAgeEl.textContent = `${remaining}s`;
        }
        if (this.quoteExpiryEl) {
          this.quoteExpiryEl.textContent = remaining > 0 ? `in ${remaining}s` : "refreshing…";
        }
        if (age >= QUOTE_REFRESH_SECS) {
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
