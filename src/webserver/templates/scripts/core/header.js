// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Header controls for global dashboard interactions (trader toggle + metrics)
import { loadPage } from "./router.js";
import * as Utils from "./utils.js";
import { notificationManager } from "./notifications.js";
import * as NotificationPanel from "../ui/notification_panel.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { subscribeToBootstrap, whenInitialized } from "./bootstrap.js";
import { createHeaderMetrics } from "./header_metrics.js";
import { showSettingsDialog } from "../ui/settings_dialog.js";
import { attachTabScrollStrip } from "../ui/tab_bar.js";
import { SetupDialog } from "../ui/setup_dialog.js";
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
import { pushEscapeHandler } from "./escape_stack.js";

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

// Open the wallet + RPC setup dialog so the user can complete setup from Explore Mode
// without leaving the current page. On success the dialog reloads into full mode.
function openSetupWizard() {
  SetupDialog.show();
}

// Keep the Explore Mode setup action synchronized with the process-wide boot mode.
function updateExploreSetupControl(status) {
  const control = document.getElementById("exploreSetupControl");
  if (!control) {
    return;
  }

  const exploreMode = Boolean(status?.explore_mode);
  control.hidden = !exploreMode;
  control.closest(".modern-header")?.classList.toggle("has-explore-setup", exploreMode);

  if (exploreMode && !control.dataset.bound) {
    control.dataset.bound = "true";
    control.addEventListener("click", openSetupWizard);
  }
}

function applyBootstrapStatus(status) {
  state.bootstrapStatus = status;
  updateExploreSetupControl(status);
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

  setLoading(true);

  const endpoint = action === "start" ? "/api/trader/start" : "/api/trader/stop";

  try {
    const res = await fetch(endpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Requested-With": "fetch",
      },
      cache: "no-store",
    });

    const payload = await res.json().catch(() => null);

    if (!res.ok) {
      const message = apiErrorMessage(payload, `Trader request failed (${res.status})`);
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
 * The quick actions sit inline while they fit beside the header cards and fold behind
 * one disclosure toggle only when they do not. The fold is measured, not tied to a
 * viewport width: the cards scroll inside their own strip, so the actions fit exactly
 * when that strip does not overflow with the actions laid out inline. Phones (the
 * two-row header) always fold; the tablet header gives the actions their own cell and
 * never folds.
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

  // Measured with the actions inline, in one synchronous pass, so nothing paints between
  // the unfold and the re-fold.
  const shouldFold = () => {
    if (phone.matches) return true;
    if (tablet.matches) return false;
    actions.classList.remove("is-folded");
    return cards.scrollWidth > cards.clientWidth + 1 || row.scrollWidth > row.clientWidth + 1;
  };

  let measurePending = false;
  const measure = () => {
    if (measurePending) return;
    measurePending = true;
    // Deferred out of the ResizeObserver callback: folding resizes observed elements.
    requestAnimationFrame(() => {
      measurePending = false;
      const fold = shouldFold();
      actions.classList.toggle("is-folded", fold);
      if (!fold) close();
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
        openSetupWizard();
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
  document.getElementById("restartBtn")?.addEventListener("click", () => handleRestart());
}

// The main navigation row is a tab scroll strip: edge fades, page buttons and wheel
// scrolling come from `attachTabScrollStrip`, the owner shared with the sub-tab row.
let navStrip = null;

function initHeaderTabsScroll() {
  const headerRow = document.querySelector(".header-row-2");
  if (headerRow) navStrip = attachTabScrollStrip(headerRow);
}

// ============================================================================
// HEADER TABS ACTIVE INDICATOR + KEYBOARD NAVIGATION
// ============================================================================

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
 * change the active tab: the router (toggling `.active`) and the settings dialog
 * (rebuilding the whole nav's innerHTML, which would otherwise throw the bar away).
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
    const active = navTabs.querySelector("a.active");
    if (!active) {
      bar.classList.remove("is-visible");
      return;
    }

    // The glow spans the tab's FULL width (its own ends are faded by a mask in CSS), so
    // it is measured edge to edge — no inset.
    navTabs.style.setProperty("--nav-indicator-x", `${active.offsetLeft}px`);
    navTabs.style.setProperty("--nav-indicator-w", `${active.offsetWidth}px`);
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

  // The router toggles `.active`; the settings dialog replaces the tabs wholesale.
  // Mutations we caused ourselves (the bar's own classes, the bar being appended) are
  // ignored, or `update()` would re-trigger the observer that called it.
  const observer = new MutationObserver((records) => {
    const ours = records.every((record) => record.target === indicator);
    if (!ours) update();
  });
  observer.observe(navTabs, { childList: true, subtree: true, attributeFilter: ["class"] });

  // Tab widths follow the font and the row width, so re-measure on both.
  new ResizeObserver(() => update()).observe(navTabs);
  document.fonts?.ready.then(() => update()).catch(() => {});

  requestAnimationFrame(update);
}

/**
 * Arrow-key navigation across the main tabs. Focus only moves — the tab is followed on
 * Enter, like any link — so a keyboard user can survey the nav without triggering a
 * page change on every keypress.
 */
function initNavTabsKeyboard() {
  const navTabs = document.getElementById("navTabs");
  if (!navTabs) return;

  navTabs.addEventListener("keydown", (event) => {
    const keys = ["ArrowRight", "ArrowLeft", "Home", "End"];
    if (!keys.includes(event.key)) return;

    const tabs = [...navTabs.querySelectorAll("a.tab")];
    const current = tabs.indexOf(document.activeElement);
    if (current === -1) return;

    let next;
    if (event.key === "Home") next = 0;
    else if (event.key === "End") next = tabs.length - 1;
    else if (event.key === "ArrowRight") next = (current + 1) % tabs.length;
    else next = (current - 1 + tabs.length) % tabs.length;

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

async function handleRestart() {
  const { confirmed } = await ConfirmationDialog.show({
    title: I18n.t("shell-restart-confirm-title"),
    message: I18n.t("shell-restart-confirm-message"),
    confirmLabel: I18n.t("shell-restart-confirm-action"),
    cancelLabel: I18n.t("common-action-cancel"),
    variant: "warning",
  });

  if (!confirmed) return;

  try {
    // ONE notice for the whole restart: it is replaced in place if the restart
    // fails, and the page reloads out from under it when it succeeds.
    Utils.showToast({
      key: "system-restart",
      type: "progress",
      title: I18n.t("shell-restart-progress"),
    });

    const res = await fetch("/api/system/reboot", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });

    if (!res.ok) {
      throw new Error(I18n.t("shell-restart-failed-status", { status: String(res.status) }));
    }

    const result = await res.json();

    const waitForRestart = window.waitForScreenerBotRestart;
    if (typeof waitForRestart !== "function") {
      throw new Error(I18n.t("shell-restart-helper-unavailable"));
    }
    await waitForRestart(result.instance_id, { target: window.location.pathname || "/home" });
  } catch (err) {
    console.error("[Header] Restart failed:", err);
    Utils.showToast({
      key: "system-restart",
      type: "error",
      title: I18n.t("shell-restart-failed"),
      message: err.message,
    });
  }
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initTraderControls);
} else {
  initTraderControls();
}
