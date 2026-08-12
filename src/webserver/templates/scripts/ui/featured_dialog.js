/**
 * Featured Dialog - the full discovery view behind the featured row.
 *
 * BOOSTED comes first and is the only paid category: those teams bought
 * visibility on screenerbot.io, so their cards lead the view and carry the gold
 * treatment plus their active boost count. Everything below it is third-party
 * discovery (Jupiter, DexScreener) and is never gold.
 *
 * Cards are profile-style in a responsive grid; the whole page scrolls, so every
 * token in a category is reachable without a per-row horizontal scroll.
 */

import { $ } from "../core/dom.js";
import { pushEscapeHandler } from "../core/escape_stack.js";
import * as Utils from "../core/utils.js";
import {
  openExternal,
  openDexScreener,
  openGMGN,
  openSolscan,
  resolveTokenLogoUrl,
  resolveTokenBannerUrl,
} from "../core/utils.js";
import { manualTrade } from "./manual_trade.js";
import { boostTier, formatBoostCount } from "../core/boosts.js";
import { getTokenAccent, fallbackAccent } from "../core/token_accent.js";
// Side-effect import: registers the global "screenerbot:open-token-details"
// window listener so cards open the token details dialog even when the featured
// dialog is opened from a page (e.g. Home) that doesn't otherwise load it.
import "./token_details_dialog.js";

const DIALOG_ID = "featured-dialog";

