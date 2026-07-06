/**
 * Token Details Dialog
 * Full-screen dialog showing comprehensive token information with multiple tabs
 */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { Poller } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "./hint_popover.js";
import { showImageLightbox } from "./image_lightbox.js";
import { renderOverviewTab, renderOverviewLeft } from "./token_details/overview_tab.js";
import { renderSecurityTab } from "./token_details/security_tab.js";
import {
  renderPoolsTab,
  initPoolsTabEvents,
  renderLinksTab,
  initLinksTabEvents,
} from "./token_details/pools_links_tab.js";
import { applyTradeActionsMixin } from "./token_details/trade_actions.js";
import { applyTransactionsTabMixin } from "./token_details/transactions_tab.js";
import { applyChartTabMixin, CHART_CANDLE_LIMIT } from "./token_details/chart_tab.js";
import { applyUtilitiesMixin } from "./token_details/utilities.js";
import { applyStateHandlingMixin } from "./token_details/state_handling.js";
import { applyPositionsTabMixin } from "./token_details/positions_tab.js";

// Data source status constants
const DATA_SOURCE_STATUS = {
  PENDING: "pending",
  LOADING: "loading",
  SUCCESS: "success",
  ERROR: "error",
  CACHED: "cached",
};

// How long after opening a token we suppress the "no data" row, so source
// failures are never shown while the initial fetch/refresh is still in flight.
const INITIAL_LOAD_GRACE_MS = 6000;

