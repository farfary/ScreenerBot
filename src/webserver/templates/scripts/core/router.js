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

function ensurePageStyles(pageName) {
  if (typeof pageName !== "string" || !pageName) {
    return;
  }
  const registry = window.__PAGE_STYLES__;
  if (!registry || typeof registry !== "object") {
    return;
  }
  if (document.head.querySelector(`style[data-page-style="${pageName}"]`)) {
    return;
  }
  const styles = registry[pageName];
  if (typeof styles !== "string" || !styles.trim()) {
    return;
  }
  const styleTag = document.createElement("style");
  styleTag.setAttribute("data-page-style", pageName);
  styleTag.textContent = styles;
  document.head.appendChild(styleTag);
}

export function setActiveTab(pageName) {
  document.querySelectorAll("nav .tab").forEach((tab) => {
    const tabPage = tab.getAttribute("data-page");
    if (tabPage === pageName) {
      tab.classList.add("active");
    } else {
      tab.classList.remove("active");
    }
  });
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

    ensurePageStyles(pageName);
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
  loadingEl.style.cssText = "padding: 2rem; text-align: center;";
  loadingEl.innerHTML = `
    <div style="font-size: 1.1rem; color: var(--text-secondary);">
      Loading ${pageName}...
    </div>
  `;

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
    ensurePageStyles(pageName);
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

    loadingEl.innerHTML = `
      <div style="padding: 2rem; text-align: center;">
        <h2 style="color: #ef4444;"><i class="icon-triangle-alert"></i> Failed to Load Page</h2>
        <p style="color: #9ca3af; margin-top: 1rem;">
          ${error.message}
        </p>
        <button onclick="location.reload()" style="margin-top: 1rem; padding: 0.5rem 1rem; background: #3b82f6; color: white; border: none; border-radius: 0.375rem; cursor: pointer;">
          Reload Page
        </button>
      </div>
    `;
  }
}

export function initRouter() {
  // Guard against double initialization using a global flag
  // ES module URL mismatch (with/without ?v=xxx) can cause router.js to load twice
  if (window.__ROUTER_INITIALIZED__) {
    console.log("[Router] Already initialized, skipping duplicate initialization");
    return;
  }
  window.__ROUTER_INITIALIZED__ = true;

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
    ensurePageStyles(initialPage);

    // Load and activate page module for cached container (needed for event handlers)
    (async () => {
      try {
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
    ensurePageStyles(initialPage);

    // Try to load and activate page module
    (async () => {
      try {
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
  await waitForReady();

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
