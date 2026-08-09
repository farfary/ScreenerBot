// Client-Side Router - SPA Navigation
import { PageLifecycleRegistry } from "./lifecycle.js";
import * as AppState from "./app_state.js";
import { waitForReady } from "./bootstrap.js";
import { playClick, playTabSwitch } from "./sounds.js";

const assetVersion = window.__ASSET_VERSION__ || "";
const assetQuery = assetVersion ? `?v=${encodeURIComponent(assetVersion)}` : "";

// Import TabBarManager for coordinated tab bar management
let TabBarManager = null;
try {
  const tabBarModule = await import(`../ui/tab_bar.js${assetQuery}`);
  TabBarManager = tabBarModule.TabBarManager;
} catch (err) {
  console.warn("[Router] TabBar module not available:", err.message);
}

// Import ActionBarManager for coordinated action bar management
let ActionBarManager = null;
try {
  const actionBarModule = await import(`../ui/action_bar.js${assetQuery}`);
  ActionBarManager = actionBarModule.ActionBarManager;
} catch (err) {
  console.warn("[Router] ActionBar module not available:", err.message);
}

const _state = {
  currentPage: null,
  cleanupHandlers: [],
  timeoutMs: 10000,
  pageCache: {},
  initializedPages: {},
};

export function getCurrentPage() {
  return _state.currentPage;
}

function activatePageStyles(pageName) {
  document.head.querySelectorAll("[data-page-style]").forEach((styleEl) => {
    if (styleEl.getAttribute("data-page-style") !== pageName) {
      styleEl.remove();
    }
  });
}

function ensurePageStyles(pageName) {
  if (typeof pageName !== "string" || !pageName) {
    return Promise.resolve();
  }

  const existing = document.head.querySelector(`[data-page-style="${pageName}"]`);
  if (existing) {
    activatePageStyles(pageName);
    return Promise.resolve();
  }

  return new Promise((resolve, reject) => {
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = `/styles/pages/${encodeURIComponent(pageName)}.css${assetQuery}`;
    link.setAttribute("data-page-style", pageName);
    link.addEventListener(
      "load",
      () => {
        activatePageStyles(pageName);
        resolve();
      },
      { once: true }
    );
    link.addEventListener(
      "error",
      () => {
        link.remove();
        reject(new Error(`Stylesheet unavailable for ${pageName}`));
      },
      { once: true }
    );
    document.head.appendChild(link);
  });
}

export function setActiveTab(pageName) {
  let activeTab = null;
  document.querySelectorAll("nav .tab").forEach((tab) => {
    const tabPage = tab.getAttribute("data-page");
    if (tabPage === pageName) {
      tab.classList.add("active");
      tab.setAttribute("aria-current", "page");
      activeTab = tab;
    } else {
      tab.classList.remove("active");
      tab.removeAttribute("aria-current");
    }
  });
  activeTab?.scrollIntoView({ block: "nearest", inline: "nearest" });
}

export function registerCleanup(handler) {
  if (typeof handler === "function") {
    _state.cleanupHandlers.push(handler);
  }
  return handler;
}

export function runCleanupHandlers() {
  while (_state.cleanupHandlers.length) {
    const handler = _state.cleanupHandlers.pop();
    try {
      handler();
    } catch (err) {
      console.error("[Router] Cleanup handler failed:", err);
    }
  }
}

export function trackInterval(intervalId) {
  if (intervalId != null) {
    registerCleanup(() => clearInterval(intervalId));
  }
  return intervalId;
}

export function trackTimeout(timeoutId) {
  if (timeoutId != null) {
    registerCleanup(() => clearTimeout(timeoutId));
  }
  return timeoutId;
}

function removeCachedPageElements(mainContent) {
  if (!mainContent) return;

  // Remove ALL page containers from mainContent, not just cached ones
  // This prevents duplicate content from WebView cache or stale renders
  mainContent.querySelectorAll(".page-container").forEach((el) => {
    el.style.display = "none";
    if (el.parentElement === mainContent) {
      mainContent.removeChild(el);
    }
  });
}

function displayPageElement(mainContent, pageEl) {
  if (!mainContent || !pageEl) return;

  // Remove all existing page containers first
  removeCachedPageElements(mainContent);

  // Only append if not already in mainContent
  if (pageEl.parentElement !== mainContent) {
    mainContent.appendChild(pageEl);
  }
  pageEl.style.display = "";
}

