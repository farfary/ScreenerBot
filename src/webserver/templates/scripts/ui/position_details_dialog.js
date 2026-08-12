/**
 * Position Details Dialog
 * Full-screen dialog showing comprehensive position information with multiple tabs
 */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { Poller } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import { notificationManager } from "../core/notifications.js";
import * as Hints from "../core/hints.js";
import { DialogTabBar, renderDialogTabRow } from "./dialog_tab_bar.js";
import { HintTrigger } from "./hint_popover.js";
import { applyOverviewTabMixin } from "./position_details/overview_tab.js";
import { applyChartTabMixin } from "./position_details/chart_tab.js";
import { applyActivityTabMixin } from "./position_details/activity_tab.js";
import { applyUtilitiesMixin } from "./position_details/utilities.js";
import { pushEscapeHandler } from "../core/escape_stack.js";

// Refresh cadence. `/details` is a heavy endpoint (full token assembly, decimals batch,
// pool lookup, two history queries, SOL price) and the chart tab refetches candles off the
// same tick, so it does not belong on the global 1s dashboard interval.
const POLL_IDLE_MS = 2000;
// While a swap is submitted but unverified the position's numbers are ABOUT to change and
// nothing else will announce it — verification emits no action event — so poll tighter to
// catch the moment it lands.
const POLL_PENDING_MS = 1000;

