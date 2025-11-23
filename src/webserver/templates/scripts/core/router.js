// Client-Side Router - SPA Navigation
import { PageLifecycleRegistry } from "./lifecycle.js";
import * as AppState from "./app_state.js";
import * as PollingManager from "./poller.js";

// Import TabBarManager for coordinated tab bar management
let TabBarManager = null;
try {
  const tabBarModule = await import("../ui/tab_bar.js");
  TabBarManager = tabBarModule.TabBarManager;
} catch (err) {
  console.warn("[Router] TabBar module not available:", err.message);
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

  Object.values(_state.pageCache).forEach((el) => {
    if (el && el.parentElement === mainContent) {
      mainContent.removeChild(el);
      el.style.display = "none";
    }
  });
}

function displayPageElement(mainContent, pageEl) {
  if (!mainContent || !pageEl) return;

  removeCachedPageElements(mainContent);
  pageEl.style.display = "";
  mainContent.appendChild(pageEl);
}

async function fetchPageContent(pageName, timeoutMs) {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const response = await fetch(`/api/pages/${pageName}`, {
      signal: controller.signal,
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

  const mainContent = document.querySelector("main");
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
      await import(`../pages/${pageName}.js`);
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
        <h2 style="color: #ef4444;"><i class="icon-alert-triangle"></i> Failed to Load Page</h2>
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
  // Initialize polling interval selector
  initPollingIntervalControl();

  // Handle navigation links
  document.addEventListener("click", (e) => {
    const link = e.target.closest("a[data-page]");
    if (!link) return;

    e.preventDefault();
    const pageName = link.getAttribute("data-page");
    if (pageName && pageName !== _state.currentPage) {
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
  const mainContent = document.querySelector("main");
  if (
    mainContent &&
    mainContent.children.length > 0 &&
    !mainContent.querySelector(".page-loading")
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
        await import(`../pages/${initialPage}.js`);
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

/**
 * Initialize polling interval control in header
 * Connects the dropdown to PollingManager
 */
function initPollingIntervalControl() {
  const dropdown = document.getElementById("refreshInterval");
  if (!dropdown) {
    console.warn("[Router] Polling interval dropdown not found");
    return;
  }

  // Load saved interval and set dropdown value
  const currentInterval = PollingManager.getInterval();
  dropdown.value = String(currentInterval);

  // Listen for changes
  dropdown.addEventListener("change", (e) => {
    const newInterval = parseInt(e.target.value, 10);
    if (Number.isFinite(newInterval) && newInterval > 0) {
      PollingManager.setInterval(newInterval);
      console.log(`[Router] Polling interval changed to ${newInterval}ms`);
    }
  });

  console.log("[Router] Polling interval control initialized:", currentInterval, "ms");
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initRouter);
} else {
  initRouter();
}
