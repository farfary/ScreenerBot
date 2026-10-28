// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Header controls for global dashboard interactions (trader toggle + metrics)
import { loadPage } from "./router.js";
import * as Utils from "./utils.js";
import { notificationManager } from "./notifications.js";
import * as NotificationPanel from "../ui/notification_panel.js";
import { openSetup } from "../ui/access_gate.js";
import { subscribeToBootstrap, whenInitialized } from "./bootstrap.js";
import { createHeaderMetrics } from "./header_metrics.js";
import { showSettingsDialog } from "../ui/settings_dialog.js";
import { attachTabScrollStrip } from "../ui/tab_bar.js";
import { attachNavMoreMenu } from "../ui/nav_more_menu.js";
import { dirSign } from "./dom.js";
import { Poller } from "./poller.js";
import { requestManager } from "./request_manager.js";
import { playToggleOn, playToggleOff, playError } from "./sounds.js";
// Side-effect import: registers the `screenerbot:open-token-details` window
// listener so the SOL price card (and any header control) can open the dialog.
// Without this the event fires into the void on pages that don't load the dialog.
import "../ui/token_details_dialog.js";

// Side-effect import: subscribes the action -> toast bridge to the notification
// stream. It must load on every page (a trade can be running whatever the user
// is looking at), and it must load through a plain relative specifier — a
// `<script src=...?v=>` tag would create a SECOND module instance alongside the
// one `ui/manual_trade.js` imports, and every trade would toast twice.
import "./action_toasts.js";

// Side-effect import: registers the Cmd/Ctrl+B and Cmd/Ctrl+Shift+S quick-trade
// shortcuts. Same relative-specifier rule as above — it shares the trade dialog
// and the manual-trade submitter with the rest of the dashboard.
import "../ui/quick_trade_shortcuts.js";
import { apiErrorMessage } from "./request_manager.js";
import { setAutoTraderEnabled } from "../ui/auto_trader_control.js";
import { pushEscapeHandler } from "./escape_stack.js";
import { restartApp } from "./restart.js";

const state = {
  traderEnabled: false,
  traderStatus: "loading",
  available: false,
  loading: false,
  bootstrapping: true,
  bootstrapStatus: null,
};

let bootstrapUnsubscribe = null;
let headerMetrics = null;

function getElements() {
  return {
    connectionStatus: document.getElementById("connectionStatus"),
    connectionIcon: document.getElementById("connectionIcon"),
  };
}

function applyStatus(newStatus) {
  if (!newStatus || typeof newStatus !== "object") {
    return;
  }
  const enabled = typeof newStatus.enabled === "boolean" ? newStatus.enabled : newStatus.running;
  if (typeof enabled === "boolean") state.traderEnabled = enabled;
  state.available = true;
  updateConnectionStatus(true);
  headerMetrics?.syncBotControlState();
}

function setAvailability(isAvailable) {
  state.available = isAvailable;
  updateConnectionStatus(isAvailable);
  headerMetrics?.syncBotControlState();
}

function updateConnectionStatus(isConnected) {
  const elements = getElements();
  if (!elements.connectionStatus || !elements.connectionIcon) {
    return;
  }

  elements.connectionStatus.classList.remove("connected", "disconnected", "connecting");

  if (isConnected) {
    elements.connectionStatus.classList.add("connected");
    elements.connectionIcon.className = "icon-circle-check";
    elements.connectionStatus.title = I18n.t("shell-connection-connected");
  } else {
    elements.connectionStatus.classList.add("disconnected");
    elements.connectionIcon.className = "icon-circle-x";
    elements.connectionStatus.title = I18n.t("shell-connection-waiting");
  }
}

function setLoading(isLoading) {
  state.loading = Boolean(isLoading);
  headerMetrics?.syncBotControlState();
}


// How often the header reads the chain connection while it runs on the ScreenerBot service.
const ACCESS_POLL_MS = 10000;
// A connection still being checked shows only after this long, so a normal start
// never flashes a paused state.
const ACCESS_CHECKING_GRACE_MS = 5000;

const access = {
  status: null,
  poller: null,
  checkingSince: null,
  graceTimer: null,
};

/**
 * The control's content for the current state, or null when it hides: Explore Mode
 * offers setup; on the ScreenerBot service a paused state names itself and the
 * action that resumes it (sign in, or add an RPC at the daily limit).
 */
