/**
 * Context Menu Actions
 * Action handler methods for the ContextMenuManager
 *
 * These methods handle:
 * - Token trading actions (buy, sell, add to position)
 * - Position actions
 * - Clipboard/sharing actions
 * - Favorites management
 * - Blacklisting
 * - View details navigation
 *
 * Applied as a mixin to ContextMenuManager instance.
 */

(function () {
  "use strict";

  function applyActionsMixin(manager) {
    // =========================================================================
    // Token Trading Actions
    //
    // All three go through the shared manual-trade flow (ui/manual_trade.js), which
    // owns the dialog, the payload and the toasts. This file used to hand-roll its
    // own copy, which had drifted: it dropped the dialog's `manual_management` flag
    // (so a manual buy could be auto-sold) and POSTed "add" to the /buy endpoint.
    // =========================================================================

    /**
     * Buy token action
     */
    manager._buyToken = async function (context) {
      const { manualTrade } = await import("../manual_trade.js");
      await manualTrade({ action: "buy", mint: context.mint, symbol: context.symbol });
    };

    /**
     * Sell token action
     */
    manager._sellToken = async function (context) {
      const { manualTrade } = await import("../manual_trade.js");

      // Holdings size the sell percentage; 0 is a safe fallback (the dialog just
      // cannot show a token quote).
      let holdings = 0;
      try {
        const posRes = await fetch(`/api/positions/${encodeURIComponent(context.mint)}/details`);
        if (posRes.ok) {
          const posData = await posRes.json();
          // /details returns PositionDetailResponse directly; `position` flattens
          // the summary fields onto itself (no {success,data} envelope).
          const pos = posData?.position;
          if (pos) {
            // remaining_token_amount reflects partial exits; token_amount is the original.
            holdings = pos.remaining_token_amount ?? pos.token_amount ?? 0;
          }
        }
      } catch {
        // Use 0 as fallback if position fetch fails
      }

      await manualTrade({
        action: "sell",
        mint: context.mint,
        symbol: context.symbol,
        holdings,
      });
    };

    /**
     * Add to existing position action
     */
    manager._addToPosition = async function (context) {
      const { manualTrade } = await import("../manual_trade.js");
      await manualTrade({ action: "add", mint: context.mint, symbol: context.symbol });
    };

    // =========================================================================
    // View Details Navigation
    // =========================================================================

    /**
     * View token details page
     */
    manager._viewTokenDetails = function (context) {
      // Dispatch custom event that token pages listen for
      window.dispatchEvent(
        new CustomEvent("screenerbot:open-token-details", {
          detail: { mint: context.mint, symbol: context.symbol },
        })
      );
    };

    /**
     * View position details dialog
     */
    manager._viewPositionDetails = function (context) {
      // Dispatch custom event to open position details dialog
      window.dispatchEvent(
        new CustomEvent("screenerbot:open-position-details", {
          detail: {
            id: context.id,
            mint: context.mint,
            symbol: context.symbol,
            position_type: context.position_type,
          },
        })
      );
    };

    // Toggle manual management for a position. The positions page owns the position
    // id + reload, so we dispatch a decoupled event it listens for (same pattern as
    // open-position-details).
    manager._toggleManualManagement = function (context, enabled) {
      // The positions table keys rows by position id (data-row-id), so read it from the
      // row element for an unambiguous target.
      const id = context.element?.dataset?.rowId || null;
      window.dispatchEvent(
        new CustomEvent("screenerbot:toggle-position-management", {
          detail: { id, mint: context.mint, enabled },
        })
      );
    };

    // =========================================================================
    // External Links
    // =========================================================================

    /**
     * Open token on external explorer/service
     */
    manager._openExplorer = function (mint, explorer) {
      const urls = {
        solscan: `https://solscan.io/token/${mint}`,
        solanafm: `https://solana.fm/address/${mint}`,
        dexscreener: `https://dexscreener.com/solana/${mint}`,
        birdeye: `https://birdeye.so/token/${mint}?chain=solana`,
        photon: `https://photon-sol.tinyastro.io/en/lp/${mint}`,
        rugcheck: `https://rugcheck.xyz/tokens/${mint}`,
        bubblemaps: `https://app.bubblemaps.io/sol/token/${mint}`,
      };
      window.open(urls[explorer] || urls.solscan, "_blank");
    };

    // =========================================================================
    // Favorites Management
    // =========================================================================

    /**
     * Load favorites cache from API
     */
    manager._loadFavoritesCache = async function () {
      if (this.favoritesCacheLoaded) return;
      try {
        const response = await fetch("/api/tokens/favorites");
        if (response.ok) {
          const data = await response.json();
          const favorites = data.favorites || [];
          this.favoritesCache.clear();
          favorites.forEach((fav) => this.favoritesCache.set(fav.mint, true));
          this.favoritesCacheLoaded = true;
        } else {
          console.warn("[ContextMenu] Failed to load favorites:", response.status);
        }
      } catch (e) {
        console.warn("[ContextMenu] Failed to load favorites cache:", e);
      }
    };

    /**
     * Check if a token is in favorites
     */
    manager._isFavorite = function (mint) {
      return this.favoritesCache.get(mint) === true;
    };

    /**
     * Update favorites cache after toggle
     */
    manager._updateFavoriteCache = function (mint, isFavorite) {
      if (isFavorite) {
        this.favoritesCache.set(mint, true);
      } else {
        this.favoritesCache.delete(mint);
      }
    };

    /**
     * Toggle favorite status for a token
     */
    manager._toggleFavorite = async function (context, currentlyFavorite) {
      try {
        if (currentlyFavorite) {
          const response = await fetch(`/api/tokens/favorites/${encodeURIComponent(context.mint)}`, {
            method: "DELETE",
          });
          if (!response.ok) throw new Error("Failed to remove favorite");
          this._updateFavoriteCache(context.mint, false);
          window.showToast?.(`${context.symbol || "Token"} removed from favorites`, "success");
        } else {
          const response = await fetch("/api/tokens/favorites", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              mint: context.mint,
              symbol: context.symbol || null,
              name: context.name || null,
              logo_url: context.icon || null,
            }),
          });
          if (!response.ok) throw new Error("Failed to add favorite");
          this._updateFavoriteCache(context.mint, true);
          window.showToast?.(`${context.symbol || "Token"} added to favorites`, "success");
        }

        // Emit event for other UI components
        window.dispatchEvent(
          new CustomEvent("screenerbot:favorites-changed", {
            detail: { mint: context.mint, isFavorite: !currentlyFavorite },
          })
        );
      } catch (error) {
        window.showToast?.(error.message || "Failed to update favorites", "error");
      }
    };

    // =========================================================================
    // Clipboard Actions
    // =========================================================================

    /**
     * Copy text to clipboard
     */
    manager._copyToClipboard = async function (text, label) {
      try {
        await navigator.clipboard.writeText(text);
        this._showToast(`${label} copied!`, "success");
      } catch {
        this._showToast("Failed to copy", "error");
      }
    };

    // =========================================================================
    // Token Management
    // =========================================================================

    /**
     * Blacklist a token
     */
    manager._blacklistToken = async function (context) {
      try {
        const { ConfirmationDialog } = await import("../confirmation_dialog.js");

        const result = await ConfirmationDialog.show({
          title: "Blacklist Token",
          message: `Are you sure you want to blacklist ${context.symbol}? This token will be excluded from trading.`,
          confirmLabel: "Blacklist",
          cancelLabel: "Cancel",
          variant: "danger",
        });

        if (!result.confirmed) return;

        const response = await fetch(`/api/tokens/${context.mint}/blacklist`, {
          method: "POST",
        });

        if (!response.ok) {
          throw new Error("Failed to blacklist token");
        }

        this._showToast(`${context.symbol} blacklisted`, "success");

        // Emit event for UI refresh
        window.dispatchEvent(
          new CustomEvent("screenerbot:token-blacklisted", {
            detail: { mint: context.mint },
          })
        );
      } catch (error) {
        this._showToast(error.message || "Failed to blacklist", "error");
      }
    };

    /**
     * Refresh token data from backend
     */
    manager._refreshToken = async function (context) {
      try {
        const response = await fetch(`/api/tokens/${context.mint}/refresh`, {
          method: "POST",
        });

        if (!response.ok) {
          throw new Error("Failed to refresh token data");
        }

        this._showToast("Token data refreshed", "success");
      } catch (error) {
        this._showToast(error.message || "Failed to refresh", "error");
      }
    };

    // =========================================================================
    // Developer Tools
    // =========================================================================

    /**
     * Inspect element (open devtools)
     */
    manager._inspectElement = function (element) {
      // In Electron, use the electronAPI to open devtools
      if (typeof window.electronAPI !== "undefined" && window.electronAPI.openDevTools) {
        window.electronAPI.openDevTools();
      } else {
        // Browser fallback
        console.log("Inspect Element:", element);
        console.dir(element);
      }
    };
  }

  // Export mixin
  window.ContextMenuActions = { apply: applyActionsMixin };
})();
