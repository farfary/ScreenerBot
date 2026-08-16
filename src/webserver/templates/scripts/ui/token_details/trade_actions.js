/**
 * Token Details Dialog - Trade Actions Mixin
 * Extracted from token_details_dialog.js to reduce file size
 * Handles buy/sell button actions and trade dialog integration
 */
import { manualTrade } from "../manual_trade.js";

/**
 * Apply trade actions mixin to TokenDetailsDialog class
 * @param {class} DialogClass - TokenDetailsDialog class
 */
export function applyTradeActionsMixin(DialogClass) {
  const proto = DialogClass.prototype;

  /**
   * Handle buy button click
   *
   * The trade itself runs through the shared manual-trade flow (ui/manual_trade.js),
   * which owns the dialog, the payload and the toasts. This used to be a local copy
   * that silently dropped the dialog's ownership choice, so a manual buy
   * from here could be picked up and auto-sold by the trader.
   * @private
   */
  proto._handleBuyClick = async function () {
    // Capture the mint BEFORE any await: this.tokenData is nulled when the dialog
    // closes, and re-reading it after the trade dialog resolves threw.
    const mint = this.tokenData?.mint;
    const symbol = this.fullTokenData?.symbol || this.tokenData?.symbol || "?";

    const placed = await manualTrade({
      action: "buy",
      mint,
      symbol,
      btn: this.dialogEl?.querySelector("#headerBuyBtn"),
    });

    if (!placed) return;

    this._refreshPositionsData();
    this.onTradeComplete("buy", mint);
  };

  /**
   * Handle sell button click
   * @private
   */
  proto._handleSellClick = async function () {
    const mint = this.tokenData?.mint;
    const symbol = this.fullTokenData?.symbol || this.tokenData?.symbol || "?";

    const placed = await manualTrade({
      action: "sell",
      mint,
      symbol,
      // No holdings passed on purpose: manualTrade resolves them from the position.
      // `fullTokenData.holdings` is not a field the token API returns, so this only
      // ever forwarded 0.
      btn: this.dialogEl?.querySelector("#headerSellBtn"),
    });

    if (!placed) return;

    this._refreshPositionsData();
    this.onTradeComplete("sell", mint);
  };

  /**
   * Refresh positions data after trade
   * @private
   */
  proto._refreshPositionsData = async function () {
    // Refresh positions tab if loaded
    const positionsContent = this.dialogEl?.querySelector('[data-tab-content="positions"]');
    if (positionsContent) {
      positionsContent.dataset.loaded = "false";
      if (this.currentTab === "positions") {
        this._loadPositionsTab(positionsContent);
      }
    }
    // Refresh token data to update has_open_position
    await this._fetchTokenData();
  };

  /**
   * Load tab content on demand
   * @private
   * @param {string} tabId - Tab identifier
   */
  proto._loadTabContent = function (tabId) {
    const content = this.dialogEl.querySelector(`[data-tab-content="${tabId}"]`);
    if (!content) return;

    // Security re-evaluates on every switch: its loader is idempotent (repaints
    // only on real change), so this keeps it current without re-flashing.
    if (tabId !== "security" && content.dataset.loaded === "true") return;

    switch (tabId) {
      case "overview":
        this._loadOverviewTab(content);
        break;
      case "security":
        this._loadSecurityTab(content);
        break;
      case "positions":
        this._loadPositionsTab(content);
        break;
      case "pools":
        this._loadPoolsTab(content);
        break;
      case "links":
        this._loadLinksTab(content);
        break;
      case "transactions":
        this._loadTransactionsTab(content);
        break;
    }
  };
}
