// Live metrics and effective Auto Trader state for the global dashboard header.
import { Poller } from "./poller.js";
import { requestManager } from "./request_manager.js";
import { formatPercentValue, formatSignedSol, signedTone, withUsdSymbol } from "./format.js";
import { formatNumber, showToast } from "./utils.js";

const METRICS_POLL_INTERVAL = 5000;

// Wallet SOL figures render with the same precision as the home hero — a headline that
// reads 1.234 in one place and 1.2345 in the other looks like two different numbers.
const WALLET_SOL_DECIMALS = 4;

// Effective Auto Trader states: badge text and the card's control hint.
const TRADER_STATE_LABELS = Object.freeze({
  explore: "shell-bot-state-explore",
  force_stopped: "shell-bot-state-halted",
  stopped: "shell-bot-state-off",
  waiting: "shell-bot-state-waiting",
  idle: "shell-bot-state-idle",
  entry_paused: "shell-bot-state-entry-paused",
  running: "shell-bot-state-running",
});

const TRADER_STATE_CONTROLS = Object.freeze({
  explore: "shell-bot-control-explore",
  force_stopped: "shell-bot-control-halted",
  stopped: "shell-bot-control-off",
  waiting: "shell-bot-control-waiting",
  idle: "shell-bot-control-idle",
  entry_paused: "shell-bot-control-entry-paused",
  running: "shell-bot-control-running",
});

function finiteNumber(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : Number.NaN;
}

/** Tone class of a signed amount as shown at `decimals`; a rounded zero is neutral. */
function setValueClass(element, value, decimals) {
  element.classList.remove("positive", "negative", "neutral");
  element.classList.add(signedTone(value, decimals));
}

function updateBotCard(trader, state) {
  const card = document.getElementById("botCard");
  const status = document.getElementById("botStatus");
  const pnl = document.getElementById("botPnL");
  if (!card || !status || !pnl || !trader) return;

  const statusKey = Object.hasOwn(TRADER_STATE_LABELS, trader.state) ? trader.state : "waiting";
  const control = I18n.label(TRADER_STATE_CONTROLS, statusKey);
  state.traderEnabled = Boolean(trader.enabled);
  state.traderStatus = statusKey;
  state.available = true;

  card.dataset.status = statusKey;
  card.setAttribute("aria-pressed", state.traderEnabled ? "true" : "false");
  card.setAttribute("aria-label", control);
  card.title = control;
  status.textContent = I18n.label(TRADER_STATE_LABELS, statusKey);

  if (statusKey === "explore") {
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

  pnl.innerHTML = `<span class="pnl-num">${formatSignedSol(value, { decimals: 3, unit: false })}</span><span class="pnl-unit"> SOL</span>`;
  setValueClass(pnl, value, 3);
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

  if (state.traderStatus === "explore" || !wallet) {
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
      change.textContent = `${direction}${formatPercentValue(Math.abs(changePercent), {
        decimals: 1,
        includeSign: false,
      })}`;
      setValueClass(change, changePercent, 1);
    } else {
      change.textContent = "—";
      change.classList.remove("positive", "negative", "neutral");
    }
  }

  if (tokenCount) tokenCount.textContent = formatNumber(wallet.token_count, 0);
  card.setAttribute(
    "aria-label",
    I18n.t("shell-wallet-card-summary", {
      equity: formatNumber(equity, WALLET_SOL_DECIMALS),
      balance: formatNumber(balance, WALLET_SOL_DECIMALS),
      tokens: formatNumber(wallet.token_count, 0),
    })
  );
}

function updateSolPriceCard(sol) {
  const value = document.getElementById("solPriceValue");
  const change = document.getElementById("solPriceChange");
  if (!value || !change) return;

  const price = finiteNumber(sol?.price_usd);
  value.textContent =
    Number.isFinite(price) && price > 0
      ? withUsdSymbol(formatNumber(price, 2))
      : "—";

  const percent = finiteNumber(sol?.change_24h_percent);
  if (Number.isFinite(percent)) {
    change.textContent = formatPercentValue(percent, { decimals: 2 });
    setValueClass(change, percent, 2);
  } else {
    change.textContent = "—";
    change.classList.remove("positive", "negative", "neutral");
  }
}

