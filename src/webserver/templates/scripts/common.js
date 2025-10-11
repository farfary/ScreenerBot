// State Manager - Browser Storage (globally accessible)
window.AppState = {
  save(key, value) {
    try {
      localStorage.setItem(`screenerbot_${key}`, JSON.stringify(value));
    } catch (e) {
      console.warn("Failed to save state:", key, e);
    }
  },

  load(key, defaultValue = null) {
    try {
      const item = localStorage.getItem(`screenerbot_${key}`);
      return item ? JSON.parse(item) : defaultValue;
    } catch (e) {
      console.warn("Failed to load state:", key, e);
      return defaultValue;
    }
  },

  remove(key) {
    try {
      localStorage.removeItem(`screenerbot_${key}`);
    } catch (e) {
      console.warn("Failed to remove state:", key, e);
    }
  },

  clearAll() {
    try {
      Object.keys(localStorage)
        .filter((key) => key.startsWith("screenerbot_"))
        .forEach((key) => localStorage.removeItem(key));
    } catch (e) {
      console.warn("Failed to clear state:", e);
    }
  },
};

// Global Polling Interval Manager
window.PollingManager = {
  _interval: null,
  _listeners: [],

  init() {
    // Load saved interval or default to 1000ms
    this._interval = AppState.load("pollingInterval", 1000);
    console.log(
      "[PollingManager] Initialized with interval:",
      this._interval,
      "ms"
    );
  },

  getInterval() {
    if (this._interval === null) {
      this.init();
    }
    return this._interval;
  },

  setInterval(ms) {
    const oldInterval = this._interval;
    this._interval = ms;
    AppState.save("pollingInterval", ms);
    console.log(
      "[PollingManager] Interval changed from",
      oldInterval,
      "ms to",
      ms,
      "ms"
    );

    // Notify all listeners
    this._listeners.forEach((callback) => {
      try {
        callback(ms, oldInterval);
      } catch (err) {
        console.error("[PollingManager] Listener callback failed:", err);
      }
    });
  },

  onChange(callback) {
    if (typeof callback === "function") {
      this._listeners.push(callback);
    }
    return callback;
  },

  removeListener(callback) {
    const index = this._listeners.indexOf(callback);
    if (index > -1) {
      this._listeners.splice(index, 1);
    }
  },
};

// Initialize on load
PollingManager.init();

