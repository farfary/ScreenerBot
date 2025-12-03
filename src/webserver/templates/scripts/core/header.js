// Header controls for global dashboard interactions (trader toggle + metrics)
import { loadPage } from "./router.js";
import { Poller, getInterval as getGlobalPollingInterval } from "./poller.js";
import * as Utils from "./utils.js";
import { Dropdown } from "../ui/dropdown.js";
import { notificationManager } from "./notifications.js";
import * as NotificationPanel from "../ui/notification_panel.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { requestManager } from "./request_manager.js";
import { subscribeToBootstrap, waitForReady } from "./bootstrap.js";
import { showSettingsDialog } from "../ui/settings_dialog.js";

const MIN_STATUS_POLL_INTERVAL = 5000;
const METRICS_POLL_INTERVAL = 5000; // Header metrics update every 5s

const state = {
  enabled: false,
  running: false,
  available: false,
  loading: false,
  fetching: false,
  connected: false,
  bootstrapping: true,
  uiReady: false,
  coreReady: false,
  bootstrapStatus: null,
};

let statusPoller = null;
let metricsPoller = null;
let currentStatusPromise = null;
let currentController = null;
let powerDropdown = null;
let traderDropdown = null;
let bootstrapUnsubscribe = null;

function getElements() {
  return {
    toggle: document.getElementById("traderToggle"),
    icon: document.getElementById("traderIcon"),
    text: document.getElementById("traderText"),
    badge: document.getElementById("botBadge"),
    badgeIcon: document.getElementById("botStatusIcon"),
    badgeText: document.getElementById("botStatusText"),
    connectionStatus: document.getElementById("connectionStatus"),
    connectionIcon: document.getElementById("connectionIcon"),
    notificationBadge: document.getElementById("notificationBadge"),
  };
}

function updateBadge({ badge, badgeIcon, badgeText }) {
  if (!badge || !badgeIcon || !badgeText) {
    return;
  }

  badge.classList.remove("loading", "success", "warning", "error");

  if (state.loading) {
    badge.classList.add("loading");
    badgeIcon.className = "icon-bot";
    badgeText.textContent = "LOADING";
    return;
  }

  if (!state.available) {
    badge.classList.add("warning");
    badgeIcon.className = "icon-triangle-alert";
    badgeText.textContent = "UNKNOWN";
    return;
  }

  if (state.running) {
    badge.classList.add("success");
    badgeIcon.className = "icon-circle-check";
    badgeText.textContent = "RUNNING";
  } else {
    badge.classList.add("warning");
    badgeIcon.className = "icon-circle-pause";
    badgeText.textContent = "STOPPED";
  }
}

function updateToggle({ toggle, icon, text }) {
  if (!toggle || !icon || !text) {
    return;
  }

  toggle.disabled = state.loading || !state.available;
  toggle.setAttribute("aria-busy", state.loading ? "true" : "false");
  toggle.dataset.traderState = state.running ? "running" : "stopped";

  // Update button classes for styling
  toggle.classList.remove("running", "stopped");
  if (state.running) {
    toggle.classList.add("running");
  } else {
    toggle.classList.add("stopped");
  }

  if (state.loading) {
    icon.innerHTML = '<i class="icon-loader"></i>';
    text.textContent = "Updating...";
    return;
  }

  if (!state.available) {
    icon.className = "icon-triangle-alert";
    text.textContent = "Status unavailable";
    return;
  }

  if (state.running) {
    icon.className = "icon-pause";
    text.textContent = "Stop Trader";
  } else {
    icon.className = "icon-play";
    text.textContent = "Start Trader";
  }
}

function applyStatus(newStatus) {
  if (!newStatus || typeof newStatus !== "object") {
    return;
  }
  if (typeof newStatus.enabled === "boolean") {
    state.enabled = newStatus.enabled;
  }
  if (typeof newStatus.running === "boolean") {
    state.running = newStatus.running;
  }
  state.available = true;
  const elements = getElements();
  updateToggle(elements);
  updateBadge(elements);
  updateConnectionStatus(true);
}

