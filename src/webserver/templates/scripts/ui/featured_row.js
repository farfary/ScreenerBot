/**
 * Featured Row - the horizontal discovery strip above the status bar.
 *
 * Two bands, in this order and never mixed:
 *   1. BOOSTED - tokens whose teams paid to promote them on screenerbot.io. They
 *      lead the row, carry the gold treatment and print their active boost count.
 *   2. DISCOVERY - Jupiter top-organic/top-traded and DexScreener trending, in
 *      that order, deduped against the boosted band and against each other.
 *
 * Boosted tokens lead here rather than being spread through the row (which is what
 * the website does on a page of dozens of rows): this strip shows a handful of
 * cards at a time, so a spread promotion would simply scroll out of sight.
 *
 * Visibility: Home and Tokens pages, when enabled in settings.
 */

import { $ } from "../core/dom.js";
import { resolveTokenLogoUrl } from "../core/utils.js";
import { openFeaturedDialog } from "./featured_dialog.js";
import { boostTier, formatBoostCount } from "../core/boosts.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "./hint_popover.js";
// Side-effect import: registers the global "screenerbot:open-token-details"
// window listener. The featured row appears on pages (e.g. Home) that do not
// otherwise load the token details dialog module, so without this the
// open-token-details event a card click dispatches would have no listener.
import "./token_details_dialog.js";

const CONTAINER_ID = "featured-row";
const CACHE_TTL_MS = 5 * 60 * 1000;
const MAX_DISPLAY_LENGTH = 14; // name or symbol, prefer symbol for density

// Cache for featured data
let cachedTokens = null;
let cacheTimestamp = 0;

// Cache for config setting
let configEnabled = null;
let configCheckTimestamp = 0;
const CONFIG_CHECK_TTL_MS = 30 * 1000; // 30 seconds

/**
 * Check if the featured row is enabled in config
 */
