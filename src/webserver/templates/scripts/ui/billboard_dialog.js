/**
 * Billboard Dialog - Shows featured tokens and external sources
 *
 * Displays community-submitted tokens plus Jupiter and DexScreener trending tokens
 * in a Netflix-style category layout with horizontal scrolling rows.
 */

import { $ } from "../core/dom.js";

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
      this.dialogEl.classList.remove("visible");
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
    dialog.className = "billboard-dialog-overlay";
    dialog.innerHTML = `
      <div class="billboard-dialog billboard-dialog-categories">
        <div class="billboard-header">
          <div class="billboard-title">
            <i class="icon-megaphone"></i>
            <h2>Billboard</h2>
          </div>
          <p class="billboard-subtitle">Featured tokens & trending across Solana</p>
          <button class="billboard-close-btn" title="Close">
            <i class="icon-x"></i>
          </button>
        </div>
        <div class="billboard-body">
          <div class="billboard-categories" id="billboard-categories"></div>
        </div>
        <div class="billboard-footer">
          <a href="https://screenerbot.io/submit-token" target="_blank" rel="noopener noreferrer" class="billboard-submit-link">
            <i class="icon-plus"></i>
            <span>Submit Your Token</span>
          </a>
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

    // Close on backdrop click
    dialog.addEventListener("click", (e) => {
      if (e.target === dialog) this.close();
    });

    // Close on Escape key
    this._handleKeydown = (e) => {
      if (e.key === "Escape") this.close();
    };
    document.addEventListener("keydown", this._handleKeydown);

    // Animate in
    requestAnimationFrame(() => {
      dialog.classList.add("visible");
    });
  }

  _showLoading() {
    const container = $("#billboard-categories");
    if (container) {
      container.innerHTML = `
        <div class="billboard-state billboard-loading">
          <i class="icon-loader spin"></i>
          <span>Loading billboard...</span>
        </div>
      `;
    }
  }

  _showError(message) {
    const container = $("#billboard-categories");
    if (container) {
      container.innerHTML = `
        <div class="billboard-state billboard-error">
          <i class="icon-alert-circle"></i>
          <span>${this._escapeHtml(message)}</span>
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

    const emptyState = tokens.length === 0
      ? "<div class=\"billboard-cat-empty\"><i class=\"icon-inbox\"></i><span>No tokens yet</span></div>"
      : "";

    const tokenCards = tokens
      .map((token) => this._renderTokenCard(token))
      .join("");

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
    const symbol = token.symbol || "???";
    const mint = token.mint || token.id || "";
    const featuredClass = token.featured ? "featured" : "";
    const socials = this._buildSocialIcons(token);

    const badge = token.featured
      ? '<span class="billboard-cat-badge"><i class="icon-star"></i></span>'
      : "";

    // Use placeholder SVG if no valid logo URL
    const logoHtml = logoUrl
      ? `<img src="${this._escapeHtml(logoUrl)}" alt="${this._escapeHtml(symbol)}" class="billboard-cat-logo" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'"/><div class="billboard-cat-logo-placeholder" style="display:none"><span>${this._escapeHtml(symbol.charAt(0).toUpperCase())}</span></div>`
      : `<div class="billboard-cat-logo-placeholder"><span>${this._escapeHtml(symbol.charAt(0).toUpperCase())}</span></div>`;

    return `
      <div class="billboard-cat-card ${featuredClass}" data-mint="${mint}" title="${name} (${symbol})">
        ${badge}
        ${logoHtml}
        <div class="billboard-cat-info">
          <span class="billboard-cat-name">${this._escapeHtml(name)}</span>
          <span class="billboard-cat-symbol">${this._escapeHtml(symbol)}</span>
        </div>
        <div class="billboard-cat-actions">
          ${socials}
          <button class="billboard-cat-copy-btn" data-mint="${mint}" title="Copy mint address">
            <i class="icon-copy"></i>
          </button>
        </div>
      </div>
    `;
  }

  _buildSocialIcons(token) {
    const icons = [];
    if (token.website) {
      icons.push(`<a href="${this._escapeHtml(token.website)}" target="_blank" rel="noopener noreferrer" class="billboard-cat-social" title="Website"><i class="icon-globe"></i></a>`);
    }
    if (token.twitter) {
      icons.push(`<a href="${this._escapeHtml(token.twitter)}" target="_blank" rel="noopener noreferrer" class="billboard-cat-social" title="Twitter"><i class="icon-twitter"></i></a>`);
    }
    if (token.telegram) {
      icons.push(`<a href="${this._escapeHtml(token.telegram)}" target="_blank" rel="noopener noreferrer" class="billboard-cat-social" title="Telegram"><i class="icon-send"></i></a>`);
    }
    if (token.discord) {
      icons.push(`<a href="${this._escapeHtml(token.discord)}" target="_blank" rel="noopener noreferrer" class="billboard-cat-social" title="Discord"><i class="icon-message-circle"></i></a>`);
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
