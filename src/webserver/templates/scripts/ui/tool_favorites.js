// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tool Favorites Component
 * Reusable favorites dropdown for trading tools
 */

import { $, on } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { openMenu, closeMenu } from "../core/menu_manager.js";
import { ConfirmationDialog } from "./confirmation_dialog.js";
import { InputDialog } from "./input_dialog.js";

export class ToolFavorites {
  constructor(options) {
    this.toolType = options.toolType; // e.g., 'volume_aggregator'
    this.container = options.container; // Container element or selector
    this.onSelect = options.onSelect || (() => {}); // Callback when favorite selected
    this.getConfig = options.getConfig || (() => ({})); // Get current form config
    this.favorites = [];
    this.isOpen = false;
    this._closeTimer = null;
    this._menuHandle = {
      close: (reason) =>
        this.closeDropdown({
          restoreFocus: reason === "escape",
          immediate: [
            "superseded",
            "outside-pointer",
            "focus-left",
            "document-hidden",
            "navigation",
            "dialog-open",
          ].includes(reason),
        }),
      owns: (target) =>
        (this.triggerBtn && this.triggerBtn.contains(target)) ||
        (this.dropdown && this.dropdown.contains(target)),
    };

    this.init();
  }

  async init() {
    this.render();
    await this.loadFavorites();
    this.bindEvents();
  }

  render() {
    const containerEl = typeof this.container === "string" ? $(this.container) : this.container;
    if (!containerEl) return;

    containerEl.innerHTML = `
      <div class="tool-favorites">
        <button class="tool-favorites-trigger" type="button" aria-haspopup="menu" aria-expanded="false">
          <i class="icon-star"></i>
          <span>${Utils.escapeHtml(I18n.t("tools-favorites-title"))}</span>
          <span class="favorites-count" style="display: none;">0</span>
          <i class="icon-chevron-down"></i>
        </button>
        <div class="tool-favorites-dropdown" role="menu" hidden>
          <div class="favorites-header">
            <span>${Utils.escapeHtml(I18n.t("tools-favorites-saved"))}</span>
            <button class="btn btn-xs" id="favorites-save-btn" type="button" role="menuitem">
              <i class="icon-plus"></i> ${Utils.escapeHtml(I18n.t("tools-favorites-save-current"))}
            </button>
          </div>
          <div class="favorites-list">
            <div class="favorites-empty">${Utils.escapeHtml(I18n.t("tools-favorites-empty"))}</div>
          </div>
        </div>
      </div>
    `;

    this.triggerBtn = containerEl.querySelector(".tool-favorites-trigger");
    this.dropdown = containerEl.querySelector(".tool-favorites-dropdown");
    this.countBadge = containerEl.querySelector(".favorites-count");
    this.listEl = containerEl.querySelector(".favorites-list");
    this.saveBtn = containerEl.querySelector("#favorites-save-btn");
  }

