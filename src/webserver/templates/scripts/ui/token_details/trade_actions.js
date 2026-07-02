/**
 * Token Details Dialog - Trade Actions Mixin
 * Extracted from token_details_dialog.js to reduce file size
 * Handles buy/sell button actions and trade dialog integration
 */
import * as Utils from "../../core/utils.js";
import { TradeActionDialog } from "../trade_action_dialog.js";
import { requestManager } from "../../core/request_manager.js";

/**
 * Apply trade actions mixin to TokenDetailsDialog class
 * @param {class} DialogClass - TokenDetailsDialog class
 */
export function applyTradeActionsMixin(DialogClass) {
  const proto = DialogClass.prototype;

  /**
   * Ensure trade dialog instance exists
   * @private
   */
  proto._ensureTradeDialog = function () {
    if (!this.tradeDialog) {
      this.tradeDialog = new TradeActionDialog();
    }
    return this.tradeDialog;
  };

  /**
   * Get wallet balance with 10s cache
   * @private
   * @returns {Promise<number>} Wallet balance in SOL
   */
  proto._getWalletBalance = async function () {
    const now = Date.now();
    if (this.walletBalance != null && now - this.walletBalanceFetchedAt < 10000) {
      return this.walletBalance;
    }

    try {
      const data = await requestManager.fetch("/api/wallet/balance", { priority: "low" });
      const parsedBalance = Number(data?.sol_balance);
      if (Number.isFinite(parsedBalance)) {
        this.walletBalance = parsedBalance;
        this.walletBalanceFetchedAt = now;
        return this.walletBalance;
      }
    } catch (error) {
      console.warn("[TokenDetailsDialog] Failed to fetch wallet balance", error);
    }

    this.walletBalance = 0;
    this.walletBalanceFetchedAt = now;
    return this.walletBalance;
  };

  /**
   * Handle buy button click
   * @private
   */
  proto._handleBuyClick = async function () {
    const dialog = this._ensureTradeDialog();
    const symbol = this.fullTokenData?.symbol || this.tokenData?.symbol || "?";
    const mint = this.tokenData?.mint;
    const balance = await this._getWalletBalance();

    if (!mint) {
      Utils.showToast("No mint address available", "error");
      return;
    }

    try {
      const result = await dialog.open({
        action: "buy",
        symbol,
        context: { balance, mint },
      });

      if (!result) return; // User cancelled

      const buyBtn = this.dialogEl.querySelector("#headerBuyBtn");
      if (buyBtn) buyBtn.disabled = true;

      const response = await fetch("/api/trader/manual/buy", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          // Use the mint captured before the dialog opened: this.tokenData can
          // be nulled while the dialog is open (dialog close resets it), and
          // re-reading it here threw "Cannot read properties of null".
          mint,
          ...(result.amount ? { size_sol: result.amount } : {}),
        }),
      });

      if (buyBtn) buyBtn.disabled = false;

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.message || "Buy failed");
      }

      Utils.showToast("Buy order placed!", "success");
      this._refreshPositionsData();
      this.onTradeComplete("buy", mint);
    } catch (error) {
      Utils.showToast(error.message || "Buy failed", "error");
    }
  };

  /**
   * Handle sell button click
   * @private
   */
  proto._handleSellClick = async function () {
    const dialog = this._ensureTradeDialog();
    const symbol = this.fullTokenData?.symbol || this.tokenData?.symbol || "?";
    const mint = this.tokenData?.mint;

    if (!mint) {
      Utils.showToast("No mint address available", "error");
      return;
    }

    // Get holdings for sell percentage calculation
    const holdings = this.fullTokenData?.holdings || 0;

    try {
      const result = await dialog.open({
        action: "sell",
        symbol,
        context: { mint, holdings },
      });

      if (!result) return; // User cancelled

      const sellBtn = this.dialogEl.querySelector("#headerSellBtn");
      if (sellBtn) sellBtn.disabled = true;

      // Use the mint captured before the dialog opened (this.tokenData may be
      // nulled while the dialog is open).
      const body =
        result.percentage === 100
          ? { mint, close_all: true }
          : { mint, percentage: result.percentage };

      const response = await fetch("/api/trader/manual/sell", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (sellBtn) sellBtn.disabled = false;

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.message || "Sell failed");
      }

      Utils.showToast("Sell order placed!", "success");
      this._refreshPositionsData();
      this.onTradeComplete("sell", mint);
    } catch (error) {
      Utils.showToast(error.message || "Sell failed", "error");
    }
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
   * Switch to a different tab
   * @private
   * @param {string} tabId - Tab identifier
   */
  proto._switchTab = function (tabId) {
    if (tabId === this.currentTab) return;

    const tabButtons = this.dialogEl.querySelectorAll(".tab-button");
    tabButtons.forEach((btn) => {
      if (btn.dataset.tab === tabId) {
        btn.classList.add("active");
      } else {
        btn.classList.remove("active");
      }
    });

    const tabContents = this.dialogEl.querySelectorAll(".tab-content");
    tabContents.forEach((content) => {
      if (content.dataset.tabContent === tabId) {
        content.classList.add("active");
      } else {
        content.classList.remove("active");
      }
    });

    if (this.currentTab === "overview" && tabId !== "overview") {
      this._stopChartPolling();
    }

    if (tabId === "overview" && this.advancedChart) {
      this._startChartPolling();
    }

    this.currentTab = tabId;
    this._loadTabContent(tabId);
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
