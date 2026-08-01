/**
 * Billboard Dialog - Shows featured tokens and external sources
 *
 * Displays community-submitted tokens plus Jupiter and DexScreener trending tokens
 * as profile-style cards in a responsive grid: the whole page scrolls, so every
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
import { getTokenAccent, fallbackAccent } from "../core/token_accent.js";
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
            <button class="dialog-close" type="button" title="Close (ESC)">
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
    const closeBtn = dialog.querySelector(".dialog-close");
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

    // Escape is owned by the shared stack, so while token details is open on top
    // of the billboard a single Escape closes only the details dialog.
    this._releaseEscape = pushEscapeHandler(() => this.close());

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

    const visible = CATEGORIES.filter((cat) => (this.data[cat.key] || []).length > 0);

    if (visible.length === 0) {
      container.innerHTML = `
        <div class="billboard-state billboard-empty">
          <i class="icon-inbox"></i>
          <span>No tokens available right now</span>
        </div>
      `;
      return;
    }

    container.innerHTML = visible.map((cat) => this._renderCategory(cat)).join("");

    // Clicking a card (outside its action buttons/links) opens token details.
    // The billboard stays open UNDERNEATH (--z-billboard-dialog sits below
    // --z-dialog), so closing the details dialog reveals the billboard again
    // instead of the page below it.
    container.querySelectorAll(".bb-card").forEach((card) => {
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

        const card = btn.closest(".bb-card");
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
        <div class="billboard-cat-grid" id="billboard-cat-${category.id}">
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
      ? `<img src="${this._escapeHtml(bannerUrl)}" alt="" class="bb-card-banner-img" loading="lazy" onerror="this.remove()"/>`
      : "";

    const avatarHtml = logoUrl
      ? `<img src="${this._escapeHtml(logoUrl)}" alt="" class="bb-card-avatar-img" loading="lazy" onerror="this.replaceWith(Object.assign(document.createElement('span'),{className:'bb-card-avatar-initial',textContent:'${initial}'}))"/>`
      : `<span class="bb-card-avatar-initial">${initial}</span>`;

    const change = token.price_change_24h;
    const changeHtml =
      change != null
        ? `<span class="bb-card-change ${change >= 0 ? "pos" : "neg"}">${change > 0 ? "+" : ""}${change.toFixed(1)}%</span>`
        : "";

    const priceHtml =
      token.price_usd != null
        ? `<span class="bb-card-price">${Utils.formatCurrencyUSD(token.price_usd)}</span>`
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
          <div class="bb-card-stat">
            <span class="bb-card-stat-label">${label}</span>
            <span class="bb-card-stat-value">${
              kind === "compact"
                ? Utils.formatCompactNumber(value, { prefix: "$" })
                : Utils.formatCompactNumber(value)
            }</span>
          </div>`
      )
      .join("");

    return `
      <article class="bb-card${token.featured ? " featured" : ""}"
        data-mint="${this._escapeHtml(mint)}"
        data-symbol="${this._escapeHtml(symbol)}"
        style="--bb-hue:${accent.hue};--bb-sat:${accent.saturation}%"
        title="${this._escapeHtml(name)} (${this._escapeHtml(symbol)})">

        <div class="bb-card-banner">${bannerHtml}</div>

        <div class="bb-card-head">
          <div class="bb-card-avatar">${avatarHtml}</div>
        </div>

        <div class="bb-card-identity">
          <div class="bb-card-row">
            <span class="bb-card-name">${this._escapeHtml(name)}</span>
            ${token.featured ? '<i class="icon-star bb-card-featured" title="Featured"></i>' : ""}
            ${priceHtml}
          </div>
          <div class="bb-card-row">
            <span class="bb-card-symbol">${this._escapeHtml(symbol)}</span>
            ${this._renderSecurity(token.security_score)}
            ${changeHtml}
          </div>
        </div>

        ${stats ? `<div class="bb-card-stats">${stats}</div>` : ""}

        <div class="bb-card-actions">
          <div class="bb-card-links">
            ${this._buildSocialIcons(token)}
            ${this._buildShortcuts(mint)}
          </div>
          <button class="bb-card-buy" data-action="buy" title="Buy ${this._escapeHtml(symbol)}">
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
      <button class="bb-card-link" data-action="dexscreener" data-mint="${safeMint}" title="DexScreener">
        <i class="icon-chart-candlestick"></i>
      </button>
      <button class="bb-card-link" data-action="gmgn" data-mint="${safeMint}" title="GMGN">
        <i class="icon-trending-up"></i>
      </button>
      <button class="bb-card-link" data-action="solscan" data-mint="${safeMint}" title="Solscan">
        <i class="icon-search"></i>
      </button>
      <button class="bb-card-link bb-card-copy" data-action="copy" data-mint="${safeMint}" title="Copy mint">
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

    return `<span class="bb-card-security ${level}" title="Security score: ${score}/100">${label}</span>`;
  }

  /**
   * Repaint a card's tint once its logo colour has been sampled. The card may
   * already be gone (dialog closed, category re-rendered), hence the lookup.
   */
  _applyAccent(mint, accent) {
    if (!accent || !this.dialogEl) return;

    const selector = `.bb-card[data-mint="${CSS.escape(mint)}"]`;
    this.dialogEl.querySelectorAll(selector).forEach((card) => {
      card.style.setProperty("--bb-hue", accent.hue);
      card.style.setProperty("--bb-sat", `${accent.saturation}%`);
    });
  }

  _buildSocialIcons(token) {
    const icons = [];
    if (token.website) {
      icons.push(
        `<a href="${this._escapeHtml(token.website)}" target="_blank" rel="noopener noreferrer" class="bb-card-social" title="Website"><i class="icon-globe"></i></a>`
      );
    }
    if (token.twitter) {
      icons.push(
        `<a href="${this._escapeHtml(token.twitter)}" target="_blank" rel="noopener noreferrer" class="bb-card-social" title="Twitter"><i class="icon-twitter"></i></a>`
      );
    }
    if (token.telegram) {
      icons.push(
        `<a href="${this._escapeHtml(token.telegram)}" target="_blank" rel="noopener noreferrer" class="bb-card-social" title="Telegram"><i class="icon-send"></i></a>`
      );
    }
    if (token.discord) {
      icons.push(
        `<a href="${this._escapeHtml(token.discord)}" target="_blank" rel="noopener noreferrer" class="bb-card-social" title="Discord"><i class="icon-message-circle"></i></a>`
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