// Client-Side Router - SPA Architecture
window.Router = {
  currentPage: null,
  cleanupHandlers: [],
  _timeoutMs: 10000,
  pageCache: {}, // Cache page containers
  initializedPages: {}, // Track which pages have been initialized

  registerCleanup(handler) {
    if (typeof handler === "function") {
      this.cleanupHandlers.push(handler);
    }
    return handler;
  },

  runCleanupHandlers() {
    while (this.cleanupHandlers.length) {
      const handler = this.cleanupHandlers.pop();
      try {
        handler();
      } catch (err) {
        console.error("[Router] Cleanup handler failed:", err);
      }
    }
  },

  trackInterval(intervalId) {
    if (intervalId != null) {
      this.registerCleanup(() => clearInterval(intervalId));
    }
    return intervalId;
  },

  trackTimeout(timeoutId) {
    if (timeoutId != null) {
      this.registerCleanup(() => clearTimeout(timeoutId));
    }
    return timeoutId;
  },

  removeCachedPageElements(mainContent) {
    if (!mainContent) {
      return;
    }

    Object.values(this.pageCache).forEach((el) => {
      if (el && el.parentElement === mainContent) {
        mainContent.removeChild(el);
        el.style.display = "none";
      }
    });
  },

  displayPageElement(mainContent, pageEl) {
    if (!mainContent || !pageEl) {
      return;
    }

    this.removeCachedPageElements(mainContent);
    pageEl.style.display = "";
    mainContent.appendChild(pageEl);
  },

  async loadPage(pageName) {
    console.log("[Router] Loading page:", pageName);

    // Update current page
    this.currentPage = pageName;

    // Run cleanup handlers for previous page
    this.runCleanupHandlers();

    // Update active tab styling
    document.querySelectorAll("nav .tab").forEach((tab) => {
      const tabPage = tab.getAttribute("data-page");
      if (tabPage === pageName) {
        tab.classList.add("active");
      } else {
        tab.classList.remove("active");
      }
    });

    const mainContent = document.querySelector("main");
    if (!mainContent) {
      console.error("[Router] Main content container not found");
      return;
    }

    // Check if page is already cached
    let pageEl = this.pageCache[pageName];

    if (pageEl) {
      // Page is cached - just swap visibility
      console.log("[Router] Using cached page:", pageName);

      this.displayPageElement(mainContent, pageEl);

      // Initialize page-specific scripts
      this.initPageScripts(pageName);

      // Clean up sub-tabs and toolbar for pages that don't use them
      cleanupTabContainers();

      // Update browser history only if path actually changed
      const targetUrl = pageName === "home" ? "/" : `/${pageName}`;
      if (window.location.pathname !== targetUrl) {
        window.history.pushState({ page: pageName }, "", targetUrl);
      }

      // Save last visited tab
      AppState.save("lastTab", pageName);

      console.log("[Router] Cached page displayed:", pageName);
      return;
    }

    // Page not cached - fetch and create
    mainContent.setAttribute("data-loading", "true");
    this.removeCachedPageElements(mainContent);

    // Show loading indicator in a temporary container
    const loadingEl = document.createElement("div");
    loadingEl.className = "page-loading";
    loadingEl.style.cssText = "padding: 2rem; text-align: center;";
    loadingEl.innerHTML = `
                    <div style="font-size: 1.1rem; color: var(--text-secondary);">
                        Loading ${pageName}...
                    </div>
                `;

    // Hide all existing pages and show loading
    Object.values(this.pageCache).forEach((el) => {
      el.style.display = "none";
    });
    mainContent.appendChild(loadingEl);

    // Fetch page content from API with timeout protection
    try {
      const html = await this.fetchPageContent(pageName, this._timeoutMs);

      // Create new page container
      pageEl = document.createElement("div");
      pageEl.className = "page-container";
      pageEl.id = `page-${pageName}`;
      pageEl.setAttribute("data-page", pageName);
      pageEl.innerHTML = html;

      // Cache the page
      this.pageCache[pageName] = pageEl;

      // Remove loading indicator
      loadingEl.remove();

      // Add to DOM, removing other cached pages from the container
      this.displayPageElement(mainContent, pageEl);

      // Execute embedded scripts
      this.executeEmbeddedScripts(pageEl);

      // Initialize page-specific scripts (only once)
      this.initPageScripts(pageName);

      // Clean up sub-tabs and toolbar for pages that don't use them
      cleanupTabContainers();

      // Update browser history only if path actually changed
      const targetUrl = pageName === "home" ? "/" : `/${pageName}`;
      if (window.location.pathname !== targetUrl) {
        window.history.pushState({ page: pageName }, "", targetUrl);
      }

      // Save last visited tab
      AppState.save("lastTab", pageName);

      console.log("[Router] New page loaded and cached:", pageName);
    } catch (error) {
      console.error("[Router] Failed to load page:", pageName, error);

      // Show error in loading container
      loadingEl.innerHTML = `
                        <div style="padding: 2rem; text-align: center;">
                            <h2 style="color: #ef4444;">⚠️ Failed to Load Page</h2>
                            <p style="color: #9ca3af; margin-top: 1rem;">
                                ${error.message}
                            </p>
                            <button onclick="Router.loadPage('${pageName}')" 
                                style="margin-top: 1rem; padding: 0.5rem 1rem; 
                                       background: #3b82f6; color: white; border: none; 
                                       border-radius: 0.5rem; cursor: pointer;">
                                Retry
                            </button>
                        </div>
                    `;
    } finally {
      mainContent.removeAttribute("data-loading");
    }
  },

  async fetchPageContent(pageName, timeoutMs = 10000) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
    const url = `/api/pages/${pageName}`;

    try {
      const response = await fetch(url, {
        signal: controller.signal,
        headers: { "X-Requested-With": "fetch" },
      });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      return await response.text();
    } catch (error) {
      if (error.name === "AbortError") {
        throw new Error(
          `Request timed out after ${Math.round(timeoutMs / 1000)}s`
        );
      }
      throw error;
    } finally {
      clearTimeout(timeoutId);
    }
  },

  executeEmbeddedScripts(container) {
    if (!container) return;

    const scripts = container.querySelectorAll("script");
    scripts.forEach((script) => {
      const newScript = document.createElement("script");
      Array.from(script.attributes).forEach((attr) =>
        newScript.setAttribute(attr.name, attr.value)
      );

      if (script.src) {
        newScript.src = script.src;
      } else {
        newScript.textContent = script.textContent;
      }

      script.parentNode?.replaceChild(newScript, script);
    });
  },

  initPageScripts(pageName) {
    // Only initialize each page once (on first load)
    if (this.initializedPages[pageName]) {
      console.log(
        "[Router] Page already initialized, skipping init:",
        pageName
      );

      switch (pageName) {
        case "status":
          if (typeof ensureStatusSubTabsVisible === "function") {
            ensureStatusSubTabsVisible();
          }
          break;
        case "tokens":
          if (typeof ensureTokensPageReady === "function") {
            ensureTokensPageReady();
          }
          break;
        case "services":
          if (typeof ensureServicesPageReady === "function") {
            ensureServicesPageReady();
          }
          break;
      }

      return;
    }

    console.log("[Router] Initializing page for first time:", pageName);

    // Re-initialize page-specific functionality after dynamic load
    switch (pageName) {
      case "home":
        if (typeof initHomePage === "function") initHomePage();
        break;
      case "status":
        if (typeof initStatusSubTabs === "function") initStatusSubTabs();
        if (typeof ensureStatusSubTabsVisible === "function")
          ensureStatusSubTabsVisible();
        break;
      case "tokens":
        if (typeof initTokensPage === "function") initTokensPage();
        break;
      case "positions":
        if (typeof initPositionsPage === "function") initPositionsPage();
        break;
      case "transactions":
        if (typeof initTransactionsPage === "function") initTransactionsPage();
        break;
      case "events":
        if (typeof initEventsPage === "function") initEventsPage();
        break;
      case "config":
        if (typeof initConfigPage === "function") initConfigPage();
        break;
      case "services":
        if (typeof initServicesPage === "function") initServicesPage();
        break;
    }

    // Mark as initialized
    this.initializedPages[pageName] = true;
  },
};

