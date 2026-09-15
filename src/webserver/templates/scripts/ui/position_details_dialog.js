/**
 * Position Details Dialog
 *
 * One full-screen view of a position. The header carries identity, the four headline figures
 * and the controls that act on the position; the body sets the position chart and the
 * token's activity as two fixed panes (panes.js) beside a summary rail. There are no sub-tabs
 * and no page scroll: everything the position has to say is on screen together.
 */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { Poller } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import { notificationManager } from "../core/notifications.js";
import { pushEscapeHandler } from "../core/escape_stack.js";
import { HintTrigger } from "./hint_popover.js";
import { applyHeaderMixin } from "./position_details/header.js";
import { applySummaryMixin } from "./position_details/summary.js";
import { applyChartMixin } from "./position_details/chart.js";
import { applyActivityMixin } from "./position_details/activity.js";
import { applyPanesMixin } from "./position_details/panes.js";
import { applyUtilitiesMixin } from "./position_details/utilities.js";

// Refresh cadence. `/details` is a heavy endpoint (full token assembly, decimals batch, pool
// lookup, two history queries, SOL price) and the chart refreshes off the same tick, so it
// does not belong on the global 1s dashboard interval.
const POLL_IDLE_MS = 2000;
// While a swap is submitted but unverified the position's numbers are ABOUT to change and
// nothing else will announce it — verification emits no action event — so poll tighter.
const POLL_PENDING_MS = 1000;