export class PositionDetailsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.onTradeComplete = options.onTradeComplete || (() => {});
    this.dialogEl = null;
    this.currentTab = "overview";
    this.positionData = null;
    this.fullDetails = null;
    this.isLoading = false;
    this.isOpening = false;
    this.refreshPoller = null;
    this.tradeDialog = null;
    this._dialogTabBar = null;
    // Chosen from the position's own duration on first render (see chart_tab.js),
    // so a week-old position does not open on a timeframe that shows ten hours.
    this._chartTimeframe = null;
    this._releaseEscape = null;
    this._closeHandler = null;
    this._backdropHandler = null;
    this._copyMintHandler = null;
    this._favoriteHandler = null;
    this._actionHandlers = null;
    this._manualToggleHandler = null;
    this._managementChangedHandler = null;
    this._focusTrap = null;
    // Cadence the refresh poller is currently running at, so it is only rescheduled when
    // the pending/idle state actually flips.
    this._pollMs = null;
    this._fetchFailures = 0;
    // The token's all-time activity: its own payload (own endpoint, fetched lazily) plus
    // the view state, kept on the dialog so a poll's re-render restores the filter, the
    // filter and every card the user had expanded.
    this._activity = null;
    this._activityLoading = false;
    this._activityError = null;
    this._activityRenderedFp = null;
    this._activityFilter = "all";
    this._activityExpanded = new Set();
    this._activityOpenRounds = new Set();
    this._activityRoundsInitialized = false;
    this._activityClickHandler = null;
  }

  /**
   * Show dialog with position data
   * @param {Object} positionData - Position data object (at minimum needs id or mint)
   */
  async show(positionData) {
    if (!positionData || (!positionData.id && !positionData.mint)) {
      console.error("Invalid position data provided to PositionDetailsDialog");
      return;
    }

    if (this.isOpening) {
      console.log("Dialog already opening, ignoring duplicate request");
      return;
    }

    if (this.dialogEl) {
      this.close();
      await new Promise((resolve) => setTimeout(resolve, 350));
    }

    this.isOpening = true;

    try {
      this.positionData = positionData;
      this.fullDetails = null;
      this.currentTab = "overview";
      this._resetActivityState();

      this._createDialog();
      this._attachEventHandlers();
      this._checkFavoriteState();

      requestAnimationFrame(() => {
        if (this.dialogEl) {
          this.dialogEl.classList.add("active");
          // Add ARIA attributes for accessibility
          const container = this.dialogEl.querySelector(".dialog-container");
          if (container) {
            container.setAttribute("role", "dialog");
            container.setAttribute("aria-modal", "true");
            container.setAttribute("aria-labelledby", "pdd-dialog-title");
          }
          // Activate focus trap
          this._focusTrap = createFocusTrap(this.dialogEl);
          this._focusTrap.activate();
        }
      });

      // Fetch full details
      await this._fetchDetails();

      // Start polling for live price updates
      this._startPolling();
    } finally {
      this.isOpening = false;
    }
  }

  /**
   * Fetch full position details from API
   */
  async _fetchDetails() {
    if (this.isLoading) return;
    this.isLoading = true;

    try {
      const key = this._getPositionKey();
      const data = await requestManager.fetch(`/api/positions/${key}/details`, {
        priority: "high",
      });

      this.fullDetails = data;
      this._fetchFailures = 0;
      this._syncPollCadence();
      this._updateDialogContent();
    } catch (error) {
      console.error("Error loading position details:", error);
      // A poll that fails once must not wipe a working view: this used to replace the
      // whole active tab with an error state on any dropped request, so a single blip
      // destroyed the rendered position until the next tick rebuilt it. A view that is
      // stale for good is worse than an error, though, so keep counting.
      this._fetchFailures = (this._fetchFailures || 0) + 1;
      if (!this.fullDetails || this._fetchFailures >= 3) {
        this._showError("Failed to load position details");
      }
    } finally {
      this.isLoading = false;
    }

    // The token's all-time activity is its own (much heavier) endpoint and is refreshed
    // only while its tab is open — see activity_tab.js.
    if (this.currentTab === "activity") {
      this._fetchActivity();
    }
  }

  /**
   * Drop everything the activity tab held. The delegated listener lives on the tab-content
   * node, which the dialog element takes with it — but the REFERENCE must go too, or the
   * next open would see a handler already set and skip binding one onto the fresh DOM.
   */
  _resetActivityState() {
    this._activity = null;
    this._activityLoading = false;
    this._activityError = null;
    this._activityRenderedFp = null;
    this._activityFilter = "all";
    this._activityExpanded = new Set();
    this._activityOpenRounds = new Set();
    this._activityRoundsInitialized = false;
    this._activityClickHandler = null;
  }

  /**
   * Get position key for API request (id:123 or mint:address)
   */
  _getPositionKey() {
    if (this.positionData.id) {
      return `id:${this.positionData.id}`;
    }
    return `mint:${this.positionData.mint}`;
  }

  /**
   * Start polling for live updates
   */
  _startPolling() {
    this._stopPolling();

    // Only poll for open positions
    if (this.positionData.position_type === "closed") {
      return;
    }

    this._pollMs = this._desiredPollMs();
    this.refreshPoller = new Poller(
      () => {
        this._fetchDetails();
      },
      { label: "PositionDetails", intervalMs: this._pollMs }
    );
    this.refreshPoller.start();

    // React immediately to live buy/sell action events (SSE) so the trade buttons
    // disable/enable, and pull fresh details: a manual trade's action reaches
    // "completed" when the swap is SUBMITTED, which is exactly when the pending swap
    // appears on the position and the dialog should start showing it.
    this._liveTradeUnsub = notificationManager.subscribe((event) => {
      this._updateTradeButtonsState();
      const mint = this.fullDetails?.position?.mint || this.positionData?.mint;
      const eventMint = event?.notification?.entity_id || event?.notification?.metadata?.mint;
      if (mint && eventMint && eventMint !== mint) return;
      this._fetchDetails();
    });
  }

  /** Cadence this position warrants right now. */
  _desiredPollMs() {
    return this.fullDetails?.pending_swaps?.length ? POLL_PENDING_MS : POLL_IDLE_MS;
  }

  /**
   * Reschedule the poller when the position moves between "settled" and "a swap is
   * confirming". `Poller` reads its interval when it starts, so a change needs a restart.
   */
  _syncPollCadence() {
    if (!this.refreshPoller) return;
    const next = this._desiredPollMs();
    if (next === this._pollMs) return;
    this._pollMs = next;
    this.refreshPoller.intervalMs = next;
    this.refreshPoller.start({ silent: true });
  }

  /**
   * Stop polling
   */
  _stopPolling() {
    if (this.refreshPoller) {
      this.refreshPoller.stop();
      this.refreshPoller.cleanup();
      this.refreshPoller = null;
    }
    this._pollMs = null;
    if (this._liveTradeUnsub) {
      this._liveTradeUnsub();
      this._liveTradeUnsub = null;
    }
  }

  /**
   * Show error message in dialog. Uses the shared empty state — `pdd-error-state` had no
   * CSS rule anywhere, so a failed load rendered as unstyled text in the corner.
   */
  _showError(message) {
    const content = this.dialogEl?.querySelector(".tab-content.active");
    if (content) {
      content.innerHTML = `
        <div class="pdd-empty-state">
          <i class="icon-circle-alert"></i>
          <p>${Utils.escapeHtml(message)}</p>
        </div>
      `;
    }
  }

  /**
   * Update dialog content after data fetch
   */
  _updateDialogContent() {
    if (!this.fullDetails) return;
    this._updateHeader();
    this._updateTradeButtonsState();
    this._loadTabContent(this.currentTab);
  }

  /**
   * Detect whether a trade is processing in the background for this position.
   * Mirrors the positions list: an exit tx pending verification ("closing"),
   * or a live buy/sell action in-flight for this mint.
   * @returns {"buying"|"selling"|"closing"|null}
   */
  _inFlightTradeState() {
    const pos = this.fullDetails?.position || this.positionData;
    if (!pos) return null;
    if (pos.exit_transaction_signature && !pos.transaction_exit_verified) {
      return "closing";
    }

    const live = notificationManager.getInFlightTradeForMint(pos.mint);
    if (live) return live;

    // An action reports "completed" the moment its swap is SUBMITTED, so it stops
    // covering the position well before the position changes. A pending swap keeps the
    // buttons locked for the rest of that window, which is exactly when a second add
    // would be fired on top of one still confirming.
    const pending = this.fullDetails?.pending_swaps?.[0];
    if (pending) return pending.kind === "dca" ? "buying" : "selling";

    return null;
  }

  /**
   * Disable the Add/Partial/Close buttons while a trade is in-flight so the user
   * can't double-fire, and re-enable them once it settles. Runs on every poll tick.
   */
  _updateTradeButtonsState() {
    const inFlight = this._inFlightTradeState();
    const busy = inFlight !== null;
    const label =
      inFlight === "buying"
        ? "Buy in progress\u2026"
        : inFlight === "selling"
          ? "Sell in progress\u2026"
          : inFlight === "closing"
            ? "Close in progress\u2026"
            : "";
    ["#pddAddBtn", "#pddPartialBtn", "#pddCloseBtn"].forEach((id) => {
      const btn = this.dialogEl?.querySelector(id);
      if (!btn) return;
      btn.disabled = busy;
      if (busy) {
        btn.title = label;
      } else {
        btn.removeAttribute("title");
      }
    });
  }

  /**
   * Close dialog
   */
  close() {
    if (!this.dialogEl) return;

    // Deactivate focus trap
    if (this._focusTrap) {
      this._focusTrap.deactivate();
      this._focusTrap = null;
    }

    this._stopPolling();
    // The chart owns a lightweight-charts instance, a ResizeObserver and a
    // MutationObserver on <html>. Closing the dialog from the Chart tab used to
    // leave all three alive — one leaked set per open/close, each still
    // re-theming a chart whose DOM was already gone.
    this._destroyPositionChart?.();
    // Released before the exit animation, not after: a second Escape during those 300ms
    // would otherwise re-enter close() on a dialog that is already going away.
    this._releaseEscape?.();
    this._releaseEscape = null;
    this.dialogEl.classList.remove("active");

    setTimeout(() => {
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

        if (this._dialogTabBar) {
          this._dialogTabBar.destroy();
          this._dialogTabBar = null;
        }

        if (this._manualToggleHandler) {
          const manualToggle = this.dialogEl.querySelector("#pddManualToggle");
          if (manualToggle) {
            manualToggle.removeEventListener("change", this._manualToggleHandler);
          }
          this._manualToggleHandler = null;
        }

        if (this._managementChangedHandler) {
          window.removeEventListener(
            "screenerbot:position-management-changed",
            this._managementChangedHandler
          );
          this._managementChangedHandler = null;
        }

        // Clean up action button handlers
        if (this._actionHandlers) {
          this._actionHandlers.forEach(({ element, handler }) => {
            element.removeEventListener("click", handler);
          });
          this._actionHandlers = null;
        }

        this.dialogEl.remove();
        this.dialogEl = null;
      }

      this._resetActivityState();

      this.positionData = null;
      this.fullDetails = null;
      this.currentTab = "overview";
      // The dialog instance is reused for every position, so the timeframe must
      // go back to "derive it from the next position" rather than carrying one
      // position's choice onto another.
      this._chartTimeframe = null;
      this._fetchFailures = 0;
      this.isLoading = false;
      this.isOpening = false;

      this.onClose();
    }, 300);
  }

  /**
   * Destroy dialog completely, cleaning up all resources
   */
  destroy() {
    this._stopPolling();
    this._destroyPositionChart?.();
    this._dialogTabBar?.destroy();
    this._dialogTabBar = null;

    this._releaseEscape?.();
    this._releaseEscape = null;

    if (this.dialogEl) {
      this.dialogEl.remove();
      this.dialogEl = null;
    }

    if (this.tradeDialog) {
      this.tradeDialog = null;
    }

    this._closeHandler = null;
    this._backdropHandler = null;
    this._copyMintHandler = null;
    this._favoriteHandler = null;
    this._resetActivityState();
    this.positionData = null;
    this.fullDetails = null;
  }

  /**
   * Create dialog DOM structure
   */
  _createDialog() {
    this.dialogEl = document.createElement("div");
    this.dialogEl.className = "position-details-dialog";
    this.dialogEl.innerHTML = this._getDialogHTML();
    document.body.appendChild(this.dialogEl);
  }

  /**
   * Get initial dialog HTML
   */
  _getDialogHTML() {
    const pos = this.positionData;
    const symbol = pos.symbol || "Unknown";
    const name = pos.name || "Unknown Token";
    const logoUrl = pos.logo_url || "";
    const isOpen = pos.position_type !== "closed";
    const tabs = this._getDialogTabs();
    const tabActions = `
      <div class="pdd-trade-actions">
        <span class="pdd-trade-actions-label">${isOpen ? "Manage position" : "Position"}</span>
        ${
          isOpen
            ? `
          <button class="pdd-trade-btn pdd-trade-add" id="pddAddBtn"><i class="icon-circle-plus"></i> Add</button>
          <button class="pdd-trade-btn pdd-trade-partial" id="pddPartialBtn"><i class="icon-scissors"></i> Partial</button>
          <button class="pdd-trade-btn pdd-trade-close" id="pddCloseBtn"><i class="icon-circle-x"></i> Close</button>
        `
            : `
          <button class="pdd-trade-btn pdd-trade-view" id="pddViewTokenBtn"><i class="icon-external-link"></i> Token Details</button>
        `
        }
      </div>
    `;
    const statusBadge = isOpen
      ? '<span class="pdd-badge pdd-position-status pdd-badge-success">Open</span>'
      : '<span class="pdd-badge pdd-position-status pdd-badge-secondary">Closed</span>';
    const manualHint = this._renderManualHint();
    const managementLabels = {
      auto_trader: "Auto Trader",
      user_only: "User Only",
      copy_task: "Copy Task",
      hybrid: "Hybrid",
    };
    const origin = pos.origin || { kind: "auto" };
    const sourceWallet = String(origin.source_wallet || "unknown");
    const shortSourceWallet =
      sourceWallet.length > 14
        ? `${sourceWallet.slice(0, 6)}…${sourceWallet.slice(-4)}`
        : sourceWallet;
    const originLabel =
      origin.kind === "copy"
        ? `Copied · task ${Utils.escapeHtml(String(origin.task_id ?? "unknown"))} · ${Utils.escapeHtml(shortSourceWallet)}`
        : origin.kind === "manual"
          ? "Manual entry"
          : origin.strategy_id
            ? `Auto · ${Utils.escapeHtml(origin.strategy_id)}`
            : "Auto entry";
    const originBadge = `<span class="pdd-badge pdd-badge-secondary" title="Position origin">${originLabel}</span>`;
    const availableManagementLabels =
      origin.kind === "copy"
        ? managementLabels
        : Object.fromEntries(
            Object.entries(managementLabels).filter(([value]) =>
              ["auto_trader", "user_only"].includes(value)
            )
          );
    const manualBadge = isOpen
      ? `<label class="pdd-badge pdd-manual-toggle"><i class="icon-shield"></i><select id="pddManagementSelect" aria-label="Position management" data-custom-select>${Object.entries(
          availableManagementLabels
        )
          .map(
            ([value, label]) =>
              `<option value="${value}"${pos.management === value ? " selected" : ""}>${label}</option>`
          )
          .join("")}</select></label>${manualHint}`
      : `<span class="pdd-badge pdd-badge-secondary"><i class="icon-shield"></i> ${
          managementLabels[pos.management] || pos.management
        }</span>`;

    return `
      <div class="dialog-backdrop"></div>
      <div class="dialog-container">
        <div class="dialog-header">
          <div class="header-top-row">
            <div class="header-left">
              <div class="header-logo">
                ${logoUrl ? `<img src="${Utils.escapeHtml(logoUrl)}" alt="${Utils.escapeHtml(symbol)}" onerror="this.parentElement.innerHTML='<div class=\\'logo-placeholder\\'>${Utils.escapeHtml(symbol.charAt(0))}</div>'" />` : `<div class="logo-placeholder">${Utils.escapeHtml(symbol.charAt(0))}</div>`}
              </div>
              <div class="header-identity">
                <div class="header-title-row">
                  <div class="header-title">
                    <span class="title-main" id="pdd-dialog-title">${Utils.escapeHtml(name)}</span>
                    <span class="title-symbol token-symbol-type">$${Utils.escapeHtml(symbol.toUpperCase())}</span>
                  </div>
                  <div class="header-security" id="pddHeaderSecurity"></div>
                </div>
                <div class="header-token-context-row" id="pddHeaderPool"></div>
                <div class="header-mint-full" title="${Utils.escapeHtml(pos.mint)}">${Utils.escapeHtml(pos.mint)}</div>
              </div>
            </div>
            <div class="header-center">
              <div class="header-price" id="pddHeaderPrice">
                <div class="price-loading">Loading...</div>
              </div>
            </div>
            <div class="header-right">
              <div class="dialog-header-actions">
                <button class="dialog-header-action favorite-btn" id="pddFavoriteBtn" type="button" title="Add to Favorites">
                  <i class="icon-star"></i>
                </button>
                <button class="dialog-header-action" id="pddCopyMintBtn" type="button" title="Copy Mint Address">
                  <i class="icon-copy"></i>
                </button>
                <a href="https://solscan.io/token/${Utils.escapeHtml(pos.mint)}" target="_blank" rel="noopener" class="dialog-header-action" title="View on Solscan" aria-label="View token on Solscan">
                  <i class="icon-external-link"></i>
                </a>
              </div>
              <button class="dialog-close" type="button" title="Close (ESC)">
                <i class="icon-x"></i>
              </button>
            </div>
            <div class="header-lower-row">
              <div class="header-badges">
                ${statusBadge}
                ${originBadge}
                ${manualBadge}
                <span class="pdd-pending-swaps" id="pddPendingSwaps"></span>
              </div>
              <div class="header-shortcuts" id="pddHeaderShortcuts"></div>
            </div>
          </div>
        </div>
        ${renderDialogTabRow({
          tabs,
          activeTab: this.currentTab,
          idPrefix: "position-details",
          ariaLabel: "Position details sections",
          actionsHtml: tabActions,
        })}

        <div class="dialog-body">
          <div class="tab-content active" data-tab-content="overview">
            <div class="loading-spinner">Loading position details...</div>
          </div>
          <div class="tab-content" data-tab-content="chart">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="activity">
            <div class="loading-spinner">Loading...</div>
          </div>
        </div>
      </div>
    `;
  }

  _getDialogTabs() {
    return [
      { id: "overview", label: "Overview", icon: "icon-info" },
      { id: "chart", label: "Chart", icon: "icon-chart-bar" },
      { id: "activity", label: "Activity", icon: "icon-activity" },
    ];
  }

  /**
   * Update header with current position data
   */
  _updateHeader() {
    const pos = this.fullDetails?.position;
    if (!pos) return;

    const priceContainer = this.dialogEl?.querySelector("#pddHeaderPrice");
    if (priceContainer) {
      priceContainer.innerHTML = this._buildHeaderPrice(pos);
    }

    const securityContainer = this.dialogEl?.querySelector("#pddHeaderSecurity");
    if (securityContainer) securityContainer.innerHTML = this._buildHeaderSecurity();

    const poolContainer = this.dialogEl?.querySelector("#pddHeaderPool");
    if (poolContainer) poolContainer.innerHTML = this._buildHeaderPoolContext();

    const shortcutsContainer = this.dialogEl?.querySelector("#pddHeaderShortcuts");
    if (shortcutsContainer) shortcutsContainer.innerHTML = this._buildHeaderShortcuts();

    const pendingContainer = this.dialogEl?.querySelector("#pddPendingSwaps");
    if (pendingContainer) pendingContainer.innerHTML = this._buildPendingSwaps();
  }

  /**
   * Swaps this position has submitted but not booked yet.
   *
   * A manual add returns success as soon as the swap is SUBMITTED; the position's invested
   * total, average entry and DCA count only move when the verifier applies it — seconds
   * later, with no event of its own. Without this the dialog read as broken: the toast
   * said "Added to position!" and every figure still showed the pre-trade state. The
   * amounts here are what was SENT, never what the position has booked.
   */
  _buildPendingSwaps() {
    const pending = this.fullDetails?.pending_swaps || [];
    if (!pending.length) return "";

    return pending
      .map((swap) => {
        let label;
        if (swap.kind === "dca") {
          const size =
            swap.size_sol != null
              ? ` ${Utils.formatSol(swap.size_sol, { decimals: 4, suffix: "" })} SOL`
              : "";
          label = `Adding${size}`;
        } else {
          const pct =
            swap.exit_percentage != null ? ` ${Utils.formatNumber(swap.exit_percentage, 0)}%` : "";
          label = `Selling${pct}`;
        }
        return `<span class="pdd-badge pdd-badge-pending" title="Submitted — waiting for on-chain confirmation. The position's figures update once it is verified."><i class="icon-loader spin"></i> ${Utils.escapeHtml(label)} · confirming</span>`;
      })
      .join("");
  }

  _buildHeaderSecurity() {
    const security = this.fullDetails?.security;
    if (!security) return "";

    const score = security.score_normalized;
    const riskLevel = security.risk_level || "unknown";
    const riskClass = this._getRiskLevelClass(riskLevel);
    const riskLabel = this._getRiskLevelLabel(riskLevel);
    const scoreStr = score !== null && score !== undefined ? ` · ${score}/100` : "";
    const authorityBadges = [];
    if (security.has_mint_authority) {
      authorityBadges.push(
        '<span class="meta-authority-badge"><i class="icon-triangle-alert"></i> Mint Auth</span>'
      );
    }
    if (security.has_freeze_authority) {
      authorityBadges.push(
        '<span class="meta-authority-badge"><i class="icon-triangle-alert"></i> Freeze Auth</span>'
      );
    }

    return `<div class="meta-security-group"><span class="meta-security-badge ${riskClass}"><i class="icon-shield"></i> ${riskLabel}${scoreStr}</span>${authorityBadges.join("")}</div>`;
  }

  _buildHeaderPoolContext() {
    const poolInfo = this.fullDetails?.pool_info;
    const dexBadge = poolInfo?.dex_name
      ? `<span class="pdd-dex-badge">${Utils.escapeHtml(poolInfo.dex_name)}</span>`
      : "";
    const poolLiqHtml =
      poolInfo?.liquidity_sol !== null && poolInfo?.liquidity_sol !== undefined
        ? `<span class="meta-metric"><span class="meta-metric-label">Liq</span><span class="meta-metric-value">${Utils.formatSol(poolInfo.liquidity_sol, { decimals: 2, suffix: "" })} SOL</span></span>`
        : "";

    return dexBadge || poolLiqHtml
      ? `<div class="meta-token-group">${dexBadge}${poolLiqHtml}</div>`
      : "";
  }

  _buildHeaderShortcuts() {
    const tokenInfo = this.fullDetails?.token_info;
    const links = this.fullDetails?.external_links || {};
    const shortcuts = [
      ["Website", tokenInfo?.website, "icon-globe"],
      ["X", tokenInfo?.twitter, "icon-twitter"],
      ["Telegram", tokenInfo?.telegram, "icon-send"],
      ["Solscan", links.solscan, "icon-external-link"],
      ["DexScreener", links.dexscreener, "icon-external-link"],
      ["Birdeye", links.birdeye, "icon-external-link"],
      ["RugCheck", links.rugcheck, "icon-external-link"],
      ["Photon", links.photon, "icon-external-link"],
    ];

    return shortcuts
      .filter(([, url]) => url)
      .map(
        ([label, url, icon]) =>
          `<a href="${Utils.escapeHtml(url)}" target="_blank" rel="noopener" class="pdd-header-shortcut" title="Open ${label}" aria-label="Open ${label}"><i class="${icon}"></i><span>${label}</span></a>`
      )
      .join("");
  }

  async _checkFavoriteState() {
    const mint = this.positionData?.mint;
    if (!mint) return;
    try {
      const response = await fetch("/api/tokens/favorites");
      if (!response.ok) return;
      const data = await response.json();
      const favorites = data.favorites || [];
      this._updateFavoriteButton(favorites.some((favorite) => favorite.mint === mint));
    } catch {
      // Best-effort state check; the button remains available if this request fails.
    }
  }

  _updateFavoriteButton(isFavorite) {
    const button = this.dialogEl?.querySelector("#pddFavoriteBtn");
    if (!button) return;
    button.classList.toggle("active", isFavorite);
    button.title = isFavorite ? "Remove from Favorites" : "Add to Favorites";
  }

  async _toggleFavorite() {
    const button = this.dialogEl?.querySelector("#pddFavoriteBtn");
    const position = this.fullDetails?.position || this.positionData;
    if (!button || button.disabled || !position?.mint) return;

    const currentlyFavorite = button.classList.contains("active");
    button.disabled = true;
    try {
      if (currentlyFavorite) {
        const response = await fetch(`/api/tokens/favorites/${encodeURIComponent(position.mint)}`, {
          method: "DELETE",
        });
        if (!response.ok) throw new Error("Failed to remove favorite");
      } else {
        const response = await fetch("/api/tokens/favorites", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            mint: position.mint,
            symbol: position.symbol || null,
            name: position.name || null,
            logo_url: position.logo_url || null,
          }),
        });
        if (!response.ok) throw new Error("Failed to add favorite");
      }

      const isFavorite = !currentlyFavorite;
      this._updateFavoriteButton(isFavorite);
      Utils.showToast(
        `${position.symbol || "Token"} ${isFavorite ? "added to" : "removed from"} favorites`,
        "success"
      );
      window.dispatchEvent(
        new CustomEvent("screenerbot:favorites-changed", {
          detail: { mint: position.mint, isFavorite },
        })
      );
    } catch (error) {
      Utils.showToast(error.message || "Failed to update favorites", "error");
    } finally {
      button.disabled = false;
    }
  }

  /**
   * Build header price section HTML
   * Note: Position data is flattened (no summary wrapper) due to serde(flatten) on backend
   */
  _buildHeaderPrice(pos) {
    const currentPrice = pos?.current_price;
    const entryPrice = pos?.average_entry_price || pos?.entry_price;
    const isOpen = pos?.position_type !== "closed";
    const pnl = isOpen ? pos?.unrealized_pnl : pos?.pnl;
    const pnlPct = isOpen ? pos?.unrealized_pnl_percent : pos?.pnl_percent;
    const pnlPositive = pnl != null && pnl >= 0;
    const pnlTone = pnl == null ? "" : pnlPositive ? "pdd-positive" : "pdd-negative";
    const pnlSign = pnlPositive ? "+" : "";
    const pnlAbs = Math.abs(pnl || 0);
    const pnlDecimals = pnlAbs > 0 && pnlAbs < 0.0001 ? 8 : pnlAbs < 0.01 ? 6 : 4;
    const pnlValue =
      pnl == null
        ? "—"
        : `${pnlSign}${Utils.formatSol(pnl, { decimals: pnlDecimals, suffix: "" })}`;
    const pnlPercent = pnlPct == null ? "" : `${pnlSign}${Utils.formatNumber(pnlPct, 2)}%`;

    const metric = (label, value, unit = "", sub = "", className = "") => `
      <div class="header-metric ${className}">
        <span class="header-metric-label">${label}</span>
        <span class="header-metric-value">${value}${unit ? `<small>${unit}</small>` : ""}</span>
        <span class="header-metric-sub">${sub || "&nbsp;"}</span>
      </div>`;

    return `
      ${metric("Current price", currentPrice != null ? this._formatPrice(currentPrice) : "—", "SOL")}
      ${metric(isOpen ? "Unrealized P&L" : "Realized P&L", pnlValue, "SOL", pnlPercent, pnlTone)}
      ${metric("Average entry", entryPrice != null ? this._formatPrice(entryPrice) : "—", "SOL")}
      ${metric("Invested", Utils.formatSol(pos?.total_size_sol, { decimals: 4, suffix: "" }), "SOL")}
    `;
  }

  /**
   * Attach event handlers
   */
  _attachEventHandlers() {
    const closeBtn = this.dialogEl.querySelector(".dialog-close");
    this._closeHandler = () => this.close();
    closeBtn.addEventListener("click", this._closeHandler);

    const backdrop = this.dialogEl.querySelector(".dialog-backdrop");
    this._backdropHandler = () => this.close();
    backdrop.addEventListener("click", this._backdropHandler);

    const copyMintButton = this.dialogEl.querySelector("#pddCopyMintBtn");
    this._copyMintHandler = () => {
      Utils.copyToClipboard(this.positionData.mint);
      Utils.showToast("Mint address copied", "success");
    };
    copyMintButton.addEventListener("click", this._copyMintHandler);

    const favoriteButton = this.dialogEl.querySelector("#pddFavoriteBtn");
    this._favoriteHandler = () => this._toggleFavorite();
    favoriteButton.addEventListener("click", this._favoriteHandler);

    // Escape goes through the shared stack, so the topmost overlay wins. Bound directly
    // on `document`, this dialog closed underneath the trade dialog opened from its own
    // Add/Partial/Close buttons on a single Escape.
    this._releaseEscape = pushEscapeHandler(() => this.close());

    // Trade action buttons (now in the top tab bar). Bound once; pos is resolved
    // fresh at click time so it always reflects the latest details.
    this._attachTradeButtons();

    this._dialogTabBar = new DialogTabBar({
      root: this.dialogEl,
      tabs: this._getDialogTabs(),
      activeTab: this.currentTab,
      beforeChange: (tabId, previousTab) => {
        if (previousTab === "chart" && tabId !== "chart") {
          this._destroyPositionChart?.();
        }
        return true;
      },
      onChange: (tabId) => {
        this.currentTab = tabId;
        this._loadTabContent(tabId);
      },
    });

    // Manual-management toggle (open positions). Dispatches the shared toggle event;
    // the global handler does the POST + toast and broadcasts the change back.
    const manualToggle = this.dialogEl.querySelector("#pddManagementSelect");
    if (manualToggle) {
      this._manualToggleHandler = () => {
        window.dispatchEvent(
          new CustomEvent("screenerbot:toggle-position-management", {
            detail: {
              id: this.positionData.id,
              mint: this.positionData.mint,
              management: manualToggle.value,
            },
          })
        );
      };
      manualToggle.addEventListener("change", this._manualToggleHandler);
    }

    // Reflect management changes (from here or the row context menu) in this dialog.
    this._managementChangedHandler = (event) => {
      const { id, management } = event.detail || {};
      if (id == null || id !== this.positionData?.id) return;
      this.positionData.management = management;
      this._refreshManualToggle();
    };
    window.addEventListener(
      "screenerbot:position-management-changed",
      this._managementChangedHandler
    );

    // Activate the hint trigger's delegated click handler.
    HintTrigger.initAll();
  }

  /** Render the position-management hint trigger HTML (empty if hints are off). */
  _renderManualHint() {
    const hint = Hints.getHint("positions.positionManagement");
    if (!hint) return "";
    return HintTrigger.render(hint, "positions.positionManagement", {
      size: "sm",
      position: "bottom",
    });
  }

  /** Update the toggle button's state/label in place after a management change. */
  _refreshManualToggle() {
    const select = this.dialogEl?.querySelector("#pddManagementSelect");
    if (select) {
      select.value = this.positionData.management;
    }
  }

  /**
   * Load content for a specific tab
   */
  _loadTabContent(tabId) {
    const content = this.dialogEl?.querySelector(`[data-tab-content="${tabId}"]`);
    if (!content) return;

    if (!this.fullDetails) {
      content.innerHTML = '<div class="loading-spinner">Loading position details...</div>';
      return;
    }

    switch (tabId) {
      case "overview":
        this._renderOverviewTab(content);
        break;
      case "chart":
        this._renderChartTab(content);
        break;
      case "activity":
        // The token's all-time activity lives behind its own endpoint. Paint whatever we
        // already have (guarded, so an expanded card survives a poll), then fetch it the
        // first time the tab is opened.
        this._paintActivity(content);
        if (!this._activity && !this._activityLoading) this._fetchActivity();
        break;
    }
  }

  // ===========================================================================
  // OVERVIEW TAB — applyOverviewTabMixin()
  // CHART TAB    — applyChartTabMixin()
  // ACTIVITY TAB — applyActivityTabMixin()
  // UTILITIES    — applyUtilitiesMixin()
  // ===========================================================================
}