function accessControlView(status) {
  if (status?.explore_mode) {
    return {
      kind: "explore",
      icon: "icon-info",
      title: I18n.t("shell-explore-title"),
      detail: I18n.t("shell-explore-detail"),
      action: I18n.t("shell-explore-action"),
      label: I18n.attr("shell-explore-control", "aria-label"),
      open: openSetup,
    };
  }
  const chain = access.status;
  if (chain?.mode !== "service" || chain.ready) return null;
  if (chain.state === "checking") {
    const since = access.checkingSince ?? Date.now();
    if (Date.now() - since < ACCESS_CHECKING_GRACE_MS) return null;
  }
  const signIn = chain.state === "sign_in_required" || chain.state === "reauthorization_required";
  const addRpc = chain.state === "daily_limit";
  let action = "";
  if (signIn) action = I18n.t("shell-service-action-signin");
  else if (addRpc) action = I18n.t("shell-service-action-connection");
  return {
    kind: "service",
    icon: chain.state === "checking" ? "icon-cloud" : "icon-circle-pause",
    title: I18n.text(chain.text),
    detail: I18n.textAttr(chain.text, "detail"),
    action,
    label: I18n.attr("shell-service-control", "aria-label"),
    open: () => showSettingsDialog({ tab: signIn ? "account" : "connection" }),
  };
}

// Keep the access control synchronized with the boot mode and the chain connection.
function updateAccessControl() {
  const control = document.getElementById("accessControl");
  if (!control) {
    return;
  }

  const view = accessControlView(state.bootstrapStatus);
  control.hidden = !view;
  control.closest(".modern-header")?.classList.toggle("has-access-control", Boolean(view));
  if (!view) return;

  control.dataset.kind = view.kind;
  control.setAttribute("aria-label", view.label);
  control.title = view.detail;
  control.querySelector(".access-control-icon").className = `access-control-icon ${view.icon}`;
  control.querySelector(".access-control-title").textContent = view.title;
  control.querySelector(".access-control-detail").textContent = view.detail;
  const action = control.querySelector(".access-control-action");
  action.textContent = view.action;
  action.hidden = !view.action;
  control.querySelector(".access-control-arrow").hidden = !view.action;
  control.onclick = view.open;
}

/** Records a chain connection status and repaints the control. */
function applyAccessStatus(status) {
  access.status = status || null;
  if (status?.state === "checking") access.checkingSince ??= Date.now();
  else access.checkingSince = null;
  window.clearTimeout(access.graceTimer);
  if (access.checkingSince !== null) {
    access.graceTimer = window.setTimeout(
      updateAccessControl,
      ACCESS_CHECKING_GRACE_MS - (Date.now() - access.checkingSince)
    );
  }
  updateAccessControl();
}

/**
 * The bootstrap poll ends once the app is ready, so on the ScreenerBot service the
 * header reads the connection itself to see a pause begin and end.
 */
function syncAccessPolling(status) {
  const service = status?.chain_access?.mode === "service" && !status?.explore_mode;
  if (service && !access.poller) {
    access.poller = new Poller(
      async () => {
        try {
          applyAccessStatus(
            await requestManager.fetch("/api/chain/access", { priority: "normal" })
          );
        } catch {
          // An unanswered read keeps the last known state on screen.
        }
      },
      { label: "AccessControl", intervalMs: ACCESS_POLL_MS } // l10n-ignore: internal poller name
    );
    access.poller.start();
  } else if (!service && access.poller) {
    access.poller.cleanup();
    access.poller = null;
  }
}

function applyBootstrapStatus(status) {
  state.bootstrapStatus = status;
  if (status?.chain_access) applyAccessStatus(status.chain_access);
  else updateAccessControl();
  syncAccessPolling(status);
  const initializationRequired = Boolean(status?.initialization_required);
  const uiReady = Boolean(status && (status.ui_ready || initializationRequired));

  state.bootstrapping = !uiReady;

  if (state.bootstrapping) {
    state.available = false;
    state.loading = true;
  }

  if (!uiReady) {
    updateConnectionStatus(false);
    return;
  }

  state.loading = false;
  state.available = true;
  updateConnectionStatus(true);
  headerMetrics?.syncBotControlState();
}

