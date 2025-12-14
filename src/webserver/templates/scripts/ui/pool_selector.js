/**
 * Pool Selector Dialog
 * Shows available pools for a token and lets user select one
 */

import { create, on, off } from "../core/dom.js";
import { escapeHtml } from "../core/utils.js";

/**
 * Format number in compact notation (1.2K, 3.4M, etc.)
 */
function formatCompact(num) {
  if (num === null || num === undefined || !Number.isFinite(num)) return "—";
  if (num >= 1_000_000) return (num / 1_000_000).toFixed(1) + "M";
  if (num >= 1_000) return (num / 1_000).toFixed(1) + "K";
  return num.toFixed(0);
}

export class PoolSelector {
  constructor(options = {}) {
    this.onSelect = options.onSelect || (() => {});
    this.onClose = options.onClose || (() => {});
    this.dialog = null;
    this.pools = [];
    this.tokenMint = null;
    this.keydownHandler = null;
  }

  async open(tokenMint) {
    this.tokenMint = tokenMint;

    // Show loading dialog
    this.showDialog();
    this.renderLoading();

    try {
      // Fetch pools from API
      const response = await fetch(`/api/tools/search-pools/${tokenMint}`);
      const data = await response.json();

      if (!response.ok) {
        throw new Error(data.error || `HTTP ${response.status}`);
      }

      if (!data.success || !data.pools || data.pools.length === 0) {
        this.renderError("No pools found for this token");
        return;
      }

      this.pools = data.pools;

      // Render pool list
      this.renderPools(data.pools, tokenMint);
    } catch (error) {
      this.renderError(`Failed to load pools: ${error.message}`);
    }
  }

  showDialog() {
    // Create dialog if not exists
    if (!this.dialog) {
      this.dialog = create("div", { class: "pool-selector-overlay" });
      document.body.appendChild(this.dialog);

      // Setup keyboard handler
      this.keydownHandler = (e) => {
        if (e.key === "Escape") {
          this.close();
        }
      };
      on(document, "keydown", this.keydownHandler);
    }

    this.dialog.innerHTML = `
      <div class="pool-selector-dialog" role="dialog" aria-modal="true" aria-labelledby="pool-selector-title">
        <div class="pool-selector-header">
          <h3 id="pool-selector-title">Select Pool</h3>
          <button class="pool-selector-close" type="button" aria-label="Close">
            <i class="icon-x"></i>
          </button>
        </div>
        <div class="pool-selector-content"></div>
      </div>
    `;

    // Wire up close button
    const closeBtn = this.dialog.querySelector(".pool-selector-close");
    on(closeBtn, "click", () => this.close());

    // Wire up overlay click
    on(this.dialog, "click", (e) => {
      if (e.target === this.dialog) {
        this.close();
      }
    });

    // Show with animation
    requestAnimationFrame(() => {
      this.dialog.classList.add("visible");
    });
  }

  renderLoading() {
    const content = this.dialog.querySelector(".pool-selector-content");
    if (!content) return;

    content.innerHTML = `
      <div class="pool-selector-loading">
        <i class="icon-loader spin"></i>
        <p>Loading pools...</p>
      </div>
    `;
  }

  renderPools(pools, tokenMint) {
    const content = this.dialog.querySelector(".pool-selector-content");
    if (!content) return;

    content.innerHTML = `
      <div class="pool-selector-info">
        <span class="pool-count">${pools.length} pool${pools.length !== 1 ? "s" : ""} found</span>
        <span class="pool-mint">${tokenMint.slice(0, 8)}...${tokenMint.slice(-6)}</span>
      </div>
      <div class="pool-list">
        ${pools
          .map(
            (pool, i) => `
          <div class="pool-item" data-index="${i}" tabindex="0" role="button">
            <div class="pool-item-main">
              <span class="pool-dex">${escapeHtml(pool.dex || "Unknown")}</span>
              <span class="pool-pair">${escapeHtml(pool.base_symbol || "?")}/${escapeHtml(pool.quote_symbol || "?")}</span>
              <span class="pool-source ${(pool.source || "unknown").toLowerCase()}">${escapeHtml(pool.source || "Unknown")}</span>
            </div>
            <div class="pool-item-stats">
              <span class="pool-liquidity" title="Liquidity">$${formatCompact(pool.liquidity_usd)} liq</span>
              <span class="pool-volume" title="24h Volume">$${formatCompact(pool.volume_24h)} 24h</span>
            </div>
            <div class="pool-address">${pool.address.slice(0, 8)}...${pool.address.slice(-6)}</div>
          </div>
        `
          )
          .join("")}
      </div>
    `;

    // Add click handlers
    content.querySelectorAll(".pool-item").forEach((item) => {
      const index = parseInt(item.dataset.index, 10);

      on(item, "click", () => {
        this.selectPool(pools[index]);
      });

      on(item, "keydown", (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          this.selectPool(pools[index]);
        }
      });
    });

    // Focus first item
    const firstItem = content.querySelector(".pool-item");
    if (firstItem) {
      firstItem.focus();
    }
  }

  selectPool(pool) {
    this.onSelect(pool, this.tokenMint);
    this.close();
  }

  renderError(message) {
    const content = this.dialog.querySelector(".pool-selector-content");
    if (!content) return;

    content.innerHTML = `
      <div class="pool-selector-error">
        <i class="icon-alert-circle"></i>
        <p>${escapeHtml(message)}</p>
        <button class="btn btn-sm" type="button">Dismiss</button>
      </div>
    `;

    const dismissBtn = content.querySelector("button");
    on(dismissBtn, "click", () => this.close());
  }

  close() {
    if (this.dialog) {
      this.dialog.classList.remove("visible");

      // Remove after animation
      setTimeout(() => {
        if (this.dialog && this.dialog.parentNode) {
          this.dialog.remove();
        }
        this.dialog = null;
      }, 200);
    }

    // Cleanup keyboard handler
    if (this.keydownHandler) {
      off(document, "keydown", this.keydownHandler);
      this.keydownHandler = null;
    }

    this.pools = [];
    this.tokenMint = null;
    this.onClose();
  }

  dispose() {
    this.close();
  }
}

export default PoolSelector;