function setAvailability(isAvailable) {
  state.available = isAvailable;
  const elements = getElements();
  updateToggle(elements);
  updateBadge(elements);
  updateConnectionStatus(isAvailable);
}

function updateConnectionStatus(isConnected) {
  const elements = getElements();
  if (!elements.connectionStatus || !elements.connectionIcon) {
    return;
  }

  state.connected = isConnected;

  elements.connectionStatus.classList.remove("connected", "disconnected", "connecting");

  if (isConnected) {
    elements.connectionStatus.classList.add("connected");
    elements.connectionIcon.className = "icon-circle-check";
    elements.connectionStatus.title = "Backend Connected";
  } else {
    elements.connectionStatus.classList.add("disconnected");
    elements.connectionIcon.className = "icon-circle-x";
    elements.connectionStatus.title = "Backend Disconnected";
  }
}

function setLoading(isLoading) {
  state.loading = Boolean(isLoading);
  const elements = getElements();
  updateToggle(elements);
  updateBadge(elements);
}

function applyBootstrapStatus(status) {
  state.bootstrapStatus = status;
  const initializationRequired = Boolean(status?.initialization_required);
  const uiReady = Boolean(status && (status.ui_ready || initializationRequired));
  const coreReady = Boolean(status?.ready_for_requests);

  state.uiReady = uiReady;
  state.coreReady = coreReady;
  state.bootstrapping = !uiReady;

  if (state.bootstrapping) {
    state.available = false;
    state.loading = true;
  }

  const elements = getElements();

  if (!uiReady) {
    if (elements.badgeText) {
      const label = status?.message?.toUpperCase() || "BOOTING";
      elements.badgeText.textContent = label;
    }
    if (elements.badgeIcon) {
      elements.badgeIcon.className = "icon-loader";
    }
    if (elements.badge) {
      elements.badge.classList.add("loading");
    }
    updateBadge(elements);
    updateConnectionStatus(false);
    return;
  }

  state.loading = false;
  state.available = true;
  updateBadge(elements);
  updateConnectionStatus(true);
}

// ============================================================================
// HEADER METRICS (New Advanced Header Design)
// ============================================================================

async function fetchHeaderMetrics() {
  try {
    const data = await requestManager.fetch("/api/header/metrics", {
      method: "GET",
      headers: { "X-Requested-With": "fetch" },
      cache: "no-store",
      priority: "high",
      skipQueue: true,
      skipDedup: true,
    });

    updateHeaderMetrics(data);
    return data;
  } catch (err) {
    if (err?.name !== "AbortError" && err?.name !== "TimeoutError") {
      console.error("[Header] Failed to fetch metrics:", err);
    }
    return null;
  }
}

function updateHeaderMetrics(metrics) {
  if (!metrics) return;

  // Update Bot Card
  updateBotCard(metrics.trader);

  // Update Wallet Card
  updateWalletCard(metrics.wallet);

  // Update Positions Card
  updatePositionsCard(metrics.positions, metrics.rpc);

  // Update Ticker
  updateTicker(metrics);
}

function updateBotCard(trader) {
  const card = document.getElementById("botCard");
  const icon = document.getElementById("botIcon");
  const status = document.getElementById("botStatus");
  const pnl = document.getElementById("botPnL");

  if (!card || !status || !pnl) return;

  // Update status
  const statusText = trader.running ? "RUNNING" : "STOPPED";
  const statusAttr = trader.running ? "running" : "stopped";

  card.setAttribute("data-status", statusAttr);
  status.textContent = statusText;

  // Update P&L
  const pnlText =
    trader.today_pnl_sol >= 0
      ? `+${trader.today_pnl_sol.toFixed(3)} SOL`
      : `${trader.today_pnl_sol.toFixed(3)} SOL`;

  pnl.textContent = pnlText;
  // Use classList to avoid className replacement flash
  pnl.classList.remove("positive", "negative");
  pnl.classList.add(trader.today_pnl_sol >= 0 ? "positive" : "negative");
}