// Initialize router on page load
document.addEventListener("DOMContentLoaded", () => {
  // Set initial page from URL
  const currentPath = window.location.pathname;
  const initialPage = currentPath === "/" ? "home" : currentPath.substring(1);
  Router.currentPage = initialPage;

  const mainContent = document.querySelector("main");
  if (mainContent) {
    let initialContainer = mainContent.querySelector(".page-container");

    if (!initialContainer) {
      initialContainer = document.createElement("div");
      initialContainer.className = "page-container";
      initialContainer.id = `page-${initialPage}`;
      initialContainer.setAttribute("data-page", initialPage);

      while (mainContent.firstChild) {
        initialContainer.appendChild(mainContent.firstChild);
      }

      mainContent.appendChild(initialContainer);
    }

    Router.pageCache[initialPage] = initialContainer;
    Router.initializedPages[initialPage] = true;

    if (typeof Router.initPageScripts === "function") {
      Router.initPageScripts(initialPage);
    }
  }

  // Intercept all navigation link clicks
  document.addEventListener("click", (e) => {
    const target = e.target.closest("a[data-page]");
    if (target) {
      e.preventDefault();
      const pageName = target.getAttribute("data-page");
      Router.loadPage(pageName);
    }
  });

  // Handle browser back/forward buttons
  window.addEventListener("popstate", (e) => {
    if (e.state && e.state.page) {
      Router.loadPage(e.state.page);
    } else {
      const path = window.location.pathname;
      const page = path === "/" ? "home" : path.substring(1);
      Router.loadPage(page);
    }
  });

  // Save initial state
  AppState.save("lastTab", initialPage);
  cleanupTabContainers();

  console.log("[Router] Initialized - SPA mode active");
});