function updateCopyCard(copy) {
  const card = document.getElementById("copyCard");
  const value = document.getElementById("copyCardValue");
  const sub = document.getElementById("copyCardSub");
  if (!card || !value || !sub) return;
  if (!copy?.total_tasks) {
    card.hidden = true;
    return;
  }
  card.hidden = false;
  const active = copy.live_tasks + copy.paper_tasks;
  const running = [
    copy.live_tasks ? I18n.t("shell-copy-running-live", { count: copy.live_tasks }) : "",
    copy.paper_tasks ? I18n.t("shell-copy-running-paper", { count: copy.paper_tasks }) : "",
  ]
    .filter(Boolean)
    .join(" · ");
  value.textContent = !copy.enabled
    ? I18n.t("shell-copy-value-paused")
    : running || I18n.t("shell-copy-value-idle");
  sub.textContent = I18n.t("shell-copy-sub-active", { active, total: copy.total_tasks });
}

// Copy notices carry a per-process sequence; the first poll only sets the
// baseline so a reload does not replay old notices. Paper fills and exits stay
// on the Copy Trading page and in Events; a toast is for what needs attention:
// auto-pauses, failures and live trades.
let copyNoticeSeq = null;

function announceCopyNotices(copy) {
  if (!copy) return;
  const notices = copy.notices || [];
  const newest = notices.reduce((max, notice) => Math.max(max, notice.seq), 0);
  if (copyNoticeSeq === null || newest < copyNoticeSeq) {
    copyNoticeSeq = newest;
    return;
  }
  notices
    .filter((notice) => notice.seq > copyNoticeSeq && (notice.warning || !notice.paper))
    .sort((a, b) => a.seq - b.seq)
    .slice(-3)
    .forEach((notice) =>
      showToast({
        key: `copy-notice-${notice.seq}`,
        type: notice.warning ? "warning" : "info",
        title: I18n.t("copy-notice-heading", {
          task: notice.task ?? I18n.t("copy-notice-task-unnamed", { id: String(notice.task_id) }),
          title: I18n.text(notice.title),
        }),
        message: I18n.text(notice.detail),
      })
    );
  copyNoticeSeq = newest;
}

// Status dot and the localized "Services: <state>" line, a single markup message.
function renderServicesStatus(container, dotClass, id, args) {
  const dot = container.querySelector(".status-dot");
  const line = container.querySelector("#tickerServicesLine");
  if (!dot || !line) return;
  dot.className = dotClass ? `status-dot ${dotClass}` : "status-dot";
  line.setAttribute("data-l10n-id", id);
  if (args) line.setAttribute("data-l10n-args", JSON.stringify(args));
  else line.removeAttribute("data-l10n-args");
  I18n.localizeTree(line);
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
      todayPnl.textContent = `${formatSignedSol(pnl, { decimals: 3 })} (${formatPercentValue(percent, { decimals: 1 })})`;
      setValueClass(todayPnl, pnl, 3);
    } else {
      todayPnl.textContent = "—";
      todayPnl.classList.remove("positive", "negative", "neutral");
    }
  }

  // Each value and its unit are one text run, so the ticker's flex gap never splits them.
  if (rpcCalls) {
    const calls = finiteNumber(metrics.rpc?.calls_per_minute);
    rpcCalls.textContent = Number.isFinite(calls)
      ? I18n.t("shell-ticker-rpc-rate", { amount: formatNumber(calls, 1) })
      : "—";
  }
  if (rpcSuccess) {
    rpcSuccess.textContent = formatPercentValue(metrics.rpc?.success_rate_percent, {
      decimals: 0,
      includeSign: false,
    });
  }

  if (servicesText && metrics.system) {
    if (metrics.system.all_services_healthy) {
      renderServicesStatus(servicesText, "", "shell-ticker-services-healthy");
    } else {
      const count = metrics.system.unhealthy_services?.length ?? 0;
      const dotClass = metrics.system.critical_degraded ? "error" : "warning";
      renderServicesStatus(servicesText, dotClass, "shell-ticker-services-issues", { count });
    }
  }
}

/** Dispatched by `core/action_toasts.js` when a trade completes or fails. */
const TRADE_SETTLED_EVENT = "screenerbot:trade-settled";

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
        updateCopyCard(metrics.copy);
        announceCopyNotices(metrics.copy);
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
      // l10n-ignore: poller name used in logs, never displayed
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
      // A settled trade moves the wallet; show it now instead of on the next poll.
      window.addEventListener(TRADE_SETTLED_EVENT, () => {
        fetchHeaderMetrics().catch(() => {});
      });
      visibilityHandlerAdded = true;
    }
  };

  return { fetchHeaderMetrics, startMetricsPolling, syncBotControlState };
}
