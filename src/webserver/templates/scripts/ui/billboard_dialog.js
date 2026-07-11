/**
 * Billboard Dialog - Shows featured tokens and external sources
 *
 * Displays community-submitted tokens plus Jupiter and DexScreener trending tokens
 * in a Netflix-style category layout with horizontal scrolling rows.
 */

import { $ } from "../core/dom.js";
import { openExternal } from "../core/utils.js";
// Side-effect import: registers the global "screenerbot:open-token-details"
// window listener so cards open the token details dialog even when the billboard
// dialog is opened from a page (e.g. Home) that doesn't otherwise load it.
import "./token_details_dialog.js";

const DIALOG_ID = "billboard-dialog";

// Category definitions with metadata
const CATEGORIES = [
  {
    id: "featured",
    title: "Featured Tokens",
    icon: "icon-star",
    key: "featured",
    isFeatured: true,
  },
  {
    id: "jupiter-organic",
    title: "Jupiter Top Organic",
    icon: "icon-trending-up",
    key: "jupiter_organic",
    source: "jupiter",
  },
  {
    id: "jupiter-traded",
    title: "Jupiter Top Traded",
    icon: "icon-activity",
    key: "jupiter_traded",
    source: "jupiter",
  },
  {
    id: "dexscreener-trending",
    title: "DexScreener Trending",
    icon: "icon-zap",
    key: "dexscreener_trending",
    source: "dexscreener",
  },
];

class BillboardDialog {
  constructor() {
    this.isOpen = false;
    this.data = null;
    this.dialogEl = null;
  }

  async open() {
    if (this.isOpen) return;
    this.isOpen = true;

    this._createDialog();
    this._showLoading();

    try {
      const response = await fetch("/api/billboard/all");
      const data = await response.json();

      if (data.success) {
        this.data = data;
        this._renderCategories();
      } else {
        this._showError(data.error || "Failed to load billboard");
      }
    } catch (e) {
      this._showError("Network error: " + e.message);
    }
  }

  close() {
    if (this.dialogEl) {
      this.dialogEl.classList.remove("active");
      setTimeout(() => {
        if (this.dialogEl) {
          this.dialogEl.remove();
          this.dialogEl = null;
        }
      }, 250);
    }
    if (this._handleKeydown) {
      document.removeEventListener("keydown", this._handleKeydown);
    }
    this.isOpen = false;
  }

  _createDialog() {
    // Remove existing if present
    const existing = $(`#${DIALOG_ID}`);
    if (existing) existing.remove();

    const dialog = document.createElement("div");
    dialog.id = DIALOG_ID;
    dialog.className = "billboard-dialog";
    dialog.innerHTML = `
      <div class="billboard-backdrop"></div>
      <div class="billboard-container">
        <div class="billboard-header">
          <div class="billboard-title-group">
            <h1 class="billboard-title">Billboard</h1>
            <p class="billboard-subtitle">Curated tokens &amp; trending across Solana</p>
          </div>
          <div class="billboard-actions">
            <button type="button" class="billboard-submit-btn" data-external-url="https://screenerbot.io/submit-token">
              Submit Token
            </button>
            <button class="billboard-close-btn" title="Close (ESC)">
              <i class="icon-x"></i>
            </button>
          </div>
        </div>
        <div class="billboard-body">
          <div class="billboard-categories" id="billboard-categories"></div>
        </div>
      </div>
    `;

    document.body.appendChild(dialog);
    this.dialogEl = dialog;

    // Event listeners
    const closeBtn = dialog.querySelector(".billboard-close-btn");
    if (closeBtn) {
      closeBtn.addEventListener("click", () => this.close());
    }

    // Submit Token button - opens external URL
    const submitBtn = dialog.querySelector(".billboard-submit-btn");
    if (submitBtn) {
      submitBtn.addEventListener("click", () => {
        const url = submitBtn.dataset.externalUrl;
        if (url) openExternal(url);
      });
    }

    // Close on Escape key
    this._handleKeydown = (e) => {
      if (e.key === "Escape") this.close();
    };
    document.addEventListener("keydown", this._handleKeydown);

    // Animate in
    requestAnimationFrame(() => {
      dialog.classList.add("active");
    });
  }