// Helper function to hide sub-tabs/toolbar containers
function cleanupTabContainers() {
  const subTabsContainer = document.getElementById("subTabsContainer");
  const toolbarContainer = document.getElementById("toolbarContainer");

  // Only hide if not on a page that explicitly shows them
  // Pages can call initPageSubTabs() to show and populate them
  const activePage =
    (window.Router && Router.currentPage) ||
    (window.location.pathname === "/"
      ? "home"
      : window.location.pathname.substring(1));

  const pagesWithSubTabs = ["tokens", "status"]; // Add more as needed
  const pagesWithToolbar = ["tokens"];

  if (!pagesWithSubTabs.includes(activePage)) {
    if (subTabsContainer) {
      subTabsContainer.style.display = "none";
      subTabsContainer.innerHTML = "";
    }
  }

  if (!pagesWithToolbar.includes(activePage)) {
    if (toolbarContainer) {
      toolbarContainer.style.display = "none";
      toolbarContainer.innerHTML = "";
    }
  }
}

let statusPollInterval = null;

function setBotBadge(state, message) {
  const badge = document.getElementById("botBadge");
  if (!badge) return;

  switch (state) {
    case "running":
      badge.className = "badge online";
      badge.innerHTML = message || "🤖 Running";
      badge.title = "Bot: Running";
      break;
    case "stopped":
      badge.className = "badge error";
      badge.innerHTML = message || "🤖 Stopped";
      badge.title = "Bot: Stopped";
      break;
    case "error":
      badge.className = "badge error";
      badge.innerHTML = message || "🤖 Error";
      badge.title = "Bot: Error";
      break;
    case "starting":
      badge.className = "badge loading";
      badge.innerHTML = message || "🤖 Starting";
      badge.title = "Bot: Starting...";
      break;
    default:
      badge.className = "badge loading";
      badge.innerHTML = message || "🤖 BOT";
      badge.title = "Bot: Unknown";
      break;
  }
}

function deriveAllReady(payload) {
  if (!payload || typeof payload !== "object") return null;

  if (typeof payload.all_ready === "boolean") {
    return payload.all_ready;
  }

  if (payload.services) {
    if (typeof payload.services.all_ready === "boolean") {
      return payload.services.all_ready;
    }

    const serviceStatuses = Object.values(payload.services);
    if (serviceStatuses.length > 0) {
      return serviceStatuses.every(
        (status) =>
          typeof status === "string" && status.toLowerCase().includes("healthy")
      );
    }
  }

  return null;
}

function renderStatusBadgesFromSnapshot(snapshot) {
  if (!snapshot || typeof snapshot !== "object") {
    setBotBadge("error", "🤖 Error");
    return;
  }

  // Update bot status badge
  const allReady = deriveAllReady(snapshot);
  const tradingEnabled = snapshot.trading_enabled;

  if (allReady === true) {
    if (tradingEnabled === true) {
      setBotBadge("running", "🤖 Running");
    } else if (tradingEnabled === false) {
      setBotBadge("stopped", "🤖 Stopped");
    } else {
      setBotBadge("running", "🤖 Ready");
    }
  } else if (allReady === false) {
    setBotBadge("starting", "🤖 Starting");
  } else {
    setBotBadge("starting", "🤖 Connecting");
  }
}

