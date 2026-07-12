// Live metrics and effective Auto Trader state for the global dashboard header.
import { Poller } from "./poller.js";
import { requestManager } from "./request_manager.js";
import { formatNumber } from "./utils.js";

const METRICS_POLL_INTERVAL = 5000;

// Wallet SOL figures render with the same precision as the home hero — a headline that
// reads 1.234 in one place and 1.2345 in the other looks like two different numbers.
const WALLET_SOL_DECIMALS = 4;

const TRADER_STATES = {
  preview: {
    label: "PREVIEW",
    control: "Auto Trader unavailable in preview mode. Open wallet and RPC setup.",
  },
  force_stopped: {
    label: "HALTED",
    control: "Emergency stop is active. Open Auto Trader controls.",
  },
  stopped: {
    label: "OFF",
    control: "Auto Trader is off. Click to enable it.",
  },
  waiting: {
    label: "WAITING",
    control: "Auto Trader is enabled and waiting for core services. Click to disable it.",
  },
  idle: {
    label: "IDLE",
    control: "Auto Trader is enabled, but both monitors are off. Open Auto Trader controls.",
  },
  entry_paused: {
    label: "ENTRY PAUSED",
    control: "Loss protection paused entries; exits can continue. Open Auto Trader controls.",
  },
  running: {
    label: "RUNNING",
    control: "Auto Trader is running. Click to disable it.",
  },
};

function finiteNumber(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : Number.NaN;
}

function setValueClass(element, value) {
  element.classList.remove("positive", "negative", "neutral");
  element.classList.add(value > 0 ? "positive" : value < 0 ? "negative" : "neutral");
}

function updateBotCard(trader, state) {
  const card = document.getElementById("botCard");
  const status = document.getElementById("botStatus");
  const pnl = document.getElementById("botPnL");
  if (!card || !status || !pnl || !trader) return;

  const statusKey = TRADER_STATES[trader.state] ? trader.state : "waiting";
  const statusConfig = TRADER_STATES[statusKey];
  state.traderEnabled = Boolean(trader.enabled);
  state.traderStatus = statusKey;
  state.available = true;

  card.dataset.status = statusKey;
  card.setAttribute("aria-pressed", state.traderEnabled ? "true" : "false");
  card.setAttribute("aria-label", statusConfig.control);
  card.title = statusConfig.control;
  status.textContent = statusConfig.label;

  if (statusKey === "preview") {
    pnl.textContent = "—";
    pnl.classList.remove("positive", "negative", "neutral");
    return;
  }

  const value = finiteNumber(trader.today_pnl_sol);
  if (!Number.isFinite(value)) {
    pnl.textContent = "—";
    pnl.classList.remove("positive", "negative", "neutral");
    return;
  }

  const sign = value > 0 ? "+" : value < 0 ? "−" : "";
  pnl.innerHTML = `<span class="pnl-num">${sign}${Math.abs(value).toFixed(3)}</span><span class="pnl-unit"> SOL</span>`;
  setValueClass(pnl, value);
}

// The card headlines the wallet's full WORTH (cash + every token held), which is the
// identical figure — and identical formatting — the home hero renders. Both read
// `total_equity_sol` off the backend's one wallet-worth source, so they cannot drift.
// The bottom row breaks the headline down into its cash part and the token count.
function updateWalletCard(wallet, state) {
  const card = document.getElementById("walletCard");
  const worth = document.getElementById("walletWorth");
  const sol = document.getElementById("walletSol");
  const change = document.getElementById("walletChange");
  const tokenCount = document.getElementById("walletTokenCount");
  if (!card || !worth) return;

  if (state.traderStatus === "preview" || !wallet) {
    worth.textContent = "—";
    if (sol) sol.textContent = "—";
    if (change) {
      change.textContent = "—";
      change.classList.remove("positive", "negative", "neutral");
    }
    if (tokenCount) tokenCount.textContent = "—";
    return;
  }

  const equity = finiteNumber(wallet.total_equity_sol);
  worth.textContent = formatNumber(equity, WALLET_SOL_DECIMALS);

  const balance = finiteNumber(wallet.sol_balance);
  if (sol) sol.textContent = formatNumber(balance, WALLET_SOL_DECIMALS);

  const changePercent = finiteNumber(wallet.change_today_percent);
  if (change) {
    if (Number.isFinite(changePercent)) {
      const direction = changePercent > 0 ? "↑" : changePercent < 0 ? "↓" : "";
      change.textContent = `${direction}${Math.abs(changePercent).toFixed(1)}%`;
      setValueClass(change, changePercent);
    } else {
      change.textContent = "—";
      change.classList.remove("positive", "negative", "neutral");
    }
  }

  if (tokenCount) tokenCount.textContent = formatNumber(wallet.token_count, 0);
  card.setAttribute(
    "aria-label",
    `Wallet worth: ${formatNumber(equity, WALLET_SOL_DECIMALS)} SOL (${formatNumber(balance, WALLET_SOL_DECIMALS)} SOL cash, ${formatNumber(wallet.token_count, 0)} tokens); open Positions`
  );
}