  _showLoading() {
    const container = $("#billboard-categories");
    if (container) {
      container.innerHTML = `
        <div class="billboard-state billboard-loading">
          <i class="icon-loader spin"></i>
          <span>Loading featured &amp; trending...</span>
        </div>
      `;
    }
  }

  _showError(message) {
    const container = $("#billboard-categories");
    if (container) {
      container.innerHTML = `
        <div class="billboard-state billboard-error">
          <i class="icon-circle-alert"></i>
          <span>${this._escapeHtml(message)}</span>
          <span style="font-size:0.75rem;opacity:0.6">Check connection or try again</span>
        </div>
      `;
    }
  }

  _renderCategories() {
    const container = $("#billboard-categories");
    if (!container || !this.data) return;

    container.innerHTML = CATEGORIES.map((cat) => this._renderCategory(cat)).join("");

    // Attach scroll handlers for each category
    CATEGORIES.forEach((cat) => {
      this._initScrollBehavior(cat.id);
    });

    // Clicking a card (outside its action buttons/links) opens token details.
    container.querySelectorAll(".billboard-cat-card").forEach((card) => {
      const mint = card.dataset.mint;
      if (!mint) return;
      card.addEventListener("click", (e) => {
        if (e.target.closest("a, button, .billboard-cat-actions")) return;
        this.close();
        window.dispatchEvent(
          new CustomEvent("screenerbot:open-token-details", {
            detail: { mint, symbol: card.dataset.symbol || "" },
          })
        );
      });
    });

    // Attach copy handlers
    container.querySelectorAll(".billboard-cat-copy-btn").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();
        const mint = btn.dataset.mint;
        if (mint) {
          navigator.clipboard.writeText(mint).then(() => {
            const icon = btn.querySelector("i");
            if (icon) {
              icon.className = "icon-check";
              setTimeout(() => {
                icon.className = "icon-copy";
              }, 1500);
            }
          });
        }
      });
    });
  }

  _renderCategory(category) {
    const tokens = this.data[category.key] || [];

    if (tokens.length === 0 && !category.isFeatured) {
      return ""; // Skip empty external categories
    }

    const emptyState =
      tokens.length === 0
        ? '<div class="billboard-cat-empty"><i class="icon-inbox"></i><span>No tokens in this category</span></div>'
        : "";

    const tokenCards = tokens.map((token) => this._renderTokenCard(token)).join("");

    const sourceTag = category.source
      ? `<span class="billboard-cat-source">${category.source}</span>`
      : "";

    return `
      <div class="billboard-category" data-category="${category.id}">
        <div class="billboard-cat-header">
          <div class="billboard-cat-title">
            <i class="${category.icon}"></i>
            <span>${category.title}</span>
            ${sourceTag}
          </div>
          <span class="billboard-cat-count">${tokens.length} tokens</span>
        </div>
        <div class="billboard-cat-scroll-wrapper">
          <button class="billboard-cat-arrow billboard-cat-arrow-left hidden" data-dir="left">
            <i class="icon-chevron-left"></i>
          </button>
          <div class="billboard-cat-tokens" id="billboard-cat-${category.id}">
            ${emptyState || tokenCards}
          </div>
          <button class="billboard-cat-arrow billboard-cat-arrow-right ${tokens.length <= 4 ? "hidden" : ""}" data-dir="right">
            <i class="icon-chevron-right"></i>
          </button>
        </div>
      </div>
    `;
  }

  _renderTokenCard(token) {
    const logoUrl = this._getValidLogoUrl(token);
    const name = token.name || "Unknown";
    const symbol = (token.symbol || "???").toUpperCase();
    const mint = token.mint || token.id || "";
    const featuredClass = token.featured ? "featured" : "";
    const socials = this._buildSocialIcons(token);

    const badge = token.featured
      ? '<span class="billboard-cat-badge"><i class="icon-star"></i></span>'
      : "";

    // Small security dot for enriched tokens (higher = safer)
    let secHtml = '';
    if (token.security_score_normalised != null) {
      const s = token.security_score_normalised;
      const secClass = s >= 70 ? 'good' : s >= 40 ? 'mid' : 'bad';
      secHtml = `<span class="sec-dot ${secClass}" title="Security: ${s}"></span>`;
    }

    const logoHtml = logoUrl
      ? `<img src="${this._escapeHtml(logoUrl)}" alt="${this._escapeHtml(symbol)}" class="billboard-cat-logo" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'"/><div class="billboard-cat-logo-placeholder" style="display:none"><span>${this._escapeHtml(symbol.charAt(0))}</span></div>`
      : `<div class="billboard-cat-logo-placeholder"><span>${this._escapeHtml(symbol.charAt(0))}</span></div>`;

    // Build metrics line if available (from enrichment or external)
    let metricsHtml = '';
    const price = token.price_usd ?? token.price_sol;
    const mc = token.market_cap_usd ?? token.fdv_usd;
    const liq = token.liquidity_usd ?? token.liquidity;
    const ch24 = token.price_change_24h ?? token.price_change_h24;

    if (price != null || mc != null || liq != null || ch24 != null) {
      const parts = [];
      if (price != null) {
        const p = typeof price === 'number' ? price.toLocaleString(undefined, {maximumFractionDigits: price < 0.01 ? 6 : 2}) : price;
        parts.push(`<span class="metric">$${p}</span>`);
      }
      if (mc != null) {
        parts.push(`<span class="metric">MC ${this._formatCompact(mc)}</span>`);
      }
      if (liq != null) {
        parts.push(`<span class="metric">Liq ${this._formatCompact(liq)}</span>`);
      }
      if (ch24 != null) {
        const cls = ch24 >= 0 ? 'pos' : 'neg';
        parts.push(`<span class="metric ${cls}">${ch24 > 0 ? '+' : ''}${ch24.toFixed(1)}%</span>`);
      }
      if (parts.length) {
        metricsHtml = `<div class="billboard-cat-metrics">${parts.join('')}</div>`;
      }
    }

    return `
      <div class="billboard-cat-card ${featuredClass}" data-mint="${this._escapeHtml(mint)}" data-symbol="${this._escapeHtml(symbol)}" title="${this._escapeHtml(name)} (${this._escapeHtml(symbol)})">
        <div class="card-main">
          <!-- Logo top left -->
          <div class="logo-col">
            ${logoHtml}
            ${badge}
          </div>

          <!-- Infos below logo -->
          <div class="info-col">
            <div class="name-row">
              <span class="cat-name">${this._escapeHtml(name)}</span>
              ${secHtml}
            </div>
            <span class="cat-symbol">${this._escapeHtml(symbol)}</span>
          </div>

          <!-- Some infos on the right -->
          <div class="metrics-col">
            ${price != null ? `<div class="price">${typeof price === 'number' ? '$' + price.toLocaleString(undefined, {maximumFractionDigits: price < 0.01 ? 6 : 2}) : price}</div>` : ''}
            ${ch24 != null ? `<div class="change ${ch24 >= 0 ? 'pos' : 'neg'}">${ch24 > 0 ? '+' : ''}${ch24.toFixed(1)}%</div>` : ''}
            ${mc != null ? `<div class="meta">MC ${this._formatCompact(mc)}</div>` : ''}
            ${liq != null ? `<div class="meta">Liq ${this._formatCompact(liq)}</div>` : ''}
          </div>
        </div>

        <div class="card-actions">
          ${socials}
          <button class="cat-copy-btn" data-mint="${this._escapeHtml(mint)}" title="Copy mint">
            <i class="icon-copy"></i>
          </button>
        </div>
      </div>
    `;
  }

  _buildSocialIcons(token) {
    const icons = [];
    if (token.website) {
      icons.push(
        `<a href="${this._escapeHtml(token.website)}" target="_blank" rel="noopener noreferrer" class="billboard-cat-social" title="Website"><i class="icon-globe"></i></a>`
      );
    }
    if (token.twitter) {
      icons.push(
        `<a href="${this._escapeHtml(token.twitter)}" target="_blank" rel="noopener noreferrer" class="billboard-cat-social" title="Twitter"><i class="icon-twitter"></i></a>`
      );
    }
    if (token.telegram) {
      icons.push(
        `<a href="${this._escapeHtml(token.telegram)}" target="_blank" rel="noopener noreferrer" class="billboard-cat-social" title="Telegram"><i class="icon-send"></i></a>`
      );
    }
    if (token.discord) {
      icons.push(
        `<a href="${this._escapeHtml(token.discord)}" target="_blank" rel="noopener noreferrer" class="billboard-cat-social" title="Discord"><i class="icon-message-circle"></i></a>`
      );
    }
    return icons.join("");
  }

  _initScrollBehavior(categoryId) {
    const container = document.getElementById(`billboard-cat-${categoryId}`);
    if (!container) return;

    const wrapper = container.closest(".billboard-cat-scroll-wrapper");
    if (!wrapper) return;

    const leftArrow = wrapper.querySelector(".billboard-cat-arrow-left");
    const rightArrow = wrapper.querySelector(".billboard-cat-arrow-right");

    const updateArrows = () => {
      const { scrollLeft, scrollWidth, clientWidth } = container;
      const atStart = scrollLeft <= 0;
      const atEnd = scrollLeft + clientWidth >= scrollWidth - 10;
      if (leftArrow) leftArrow.classList.toggle("hidden", atStart);
      if (rightArrow) rightArrow.classList.toggle("hidden", atEnd);
    };

    container.addEventListener("scroll", updateArrows);
    if (leftArrow) {
      leftArrow.addEventListener("click", () => {
        container.scrollBy({ left: -300, behavior: "smooth" });
      });
    }
    if (rightArrow) {
      rightArrow.addEventListener("click", () => {
        container.scrollBy({ left: 300, behavior: "smooth" });
      });
    }
    updateArrows();
  }

  _escapeHtml(text) {
    if (!text) return "";
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  _formatCompact(n) {
    if (n == null || !Number.isFinite(n)) return '—';
    if (n >= 1e9) return (n / 1e9).toFixed(1) + 'B';
    if (n >= 1e6) return (n / 1e6).toFixed(1) + 'M';
    if (n >= 1e3) return (n / 1e3).toFixed(0) + 'K';
    return n.toFixed(0);
  }

  /**
   * Get a valid logo URL - only use if starts with http:// or https://
   * @param {Object} token - Token object with potential logo fields
   * @returns {string|null} Valid URL or null for placeholder
   */
  _getValidLogoUrl(token) {
    const url = token.logo_url || token.logo || token.icon || null;
    if (url && (url.startsWith("http://") || url.startsWith("https://"))) {
      return url;
    }
    return null;
  }
}

// Singleton instance
const billboardDialog = new BillboardDialog();

/**
 * Open the billboard dialog
 */
export function openBillboard() {
  billboardDialog.open();
}

/**
 * Close the billboard dialog
 */
export function closeBillboard() {
  billboardDialog.close();
}

/**
 * Initialize billboard button handler
 */
export function initBillboard() {
  const btn = $("#billboard-btn");
  if (btn) {
    btn.addEventListener("click", () => openBillboard());
  }
}