// Category definitions with metadata
const CATEGORIES = [
  {
    id: "boosted",
    title: "Boosted",
    icon: "icon-zap",
    key: "boosted",
    // The one paid category. Its own note explains what a boost buys, so a user
    // never has to guess why these tokens are at the top.
    note: "Promoted by their teams",
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

class FeaturedDialog {
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
      const response = await fetch("/api/featured/all");
      const data = await response.json();

      if (data.success) {
        this.data = data;
        this._renderCategories();
      } else {
        this._showError(data.error || "Failed to load featured");
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
    if (this._releaseEscape) {
      this._releaseEscape();
      this._releaseEscape = null;
    }
    this.isOpen = false;
  }

  _createDialog() {
    // Remove existing if present
    const existing = $(`#${DIALOG_ID}`);
    if (existing) existing.remove();

    const dialog = document.createElement("div");
    dialog.id = DIALOG_ID;
    dialog.className = "featured-dialog";
    dialog.innerHTML = `
      <div class="featured-backdrop"></div>
      <div class="featured-container">
        <div class="featured-header">
          <div class="featured-title-group">
            <h1 class="featured-title">Featured</h1>
            <p class="featured-subtitle">Boosted tokens first, then trending across Solana</p>
          </div>
          <div class="featured-actions">
            <button type="button" class="featured-boost-btn" data-external-url="https://screenerbot.io/boost">
              Boost a Token
            </button>
            <button class="dialog-close" type="button" title="Close (ESC)">
              <i class="icon-x"></i>
            </button>
          </div>
        </div>
        <div class="featured-body">
          <div class="featured-categories" id="featured-categories"></div>
        </div>
      </div>
    `;

    document.body.appendChild(dialog);
    this.dialogEl = dialog;

    // Event listeners
    const closeBtn = dialog.querySelector(".dialog-close");
    if (closeBtn) {
      closeBtn.addEventListener("click", () => this.close());
    }

    // Boost button - opens the public boost page in the user's browser
    const boostBtn = dialog.querySelector(".featured-boost-btn");
    if (boostBtn) {
      boostBtn.addEventListener("click", () => {
        const url = boostBtn.dataset.externalUrl;
        if (url) openExternal(url);
      });
    }

    // Escape is owned by the shared stack, so while token details is open on top
    // of the featured a single Escape closes only the details dialog.
    this._releaseEscape = pushEscapeHandler(() => this.close());

    // Animate in
    requestAnimationFrame(() => {
      dialog.classList.add("active");
    });
  }

  _showLoading() {
    const container = $("#featured-categories");
    if (container) {
      container.innerHTML = `
        <div class="featured-state featured-loading">
          <i class="icon-loader spin"></i>
          <span>Loading featured &amp; trending...</span>
        </div>
      `;
    }
  }

  _showError(message) {
    const container = $("#featured-categories");
    if (container) {
      container.innerHTML = `
        <div class="featured-state featured-error">
          <i class="icon-circle-alert"></i>
          <span>${this._escapeHtml(message)}</span>
          <span style="font-size:0.75rem;opacity:0.6">Check connection or try again</span>
        </div>
      `;
    }
  }

  _renderCategories() {
    const container = $("#featured-categories");
    if (!container || !this.data) return;

    const visible = CATEGORIES.filter((cat) => (this.data[cat.key] || []).length > 0);

    if (visible.length === 0) {
      container.innerHTML = `
        <div class="featured-state featured-empty">
          <i class="icon-inbox"></i>
          <span>No tokens available right now</span>
        </div>
      `;
      return;
    }

    container.innerHTML = visible.map((cat) => this._renderCategory(cat)).join("");

    // Clicking a card (outside its action buttons/links) opens token details.
    // The featured stays open UNDERNEATH (--z-featured-dialog sits below
    // --z-dialog), so closing the details dialog reveals the featured again
    // instead of the page below it.
    container.querySelectorAll(".feat-card").forEach((card) => {
      const mint = card.dataset.mint;
      if (!mint) return;
      card.addEventListener("click", (e) => {
        if (e.target.closest("a, button")) return;
        window.dispatchEvent(
          new CustomEvent("screenerbot:open-token-details", {
            detail: { mint, symbol: card.dataset.symbol || "" },
          })
        );
      });
    });

    // One delegated listener for every card action, so adding a shortcut is a
    // markup change only. stopPropagation keeps a shortcut click from also
    // opening the token details dialog behind it.
    container.querySelectorAll("[data-action]").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();

        const card = btn.closest(".feat-card");
        const mint = btn.dataset.mint || card?.dataset.mint;
        if (!mint) return;

        switch (btn.dataset.action) {
          case "dexscreener":
            openDexScreener(mint);
            break;
          case "gmgn":
            openGMGN(mint);
            break;
          case "solscan":
            openSolscan(mint);
            break;
          case "copy":
            this._copyMint(btn, mint);
            break;
          case "buy":
            this._handleBuy(mint, card?.dataset.symbol || "");
            break;
        }
      });
    });
  }

  _copyMint(btn, mint) {
    navigator.clipboard.writeText(mint).then(() => {
      const icon = btn.querySelector("i");
      if (!icon) return;
      icon.className = "icon-check";
      setTimeout(() => {
        icon.className = "icon-copy";
      }, 1500);
    });
  }

  /**
   * Buy straight from the card, via the one shared manual-trade flow (the same one
   * the context menu, token details and the tokens page use).
   */
  async _handleBuy(mint, symbol) {
    await manualTrade({ action: "buy", mint, symbol });
  }

  _renderCategory(category) {
    const tokens = this.data[category.key] || [];
    const tokenCards = tokens.map((token) => this._renderTokenCard(token)).join("");

    const sourceTag = category.source
      ? `<span class="featured-cat-source">${category.source}</span>`
      : "";
    const note = category.note
      ? `<span class="featured-cat-note">${category.note}</span>`
      : "";

    return `
      <div class="featured-category${category.key === "boosted" ? " boosted" : ""}" data-category="${category.id}">
        <div class="featured-cat-header">
          <div class="featured-cat-title">
            <i class="${category.icon}"></i>
            <span>${category.title}</span>
            ${sourceTag}
            ${note}
          </div>
          <span class="featured-cat-count">${tokens.length} tokens</span>
        </div>
        <div class="featured-cat-grid" id="featured-cat-${category.id}">
          ${tokenCards}
        </div>
      </div>
    `;
  }

  /**
   * Render one token card, laid out like a social profile: banner across the top,
   * circular avatar straddling its lower edge, identity and stats beneath.
   *
   * Every stat is optional -- the backend fills them from our local database only,
   * so a token we do not track simply shows fewer cells rather than "-" filler.
   */
  _renderTokenCard(token) {
    const tier = boostTier(token);
    const boostCount = formatBoostCount(token.boosts);
    const logoUrl = resolveTokenLogoUrl(token);
    const bannerUrl = resolveTokenBannerUrl(token);
    const name = token.name || "Unknown";
    const symbol = (token.symbol || "???").toUpperCase();
    const mint = token.mint || "";

    // Tint the card with the token's own colour. Sampling the logo is async (and
    // may fail on a CORS-locked CDN), so the card is painted immediately with the
    // deterministic mint-derived accent and upgraded in place once the logo is
    // sampled -- no layout shift, no flash of un-tinted card.
    const accent = fallbackAccent(mint);
    if (logoUrl) {
      getTokenAccent(mint, logoUrl).then((resolved) => this._applyAccent(mint, resolved));
    }

    const initial = this._escapeHtml(symbol.charAt(0));

    // A missing banner falls back to a gradient built from the accent, so the top
    // of the card is never dead white space.
    const bannerHtml = bannerUrl
      ? `<img src="${this._escapeHtml(bannerUrl)}" alt="" class="feat-card-banner-img" loading="lazy" onerror="this.remove()"/>`
      : "";

    const avatarHtml = logoUrl
      ? `<img src="${this._escapeHtml(logoUrl)}" alt="" class="feat-card-avatar-img" loading="lazy" onerror="this.replaceWith(Object.assign(document.createElement('span'),{className:'feat-card-avatar-initial',textContent:'${initial}'}))"/>`
      : `<span class="feat-card-avatar-initial">${initial}</span>`;

    const change = token.price_change_24h;
    const changeHtml =
      change != null
        ? `<span class="feat-card-change ${change >= 0 ? "pos" : "neg"}">${change > 0 ? "+" : ""}${change.toFixed(1)}%</span>`
        : "";

    const priceHtml =
      token.price_usd != null
        ? `<span class="feat-card-price">${Utils.formatCurrencyUSD(token.price_usd)}</span>`
        : "";

    const txns =
      token.txns_24h_buys != null || token.txns_24h_sells != null
        ? (token.txns_24h_buys || 0) + (token.txns_24h_sells || 0)
        : null;

    const stats = [
      ["Market Cap", token.market_cap, "compact"],
      ["Liquidity", token.liquidity_usd, "compact"],
      ["Vol 24H", token.volume_24h, "compact"],
      ["Holders", token.holders, "count"],
      ["Txns 24H", txns, "count"],
    ]
      .filter(([, value]) => value != null)
      .map(
        ([label, value, kind]) => `
          <div class="feat-card-stat">
            <span class="feat-card-stat-label">${label}</span>
            <span class="feat-card-stat-value">${
              kind === "compact"
                ? Utils.formatCompactNumber(value, { prefix: "$" })
                : Utils.formatCompactNumber(value)
            }</span>
          </div>`
      )
      .join("");

    return `
      <article class="feat-card${tier ? ` boosted ${tier}` : ""}"
        data-mint="${this._escapeHtml(mint)}"
        data-symbol="${this._escapeHtml(symbol)}"
        style="--feat-hue:${accent.hue};--feat-sat:${accent.saturation}%"
        title="${this._escapeHtml(name)} (${this._escapeHtml(symbol)})">

        <div class="feat-card-banner">${bannerHtml}</div>

        <div class="feat-card-head">
          <div class="feat-card-avatar">${avatarHtml}</div>
        </div>

        <div class="feat-card-identity">
          <div class="feat-card-row">
            <span class="feat-card-name">${this._escapeHtml(name)}</span>
            ${
              tier
                ? `<span class="boost-mark${tier === "golden" ? " golden" : ""}" title="Boosted ${this._escapeHtml(boostCount)} on screenerbot.io"><i class="icon-zap" aria-hidden="true"></i><span class="boost-mark-count">${this._escapeHtml(boostCount)}</span></span>`
                : ""
            }
            ${priceHtml}
          </div>
          <div class="feat-card-row">
            <span class="feat-card-symbol">${this._escapeHtml(symbol)}</span>
            ${this._renderSecurity(token.security_score)}
            ${changeHtml}
          </div>
        </div>

        ${stats ? `<div class="feat-card-stats">${stats}</div>` : ""}

        <div class="feat-card-actions">
          <div class="feat-card-links">
            ${this._buildSocialIcons(token)}
            ${this._buildShortcuts(mint)}
          </div>
          <button class="feat-card-buy" data-action="buy" title="Buy ${this._escapeHtml(symbol)}">
            <i class="icon-zap"></i>
            <span>Buy</span>
          </button>
        </div>
      </article>
    `;
  }

  /**
   * Explorer / chart shortcuts plus copy-mint. Always available — they only need
   * the mint, unlike the socials, which most tokens simply do not have.
   */
  _buildShortcuts(mint) {
    const safeMint = this._escapeHtml(mint);
    return `
      <button class="feat-card-link" data-action="dexscreener" data-mint="${safeMint}" title="DexScreener">
        <i class="icon-chart-candlestick"></i>
      </button>
      <button class="feat-card-link" data-action="gmgn" data-mint="${safeMint}" title="GMGN">
        <i class="icon-trending-up"></i>
      </button>
      <button class="feat-card-link" data-action="solscan" data-mint="${safeMint}" title="Solscan">
        <i class="icon-search"></i>
      </button>
      <button class="feat-card-link feat-card-copy" data-action="copy" data-mint="${safeMint}" title="Copy mint">
        <i class="icon-copy"></i>
      </button>
    `;
  }

  /**
   * Security badge. The score is normalised so HIGHER IS SAFER.
   */
  _renderSecurity(score) {
    if (score == null) return "";

    const level = score >= 70 ? "good" : score >= 40 ? "mid" : "bad";
    const label = score >= 70 ? "Safe" : score >= 40 ? "Caution" : "Risky";

    return `<span class="feat-card-security ${level}" title="Security score: ${score}/100">${label}</span>`;
  }

  /**
   * Repaint a card's tint once its logo colour has been sampled. The card may
   * already be gone (dialog closed, category re-rendered), hence the lookup.
   */
  _applyAccent(mint, accent) {
    if (!accent || !this.dialogEl) return;

    const selector = `.feat-card[data-mint="${CSS.escape(mint)}"]`;
    this.dialogEl.querySelectorAll(selector).forEach((card) => {
      card.style.setProperty("--feat-hue", accent.hue);
      card.style.setProperty("--feat-sat", `${accent.saturation}%`);
    });
  }

  _buildSocialIcons(token) {
    const icons = [];
    if (token.website) {
      icons.push(
        `<a href="${this._escapeHtml(token.website)}" target="_blank" rel="noopener noreferrer" class="feat-card-social" title="Website"><i class="icon-globe"></i></a>`
      );
    }
    if (token.twitter) {
      icons.push(
        `<a href="${this._escapeHtml(token.twitter)}" target="_blank" rel="noopener noreferrer" class="feat-card-social" title="Twitter"><i class="icon-twitter"></i></a>`
      );
    }
    if (token.telegram) {
      icons.push(
        `<a href="${this._escapeHtml(token.telegram)}" target="_blank" rel="noopener noreferrer" class="feat-card-social" title="Telegram"><i class="icon-send"></i></a>`
      );
    }
    if (token.discord) {
      icons.push(
        `<a href="${this._escapeHtml(token.discord)}" target="_blank" rel="noopener noreferrer" class="feat-card-social" title="Discord"><i class="icon-message-circle"></i></a>`
      );
    }
    return icons.join("");
  }

  _escapeHtml(text) {
    if (!text) return "";
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }


}

// Singleton instance
const featuredDialog = new FeaturedDialog();

/**
 * Open the featured dialog
 */
export function openFeaturedDialog() {
  featuredDialog.open();
}

/**
 * Close the featured dialog
 */
export function closeFeaturedDialog() {
  featuredDialog.close();
}

/**
 * Initialize featured button handler
 */
export function initFeaturedDialog() {
  const btn = $("#featured-btn");
  if (btn) {
    btn.addEventListener("click", () => openFeaturedDialog());
  }
}
