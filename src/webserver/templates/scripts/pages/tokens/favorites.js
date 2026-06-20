/**
 * Favorites table module for tokens page
 * Lines extracted from tokens.js (1970-2239)
 * Factory function pattern for shared state access
 */

import { timeAgoCell } from "./formatters.js";

/**
 * Create Favorites module with access to page state and dependencies
 * @param {Object} deps - Dependencies and state
 * @returns {Object} Favorites module functions
 */
export function createFavoritesModule(deps) {
  const { favoritesState, requestManager, DataTable, Utils } = deps;

  const buildFavoritesColumns = () => {
    return [
      {
        id: "actions",
        label: "",
        sortable: false,
        floating: true,
        // Trade buttons (Buy, or Add + Sell when a position is open) + 3 utility icons.
        // Up to 5 collapsed icon buttons: 5*32 + 4*6 gaps + ~24 cell padding + safety.
        // No hard maxWidth so the column stays resizable like every other pinned column.
        minWidth: 210,
        width: 210,
        wrap: false,
        render: (_value, row) => {
          const mint = Utils.escapeHtml(row.mint);
          const isBlacklisted = Boolean(row.blacklisted);
          const hasOpen = Boolean(row.has_open_position);
          const disabledAttr = isBlacklisted ? ' disabled aria-disabled="true"' : "";

          // Manual trade buttons reuse the shared handler (deps.performManualTrade).
          const tradeButtons = hasOpen
            ? `
              <button class="btn btn-small btn-icon favorites-trade-btn" data-action="add" data-mint="${mint}" title="Add to position (DCA)"${disabledAttr}><i class="icon-circle-plus"></i></button>
              <button class="btn btn-small btn-icon favorites-trade-btn" data-action="sell" data-mint="${mint}" title="Sell (full or % partial)"${disabledAttr}><i class="icon-trending-down"></i></button>`
            : `
              <button class="btn btn-small btn-icon favorites-trade-btn" data-action="buy" data-mint="${mint}" title="Buy position"${disabledAttr}><i class="icon-shopping-cart"></i></button>`;

          return `
            <div class="favorites-actions dt-actions-flex">${tradeButtons}
              <button class="btn btn-small btn-icon favorites-action-btn" data-action="copy" data-mint="${mint}" title="Copy Mint">
                <i class="icon-copy"></i>
              </button>
              <button class="btn btn-small btn-icon favorites-action-btn" data-action="external" data-mint="${mint}" title="View on DexScreener">
                <i class="icon-external-link"></i>
              </button>
              <button class="btn btn-small btn-icon btn-danger favorites-action-btn" data-action="remove" data-mint="${mint}" title="Remove from favorites">
                <i class="icon-trash-2"></i>
              </button>
            </div>
          `;
        },
      },
      {
        id: "token",
        label: "Token",
        sortable: true,
        floating: true,
        minWidth: 200,
        wrap: false,
        render: (_v, row) => {
          const logo = row.logo_url
            ? `<img class="token-logo" alt="" src="${Utils.escapeHtml(row.logo_url)}" />`
            : '<span class="token-logo">N/A</span>';
          const sym = Utils.escapeHtml(row.symbol || "???");
          const name = row.name
            ? `<div class="token-name">${Utils.escapeHtml(row.name)}</div>`
            : "";
          return `<div class="token-cell">${logo}<div><div class="token-symbol">${sym}</div>${name}</div></div>`;
        },
      },
      {
        id: "mint",
        label: "Mint",
        sortable: true,
        minWidth: 150,
        wrap: false,
        render: (value) => {
          const short = value ? `${value.slice(0, 8)}...${value.slice(-4)}` : "—";
          return `<code class="mint-code" title="${Utils.escapeHtml(value)}">${short}</code>`;
        },
      },
      {
        id: "notes",
        label: "Notes",
        sortable: false,
        minWidth: 200,
        wrap: true,
        render: (value) => {
          if (!value) return '<span class="text-muted">—</span>';
          return Utils.escapeHtml(value);
        },
      },
      {
        id: "created_at",
        label: "Added",
        sortable: true,
        minWidth: 120,
        wrap: false,
        render: (value) => {
          if (!value) return "—";
          const timestamp =
            typeof value === "string" ? Math.floor(new Date(value).getTime() / 1000) : value;
          return timeAgoCell(timestamp);
        },
      },
    ];
  };

  const fetchFavorites = async () => {
    favoritesState.isLoading = true;
    try {
      const response = await requestManager.fetch("/api/tokens/favorites", { priority: "normal" });
      if (response && response.favorites) {
        favoritesState.favorites = response.favorites;
      }
    } catch (err) {
      console.error("Failed to fetch favorites:", err);
      Utils.showToast("Failed to load favorites", "error");
    } finally {
      favoritesState.isLoading = false;
    }
  };

  const updateFavoritesTable = () => {
    if (!deps.favoritesTable) return;
    const isEmpty = favoritesState.favorites.length === 0;
    const emptyState = document.querySelector("#favorites-empty-state");

    if (isEmpty) {
      deps.favoritesTable.setData([], { preserveScroll: false });
      if (emptyState) emptyState.style.display = "";
    } else {
      if (emptyState) emptyState.style.display = "none";
      deps.favoritesTable.setData(favoritesState.favorites, { preserveScroll: true });
    }
    updateFavoritesToolbar();
  };

  const updateFavoritesToolbar = () => {
    if (!deps.favoritesTable) return;
    const count = favoritesState.favorites.length;

    deps.favoritesTable.updateToolbarSummary([
      {
        id: "favorites-total",
        label: "Total Favorites",
        value: Utils.formatNumber(count, 0),
        variant: count > 0 ? "info" : "secondary",
      },
    ]);
  };

  const handleFavoriteAction = async (action, mint) => {
    switch (action) {
      case "copy":
        try {
          await navigator.clipboard.writeText(mint);
          Utils.showToast("Mint address copied", "success");
        } catch (err) {
          console.error("Failed to copy mint:", err);
          Utils.showToast("Failed to copy mint", "warning");
        }
        break;

      case "external":
        Utils.openExternal(`https://dexscreener.com/solana/${mint}`);
        break;

      case "remove":
        try {
          await requestManager.fetch(`/api/tokens/favorites/${encodeURIComponent(mint)}`, {
            method: "DELETE",
            priority: "high",
          });
          // Remove from local state
          favoritesState.favorites = favoritesState.favorites.filter((f) => f.mint !== mint);
          updateFavoritesTable();
          Utils.showToast("Removed from favorites", "success");
        } catch (err) {
          console.error("Failed to remove favorite:", err);
          Utils.showToast("Failed to remove favorite", "error");
        }
        break;
    }
  };

  const initFavoritesTable = () => {
    if (deps.favoritesTable) return; // Already initialized

    // Create container for favorites table
    const rootEl = document.querySelector("#tokens-root");
    if (!rootEl) return;

    // Create favorites container
    let favoritesContainer = document.querySelector("#favorites-table-container");
    if (!favoritesContainer) {
      favoritesContainer = document.createElement("div");
      favoritesContainer.id = "favorites-table-container";
      favoritesContainer.className = "favorites-table-container";
      favoritesContainer.style.display = "none";
      rootEl.parentNode.insertBefore(favoritesContainer, rootEl.nextSibling);
    }

    // Create empty state element
    let emptyState = document.querySelector("#favorites-empty-state");
    if (!emptyState) {
      emptyState = document.createElement("div");
      emptyState.id = "favorites-empty-state";
      emptyState.className = "empty-state";
      emptyState.style.display = "none";
      emptyState.innerHTML = `
        <div class="empty-state-icon">⭐</div>
        <h3 class="empty-state-title">No Favorites Yet</h3>
        <p class="empty-state-description">
          Use the search (<kbd>⌘K</kbd>) to find tokens and add them to your favorites.
        </p>
      `;
      favoritesContainer.appendChild(emptyState);
    }

    deps.favoritesTable = new DataTable({
      container: "#favorites-table-container",
      columns: buildFavoritesColumns(),
      rowIdField: "mint",
      stateKey: "favorites-table",
      enableLogging: false,
      sorting: {
        mode: "client",
        column: "created_at",
        direction: "desc",
      },
      clientPagination: {
        enabled: true,
        pageSizes: [10, 20, 50, 100, "all"],
        defaultPageSize: 50,
        stateKey: "tokens.favorites.pageSize",
      },
      compact: true,
      stickyHeader: true,
      zebra: true,
      fitToContainer: true,
      autoSizeColumns: false,
      uniformRowHeight: 2,
      toolbar: {
        title: {
          icon: "icon-star",
          text: "Favorite Tokens",
        },
        summary: [
          { id: "favorites-total", label: "Total Favorites", value: "0", variant: "secondary" },
        ],
      },
    });

    // Add click handler for action buttons
    favoritesContainer.addEventListener("click", (e) => {
      // Manual trade buttons (Buy / Add / Sell) reuse the shared trade handler and
      // refresh favorites afterwards so the row's Buy<->Add/Sell state stays correct.
      const tradeBtn = e.target.closest(".favorites-trade-btn");
      if (tradeBtn) {
        const action = tradeBtn.getAttribute("data-action");
        const mint = tradeBtn.getAttribute("data-mint");
        const row = favoritesState.favorites.find((f) => f.mint === mint) || {};
        if (action && mint && typeof deps.performManualTrade === "function") {
          deps.performManualTrade({
            action,
            mint,
            row,
            btn: tradeBtn,
            onReload: () => fetchFavorites().then(() => updateFavoritesTable()),
          });
        }
        return;
      }

      const actionBtn = e.target.closest(".favorites-action-btn");
      if (actionBtn) {
        const action = actionBtn.getAttribute("data-action");
        const mint = actionBtn.getAttribute("data-mint");
        if (action && mint) handleFavoriteAction(action, mint);
      }
    });
  };

  const showFavoritesView = () => {
    const tokensRoot = document.querySelector("#tokens-root");
    const favoritesContainer = document.querySelector("#favorites-table-container");
    const ohlcvContainer = document.querySelector("#ohlcv-table-container");

    if (tokensRoot) tokensRoot.style.display = "none";
    if (favoritesContainer) favoritesContainer.style.display = "";
    if (ohlcvContainer) ohlcvContainer.style.display = "none";

    // Pause main table poller
    if (deps.poller) deps.poller.pause();
    if (deps.lastUpdatePoller) deps.lastUpdatePoller.pause();
    if (deps.ohlcvPoller) deps.ohlcvPoller.pause();

    // Initial load
    fetchFavorites().then(() => updateFavoritesTable());
  };

  const hideFavoritesView = () => {
    const tokensRoot = document.querySelector("#tokens-root");
    const favoritesContainer = document.querySelector("#favorites-table-container");

    if (tokensRoot) tokensRoot.style.display = "";
    if (favoritesContainer) favoritesContainer.style.display = "none";

    // Resume main poller
    if (deps.poller) deps.poller.start();
    if (deps.lastUpdatePoller) deps.lastUpdatePoller.start();
  };

  return {
    buildFavoritesColumns,
    fetchFavorites,
    updateFavoritesTable,
    updateFavoritesToolbar,
    handleFavoriteAction,
    initFavoritesTable,
    showFavoritesView,
    hideFavoritesView,
  };
}