headerMetrics = createHeaderMetrics({ state, setAvailability });

async function controlTrader(action) {
  if (state.loading) {
    return;
  }

  try {
    // The card turns busy only once the request goes out: while the start confirmation
    // is open it keeps its state, so a declined start changes nothing on screen.
    const control = await setAutoTraderEnabled(action === "start", {
      onRequest: () => setLoading(true),
    });
    if (control.cancelled) return;
    const { response: res, payload } = control;

    if (!res.ok) {
      const message = apiErrorMessage(
        payload,
        I18n.t("shell-trader-request-failed-status", { status: String(res.status) })
      );
      throw new Error(message);
    }

    if (payload?.status) {
      applyStatus(payload.status);
    }

    // No toast: the bot card flips to the new state and a confirmation sound
    // plays, so a notice would only repeat what the user just watched happen.
    if (action === "start") {
      playToggleOn();
    } else {
      playToggleOff();
    }
  } catch (err) {
    console.error("[TraderHeader] Control action failed", err);
    playError();
    Utils.showToast({
      key: "trader-control",
      type: "error",
      title: I18n.t("shell-trader-control-failed"),
      message: err.message || null,
    });
    setAvailability(false);
  } finally {
    setLoading(false);
    // Refresh the header immediately so the bot card reflects the new state
    // without waiting for the next metrics tick.
    headerMetrics.fetchHeaderMetrics().catch(() => {});
  }
}

function initTraderControls() {
  if (!bootstrapUnsubscribe) {
    bootstrapUnsubscribe = subscribeToBootstrap(applyBootstrapStatus);
  }

  // Initialize connection status as connecting
  const elements = getElements();
  if (elements.connectionStatus && elements.connectionIcon) {
    elements.connectionStatus.classList.add("connecting");
    elements.connectionIcon.className = "icon-circle-dot";
    elements.connectionStatus.title = I18n.t("shell-connection-waiting");
  }

  // Initialize card click handlers
  initCardHandlers();

  // Initialize the collapsible action drawer (mid screens / touch)
  initHeaderActionsToggle();

  // Initialize settings button
  initSettingsButton();

  // Initialize restart control
  initRestartButton();

  // Initialize notifications
  initNotifications();

  // Initialize notification panel UI
  NotificationPanel.init();

  // Initialize scroll navigation for main header tabs
  initHeaderTabsScroll();

  // Travelling active-tab underline + arrow-key navigation
  initNavTabsIndicator();
  initNavTabsKeyboard();

  whenInitialized(async () => {
    try {
      await headerMetrics.fetchHeaderMetrics();
    } catch {
      // The poller remains active and will recover after a transient first fetch.
    }
    headerMetrics.startMetricsPolling();
  }).catch((error) => {
    console.error("[Header] Failed to initialize after bootstrap", error);
  });
}

/**
 * How the desktop row makes room before the quick actions fold, cheapest first. Each
 * step keeps the previous ones: the access control drops its detail line, the
 * wordmark gives way to the logo glyph, then the control keeps one element: Explore
 * keeps its setup action (the trader card already reads Explore), a paused service
 * keeps the state that names the pause.
 */
const HEADER_FIT_STEPS = Object.freeze([
  Object.freeze([]),
  Object.freeze(["is-access-compact"]),
  Object.freeze(["is-access-compact", "is-brand-compact"]),
  Object.freeze(["is-access-compact", "is-brand-compact", "is-access-minimal"]),
]);
const HEADER_FIT_CLASSES = HEADER_FIT_STEPS.at(-1);

/**
 * The quick actions sit inline while they fit beside the header cards and fold behind
 * one disclosure toggle only when nothing else can make room. The row is measured, not
 * tied to viewport widths: the cards scroll inside their own strip, so the row fits
 * exactly when neither the strip nor the row overflows. Room is made by
 * `HEADER_FIT_STEPS` first, and the actions fold only when every step is not enough.
 * Phones (the two-row header) always fold; the tablet header gives the actions their
 * own cell and never folds.
 */
