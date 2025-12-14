/**
 * Tool Favorites Component
 * Reusable favorites dropdown for trading tools
 */

import { $, on } from "../core/dom.js";
import * as Utils from "../core/utils.js";

export class ToolFavorites {
  constructor(options) {
    this.toolType = options.toolType; // e.g., 'volume_aggregator'
    this.container = options.container; // Container element or selector
    this.onSelect = options.onSelect || (() => {}); // Callback when favorite selected
    this.getConfig = options.getConfig || (() => ({})); // Get current form config
    this.favorites = [];
    this.isOpen = false;

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
        <button class="tool-favorites-trigger" type="button">
          <i class="icon-star"></i>
          <span>Favorites</span>
          <span class="favorites-count" style="display: none;">0</span>
          <i class="icon-chevron-down"></i>
        </button>
        <div class="tool-favorites-dropdown" style="display: none;">
          <div class="favorites-header">
            <span>Saved Favorites</span>
            <button class="btn btn-xs" id="favorites-save-btn" type="button">
              <i class="icon-plus"></i> Save Current
            </button>
          </div>
          <div class="favorites-list">
            <div class="favorites-empty">No favorites saved yet</div>
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

    // Close dropdown when clicking outside
    this._outsideClickHandler = (e) => {
      if (this.isOpen && !e.target.closest(".tool-favorites")) {
        this.closeDropdown();
      }
    };
    document.addEventListener("click", this._outsideClickHandler);
  }

  toggleDropdown() {
    this.isOpen ? this.closeDropdown() : this.openDropdown();
  }

  openDropdown() {
    if (this.dropdown) {
      this.dropdown.style.display = "block";
      this.isOpen = true;
    }
  }

  closeDropdown() {
    if (this.dropdown) {
      this.dropdown.style.display = "none";
      this.isOpen = false;
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
      this.listEl.innerHTML = '<div class="favorites-empty">No favorites saved yet</div>';
      return;
    }

    // Sort by use_count desc
    const sorted = [...this.favorites].sort((a, b) => (b.use_count || 0) - (a.use_count || 0));

    this.listEl.innerHTML = sorted
      .map(
        (fav) => `
      <div class="favorite-item" data-id="${fav.id}">
        <div class="favorite-info" data-action="select">
          <div class="favorite-token">
            ${fav.logo_url ? `<img src="${fav.logo_url}" class="favorite-logo" alt="">` : '<i class="icon-circle"></i>'}
            <span class="favorite-symbol">${fav.symbol || fav.mint.slice(0, 6)}</span>
          </div>
          <div class="favorite-label">${fav.label || "No label"}</div>
          ${fav.use_count > 0 ? `<span class="favorite-uses">${fav.use_count}x</span>` : ""}
        </div>
        <button class="favorite-delete-btn" data-action="delete" type="button" title="Remove">
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

    Utils.showToast(`Loaded favorite: ${favorite.label || favorite.symbol || "Config"}`, "success");
  }

  async saveCurrent() {
    // Get current config from the form
    const config = this.getConfig();
    if (!config.mint) {
      Utils.showToast("Please enter a token mint address first", "warning");
      return;
    }

    // Prompt for label
    const label = window.prompt("Enter a label for this favorite:", config.symbol || "");
    if (label === null) return; // Cancelled

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

      Utils.showToast("Saved to favorites", "success");
      await this.loadFavorites();
    } catch (error) {
      console.error("Failed to save favorite:", error);
      Utils.showToast("Failed to save favorite", "error");
    }
  }

  async deleteFavorite(id) {
    if (!window.confirm("Remove this favorite?")) return;

    try {
      const response = await fetch(`/api/tools/favorites/${id}`, {
        method: "DELETE",
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      Utils.showToast("Favorite removed", "success");
      await this.loadFavorites();
    } catch (error) {
      console.error("Failed to delete favorite:", error);
      Utils.showToast("Failed to remove favorite", "error");
    }
  }

  dispose() {
    if (this._outsideClickHandler) {
      document.removeEventListener("click", this._outsideClickHandler);
    }
  }
}

export default ToolFavorites;
