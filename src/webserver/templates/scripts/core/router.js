// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Client-Side Router - SPA Navigation
import { isNetworkError } from "./request_manager.js";
import { PageLifecycleRegistry } from "./lifecycle.js";
import * as AppState from "./app_state.js";
import { waitForReady } from "./bootstrap.js";
import { playClick, playTabSwitch } from "./sounds.js";
import { closeStackedOverlays } from "./escape_stack.js";
import { stripIsolates } from "./format.js";
import { ActionBarManager } from "../ui/action_bar.js";
import { TabBarManager } from "../ui/tab_bar.js";

const assetVersion = window.__ASSET_VERSION__ || "";
const assetQuery = assetVersion ? `?v=${encodeURIComponent(assetVersion)}` : "";

// Routable page ids. Each has a `nav-page-title-<id>` catalog message.
const PAGE_IDS = Object.freeze([
  "home",
  "tokens",
  "positions",
  "events",
  "services",
  "transactions",
  "filtering",
  "wallets",
  "tools",
  "assistant",
  "config",
  "trader",
  "copy",
]);

const _state = {
  currentPage: null,
  pendingPage: null,
  cleanupHandlers: [],
  timeoutMs: 10000,
  pageCache: {},
  navigationId: 0,
  navigationController: null,
  pageStyleLoads: new Map(),
};

export function getCurrentPage() {
  return _state.currentPage;
}

function activatePageStyles(pageName) {
  document.head.querySelectorAll("[data-page-style]").forEach((styleEl) => {
    if (styleEl.getAttribute("data-page-style") !== pageName) {
      const stalePage = styleEl.getAttribute("data-page-style");
      _state.pageStyleLoads.get(stalePage)?.cancel();
      styleEl.remove();
    }
  });
}

function updateDocumentTitle(pageName) {
  const title = PAGE_IDS.includes(pageName)
    ? I18n.t("nav-page-title-" + pageName) // l10n-dynamic: nav-
    : I18n.t("shell-page-title-fallback");
  // Window titles render bidi controls literally, so the message's isolates are removed.
  document.title = stripIsolates(I18n.t("shell-document-title", { page: title }));
}

function waitForPageStylesheet(pageName, link) {
  const pending = _state.pageStyleLoads.get(pageName);
  if (pending) {
    return pending.promise;
  }

  let settle;
  const promise = new Promise((resolve, reject) => {
    let settled = false;
    const timeoutId = setTimeout(() => {
      settle(new Error(`Stylesheet unavailable for ${pageName}`));
    }, _state.timeoutMs);

    settle = (error = null) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeoutId);
      link.removeEventListener("load", onLoad);
      link.removeEventListener("error", onError);
      if (_state.pageStyleLoads.get(pageName)?.link === link) {
        _state.pageStyleLoads.delete(pageName);
      }
      if (error) {
        link.remove();
        reject(error);
      } else {
        resolve();
      }
    };

    const onLoad = () => {
      link.dataset.pageStyleReady = "true";
      settle();
    };
    const onError = () => {
      settle(new Error(`Stylesheet unavailable for ${pageName}`));
    };

    link.addEventListener("load", onLoad, { once: true });
    link.addEventListener("error", onError, { once: true });
  });

  _state.pageStyleLoads.set(pageName, {
    link,
    promise,
    cancel: () => {
      const error = new Error(`Stylesheet load superseded for ${pageName}`);
      error.name = "AbortError";
      settle(error);
    },
  });
  return promise;
}

function ensurePageStyles(pageName) {
  if (typeof pageName !== "string" || !pageName) {
    return Promise.resolve();
  }

  const existing = document.head.querySelector(`[data-page-style="${pageName}"]`);
  if (existing) {
    if (
      existing.tagName !== "LINK" ||
      existing.dataset.pageStyleReady === "true" ||
      existing.sheet
    ) {
      return Promise.resolve();
    }
    return waitForPageStylesheet(pageName, existing);
  }

  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = `/styles/pages/${encodeURIComponent(pageName)}.css${assetQuery}`;
  link.setAttribute("data-page-style", pageName);
  const ready = waitForPageStylesheet(pageName, link);
  document.head.appendChild(link);
  return ready;
}