async function fetchPageContent(pageName, timeoutMs) {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const response = await fetch(`/api/pages/${pageName}${assetQuery}`, {
      signal: controller.signal,
      cache: "no-store",
    });

    clearTimeout(timeoutId);

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const html = await response.text();
    return html;
  } catch (error) {
    clearTimeout(timeoutId);
    if (error.name === "AbortError") {
      throw new Error("Request timeout");
    }
    throw error;
  }
}

export async function loadPage(pageName) {
  if (!pageName) return;

  console.log("[Router] Loading page:", pageName);

  const previousPage = _state.currentPage;
  if (previousPage) {
    await PageLifecycleRegistry.deactivate(previousPage);
  }

  runCleanupHandlers();

  _state.currentPage = pageName;
  setActiveTab(pageName);

  // Notify TabBarManager about page switch (deferred to ensure DOM is ready)
  if (TabBarManager) {
    TabBarManager.onPageSwitch(pageName, previousPage);
  }

  // Notify ActionBarManager about page switch (deferred to ensure DOM is ready)
  if (ActionBarManager) {
    ActionBarManager.onPageSwitch(pageName, previousPage);
  }

  // Select main.content specifically - there are multiple <main> elements
  // (onboarding-content, setup-content, content) and we need the visible one
  const mainContent = document.querySelector("main.content");
  if (!mainContent) {
    console.error("[Router] Main content container not found");
    return;
  }

  // Remove any unresolved loading placeholders
  mainContent.querySelectorAll(".page-loading").forEach((el) => el.remove());

  // Cached page path – reuse existing container
  const cachedEl = _state.pageCache[pageName];
  if (cachedEl) {
    console.log("[Router] Using cached page:", pageName);

    await ensurePageStyles(pageName);
    displayPageElement(mainContent, cachedEl);
    await PageLifecycleRegistry.activate(pageName);

    const targetUrl = `/${pageName}`;
    if (window.location.pathname !== targetUrl) {
      window.history.pushState({ page: pageName }, "", targetUrl);
    }

    AppState.save("lastTab", pageName);
    console.log("[Router] Cached page displayed:", pageName);
    return;
  }

  // Page not cached – show loading state and fetch content
  mainContent.setAttribute("data-loading", "true");
  removeCachedPageElements(mainContent);

  const loadingEl = document.createElement("div");
  loadingEl.className = "page-loading";
  loadingEl.innerHTML = '<div class="loading-spinner">Loading…</div>';

  Object.values(_state.pageCache).forEach((el) => {
    el.style.display = "none";
  });
  mainContent.appendChild(loadingEl);

  try {
    const html = await fetchPageContent(pageName, _state.timeoutMs);

    const pageEl = document.createElement("div");
    pageEl.className = "page-container";
    pageEl.id = `page-${pageName}`;
    pageEl.setAttribute("data-page", pageName);
    pageEl.innerHTML = html;

    _state.pageCache[pageName] = pageEl;

    loadingEl.remove();
    await ensurePageStyles(pageName);
    displayPageElement(mainContent, pageEl);

    // Load page-specific module if it exists
    try {
      await import(`../pages/${pageName}.js${assetQuery}`);
    } catch (err) {
      console.warn(`[Router] No module for page ${pageName}:`, err.message);
    }

    await PageLifecycleRegistry.activate(pageName);

    const targetUrl = `/${pageName}`;
    if (window.location.pathname !== targetUrl) {
      window.history.pushState({ page: pageName }, "", targetUrl);
    }

    AppState.save("lastTab", pageName);
    console.log("[Router] New page loaded and cached:", pageName);
  } catch (error) {
    console.error("[Router] Failed to load page:", pageName, error);

    // A connection error (backend crashed / network dropped / restart in
    // progress) gets a calm, auto-recovering offline state rather than a hard
    // error — the connectivity watcher already shows the global overlay, and we
    // reload this page automatically the moment the backend answers again.
    if (isConnectionError(error)) {
      renderOfflinePlaceholder(loadingEl, pageName);
      return;
    }

    loadingEl.innerHTML = `
      <div class="page-load-error">
        <h2><i class="icon-triangle-alert"></i> Failed to Load Page</h2>
        <p>${error.message}</p>
        <button type="button" class="page-load-retry">Retry</button>
      </div>
    `;
    const retryBtn = loadingEl.querySelector(".page-load-retry");
    if (retryBtn) retryBtn.addEventListener("click", () => loadPage(pageName));
  }
}