export class PositionDetailsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.onTradeComplete = options.onTradeComplete || (() => {});
    this.dialogEl = null;
    this.positionData = null;
    this.fullDetails = null;
    this.isLoading = false;
    this.isOpening = false;
    this.refreshPoller = null;
    // Chosen from the position's own duration on first render (see chart.js), so a week-old
    // position does not open on a timeframe that shows ten hours.
    this._chartTimeframe = null;
    this._releaseEscape = null;
    this._clickHandler = null;
    this._changeHandler = null;
    this._managementChangedHandler = null;
    this._liveTradeUnsub = null;
    this._focusTrap = null;
    this._pollMs = null;
    this._fetchFailures = 0;
    // Incremented per open, so a response that lands after the dialog moved on to another
    // position is dropped instead of painted onto it.
    this._openSeq = 0;
    // Last markup painted per region (see _paintRegion): a poll that changed nothing
    // repaints nothing.
    this._renderKeys = {};
    this._resetActivityState();
  }

  /**
   * Show the dialog for a position.
   * @param {Object} positionData - Position row (at minimum an id or a mint)
   */
  async show(positionData) {
    if (!positionData || (!positionData.id && !positionData.mint)) {
      console.error("Invalid position data provided to PositionDetailsDialog");
      return;
    }
    if (this.isOpening) return;

    if (this.dialogEl) {
      this.close();
      await new Promise((resolve) => setTimeout(resolve, 350));
    }

    this.isOpening = true;
    try {
      this._openSeq += 1;
      this.positionData = positionData;
      this.fullDetails = null;
      this._renderKeys = {};
      this._resetActivityState();

      this._createDialog();
      this._attachEventHandlers();
      // Paint from the row the dialog was opened with; the details response replaces it.
      this._renderHeader();
      this._paintActivity();
      this._checkFavoriteState();

      requestAnimationFrame(() => {
        if (!this.dialogEl) return;
        this.dialogEl.classList.add("active");
        this._focusTrap = createFocusTrap(this.dialogEl);
        this._focusTrap.activate();
      });

      await this._fetchDetails();
      this._startPolling();
    } finally {
      this.isOpening = false;
    }
  }

  /** Fetch the full position details and repaint every region from them. */
  async _fetchDetails() {
    if (this.isLoading || !this.positionData) return;
    this.isLoading = true;
    const seq = this._openSeq;

    try {
      const data = await requestManager.fetch(`/api/positions/${this._getPositionKey()}/details`, {
        priority: "high",
      });
      if (seq !== this._openSeq || !this.dialogEl) return;

      this.fullDetails = data;
      this._fetchFailures = 0;
      this._updateDialogContent();
      this._syncPollCadence();
    } catch (error) {
      console.error("Error loading position details:", error);
      // A poll that fails once must not wipe a working view; a view that is stale for good is
      // worse than an error, though, so keep counting.
      this._fetchFailures += 1;
      if (seq === this._openSeq && (!this.fullDetails || this._fetchFailures >= 3)) {
        this._showBodyState("Failed to load position details");
      }
    } finally {
      this.isLoading = false;
    }
  }

  /** Position key for API requests (`id:123` or `mint:address`). */
  _getPositionKey() {
    if (this.positionData.id) return `id:${this.positionData.id}`;
    return `mint:${this.positionData.mint}`;
  }

  _updateDialogContent() {
    if (!this.fullDetails || !this.dialogEl) return;
    this._hideBodyState();
    this._renderHeader();
    this._renderSummary();
    this._renderChart();
    this._syncActivity();
  }

  /** Cadence this position warrants right now, or null once it can no longer change. */
  _desiredPollMs() {
    const pos = this._position();
    const status = this._status();
    const closing = Boolean(pos?.exit_transaction_signature && !pos?.transaction_exit_verified);
    // A closed or archived position does not move; one whose exit is still confirming does.
    if (status && status !== "open" && !closing) return null;
    return this.fullDetails?.pending_swaps?.length || closing ? POLL_PENDING_MS : POLL_IDLE_MS;
  }

  _startPolling() {
    this._stopPolling();
    if (!this.dialogEl) return;
    const intervalMs = this._desiredPollMs();
    if (intervalMs === null) return;

    this._pollMs = intervalMs;
    this.refreshPoller = new Poller(
      () => {
        this._fetchDetails();
      },
      { label: "PositionDetails", intervalMs }
    );
    this.refreshPoller.start();

    // React immediately to live buy/sell action events (SSE) so the trade controls disable
    // and enable, and pull fresh details: a manual trade's action reaches "completed" when the
    // swap is SUBMITTED, which is exactly when the pending swap appears on the position.
    this._liveTradeUnsub = notificationManager.subscribe((event) => {
      this._updateTradeButtonsState();
      const mint = this._position()?.mint;
      const eventMint = event?.notification?.entity_id || event?.notification?.metadata?.mint;
      if (mint && eventMint && eventMint !== mint) return;
      this._fetchDetails();
    });
  }

  /**
   * Follow the position between "settled", "a swap is confirming" and "no longer changes".
   * `Poller` reads its interval when it starts, so a new cadence needs a restart.
   */
  _syncPollCadence() {
    if (!this.refreshPoller) return;
    const next = this._desiredPollMs();
    if (next === null) {
      this._stopPolling();
      return;
    }
    if (next === this._pollMs) return;
    this._pollMs = next;
    this.refreshPoller.intervalMs = next;
    this.refreshPoller.start({ silent: true });
  }

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

  /** Cover the body with a notice; used only when there is no position to show at all. */
  _showBodyState(message) {
    const state = this.dialogEl?.querySelector("#pddBodyState");
    if (!state) return;
    state.innerHTML = `
      <div class="pdd-empty-state">
        <i class="icon-circle-alert"></i>
        <p>${Utils.escapeHtml(message)}</p>
      </div>`;
    state.hidden = false;
  }

  _hideBodyState() {
    const state = this.dialogEl?.querySelector("#pddBodyState");
    if (state) state.hidden = true;
  }

  close() {
    if (!this.dialogEl) return;

    this._focusTrap?.deactivate();
    this._focusTrap = null;
    this._stopPolling();
    // The chart owns a lightweight-charts instance, a ResizeObserver and a MutationObserver
    // on <html>; all three go with the dialog, not with a later destroy().
    this._destroyPositionChart();
    // Released before the exit animation: a second Escape during those 300ms would otherwise
    // re-enter close() on a dialog that is already going away.
    this._releaseEscape?.();
    this._releaseEscape = null;
    this._removeWindowListeners();
    this.dialogEl.classList.remove("active");

    const dialogEl = this.dialogEl;
    setTimeout(() => {
      dialogEl.remove();
      if (this.dialogEl !== dialogEl) return;

      this.dialogEl = null;
      this._resetActivityState();
      this.positionData = null;
      this.fullDetails = null;
      this._renderKeys = {};
      // The instance is reused for every position: the next one derives its own timeframe.
      this._chartTimeframe = null;
      this._fetchFailures = 0;
      this.isLoading = false;
      this.isOpening = false;
      this.onClose();
    }, 300);
  }

  /** Destroy the dialog immediately, releasing every resource. */
  destroy() {
    this._stopPolling();
    this._destroyPositionChart();
    this._focusTrap?.deactivate();
    this._focusTrap = null;
    this._releaseEscape?.();
    this._releaseEscape = null;
    this._removeWindowListeners();
    this.dialogEl?.remove();
    this.dialogEl = null;
    this._resetActivityState();
    this.positionData = null;
    this.fullDetails = null;
    this._renderKeys = {};
  }

  _createDialog() {
    this.dialogEl = document.createElement("div");
    this.dialogEl.className = "position-details-dialog";
    this.dialogEl.innerHTML = this._getDialogHTML();
    document.body.appendChild(this.dialogEl);
  }

  /** The static frame. Every region inside it is painted by its own mixin. */
  _getDialogHTML() {
    const mint = Utils.escapeHtml(this.positionData.mint || "");

    return `
      <div class="dialog-backdrop"></div>
      <div class="dialog-container" role="dialog" aria-modal="true" aria-labelledby="pdd-dialog-title">
        <header class="dialog-header">
          <div class="header-top-row">
            <div class="header-left" id="pddIdentity"></div>
            <div class="header-center">
              <div class="header-price" id="pddHeaderPrice"></div>
            </div>
            <div class="header-right">
              <div class="pdd-trade-actions" id="pddTradeActions"></div>
              <div class="dialog-header-actions">
                <button class="dialog-header-action favorite-btn" id="pddFavoriteBtn" type="button" title="Add to favorites" aria-label="Add to favorites">
                  <i class="icon-star"></i>
                </button>
                <button class="dialog-header-action" id="pddCopyMintBtn" type="button" title="Copy mint address" aria-label="Copy mint address">
                  <i class="icon-copy"></i>
                </button>
                <a class="dialog-header-action" href="https://solscan.io/token/${mint}" target="_blank" rel="noopener" title="View on Solscan" aria-label="View token on Solscan">
                  <i class="icon-external-link"></i>
                </a>
                <button class="dialog-close" type="button" title="Close (Esc)" aria-label="Close">
                  <i class="icon-x"></i>
                </button>
              </div>
            </div>
            <div class="header-lower-row">
              <div class="header-badges" id="pddHeaderBadges"></div>
              <div class="pdd-pending-swaps" id="pddPendingSwaps"></div>
            </div>
          </div>
        </header>

        <div class="dialog-body">
          <div class="pdd-layout">
            <div class="pdd-main" data-split="balanced">
              <section class="pdd-chart-section" id="pddChartSection" aria-label="Price chart">
                <div class="loading-spinner">Loading chart...</div>
              </section>
              <section class="pdd-activity" id="pddActivity" aria-label="Activity">
                <div class="pdd-activity-head" id="pddActivityHead">
                  <div class="pdd-split-handle" id="pddSplitHandle" role="separator" tabindex="0" aria-orientation="horizontal" aria-controls="pddChartSection" aria-label="Resize chart and activity" aria-valuemin="0" aria-valuemax="100"></div>
                  <div class="pdd-activity-title">
                    <h3>Activity</h3>
                    <span class="pdd-activity-meta" id="pddActivityMeta"></span>
                  </div>
                  <div class="pdd-activity-controls">
                    <div class="pdd-act-filter-slot" id="pddActivityFilters"></div>
                    <div class="timeframe-buttons" role="group" aria-label="Activity pane">
                      <button type="button" class="timeframe-btn pdd-pane-btn" id="pddActivityToggle" aria-controls="pddActivityBody" aria-expanded="true" title="Expand activity" aria-label="Expand activity"><i class="icon-chevron-up"></i></button>
                    </div>
                  </div>
                </div>
                <div class="pdd-activity-body" id="pddActivityBody">
                  <div id="pddActivityContent"></div>
                </div>
              </section>
            </div>
            <aside class="pdd-rail" id="pddSummary" aria-label="Position summary">
              <div class="loading-spinner">Loading position...</div>
            </aside>
          </div>
          <div class="pdd-body-state" id="pddBodyState" hidden></div>
        </div>
      </div>
    `;
  }

  _attachEventHandlers() {
    // One delegated listener: header regions are repainted, so nothing binds to their nodes.
    this._clickHandler = (event) => this._handleDialogClick(event);
    this.dialogEl.addEventListener("click", this._clickHandler);

    // The custom select dispatches a bubbling `change` on the native select it enhances.
    this._changeHandler = (event) => {
      if (event.target?.id !== "pddManagementSelect") return;
      const pos = this._position();
      window.dispatchEvent(
        new CustomEvent("screenerbot:toggle-position-management", {
          detail: { id: pos?.id, mint: pos?.mint, management: event.target.value },
        })
      );
    };
    this.dialogEl.addEventListener("change", this._changeHandler);

    // Escape goes through the shared stack, so the topmost overlay wins: the trade dialog
    // opened from this dialog's controls closes first.
    this._releaseEscape = pushEscapeHandler(() => this.close());

    // Reflect management changes (from here or the row context menu) in this dialog.
    this._managementChangedHandler = (event) => {
      const { id, management } = event.detail || {};
      if (id == null || id !== this._position()?.id) return;
      if (this.positionData) this.positionData.management = management;
      if (this.fullDetails?.position) this.fullDetails.position.management = management;
      this._renderHeader();
    };
    window.addEventListener(
      "screenerbot:position-management-changed",
      this._managementChangedHandler
    );

    this._bindActivityHandlers();
    this._initPanes();
    // Activate the hint trigger's delegated click handler.
    HintTrigger.initAll();
  }

  _removeWindowListeners() {
    this._teardownPanes();
    if (this._managementChangedHandler) {
      window.removeEventListener(
        "screenerbot:position-management-changed",
        this._managementChangedHandler
      );
      this._managementChangedHandler = null;
    }
  }

  _handleDialogClick(event) {
    const target = event.target;
    if (!(target instanceof Element)) return;

    if (target.classList.contains("dialog-backdrop") || target.closest(".dialog-close")) {
      this.close();
      return;
    }
    if (target.closest("#pddCopyMintBtn")) {
      const mint = this._position()?.mint;
      if (mint) {
        Utils.copyToClipboard(mint);
        Utils.notifyCopied("Mint address");
      }
      return;
    }
    if (target.closest("#pddFavoriteBtn")) {
      this._toggleFavorite();
      return;
    }
    const trade = target.closest("[data-trade-action]");
    if (trade && !trade.disabled) {
      this._handleTradeAction(trade.dataset.tradeAction, trade);
    }
  }
}