async function fetchStatusSnapshot() {
  try {
    const res = await fetch("/api/status");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = await res.json();
    renderStatusBadgesFromSnapshot(data);
    return data;
  } catch (error) {
    console.warn("Failed to fetch status snapshot:", error);
    setBotBadge("error", "🤖 Error");
    return null;
  }
}

function startStatusPolling(intervalMs = 5000) {
  if (statusPollInterval) return;
  statusPollInterval = setInterval(fetchStatusSnapshot, intervalMs);
}

function stopStatusPolling() {
  if (!statusPollInterval) return;
  clearInterval(statusPollInterval);
  statusPollInterval = null;
}

// Initialize (refresh silently every 1s)
fetchStatusSnapshot();
startStatusPolling();

if (window.Router && typeof Router.registerCleanup === "function") {
  Router.registerCleanup(() => {
    stopStatusPolling();
  });
}

// Show Debug Modal
async function showDebugModal(mint, type) {
  const modal = document.getElementById("debugModal");
  const endpoint =
    type === "position"
      ? `/api/positions/${mint}/debug`
      : `/api/tokens/${mint}/debug`;

  modal.classList.add("show");

  try {
    const response = await fetch(endpoint);
    const data = await response.json();
    populateDebugModal(data, type);
  } catch (error) {
    Utils.showToast("❌ Failed to load debug info: " + error, "error");
    console.error("Debug modal error:", error);
  }
}

// Close Debug Modal
function closeDebugModal() {
  document.getElementById("debugModal").classList.remove("show");
}

// Switch Debug Modal Tabs
function switchDebugTab(tabName) {
  // Update tab buttons
  document.querySelectorAll(".modal-tab").forEach((tab) => {
    tab.classList.remove("active");
  });
  event.currentTarget.classList.add("active");

  // Update tab content
  document.querySelectorAll(".modal-tab-content").forEach((content) => {
    content.classList.remove("active");
  });
  document.getElementById(`tab-${tabName}`).classList.add("active");
}