async function isFeaturedRowEnabled() {
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
      configEnabled = config?.dashboard?.interface?.show_featured_row !== false;
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
export function resetFeaturedRowConfigCache() {
  configEnabled = null;
  configCheckTimestamp = 0;
}

/**
 * Truncate for compact display (prefer symbol when possible)
 */
function truncateForDisplay(name, symbol, maxLength = MAX_DISPLAY_LENGTH) {
  const s = (symbol || "???").toUpperCase();
  if (s.length <= maxLength) return s;
  const n = name || s;
  if (n.length <= maxLength) return n;
  return n.slice(0, maxLength - 1) + "…";
}

/**
 * Featured Row Manager
 */
class FeaturedRow {
  constructor() {
    this.containerEl = null;
    this.isVisible = false;
  }

  /**
   * Create and show the featured row
   */
  async show() {
    if (this.isVisible) return;

    // Check if the row is enabled in settings
    const enabled = await isFeaturedRowEnabled();
    if (!enabled) {
      return;
    }

    this._createContainer();
    this.isVisible = true;

    // Add padding class to content to make room for the row
    const content = $(".content");
    if (content) {
      content.classList.add("has-featured-row");
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
      console.warn("[FeaturedRow] Failed to load:", e.message);
      this._showEmpty();
    }
  }

  /**
   * Hide and remove the featured row
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
      content.classList.remove("has-featured-row");
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
    container.className = "featured-row";
    container.innerHTML = `
      <div class="featured-row-inner">
        <div class="featured-row-header">
          <span class="featured-row-label">Featured</span>
          <button class="featured-row-view-all" title="Open the full Featured view">
            <span>All</span>
            <i class="icon-chevron-right"></i>
          </button>
        </div>
        <div class="featured-row-scroll">
          <button class="featured-row-arrow featured-row-arrow-left" aria-label="Scroll left">
            <i class="icon-chevron-left"></i>
          </button>
          <div class="featured-row-tokens" id="featured-row-tokens"></div>
          <button class="featured-row-arrow featured-row-arrow-right" aria-label="Scroll right">
            <i class="icon-chevron-right"></i>
          </button>
        </div>
      </div>
    `;

    // Append to body (the row is fixed positioned above the status bar)
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

    const hint = Hints.getHint("ui.featured");
    if (!hint || Hints.isDismissed(hint.id)) return;

    const header = this.containerEl?.querySelector(".featured-row-header");
    if (!header) return;

    // Insert hint trigger after the label
    const label = header.querySelector(".featured-row-label");
    if (label) {
      HintTrigger.attach(label.parentNode, hint, "ui.featured", {
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

    // View All button - opens the full featured dialog
    const viewAllBtn = this.containerEl.querySelector(".featured-row-view-all");
    if (viewAllBtn) {
      viewAllBtn.addEventListener("click", () => openFeaturedDialog());
    }

    // Scroll arrows
    const scrollContainer = this.containerEl.querySelector(".featured-row-tokens");
    const leftArrow = this.containerEl.querySelector(".featured-row-arrow-left");
    const rightArrow = this.containerEl.querySelector(".featured-row-arrow-right");

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
   * Fetch tokens from the API with caching.
   *
   * Pulls every featured source in one request and merges them boosted-first,
   * deduped by mint, so the row still fills with discovery tokens when nobody is
   * currently boosting instead of showing nothing.
   */
  async _fetchTokens() {
    const now = Date.now();

    // Return cached data if still valid
    if (cachedTokens && now - cacheTimestamp < CACHE_TTL_MS) {
      return cachedTokens;
    }

    const response = await fetch("/api/featured/all");
    const data = await response.json();

    if (!data || data.success === false) {
      return [];
    }

    const merged = [];
    const seen = new Set();
    const add = (list) => {
      (list || []).forEach((token) => {
        const mint = token.mint;
        if (!mint || seen.has(mint)) return;
        seen.add(mint);
        merged.push(token);
      });
    };

    add(data.boosted);
    add(data.jupiter_organic);
    add(data.jupiter_traded);
    add(data.dexscreener_trending);

    cachedTokens = merged;
    cacheTimestamp = now;
    return merged;
  }

  /**
   * Show loading state with skeleton cards
   */
  _showLoading() {
    const container = this.containerEl?.querySelector("#featured-row-tokens");
    if (container) {
      // Show 5 skeleton cards
      const skeletons = Array(5)
        .fill(0)
        .map(
          () => `
        <div class="featured-row-card featured-row-skeleton">
          <div class="featured-row-card-logo-placeholder skeleton-pulse"></div>
          <span class="featured-row-card-name skeleton-pulse"></span>
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
    const container = this.containerEl?.querySelector("#featured-row-tokens");
    if (container) {
      const placeholder = `
        <div class="featured-row-card featured-row-card-placeholder">
          <div class="featured-row-card-logo-placeholder">
            <i class="icon-coins"></i>
          </div>
          <span class="featured-row-card-name">—</span>
        </div>`;
      container.innerHTML = `
        <div class="featured-row-empty">
          <div class="featured-row-empty-cards">${placeholder.repeat(3)}</div>
          <span class="featured-row-empty-text">No featured tokens</span>
        </div>
      `;
    }
  }

  /**
   * Render token cards, boosted band first with a rule closing it off.
   */
  _renderTokens(tokens) {
    const container = this.containerEl?.querySelector("#featured-row-tokens");
    if (!container) return;

    const boosted = tokens.filter((token) => boostTier(token) !== null);
    const organic = tokens.filter((token) => boostTier(token) === null);
    // Only draw the rule when it actually separates two bands.
    const divider =
      boosted.length > 0 && organic.length > 0
        ? '<span class="featured-row-divider" aria-hidden="true"></span>'
        : "";

    container.innerHTML =
      boosted.map((token) => this._renderTokenCard(token)).join("") +
      divider +
      organic.map((token) => this._renderTokenCard(token)).join("");

    // Clicking a token card opens its full details dialog.
    container.querySelectorAll(".featured-row-card").forEach((card) => {
      const mint = card.dataset.mint;
      if (!mint) return;
      card.addEventListener("click", () => {
        window.dispatchEvent(
          new CustomEvent("screenerbot:open-token-details", {
            detail: {
              mint,
              symbol: card.dataset.symbol || "",
              name: card.dataset.name || "",
              logo_url: card.dataset.logoUrl || "",
            },
          })
        );
      });
    });

    // Update arrow visibility after render
    requestAnimationFrame(() => {
      const leftArrow = this.containerEl?.querySelector(".featured-row-arrow-left");
      const rightArrow = this.containerEl?.querySelector(".featured-row-arrow-right");
      if (leftArrow && rightArrow) {
        const { scrollLeft, scrollWidth, clientWidth } = container;
        leftArrow.classList.toggle("hidden", scrollLeft <= 0);
        rightArrow.classList.toggle("hidden", scrollWidth <= clientWidth);
      }
    });
  }

  /**
   * Render a single compact token card (logo + name + one metric).
   */
  _renderTokenCard(token) {
    const logoUrl = resolveTokenLogoUrl(token);
    const tier = boostTier(token);
    const symbol = (token.symbol || "???").toUpperCase();
    const name = token.name || symbol;
    const mint = token.mint || "";
    const display = truncateForDisplay(name, symbol);
    const boostCount = formatBoostCount(token.boosts);
    const fullTitle = tier
      ? `${name} (${symbol}) — boosted ${boostCount}`
      : `${name} (${symbol})`;

    // A boosted card prints its boost count instead of a market metric: the count
    // is the reason the card is at the front, and one number per card is the
    // whole density budget at 28px tall.
    let metric = "";
    if (tier) {
      metric = `<span class="featured-row-boost">${this._escapeHtml(boostCount)}</span>`;
    } else {
      const change = token.price_change_24h;
      if (change != null) {
        const cls = change >= 0 ? "pos" : "neg";
        metric = `<span class="row-metric ${cls}">${change > 0 ? "+" : ""}${change.toFixed(0)}%</span>`;
      } else if (token.price_usd != null) {
        const price = token.price_usd;
        const shown = price < 0.01 ? price.toFixed(4) : price.toFixed(2);
        metric = `<span class="row-metric">$${shown}</span>`;
      }
    }

    const logoHtml = logoUrl
      ? `<img src="${this._escapeHtml(logoUrl)}" alt="" class="featured-row-card-logo" loading="lazy" onerror="this.style.display='none'; this.nextElementSibling.style.display='flex'"/><div class="featured-row-card-logo-placeholder" style="display:none"><span>${this._escapeHtml(symbol.charAt(0))}</span></div>`
      : `<div class="featured-row-card-logo-placeholder"><span>${this._escapeHtml(symbol.charAt(0))}</span></div>`;

    return `
      <div class="featured-row-card${tier ? ` boosted ${tier}` : ""}" data-mint="${this._escapeHtml(mint)}" data-symbol="${this._escapeHtml(symbol)}" data-name="${this._escapeHtml(name)}" data-logo-url="${this._escapeHtml(logoUrl || "")}" title="${this._escapeHtml(fullTitle)}">
        ${logoHtml}
        <span class="featured-row-card-name">${this._escapeHtml(display)}</span>
        ${metric}
        ${tier ? '<i class="icon-zap featured-row-card-boost-mark" aria-hidden="true"></i>' : ""}
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
}

// Singleton instance
const featuredRow = new FeaturedRow();

/**
 * Show the featured row
 */
export function showFeaturedRow() {
  featuredRow.show();
}

/**
 * Hide the featured row
 */
export function hideFeaturedRow() {
  featuredRow.hide();
}

/**
 * Check if the featured row is currently visible
 */
export function isFeaturedRowVisible() {
  return featuredRow.isVisible;
}