  bindEvents() {
    if (this.triggerBtn) {
      on(this.triggerBtn, "click", (e) => {
        e.stopPropagation();
        this.toggleDropdown();
      });
    }

    if (this.saveBtn) {
      on(this.saveBtn, "click", (e) => {
        e.stopPropagation();
        this.saveCurrent();
      });
    }

    this._keyHandler = (event) => {
      if (!this.isOpen) return;
      const items = Array.from(this.dropdown?.querySelectorAll("[role='menuitem']") || []).filter(
        (item) => !item.disabled
      );
      const index = items.indexOf(document.activeElement);

      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const direction = event.key === "ArrowDown" ? 1 : -1;
        const nextIndex =
          index < 0
            ? direction > 0
              ? 0
              : items.length - 1
            : (index + direction + items.length) % items.length;
        items[nextIndex]?.focus();
      } else if (event.key === "Home" || event.key === "End") {
        event.preventDefault();
        items[event.key === "Home" ? 0 : items.length - 1]?.focus();
      }
    };
    this.dropdown?.addEventListener("keydown", this._keyHandler);
  }

  toggleDropdown() {
    this.isOpen ? this.closeDropdown() : this.openDropdown();
  }

  openDropdown() {
    if (this.dropdown) {
      if (this._closeTimer !== null) {
        clearTimeout(this._closeTimer);
        this._closeTimer = null;
      }
      openMenu(this._menuHandle);
      this.dropdown.hidden = false;
      this.isOpen = true;
      this.triggerBtn?.classList.add("active");
      this.triggerBtn?.setAttribute("aria-expanded", "true");
      requestAnimationFrame(() => {
        if (!this.isOpen) return;
        this.dropdown.classList.add("open");
        this.saveBtn?.focus({ preventScroll: true });
      });
    }
  }

  closeDropdown({ restoreFocus = false, immediate = false } = {}) {
    if (this.dropdown) {
      closeMenu(this._menuHandle);
      this.dropdown.classList.remove("open");
      this.isOpen = false;
      this.triggerBtn?.setAttribute("aria-expanded", "false");
      if (restoreFocus) this.triggerBtn?.focus({ preventScroll: true });
      if (this._closeTimer !== null) clearTimeout(this._closeTimer);
      const finish = () => {
        this._closeTimer = null;
        if (this.isOpen) return;
        this.dropdown.hidden = true;
        this.triggerBtn?.classList.remove("active");
      };
      if (immediate) finish();
      else this._closeTimer = setTimeout(finish, 220);
    }
  }

  async loadFavorites() {
    try {
      const response = await fetch(`/api/tools/favorites?tool_type=${this.toolType}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const result = await response.json();
      this.favorites = result.data?.favorites || result.favorites || [];
      this.renderFavoritesList();
    } catch (error) {
      console.error("Failed to load favorites:", error);
    }
  }

  renderFavoritesList() {
    if (!this.listEl) return;

    // Update count badge
    if (this.countBadge) {
      this.countBadge.textContent = this.favorites.length;
      this.countBadge.style.display = this.favorites.length > 0 ? "inline" : "none";
    }

    if (this.favorites.length === 0) {
      this.listEl.innerHTML = `<div class="favorites-empty">${Utils.escapeHtml(I18n.t("tools-favorites-empty"))}</div>`;
      return;
    }

    // Sort by use_count desc
    const sorted = [...this.favorites].sort((a, b) => (b.use_count || 0) - (a.use_count || 0));

    this.listEl.innerHTML = sorted
      .map(
        (fav) => `
      <div class="favorite-item" data-id="${Utils.escapeHtml(fav.id)}">
        <button class="favorite-info" data-action="select" type="button" role="menuitem">
          <div class="favorite-token">
            ${fav.logo_url ? `<img src="${Utils.escapeHtml(fav.logo_url)}" class="favorite-logo token-logo-artwork" alt="">` : '<i class="icon-circle"></i>'}
            <span class="favorite-symbol token-symbol-type">${Utils.escapeHtml(fav.symbol || fav.mint.slice(0, 6))}</span>
          </div>
          <div class="favorite-label">${fav.label ? Utils.escapeHtml(fav.label) : Utils.escapeHtml(I18n.t("tools-favorites-no-label"))}</div>
          ${fav.use_count > 0 ? `<span class="favorite-uses">${Utils.escapeHtml(I18n.t("tools-favorites-uses", { count: fav.use_count }))}</span>` : ""}
        </button>
        <button class="favorite-delete-btn" data-action="delete" type="button" role="menuitem" title="${Utils.escapeHtml(I18n.t("tools-favorites-remove"))}">
          <i class="icon-x"></i>
        </button>
      </div>
    `
      )
      .join("");

    // Bind click events for items
    this.listEl.querySelectorAll(".favorite-item").forEach((item) => {
      const id = parseInt(item.dataset.id, 10);
      const fav = this.favorites.find((f) => f.id === id);

      item.querySelector('[data-action="select"]')?.addEventListener("click", () => {
        this.selectFavorite(fav);
      });

      item.querySelector('[data-action="delete"]')?.addEventListener("click", (e) => {
        e.stopPropagation();
        this.deleteFavorite(id);
      });
    });
  }

  async selectFavorite(favorite) {
    if (!favorite) return;

    // Mark as used
    try {
      await fetch(`/api/tools/favorites/${favorite.id}/use`, { method: "POST" });
    } catch (e) {
      console.warn("Failed to mark favorite as used:", e);
    }

    // Parse config and call callback
    let config = {};
    if (favorite.config_json) {
      try {
        config = JSON.parse(favorite.config_json);
      } catch (e) {
        console.warn("Failed to parse favorite config:", e);
      }
    }

    this.closeDropdown();
    this.onSelect({ ...favorite, config });

    Utils.showToast(
      I18n.t("tools-favorites-loaded", {
        name: favorite.label || favorite.symbol || I18n.t("tools-favorites-default-name"),
      }),
      "success"
    );
  }

  async saveCurrent() {
    // Get current config from the form
    const config = this.getConfig();
    if (!config.mint) {
      Utils.showToast(I18n.t("tools-favorites-mint-required"), "warning");
      return;
    }

    // Prompt for label
    const result = await InputDialog.show({
      title: I18n.t("tools-favorites-add-title"),
      message: I18n.t("tools-favorites-add-message"),
      placeholder: I18n.t("tools-favorites-add-placeholder"),
      defaultValue: config.symbol || "",
      confirmLabel: I18n.t("common-action-save"),
    });
    if (!result) return; // Cancelled
    const label = result.value;

    try {
      const response = await fetch("/api/tools/favorites", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          mint: config.mint,
          symbol: config.symbol || null,
          name: config.name || null,
          logo_url: config.logo_url || null,
          tool_type: this.toolType,
          config_json: JSON.stringify(config),
          label: label || null,
        }),
      });

      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      Utils.showToast(I18n.t("tools-favorites-saved-toast"), "success");
      await this.loadFavorites();
    } catch (error) {
      console.error("Failed to save favorite:", error);
      Utils.showToast(I18n.t("tools-favorites-save-failed"), "error");
    }
  }

  async deleteFavorite(id) {
    const result = await ConfirmationDialog.show({
      title: I18n.t("tools-favorites-remove-title"),
      message: I18n.t("tools-favorites-remove-message"),
      confirmLabel: I18n.t("common-action-remove"),
      variant: "warning",
    });
    if (!result.confirmed) return;

    try {
      const response = await fetch(`/api/tools/favorites/${id}`, {
        method: "DELETE",
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      Utils.showToast(I18n.t("tools-favorites-removed-toast"), "success");
      await this.loadFavorites();
    } catch (error) {
      console.error("Failed to delete favorite:", error);
      Utils.showToast(I18n.t("tools-favorites-remove-failed"), "error");
    }
  }

  dispose() {
    this.closeDropdown();
    if (this._closeTimer !== null) {
      clearTimeout(this._closeTimer);
      this._closeTimer = null;
      this.dropdown.hidden = true;
      this.triggerBtn?.classList.remove("active");
    }
    this.dropdown?.removeEventListener("keydown", this._keyHandler);
    this._keyHandler = null;
  }
}

export default ToolFavorites;