export class TokenDetailsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.onTradeComplete = options.onTradeComplete || (() => {});
    this.dialogEl = null;
    this.currentTab = "overview";
    this.tokenData = null;
    this.tabHandlers = new Map();
    this.refreshPoller = null;
    this.chartPoller = null;
    this.dataStatusPoller = null;
    this.isRefreshing = false;
    this.currentTimeframe = "5m";
    this.isOpening = false;
    this.tradeDialog = null;
    this.positionsData = null;
    this.walletBalance = 0;
    this.walletBalanceFetchedAt = 0;
    this.advancedChart = null;
    this.chartDataLoaded = false; // Track whether OHLCV data has been loaded
    this._focusTrap = null;
    // Data source status tracking
    this._dataSourceStatus = {
      token: DATA_SOURCE_STATUS.PENDING,
      dexscreener: DATA_SOURCE_STATUS.PENDING,
      rugcheck: DATA_SOURCE_STATUS.PENDING,
      ohlcv: DATA_SOURCE_STATUS.PENDING,
    };
    this._initialLoadComplete = false;
    this._retryCount = 0;
    this._maxRetries = 3;
    // Connection-drop tracking: "online" | "reconnecting" | "offline".
    // Transient poll failures after the initial load surface here (subtle chip)
    // instead of wiping content or flipping source dots to a hard error.
    this._connectionState = "online";
    this._consecutiveFailures = 0;
  }

  /**
   * Show dialog with token data
   * @param {Object} tokenData - Complete token data object (minimal - just mint required)
   */
  async show(tokenData) {
    if (!tokenData || !tokenData.mint) {
      console.error("Invalid token data provided to TokenDetailsDialog");
      return;
    }

    if (this.isOpening) {
      console.log("Dialog already opening, ignoring duplicate request");
      return;
    }

    if (this.dialogEl && this.tokenData && this.tokenData.mint !== tokenData.mint) {
      console.log("Closing existing dialog to open new token");
      this.close();
      await new Promise((resolve) => setTimeout(resolve, 350));
    }

    if (this.dialogEl && this.tokenData && this.tokenData.mint === tokenData.mint) {
      console.log("Dialog already open for this token, ignoring");
      return;
    }

    this.isOpening = true;

    try {
      this.tokenData = tokenData;
      // Use initial tokenData for immediate display (don't wait for API)
      // fullTokenData will be updated by polling when fresh data arrives
      this.fullTokenData = tokenData;
      // Reset chart data state for new token
      this.chartDataLoaded = false;
      this._chartEmptyCount = 0;
      this._chartPollBackedOff = false;
      // Grace window before the "no data" row may appear, so we never flash
      // source failures while the very first fetch/refresh is still in flight.
      this._issuesSettleAt = Date.now() + INITIAL_LOAD_GRACE_MS;

      // Initialize hints system before creating dialog
      await Hints.init();

      this._createDialog();
      this._attachEventHandlers();
      this._loadTabContent("overview");

      // Initialize hint triggers after content is loaded
      HintTrigger.initAll();

      requestAnimationFrame(() => {
        if (this.dialogEl) {
          this.dialogEl.classList.add("active");
          // Add ARIA attributes for accessibility
          const container = this.dialogEl.querySelector(".dialog-container");
          if (container) {
            container.setAttribute("role", "dialog");
            container.setAttribute("aria-modal", "true");
            container.setAttribute("aria-labelledby", "tdd-dialog-title");
          }
          // Activate focus trap
          this._focusTrap = createFocusTrap(this.dialogEl);
          this._focusTrap.activate();
        }
      });

      // Set this token as dashboard-active for priority data fetching (fire and forget)
      this._focusToken().catch(() => {
        // Silent - focus is best-effort
      });

      // Fetch the full token detail IMMEDIATELY (fire and forget). The row data
      // we opened with has no security fields (safety_score, risks, holders live
      // only on /api/tokens/{mint}), and the Poller below only fires after its
      // first interval — so without this, cached Rugcheck/chart data wouldn't
      // appear for ~5s and the security tab would sit on "fetching" even for
      // tokens that already passed filtering. This loads the cached detail now;
      // the refresh + poller then keep it fresh.
      this._fetchTokenData().catch(() => {
        // Errors are handled inside _fetchTokenData (retry/connection state)
      });

      // Trigger backend refresh endpoints in parallel (fire and forget)
      // These trigger high-priority data fetching on backend
      // Don't await - let them run in background while dialog shows
      this._triggerTokenRefresh().catch((err) => {
        console.warn("Token refresh failed:", err);
      });
      this._triggerOhlcvRefresh().catch(() => {
        // Silent - expected for new tokens without OHLCV
      });

      // Start polling after a short delay to give refresh time to start
      // The poller will fetch fresh data as it becomes available
      setTimeout(() => {
        if (this.dialogEl) {
          this._startPolling();
        }
      }, 500);
    } finally {
      this.isOpening = false;
    }
  }

  async _triggerTokenRefresh() {
    try {
      // Use requestManager with high priority for immediate refresh
      const response = await requestManager.fetch(`/api/tokens/${this.tokenData.mint}/refresh`, {
        method: "POST",
        priority: "high",
      });
      if (response.success !== false) {
        console.log("Token data refresh triggered:", response);
        return response;
      }
    } catch (error) {
      console.warn("Failed to trigger token refresh:", error);
    }
    return null;
  }

  async _triggerOhlcvRefresh() {
    try {
      // Use requestManager with high priority for immediate OHLCV refresh
      const response = await requestManager.fetch(
        `/api/tokens/${this.tokenData.mint}/ohlcv/refresh`,
        {
          method: "POST",
          priority: "high",
        }
      );
      if (response.success !== false) {
        console.log("OHLCV data refresh triggered:", response);
        return response;
      }
    } catch {
      // Silently ignore - OHLCV may not be available for new tokens
    }
    return null;
  }

  /**
   * Set this token as dashboard-active for priority data fetching
   * Background batch updates will skip this token while it's focused
   */
  async _focusToken() {
    try {
      const response = await requestManager.fetch(`/api/tokens/${this.tokenData.mint}/focus`, {
        method: "POST",
        priority: "high",
      });
      if (response.success) {
        console.log("Token focused for priority updates:", response);
      }
      return response;
    } catch (error) {
      console.warn("Failed to focus token:", error);
    }
    return null;
  }

  /**
   * Clear dashboard focus when dialog closes
   * Resets OHLCV priority unless token has an open position
   */
  async _unfocusToken() {
    try {
      const response = await requestManager.fetch(`/api/tokens/${this.tokenData.mint}/unfocus`, {
        method: "POST",
        priority: "low",
      });
      if (response.success) {
        console.log("Token unfocused:", response);
      }
      return response;
    } catch {
      // Silent - unfocus is best-effort
    }
    return null;
  }

  async _fetchTokenData() {
    if (this.isRefreshing) return;
    this.isRefreshing = true;

    // Update status to loading on first fetch
    if (!this._initialLoadComplete) {
      this._updateDataSourceStatus("token", DATA_SOURCE_STATUS.LOADING);
    }

    try {
      // Use requestManager with high priority for token detail fetch
      const newData = await requestManager.fetch(`/api/tokens/${this.tokenData.mint}`, {
        priority: "high",
        // Uncached tokens can spend >10s trying external sources before the
        // backend returns a normal NOT_FOUND payload. Do not misclassify that
        // slow-but-valid path as a reconnect.
        timeout: this._initialLoadComplete ? 10000 : 25000,
      });

      if (newData) {
        const isInitialLoad = !this._initialLoadComplete;
        this.fullTokenData = newData;
        this._updateHeader(this.fullTokenData);
        this._initialLoadComplete = true;
        this._retryCount = 0;
        // Poll succeeded — clear any reconnect chip / failure streak.
        this._recordPollOutcome(true);

        // Update data source statuses based on what data we have
        this._updateDataSourceStatus("token", DATA_SOURCE_STATUS.SUCCESS);
        this._updateDataSourceFromToken(newData);

        if (isInitialLoad) {
          // First data arrived: populate whatever tab is active (any tab may have
          // been showing a waiting/error placeholder), so every tab recovers, not
          // just overview.
          this._loadTabContent(this.currentTab);
        } else if (this.currentTab === "overview") {
          this._refreshOverviewTab();
        } else if (this.currentTab === "security") {
          // Keep the security tab in sync. The loader is idempotent (it only
          // repaints when the produced HTML actually changes), so calling it on
          // every poll is free and never restarts the card entry animations while
          // we are still waiting on Rugcheck data.
          const content = this.dialogEl?.querySelector('[data-tab-content="security"]');
          if (content) {
            this._loadSecurityTab(content);
          }
        } else if (this.currentTab === "positions") {
          // Live position: re-fetch so PnL / current price update while watching.
          // The loader is idempotent (_renderHtmlIfChanged), so unchanged data
          // causes no repaint/flash. Throttle to ~3s because this poller actually
          // ticks at the global interval (~1s) and the position-details endpoint
          // is heavier than a token fetch — 3s is plenty "live" for PnL. Tab
          // switches and post-trade refreshes still load immediately (they call
          // _loadPositionsTab directly, bypassing this throttle).
          const now = Date.now();
          if (now - (this._positionsLastPoll || 0) >= 3000) {
            this._positionsLastPoll = now;
            const content = this.dialogEl?.querySelector('[data-tab-content="positions"]');
            if (content) {
              this._loadPositionsTab(content);
            }
          }
        }
      }
    } catch (error) {
      console.error("Error loading token details:", error);

      if (!this._initialLoadComplete) {
        // Initial load failed: retry with exponential backoff, then show a
        // friendly error state (with Retry) in the active tab.
        this._updateDataSourceStatus("token", DATA_SOURCE_STATUS.ERROR);
        if (this._retryCount < this._maxRetries) {
          this._retryCount++;
          const delay = 1000 * Math.pow(2, this._retryCount - 1); // 1s, 2s, 4s
          console.log(
            `Retrying token fetch (${this._retryCount}/${this._maxRetries}) in ${delay}ms`
          );
          setTimeout(() => {
            this.isRefreshing = false;
            this._fetchTokenData();
          }, delay);
          return;
        }
        this._recordPollOutcome(false);
        const content = this.dialogEl?.querySelector(`[data-tab-content="${this.currentTab}"]`);
        this._renderTabError(content, { title: "Couldn't load token data" });
      } else {
        // Already showing good data. Keep the last good content; only show the
        // connection chip if the global backend watcher has confirmed an outage.
        this._recordPollOutcome(false);
      }
    } finally {
      this.isRefreshing = false;
    }
  }

  /**
   * Update data source statuses based on token data fields
   */
  _updateDataSourceFromToken(token) {
    // Check DexScreener data
    if (token.market_cap || token.volume_24h || token.liquidity_usd) {
      this._updateDataSourceStatus("dexscreener", DATA_SOURCE_STATUS.SUCCESS);
    } else if (this._dataSourceStatus.dexscreener === DATA_SOURCE_STATUS.PENDING) {
      this._updateDataSourceStatus("dexscreener", DATA_SOURCE_STATUS.LOADING);
    }

    // Check Rugcheck data
    if (token.safety_score !== undefined && token.safety_score !== null) {
      this._updateDataSourceStatus("rugcheck", DATA_SOURCE_STATUS.SUCCESS);
    } else if (this._dataSourceStatus.rugcheck === DATA_SOURCE_STATUS.PENDING) {
      this._updateDataSourceStatus("rugcheck", DATA_SOURCE_STATUS.LOADING);
    }

    // Check OHLCV availability
    if (token.has_ohlcv) {
      this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.SUCCESS);
    } else if (this._dataSourceStatus.ohlcv === DATA_SOURCE_STATUS.PENDING) {
      this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.LOADING);
    }

    // Detailed per-source "no data / unavailable" row (backend-driven).
    this._renderSourceIssues(token.source_status);
  }

  /**
   * Render the thin row below the header that spells out which upstream sources
   * have no data (or are temporarily unavailable) for this token, so blanks are
   * explained instead of silent.
   *
   * Rules that keep it honest and quiet:
   * - Suppressed entirely during the initial-load grace window, so we never flash
   *   failures while the first fetch/refresh is still in flight.
   * - DexScreener and GeckoTerminal are one "market" concern: if either has data,
   *   neither is listed (the token IS priced — the other provider is just an
   *   unused alternative, not a real gap).
   * - The chart is only flagged once its poll has actually settled on "empty"
   *   (backed off), not while candles may still be loading.
   * @param {Array<{source:string,label:string,state:string,message:string}>} sourceStatus
   */
  _renderSourceIssues(sourceStatus) {
    const row = this.dialogEl?.querySelector("#sourceIssuesRow");
    if (!row) return;

    const list = Array.isArray(sourceStatus) ? sourceStatus : [];
    const by = Object.fromEntries(list.map((s) => [s.source, s]));
    const isOk = (id) => by[id]?.state === "ok";

    // Hold everything back until the initial load has had a chance to settle.
    const settled = Date.now() >= (this._issuesSettleAt || 0);

    const issues = [];
    const marketOk = isOk("dexscreener") || isOk("geckoterminal");
    if (settled) {
      // Market: only when NEITHER provider has data (genuine no-market state).
      if (!marketOk) {
        if (by.dexscreener) issues.push(by.dexscreener);
        if (by.geckoterminal) issues.push(by.geckoterminal);
      }
      // Rugcheck: flag once settled and still absent.
      if (!isOk("rugcheck") && by.rugcheck) issues.push(by.rugcheck);
    }
    // Chart: independent of the grace timer — only once the poll has confirmed
    // there is no OHLCV (backed off), so it never shows mid-load.
    if (!isOk("ohlcv") && this._chartPollBackedOff && by.ohlcv) {
      issues.push(by.ohlcv);
    }

    if (issues.length === 0) {
      row.hidden = true;
      row.innerHTML = "";
      return;
    }

    // "All failed" = no market, no security, no chart — the full blackout case.
    const allFailed = !marketOk && !isOk("rugcheck") && !isOk("ohlcv");
    const icon = (state) =>
      state === "unavailable" ? "icon-circle-alert" : "icon-circle-x";

    const chips = issues
      .map(
        (s) =>
          `<span class="source-issue source-issue--${this._escapeHtml(s.state)}">
             <i class="${icon(s.state)}" aria-hidden="true"></i>
             <span class="source-issue-text">${this._escapeHtml(s.message)}</span>
           </span>`
      )
      .join("");

    const lead = allFailed
      ? '<span class="source-issues-lead">No data available</span>'
      : "";

    row.innerHTML = `${lead}${chips}`;
    row.hidden = false;
    row.classList.toggle("source-issues-row--all", allFailed);
  }

  /**
   * Update data source status and UI indicator
   * @param {string} source - 'token' | 'dexscreener' | 'rugcheck' | 'ohlcv'
   * @param {string} status - DATA_SOURCE_STATUS constant
   */
  _updateDataSourceStatus(source, status) {
    this._dataSourceStatus[source] = status;

    // Update UI indicator
    const indicator = this.dialogEl?.querySelector(`.source-status[data-source="${source}"]`);
    if (indicator) {
      const icon = indicator.querySelector(".status-icon");
      if (icon) {
        icon.className = `status-icon ${status}`;
      }
    }

    // Check if all sources are done loading
    const allDone = Object.values(this._dataSourceStatus).every(
      (s) =>
        s === DATA_SOURCE_STATUS.SUCCESS ||
        s === DATA_SOURCE_STATUS.ERROR ||
        s === DATA_SOURCE_STATUS.CACHED
    );

    // Hide status bar when all sources are loaded successfully
    if (allDone) {
      const statusBar = this.dialogEl?.querySelector(".data-sources-status");
      if (statusBar) {
        statusBar.classList.add("all-loaded");
      }
    }
  }

  _startPolling() {
    this._stopPolling();
    // Use 5 second polling interval (reduced from 1 second)
    this.refreshPoller = new Poller(
      () => {
        this._fetchTokenData();
      },
      { label: "TokenRefresh", interval: 5000 }
    );
    this.refreshPoller.start();
  }

  _stopPolling() {
    if (this.refreshPoller) {
      this.refreshPoller.stop();
      this.refreshPoller.cleanup();
      this.refreshPoller = null;
    }
  }

  _startChartPolling() {
    this._stopChartPolling();
    // 10s once data is loaded; 3s while actively waiting; but back off to 15s once
    // it's clear the token simply has no OHLCV yet, so we don't hammer the OHLCV
    // endpoint every 3s for the whole time the dialog stays open on a dead chart.
    let interval = 3000;
    if (this.chartDataLoaded) {
      interval = 10000;
    } else if (this._chartPollBackedOff) {
      interval = 15000;
    }
    this.chartPoller = new Poller(
      () => {
        this._refreshChartData();
      },
      { label: "ChartRefresh", interval }
    );
    this.chartPoller.start();

    // Keep the DATA popover live on a steady, independent cadence — the chart
    // poll can back off to 15s on a dead chart, but the data-status indicator
    // (candle counts, last-checked / last-new-candle times) should still tick so
    // it reflects new candles / checks promptly while the dialog is open.
    if (this.dataStatusPoller) {
      this.dataStatusPoller.stop();
      this.dataStatusPoller.cleanup();
    }
    this.dataStatusPoller = new Poller(
      () => {
        if (this.currentTab === "overview" && this.tokenData?.mint) {
          this._updateDataIndicator(this.tokenData.mint);
        }
      },
      { label: "DataStatus", interval: 5000 }
    );
    this.dataStatusPoller.start();
  }

  _stopChartPolling() {
    if (this.chartPoller) {
      this.chartPoller.stop();
      this.chartPoller.cleanup();
      this.chartPoller = null;
    }
    if (this.dataStatusPoller) {
      this.dataStatusPoller.stop();
      this.dataStatusPoller.cleanup();
      this.dataStatusPoller = null;
    }
  }

  async _refreshChartData() {
    if (!this.advancedChart || !this.tokenData || this.currentTab !== "overview") {
      return;
    }

    // Keep the data-status indicator in sync each poll (cheap, fire-and-forget).
    this._updateDataIndicator(this.tokenData.mint);

    const loadingOverlay = this.dialogEl?.querySelector("#chartLoadingOverlay");
    const loadingText = loadingOverlay?.querySelector(".chart-loading-text");
    const wasDataLoaded = this.chartDataLoaded;
    // Capture the identity this poll is fetching for; if the user switches token
    // or timeframe during the await, a late response must not paint stale candles.
    const pollMint = this.tokenData.mint;
    const pollTimeframe = this.currentTimeframe;

    // Update OHLCV status to loading if not yet loaded
    if (!this.chartDataLoaded && this._dataSourceStatus.ohlcv !== DATA_SOURCE_STATUS.SUCCESS) {
      this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.LOADING);
    }

    try {
      // Use requestManager with normal priority for periodic chart refresh
      const data = await requestManager.fetch(
        `/api/tokens/${pollMint}/ohlcv?timeframe=${pollTimeframe}&limit=${CHART_CANDLE_LIMIT}`,
        { priority: "normal" }
      );

      // Drop a response that arrived after the user moved to another token or
      // timeframe (see _loadChartData) — otherwise it overwrites the live chart
      // with the previous token's data.
      if (this.tokenData?.mint !== pollMint || this.currentTimeframe !== pollTimeframe) {
        return;
      }

      if (!Array.isArray(data) || data.length === 0) {
        // No data yet. After a streak of empty responses, switch to a clearer
        // message (an unchanging "Waiting…" spinner reads as "broken") and back
        // off the 3s poll so a token that simply has no OHLCV stops hammering the
        // endpoint. We keep polling (slower) because data may still arrive.
        this._chartEmptyCount = (this._chartEmptyCount || 0) + 1;
        if (loadingText) {
          loadingText.textContent =
            this._chartEmptyCount >= 6
              ? "No chart data available yet — still checking…"
              : "Waiting for chart data...";
        }
        if (loadingOverlay) {
          loadingOverlay.classList.remove("hidden");
        }
        this.chartDataLoaded = false;
        if (this._chartEmptyCount === 6 && !this._chartPollBackedOff) {
          this._chartPollBackedOff = true;
          this._startChartPolling();
        }
        return;
      }

      const chartData = data.map((candle) => ({
        time: candle.timestamp,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume || 0,
      }));

      this.advancedChart.setData(chartData);

      // Hide loading overlay and mark data as loaded
      if (loadingOverlay) {
        loadingOverlay.classList.add("hidden");
      }
      this.chartDataLoaded = true;
      this._chartErrorCount = 0; // Reset error counter on success
      this._chartEmptyCount = 0;
      this._chartPollBackedOff = false;

      // Update OHLCV status to success
      this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.SUCCESS);

      // Update OHLCV display
      this._updateOhlcvDisplay(chartData);

      // If data just loaded (was not loaded before), restart polling with slower interval
      if (!wasDataLoaded && this.chartDataLoaded) {
        this._startChartPolling();
      }
    } catch {
      // On error when no data yet, keep showing waiting message
      if (!this.chartDataLoaded && loadingText) {
        loadingText.textContent = "Waiting for chart data...";
      }
      if (!this.chartDataLoaded && loadingOverlay) {
        loadingOverlay.classList.remove("hidden");
      }
      // Track chart fetch failures - after multiple failures, mark as error
      if (!this.chartDataLoaded) {
        this._chartErrorCount = (this._chartErrorCount || 0) + 1;
        // After 5 consecutive failures (~15-50s depending on interval), mark as error
        if (this._chartErrorCount >= 5) {
          this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.ERROR);
        }
      }
    }
  }

  _refreshOverviewTab() {
    const content = this.dialogEl?.querySelector('[data-tab-content="overview"]');
    if (!content || !this.fullTokenData) return;
    if (content.dataset.loaded !== "true") return;

    const overviewTable = content.querySelector(".overview-left");
    if (!overviewTable) return;

    // Repaint only the left column (live metrics/details) and only when its
    // markup changed, so the chart on the right is untouched and unchanged polls
    // cause no flicker. Previously this called a non-existent
    // `_buildOverviewContent`, which threw on every poll and falsely flipped the
    // "Token" source dot to an error.
    const html = renderOverviewLeft(this.fullTokenData, {
      renderHintTrigger: this._renderHintTrigger.bind(this),
      escapeHtml: this._escapeHtml.bind(this),
      formatShortAddress: this._formatShortAddress.bind(this),
      getRejectionDisplayLabel: this._getRejectionDisplayLabel.bind(this),
    });
    this._renderHtmlIfChanged(overviewTable, html, "__ovHtml");
  }

  close() {
    if (!this.dialogEl) return;

    // Deactivate focus trap
    if (this._focusTrap) {
      this._focusTrap.deactivate();
      this._focusTrap = null;
    }

    // Clear dashboard focus and deprioritize OHLCV when dialog closes (fire and forget)
    // Store mint in local variable before tokenData is nulled
    const mintToUnfocus = this.tokenData?.mint;
    if (mintToUnfocus) {
      this._unfocusToken().catch(() => {
        // Silent - unfocus is best-effort
      });
    }

    this._stopPolling();
    this._stopChartPolling();

    this.dialogEl.classList.remove("active");

    setTimeout(() => {
      if (this._escapeHandler) {
        document.removeEventListener("keydown", this._escapeHandler);
        this._escapeHandler = null;
      }

      if (this.dialogEl) {
        if (this._closeHandler) {
          const closeBtn = this.dialogEl.querySelector(".dialog-close");
          if (closeBtn) {
            closeBtn.removeEventListener("click", this._closeHandler);
          }
          this._closeHandler = null;
        }

        if (this._backdropHandler) {
          const backdrop = this.dialogEl.querySelector(".dialog-backdrop");
          if (backdrop) {
            backdrop.removeEventListener("click", this._backdropHandler);
          }
          this._backdropHandler = null;
        }

        if (this._tabHandlers) {
          this._tabHandlers.forEach(({ element, handler }) => {
            element.removeEventListener("click", handler);
          });
          this._tabHandlers = null;
        }

        if (this._retryHandler) {
          const body = this.dialogEl.querySelector(".dialog-body");
          if (body) {
            body.removeEventListener("click", this._retryHandler);
          }
          this._retryHandler = null;
        }

        // Clean up buy/sell button handlers
        if (this._buyHandler) {
          const buyBtn = this.dialogEl.querySelector("#headerBuyBtn");
          if (buyBtn) {
            buyBtn.removeEventListener("click", this._buyHandler);
          }
          this._buyHandler = null;
        }

        if (this._sellHandler) {
          const sellBtn = this.dialogEl.querySelector("#headerSellBtn");
          if (sellBtn) {
            sellBtn.removeEventListener("click", this._sellHandler);
          }
          this._sellHandler = null;
        }

        if (this._logoZoomHandler) {
          const logoEl = this.dialogEl.querySelector(".header-logo");
          if (logoEl) {
            logoEl.removeEventListener("click", this._logoZoomHandler);
          }
          this._logoZoomHandler = null;
        }

        // Clean up favorites-changed listener
        if (this._favoritesChangedHandler) {
          window.removeEventListener(
            "screenerbot:favorites-changed",
            this._favoritesChangedHandler
          );
          this._favoritesChangedHandler = null;
        }
      }

      if (this.chartResizeObserver) {
        this.chartResizeObserver.disconnect();
        this.chartResizeObserver = null;
      }

      // Clean up theme observer
      if (this._themeObserver) {
        this._themeObserver.disconnect();
        this._themeObserver = null;
      }

      // Clean up advanced chart
      if (this.advancedChart) {
        this.advancedChart.destroy();
        this.advancedChart = null;
      }
      this.chart = null;

      if (this.dialogEl) {
        this.dialogEl.remove();
        this.dialogEl = null;
      }

      this.tokenData = null;
      this.fullTokenData = null;
      this.currentTab = "overview";
      this.currentTimeframe = "5m";
      this.isRefreshing = false;
      this.isOpening = false;
      this.tabHandlers.clear();

      // Reset data source tracking
      this._dataSourceStatus = {
        token: DATA_SOURCE_STATUS.PENDING,
        dexscreener: DATA_SOURCE_STATUS.PENDING,
        rugcheck: DATA_SOURCE_STATUS.PENDING,
        ohlcv: DATA_SOURCE_STATUS.PENDING,
      };
      this._initialLoadComplete = false;
      this._retryCount = 0;
      this._chartErrorCount = 0;
      this._chartEmptyCount = 0;
      this._chartPollBackedOff = false;
      this._connectionState = "online";
      this._consecutiveFailures = 0;
      this.positionsData = null;
      this._positionsFetching = false;
      this._positionsLastPoll = 0;

      this.onClose();
    }, 300);
  }

  _createDialog() {
    this.dialogEl = document.createElement("div");
    this.dialogEl.className = "token-details-dialog";
    this.dialogEl.innerHTML = this._getDialogHTML();
    document.body.appendChild(this.dialogEl);
  }

  _getDialogHTML() {
    const symbol = this.tokenData.symbol || "Unknown";
    const name = this.tokenData.name || "Unknown Token";
    const logoUrl = this.tokenData.logo_url || this.tokenData.image_url || "";
    // WSOL/SOL is the base currency, not a tradeable/analyzable token: hide the
    // trade buttons, the per-token actions (favorite/copy/Solscan), and every tab
    // except Overview — only its price + SOL/USD chart are meaningful here.
    const isSol = this.tokenData.mint === "So11111111111111111111111111111111111111112";

    return `
      <div class="dialog-backdrop"></div>
      <div class="dialog-container">
        <div class="dialog-header">
          <div class="header-top-row">
            <div class="header-left">
              <div class="header-logo">
                ${logoUrl ? `<img src="${this._escapeHtml(logoUrl)}" alt="${this._escapeHtml(symbol)}" onerror="this.parentElement.innerHTML='<div class=\\'logo-placeholder\\'>${this._escapeHtml(symbol.charAt(0))}</div>'" />` : `<div class="logo-placeholder">${this._escapeHtml(symbol.charAt(0))}</div>`}
              </div>
              <div class="header-title">
                <span class="title-main">${this._escapeHtml(symbol)}</span>
                <span class="title-sub">${this._escapeHtml(name)}</span>
              </div>
            </div>
            <div class="header-center">
              <div class="header-price" id="headerPrice">
                <div class="price-skeleton" role="status" aria-label="Loading price">
                  <span class="price-skel price-skel-main shimmer"></span>
                  <span class="price-skel price-skel-sub shimmer"></span>
                  <span class="price-skel price-skel-badge shimmer"></span>
                </div>
              </div>
            </div>
            <div class="header-right">
              ${
                isSol
                  ? ""
                  : `<div class="header-trade-actions">
                <button class="trade-btn buy-btn" id="headerBuyBtn" title="Buy this token">
                  <i class="icon-shopping-cart"></i>
                  Buy
                </button>
                <button class="trade-btn sell-btn" id="headerSellBtn" title="Sell position" disabled>
                  <i class="icon-dollar-sign"></i>
                  Sell
                </button>
              </div>
              <div class="header-actions">
                <button class="action-btn favorite-btn" id="favoriteBtn" title="Add to Favorites" type="button">
                  <i class="icon-star"></i>
                </button>
                <button class="action-btn" id="copyMintBtn" title="Copy Mint Address">
                  <i class="icon-copy"></i>
                </button>
                <a href="https://solscan.io/token/${this._escapeHtml(this.tokenData.mint)}" target="_blank" class="action-btn" title="View on Solscan">
                  <i class="icon-external-link"></i>
                </a>
              </div>`
              }
              <button class="dialog-close" type="button" title="Close (ESC)">
                <i class="icon-x"></i>
              </button>
            </div>
          </div>
          <div class="header-badges-row" id="headerBadgesRow">
            <div class="title-badges" id="headerBadges"></div>
            <div class="header-status-area">
              <div class="data-sources-status" role="status" aria-label="Data loading status">
                <span class="source-status" data-source="token" title="Token info">
                  <span class="status-icon pending" aria-hidden="true"></span>
                  <span class="status-label">Token</span>
                </span>
                <span class="source-status" data-source="dexscreener" title="Market data">
                  <span class="status-icon pending" aria-hidden="true"></span>
                  <span class="status-label">Market</span>
                </span>
                <span class="source-status" data-source="rugcheck" title="Security analysis">
                  <span class="status-icon pending" aria-hidden="true"></span>
                  <span class="status-label">Security</span>
                </span>
                <span class="source-status" data-source="ohlcv" title="Chart data">
                  <span class="status-icon pending" aria-hidden="true"></span>
                  <span class="status-label">Chart</span>
                </span>
              </div>
              <div class="tdd-connection-chip" data-state="online" role="status" hidden>
                <span class="tdd-connection-dot" aria-hidden="true"></span>
                <span class="tdd-connection-text"></span>
              </div>
              <div class="last-updated" id="lastUpdatedTime"></div>
            </div>
          </div>
        </div>

        <div class="source-issues-row" id="sourceIssuesRow" role="status" hidden></div>

        <div class="dialog-tabs">
          <button class="tab-button active" data-tab="overview">
            <i class="icon-info"></i>
            Overview
          </button>
          ${
            isSol
              ? ""
              : `<button class="tab-button" data-tab="security">
            <i class="icon-shield"></i>
            Security
          </button>
          <button class="tab-button" data-tab="positions">
            <i class="icon-chart-bar"></i>
            Positions
          </button>
          <button class="tab-button" data-tab="pools">
            <i class="icon-droplet"></i>
            Pools
          </button>
          <button class="tab-button" data-tab="links">
            <i class="icon-link"></i>
            Links
          </button>
          <button class="tab-button" data-tab="transactions">
            <i class="icon-activity"></i>
            Txns
          </button>`
          }
        </div>

        <div class="dialog-body">
          <div class="tab-content active" data-tab-content="overview">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="security">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="positions">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="pools">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="links">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="transactions">
            <div class="loading-spinner">Loading...</div>
          </div>
        </div>
      </div>
    `;
  }

  _updateHeader(token) {
    // Update symbol / name / logo from the latest data. The header markup is
    // built once from the seed token (which, when opened from the billboard or
    // search, only carries {mint, symbol}); without refreshing here the name
    // stays "Unknown Token" and the logo a placeholder even after full data loads.
    const symbolEl = this.dialogEl.querySelector(".title-main");
    if (symbolEl && token.symbol && token.symbol !== "NOT_FOUND") {
      if (symbolEl.textContent !== token.symbol) symbolEl.textContent = token.symbol;
    }
    const nameEl = this.dialogEl.querySelector(".title-sub");
    if (nameEl && token.name && token.name !== "Token not in cache") {
      if (nameEl.textContent !== token.name) nameEl.textContent = token.name;
    }
    const logoEl = this.dialogEl.querySelector(".header-logo");
    if (logoEl) {
      const sym = token.symbol && token.symbol !== "NOT_FOUND" ? token.symbol : "?";
      const logoUrl = token.logo_url || token.image_url || "";
      const logoHtml = logoUrl
        ? `<img src="${this._escapeHtml(logoUrl)}" alt="${this._escapeHtml(sym)}" onerror="this.parentElement.innerHTML='<div class=\\'logo-placeholder\\'>${this._escapeHtml(sym.charAt(0))}</div>'" />`
        : `<div class="logo-placeholder">${this._escapeHtml(sym.charAt(0))}</div>`;
      this._renderHtmlIfChanged(logoEl, logoHtml, "__logoHtml");
    }

    // Update badges in separate row
    const badgesContainer = this.dialogEl.querySelector("#headerBadges");
    const badgesRow = this.dialogEl.querySelector("#headerBadgesRow");
    if (badgesContainer && badgesRow) {
      const badges = [];

      // Price source badge (first, left of Verified): shows which price system
      // produced the header quote — POOL (real-time on-chain, preferred) or API
      // (cached market-data fallback).
      if (token.price_source) {
        const isPool = token.price_source === "pool";
        badges.push(
          `<span class="badge ${isPool ? "badge-success" : "badge-secondary"}" title="${
            isPool ? "Price from real-time on-chain pool" : "Price from cached market-data (API)"
          }">Price ${isPool ? "Pool" : "API"}</span>`
        );
      }

      if (token.verified) badges.push('<span class="badge badge-success">Verified</span>');

      // Mutable/Immutable badge
      if (token.is_mutable === false) {
        badges.push('<span class="badge badge-success">Immutable</span>');
      } else if (token.is_mutable === true) {
        badges.push('<span class="badge badge-warning">Mutable</span>');
      }

      // Update Authority badge
      if (token.update_authority) {
        const auth = token.update_authority;
        const trunc = auth.slice(0, 4) + "..." + auth.slice(-4);
        badges.push(
          `<span class="badge badge-secondary" title="Update Authority: ${auth}">Auth: ${trunc}</span>`
        );
      }

      if (token.has_open_position) badges.push('<span class="badge badge-info">Position</span>');
      if (token.blacklisted) badges.push('<span class="badge badge-danger">Blacklisted</span>');
      if (token.has_ohlcv) badges.push('<span class="badge badge-secondary">OHLCV</span>');

      // Only repaint when the badge set changed. _updateHeader runs on every 5s
      // poll, and an unconditional innerHTML write would drop any text selection
      // in the header and dismiss the Auth tooltip every tick.
      this._renderHtmlIfChanged(badgesContainer, badges.join(""), "__badgesHtml");
      // Always show badges row for layout consistency (contains status now)
      badgesRow.style.display = "flex";
    }

    // Update Last Updated time
    const lastUpdatedEl = this.dialogEl.querySelector("#lastUpdatedTime");
    if (lastUpdatedEl) {
      const marketFetchedAt = token.market_data_last_fetched_at;
      const poolFetchedAt = token.pool_price_last_calculated_at;

      // Use the most recent timestamp
      let lastTs = 0;
      if (marketFetchedAt && marketFetchedAt > lastTs) lastTs = marketFetchedAt;
      if (poolFetchedAt && poolFetchedAt > lastTs) lastTs = poolFetchedAt;

      if (lastTs > 0) {
        // Convert timestamp (seconds or milliseconds) to relative time
        // If it's very large, assume ms, but DB usually stores ms or sec.
        // Rust types.rs says i64 for ts, usually ms in JS land if passed directly.
        // Assuming backend sends milliseconds or seconds. Let's check Utils.formatTimestamp or relative
        const now = Date.now();
        // Check if timestamp is likely seconds (less than 2030 in s)
        const tsMs = lastTs < 2000000000 ? lastTs * 1000 : lastTs;
        const diff = Math.max(0, now - tsMs);

        let timeStr = "";
        if (diff < 60000) {
          timeStr = "Just now";
        } else if (diff < 3600000) {
          timeStr = Math.floor(diff / 60000) + "m ago";
        } else {
          timeStr = new Date(tsMs).toLocaleTimeString();
        }

        lastUpdatedEl.textContent = `Updated: ${timeStr}`;
      } else {
        lastUpdatedEl.textContent = "";
      }
    }

    // Update price section (idempotent: only repaints when the price/metrics
    // markup actually changes, avoiding per-poll churn and selection loss).
    const priceContainer = this.dialogEl.querySelector("#headerPrice");
    if (priceContainer) {
      this._renderHtmlIfChanged(priceContainer, this._buildHeaderPrice(token), "__priceHtml");
    }

    // Update sell button state based on open positions
    const sellBtn = this.dialogEl.querySelector("#headerSellBtn");
    if (sellBtn) {
      sellBtn.disabled = !token.has_open_position;
    }

    // Setup copy mint button
    const copyBtn = this.dialogEl.querySelector("#copyMintBtn");
    if (copyBtn && !copyBtn._hasListener) {
      copyBtn._hasListener = true;
      copyBtn.addEventListener("click", () => {
        Utils.copyToClipboard(token.mint);
        Utils.showToast("Mint address copied!", "success");
      });
    }

    // Setup favorite button
    const favBtn = this.dialogEl.querySelector("#favoriteBtn");
    if (favBtn && !favBtn._hasListener) {
      favBtn._hasListener = true;
      favBtn.addEventListener("click", () => this._toggleFavorite());
      // Check initial favorite state once
      this._checkFavoriteState();
    }
  }

  /**
   * Fetch the favorites list and update the button to reflect whether the
   * current token is already favorited. Runs once per dialog open.
   */
  async _checkFavoriteState() {
    const mint = this.tokenData?.mint;
    if (!mint) return;
    try {
      const response = await fetch("/api/tokens/favorites");
      if (!response.ok) return;
      const data = await response.json();
      const favorites = data.favorites || [];
      const isFav = favorites.some((f) => f.mint === mint);
      this._updateFavoriteButton(isFav);
    } catch {
      // Silent — best-effort initial state check
    }
  }

  /**
   * Update the favorite button visual state (active class + title).
   * @param {boolean} isFavorite
   */
  _updateFavoriteButton(isFavorite) {
    const btn = this.dialogEl?.querySelector("#favoriteBtn");
    if (!btn) return;
    btn.classList.toggle("active", isFavorite);
    btn.title = isFavorite ? "Remove from Favorites" : "Add to Favorites";
  }

  /**
   * Toggle favorite status for the current token. POST to add, DELETE to remove.
   * Dispatches screenerbot:favorites-changed so other components stay in sync.
   */
  async _toggleFavorite() {
    const btn = this.dialogEl?.querySelector("#favoriteBtn");
    if (!btn || btn.disabled) return;
    const mint = this.tokenData?.mint;
    if (!mint) return;

    const currentlyFavorite = btn.classList.contains("active");
    const symbol =
      this.fullTokenData?.symbol || this.tokenData?.symbol || "";
    const name = this.fullTokenData?.name || this.tokenData?.name || null;
    const logo_url =
      this.fullTokenData?.logo_url || this.tokenData?.logo_url || null;

    btn.disabled = true;
    try {
      if (currentlyFavorite) {
        const response = await fetch(
          `/api/tokens/favorites/${encodeURIComponent(mint)}`,
          { method: "DELETE" }
        );
        if (!response.ok) throw new Error("Failed to remove favorite");
        this._updateFavoriteButton(false);
        Utils.showToast(
          `${symbol || "Token"} removed from favorites`,
          "success"
        );
      } else {
        const response = await fetch("/api/tokens/favorites", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            mint,
            symbol: symbol || null,
            name,
            logo_url,
          }),
        });
        if (!response.ok) throw new Error("Failed to add favorite");
        this._updateFavoriteButton(true);
        Utils.showToast(
          `${symbol || "Token"} added to favorites`,
          "success"
        );
      }

      // Emit event for other UI components (context menu, favorites tab, etc.)
      window.dispatchEvent(
        new CustomEvent("screenerbot:favorites-changed", {
          detail: { mint, isFavorite: !currentlyFavorite },
        })
      );
    } catch (error) {
      Utils.showToast(error.message || "Failed to update favorites", "error");
    } finally {
      btn.disabled = false;
    }
  }

  _buildHeaderPrice(token) {
    const priceSol =
      token.price_sol !== null && token.price_sol !== undefined
        ? Utils.formatPriceSubscript(token.price_sol, { precision: 5 })
        : "—";
    const priceUsd =
      token.price_usd !== null && token.price_usd !== undefined
        ? Utils.formatCurrencyUSD(token.price_usd)
        : "";

    // Price change badge
    let changeHtml = "";
    if (token.price_change_periods) {
      const change24h = token.price_change_periods.h24;
      if (change24h !== null && change24h !== undefined) {
        const changeClass = change24h >= 0 ? "positive" : "negative";
        const sign = change24h >= 0 ? "+" : "";
        changeHtml = `<span class="price-change ${changeClass}">${sign}${change24h.toFixed(2)}%</span>`;
      }
    }

    return `
      <div class="price-block">
        <div class="price-sol-row">
          <span class="price-sol">${priceSol}</span>
          <span class="price-sol-unit">SOL</span>
        </div>
        ${priceUsd ? `<span class="price-usd">${priceUsd}</span>` : ""}
      </div>
      ${changeHtml}
      <div class="price-metrics">
        <div class="metric-item">
          <span class="metric-label">MCap</span>
          <span class="metric-value">${token.market_cap ? Utils.formatCompactNumber(token.market_cap, { prefix: "$" }) : "—"}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Liq</span>
          <span class="metric-value">${token.liquidity_usd ? Utils.formatCompactNumber(token.liquidity_usd, { prefix: "$" }) : "—"}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Vol 24H</span>
          <span class="metric-value">${token.volume_24h ? Utils.formatCompactNumber(token.volume_24h, { prefix: "$" }) : "—"}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Holders</span>
          <span class="metric-value">${token.total_holders ? Utils.formatCompactNumber(token.total_holders) : "—"}</span>
        </div>
      </div>
    `;
  }

  _attachEventHandlers() {
    const closeBtn = this.dialogEl.querySelector(".dialog-close");
    this._closeHandler = () => this.close();
    closeBtn.addEventListener("click", this._closeHandler);

    const backdrop = this.dialogEl.querySelector(".dialog-backdrop");
    this._backdropHandler = () => this.close();
    backdrop.addEventListener("click", this._backdropHandler);

    this._escapeHandler = (e) => {
      if (e.key === "Escape") {
        this.close();
      }
    };
    document.addEventListener("keydown", this._escapeHandler);

    // Trade action buttons
    const buyBtn = this.dialogEl.querySelector("#headerBuyBtn");
    if (buyBtn) {
      this._buyHandler = () => this._handleBuyClick();
      buyBtn.addEventListener("click", this._buyHandler);
    }

    const sellBtn = this.dialogEl.querySelector("#headerSellBtn");
    if (sellBtn) {
      this._sellHandler = () => this._handleSellClick();
      sellBtn.addEventListener("click", this._sellHandler);
    }

    // Click the logo to zoom it (same lightbox as the tokens table)
    const logoEl = this.dialogEl.querySelector(".header-logo");
    if (logoEl) {
      this._logoZoomHandler = () => {
        const url = logoEl.querySelector("img")?.getAttribute("src");
        if (!url) return; // placeholder letter, nothing to zoom
        showImageLightbox({
          imageUrl: url,
          symbol: this.tokenData?.symbol || "",
          name: this.tokenData?.name || "",
        });
      };
      logoEl.addEventListener("click", this._logoZoomHandler);
      logoEl.classList.add("clickable-logo");
    }

    // Listen for favorite changes from other UI components (context menu, etc.)
    this._favoritesChangedHandler = (e) => {
      if (e.detail?.mint === this.tokenData?.mint) {
        this._updateFavoriteButton(e.detail.isFavorite);
      }
    };
    window.addEventListener(
      "screenerbot:favorites-changed",
      this._favoritesChangedHandler
    );

    // Delegated retry handler for the initial-load error state's Retry button.
    const body = this.dialogEl.querySelector(".dialog-body");
    if (body) {
      this._retryHandler = (e) => {
        const btn = e.target.closest('[data-action="tdd-retry"]');
        if (btn) {
          this._retryInitialLoad();
        }
      };
      body.addEventListener("click", this._retryHandler);
    }

    const tabButtons = this.dialogEl.querySelectorAll(".tab-button");
    this._tabHandlers = [];
    tabButtons.forEach((btn) => {
      const handler = () => {
        const tabId = btn.dataset.tab;
        this._switchTab(tabId);
      };
      btn.addEventListener("click", handler);
      this._tabHandlers.push({ element: btn, handler });
    });
  }

  // =========================================================================
  // OVERVIEW TAB
  // =========================================================================

  _loadOverviewTab(content) {
    // Use whatever data we have - show partial content rather than blocking
    const tokenToUse = this.fullTokenData || this.tokenData;

    if (!tokenToUse || !tokenToUse.mint) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    // Build overview with available data - placeholders for missing fields
    content.innerHTML = renderOverviewTab(tokenToUse, {
      renderHintTrigger: this._renderHintTrigger.bind(this),
      escapeHtml: this._escapeHtml.bind(this),
      formatShortAddress: this._formatShortAddress.bind(this),
      getRejectionDisplayLabel: this._getRejectionDisplayLabel.bind(this),
    });

    setTimeout(() => {
      this._initializeChart(tokenToUse.mint);
    }, 100);

    // Only mark as fully loaded if we have complete data
    if (this.fullTokenData && this._initialLoadComplete) {
      content.dataset.loaded = "true";
    }
  }

  // =========================================================================
  // SECURITY TAB
  // =========================================================================

  _loadSecurityTab(content) {
    if (!content) return;
    // Use whatever data we have - show partial content rather than blocking
    const tokenToUse = this.fullTokenData || this.tokenData;

    if (!tokenToUse || !tokenToUse.mint) {
      this._renderTabWaiting(content, "Waiting for token data…");
      return;
    }

    const html = renderSecurityTab(tokenToUse, {
      renderHintTrigger: this._renderHintTrigger.bind(this),
      escapeHtml: this._escapeHtml.bind(this),
      formatShortAddress: this._formatShortAddress.bind(this),
    });

    // Only repaint when the markup actually changed. While Rugcheck data is still
    // pending the loading markup is identical every poll, so this is a no-op and
    // the staggered card animations never restart. Re-init hints only on repaint.
    const changed = this._renderHtmlIfChanged(content, html, "__stateHtml");
    if (changed) {
      HintTrigger.initAll();
    }

    // Check if we have security data
    const hasSecurityData =
      tokenToUse.safety_score !== undefined && tokenToUse.safety_score !== null;
    content.dataset.loaded = hasSecurityData ? "true" : "false";
  }

  // =========================================================================
  // POOLS TAB
  // =========================================================================

  _loadPoolsTab(content) {
    if (!this.fullTokenData) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    content.innerHTML = renderPoolsTab(this.fullTokenData, {
      renderHintTrigger: this._renderHintTrigger.bind(this),
      escapeHtml: this._escapeHtml.bind(this),
      formatShortAddress: this._formatShortAddress.bind(this),
    });
    content.dataset.loaded = "true";
    initPoolsTabEvents(content);
  }

  // =========================================================================
  // LINKS TAB
  // =========================================================================

  _loadLinksTab(content) {
    if (!this.fullTokenData) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    content.innerHTML = renderLinksTab(this.fullTokenData, {
      escapeHtml: this._escapeHtml.bind(this),
      formatShortAddress: this._formatShortAddress.bind(this),
    });
    content.dataset.loaded = "true";
    initLinksTabEvents(content);
  }

  _buildLinksContent(token) {
    const mint = token.mint;
    const hasWebsites = token.websites && token.websites.length > 0;
    const hasSocials = token.socials && token.socials.length > 0;
    const hasLogo = !!token.logo_url;
    const hasHeader = !!token.header_image_url;
    const hasDescription = !!token.description;

    // Build left column - Media & Info
    const leftCol = this._buildLinksLeftColumn(token, hasLogo, hasHeader, hasDescription);

    // Build right column - All links organized
    const rightCol = this._buildLinksRightColumn(token, mint, hasWebsites, hasSocials);

    return `<div class="links-container">${leftCol}${rightCol}</div>`;
  }

  _buildLinksLeftColumn(token, hasLogo, hasHeader, hasDescription) {
    const mint = token.mint;

    // Token info section
    const tokenInfoSection = `
      <div class="links-info-card">
        <div class="links-info-title">
          <i class="icon-info"></i>
          Token Info
        </div>
        <div class="links-info-content">
          <div class="links-info-row">
            <span class="links-info-label">Mint Address</span>
            <div class="links-info-value-group">
              <span class="links-info-value mono" title="${this._escapeHtml(mint)}">${this._formatShortAddress(mint)}</span>
              <button class="copy-btn-mini" data-copy="${this._escapeHtml(mint)}" title="Copy mint address">
                <i class="icon-copy"></i>
              </button>
            </div>
          </div>
          ${
            token.data_source
              ? `
          <div class="links-info-row">
            <span class="links-info-label">Data Source</span>
            <span class="links-info-value badge">${this._escapeHtml(token.data_source)}</span>
          </div>
          `
              : ""
          }
          ${
            token.verified
              ? `
          <div class="links-info-row">
            <span class="links-info-label">Status</span>
            <span class="links-info-value badge success"><i class="icon-shield-check"></i> Verified</span>
          </div>
          `
              : ""
          }
        </div>
      </div>
    `;

    // Media section - logo and header
    let mediaSection = "";
    if (hasLogo || hasHeader) {
      const logoHtml = hasLogo
        ? `
        <div class="links-media-item">
          <div class="links-media-label">Logo</div>
          <div class="links-media-preview logo">
            <img src="${this._escapeHtml(token.logo_url)}" alt="Token Logo" onerror="this.parentElement.innerHTML='<i class=\\'icon-image-off\\'></i>'" />
          </div>
          <a href="${this._escapeHtml(token.logo_url)}" target="_blank" class="links-media-link">
            <i class="icon-external-link"></i> Open Image
          </a>
        </div>
      `
        : "";

      const headerHtml = hasHeader
        ? `
        <div class="links-media-item">
          <div class="links-media-label">Header Image</div>
          <div class="links-media-preview header">
            <img src="${this._escapeHtml(token.header_image_url)}" alt="Header Image" onerror="this.parentElement.innerHTML='<i class=\\'icon-image-off\\'></i>'" />
          </div>
          <a href="${this._escapeHtml(token.header_image_url)}" target="_blank" class="links-media-link">
            <i class="icon-external-link"></i> Open Image
          </a>
        </div>
      `
        : "";

      mediaSection = `
        <div class="links-info-card">
          <div class="links-info-title">
            <i class="icon-image"></i>
            Media Assets
          </div>
          <div class="links-media-grid">
            ${logoHtml}
            ${headerHtml}
          </div>
        </div>
      `;
    }

    // Description section
    let descriptionSection = "";
    if (hasDescription) {
      descriptionSection = `
        <div class="links-info-card">
          <div class="links-info-title">
            <i class="icon-file-text"></i>
            Description
          </div>
          <div class="links-description">
            ${this._escapeHtml(token.description)}
          </div>
        </div>
      `;
    }

    return `
      <div class="links-left-col">
        ${tokenInfoSection}
        ${mediaSection}
        ${descriptionSection}
      </div>
    `;
  }

  _buildLinksRightColumn(token, mint, hasWebsites, hasSocials) {
    // Explorers section - comprehensive list
    const explorersSection = `
      <div class="links-section-card">
        <div class="links-section-title">
          <i class="icon-search"></i>
          Explorers & Analytics
        </div>
        <div class="links-grid-compact">
          ${this._buildExplorerLink("https://solscan.io/token/" + mint, "Solscan")}
          ${this._buildExplorerLink("https://explorer.solana.com/address/" + mint, "Solana Explorer")}
          ${this._buildExplorerLink("https://birdeye.so/token/" + mint + "?chain=solana", "Birdeye")}
          ${this._buildExplorerLink("https://dexscreener.com/solana/" + mint, "DEX Screener")}
          ${this._buildExplorerLink("https://www.geckoterminal.com/solana/tokens/" + mint, "GeckoTerminal")}
          ${this._buildExplorerLink("https://www.dextools.io/app/en/solana/pair-explorer/" + mint, "DexTools")}
          ${this._buildExplorerLink("https://gmgn.ai/sol/token/" + mint, "GMGN")}
          ${this._buildExplorerLink("https://photon-sol.tinyastro.io/en/lp/" + mint, "Photon")}
          ${this._buildExplorerLink("https://rugcheck.xyz/tokens/" + mint, "RugCheck")}
          ${this._buildExplorerLink("https://app.bubblemaps.io/sol/token/" + mint, "Bubblemaps")}
          ${this._buildExplorerLink("https://www.coingecko.com/en/coins/" + mint, "CoinGecko")}
          ${this._buildExplorerLink("https://jup.ag/swap/SOL-" + mint, "Jupiter Swap")}
        </div>
      </div>
    `;

    // Official websites section
    let websitesSection = "";
    if (hasWebsites) {
      const websiteLinks = token.websites
        .map((site) => {
          const label = site.label || this._extractDomainName(site.url) || "Website";
          return this._buildOfficialLink(site.url, label);
        })
        .join("");

      websitesSection = `
        <div class="links-section-card">
          <div class="links-section-title">
            <i class="icon-globe"></i>
            Official Websites
          </div>
          <div class="links-list">
            ${websiteLinks}
          </div>
        </div>
      `;
    }

    // Social links section
    let socialsSection = "";
    if (hasSocials) {
      const socialLinks = token.socials
        .map((social) => {
          const { label } = this._getSocialMeta(social.platform);
          return this._buildSocialLink(social.url, label);
        })
        .join("");

      socialsSection = `
        <div class="links-section-card">
          <div class="links-section-title">
            <i class="icon-share-2"></i>
            Social Media
          </div>
          <div class="links-list">
            ${socialLinks}
          </div>
        </div>
      `;
    }

    // No links message
    let noLinksMessage = "";
    if (!hasWebsites && !hasSocials) {
      noLinksMessage = `
        <div class="links-empty-notice">
          <i class="icon-link-2-off"></i>
          <span>No official website or social links available for this token.</span>
        </div>
      `;
    }

    return `
      <div class="links-right-col">
        ${explorersSection}
        ${websitesSection}
        ${socialsSection}
        ${noLinksMessage}
      </div>
    `;
  }
}