function updateWalletCard(wallet) {
  const sol = document.getElementById("walletSol");
  const change = document.getElementById("walletChange");
  const tokenCount = document.getElementById("walletTokenCount");
  const tokenWorth = document.getElementById("walletTokenWorth");

  if (!sol) return;

  // Update SOL balance
  sol.textContent = wallet.sol_balance.toFixed(3);

  // Update 24h change
  if (change) {
    const changeText =
      wallet.change_24h_percent >= 0
        ? `↑${Math.abs(wallet.change_24h_percent).toFixed(1)}%`
        : `↓${Math.abs(wallet.change_24h_percent).toFixed(1)}%`;

    change.textContent = changeText;
    // Use classList to avoid className replacement flash
    change.classList.remove("positive", "negative");
    change.classList.add(wallet.change_24h_percent >= 0 ? "positive" : "negative");
  }

  // Update token count
  if (tokenCount) {
    tokenCount.textContent = wallet.token_count.toString();
  }

  // Update token worth
  if (tokenWorth) {
    tokenWorth.textContent = `${wallet.tokens_worth_sol.toFixed(2)} SOL`;
  }
}

function updatePositionsCard(positions, rpc) {
  const count = document.getElementById("positionsCount");
  const pnl = document.getElementById("positionsPnL");
  const rpcSuccess = document.getElementById("rpcSuccess");
  const rpcLatency = document.getElementById("rpcLatency");

  if (!count) return;

  // Update positions count
  count.textContent = positions.open_count.toString();

  // Update unrealized P&L
  if (pnl) {
    const pnlText =
      positions.unrealized_pnl_sol >= 0
        ? `+${positions.unrealized_pnl_sol.toFixed(3)}`
        : `${positions.unrealized_pnl_sol.toFixed(3)}`;

    pnl.textContent = pnlText;
    pnl.className = `card-change ${positions.unrealized_pnl_sol >= 0 ? "positive" : "negative"}`;
  }

  // Update RPC stats
  if (rpcSuccess) {
    rpcSuccess.textContent = `${rpc.success_rate_percent.toFixed(0)}%`;
  }

  if (rpcLatency) {
    rpcLatency.textContent = `${rpc.avg_latency_ms}ms`;
  }
}

function updateTicker(metrics) {
  // Update monitoring count
  const monitoringCount = document.getElementById("tickerMonitoringCount");
  if (monitoringCount) {
    monitoringCount.textContent = metrics.filtering.monitoring_count.toString();
  }

  // Update passed/rejected counts
  const passedCount = document.getElementById("tickerPassedCount");
  const rejectedCount = document.getElementById("tickerRejectedCount");
  if (passedCount) {
    passedCount.textContent = metrics.filtering.passed_count.toString();
  }
  if (rejectedCount) {
    rejectedCount.textContent = metrics.filtering.rejected_count.toString();
  }

  // Update today P&L
  const todayPnL = document.getElementById("tickerTodayPnL");
  if (todayPnL) {
    const pnlText =
      metrics.trader.today_pnl_sol >= 0
        ? `+${metrics.trader.today_pnl_sol.toFixed(3)} SOL (↑${metrics.trader.today_pnl_percent.toFixed(1)}%)`
        : `${metrics.trader.today_pnl_sol.toFixed(3)} SOL (↓${Math.abs(metrics.trader.today_pnl_percent).toFixed(1)}%)`;

    todayPnL.textContent = pnlText;
    todayPnL.style.color = metrics.trader.today_pnl_sol >= 0 ? "#10b981" : "#ef4444";
  }

  // Update RPC calls
  const rpcCalls = document.getElementById("tickerRPCCalls");
  const rpcSuccess = document.getElementById("tickerRPCSuccess");
  if (rpcCalls) {
    rpcCalls.textContent = metrics.rpc.calls_per_minute.toFixed(1);
  }
  if (rpcSuccess) {
    rpcSuccess.textContent = metrics.rpc.success_rate_percent.toFixed(0);
  }

  // Update services status
  const servicesText = document.getElementById("tickerServicesText");
  if (servicesText) {
    if (metrics.system.all_services_healthy) {
      servicesText.innerHTML = 'Services: All Healthy <i class="icon-circle-check"></i>';
      servicesText.style.color = "#10b981";
    } else {
      const unhealthyCount = metrics.system.unhealthy_services.length;
      servicesText.innerHTML = `Services: ${unhealthyCount} Issues <i class="icon-triangle-alert"></i>`;
      servicesText.style.color = metrics.system.critical_degraded ? "#ef4444" : "#fbbf24";
    }
  }
}

