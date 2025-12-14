/**
 * Context Menu - macOS Native Style
 *
 * Features:
 * - macOS-style appearance with blur, shadows, animations
 * - Keyboard navigation (arrow keys, enter, escape)
 * - Submenus with hover delay
 * - Lucide font icons
 * - Token-specific actions (trade, blacklist, copy)
 * - Position-specific actions
 * - Devtools option in debug mode
 * - Automatic positioning to stay within viewport
 */

// Icon mapping to Lucide font classes
const ICONS = {
  copy: "icon-copy",
  externalLink: "icon-external-link",
  refresh: "icon-refresh-cw",
  shoppingCart: "icon-shopping-cart",
  trendingDown: "icon-trending-down",
  trendingUp: "icon-trending-up",
  ban: "icon-ban",
  eye: "icon-eye",
  info: "icon-info",
  code: "icon-code",
  settings: "icon-settings",
  chevronRight: "icon-chevron-right",
  check: "icon-check",
  star: "icon-star",
  zoomIn: "icon-zoom-in",
  zoomOut: "icon-zoom-out",
  arrowLeft: "icon-arrow-left",
  plus: "icon-plus",
  trash: "icon-trash-2",
  globe: "icon-globe",
  shield: "icon-shield",
  chart: "icon-chart-bar",
  search: "icon-search",
  clipboard: "icon-clipboard",
  link: "icon-link",
  x: "icon-x",
  zap: "icon-zap",
};

/**
 * Context Menu Manager
 * Singleton class that handles all context menu operations
 */
class ContextMenuManager {
  constructor() {
    this.menuEl = null;
    this.overlayEl = null;
    this.currentContext = null;
    this.activeItemIndex = -1;
    this.items = [];
    this.flatItems = []; // Flattened actionable items for keyboard nav
    this.submenuTimeout = null;
    this.hideTimeout = null;
    this.isVisible = false;
    this.isTransitioning = false;
    this.isShowing = false; // Guard against concurrent show() calls

    // Favorites cache for quick lookup
    this.favoritesCache = new Map(); // mint -> boolean
    this.favoritesCacheLoaded = false;

    // Check if devtools are enabled (set by Tauri/backend)
    this.devtoolsEnabled = window.__SCREENERBOT_DEVTOOLS__ === true;

    // Bound handlers for proper cleanup
    this._boundHandleKeyDown = this._handleKeyDown.bind(this);
    this._boundHandleScroll = this._handleScroll.bind(this);
    this._boundHandleResize = this._handleResize.bind(this);

    this._init();
  }