// Populate Debug Modal with Data (no per-field copy buttons)
function populateDebugModal(data, type) {
  // Store mint for copying
  const mintAddress = data.mint;

  // Token Info Tab
  const tokenInfo = data.token_info || {};
  document.getElementById("tokenSymbol").textContent =
    tokenInfo.symbol || "N/A";
  document.getElementById("tokenName").textContent = tokenInfo.name || "N/A";
  document.getElementById("tokenDecimals").textContent =
    tokenInfo.decimals || "N/A";
  document.getElementById("tokenWebsite").innerHTML = tokenInfo.website
    ? `<a href="${tokenInfo.website}" target="_blank" style="color: var(--link-color);">${tokenInfo.website}</a>`
    : '<span class="debug-value-text">N/A</span>';
  document.getElementById("tokenVerified").textContent = tokenInfo.is_verified
    ? "✅ Yes"
    : "❌ No";
  document.getElementById("tokenTags").textContent =
    tokenInfo.tags?.join(", ") || "None";

  // Add mint address display at the top
  document.getElementById("debugMintAddress").textContent =
    mintAddress || "N/A";

  // Price Data Tab
  const priceData = data.price_data || {};
  document.getElementById("priceSol").textContent = priceData.pool_price_sol
    ? priceData.pool_price_sol.toFixed(9)
    : "N/A";
  document.getElementById("priceConfidence").textContent = priceData.confidence
    ? (priceData.confidence * 100).toFixed(1) + "%"
    : "N/A";
  document.getElementById("priceUpdated").textContent = priceData.last_updated
    ? new Date(priceData.last_updated * 1000).toLocaleString()
    : "N/A";

  // Market Data
  const marketData = data.market_data || {};
  document.getElementById("marketCap").textContent = marketData.market_cap
    ? "$" + marketData.market_cap.toLocaleString()
    : "N/A";
  document.getElementById("fdv").textContent = marketData.fdv
    ? "$" + marketData.fdv.toLocaleString()
    : "N/A";
  document.getElementById("liquidity").textContent = marketData.liquidity_usd
    ? "$" + marketData.liquidity_usd.toLocaleString()
    : "N/A";
  document.getElementById("volume24h").textContent = marketData.volume_24h
    ? "$" + marketData.volume_24h.toLocaleString()
    : "N/A";

  // Pool Data Tab
  const poolsHtml = (data.pools || [])
    .map(
      (pool) => `
                <div class="debug-section">
                    <div class="debug-row">
                        <span class="debug-label">Pool Address:</span>
                        <span class="debug-value"><span class="debug-value-text">${
                          pool.pool_address
                        }</span></span>
                    </div>
                    <div class="debug-row">
                        <span class="debug-label">DEX:</span>
                        <span class="debug-value"><span class="debug-value-text">${
                          pool.dex_name
                        }</span></span>
                    </div>
                    <div class="debug-row">
                        <span class="debug-label">SOL Reserves:</span>
                        <span class="debug-value"><span class="debug-value-text">${pool.sol_reserves.toFixed(
                          2
                        )}</span></span>
                    </div>
                    <div class="debug-row">
                        <span class="debug-label">Token Reserves:</span>
                        <span class="debug-value"><span class="debug-value-text">${pool.token_reserves.toFixed(
                          2
                        )}</span></span>
                    </div>
                    <div class="debug-row">
                        <span class="debug-label">Price (SOL):</span>
                        <span class="debug-value"><span class="debug-value-text">${pool.price_sol.toFixed(
                          9
                        )}</span></span>
                    </div>
                </div>
            `
    )
    .join("");
  document.getElementById("poolsList").innerHTML =
    poolsHtml || "<p>No pool data available</p>";

  // Security Tab
  const security = data.security || {};
  document.getElementById("securityScore").textContent =
    security.score ?? "N/A";
  document.getElementById("securityRugged").textContent = security.rugged
    ? "❌ Yes"
    : "✅ No";
  document.getElementById("securityHolders").textContent =
    security.total_holders ?? "N/A";
  document.getElementById("securityTop10").textContent =
    security.top_10_concentration != null
      ? security.top_10_concentration.toFixed(2) + "%"
      : "N/A";
  document.getElementById("securityMintAuth").textContent =
    security.mint_authority ?? "None";
  document.getElementById("securityFreezeAuth").textContent =
    security.freeze_authority ?? "None";

  const risksHtml = (security.risks || [])
    .map(
      (risk) => `
                <div class="debug-row">
                    <span class="debug-label">${risk.name}:</span>
                    <span class="debug-value">${risk.level} (${risk.description})</span>
                </div>
            `
    )
    .join("");
  document.getElementById("securityRisks").innerHTML =
    risksHtml || "<p>No risks detected</p>";

  // Position-specific data
  if (type === "position" && data.position_data) {
    const posData = data.position_data;
    document.getElementById("positionOpenPositions").textContent =
      posData.open_position ? "1 Open" : "None";
    document.getElementById("positionClosedCount").textContent =
      posData.closed_positions_count;
    document.getElementById("positionTotalPnL").textContent =
      posData.total_pnl.toFixed(4) + " SOL";
    document.getElementById("positionWinRate").textContent =
      posData.win_rate.toFixed(1) + "%";

    if (posData.open_position) {
      const open = posData.open_position;
      document.getElementById("positionEntryPrice").textContent =
        open.entry_price.toFixed(9);
      document.getElementById("positionEntrySize").textContent =
        open.entry_size_sol.toFixed(4) + " SOL";
      document.getElementById("positionCurrentPrice").textContent =
        open.current_price ? open.current_price.toFixed(9) : "N/A";
      document.getElementById("positionUnrealizedPnL").textContent =
        open.unrealized_pnl
          ? open.unrealized_pnl.toFixed(4) +
            " SOL (" +
            open.unrealized_pnl_percent.toFixed(2) +
            "%)"
          : "N/A";
    }
  }
}