function startMetricsPolling() {
  if (metricsPoller) {
    metricsPoller.cleanup();
  }

  metricsPoller = new Poller(() => fetchHeaderMetrics(), {
    label: "HeaderMetrics",
    interval: METRICS_POLL_INTERVAL,
    pauseWhenHidden: true, // Pause when tab hidden
  });

  metricsPoller.start({ silent: true });

  // Add visibility change handler
  setupVisibilityHandler();
}

// ============================================================================
// END HEADER METRICS
// ============================================================================

async function fetchTraderStatus({ silent = false, showLoading = false } = {}) {
  if (state.bootstrapping) {
    return null;
  }

  if (state.fetching && currentStatusPromise) {
    return currentStatusPromise;
  }

  const elements = getElements();

  if (!elements.toggle) {
    return null;
  }

  if (showLoading) {
    setLoading(true);
  }

  state.fetching = true;
  const controller = new AbortController();
  if (currentController) {
    currentController.abort();
  }
  currentController = controller;

  const request = fetch("/api/trader/status", {
    method: "GET",
    headers: { "X-Requested-With": "fetch" },
    cache: "no-store",
    signal: controller.signal,
  })
    .then(async (res) => {
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const data = await res.json();
      applyStatus(data);
      return data;
    })
    .catch((err) => {
      if (err?.name === "AbortError") {
        return null;
      }
      console.error("[TraderHeader] Failed to fetch status", err);
      setAvailability(false);
      if (!silent) {
        Utils.showToast(
          '<i class="icon-triangle-alert"></i> Failed to refresh trader status',
          "warning"
        );
      }
      return null;
    })
    .finally(() => {
      if (currentController === controller) {
        currentController = null;
      }
      state.fetching = false;
      if (showLoading) {
        setLoading(false);
      }
    });

  currentStatusPromise = request;
  return request;
}

async function controlTrader(action) {
  const elements = getElements();
  if (!elements.toggle) {
    return;
  }

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
      const message = payload?.message || `Trader request failed (${res.status})`;
      throw new Error(message);
    }

    if (payload?.status) {
      applyStatus(payload.status);
    }

    const icon = action === "start" ? '<i class="icon-check"></i>' : '<i class="icon-check"></i>';
    Utils.showToast(`${icon} Trader ${action === "start" ? "started" : "stopped"}`, "success");
  } catch (err) {
    console.error("[TraderHeader] Control action failed", err);
    Utils.showToast(`<i class="icon-x"></i> ${err.message || "Trader control failed"}`, "error");
    setAvailability(false);
  } finally {
    setLoading(false);
    fetchTraderStatus({ silent: true });
  }
}

function attachToggleHandler(toggle) {
  if (!toggle) {
    return;
  }

  toggle.addEventListener("click", (event) => {
    event.preventDefault();
    if (!state.available || state.loading) {
      return;
    }

    const action = state.running ? "stop" : "start";
    controlTrader(action);
  });
}