function initHeaderActionsToggle() {
  const actions = document.getElementById("headerActions");
  const toggle = document.getElementById("headerActionsToggle");
  const items = document.getElementById("headerActionsItems");
  const cards = document.getElementById("headerCards");
  const row = actions?.closest(".header-row-1");
  if (!actions || !toggle || !items || !cards || !row) return;
  const phone = window.matchMedia("(width <= 500px)");
  const tablet = window.matchMedia("(width >= 501px) and (width <= 799px)");
  let releaseEscape = null;

  const isOpen = () => actions.classList.contains("is-open");

  const close = ({ restoreFocus = false } = {}) => {
    if (!isOpen()) return;
    actions.classList.remove("is-open");
    toggle.setAttribute("aria-expanded", "false");
    releaseEscape?.();
    releaseEscape = null;
    if (restoreFocus) toggle.focus();
  };

  const open = () => {
    if (isOpen() || !actions.classList.contains("is-folded")) return;
    actions.classList.add("is-open");
    toggle.setAttribute("aria-expanded", "true");
    releaseEscape = pushEscapeHandler(() => close({ restoreFocus: true }));
  };

  const apply = (step, folded) => {
    HEADER_FIT_CLASSES.forEach((name) => row.classList.toggle(name, step.includes(name)));
    actions.classList.toggle("is-folded", folded);
  };
  const overflows = () =>
    cards.scrollWidth > cards.clientWidth + 1 || row.scrollWidth > row.clientWidth + 1;

  // Applied step by step in one synchronous pass, so nothing paints between the reset
  // and the step that fits. Returns whether the actions folded.
  const fit = () => {
    if (phone.matches || tablet.matches) {
      apply(HEADER_FIT_STEPS[0], phone.matches);
      return phone.matches;
    }
    const fitting = HEADER_FIT_STEPS.find((step) => {
      apply(step, false);
      return !overflows();
    });
    if (fitting) return false;
    apply(HEADER_FIT_CLASSES, true);
    return true;
  };

  let measurePending = false;
  const measure = () => {
    if (measurePending) return;
    measurePending = true;
    // Deferred out of the ResizeObserver callback: folding resizes observed elements.
    requestAnimationFrame(() => {
      measurePending = false;
      if (!fit()) close();
    });
  };

  toggle.addEventListener("click", (event) => {
    event.stopPropagation();
    if (isOpen()) {
      close();
    } else {
      open();
    }
  });

  // A chosen action closes the group before its own handler runs, so a dialog it opens
  // does not find the group still registered as an open overlay.
  items.addEventListener("click", () => close(), true);
  actions.addEventListener("focusout", (event) => {
    if (!actions.contains(event.relatedTarget)) close();
  });
  document.addEventListener("click", (event) => {
    if (isOpen() && !actions.contains(event.target)) close();
  });

  const resizeObserver = new ResizeObserver(measure);
  resizeObserver.observe(row);
  resizeObserver.observe(items);
  cards.querySelectorAll(".header-card").forEach((card) => resizeObserver.observe(card));
  // The access control appears and changes its text after the first fit, often from a
  // timer or a poll, so its own size change has to trigger the fit again.
  const accessControl = document.getElementById("accessControl");
  if (accessControl) resizeObserver.observe(accessControl);
  phone.addEventListener("change", measure);
  tablet.addEventListener("change", measure);
  document.fonts?.ready.then(measure).catch(() => {});
  measure();
}