function updateSolPriceCard(sol) {
  const value = document.getElementById("solPriceValue");
  const change = document.getElementById("solPriceChange");
  if (!value || !change) return;

  const price = finiteNumber(sol?.price_usd);
  value.textContent =
    Number.isFinite(price) && price > 0
      ? `$${price.toLocaleString(undefined, {
          minimumFractionDigits: 2,
          maximumFractionDigits: 2,
        })}`
      : "—";

  const percent = finiteNumber(sol?.change_24h_percent);
  if (Number.isFinite(percent)) {
    change.textContent = `${percent > 0 ? "+" : ""}${percent.toFixed(2)}%`;
    setValueClass(change, percent);
  } else {
    change.textContent = "—";
    change.classList.remove("positive", "negative", "neutral");
  }
}

function updateTicker(metrics) {
  const monitoringCount = document.getElementById("tickerMonitoringCount");
  const passedCount = document.getElementById("tickerPassedCount");
  const rejectedCount = document.getElementById("tickerRejectedCount");
  const todayPnl = document.getElementById("tickerTodayPnL");
  const rpcCalls = document.getElementById("tickerRPCCalls");
  const rpcSuccess = document.getElementById("tickerRPCSuccess");
  const servicesText = document.getElementById("tickerServicesText");

  if (monitoringCount)
    monitoringCount.textContent = formatNumber(metrics.filtering?.monitoring_count, 0);
  if (passedCount) passedCount.textContent = formatNumber(metrics.filtering?.passed_count, 0);
  if (rejectedCount) rejectedCount.textContent = formatNumber(metrics.filtering?.rejected_count, 0);

  if (todayPnl) {
    const pnl = finiteNumber(metrics.trader?.today_pnl_sol);
    const percent = finiteNumber(metrics.trader?.today_pnl_percent);
    if (Number.isFinite(pnl) && Number.isFinite(percent)) {
      const sign = pnl > 0 ? "+" : pnl < 0 ? "−" : "";
      todayPnl.textContent = `${sign}${Math.abs(pnl).toFixed(3)} SOL (${percent > 0 ? "+" : ""}${percent.toFixed(1)}%)`;
      setValueClass(todayPnl, pnl);
    } else {
      todayPnl.textContent = "—";
      todayPnl.classList.remove("positive", "negative", "neutral");
    }
  }

  if (rpcCalls) rpcCalls.textContent = formatNumber(metrics.rpc?.calls_per_minute, 1);
  if (rpcSuccess) rpcSuccess.textContent = formatNumber(metrics.rpc?.success_rate_percent, 0);

  if (servicesText && metrics.system) {
    if (metrics.system.all_services_healthy) {
      servicesText.innerHTML = '<span class="status-dot"></span>Services: <strong>Healthy</strong>';
    } else {
      const count = metrics.system.unhealthy_services?.length ?? 0;
      const dotClass = metrics.system.critical_degraded ? "error" : "warning";
      servicesText.innerHTML = `<span class="status-dot ${dotClass}"></span>Services: <strong>${count} Issues</strong>`;
    }
  }
}

export function createHeaderMetrics({ state, setAvailability }) {
  let metricsPoller = null;
  let requestInFlight = null;
  let visibilityHandlerAdded = false;

  const syncBotControlState = () => {
    const card = document.getElementById("botCard");
    if (!card) return;
    const unavailable = !state.available || state.bootstrapping;
    card.disabled = unavailable || state.loading;
    card.setAttribute("aria-busy", state.loading || state.bootstrapping ? "true" : "false");
  };

  const fetchHeaderMetrics = () => {
    if (requestInFlight) return requestInFlight;

    requestInFlight = requestManager
      .fetch("/api/header/metrics", {
        method: "GET",
        headers: { "X-Requested-With": "fetch" },
        cache: "no-store",
        priority: "high",
      })
      .then((metrics) => {
        if (!metrics) throw new Error("Header metrics response was empty");
        updateBotCard(metrics.trader, state);
        updateWalletCard(metrics.wallet, state);
        updateSolPriceCard(metrics.sol);
        updateTicker(metrics);
        setAvailability(true);
        syncBotControlState();
        return metrics;
      })
      .catch((error) => {
        setAvailability(false);
        syncBotControlState();
        if (error?.name !== "AbortError" && error?.name !== "TimeoutError") {
          console.error("[Header] Failed to fetch metrics:", error);
        }
        throw error;
      })
      .finally(() => {
        requestInFlight = null;
      });

    return requestInFlight;
  };

  const startMetricsPolling = () => {
    metricsPoller?.cleanup();
    metricsPoller = new Poller(fetchHeaderMetrics, {
      label: "HeaderMetrics",
      getInterval: () => METRICS_POLL_INTERVAL,
      pauseWhenHidden: true,
    });
    metricsPoller.start({ silent: true });

    if (!visibilityHandlerAdded) {
      document.addEventListener("visibilitychange", () => {
        if (!metricsPoller?.isActive()) return;
        if (document.hidden) {
          metricsPoller.pause();
        } else {
          metricsPoller.resume();
          fetchHeaderMetrics().catch(() => {});
        }
      });
      visibilityHandlerAdded = true;
    }
  };

  return { fetchHeaderMetrics, startMetricsPolling, syncBotControlState };
}