// Apply mixins to add tab rendering methods
applyOverviewTabMixin(PositionDetailsDialog);
applyChartTabMixin(PositionDetailsDialog);
applyActivityTabMixin(PositionDetailsDialog);
applyUtilitiesMixin(PositionDetailsDialog);

// ============================================================================
// Global Event Listener for Context Menu "View Details" Action (Positions)
// ============================================================================
// This listener allows any page to open the PositionDetailsDialog via custom event
// dispatched from context_menu.js when user clicks "View Details" on a position row

let globalPositionDialogInstance = null;

window.addEventListener("screenerbot:open-position-details", async (event) => {
  const { id, mint, symbol, position_type } = event.detail || {};

  if (!id && !mint) {
    console.error("[PositionDetailsDialog] Event received without id or mint");
    return;
  }

  console.log(`[PositionDetailsDialog] Opening details for position ${id || mint}`);

  // Close existing dialog if open for a different position
  if (globalPositionDialogInstance && globalPositionDialogInstance.dialogEl) {
    const currentId = globalPositionDialogInstance.positionData?.id;
    const currentMint = globalPositionDialogInstance.positionData?.mint;
    if ((id && currentId === id) || (!id && currentMint === mint)) {
      // Already open for this position, do nothing
      console.log("[PositionDetailsDialog] Dialog already open for this position");
      return;
    }
    globalPositionDialogInstance.close();
    await new Promise((resolve) => setTimeout(resolve, 350));
  }

  // Create new dialog instance if needed
  if (!globalPositionDialogInstance) {
    globalPositionDialogInstance = new PositionDetailsDialog({
      onClose: () => {
        // Keep instance for reuse, just clean up state
      },
    });
  }

  // Open dialog with position data (dialog will fetch full details)
  await globalPositionDialogInstance.show({
    id,
    mint,
    symbol: symbol || "",
    position_type: position_type || "open",
  });
});

// ============================================================================
// Global Manual-Management Toggle Handler
// ============================================================================
// Single place that performs the toggle so it works from any page (the positions
// context menu and this dialog both dispatch `screenerbot:toggle-position-management`).
// On success it broadcasts `screenerbot:position-management-changed` so an open
// positions table / dialog can refresh.
window.addEventListener("screenerbot:toggle-position-management", async (event) => {
  const { id, management } = event.detail || {};
  if (id == null) return;

  try {
    const data = await requestManager.fetch(`/api/positions/${encodeURIComponent(id)}/management`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ management }),
      priority: "high",
    });
    if (data && data.success === false) {
      throw new Error(data.message || "Request failed");
    }
    Utils.showToast(data?.message || `Position management set to ${management}`, "success");
    window.dispatchEvent(
      new CustomEvent("screenerbot:position-management-changed", {
        detail: { id, management },
      })
    );
  } catch (err) {
    Utils.showToast(err?.message || "Failed to update position management", "error");
  }
});
