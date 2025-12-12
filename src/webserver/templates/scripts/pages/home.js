/* global */
import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { requestManager, createScopedFetcher } from "../core/request_manager.js";

function createLifecycle() {
  let poller = null;
  let scopedFetch = null;
  let currentPeriod = "today";
  let cachedData = null;
  let hasLoadedOnce = false;

  // Event cleanup tracking
  const eventCleanups = [];
  // Animation intervals tracking
  const animationIntervals = [];

  /**
   * Add tracked event listener for cleanup
   */
  function addTrackedListener(element, event, handler) {
    if (!element) return;
    element.addEventListener(event, handler);
    eventCleanups.push(() => element.removeEventListener(event, handler));
  }

  /**
   * Set loading state on dashboard sections
   */
  function setLoadingState(isLoading) {
    const walletHero = document.querySelector(".wallet-hero");
    const dashboardCards = document.querySelectorAll(".dashboard-card");

    if (isLoading && !hasLoadedOnce) {
      // Only show loading state on first load
      walletHero?.classList.add("loading");
      dashboardCards.forEach((card) => card.classList.add("loading"));
    } else {
      // Remove loading state and add loaded animation
      walletHero?.classList.remove("loading");
      walletHero?.classList.add("loaded");
      dashboardCards.forEach((card) => {
        card.classList.remove("loading");
        card.classList.add("loaded");
      });
      hasLoadedOnce = true;
    }
  }

  // Fetch dashboard data
  async function fetchData() {
    const fetcher =
      typeof scopedFetch === "function"
        ? scopedFetch
        : (url, options) => requestManager.fetch(url, options);

    try {
      const data = await fetcher("/api/dashboard/home", {
        priority: "normal",
        cache: "no-store",
      });
      cachedData = data;
      updateUI(data);
      // Remove loading state after successful data fetch
      setLoadingState(false);
    } catch (error) {
      if (error?.name === "AbortError") {
        return;
      }
      console.error("Error fetching dashboard data:", error);
      // Remove loading state on error to avoid stuck loading
      setLoadingState(false);
    }
  }

  // Update all UI elements
  function updateUI(data) {
    if (!data) return;

    // Update trading analytics for current period
    updateTraderStats(data.trader[currentPeriod]);

    // Update wallet analytics
    updateWalletStats(data.wallet);

    // Update positions snapshot
    updatePositionsStats(data.positions);

    // Update system metrics
    updateSystemStats(data.system);

    // Update token statistics
    updateTokenStats(data.tokens);
  }

  // Update trading statistics
  function updateTraderStats(stats) {
    if (!stats) return;

    const buysEl = document.getElementById("statBuys");
    const sellsEl = document.getElementById("statSells");
    const profitEl = document.getElementById("statProfit");
    const lossEl = document.getElementById("statLoss");
    const netPnlEl = document.getElementById("statNetPnl");
    const winRateEl = document.getElementById("statWinRate");
    const drawdownEl = document.getElementById("statDrawdown");

    if (buysEl) animateValue(buysEl, stats.buys);
    if (sellsEl) animateValue(sellsEl, stats.sells);
    if (profitEl) {
      profitEl.textContent = Utils.formatSol(stats.profit_sol, { decimals: 4 });
      profitEl.classList.add("profit");
    }
    if (lossEl) {
      lossEl.textContent = Utils.formatSol(stats.loss_sol, { decimals: 4 });
      lossEl.classList.add("loss");
    }
    if (netPnlEl) {
      netPnlEl.textContent = Utils.formatSol(stats.net_pnl_sol, { decimals: 4 });
      netPnlEl.className = `stat-value ${stats.net_pnl_sol >= 0 ? "profit" : "loss"}`;
    }
    if (winRateEl) {
      winRateEl.textContent = `${Utils.formatNumber(stats.win_rate, 2)}%`;
    }
    if (drawdownEl) {
      drawdownEl.textContent = `${Utils.formatNumber(stats.drawdown_percent, 2)}%`;
    }
  }

  // Update wallet statistics
  function updateWalletStats(wallet) {
    if (!wallet) return;

    const balanceEl = document.getElementById("walletBalance");
    const changeEl = document.getElementById("homeWalletChange");
    const tokensEl = document.getElementById("walletTokens");
    const tokensWorthEl = document.getElementById("walletTokensWorth");
    const startDayEl = document.getElementById("walletStartDay");

    if (balanceEl) {
      balanceEl.textContent = Utils.formatSol(wallet.current_balance_sol, {
        decimals: 4,
      });
    }

    if (changeEl) {
      const changeSign = wallet.change_sol >= 0 ? "+" : "";
      const changeClass = wallet.change_sol >= 0 ? "profit" : "loss";
      changeEl.innerHTML = `
        <span class="hero-change-value change-value ${changeClass}">${changeSign}${Utils.formatSol(
          wallet.change_sol,
          { decimals: 4 }
        )}</span>
        <span class="change-percent ${changeClass}">(${changeSign}${Utils.formatNumber(
          wallet.change_percent,
          2
        )}%)</span>
      `;
    }

    if (tokensEl) animateValue(tokensEl, wallet.token_count);
    if (tokensWorthEl)
      tokensWorthEl.textContent = Utils.formatSol(wallet.tokens_worth_sol, {
        decimals: 4,
      });
    if (startDayEl)
      startDayEl.textContent = Utils.formatSol(wallet.start_of_day_balance_sol, {
        decimals: 4,
      });
  }

  // Update positions statistics
  function updatePositionsStats(positions) {
    if (!positions) return;

    const countEl = document.getElementById("positionsCount");
    const investedEl = document.getElementById("positionsInvested");
    const unrealizedPnlEl = document.getElementById("positionsUnrealizedPnl");
    const unrealizedPercentEl = document.getElementById("positionsUnrealizedPercent");
    const avgSizeEl = document.getElementById("positionsAvgSize");
    const avgHoldEl = document.getElementById("positionsAvgHold");
    const bestEl = document.getElementById("positionsBest");
    const worstEl = document.getElementById("positionsWorst");

    if (countEl) animateValue(countEl, positions.open_count);
    if (investedEl)
      investedEl.textContent = Utils.formatSol(positions.total_invested_sol, {
        decimals: 4,
      });
    if (unrealizedPnlEl) {
      unrealizedPnlEl.textContent = Utils.formatSol(positions.unrealized_pnl_sol, {
        decimals: 4,
      });
      unrealizedPnlEl.className = `position-value ${
        positions.unrealized_pnl_sol >= 0 ? "profit" : "loss"
      }`;
    }
    if (unrealizedPercentEl) {
      unrealizedPercentEl.textContent = `${Utils.formatNumber(
        positions.unrealized_pnl_percent,
        2
      )}%`;
      unrealizedPercentEl.className = `position-value ${
        positions.unrealized_pnl_percent >= 0 ? "profit" : "loss"
      }`;
    }
    if (avgSizeEl)
      avgSizeEl.textContent = Utils.formatSol(positions.avg_position_size_sol, {
        decimals: 4,
      });
    if (avgHoldEl) {
      const mins = positions.avg_hold_duration_mins || 0;
      if (mins >= 60) {
        const hours = Math.floor(mins / 60);
        const remainingMins = mins % 60;
        avgHoldEl.textContent = remainingMins > 0 ? `${hours}h ${remainingMins}m` : `${hours}h`;
      } else {
        avgHoldEl.textContent = `${mins}m`;
      }
    }
    if (bestEl) {
      if (positions.best_performer) {
        bestEl.textContent = `${positions.best_performer.symbol} +${Utils.formatNumber(
          positions.best_performer.pnl_percent,
          1
        )}%`;
        bestEl.className = "position-value profit";
      } else {
        bestEl.textContent = "—";
        bestEl.className = "position-value";
      }
    }
    if (worstEl) {
      if (positions.worst_performer) {
        worstEl.textContent = `${positions.worst_performer.symbol} ${Utils.formatNumber(
          positions.worst_performer.pnl_percent,
          1
        )}%`;
        worstEl.className = "position-value loss";
      } else {
        worstEl.textContent = "—";
        worstEl.className = "position-value";
      }
    }
  }

  // Update system statistics
  function updateSystemStats(system) {
    if (!system) return;

    const uptimeEl = document.getElementById("systemUptime");
    const memoryEl = document.getElementById("systemMemory");
    const cpuEl = document.getElementById("systemCpu");
    const rpcRateEl = document.getElementById("systemRpcRate");
    const rpcSuccessEl = document.getElementById("systemRpcSuccess");
    const servicesEl = document.getElementById("systemServices");

    if (uptimeEl) uptimeEl.textContent = system.uptime_formatted;
    if (memoryEl)
      memoryEl.textContent = `${Utils.formatNumber(system.memory_mb, 0)} MB (${Utils.formatNumber(
        system.memory_percent,
        1
      )}%)`;
    if (cpuEl) cpuEl.textContent = `${Utils.formatNumber(system.cpu_percent, 1)}%`;
    if (rpcRateEl) rpcRateEl.textContent = Utils.formatNumber(system.rpc_calls_per_min, 1);
    if (rpcSuccessEl)
      rpcSuccessEl.textContent = `${Utils.formatNumber(system.rpc_success_rate, 1)}%`;
    if (servicesEl) servicesEl.textContent = `${system.services_healthy}/${system.services_total}`;
  }

  // Update token statistics
  function updateTokenStats(tokens) {
    if (!tokens) return;

    const totalEl = document.getElementById("tokensTotal");
    const withPricesEl = document.getElementById("tokensWithPrices");
    const passedEl = document.getElementById("tokensPassed");
    const rejectedEl = document.getElementById("tokensRejected");
    const blacklistedEl = document.getElementById("tokensBlacklisted");
    const ohlcvEl = document.getElementById("tokensOhlcv");

    if (totalEl) animateValue(totalEl, tokens.total_in_database);
    if (withPricesEl) animateValue(withPricesEl, tokens.with_prices);
    if (passedEl) animateValue(passedEl, tokens.passed_filters);
    if (rejectedEl) animateValue(rejectedEl, tokens.rejected_filters);
    if (blacklistedEl) animateValue(blacklistedEl, tokens.blacklisted);
    if (ohlcvEl) animateValue(ohlcvEl, tokens.with_ohlcv);
  }

  // Animate number value changes
  function animateValue(element, targetValue) {
    if (!element) return;

    const currentValue = parseInt(element.textContent) || 0;
    if (currentValue === targetValue) return;

    const duration = 500;
    const steps = 20;
    const stepValue = (targetValue - currentValue) / steps;
    const stepDuration = duration / steps;

    let current = currentValue;
    let step = 0;

    const interval = setInterval(() => {
      step++;
      current += stepValue;

      if (step >= steps) {
        element.textContent = targetValue;
        clearInterval(interval);
        const idx = animationIntervals.indexOf(interval);
        if (idx !== -1) animationIntervals.splice(idx, 1);
      } else {
        element.textContent = Math.round(current);
      }
    }, stepDuration);

    animationIntervals.push(interval);
  }

  // Handle period tab clicks
  function setupPeriodTabs() {
    const tabs = document.querySelectorAll(".period-tab");
    tabs.forEach((tab) => {
      addTrackedListener(tab, "click", () => {
        tabs.forEach((t) => t.classList.remove("active"));
        tab.classList.add("active");
        currentPeriod = tab.dataset.period;

        if (cachedData && cachedData.trader) {
          updateTraderStats(cachedData.trader[currentPeriod]);
        }
      });
    });
  }

  return {
    init: (ctx) => {
      console.log("[Home] Initializing dashboard");
      scopedFetch = createScopedFetcher(ctx, { latestOnly: true });

      // Note: Loading state is already applied via HTML classes
      // Data fetch happens in activate() to avoid double call

      setupPeriodTabs();
    },

    activate: (ctx) => {
      console.log("[Home] Activating dashboard");

      if (!scopedFetch) {
        scopedFetch = createScopedFetcher(ctx, { latestOnly: true });
      }

      // If we have cached data from a previous visit, show it immediately
      // This provides instant feedback while fresh data loads
      if (cachedData) {
        updateUI(cachedData);
        setLoadingState(false);
      }

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => fetchData(), {
            label: "HomeDashboard",
            getInterval: () => 5000,
          })
        );
      }

      poller.start({ silent: true });
      fetchData();
    },

    deactivate: () => {
      console.log("[Home] Deactivating dashboard");
      if (poller) {
        poller.stop({ silent: true });
        poller = null;
      }
    },

    dispose: () => {
      console.log("[Home] Disposing dashboard");
      scopedFetch = null;
      // Note: Don't reset hasLoadedOnce or cachedData here
      // Preserving them allows instant display on page revisit

      // Remove loaded class so HTML loading state works on next init
      const walletHero = document.querySelector(".wallet-hero");
      const dashboardCards = document.querySelectorAll(".dashboard-card");
      walletHero?.classList.remove("loaded");
      dashboardCards.forEach((card) => card.classList.remove("loaded"));

      // Clean up all tracked event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

      // Clear all animation intervals
      animationIntervals.forEach((interval) => clearInterval(interval));
      animationIntervals.length = 0;
    },
  };
}

registerPage("home", createLifecycle());