// Apply mixins to TokenDetailsDialog
applyTradeActionsMixin(TokenDetailsDialog);
applyTransactionsTabMixin(TokenDetailsDialog);
applyChartTabMixin(TokenDetailsDialog);
applyUtilitiesMixin(TokenDetailsDialog);
applyStateHandlingMixin(TokenDetailsDialog);
applyPositionsTabMixin(TokenDetailsDialog);

// ============================================================================
// Global Event Listener for Context Menu "View Details" Action
// ============================================================================
// This listener allows any page to open the TokenDetailsDialog via custom event
// dispatched from context_menu.js when user clicks "View Details"

let globalDialogInstance = null;

window.addEventListener("screenerbot:open-token-details", async (event) => {
  const { mint, symbol } = event.detail || {};

  if (!mint) {
    console.error("[TokenDetailsDialog] Event received without mint address");
    return;
  }

  console.log(`[TokenDetailsDialog] Opening details for ${symbol || mint}`);

  // Close existing dialog if open for a different token
  if (globalDialogInstance && globalDialogInstance.dialogEl) {
    if (globalDialogInstance.tokenData?.mint === mint) {
      // Already open for this token, do nothing
      console.log("[TokenDetailsDialog] Dialog already open for this token");
      return;
    }
    globalDialogInstance.close();
    await new Promise((resolve) => setTimeout(resolve, 350));
  }

  // Create new dialog instance if needed
  if (!globalDialogInstance) {
    globalDialogInstance = new TokenDetailsDialog({
      onClose: () => {
        // Keep instance for reuse, just clean up state
      },
    });
  }

  // Open dialog with minimal token data (dialog will fetch full details)
  await globalDialogInstance.show({ mint, symbol: symbol || "" });
});
