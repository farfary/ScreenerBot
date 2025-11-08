import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";

function createLifecycle() {
  let poller = null;
  let memoryChart = null;
  let cpuChart = null;
  let currentPeriod = "today";
  let cachedData = null;

  // Initialize Chart.js instances
  function initCharts() {
    const memoryCtx = document.getElementById("memoryChart");
    const cpuCtx = document.getElementById("cpuChart");

    if (memoryCtx && typeof Chart !== "undefined") {
      memoryChart = new Chart(memoryCtx, {
        type: "line",
        data: {
          labels: Array(20).fill(""),
          datasets: [
            {
              data: Array(20).fill(0),
              borderColor: "#10b981",
              backgroundColor: "rgba(16, 185, 129, 0.1)",
              borderWidth: 2,
              tension: 0.4,
              fill: true,
            },
          ],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: { legend: { display: false } },
          scales: {
            y: { display: false, min: 0, max: 100 },
            x: { display: false },
          },
          elements: { point: { radius: 0 } },
        },
      });
    }

    if (cpuCtx && typeof Chart !== "undefined") {
      cpuChart = new Chart(cpuCtx, {
        type: "line",
        data: {
          labels: Array(20).fill(""),
          datasets: [
            {
              data: Array(20).fill(0),
              borderColor: "#3b82f6",
              backgroundColor: "rgba(59, 130, 246, 0.1)",
              borderWidth: 2,
              tension: 0.4,
              fill: true,
            },
          ],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: { legend: { display: false } },
          scales: {
            y: { display: false, min: 0, max: 100 },
            x: { display: false },
          },
          elements: { point: { radius: 0 } },
        },
      });
    }
  }

  // Update charts with new data
  function updateCharts(data) {
    if (memoryChart && data.system?.memory_history) {
      memoryChart.data.datasets[0].data = data.system.memory_history;
      memoryChart.update("none");
    }

    if (cpuChart && data.system?.cpu_history) {
      cpuChart.data.datasets[0].data = data.system.cpu_history;
      cpuChart.update("none");
    }
  }

  // Fetch dashboard data
  async function fetchData() {
    try {
      const response = await fetch("/api/dashboard/home");
      if (!response.ok) {
        console.error("Failed to fetch dashboard data:", response.status);
        return;
      }
      const data = await response.json();
      cachedData = data;
      updateUI(data);
    } catch (error) {
      console.error("Error fetching dashboard data:", error);
    }
  }

  // Update all UI elements
  function updateUI(data) {
    if (!data) return;

    // Update trader analytics for current period
    updateTraderStats(data.trader[currentPeriod]);

    // Update wallet analytics
    updateWalletStats(data.wallet);

    // Update positions snapshot
    updatePositionsStats(data.positions);

    // Update system metrics
    updateSystemStats(data.system);

    // Update token statistics
    updateTokenStats(data.tokens);

    // Update license info
    updateLicenseInfo(data.license);

    // Update charts
    updateCharts(data);
  }

  // Update trader statistics
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
    const changeEl = document.getElementById("walletChange");
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
  }

  // Update system statistics
  function updateSystemStats(system) {
    if (!system) return;

    const uptimeEl = document.getElementById("systemUptime");
    const memoryEl = document.getElementById("systemMemory");
    const cpuEl = document.getElementById("systemCpu");

    if (uptimeEl) uptimeEl.textContent = system.uptime_formatted;
    if (memoryEl)
      memoryEl.textContent = `${Utils.formatNumber(system.memory_mb, 0)} MB (${Utils.formatNumber(
        system.memory_percent,
        1
      )}%)`;
    if (cpuEl) cpuEl.textContent = `${Utils.formatNumber(system.cpu_percent, 1)}%`;
  }

  // Update token statistics
  function updateTokenStats(tokens) {
    if (!tokens) return;

    const totalEl = document.getElementById("tokensTotal");
    const withPricesEl = document.getElementById("tokensWithPrices");
    const passedEl = document.getElementById("tokensPassed");
    const rejectedEl = document.getElementById("tokensRejected");
    const foundTodayEl = document.getElementById("tokensFoundToday");
    const foundWeekEl = document.getElementById("tokensFoundWeek");
    const foundMonthEl = document.getElementById("tokensFoundMonth");
    const foundAllTimeEl = document.getElementById("tokensFoundAllTime");

    if (totalEl) animateValue(totalEl, tokens.total_in_database);
    if (withPricesEl) animateValue(withPricesEl, tokens.with_prices);
    if (passedEl) animateValue(passedEl, tokens.passed_filters);
    if (rejectedEl) animateValue(rejectedEl, tokens.rejected_filters);
    if (foundTodayEl) animateValue(foundTodayEl, tokens.found_today);
    if (foundWeekEl) animateValue(foundWeekEl, tokens.found_this_week);
    if (foundMonthEl) animateValue(foundMonthEl, tokens.found_this_month);
    if (foundAllTimeEl) animateValue(foundAllTimeEl, tokens.found_all_time);
  }

  // Update license information
  function updateLicenseInfo(license) {
    if (!license) return;

    const statusEl = document.getElementById("licenseStatus");
    const tierEl = document.getElementById("licenseTier");
    const daysEl = document.getElementById("licenseDaysRemaining");
    const mintEl = document.getElementById("licenseMint");
    const cardEl = document.getElementById("licenseCard");

    if (statusEl) {
      if (license.valid) {
        statusEl.innerHTML = '<span class="license-badge valid">✅ VALID</span>';
        if (cardEl) cardEl.classList.add("valid");
      } else {
        statusEl.innerHTML = '<span class="license-badge invalid">❌ INVALID</span>';
        if (cardEl) cardEl.classList.add("invalid");
      }
    }

    if (tierEl) tierEl.textContent = license.tier || "—";
    if (daysEl) {
      if (license.days_remaining !== null && license.days_remaining !== undefined) {
        const days = license.days_remaining;
        daysEl.textContent = `${days} days`;
        if (days < 7) daysEl.classList.add("expiring-soon");
      } else {
        daysEl.textContent = "—";
      }
    }
    if (mintEl) {
      if (license.mint) {
        const shortMint = `${license.mint.slice(0, 8)}...${license.mint.slice(-8)}`;
        mintEl.textContent = shortMint;
        mintEl.title = license.mint;
      } else {
        mintEl.textContent = "—";
      }
    }
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
      } else {
        element.textContent = Math.round(current);
      }
    }, stepDuration);
  }

  // Handle period tab clicks
  function setupPeriodTabs() {
    const tabs = document.querySelectorAll(".period-tab");
    tabs.forEach((tab) => {
      tab.addEventListener("click", () => {
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
      setupPeriodTabs();
      initCharts();
      fetchData();
    },

    activate: (ctx) => {
      console.log("[Home] Activating dashboard");

      // Start polling for updates every 5 seconds
      poller = new Poller(async () => {
        await fetchData();
      }, 5000);

      ctx.managePoller(poller);
    },

    deactivate: () => {
      console.log("[Home] Deactivating dashboard");
      if (poller) {
        poller.stop();
        poller = null;
      }
    },

    dispose: () => {
      console.log("[Home] Disposing dashboard");
      if (memoryChart) {
        memoryChart.destroy();
        memoryChart = null;
      }
      if (cpuChart) {
        cpuChart.destroy();
        cpuChart = null;
      }
    },
  };
}

registerPage("home", createLifecycle());
