/**
 * Billboard Row - Horizontal scrollable promotional row
 *
 * Shows featured tokens from the billboard API in a Netflix-style
 * horizontal scrolling row at the bottom of specific pages.
 *
 * Visibility: Only on Home and Tokens pages
 */

import { $ } from "../core/dom.js";
import { openBillboard } from "./billboard_dialog.js";

const CONTAINER_ID = "billboard-row";
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes

// Cache for billboard data
let cachedTokens = null;
let cacheTimestamp = 0;

/**
 * Billboard Row Manager
 */
class BillboardRow {
  constructor() {
    this.containerEl = null;
    this.isVisible = false;
  }

  /**
   * Create and show the billboard row
   */
  async show() {
    if (this.isVisible) return;

    this._createContainer();
    this.isVisible = true;

    // Show loading state
    this._showLoading();

    // Fetch and render tokens
    try {
      const tokens = await this._fetchTokens();
      if (tokens && tokens.length > 0) {
        this._renderTokens(tokens);
      } else {
        this._showEmpty();
      }
    } catch (e) {
      console.warn("[BillboardRow] Failed to load:", e.message);
      this._showEmpty();
    }
  }

  /**
   * Hide and remove the billboard row
   */
  hide() {
    if (!this.isVisible) return;

    if (this.containerEl) {
      this.containerEl.remove();
      this.containerEl = null;
    }
    this.isVisible = false;
  }

  /**
   * Create the container element
   */
  _createContainer() {
    // Remove existing if present
    const existing = $(`#${CONTAINER_ID}`);
    if (existing) existing.remove();

    const container = document.createElement("div");
    container.id = CONTAINER_ID;
    container.className = "billboard-row";
    container.innerHTML = `
      <div class="billboard-row-inner">
        <div class="billboard-row-header">
          <span class="billboard-row-title">
            <i class="icon-megaphone"></i>
            Featured Tokens
          </span>
          <button class="billboard-row-view-all" title="View All">
            View All <i class="icon-chevron-right"></i>
          </button>
        </div>
        <div class="billboard-row-scroll">
          <button class="billboard-row-arrow billboard-row-arrow-left" title="Scroll left">
            <i class="icon-chevron-left"></i>
          </button>
          <div class="billboard-row-tokens" id="billboard-row-tokens"></div>
          <button class="billboard-row-arrow billboard-row-arrow-right" title="Scroll right">
            <i class="icon-chevron-right"></i>
          </button>
        </div>
      </div>
    `;

    // Insert before status bar
    const statusBar = $("#statusBar");
    if (statusBar) {
      statusBar.parentNode.insertBefore(container, statusBar);
    } else {
      document.body.appendChild(container);
    }

    this.containerEl = container;

    // Setup event listeners
    this._setupEventListeners();
  }

  /**
   * Setup event listeners for scrolling and view all
   */
  _setupEventListeners() {
    if (!this.containerEl) return;

    // View All button - opens the full billboard dialog
    const viewAllBtn = this.containerEl.querySelector(".billboard-row-view-all");
    if (viewAllBtn) {
      viewAllBtn.addEventListener("click", () => openBillboard());
    }

    // Scroll arrows
    const scrollContainer = this.containerEl.querySelector(".billboard-row-tokens");
    const leftArrow = this.containerEl.querySelector(".billboard-row-arrow-left");
    const rightArrow = this.containerEl.querySelector(".billboard-row-arrow-right");

    if (scrollContainer && leftArrow && rightArrow) {
      const scrollAmount = 300;

      leftArrow.addEventListener("click", () => {
        scrollContainer.scrollBy({ left: -scrollAmount, behavior: "smooth" });
      });

      rightArrow.addEventListener("click", () => {
        scrollContainer.scrollBy({ left: scrollAmount, behavior: "smooth" });
      });

      // Update arrow visibility on scroll
      const updateArrows = () => {
        const { scrollLeft, scrollWidth, clientWidth } = scrollContainer;
        leftArrow.classList.toggle("hidden", scrollLeft <= 0);
        rightArrow.classList.toggle("hidden", scrollLeft >= scrollWidth - clientWidth - 1);
      };

      scrollContainer.addEventListener("scroll", updateArrows);
      // Initial update
      requestAnimationFrame(updateArrows);
    }
  }