  _init() {
    // Prevent default context menu globally
    document.addEventListener(
      "contextmenu",
      (e) => {
        // Allow default in inputs/textareas for native copy/paste
        if (this._isEditableElement(e.target)) {
          return;
        }

        e.preventDefault();
        e.stopPropagation();
        this._handleContextMenu(e);
      },
      true
    );

    // Close on escape anywhere
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && this.isVisible) {
        e.preventDefault();
        this.hide();
      }
    });
  }

  _isEditableElement(el) {
    if (!el) return false;
    const tagName = el.tagName?.toUpperCase();
    return (
      tagName === "INPUT" ||
      tagName === "TEXTAREA" ||
      el.isContentEditable ||
      el.closest('[contenteditable="true"]')
    );
  }

  /**
   * Handle context menu event
   */
  _handleContextMenu(e) {
    // Clear any pending operations
    this._clearTimeouts();

    const context = this._determineContext(e.target);
    this.show(e.clientX, e.clientY, context);
  }

  _clearTimeouts() {
    if (this.submenuTimeout) {
      clearTimeout(this.submenuTimeout);
      this.submenuTimeout = null;
    }
    if (this.hideTimeout) {
      clearTimeout(this.hideTimeout);
      this.hideTimeout = null;
    }
  }

  /**
   * Determine context based on clicked element
   */
  _determineContext(target) {
    // Check for DataTable row (has data-row-id attribute)
    const tableRow = target.closest("[data-row-id]");

    // Also check for regular table rows that may have data-mint
    const walletRow = target.closest("tr[data-mint], .dt-row");
    const rowEl = tableRow || walletRow;

    // Token tables: tokens page, wallet holdings
    const isTokensTable = target.closest(
      "#tokens-root, #holdingsTableContainer, [data-context='tokens']"
    );

    if (rowEl && isTokensTable) {
      // Get mint from data-row-id or data-mint
      const mint =
        tableRow?.dataset.rowId ||
        walletRow?.dataset.mint ||
        rowEl.querySelector("[data-mint]")?.dataset.mint;

      if (mint) {
        const symbolEl = rowEl.querySelector(
          ".token-name-group .token-symbol, .dt-symbol, [data-field='symbol'], .token-symbol"
        );
        const nameEl = rowEl.querySelector(
          ".token-name-group .token-name, .dt-name, [data-field='name'], .token-name"
        );
        const logoEl = rowEl.querySelector(
          ".token-logo img, .dt-token-logo img, [data-field='logo'] img, img.token-icon, img.token-logo"
        );

        return {
          type: "token",
          mint: mint,
          symbol: symbolEl?.textContent?.trim() || "Unknown",
          name: nameEl?.textContent?.trim() || "",
          icon: logoEl?.src || null,
          element: rowEl,
        };
      }
    }

    // Check for positions table rows
    const positionsTable = target.closest("#positions-root, [data-context='positions']");
    if (tableRow && positionsTable) {
      const mint = tableRow.dataset.rowId;
      const symbolEl = tableRow.querySelector(
        ".token-symbol, .position-symbol, [data-field='symbol']"
      );

      return {
        type: "position",
        mint: mint,
        symbol: symbolEl?.textContent?.trim() || "Unknown",
        element: tableRow,
      };
    }

    // Check for transaction rows
    const transactionsTable = target.closest("#transactions-root, [data-context='transactions']");
    if (tableRow && transactionsTable) {
      const signature = tableRow.dataset.rowId;

      return {
        type: "transaction",
        signature: signature,
        element: tableRow,
      };
    }

    // Check for link context
    const link = target.closest("a[href]");
    if (link && !link.closest(".context-menu")) {
      return {
        type: "link",
        href: link.href,
        text: link.textContent?.trim() || link.href,
        element: link,
      };
    }

    // Check for image context
    const img = target.closest("img");
    if (img && img.src) {
      return {
        type: "image",
        src: img.src,
        alt: img.alt || "",
        element: img,
      };
    }

    // Check for selected text
    const selection = window.getSelection();
    if (selection && selection.toString().trim()) {
      return {
        type: "selection",
        text: selection.toString(),
        element: target,
      };
    }

    // Default context
    return {
      type: "default",
      element: target,
    };
  }

  /**
   * Build menu items based on context
   */
  _buildMenuItems(context) {
    const items = [];

    switch (context.type) {
      case "token":
        this._buildTokenMenu(items, context);
        break;

      case "position":
        this._buildPositionMenu(items, context);
        break;

      case "transaction":
        this._buildTransactionMenu(items, context);
        break;

      case "link":
        this._buildLinkMenu(items, context);
        break;

      case "image":
        this._buildImageMenu(items, context);
        break;

      case "selection":
        this._buildSelectionMenu(items, context);
        break;

      default:
        this._buildDefaultMenu(items, context);
        break;
    }

    // Devtools option (only in debug mode)
    if (this.devtoolsEnabled) {
      this._addSeparatorIfNeeded(items);
      items.push({
        type: "item",
        label: "Inspect Element",
        icon: "code",
        shortcut: this._getModKey() + "⌥I",
        action: () => this._inspectElement(context.element),
      });
    }

    return items;
  }

  _buildTokenMenu(items, context) {
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
  }

  _buildPositionMenu(items, context) {
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
      action: () => this._copyToClipboard(context.mint, "Token address"),
    });
  }

  _buildTransactionMenu(items, context) {
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
  }

  _buildLinkMenu(items, context) {
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
  }

  _buildImageMenu(items, context) {
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
  }

  _buildSelectionMenu(items, context) {
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
  }

  _buildDefaultMenu(items, context) {
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
  }

  _addSeparatorIfNeeded(items) {
    if (items.length > 0 && items[items.length - 1].type !== "separator") {
      items.push({ type: "separator" });
    }
  }

  /**
   * Show context menu at position
   */
  async show(x, y, context) {
    // Prevent concurrent show() calls - critical for preventing hangs
    if (this.isShowing || this.isTransitioning) {
      return;
    }
    this.isShowing = true;

    try {
      // Clean up any existing menu FIRST, before async operations
      this._hideImmediate();
      
      // Also clean up any orphaned elements from previous instances
      this._cleanupOrphanedElements();

      // Load favorites cache for token/position contexts
      if (context.type === "token" || context.type === "position") {
        await this._loadFavoritesCache();
      }

      // Double-check we're still supposed to show (another show() could have been called)
      if (!this.isShowing) {
        return;
      }

      this.currentContext = context;
      this.items = this._buildMenuItems(context);
      this._buildFlatItems();

      this._createMenuElement();
      this._positionMenu(x, y);

      // Show with animation
      requestAnimationFrame(() => {
        if (this.menuEl) {
          this.menuEl.classList.add("visible");
          this.isVisible = true;
        }
      });

      // Remove any existing listeners before adding (defensive)
      document.removeEventListener("keydown", this._boundHandleKeyDown);
      window.removeEventListener("scroll", this._boundHandleScroll, true);
      window.removeEventListener("resize", this._boundHandleResize);

      // Add event listeners
      document.addEventListener("keydown", this._boundHandleKeyDown);
      window.addEventListener("scroll", this._boundHandleScroll, true);
      window.addEventListener("resize", this._boundHandleResize);
    } finally {
      this.isShowing = false;
    }
  }

  /**
   * Clean up any orphaned context menu elements from DOM
   * This prevents element accumulation if cleanup was missed
   */
  _cleanupOrphanedElements() {
    // Remove any stray overlays
    document.querySelectorAll(".context-menu-overlay").forEach(el => {
      if (el !== this.overlayEl) {
        el.remove();
      }
    });
    // Remove any stray menus
    document.querySelectorAll(".context-menu").forEach(el => {
      if (el !== this.menuEl) {
        el.remove();
      }
    });
  }

  /**
   * Build flat list of actionable items for keyboard navigation
   */
  _buildFlatItems() {
    this.flatItems = [];
    this.items.forEach((item, idx) => {
      if (item.type === "item" && !item.disabled) {
        this.flatItems.push({ item, index: idx });
      }
    });
  }

  /**
   * Hide context menu with animation
   */
  hide() {
    if (!this.isVisible || this.isTransitioning) return;

    this.isTransitioning = true;
    this._clearTimeouts();

    if (this.menuEl) {
      this.menuEl.classList.remove("visible");

      this.hideTimeout = setTimeout(() => {
        this._cleanup();
        this.isTransitioning = false;
      }, 120);
    } else {
      this._cleanup();
      this.isTransitioning = false;
    }
  }

  /**
   * Hide immediately without animation
   */
  _hideImmediate() {
    this._clearTimeouts();
    this._cleanup();
    this.isTransitioning = false;
    this.isShowing = false; // Reset showing flag
  }

  /**
   * Clean up menu elements and state
   */
  _cleanup() {
    if (this.menuEl) {
      this.menuEl.remove();
      this.menuEl = null;
    }
    if (this.overlayEl) {
      this.overlayEl.remove();
      this.overlayEl = null;
    }

    this.isVisible = false;
    this.isShowing = false; // Reset showing flag
    this.activeItemIndex = -1;
    this.items = [];
    this.flatItems = [];
    this.currentContext = null;

    document.removeEventListener("keydown", this._boundHandleKeyDown);
    window.removeEventListener("scroll", this._boundHandleScroll, true);
    window.removeEventListener("resize", this._boundHandleResize);
  }

  /**
   * Handle scroll - close menu
   */
  _handleScroll() {
    this.hide();
  }

  /**
   * Handle resize - close menu
   */
  _handleResize() {
    this.hide();
  }

  /**
   * Create menu DOM element
   */
  _createMenuElement() {
    // Create overlay for click-outside detection
    this.overlayEl = document.createElement("div");
    this.overlayEl.className = "context-menu-overlay";
    this.overlayEl.addEventListener("mousedown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.hide();
    });
    this.overlayEl.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.hide();

      // Re-trigger at new position after a small delay
      setTimeout(() => {
        const target = document.elementFromPoint(e.clientX, e.clientY);
        if (target && target !== this.overlayEl) {
          const context = this._determineContext(target);
          this.show(e.clientX, e.clientY, context);
        }
      }, 50);
    });

    // Create menu container
    this.menuEl = document.createElement("div");
    this.menuEl.className = "context-menu";
    this.menuEl.setAttribute("role", "menu");
    this.menuEl.setAttribute("tabindex", "-1");

    // Build menu content
    this._renderItems(this.menuEl, this.items, true);

    document.body.appendChild(this.overlayEl);
    document.body.appendChild(this.menuEl);
  }

  /**
   * Render menu items into container
   */
  _renderItems(container, items, isRoot = false) {
    items.forEach((item, index) => {
      const el = this._createItemElement(item, index, isRoot);
      if (el) {
        container.appendChild(el);
      }
    });
  }

  /**
   * Create individual item element
   */
  _createItemElement(item, index, isRoot = false) {
    let el;

    switch (item.type) {
      case "separator": {
        el = document.createElement("div");
        el.className = "context-menu-separator";
        el.setAttribute("role", "separator");
        break;
      }

      case "header": {
        el = document.createElement("div");
        el.className = "context-menu-header";
        el.textContent = item.label;
        break;
      }

      case "token-preview": {
        el = document.createElement("div");
        el.className = "context-menu-token-preview";

        const iconDiv = document.createElement("div");
        iconDiv.className = "context-menu-token-icon";
        if (item.icon) {
          const img = document.createElement("img");
          img.src = item.icon;
          img.alt = "";
          img.onerror = () => img.remove();
          iconDiv.appendChild(img);
        } else {
          iconDiv.textContent = item.symbol?.[0] || "?";
        }

        const infoDiv = document.createElement("div");
        infoDiv.className = "context-menu-token-info";

        const symbolDiv = document.createElement("div");
        symbolDiv.className = "context-menu-token-symbol";
        symbolDiv.textContent = item.symbol;
        infoDiv.appendChild(symbolDiv);

        if (item.name) {
          const nameDiv = document.createElement("div");
          nameDiv.className = "context-menu-token-name";
          nameDiv.textContent = item.name;
          infoDiv.appendChild(nameDiv);
        }

        el.appendChild(iconDiv);
        el.appendChild(infoDiv);
        break;
      }

      case "item":
      default: {
        el = document.createElement("div");
        el.className = "context-menu-item";
        if (item.className) el.classList.add(item.className);
        if (item.disabled) el.classList.add("disabled");
        el.setAttribute("role", "menuitem");
        el.setAttribute("tabindex", "-1");
        if (isRoot) {
          el.dataset.index = index;
        }

        // Icon
        if (item.icon) {
          const iconEl = document.createElement("span");
          iconEl.className = "context-menu-icon";
          const iconClass = ICONS[item.icon] || `icon-${item.icon}`;
          const iconI = document.createElement("i");
          iconI.className = iconClass;
          iconEl.appendChild(iconI);
          el.appendChild(iconEl);
        } else if (item.checked !== undefined) {
          const checkEl = document.createElement("span");
          checkEl.className = "context-menu-icon check";
          if (item.checked) {
            const iconI = document.createElement("i");
            iconI.className = ICONS.check;
            checkEl.appendChild(iconI);
          }
          el.appendChild(checkEl);
        }

        // Label
        const labelEl = document.createElement("span");
        labelEl.className = "context-menu-label";
        labelEl.textContent = item.label;
        el.appendChild(labelEl);

        // Badge
        if (item.badge) {
          const badgeEl = document.createElement("span");
          badgeEl.className = "context-menu-badge";
          badgeEl.textContent = item.badge;
          el.appendChild(badgeEl);
        }

        // Shortcut or submenu arrow
        if (item.submenu) {
          const arrowEl = document.createElement("span");
          arrowEl.className = "context-menu-submenu-arrow";
          const arrowI = document.createElement("i");
          arrowI.className = ICONS.chevronRight;
          arrowEl.appendChild(arrowI);
          el.appendChild(arrowEl);

          // Create submenu container
          const submenuEl = document.createElement("div");
          submenuEl.className = "context-menu-submenu";
          submenuEl.setAttribute("role", "menu");
          this._renderItems(submenuEl, item.submenu, false);
          el.appendChild(submenuEl);

          // Submenu hover handling
          el.addEventListener("mouseenter", () => {
            this._clearTimeouts();
            this.submenuTimeout = setTimeout(() => {
              this._openSubmenuForItem(el);
            }, 150);
          });

          el.addEventListener("mouseleave", () => {
            this._clearTimeouts();
            this.submenuTimeout = setTimeout(() => {
              this._closeSubmenuForItem(el);
            }, 100);
          });
        } else if (item.shortcut) {
          const shortcutEl = document.createElement("span");
          shortcutEl.className = "context-menu-shortcut";
          shortcutEl.textContent = item.shortcut;
          el.appendChild(shortcutEl);
        }

        // Event handlers for non-submenu items
        if (!item.disabled && !item.submenu && item.action) {
          el.addEventListener("click", (e) => {
            e.preventDefault();
            e.stopPropagation();
            this.hide();
            // Execute action after hide animation
            setTimeout(() => item.action(), 50);
          });
        }

        // Hover highlighting
        if (isRoot) {
          el.addEventListener("mouseenter", () => {
            this._setActiveItem(index);
          });
        }

        break;
      }
    }

    return el;
  }

  _openSubmenuForItem(el) {
    // Close other submenus first
    this.menuEl.querySelectorAll(".context-menu-item.submenu-open").forEach((item) => {
      if (item !== el) {
        item.classList.remove("submenu-open");
      }
    });

    el.classList.add("submenu-open");

    // Position submenu
    const submenu = el.querySelector(".context-menu-submenu");
    if (submenu) {
      this._positionSubmenu(el, submenu);
    }
  }

  _closeSubmenuForItem(el) {
    el.classList.remove("submenu-open");
  }

  _positionSubmenu(parentEl, submenuEl) {
    const parentRect = parentEl.getBoundingClientRect();
    const submenuRect = submenuEl.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const padding = 8;

    // Check if submenu fits on the right
    if (parentRect.right + submenuRect.width > viewportWidth - padding) {
      submenuEl.classList.add("left");
    } else {
      submenuEl.classList.remove("left");
    }
  }

  /**
   * Position menu to fit within viewport
   */
  _positionMenu(x, y) {
    // Need to render first to get dimensions
    this.menuEl.style.visibility = "hidden";
    this.menuEl.style.left = "0";
    this.menuEl.style.top = "0";

    const rect = this.menuEl.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const padding = 8;

    let finalX = x;
    let finalY = y;
    let originX = "left";
    let originY = "top";

    // Adjust horizontal position
    if (x + rect.width > viewportWidth - padding) {
      finalX = x - rect.width;
      originX = "right";
      if (finalX < padding) {
        finalX = viewportWidth - rect.width - padding;
      }
    }

    // Adjust vertical position
    if (y + rect.height > viewportHeight - padding) {
      finalY = y - rect.height;
      originY = "bottom";
      if (finalY < padding) {
        finalY = viewportHeight - rect.height - padding;
      }
    }

    this.menuEl.style.left = `${finalX}px`;
    this.menuEl.style.top = `${finalY}px`;
    this.menuEl.style.visibility = "";

    // Set transform origin for animation
    this.menuEl.style.transformOrigin = `${originY} ${originX}`;

    // Mark submenus that should open left
    if (originX === "right" || finalX + rect.width > viewportWidth - 200) {
      this.menuEl.classList.add("submenus-left");
    }
  }

  /**
   * Handle keyboard navigation
   */
  _handleKeyDown(e) {
    if (!this.isVisible) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        this._navigateItems(1);
        break;

      case "ArrowUp":
        e.preventDefault();
        this._navigateItems(-1);
        break;

      case "Enter":
      case " ":
        e.preventDefault();
        this._activateCurrentItem();
        break;

      case "ArrowRight":
        e.preventDefault();
        this._openCurrentSubmenu();
        break;

      case "ArrowLeft":
        e.preventDefault();
        this._closeCurrentSubmenu();
        break;

      case "Escape":
        e.preventDefault();
        this.hide();
        break;
    }
  }

  /**
   * Navigate through menu items
   */
  _navigateItems(direction) {
    if (this.flatItems.length === 0) return;

    // Find current position in flat list
    let currentFlatIdx = this.flatItems.findIndex((f) => f.index === this.activeItemIndex);

    if (currentFlatIdx === -1) {
      currentFlatIdx = direction > 0 ? -1 : this.flatItems.length;
    }

    let newFlatIdx = currentFlatIdx + direction;
    if (newFlatIdx < 0) newFlatIdx = this.flatItems.length - 1;
    if (newFlatIdx >= this.flatItems.length) newFlatIdx = 0;

    const newItem = this.flatItems[newFlatIdx];
    if (newItem) {
      this._setActiveItem(newItem.index);
    }
  }

  /**
   * Set active item by index
   */
  _setActiveItem(index) {
    if (!this.menuEl) return;

    const items = this.menuEl.querySelectorAll(":scope > .context-menu-item");
    items.forEach((item, i) => {
      const isActive = parseInt(item.dataset.index) === index;
      item.classList.toggle("active", isActive);
    });
    this.activeItemIndex = index;
  }

  /**
   * Activate current item (Enter key)
   */
  _activateCurrentItem() {
    if (this.activeItemIndex >= 0 && this.activeItemIndex < this.items.length) {
      const item = this.items[this.activeItemIndex];
      if (item && item.type === "item" && !item.disabled) {
        if (item.submenu) {
          this._openCurrentSubmenu();
        } else if (item.action) {
          this.hide();
          setTimeout(() => item.action(), 50);
        }
      }
    }
  }

  /**
   * Open submenu for current item
   */
  _openCurrentSubmenu() {
    if (this.activeItemIndex < 0) return;

    const activeEl = this.menuEl.querySelector(
      `.context-menu-item[data-index="${this.activeItemIndex}"]`
    );
    if (activeEl && activeEl.querySelector(".context-menu-submenu")) {
      this._openSubmenuForItem(activeEl);
    }
  }

  /**
   * Close current submenu
   */
  _closeCurrentSubmenu() {
    this.menuEl.querySelectorAll(".context-menu-item.submenu-open").forEach((el) => {
      el.classList.remove("submenu-open");
    });
  }

  // =========================================================================
  // Action Handlers
  // =========================================================================

  async _buyToken(context) {
    try {
      const { TradeActionDialog } = await import("../ui/trade_action_dialog.js");
      const dialog = new TradeActionDialog();

      const balanceRes = await fetch("/api/wallet/balance");
      const balanceData = await balanceRes.json();
      const balance = balanceData?.sol_balance || 0;

      const result = await dialog.open({
        action: "buy",
        symbol: context.symbol,
        context: { balance },
      });

      if (!result) return;

      const response = await fetch("/api/trader/manual/buy", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          mint: context.mint,
          ...(result.amount ? { size_sol: result.amount } : {}),
        }),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.message || "Buy failed");
      }

      this._showToast("Buy order placed!", "success");
    } catch (error) {
      this._showToast(error.message || "Buy failed", "error");
    }
  }

  async _sellToken(context) {
    try {
      const { TradeActionDialog } = await import("../ui/trade_action_dialog.js");
      const dialog = new TradeActionDialog();

      const result = await dialog.open({
        action: "sell",
        symbol: context.symbol,
        context: {},
      });

      if (!result) return;

      const body =
        result.percentage === 100
          ? { mint: context.mint, close_all: true }
          : { mint: context.mint, percentage: result.percentage };

      const response = await fetch("/api/trader/manual/sell", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.message || "Sell failed");
      }

      this._showToast("Sell order placed!", "success");
    } catch (error) {
      this._showToast(error.message || "Sell failed", "error");
    }
  }

  async _addToPosition(context) {
    try {
      const { TradeActionDialog } = await import("../ui/trade_action_dialog.js");
      const dialog = new TradeActionDialog();

      const balanceRes = await fetch("/api/wallet/balance");
      const balanceData = await balanceRes.json();
      const balance = balanceData?.sol_balance || 0;

      const result = await dialog.open({
        action: "add",
        symbol: context.symbol,
        context: { balance },
      });

      if (!result) return;

      const response = await fetch("/api/trader/manual/buy", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          mint: context.mint,
          size_sol: result.amount,
        }),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.message || "Add to position failed");
      }

      this._showToast("Added to position!", "success");
    } catch (error) {
      this._showToast(error.message || "Add to position failed", "error");
    }
  }

  _viewTokenDetails(context) {
    // Dispatch custom event that token pages listen for
    window.dispatchEvent(
      new CustomEvent("screenerbot:open-token-details", {
        detail: { mint: context.mint, symbol: context.symbol },
      })
    );
  }

  _openExplorer(mint, explorer) {
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
  }

  /**
   * Load favorites cache from API
   */
  async _loadFavoritesCache() {
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
  }

  /**
   * Check if a token is in favorites
   */
  _isFavorite(mint) {
    return this.favoritesCache.get(mint) === true;
  }

  /**
   * Update favorites cache after toggle
   */
  _updateFavoriteCache(mint, isFavorite) {
    if (isFavorite) {
      this.favoritesCache.set(mint, true);
    } else {
      this.favoritesCache.delete(mint);
    }
  }

  /**
   * Toggle favorite status for a token
   */
  async _toggleFavorite(context, currentlyFavorite) {
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
  }

  async _copyToClipboard(text, label) {
    try {
      await navigator.clipboard.writeText(text);
      this._showToast(`${label} copied!`, "success");
    } catch {
      this._showToast("Failed to copy", "error");
    }
  }

  async _blacklistToken(context) {
    try {
      const { ConfirmationDialog } = await import("../ui/confirmation_dialog.js");

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
  }

  async _refreshToken(context) {
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
  }

  _inspectElement(element) {
    // In Tauri, emit event to open devtools
    if (typeof window.__TAURI__ !== "undefined") {
      window.__TAURI__.event.emit("open-devtools");
    } else {
      // Browser fallback
      console.log("Inspect Element:", element);
      console.dir(element);
    }
  }

  // =========================================================================
  // Utilities
  // =========================================================================

  _getModKey() {
    return navigator.platform.includes("Mac") ? "⌘" : "Ctrl+";
  }

  _showToast(message, type) {
    // Use existing toast system if available
    if (typeof window.showToast === "function") {
      window.showToast(message, type);
    } else {
      // Fallback to Utils module
      import("../core/utils.js")
        .then((Utils) => {
          if (Utils.showToast) {
            Utils.showToast(message, type);
          }
        })
        .catch(() => {
          // Silent fail if utils not available
          console.log(`[${type}] ${message}`);
        });
    }
  }
}

// Create singleton instance
let contextMenu = null;

function getContextMenu() {
  if (!contextMenu) {
    contextMenu = new ContextMenuManager();
  }
  return contextMenu;
}

// Auto-initialize on DOM ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", getContextMenu);
} else {
  getContextMenu();
}

// Export for external use
export { getContextMenu, ContextMenuManager };
export default getContextMenu();