function startStatusPolling() {
  if (statusPoller) {
    statusPoller.cleanup();
  }

  const pollIntervalProvider = () => {
    try {
      const interval = Number(getGlobalPollingInterval());
      if (Number.isFinite(interval) && interval > 0) {
        return Math.max(interval, MIN_STATUS_POLL_INTERVAL);
      }
    } catch (err) {
      console.warn("[TraderHeader] Failed to read polling interval", err);
    }
    return MIN_STATUS_POLL_INTERVAL;
  };

  statusPoller = new Poller(() => fetchTraderStatus({ silent: true }), {
    label: "TraderStatus",
    getInterval: pollIntervalProvider,
  });

  statusPoller.start({ silent: true });
}

function initTraderControls() {
  const elements = getElements();
  if (!elements.toggle) {
    console.warn("[Header] Toggle element not found, skipping initialization");
  } else {
    attachToggleHandler(elements.toggle);
  }

  if (!bootstrapUnsubscribe) {
    bootstrapUnsubscribe = subscribeToBootstrap(applyBootstrapStatus);
  }

  // Initialize connection status as connecting
  if (elements.connectionStatus && elements.connectionIcon) {
    elements.connectionStatus.classList.add("connecting");
    elements.connectionIcon.className = "icon-circle-dot";
    elements.connectionStatus.title = "Connecting to Backend...";
  }

  // Initialize card click handlers
  initCardHandlers();

  // Initialize settings button
  initSettingsButton();

  // Initialize power menu dropdown
  initPowerMenu();

  // Initialize trader dropdown (future)
  initTraderDropdown();

  // Initialize notifications
  initNotifications();

  // Initialize notification panel UI
  NotificationPanel.init();

  // Initialize scroll navigation for main header tabs
  initHeaderTabsScroll();

  // Setup visibility handling for pollers
  setupVisibilityHandler();

  waitForReady()
    .then(() => fetchTraderStatus({ silent: true, showLoading: true }))
    .then(() => {
      startStatusPolling();
      return fetchHeaderMetrics();
    })
    .then(() => {
      startMetricsPolling();
    })
    .catch((error) => {
      console.error("[Header] Failed to initialize after bootstrap", error);
    });
}