// =============================================================================
// TRADER CONTROL SYSTEM
// =============================================================================

let traderStatusPollInterval = null;
let isReconnecting = false;

// Initialize trader controls on page load
function initializeTraderControls() {
  const traderToggle = document.getElementById("traderToggle");
  const rebootBtn = document.getElementById("rebootBtn");

  if (traderToggle) {
    traderToggle.addEventListener("click", toggleTrader);
  }

  if (rebootBtn) {
    rebootBtn.addEventListener("click", rebootSystem);
  }

  // Start polling trader status
  updateTraderStatus();
  traderStatusPollInterval = setInterval(updateTraderStatus, 2000);

  if (window.Router && typeof Router.registerCleanup === "function") {
    Router.registerCleanup(() => {
      if (traderStatusPollInterval) {
        clearInterval(traderStatusPollInterval);
        traderStatusPollInterval = null;
      }
    });
  }
}

// Update trader status from API
async function updateTraderStatus() {
  if (isReconnecting) return; // Skip during reconnect

  try {
    const res = await fetch("/api/trader/status");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);

    const data = await res.json();
    const status = data.data || data;

    updateTraderUI(status.enabled, status.running);
  } catch (error) {
    console.warn("Failed to fetch trader status:", error);
    // Don't update UI on transient network errors
  }
}

// Update trader UI based on status
function updateTraderUI(enabled, running) {
  const traderToggle = document.getElementById("traderToggle");
  const traderIcon = document.getElementById("traderIcon");
  const traderText = document.getElementById("traderText");

  if (!traderToggle || !traderIcon || !traderText) return;

  // Remove existing state classes
  traderToggle.classList.remove("running", "stopped");

  if (enabled && running) {
    traderToggle.classList.add("running");
    traderIcon.textContent = "▶️";
    traderText.textContent = "Trader Running";
  } else {
    traderToggle.classList.add("stopped");
    traderIcon.textContent = "⏸️";
    traderText.textContent = "Trader Stopped";
  }

  traderToggle.disabled = false;
}

// Toggle trader on/off
async function toggleTrader() {
  const traderToggle = document.getElementById("traderToggle");
  const traderIcon = document.getElementById("traderIcon");
  const traderText = document.getElementById("traderText");

  if (!traderToggle) return;

  // Determine current state from UI
  const isRunning = traderToggle.classList.contains("running");
  const endpoint = isRunning ? "/api/trader/stop" : "/api/trader/start";
  const action = isRunning ? "Stopping" : "Starting";

  // Disable button and show loading state
  traderToggle.disabled = true;
  traderIcon.textContent = "⏳";
  traderText.textContent = `${action}...`;

  try {
    const res = await fetch(endpoint, { method: "POST" });
    const data = await res.json();

    if (!res.ok || !data.success) {
      throw new Error(data.error || data.message || "Request failed");
    }

    // Update UI based on response
    const status = data.status || data.data?.status || {};
    updateTraderUI(status.enabled, status.running);

    const message =
      data.message ||
      data.data?.message ||
      (isRunning
        ? "Trader stopped successfully"
        : "Trader started successfully");
    Utils.showToast(`✅ ${message}`);

    // Immediate status refresh
    setTimeout(updateTraderStatus, 500);
  } catch (error) {
    console.error("Trader toggle error:", error);
    Utils.showToast(
      `❌ Failed to ${isRunning ? "stop" : "start"} trader: ${error.message}`,
      "error"
    );

    // Restore previous state
    updateTraderUI(isRunning, isRunning);
  }
}