// Heuristic: did the page fetch fail because the backend was unreachable
// (vs. a real 4xx/5xx from a live server)? `fetch` rejects with a TypeError
// (commonly "Failed to fetch") on connection refused / network down, and our
// fetchPageContent maps an aborted request to "Request timeout".
function isConnectionError(error) {
  if (window.__SB_CONNECTIVITY__ && window.__SB_CONNECTIVITY__.isBackendOnline() === false) {
    return true;
  }
  if (!navigator.onLine) return true;
  const msg = (error && error.message) || "";
  return (
    error instanceof TypeError ||
    msg.includes("Failed to fetch") ||
    msg.includes("NetworkError") ||
    msg.includes("Load failed") ||
    msg === "Request timeout"
  );
}

function renderOfflinePlaceholder(loadingEl, pageName) {
  loadingEl.innerHTML = `
    <div class="page-offline">
      <span class="page-offline-spinner" aria-hidden="true"></span>
      <h2>Waiting for core…</h2>
      <p>The core is unreachable right now. This page will load automatically once the connection is back.</p>
      <button type="button" class="page-load-retry">Retry now</button>
    </div>
  `;
  const retryBtn = loadingEl.querySelector(".page-load-retry");
  if (retryBtn) {
    retryBtn.addEventListener("click", () => {
      if (window.__SB_CONNECTIVITY__) window.__SB_CONNECTIVITY__.pingNow();
      loadPage(pageName);
    });
  }
  // Auto-recover: reload this page the moment the backend comes back.
  const onReconnect = () => {
    window.removeEventListener("screenerbot:reconnected", onReconnect);
    // Only reload if the user is still looking at this (failed) page.
    if (_state.currentPage === pageName) loadPage(pageName);
  };
  window.addEventListener("screenerbot:reconnected", onReconnect, { once: true });
}

export function initRouter() {
  // Guard against double initialization using a global flag
  // ES module URL mismatch (with/without ?v=xxx) can cause router.js to load twice
  if (window.__ROUTER_INITIALIZED__) {
    console.log("[Router] Already initialized, skipping duplicate initialization");
    return;
  }
  window.__ROUTER_INITIALIZED__ = true;

  // Install the global auto-enhancer so every `select[data-custom-select]` across
  // the dashboard (current and dynamically-added) becomes the custom dropdown,
  // without each page having to call enhanceAllSelects() itself.
  import(`../ui/custom_select.js${assetQuery}`)
    .then((m) => m.installGlobalSelectEnhancer && m.installGlobalSelectEnhancer())
    .catch((err) => console.error("[Router] select enhancer install failed", err));

  // Handle navigation links (main nav tabs)
  document.addEventListener("click", (e) => {
    const link = e.target.closest("a[data-page]");
    if (!link) return;

    e.preventDefault();
    const pageName = link.getAttribute("data-page");
    if (pageName && pageName !== _state.currentPage) {
      // Play tab switch sound for main navigation
      playTabSwitch();
      loadPage(pageName);
    }
  });

  // Handle browser back/forward
  window.addEventListener("popstate", (e) => {
    const pageName = e.state?.page || getPageFromPath();
    if (pageName) {
      loadPage(pageName);
    }
  });

  // Cleanup any duplicate/orphan elements from WebView cache before initialization
  const mainContent = document.querySelector("main.content");
  if (mainContent) {
    // Remove duplicate page containers (keep only the first one for each page)
    const seenPages = new Set();
    mainContent.querySelectorAll(".page-container").forEach((container) => {
      const page = container.getAttribute("data-page");
      if (seenPages.has(page)) {
        console.log("[Router] Removing duplicate page container:", page);
        container.remove();
      } else {
        seenPages.add(page);
      }
    });
  }

  // Detect initial page with priority: URL → server-rendered active tab → stored preference → home
  const pathPage = getPageFromPath();
  const serverActiveTab = document.querySelector("nav .tab.active")?.getAttribute("data-page");
  const storedPage = AppState.load("lastTab", null);
  const isStoredPageValid = storedPage
    ? Boolean(document.querySelector(`nav .tab[data-page="${storedPage}"]`))
    : false;
  const initialPage =
    pathPage || serverActiveTab || (isStoredPageValid ? storedPage : null) || "home";

  _state.currentPage = initialPage;
  setActiveTab(initialPage);

  // Check if content is already server-rendered
  // mainContent already queried above for cleanup

  // Check if page container already exists (WebView cache scenario)
  const existingContainer = mainContent?.querySelector(
    `.page-container[data-page="${initialPage}"]`
  );
  if (existingContainer) {
    console.log("[Router] Found existing page container (cached), reusing:", initialPage);
    _state.pageCache[initialPage] = existingContainer;

    // Load and activate page module for cached container (needed for event handlers)
    (async () => {
      try {
        await ensurePageStyles(initialPage);
        await import(`../pages/${initialPage}.js${assetQuery}`);
        await PageLifecycleRegistry.activate(initialPage);
      } catch (err) {
        console.warn(`[Router] No module for cached page ${initialPage}:`, err.message);
      }
    })();
  } else if (
    mainContent &&
    mainContent.children.length > 0 &&
    !mainContent.querySelector(".page-loading") &&
    !mainContent.querySelector(".page-container")
  ) {
    console.log("[Router] Initial page already rendered:", initialPage);

    // Cache the server-rendered content
    const pageEl = document.createElement("div");
    pageEl.className = "page-container";
    pageEl.id = `page-${initialPage}`;
    pageEl.setAttribute("data-page", initialPage);

    // Move existing content into container
    while (mainContent.firstChild) {
      pageEl.appendChild(mainContent.firstChild);
    }
    _state.pageCache[initialPage] = pageEl;
    mainContent.appendChild(pageEl);

    // Try to load and activate page module
    (async () => {
      try {
        await ensurePageStyles(initialPage);
        await import(`../pages/${initialPage}.js${assetQuery}`);
        await PageLifecycleRegistry.activate(initialPage);
      } catch (err) {
        console.warn(`[Router] No module for initial page ${initialPage}:`, err.message);
      }
    })();
  } else {
    // No server-rendered content, fetch it
    console.log("[Router] No server-rendered content, fetching:", initialPage);
    loadPage(initialPage);
  }
}

