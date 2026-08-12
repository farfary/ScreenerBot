/**
 * Favorites table module for tokens page
 *
 * The favorites subtab renders the exact same columns as the all/passed token
 * lists by reusing the parent page's buildColumns() and wireTokenTable() (shared
 * via deps). The backend (/api/tokens/favorites) returns each favorite as a full
 * token object, so every market column populates and row-click opens the token
 * details dialog — identical to the main list.
 */

import { boostRowClass } from "../../core/boosts.js";

/**
 * Create Favorites module with access to page state and dependencies
 * @param {Object} deps - Dependencies and state
 * @returns {Object} Favorites module functions
 */
export function createFavoritesModule(deps) {
  const { favoritesState, requestManager, DataTable, Utils } = deps;

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
        <div class="empty-state-icon"><i class="icon-star"></i></div>
        <h3 class="empty-state-title">No Favorites Yet</h3>
        <p class="empty-state-description">
          Use the search (<kbd>⌘K</kbd>) to find tokens and add them to your favorites.
        </p>
      `;
      favoritesContainer.appendChild(emptyState);
    }

    // Reuse the parent page's exact column set (all/passed token list columns).
    deps.favoritesTable = new DataTable({
      container: "#favorites-table-container",
      columns: deps.buildColumns(),
      rowIdField: "mint",
      stateKey: "favorites-table",
      enableLogging: false,
      // Same gold mark as the main token list -- a boosted token must not read
      // differently depending on which tab the user is standing on.
      rowClass: (row) => boostRowClass(row?.mint),
      sorting: {
        mode: "client",
        column: "liquidity_usd",
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
        summary: [
          { id: "favorites-total", label: "Total Favorites", value: "0", variant: "secondary" },
        ],
      },
    });

    // Same wiring as the main list: Buy/Add/Sell row actions, external-links
    // dropdown, logo lightbox, and row-click -> token details dialog.
    deps.wireTokenTable(deps.favoritesTable, {
      onReload: () => fetchFavorites().then(() => updateFavoritesTable()),
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
    fetchFavorites,
    updateFavoritesTable,
    updateFavoritesToolbar,
    initFavoritesTable,
    showFavoritesView,
    hideFavoritesView,
  };
}
