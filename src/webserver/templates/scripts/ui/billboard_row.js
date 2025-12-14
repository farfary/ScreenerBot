/**
 * Billboard Row - Horizontal scrollable promotional row
 *
 * Shows featured tokens from the billboard API in a compact
 * horizontal scrolling row at the bottom of specific pages.
 *
 * Visibility: Only on Home and Tokens pages (when enabled in settings)
 */

import { $ } from "../core/dom.js";
import { openBillboard } from "./billboard_dialog.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "./hint_popover.js";

const CONTAINER_ID = "billboard-row";
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes
const MAX_NAME_LENGTH = 12; // Max characters for token name display

// Cache for billboard data
let cachedTokens = null;
let cacheTimestamp = 0;

// Cache for config setting
let configEnabled = null;
let configCheckTimestamp = 0;
const CONFIG_CHECK_TTL_MS = 30 * 1000; // 30 seconds

/**
 * Check if billboard is enabled in config
 */
async function isBillboardEnabled() {
  const now = Date.now();

  // Return cached value if still valid
  if (configEnabled !== null && now - configCheckTimestamp < CONFIG_CHECK_TTL_MS) {
    return configEnabled;
  }

  try {
    const response = await fetch("/api/config/gui");
    if (response.ok) {
      const result = await response.json();
      const config = result.data?.data || result.data || result;
      configEnabled = config?.dashboard?.interface?.show_billboard !== false;
    } else {
      configEnabled = true; // Default to showing on error
    }
  } catch {
    configEnabled = true; // Default to showing on error
  }

  configCheckTimestamp = now;
  return configEnabled;
}

/**
 * Reset config cache (call when settings change)
 */
export function resetBillboardConfigCache() {
  configEnabled = null;
  configCheckTimestamp = 0;
}

/**
 * Truncate text with ellipsis
 */
function truncateName(name, maxLength = MAX_NAME_LENGTH) {
  if (!name) return "???";
  if (name.length <= maxLength) return name;
  return name.slice(0, maxLength - 1) + "…";
}

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

    // Check if billboard is enabled in settings
    const enabled = await isBillboardEnabled();
    if (!enabled) {
      return;
    }

    this._createContainer();
    this.isVisible = true;

    // Add padding class to content to make room for billboard
    const content = $(".content");
    if (content) {
      content.classList.add("has-billboard");
    }

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

    // Remove padding class from content
    const content = $(".content");
    if (content) {
      content.classList.remove("has-billboard");
    }
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
          <span class="billboard-row-label">Featured</span>
          <button class="billboard-row-view-all" title="View All Listings">
            <span>All</span>
            <i class="icon-chevron-right"></i>
          </button>
        </div>
        <div class="billboard-row-scroll">
          <button class="billboard-row-arrow billboard-row-arrow-left" aria-label="Scroll left">
            <i class="icon-chevron-left"></i>
          </button>
          <div class="billboard-row-tokens" id="billboard-row-tokens"></div>
          <button class="billboard-row-arrow billboard-row-arrow-right" aria-label="Scroll right">
            <i class="icon-chevron-right"></i>
          </button>
        </div>
      </div>
    `;

    // Append to body (billboard is fixed positioned above status bar)
    document.body.appendChild(container);

    this.containerEl = container;

    // Setup event listeners
    this._setupEventListeners();

    // Add hint button
    this._addHintButton();
  }

  /**
   * Add hint button to header
   */
  async _addHintButton() {
    await Hints.init();
    if (!Hints.isEnabled()) return;

    const hint = Hints.getHint("ui.billboard");
    if (!hint || Hints.isDismissed(hint.id)) return;

    const header = this.containerEl?.querySelector(".billboard-row-header");
    if (!header) return;

    // Insert hint trigger after the label
    const label = header.querySelector(".billboard-row-label");
    if (label) {
      HintTrigger.attach(label.parentNode, hint, "ui.billboard", {
        size: "sm",
        position: "bottom",
      });
      HintTrigger.initAll();
    }
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
      const scrollAmount = 200;

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
   * Show loading state with skeleton cards
   */
  _showLoading() {
    const container = this.containerEl?.querySelector("#billboard-row-tokens");
    if (container) {
      // Show 5 skeleton cards
      const skeletons = Array(5)
        .fill(0)
        .map(
          () => `
        <div class="billboard-row-card billboard-row-skeleton">
          <div class="billboard-row-card-logo-placeholder skeleton-pulse"></div>
          <span class="billboard-row-card-name skeleton-pulse"></span>
        </div>
      `
        )
        .join("");
      container.innerHTML = skeletons;
    }
  }

  /**
   * Show empty state with placeholder cards
   */
  _showEmpty() {
    const container = this.containerEl?.querySelector("#billboard-row-tokens");
    if (container) {
      container.innerHTML = `
        <div class="billboard-row-empty">
          <div class="billboard-row-empty-cards">
            <div class="billboard-row-card billboard-row-card-placeholder">
              <div class="billboard-row-card-logo-placeholder">
                <i class="icon-coins"></i>
              </div>
              <span class="billboard-row-card-name">—</span>
            </div>
            <div class="billboard-row-card billboard-row-card-placeholder">
              <div class="billboard-row-card-logo-placeholder">
                <i class="icon-coins"></i>
              </div>
              <span class="billboard-row-card-name">—</span>
            </div>
            <div class="billboard-row-card billboard-row-card-placeholder">
              <div class="billboard-row-card-logo-placeholder">
                <i class="icon-coins"></i>
              </div>
              <span class="billboard-row-card-name">—</span>
            </div>
          </div>
          <span class="billboard-row-empty-text">No featured tokens</span>
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
   * Render a single compact token card (logo + name only)
   */
  _renderTokenCard(token) {
    const logoUrl = this._getValidLogoUrl(token);
    const featuredClass = token.featured ? "featured" : "";
    const symbol = token.symbol || "???";
    const name = token.name || symbol;
    const displayName = truncateName(name);
    const fullTitle = `${name} (${symbol})`;

    // Use placeholder if no valid logo URL
    const logoHtml = logoUrl
      ? `<img src="${this._escapeHtml(logoUrl)}" alt="" class="billboard-row-card-logo" loading="lazy" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'"/><div class="billboard-row-card-logo-placeholder" style="display:none"><span>${this._escapeHtml(symbol.charAt(0).toUpperCase())}</span></div>`
      : `<div class="billboard-row-card-logo-placeholder"><span>${this._escapeHtml(symbol.charAt(0).toUpperCase())}</span></div>`;

    return `
      <div class="billboard-row-card ${featuredClass}" title="${this._escapeHtml(fullTitle)}">
        ${logoHtml}
        <span class="billboard-row-card-name">${this._escapeHtml(displayName)}</span>
        ${token.featured ? '<i class="icon-star billboard-row-card-star"></i>' : ""}
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
