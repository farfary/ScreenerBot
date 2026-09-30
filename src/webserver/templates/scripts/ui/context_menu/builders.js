/**
 * Context Menu Builders
 * Menu builder methods for the ContextMenuManager
 *
 * These methods build menu item arrays for:
 * - Token menus (trading, favorites, explorers)
 * - Position menus
 * - Transaction menus
 * - Link/image/selection menus
 * - Default page menus
 *
 * Applied as a mixin to ContextMenuManager instance.
 */

(function () {
  "use strict";

  function applyBuildersMixin(manager, { positionManagementLabels }) {
    // =========================================================================
    // Token Menu Builder
    // =========================================================================

    /**
     * Build menu items for token context
     */
    manager._buildTokenMenu = function (items, context) {
      // Token preview header
      items.push({
        type: "token-preview",
        symbol: context.symbol,
        name: context.name,
        icon: context.icon,
      });

      items.push({ type: "separator" });

      // Token actions
      items.push({
        type: "item",
        label: I18n.t("menu-token-buy"),
        icon: "shoppingCart",
        className: "success",
        action: () => this._buyToken(context),
      });

      items.push({
        type: "item",
        label: I18n.t("menu-token-sell"),
        icon: "trendingDown",
        className: "danger",
        action: () => this._sellToken(context),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: I18n.t("menu-view-details"),
        icon: "eye",
        shortcut: "Enter",
        action: () => this._viewTokenDetails(context),
      });

      // Favorites toggle
      const isFavorite = this._isFavorite(context.mint);
      items.push({
        type: "item",
        label: isFavorite
          ? I18n.t("menu-favorite-remove")
          : I18n.t("menu-favorite-add"),
        icon: "star",
        className: isFavorite ? "favorite-active" : "",
        action: () => this._toggleFavorite(context, isFavorite),
      });

      items.push({
        type: "item",
        label: I18n.t("links-explorer-open"),
        icon: "externalLink",
        submenu: [
          { type: "header", label: I18n.t("links-explorer-group-trading") },
          {
            type: "item",
            label: I18n.t("links-explorer-dexscreener"),
            icon: "chart",
            action: () => this._openExplorer(context.mint, "dexscreener"),
          },
          {
            type: "item",
            label: I18n.t("links-explorer-birdeye"),
            icon: "eye",
            action: () => this._openExplorer(context.mint, "birdeye"),
          },
          {
            type: "item",
            label: I18n.t("links-explorer-photon"),
            icon: "zap",
            action: () => this._openExplorer(context.mint, "photon"),
          },
          { type: "separator" },
          { type: "header", label: I18n.t("links-explorer-group-analysis") },
          {
            type: "item",
            label: I18n.t("links-explorer-rugcheck"),
            icon: "shield",
            action: () => this._openExplorer(context.mint, "rugcheck"),
          },
          {
            type: "item",
            label: I18n.t("links-explorer-bubblemaps"),
            icon: "globe",
            action: () => this._openExplorer(context.mint, "bubblemaps"),
          },
          { type: "separator" },
          { type: "header", label: I18n.t("links-explorer-group-explorers") },
          {
            type: "item",
            label: I18n.t("links-explorer-solscan"),
            icon: "globe",
            action: () => this._openExplorer(context.mint, "solscan"),
          },
          {
            type: "item",
            label: I18n.t("links-explorer-solana-fm"),
            icon: "globe",
            action: () => this._openExplorer(context.mint, "solanafm"),
          },
        ],
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: I18n.t("menu-copy-address"),
        icon: "copy",
        shortcut: this._getModKey() + "C",
        action: () => this._copyToClipboard(context.mint, I18n.t("menu-copied-token-address")),
      });

      items.push({
        type: "item",
        label: I18n.t("menu-copy-symbol"),
        icon: "copy",
        action: () => this._copyToClipboard(context.symbol, I18n.t("menu-copied-symbol")),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: I18n.t("menu-token-blacklist"),
        icon: "ban",
        className: "danger",
        action: () => this._blacklistToken(context),
      });

      items.push({
        type: "item",
        label: I18n.t("menu-token-refresh"),
        icon: "refresh",
        action: () => this._refreshToken(context),
      });
    };

    // =========================================================================
    // Position Menu Builder
    // =========================================================================

    /**
     * Build menu items for position context
     */
    manager._buildPositionMenu = function (items, context) {
      items.push({
        type: "item",
        label: I18n.t("menu-position-sell", { symbol: context.symbol }),
        icon: "trendingDown",
        className: "danger",
        action: () => this._sellToken(context),
      });

      items.push({
        type: "item",
        label: I18n.t("menu-position-add"),
        icon: "plus",
        className: "success",
        action: () => this._addToPosition(context),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: I18n.t("menu-view-details"),
        icon: "eye",
        action: () => this._viewPositionDetails(context),
      });

      const management = context.element?.dataset?.management || "auto_trader";
      const modes = ["auto_trader", "user_only"];
      if (context.element?.dataset?.originKind === "copy") {
        modes.push("copy_task", "hybrid");
      }
      items.push({
        type: "item",
        label: I18n.t("menu-position-management"),
        icon: "shield",
        submenu: modes.map((value) => ({
          type: "item",
          label:
            value === management
              ? `✓ ${I18n.label(positionManagementLabels, value)}`
              : I18n.label(positionManagementLabels, value),
          action: () => this._setPositionManagement(context, value),
        })),
      });

      // Favorites toggle
      const isFavorite = this._isFavorite(context.mint);
      items.push({
        type: "item",
        label: isFavorite
          ? I18n.t("menu-favorite-remove")
          : I18n.t("menu-favorite-add"),
        icon: "star",
        className: isFavorite ? "favorite-active" : "",
        action: () => this._toggleFavorite(context, isFavorite),
      });

      items.push({
        type: "item",
        label: I18n.t("links-explorer-open"),
        icon: "externalLink",
        submenu: [
          { type: "header", label: I18n.t("links-explorer-group-trading") },
          {
            type: "item",
            label: I18n.t("links-explorer-dexscreener"),
            icon: "chart",
            action: () => this._openExplorer(context.mint, "dexscreener"),
          },
          {
            type: "item",
            label: I18n.t("links-explorer-birdeye"),
            icon: "eye",
            action: () => this._openExplorer(context.mint, "birdeye"),
          },
          {
            type: "item",
            label: I18n.t("links-explorer-photon"),
            icon: "zap",
            action: () => this._openExplorer(context.mint, "photon"),
          },
          { type: "separator" },
          { type: "header", label: I18n.t("links-explorer-group-analysis") },
          {
            type: "item",
            label: I18n.t("links-explorer-rugcheck"),
            icon: "shield",
            action: () => this._openExplorer(context.mint, "rugcheck"),
          },
          {
            type: "item",
            label: I18n.t("links-explorer-bubblemaps"),
            icon: "globe",
            action: () => this._openExplorer(context.mint, "bubblemaps"),
          },
          { type: "separator" },
          { type: "header", label: I18n.t("links-explorer-group-explorers") },
          {
            type: "item",
            label: I18n.t("links-explorer-solscan"),
            icon: "globe",
            action: () => this._openExplorer(context.mint, "solscan"),
          },
          {
            type: "item",
            label: I18n.t("links-explorer-solana-fm"),
            icon: "globe",
            action: () => this._openExplorer(context.mint, "solanafm"),
          },
        ],
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: I18n.t("menu-copy-address"),
        icon: "copy",
        action: () => this._copyToClipboard(context.mint, I18n.t("menu-copied-token-address")),
      });
    };

    // =========================================================================
    // Transaction Menu Builder
    // =========================================================================

    /**
     * Build menu items for transaction context
     */
    manager._buildTransactionMenu = function (items, context) {
      items.push({
        type: "item",
        label: I18n.t("links-view-solscan"),
        icon: "externalLink",
        action: () => window.open(`https://solscan.io/tx/${context.signature}`, "_blank"),
      });

      items.push({
        type: "item",
        label: I18n.t("links-view-solana-fm"),
        icon: "globe",
        action: () => window.open(`https://solana.fm/tx/${context.signature}`, "_blank"),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: I18n.t("menu-copy-signature"),
        icon: "copy",
        shortcut: this._getModKey() + "C",
        action: () => this._copyToClipboard(context.signature, I18n.t("menu-copied-transaction-signature")),
      });
    };

    // =========================================================================
    // Link Menu Builder
    // =========================================================================

    /**
     * Build menu items for link context
     */
    manager._buildLinkMenu = function (items, context) {
      items.push({
        type: "item",
        label: I18n.t("menu-link-open"),
        icon: "externalLink",
        action: () => window.open(context.href, "_blank"),
      });

      items.push({
        type: "item",
        label: I18n.t("menu-link-open-new-tab"),
        icon: "plus",
        action: () => window.open(context.href, "_blank"),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: I18n.t("menu-copy-link-address"),
        icon: "copy",
        shortcut: this._getModKey() + "C",
        action: () => this._copyToClipboard(context.href, I18n.t("menu-copied-link")),
      });

      items.push({
        type: "item",
        label: I18n.t("menu-copy-link-text"),
        icon: "copy",
        action: () => this._copyToClipboard(context.text, I18n.t("menu-copied-link-text")),
      });
    };

    // =========================================================================
    // Image Menu Builder
    // =========================================================================

    /**
     * Build menu items for image context
     */
    manager._buildImageMenu = function (items, context) {
      items.push({
        type: "item",
        label: I18n.t("menu-image-open"),
        icon: "externalLink",
        action: () => window.open(context.src, "_blank"),
      });

      items.push({
        type: "item",
        label: I18n.t("menu-copy-image-address"),
        icon: "copy",
        action: () => this._copyToClipboard(context.src, I18n.t("menu-copied-image-url")),
      });
    };

    // =========================================================================
    // Selection Menu Builder
    // =========================================================================

    /**
     * Build menu items for text selection context
     */
    manager._buildSelectionMenu = function (items, context) {
      items.push({
        type: "item",
        label: I18n.t("common-action-copy"),
        icon: "copy",
        shortcut: this._getModKey() + "C",
        action: () => this._copyToClipboard(context.text, I18n.t("menu-copied-text")),
      });

      items.push({
        type: "item",
        label: I18n.t("menu-search-google"),
        icon: "search",
        action: () =>
          window.open(
            `https://www.google.com/search?q=${encodeURIComponent(context.text)}`,
            "_blank"
          ),
      });

      // Check if it looks like a Solana address (base58, 32-44 chars)
      if (/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(context.text.trim())) {
        items.push({ type: "separator" });

        items.push({
          type: "item",
          label: I18n.t("links-view-solscan"),
          icon: "globe",
          action: () => window.open(`https://solscan.io/account/${context.text.trim()}`, "_blank"),
        });
      }
    };

    // =========================================================================
    // Default Menu Builder
    // =========================================================================

    /**
     * Build menu items for default page context
     */
    manager._buildDefaultMenu = function (items, _context) {
      items.push({
        type: "item",
        label: I18n.t("common-action-back"),
        icon: "arrowLeft",
        shortcut: this._getModKey() + "[",
        disabled: !window.history.length,
        action: () => window.history.back(),
      });

      items.push({
        type: "item",
        label: I18n.t("menu-page-reload"),
        icon: "refresh",
        shortcut: this._getModKey() + "R",
        action: () => window.location.reload(),
      });
    };
  }

  // Export mixin
  window.ContextMenuBuilders = { apply: applyBuildersMixin };
})();