  /**
   * Fetch tokens from API with caching
   */
  async _fetchTokens() {
    const now = Date.now();

    // Return cached data if still valid
    if (cachedTokens && now - cacheTimestamp < CACHE_TTL_MS) {
      return cachedTokens;
    }

    const response = await fetch("/api/billboard");
    const data = await response.json();

    if (data.success && data.tokens) {
      cachedTokens = data.tokens;
      cacheTimestamp = now;
      return cachedTokens;
    }

    return [];
  }

  /**
   * Show loading state
   */
  _showLoading() {
    const container = this.containerEl?.querySelector("#billboard-row-tokens");
    if (container) {
      container.innerHTML = `
        <div class="billboard-row-state">
          <i class="icon-loader spin"></i>
          <span>Loading...</span>
        </div>
      `;
    }
  }

  /**
   * Show empty state - keep row visible with message
   */
  _showEmpty() {
    const container = this.containerEl?.querySelector("#billboard-row-tokens");
    if (container) {
      container.innerHTML = `
        <div class="billboard-row-state billboard-row-empty">
          <i class="icon-inbox"></i>
          <span>No featured tokens yet</span>
        </div>
      `;
    }
  }

  /**
   * Render token cards
   */
  _renderTokens(tokens) {
    const container = this.containerEl?.querySelector("#billboard-row-tokens");
    if (!container) return;

    container.innerHTML = tokens.map((token) => this._renderTokenCard(token)).join("");

    // Add click handlers to cards
    container.querySelectorAll(".billboard-row-card").forEach((card) => {
      card.addEventListener("click", () => openBillboard());
    });

    // Update arrow visibility after render
    requestAnimationFrame(() => {
      const leftArrow = this.containerEl?.querySelector(".billboard-row-arrow-left");
      const rightArrow = this.containerEl?.querySelector(".billboard-row-arrow-right");
      if (leftArrow && rightArrow) {
        const { scrollLeft, scrollWidth, clientWidth } = container;
        leftArrow.classList.toggle("hidden", scrollLeft <= 0);
        rightArrow.classList.toggle("hidden", scrollWidth <= clientWidth);
      }
    });
  }

  /**
   * Render a single token card
   */
  _renderTokenCard(token) {
    const logoUrl = this._getValidLogoUrl(token);
    const featuredClass = token.featured ? "featured" : "";
    const symbol = token.symbol || "???";

    // Use placeholder if no valid logo URL
    const logoHtml = logoUrl
      ? `<img src="${this._escapeHtml(logoUrl)}" alt="${this._escapeHtml(symbol)}" class="billboard-row-card-logo" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'"/><div class="billboard-row-card-logo-placeholder" style="display:none"><span>${this._escapeHtml(symbol.charAt(0).toUpperCase())}</span></div>`
      : `<div class="billboard-row-card-logo-placeholder"><span>${this._escapeHtml(symbol.charAt(0).toUpperCase())}</span></div>`;

    return `
      <div class="billboard-row-card ${featuredClass}" title="${this._escapeHtml(token.name)} (${this._escapeHtml(symbol)})">
        ${logoHtml}
        <div class="billboard-row-card-info">
          <span class="billboard-row-card-name">${this._escapeHtml(token.name)}</span>
          <span class="billboard-row-card-symbol">${this._escapeHtml(token.symbol)}</span>
        </div>
        ${token.featured ? '<span class="billboard-row-card-badge"><i class="icon-star"></i></span>' : ""}
      </div>
    `;
  }

  /**
   * Escape HTML to prevent XSS
   */
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
const billboardRow = new BillboardRow();

/**
 * Show the billboard row
 */
export function showBillboardRow() {
  billboardRow.show();
}

/**
 * Hide the billboard row
 */
export function hideBillboardRow() {
  billboardRow.hide();
}

/**
 * Check if billboard row is currently visible
 */
export function isBillboardRowVisible() {
  return billboardRow.isVisible;
}
