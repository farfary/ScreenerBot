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

  function applyBuildersMixin(manager) {
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
        label: "Buy Token",
        icon: "shoppingCart",
        className: "success",
        action: () => this._buyToken(context),
      });

      items.push({
        type: "item",
        label: "Sell Token",
        icon: "trendingDown",
        className: "danger",
        action: () => this._sellToken(context),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: "View Details",
        icon: "eye",
        shortcut: "Enter",
        action: () => this._viewTokenDetails(context),
      });

      // Favorites toggle
      const isFavorite = this._isFavorite(context.mint);
      items.push({
        type: "item",
        label: isFavorite ? "Remove from Favorites" : "Add to Favorites",
        icon: "star",
        className: isFavorite ? "favorite-active" : "",
        action: () => this._toggleFavorite(context, isFavorite),
      });

      items.push({
        type: "item",
        label: "View on Explorer",
        icon: "externalLink",
        submenu: [
          { type: "header", label: "Trading" },
          {
            type: "item",
            label: "DexScreener",
            icon: "chart",
            action: () => this._openExplorer(context.mint, "dexscreener"),
          },
          {
            type: "item",
            label: "Birdeye",
            icon: "eye",
            action: () => this._openExplorer(context.mint, "birdeye"),
          },
          {
            type: "item",
            label: "Photon",
            icon: "zap",
            action: () => this._openExplorer(context.mint, "photon"),
          },
          { type: "separator" },
          { type: "header", label: "Analysis" },
          {
            type: "item",
            label: "RugCheck",
            icon: "shield",
            action: () => this._openExplorer(context.mint, "rugcheck"),
          },
          {
            type: "item",
            label: "Bubblemaps",
            icon: "globe",
            action: () => this._openExplorer(context.mint, "bubblemaps"),
          },
          { type: "separator" },
          { type: "header", label: "Explorers" },
          {
            type: "item",
            label: "Solscan",
            icon: "globe",
            action: () => this._openExplorer(context.mint, "solscan"),
          },
          {
            type: "item",
            label: "Solana FM",
            icon: "globe",
            action: () => this._openExplorer(context.mint, "solanafm"),
          },
        ],
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: "Copy Address",
        icon: "copy",
        shortcut: this._getModKey() + "C",
        action: () => this._copyToClipboard(context.mint, "Token address"),
      });

      items.push({
        type: "item",
        label: "Copy Symbol",
        icon: "copy",
        action: () => this._copyToClipboard(context.symbol, "Symbol"),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: "Blacklist Token",
        icon: "ban",
        className: "danger",
        action: () => this._blacklistToken(context),
      });

      items.push({
        type: "item",
        label: "Refresh Data",
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
        label: `Sell ${context.symbol}`,
        icon: "trendingDown",
        className: "danger",
        action: () => this._sellToken(context),
      });

      items.push({
        type: "item",
        label: "Add to Position",
        icon: "plus",
        className: "success",
        action: () => this._addToPosition(context),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: "View Details",
        icon: "eye",
        action: () => this._viewPositionDetails(context),
      });

      // Manual-management toggle. Current state is read straight from the row DOM
      // (the row carries the `pos-ind-manual` class only when manual management is on),
      // so the menu label is always accurate without extra plumbing.
      const isManual = !!context.element?.classList?.contains("pos-ind-manual");
      items.push({
        type: "item",
        label: isManual ? "Disable manual management" : "Enable manual management",
        icon: "shield",
        action: () => this._toggleManualManagement(context, !isManual),
      });

      // Favorites toggle
      const isFavorite = this._isFavorite(context.mint);
      items.push({
        type: "item",
        label: isFavorite ? "Remove from Favorites" : "Add to Favorites",
        icon: "star",
        className: isFavorite ? "favorite-active" : "",
        action: () => this._toggleFavorite(context, isFavorite),
      });

      items.push({
        type: "item",
        label: "View on Explorer",
        icon: "externalLink",
        submenu: [
          { type: "header", label: "Trading" },
          {
            type: "item",
            label: "DexScreener",
            icon: "chart",
            action: () => this._openExplorer(context.mint, "dexscreener"),
          },
          {
            type: "item",
            label: "Birdeye",
            icon: "eye",
            action: () => this._openExplorer(context.mint, "birdeye"),
          },
          {
            type: "item",
            label: "Photon",
            icon: "zap",
            action: () => this._openExplorer(context.mint, "photon"),
          },
          { type: "separator" },
          { type: "header", label: "Analysis" },
          {
            type: "item",
            label: "RugCheck",
            icon: "shield",
            action: () => this._openExplorer(context.mint, "rugcheck"),
          },
          {
            type: "item",
            label: "Bubblemaps",
            icon: "globe",
            action: () => this._openExplorer(context.mint, "bubblemaps"),
          },
          { type: "separator" },
          { type: "header", label: "Explorers" },
          {
            type: "item",
            label: "Solscan",
            icon: "globe",
            action: () => this._openExplorer(context.mint, "solscan"),
          },
          {
            type: "item",
            label: "Solana FM",
            icon: "globe",
            action: () => this._openExplorer(context.mint, "solanafm"),
          },
        ],
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: "Copy Address",
        icon: "copy",
        action: () => this._copyToClipboard(context.mint, "Token address"),
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
        label: "View on Solscan",
        icon: "externalLink",
        action: () => window.open(`https://solscan.io/tx/${context.signature}`, "_blank"),
      });

      items.push({
        type: "item",
        label: "View on Solana FM",
        icon: "globe",
        action: () => window.open(`https://solana.fm/tx/${context.signature}`, "_blank"),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: "Copy Signature",
        icon: "copy",
        shortcut: this._getModKey() + "C",
        action: () => this._copyToClipboard(context.signature, "Transaction signature"),
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
        label: "Open Link",
        icon: "externalLink",
        action: () => window.open(context.href, "_blank"),
      });

      items.push({
        type: "item",
        label: "Open in New Tab",
        icon: "plus",
        action: () => window.open(context.href, "_blank"),
      });

      items.push({ type: "separator" });

      items.push({
        type: "item",
        label: "Copy Link Address",
        icon: "copy",
        shortcut: this._getModKey() + "C",
        action: () => this._copyToClipboard(context.href, "Link"),
      });

      items.push({
        type: "item",
        label: "Copy Link Text",
        icon: "copy",
        action: () => this._copyToClipboard(context.text, "Link text"),
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
        label: "Open Image",
        icon: "externalLink",
        action: () => window.open(context.src, "_blank"),
      });

      items.push({
        type: "item",
        label: "Copy Image Address",
        icon: "copy",
        action: () => this._copyToClipboard(context.src, "Image URL"),
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
        label: "Copy",
        icon: "copy",
        shortcut: this._getModKey() + "C",
        action: () => this._copyToClipboard(context.text, "Text"),
      });

      items.push({
        type: "item",
        label: "Search on Google",
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
          label: "View on Solscan",
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
    manager._buildDefaultMenu = function (items, context) {
      items.push({
        type: "item",
        label: "Back",
        icon: "arrowLeft",
        shortcut: this._getModKey() + "[",
        disabled: !window.history.length,
        action: () => window.history.back(),
      });

      items.push({
        type: "item",
        label: "Reload",
        icon: "refresh",
        shortcut: this._getModKey() + "R",
        action: () => window.location.reload(),
      });
    };
  }

  // Export mixin
  window.ContextMenuBuilders = { apply: applyBuildersMixin };
})();
