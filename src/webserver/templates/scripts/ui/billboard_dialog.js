/**
 * Billboard Dialog - Shows featured tokens from the website
 *
 * Displays community-submitted tokens with their logos, descriptions,
 * social links, and optional featured badge for highlighted tokens.
 */

import { $ } from "../core/dom.js";

const DIALOG_ID = "billboard-dialog";

class BillboardDialog {
  constructor() {
    this.isOpen = false;
    this.tokens = [];
    this.dialogEl = null;
  }

  async open() {
    if (this.isOpen) return;
    this.isOpen = true;

    this._createDialog();
    this._showLoading();

    try {
      const response = await fetch("/api/billboard");
      const data = await response.json();

      if (data.success && data.tokens) {
        this.tokens = data.tokens;
        this._renderTokens();
      } else {
        this._showError(data.error || "Failed to load billboard");
      }
    } catch (e) {
      this._showError("Network error: " + e.message);
    }
  }

  close() {
    if (this.dialogEl) {
      this.dialogEl.remove();
      this.dialogEl = null;
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
      <div class="billboard-dialog">
        <div class="billboard-header">
          <div class="billboard-title">
            <i class="icon-megaphone"></i>
            <h2>Billboard</h2>
          </div>
          <p class="billboard-subtitle">Featured tokens from the community</p>
          <button class="billboard-close-btn" title="Close">
            <i class="icon-x"></i>
          </button>
        </div>
        <div class="billboard-body">
          <div class="billboard-tokens" id="billboard-tokens"></div>
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
    const container = $("#billboard-tokens");
    if (container) {
      container.innerHTML = `
        <div class="billboard-state billboard-loading">
          <i class="icon-loader spin"></i>
          <span>Loading featured tokens...</span>
        </div>
      `;
    }
  }

  _showError(message) {
    const container = $("#billboard-tokens");
    if (container) {
      container.innerHTML = `
        <div class="billboard-state billboard-error">
          <i class="icon-alert-circle"></i>
          <span>${this._escapeHtml(message)}</span>
        </div>
      `;
    }
  }

  _renderTokens() {
    const container = $("#billboard-tokens");
    if (!container) return;

    if (this.tokens.length === 0) {
      container.innerHTML = `
        <div class="billboard-state billboard-empty">
          <i class="icon-inbox"></i>
          <span>No featured tokens yet</span>
          <a href="https://screenerbot.io/submit-token" target="_blank" rel="noopener noreferrer">
            Be the first to submit!
          </a>
        </div>
      `;
      return;
    }

    container.innerHTML = this.tokens.map((token) => this._renderTokenCard(token)).join("");

    // Attach copy handlers
    container.querySelectorAll(".billboard-copy-btn").forEach((btn) => {
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

  _renderTokenCard(token) {
    const logoUrl = token.logo || "/icon.png";
    const socials = this._buildSocialLinks(token);
    const featuredClass = token.featured ? "featured" : "";

    return `
      <div class="billboard-token-card ${featuredClass}">
        ${token.banner ? `<div class="billboard-token-banner" style="background-image: url('${this._escapeHtml(token.banner)}')"></div>` : ""}
        <div class="billboard-token-main">
          <img 
            src="${this._escapeHtml(logoUrl)}" 
            alt="${this._escapeHtml(token.symbol)}" 
            class="billboard-token-logo" 
            onerror="this.src='/icon.png'"
          />
          <div class="billboard-token-info">
            <h3 class="billboard-token-name">${this._escapeHtml(token.name)}</h3>
            <span class="billboard-token-symbol">${this._escapeHtml(token.symbol)}</span>
          </div>
          ${token.featured ? '<span class="billboard-featured-badge"><i class="icon-star"></i> Featured</span>' : ""}
        </div>
        <div class="billboard-token-mint">
          <span class="billboard-mint-address">${token.mint}</span>
          <button class="billboard-copy-btn" data-mint="${token.mint}" title="Copy mint address">
            <i class="icon-copy"></i>
          </button>
        </div>
        ${token.description ? `<p class="billboard-token-description">${this._escapeHtml(token.description)}</p>` : ""}
        <div class="billboard-token-links">
          ${token.website ? `<a href="${this._escapeHtml(token.website)}" target="_blank" rel="noopener noreferrer" class="billboard-link-btn" title="Website"><i class="icon-globe"></i></a>` : ""}
          ${socials}
        </div>
      </div>
    `;
  }

  _buildSocialLinks(token) {
    const links = [];
    if (token.twitter) {
      links.push(
        `<a href="${this._escapeHtml(token.twitter)}" target="_blank" rel="noopener noreferrer" class="billboard-link-btn" title="Twitter/X"><i class="icon-twitter"></i></a>`
      );
    }
    if (token.telegram) {
      links.push(
        `<a href="${this._escapeHtml(token.telegram)}" target="_blank" rel="noopener noreferrer" class="billboard-link-btn" title="Telegram"><i class="icon-send"></i></a>`
      );
    }
    if (token.discord) {
      links.push(
        `<a href="${this._escapeHtml(token.discord)}" target="_blank" rel="noopener noreferrer" class="billboard-link-btn" title="Discord"><i class="icon-message-circle"></i></a>`
      );
    }
    return links.join("");
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