function initCardHandlers() {
  // Bot card - toggle trader
  const botCard = document.getElementById("botCard");
  if (botCard) {
    botCard.addEventListener("click", () => {
      if (!state.available || state.loading) return;
      const action = state.running ? "stop" : "start";
      controlTrader(action);
    });
    botCard.style.cursor = "pointer";
  }

  // Wallet card - navigate to wallet page
  const walletCard = document.getElementById("walletCard");
  if (walletCard) {
    walletCard.addEventListener("click", () => {
      window.location.hash = "#wallet";
    });
    walletCard.style.cursor = "pointer";
  }

  // Positions card - navigate to positions page
  const positionsCard = document.getElementById("positionsCard");
  if (positionsCard) {
    positionsCard.addEventListener("click", () => {
      window.location.hash = "#positions";
    });
    positionsCard.style.cursor = "pointer";
  }

  // Ticker segments - navigate to relevant pages
  const tickerMonitoring = document.getElementById("tickerMonitoring");
  if (tickerMonitoring) {
    tickerMonitoring.addEventListener("click", () => {
      window.location.hash = "#tokens";
    });
  }

  const tickerFiltering = document.getElementById("tickerFiltering");
  if (tickerFiltering) {
    tickerFiltering.addEventListener("click", () => {
      window.location.hash = "#filtering";
    });
  }

  const tickerPnL = document.getElementById("tickerPnL");
  if (tickerPnL) {
    tickerPnL.addEventListener("click", () => {
      window.location.hash = "#positions";
    });
  }

  const tickerServices = document.getElementById("tickerServices");
  if (tickerServices) {
    tickerServices.addEventListener("click", () => {
      window.location.hash = "#services";
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

function initPowerMenu() {
  const powerBtn = document.getElementById("powerMenuBtn");
  if (!powerBtn) return;

  powerDropdown = new Dropdown({
    trigger: powerBtn,
    align: "right",
    items: [
      {
        id: "restart",
        icon: "<i class='icon-refresh-cw'></i>",
        label: "Restart Bot",
      },
      {
        id: "pause-services",
        icon: "<i class='icon-pause'></i>",
        label: "Pause Services",
        badge: "Soon",
        disabled: true,
      },
      { divider: true },
      {
        id: "shutdown",
        icon: "<i class='icon-power'></i>",
        label: "Shutdown",
        danger: true,
        disabled: true,
        badge: "Soon",
      },
      { divider: true },
      {
        id: "system-info",
        icon: "<i class='icon-info'></i>",
        label: "System Info",
        disabled: true,
      },
    ],
    onSelect: handlePowerMenuAction,
  });
}

function initTraderDropdown() {
  // Placeholder for future trader dropdown menu
  // Will add: Start, Stop, Restart, View Logs options
}

// ============================================================================
// HEADER TABS SCROLL NAVIGATION
// ============================================================================

let headerTabsScrollCleanup = null;

function initHeaderTabsScroll() {
  const headerRow = document.querySelector(".header-row-2");
  if (!headerRow) return;

  // Update scroll indicators based on scroll position
  const updateScrollIndicators = () => {
    const { scrollLeft, scrollWidth, clientWidth } = headerRow;
    const canScrollLeft = scrollLeft > 1;
    const canScrollRight = scrollLeft < scrollWidth - clientWidth - 1;

    headerRow.classList.toggle("can-scroll-left", canScrollLeft);
    headerRow.classList.toggle("can-scroll-right", canScrollRight);
  };

  // Mouse wheel horizontal scroll support
  const wheelHandler = (event) => {
    // Only handle if there's horizontal overflow
    if (headerRow.scrollWidth <= headerRow.clientWidth) return;

    // Convert vertical scroll to horizontal
    if (Math.abs(event.deltaY) > Math.abs(event.deltaX)) {
      event.preventDefault();
      headerRow.scrollLeft += event.deltaY;
      updateScrollIndicators();
    }
  };

  // Track scroll position for indicators
  const scrollHandler = () => updateScrollIndicators();

  // Attach event listeners
  headerRow.addEventListener("wheel", wheelHandler, { passive: false });
  headerRow.addEventListener("scroll", scrollHandler, { passive: true });

  // Watch for resize to update indicators
  const resizeObserver = new ResizeObserver(() => {
    updateScrollIndicators();
  });
  resizeObserver.observe(headerRow);

  // Initial update
  requestAnimationFrame(updateScrollIndicators);

  // Cleanup function
  headerTabsScrollCleanup = () => {
    headerRow.removeEventListener("wheel", wheelHandler);
    headerRow.removeEventListener("scroll", scrollHandler);
    resizeObserver.disconnect();
  };
}

// END HEADER TABS SCROLL NAVIGATION

let notificationsInitialized = false;
let notificationUnsubscribe = null;

function initNotifications() {
  if (notificationsInitialized) {
    console.warn("[Header] Notifications already initialized, skipping");
    return;
  }

  const notifBtn = document.getElementById("notificationBtn");

  if (!notifBtn) return;

  // Subscribe to notification updates
  notificationUnsubscribe = notificationManager.subscribe((event) => {
    if (event.type === "summary" && event.summary) {
      updateNotificationBadge(event.summary.unread);
    }

    // Show toast for new notifications
    if (event.type === "added" && event.notification) {
      const action = event.notification;
      const actionType = formatActionType(action.action_type);
      Utils.showToast(`<i class="icon-bell"></i> ${actionType} started`, "info");
    } else if (event.type === "updated" && event.notification) {
      const action = event.notification;
      const status = notificationManager.getStatus(action);

      if (status === "completed") {
        const actionType = formatActionType(action.action_type);
        Utils.showToast(`<i class="icon-check"></i> ${actionType} completed`, "success");
      } else if (status === "failed") {
        const actionType = formatActionType(action.action_type);
        Utils.showToast(`<i class="icon-x"></i> ${actionType} failed`, "error");
      }
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

function formatActionType(actionType) {
  if (!actionType) return "Action";

  // Backend sends snake_case format via Serde JSON serialization
  // #[serde(rename_all = "snake_case")] in src/actions/types.rs line 177
  const typeMap = {
    swap_buy: "Buying",
    swap_sell: "Selling",
    position_open: "Opening Position",
    position_close: "Closing Position",
    position_dca: "DCA",
    position_partial_exit: "Partial Exit",
    manual_order: "Manual Order",
  };

  return typeMap[actionType] || actionType;
}

function updateNotificationBadge(count) {
  const badge = document.getElementById("notificationBadge");
  if (!badge) return;

  badge.textContent = count > 99 ? "99+" : count.toString();
  badge.style.display = count > 0 ? "flex" : "none";
}

async function handlePowerMenuAction(action) {
  switch (action) {
    case "restart":
      await handleRestart();
      break;
    case "pause-services":
      Utils.showToast('<i class="icon-pause"></i> Pause Services feature coming soon', "info");
      break;
    case "shutdown":
      Utils.showToast('<i class="icon-power"></i> Shutdown feature coming soon', "info");
      break;
    case "system-info":
      Utils.showToast('<i class="icon-info"></i> System Info panel coming soon', "info");
      break;
  }
}



async function handleRestart() {
  const { confirmed } = await ConfirmationDialog.show({
    title: "Restart Bot",
    message:
      "Are you sure you want to restart the bot?\n\nThis will:\n• Stop all services\n• Restart the process\n• Take ~10-15 seconds\n\nAll active operations will be interrupted.",
    confirmLabel: "Restart",
    cancelLabel: "Cancel",
    variant: "warning",
  });

  if (!confirmed) return;

  try {
    Utils.showToast('<i class="icon-refresh-cw"></i> Restarting bot...', "info");

    const res = await fetch("/api/system/reboot", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });

    if (!res.ok) {
      throw new Error(`Restart failed: ${res.status}`);
    }

    Utils.showToast('<i class="icon-check"></i> Bot restarting... Please wait.', "success");

    // Poll for reconnection using Poller
    setTimeout(() => {
      let attempts = 0;
      const reconnectPoller = new Poller(
        async () => {
          attempts++;
          try {
            const ping = await fetch("/api/trader/status", { cache: "no-store" });
            if (ping.ok) {
              reconnectPoller.stop();
              reconnectPoller.cleanup();
              Utils.showToast('<i class="icon-check"></i> Bot restarted successfully!', "success");
              window.location.reload();
            }
          } catch {
            if (attempts > 30) {
              reconnectPoller.stop();
              reconnectPoller.cleanup();
              Utils.showToast(
                '<i class="icon-triangle-alert"></i> Restart taking longer than expected',
                "warning"
              );
            }
          }
        },
        { label: "RestartReconnect", interval: 1000 }
      );
      reconnectPoller.start();
    }, 2000);
  } catch (err) {
    console.error("[Header] Restart failed:", err);
    Utils.showToast(`<i class="icon-x"></i> ${err.message}`, "error");
  }
}

function setupVisibilityHandler() {
  // Prevent duplicate listeners
  if (window.__headerVisibilityHandlerAdded) {
    return;
  }

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      // Pause header pollers when tab hidden
      if (metricsPoller && metricsPoller.isActive()) {
        metricsPoller.pause();
      }
      if (statusPoller && statusPoller.isActive()) {
        statusPoller.pause();
      }
    } else {
      // Resume header pollers when tab visible
      if (metricsPoller && metricsPoller.isActive()) {
        metricsPoller.resume();
      }
      if (statusPoller && statusPoller.isActive()) {
        statusPoller.resume();
      }
    }
  });

  window.__headerVisibilityHandlerAdded = true;
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initTraderControls);
} else {
  initTraderControls();
}