function getPageFromPath() {
  const path = window.location.pathname;
  if (path === "/" || path === "") {
    return null;
  }
  return path.slice(1);
}

async function bootstrapRouter() {
  const status = await waitForReady();

  // If initialization is required (new install), check onboarding first
  if (status && status.initialization_required) {
    console.log("[Router] Initialization required, checking onboarding status...");

    // Add initialization-mode class to hide dashboard elements
    // This prevents dashboard from showing through during onboarding/setup
    document.body.classList.add("initialization-mode");

    // Check if onboarding needs to be shown first
    if (!status.onboarding_complete) {
      console.log("[Router] Showing onboarding introduction...");

      // Show onboarding screen
      const onboardingEl = document.getElementById("onboardingScreen");
      if (onboardingEl) {
        onboardingEl.style.display = "grid";
        // Initialize onboarding controller
        if (window.OnboardingController) {
          window.OnboardingController.init();
        }
      }

      // Update URL to reflect onboarding mode
      window.history.replaceState(null, "", "/initialization");

      // Don't initialize the main router - user must complete onboarding first
      return;
    }

    // Onboarding done, show setup screen
    console.log("[Router] Onboarding complete, showing setup screen...");

    // Show the setup wrapper and screen
    const wrapperEl = document.getElementById("setupScreenWrapper");
    if (wrapperEl) {
      wrapperEl.style.display = "block";
    }

    const setupEl = document.getElementById("setupScreen");
    if (setupEl) {
      setupEl.style.display = "grid";
      // Initialize setup controller
      if (window.SetupController) {
        window.SetupController.init();
      }
    }

    // Update URL to reflect setup mode
    window.history.replaceState(null, "", "/initialization");

    // Don't initialize the main router - user must complete setup first
    return;
  }

  // Initialize AppState from server before pages load
  // All state is stored server-side, no localStorage
  try {
    await AppState.init();
  } catch (e) {
    console.warn("[Router] Failed to initialize AppState from server:", e);
  }

  // Global button click sound - subtle audio feedback for all buttons
  document.addEventListener(
    "click",
    (e) => {
      const target = e.target.closest("button, .btn, [role='button']");
      if (target && !target.disabled) {
        playClick();
      }
    },
    true
  );

  initRouter();
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", bootstrapRouter);
} else {
  bootstrapRouter();
}