function initCardHandlers() {
  const brand = document.getElementById("headerBrand");
  brand?.addEventListener("click", () => loadPage("home"));

  // Bot card is both a truthful status summary and the master Auto Trader control.
  const botCard = document.getElementById("botCard");
  if (botCard) {
    botCard.addEventListener("click", () => {
      if (!state.available || state.loading) return;

      if (state.traderStatus === "explore") {
        openSetup();
        return;
      }
      if (["force_stopped", "idle", "entry_paused"].includes(state.traderStatus)) {
        loadPage("trader");
        return;
      }

      controlTrader(state.traderEnabled ? "stop" : "start");
    });
  }

  // The owner-confirmed destination for the compact wallet summary is Positions.
  const walletCard = document.getElementById("walletCard");
  if (walletCard) {
    walletCard.addEventListener("click", () => {
      loadPage("positions");
    });
  }

  // SOL price card - open the SOL/USD chart in the shared token-details dialog.
  // WSOL is charted as SOL's USD price (special-cased server-side), so the same
  // dialog every other token uses works unchanged here.
  document.getElementById("copyCard")?.addEventListener("click", () => loadPage("copy"));

  const solPriceCard = document.getElementById("solPriceCard");
  if (solPriceCard) {
    const openSolDetails = () => {
      window.dispatchEvent(
        new CustomEvent("screenerbot:open-token-details", {
          detail: {
            mint: "So11111111111111111111111111111111111111112",
            symbol: "SOL",
            name: "Solana",
          },
        })
      );
    };
    solPriceCard.addEventListener("click", openSolDetails);
  }

  // Ticker segments - navigate to relevant pages
  const tickerMonitoring = document.getElementById("tickerMonitoring");
  if (tickerMonitoring) {
    tickerMonitoring.addEventListener("click", () => {
      loadPage("tokens");
    });
  }

  const tickerFiltering = document.getElementById("tickerFiltering");
  if (tickerFiltering) {
    tickerFiltering.addEventListener("click", () => {
      loadPage("filtering");
    });
  }

  const tickerPnL = document.getElementById("tickerPnL");
  if (tickerPnL) {
    tickerPnL.addEventListener("click", () => {
      loadPage("positions");
    });
  }

  const tickerServices = document.getElementById("tickerServices");
  if (tickerServices) {
    tickerServices.addEventListener("click", () => {
      loadPage("services");
    });
  }
}

function initSettingsButton() {
  const settingsBtn = document.getElementById("settingsBtn");
  if (!settingsBtn) return;

  settingsBtn.addEventListener("click", () => {
    showSettingsDialog();
  });
}

function initRestartButton() {
  document.getElementById("restartBtn")?.addEventListener("click", () => restartApp());
}

// The main navigation row and the metrics ticker are tab scroll strips: edge fades,
// page buttons and wheel scrolling come from `attachTabScrollStrip`, the owner shared
// with the sub-tab row. The navigation row can instead fold the tabs that do not fit
// into a More menu (`attachNavMoreMenu`), chosen in Settings > Navigation.
let navStrip = null;

function initHeaderTabsScroll() {
  const headerRow = document.querySelector(".header-row-2");
  if (headerRow) navStrip = attachTabScrollStrip(headerRow);
  const navTabs = document.getElementById("navTabs");
  if (navTabs) attachNavMoreMenu(navTabs);
  const tickerRow = document.querySelector(".header-row-3");
  if (tickerRow) attachTabScrollStrip(tickerRow);
}

// ============================================================================
// HEADER TABS ACTIVE INDICATOR + KEYBOARD NAVIGATION
// ============================================================================

/**
 * The row entry that marks the current page: its tab, or the More control while that
 * tab is folded into the More menu.
 */
function currentNavEntry(navTabs) {
  return navTabs.querySelector(":scope > a.tab.active:not([hidden]), .nav-more-toggle.active");
}

/** The focusable entries of the navigation row, in reading order. */
function navRowEntries(navTabs) {
  return [...navTabs.querySelectorAll(":scope > a.tab, .nav-more-toggle")].filter(
    (entry) => !entry.closest("[hidden]")
  );
}

/**
 * Drive the single underline that travels between main tabs.
 *
 * The bar is one element, so switching pages MOVES it instead of fading one bar out
 * and another in. Everything it needs is measured from the active tab and written to
 * two custom properties, so the animation itself stays pure CSS (transform + width).
 *
 * It is a progressive enhancement: the `has-indicator` class is what tells the
 * stylesheet to stop drawing the per-tab `::after` bars, and that class is only added
 * once this runs. If the script never executes, each tab still marks itself.
 *
 * We observe `#navTabs` rather than hooking the router, because two unrelated things
 * change the row: the router (toggling `.active`) and the settings dialog (reordering,
 * adding and removing tabs).
 */