// Reboot the entire system
async function rebootSystem() {
  const rebootBtn = document.getElementById("rebootBtn");
  if (!rebootBtn) return;

  // Confirm action
  if (
    !confirm(
      "⚠️ Are you sure you want to reboot ScreenerBot? This will restart the entire process."
    )
  ) {
    return;
  }

  // Disable button and show loading
  rebootBtn.disabled = true;
  const originalHTML = rebootBtn.innerHTML;
  rebootBtn.innerHTML = "<span>⏳</span><span>Rebooting...</span>";

  try {
    const res = await fetch("/api/system/reboot", { method: "POST" });
    const data = await res.json();

    if (!res.ok || !data.success) {
      throw new Error(data.error || "Reboot request failed");
    }

    Utils.showToast("🔄 System reboot initiated. Reconnecting...", "info");

    // Start reconnection attempts
    isReconnecting = true;
    if (traderStatusPollInterval) {
      clearInterval(traderStatusPollInterval);
      traderStatusPollInterval = null;
    }

    attemptReconnect();
  } catch (error) {
    console.error("Reboot error:", error);
    Utils.showToast(`❌ Failed to initiate reboot: ${error.message}`, "error");

    // Restore button
    rebootBtn.disabled = false;
    rebootBtn.innerHTML = originalHTML;
  }
}

// Attempt to reconnect after reboot
async function attemptReconnect() {
  const maxAttempts = 60; // 60 attempts = 2 minutes
  let attempt = 0;

  const checkConnection = async () => {
    attempt++;

    try {
      const res = await fetch("/api/status", {
        cache: "no-cache",
        signal: AbortSignal.timeout(3000),
      });

      if (res.ok) {
        Utils.showToast("✅ System reconnected successfully!");

        // Reload the page to refresh all state
        setTimeout(() => {
          window.location.reload();
        }, 1000);
        return;
      }
    } catch (error) {
      // Connection failed, continue trying
    }

    if (attempt < maxAttempts) {
      Utils.showToast(`🔄 Reconnecting... (${attempt}/${maxAttempts})`, "info");
      setTimeout(checkConnection, 2000);
    } else {
      Utils.showToast(
        "❌ Reconnection timeout. Please refresh the page manually.",
        "error"
      );
      isReconnecting = false;

      // Re-enable reboot button
      const rebootBtn = document.getElementById("rebootBtn");
      if (rebootBtn) {
        rebootBtn.disabled = false;
        rebootBtn.innerHTML = "<span>🔄</span><span>Reboot</span>";
      }
    }
  };

  // Wait 3 seconds before first attempt (give system time to restart)
  setTimeout(checkConnection, 3000);
}

// Show notification toast
function showNotification(message, type = "info") {
  Utils.showToast(message, type);
}

// Initialize refresh interval dropdown
function initializeRefreshInterval() {
  const dropdown = document.getElementById("refreshInterval");
  if (!dropdown) {
    console.warn("[PollingManager] Refresh interval dropdown not found");
    return;
  }

  // Set current value
  const currentInterval = PollingManager.getInterval();
  dropdown.value = currentInterval.toString();

  // Handle changes
  dropdown.addEventListener("change", (e) => {
    const newInterval = parseInt(e.target.value, 10);
    if (!isNaN(newInterval) && newInterval > 0) {
      PollingManager.setInterval(newInterval);
      Utils.showToast(
        `⏱️ Refresh interval set to ${formatInterval(newInterval)}`,
        "success"
      );
    }
  });

  console.log(
    "[PollingManager] Dropdown initialized with:",
    currentInterval,
    "ms"
  );
}

// Format interval for display
function formatInterval(ms) {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${ms / 1000}s`;
  return `${ms / 60000}m`;
}

// Initialize trader controls when DOM is ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => {
    initializeTraderControls();
    initializeRefreshInterval();
  });
} else {
  initializeTraderControls();
  initializeRefreshInterval();
}