applyUtilitiesMixin(PositionDetailsDialog);
applyHeaderMixin(PositionDetailsDialog);
applySummaryMixin(PositionDetailsDialog);
applyChartMixin(PositionDetailsDialog);
applyActivityMixin(PositionDetailsDialog);
applyPanesMixin(PositionDetailsDialog);

// ============================================================================
// Global "open position details" event
// ============================================================================
// Lets any surface (the positions context menu, the token details Positions tab) open the
// dialog without importing it. The lifecycle comes from the details response, so the event
// carries only the position's identity.

let globalPositionDialogInstance = null;

window.addEventListener("screenerbot:open-position-details", async (event) => {
  const { id, mint, symbol } = event.detail || {};

  if (!id && !mint) {
    console.error("[PositionDetailsDialog] Event received without id or mint");
    return;
  }

  if (globalPositionDialogInstance?.dialogEl) {
    const current = globalPositionDialogInstance.positionData;
    if ((id && current?.id === id) || (!id && current?.mint === mint)) return;
    globalPositionDialogInstance.close();
    await new Promise((resolve) => setTimeout(resolve, 350));
  }

  if (!globalPositionDialogInstance) {
    globalPositionDialogInstance = new PositionDetailsDialog();
  }

  await globalPositionDialogInstance.show({ id, mint, symbol: symbol || "" });
});

// ============================================================================
// Global manual-management toggle handler
// ============================================================================
// Single place that performs the toggle so it works from any page (the positions context menu
// and this dialog both dispatch `screenerbot:toggle-position-management`). On success it
// broadcasts `screenerbot:position-management-changed` so an open table or dialog can refresh.
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