function initNavTabsIndicator() {
  const navTabs = document.getElementById("navTabs");
  if (!navTabs) return;

  let indicator = null;
  let lastActive = null;

  const ensureIndicator = () => {
    if (indicator?.isConnected) return indicator;
    indicator = document.createElement("span");
    indicator.className = "nav-tabs-indicator";
    indicator.setAttribute("aria-hidden", "true");
    navTabs.appendChild(indicator);
    navTabs.classList.add("has-indicator");
    return indicator;
  };

  const update = () => {
    const bar = ensureIndicator();
    const active = currentNavEntry(navTabs);
    if (!active) {
      bar.classList.remove("is-visible");
      return;
    }

    // The bar spans the tab's FULL width, so it is measured edge to edge — no inset.
    // Measured against the row rather than `offsetLeft`: the More control's toggle sits
    // inside its own wrapper.
    const row = navTabs.getBoundingClientRect();
    const box = active.getBoundingClientRect();
    navTabs.style.setProperty("--nav-indicator-x", `${box.left - row.left}px`);
    navTabs.style.setProperty("--nav-indicator-w", `${box.width}px`);
    bar.classList.add("is-visible");

    // A newly active tab is brought into the scrollable row's view; the scroll
    // handler then refreshes the edge fades.
    if (active !== lastActive) {
      lastActive = active;
      navStrip?.reveal(active);
    }

    // Placed first, animated after: otherwise the very first measurement slides the bar
    // in from the row's left edge on every page load.
    if (!bar.classList.contains("is-ready")) {
      requestAnimationFrame(() => bar.classList.add("is-ready"));
    }
  };

  // The router toggles `.active`; the settings dialog reorders, adds and removes tabs.
  // Mutations we caused ourselves (the bar's own classes, the bar being appended) are
  // ignored, or `update()` would re-trigger the observer that called it.
  const observer = new MutationObserver((records) => {
    const ours = records.every((record) => record.target === indicator);
    if (!ours) update();
  });
  observer.observe(navTabs, {
    childList: true,
    subtree: true,
    attributeFilter: ["class", "hidden"],
  });

  // Tab widths follow the font and the row width, so re-measure on both.
  new ResizeObserver(() => update()).observe(navTabs);
  document.fonts?.ready.then(() => update()).catch(() => {});

  requestAnimationFrame(update);
}

/**
 * Arrow-key navigation across the main tabs and the More control. Focus only moves —
 * the tab is followed on Enter, like any link — so a keyboard user can survey the nav
 * without triggering a page change on every keypress. The arrows follow the reading
 * direction: in a right-to-left locale ArrowLeft moves to the next entry.
 */
function initNavTabsKeyboard() {
  const navTabs = document.getElementById("navTabs");
  if (!navTabs) return;

  navTabs.addEventListener("keydown", (event) => {
    const keys = ["ArrowRight", "ArrowLeft", "Home", "End"];
    if (!keys.includes(event.key)) return;

    const tabs = navRowEntries(navTabs);
    const current = tabs.indexOf(document.activeElement);
    if (current === -1) return;

    let next;
    if (event.key === "Home") next = 0;
    else if (event.key === "End") next = tabs.length - 1;
    else {
      const step = (event.key === "ArrowRight" ? 1 : -1) * dirSign();
      next = (current + step + tabs.length) % tabs.length;
    }

    event.preventDefault();
    tabs[next].focus();
    navStrip?.reveal(tabs[next]);
  });
}

let notificationsInitialized = false;

function initNotifications() {
  if (notificationsInitialized) {
    console.warn("[Header] Notifications already initialized, skipping");
    return;
  }

  const notifBtn = document.getElementById("notificationBtn");

  if (!notifBtn) return;

  // The header owns the unread BADGE only. Turning actions into toasts belongs
  // to `core/action_toasts.js` — doing it here as well is what made a single
  // swap raise a "started" and a "completed" toast on top of its own notice.
  notificationManager.subscribe((event) => {
    if (event.type === "summary" && event.summary) {
      updateNotificationBadge(event.summary.unread);
    }
  });

  // Initial badge update
  updateNotificationBadge(notificationManager.getUnreadCount());

  // Toggle drawer on button click
  notifBtn.addEventListener("click", () => {
    NotificationPanel.toggle();
  });

  notificationsInitialized = true;
}

function updateNotificationBadge(count) {
  const badge = document.getElementById("notificationBadge");
  const button = document.getElementById("notificationBtn");
  if (!badge) return;

  badge.textContent = count > 99 ? "99+" : count.toString();
  badge.hidden = count <= 0;
  button?.setAttribute(
    "aria-label",
    count > 0
      ? I18n.t("shell-notification-button-unread", { count: count.toString() })
      : I18n.attr("shell-action-notifications", "aria-label")
  );
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initTraderControls);
} else {
  initTraderControls();
}