// The header's tab indicator observes `.active` and scrolls the new tab into view.
export function setActiveTab(pageName) {
  document.querySelectorAll("nav .tab").forEach((tab) => {
    const tabPage = tab.getAttribute("data-page");
    if (tabPage === pageName) {
      tab.classList.add("active");
      tab.setAttribute("aria-current", "page");
    } else {
      tab.classList.remove("active");
      tab.removeAttribute("aria-current");
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

function displayPageElement(mainContent, pageEl) {
  if (!mainContent || !pageEl) return;

  // The content viewport has exactly one owner at a time. Replacing its children
  // prevents the outgoing page and incoming loader from becoming flex siblings.
  mainContent.replaceChildren(pageEl);
  pageEl.style.display = "";
}

function beginPageTransition(mainContent, navigationId) {
  const loadingEl = document.createElement("div");
  loadingEl.className = "page-loading";
  loadingEl.dataset.navigationId = String(navigationId);
  loadingEl.setAttribute("role", "status");
  loadingEl.setAttribute("aria-live", "polite");
  const spinner = document.createElement("div");
  spinner.className = "loading-spinner";
  spinner.textContent = I18n.t("common-loading");
  loadingEl.replaceChildren(spinner);

  // Navigation chrome belongs to the displayed page. Release it before any
  // network/style/module wait so the outgoing page cannot remain half-visible.
  closeStackedOverlays();
  runCleanupHandlers();
  TabBarManager.hideAll();
  ActionBarManager.hideAll();

  mainContent.setAttribute("data-loading", "true");
  mainContent.setAttribute("aria-busy", "true");
  mainContent.replaceChildren(loadingEl);
  return loadingEl;
}

function finishPageTransition(mainContent) {
  mainContent.removeAttribute("data-loading");
  mainContent.removeAttribute("aria-busy");
}

async function fetchPageContent(pageName, timeoutMs, controller) {
  const timeoutId = setTimeout(() => controller.abort("timeout"), timeoutMs);

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
    if (error.name === "AbortError" && controller.signal.reason === "timeout") {
      const timeoutError = new Error("Request timeout");
      timeoutError.name = "TimeoutError";
      throw timeoutError;
    }
    throw error;
  }
}

export async function loadPage(pageName, { historyMode = "push" } = {}) {
  if (!pageName || !PAGE_IDS.includes(pageName)) return;

  const navigationId = ++_state.navigationId;
  _state.navigationController?.abort("superseded");
  const controller = new AbortController();
  _state.navigationController = controller;
  const isCurrentNavigation = () => navigationId === _state.navigationId;

  console.log("[Router] Loading page:", pageName);

  const previousPage = _state.currentPage;

  // Select main.content specifically - there are multiple <main> elements
  // (onboarding-content, setup-content, content) and we need the visible one
  const mainContent = document.querySelector("main.content");
  if (!mainContent) {
    console.error("[Router] Main content container not found");
    return;
  }

  let pageEl = _state.pageCache[pageName] || null;
  _state.pendingPage = pageName;
  setActiveTab(pageName);
  const loadingEl = beginPageTransition(mainContent, navigationId);
  const deactivatePrevious =
    previousPage && previousPage !== pageName
      ? PageLifecycleRegistry.deactivate(previousPage)
      : Promise.resolve();

  try {
    // Stop outgoing pollers/listeners immediately; page acquisition happens
    // only after the old lifecycle has released its resources.
    await deactivatePrevious;
    if (!isCurrentNavigation()) return;

    if (!pageEl) {
      const html = await fetchPageContent(pageName, _state.timeoutMs, controller);
      if (!isCurrentNavigation()) return;

      pageEl = document.createElement("div");
      pageEl.className = "page-container";
      pageEl.id = `page-${pageName}`;
      pageEl.setAttribute("data-page", pageName);
      pageEl.innerHTML = html;
      _state.pageCache[pageName] = pageEl;
    }

    await ensurePageStyles(pageName);
    if (!isCurrentNavigation()) return;

    // Load page-specific module if it exists
    try {
      await import(`../pages/${pageName}.js${assetQuery}`);
    } catch (err) {
      console.warn(`[Router] No module for page ${pageName}:`, err.message);
    }

    if (!isCurrentNavigation()) return;
    activatePageStyles(pageName);
    displayPageElement(mainContent, pageEl);
    finishPageTransition(mainContent);
    _state.currentPage = pageName;
    updateDocumentTitle(pageName);

    const targetUrl = `/${pageName}`;
    if (historyMode === "replace") {
      window.history.replaceState({ page: pageName }, "", targetUrl);
    } else if (historyMode === "push" && window.location.pathname !== targetUrl) {
      window.history.pushState({ page: pageName }, "", targetUrl);
    }

    // Commit the page path before activation. Page-owned subtab controllers can
    // then add their canonical hash to this same history entry without carrying
    // a hash over from the previous page or having the router erase the new one.
    await PageLifecycleRegistry.activate(pageName);
    if (!isCurrentNavigation()) return;
    TabBarManager.onPageSwitch(pageName, previousPage);
    ActionBarManager.onPageSwitch(pageName, previousPage);
    await TabBarManager.syncFromLocation(pageName);
    if (!isCurrentNavigation()) return;

    _state.pendingPage = null;
    AppState.save("lastTab", pageName);
    console.log("[Router] New page loaded and cached:", pageName);
  } catch (error) {
    if (!isCurrentNavigation() || controller.signal.reason === "superseded") {
      loadingEl.remove();
      return;
    }
    console.error("[Router] Failed to load page:", pageName, error);

    try {
      await PageLifecycleRegistry.deactivate(pageName);
    } catch (deactivateError) {
      console.error("[Router] Failed to release incomplete page lifecycle:", deactivateError);
    }

    if (!loadingEl.isConnected) {
      mainContent.replaceChildren(loadingEl);
    }
    mainContent.setAttribute("data-loading", "true");
    mainContent.setAttribute("aria-busy", "true");

    // A connection error (backend crashed / network dropped / restart in
    // progress) keeps the page in its loading state: the connectivity watcher's
    // overlay is the one outage notice, and the page loads the moment the
    // backend answers again.
    if (isConnectionError(error)) {
      loadOnReconnect(loadingEl, pageName);
      return;
    }

    loadingEl.innerHTML = `
      <div class="page-load-error">
        <h2><i class="icon-triangle-alert"></i> <span data-l10n-id="shell-page-load-failed"></span></h2>
        <p>${error.message}</p>
        <button type="button" class="page-load-retry" data-l10n-id="common-action-retry"></button>
      </div>
    `;
    I18n.localizeTree(loadingEl);
    const retryBtn = loadingEl.querySelector(".page-load-retry");
    if (retryBtn) retryBtn.addEventListener("click", () => loadPage(pageName));
  }
}

// Did the page fetch fail because the backend was unreachable (vs. a real
// 4xx/5xx from a live server)? A rejected `fetch` is a transport failure
// (`isNetworkError`), and fetchPageContent maps an aborted request to a
// `TimeoutError`.
function isConnectionError(error) {
  if (window.__SB_CONNECTIVITY__ && window.__SB_CONNECTIVITY__.isBackendOnline() === false) {
    return true;
  }
  if (!navigator.onLine) return true;
  return isNetworkError(error) || error?.name === "TimeoutError";
}

function loadOnReconnect(loadingEl, pageName) {
  const onReconnect = () => {
    // Only reload if this failed navigation is still the visible loading state.
    if (loadingEl.isConnected) loadPage(pageName, { historyMode: "replace" });
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

  // Same contract for numbers: every `<input type="number">` becomes a
  // `.number-field` with our stepper and its unit suffix, wherever it is
  // rendered, so no page carries its own numeric-input styling or behaviour.
  import(`../ui/number_field.js${assetQuery}`)
    .then((m) => m.installGlobalNumberFieldEnhancer && m.installGlobalNumberFieldEnhancer())
    .catch((err) => console.error("[Router] number field enhancer install failed", err));

  // Handle navigation links (main nav tabs)
  document.addEventListener("click", (e) => {
    const link = e.target.closest("a[data-page]");
    if (!link) return;

    e.preventDefault();
    const pageName = link.getAttribute("data-page");
    if (pageName && (pageName !== _state.currentPage || _state.pendingPage !== null)) {
      // Play tab switch sound for main navigation
      playTabSwitch();
      loadPage(pageName);
    }
  });

  // Handle browser back/forward
  window.addEventListener("popstate", (e) => {
    const pageName = e.state?.page || getPageFromPath();
    if (pageName) {
      if (pageName === _state.currentPage && _state.pendingPage === null) {
        void TabBarManager.syncFromLocation(pageName);
      } else {
        loadPage(pageName, { historyMode: "none" });
      }
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

  // Detect initial page with priority: URL → server-rendered active tab → stored
  // preference → home. `/` is server-rendered as the configured startup page
  // (Settings > Startup > Default Page), so the active tab already carries it.
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
  updateDocumentTitle(initialPage);

  // Check if content is already server-rendered
  // mainContent already queried above for cleanup

  // Check if page container already exists (WebView cache scenario)
  const existingContainer = mainContent?.querySelector(
    `.page-container[data-page="${initialPage}"]`
  );
  if (mainContent && serverActiveTab && serverActiveTab !== initialPage) {
    // The URL names another page than the server rendered; never relabel the
    // rendered markup as that page, fetch the real partial.
    mainContent.replaceChildren();
    loadPage(initialPage, { historyMode: "replace" });
  } else if (existingContainer) {
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
    loadPage(initialPage, { historyMode: "replace" });
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
