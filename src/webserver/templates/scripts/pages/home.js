/* global */
import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { requestManager, createScopedFetcher } from "../core/request_manager.js";
import { showBillboardRow, hideBillboardRow } from "../ui/billboard_row.js";
import { notifyClientReady } from "../core/client_ready.js";
import { createCalendar } from "./home/portfolio_calendar.js";

function createLifecycle() {
  let poller = null;
  let scopedFetch = null;
  // The calendar gets its OWN scoped fetcher (not shared with the dashboard
  // fetch): scoped fetchers tie their requests to the page lifecycle so they're
  // cancelled on dispose, and keeping them separate means neither concern can
  // cancel the other. The dashboard fetcher is deliberately NOT `latestOnly` —
  // see fetchData's in-flight guard for why.
  let calendarFetch = null;
  let cachedData = null;
  // Guards against overlapping dashboard fetches (see fetchData).
  let isFetching = false;
  let calendar = null;
  // Animation intervals tracking
  const animationIntervals = [];

  /**
   * Clear the skeleton and fade the real data in.
   *
   * The loading state is only ever entered declaratively, via the `loading`
   * class baked into the page template at first paint. This only ever needs to
   * LEAVE that state; there is no
   * "turn loading back on" path, and there must not be one: the router caches
   * the page element, so a revisit reuses the already-populated DOM and
   * re-skeletoning it would flash the cards for no reason.
   */
  function markLoaded() {
    const dashboard = document.querySelector(".home-dashboard");
    dashboard?.classList.remove("loading");
    dashboard?.classList.add("loaded");
  }

  // Fetch dashboard data
  async function fetchData() {
    // Never start a second fetch while one is still in flight. The 5s poller
    // used to fire straight into a `latestOnly` fetcher, which ABORTED the
    // still-pending request; fetchData's AbortError branch then returned
    // WITHOUT clearing the loading skeleton. Any response slower than the poll
    // interval (startup service contention, a slow RPC tick, a brief network
    // hiccup) therefore got cancelled before it could resolve, and the wallet
    // hero + cards stayed stuck in the loading state forever. Skipping the tick
    // lets the in-flight request finish and clear the skeleton.
    if (isFetching) return;
    isFetching = true;

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
      markLoaded();
      // Landing page has rendered real data — the app is fully up. Fire the
      // one-time "frontend ready" signal so the backend can log/observe it.
      notifyClientReady({ page: "home" });
    } catch (error) {
      if (error?.name === "AbortError") {
        // Only reached on page dispose (the lifecycle ctx aborts its requests).
        // The next visit's fetch will repopulate; nothing to clear here.
        return;
      }
      console.error("Error fetching dashboard data:", error);
      // Remove loading state on error to avoid stuck loading
      markLoaded();
    } finally {
      isFetching = false;
    }
  }

  // Update all UI elements
  function updateUI(data) {
    if (!data) return;

    // Portfolio headline, daily change, balances and sparkline.
    updateHeroCard(data);

    // Update the exposure ledger.
    updatePositionsStats(data.positions);

    // Update the market pipeline.
    updateTokenStats(data.tokens);
  }

  // Format a signed SOL value, e.g. "+0.1234" / "-0.0500" / "0.0000".
  function formatSignedSol(value, decimals = 4) {
    const v = value || 0;
    const sign = v > 0 ? "+" : v < 0 ? "-" : "";
    return `${sign}${Utils.formatSol(Math.abs(v), { decimals })}`;
  }

  // Number + a small muted "SOL" unit span. Suppress formatSol's built-in
  // " SOL" suffix so the unit isn't doubled.
  function solHtml(value, decimals = 4) {
    return `${Utils.formatSol(value || 0, { decimals, suffix: "" })}<span class="hero-unit">SOL</span>`;
  }

  // Signed variant of solHtml for the P&L stats.
  function signedSolHtml(value, decimals = 4) {
    const v = value || 0;
    const sign = v > 0 ? "+" : v < 0 ? "-" : "";
    return `${sign}${Utils.formatSol(Math.abs(v), {
      decimals,
      suffix: "",
    })}<span class="hero-unit">SOL</span>`;
  }

  // Profit/loss/flat semantic class for a signed value.
  function pnlClass(value) {
    if (value > 0) return "profit";
    if (value < 0) return "loss";
    return "flat";
  }

  // Render the balance-trend sparkline from an oldest-first array of SOL values.
  function renderSparkline(history) {
    const line = document.getElementById("heroSparkLine");
    const area = document.getElementById("heroSparkArea");
    const svg = document.getElementById("heroSpark");
    if (!line || !svg) return;

    const pts = Array.isArray(history) ? history.filter((n) => Number.isFinite(n)) : [];
    if (pts.length < 2) {
      // Nothing meaningful to plot — hide the line rather than draw a flat stub.
      line.setAttribute("points", "");
      if (area) area.setAttribute("points", "");
      svg.classList.add("empty");
      return;
    }
    svg.classList.remove("empty");

    const W = 120;
    const H = 40;
    const pad = 3;
    const min = Math.min(...pts);
    const max = Math.max(...pts);
    const range = max - min || 1;
    const stepX = (W - pad * 2) / (pts.length - 1);
    const coords = pts
      .map((v, i) => {
        const x = pad + i * stepX;
        const y = pad + (H - pad * 2) * (1 - (v - min) / range);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
    line.setAttribute("points", coords);

    // Close the same path down to the baseline for the soft area fill.
    if (area) {
      area.setAttribute("points", `${pad.toFixed(1)},${H} ${coords} ${(W - pad).toFixed(1)},${H}`);
    }

    // Colour the trend by net direction across the window.
    const up = pts[pts.length - 1] >= pts[0];
    svg.classList.toggle("up", up);
    svg.classList.toggle("down", !up);
  }

  // Populate the portfolio overview from the full dashboard payload.
  function updateHeroCard(data) {
    const wallet = data.wallet;
    if (!wallet) return;

    // Headline: total equity (cash + holdings).
    const balanceEl = document.getElementById("walletBalance");
    if (balanceEl) {
      balanceEl.innerHTML = solHtml(wallet.total_equity_sol, 4);
    }

    // Approximate USD value of total equity.
    const usdEl = document.getElementById("walletUsd");
    if (usdEl) {
      const usd = (wallet.total_equity_sol || 0) * (wallet.sol_price_usd || 0);
      if (usd > 0) {
        usdEl.textContent = `≈ $${Utils.formatNumber(usd, 2)}`;
        usdEl.style.display = "";
      } else {
        usdEl.style.display = "none";
      }
    }

    // Today change (equity vs start-of-day baseline). The container itself is a
    // tinted pill, so it carries the profit/loss class too.
    const changeEl = document.getElementById("homeWalletChange");
    if (changeEl) {
      const cls = pnlClass(wallet.change_sol);
      changeEl.className = `hero-change ${cls}`;
      changeEl.innerHTML = `
        <span class="hero-change-value change-value ${cls}">${formatSignedSol(
          wallet.change_sol
        )}</span>
        <span class="change-percent ${cls}">(${Utils.formatPercent(wallet.change_percent, {
          decimals: 2,
        })})</span>
      `;
    }

    // Cash tile — free SOL available to trade.
    const cashEl = document.getElementById("heroCash");
    if (cashEl) {
      cashEl.innerHTML = solHtml(wallet.current_balance_sol, 4);
    }

    // Holdings tile — SOL value of held tokens, with a token-count subscript.
    const holdingsEl = document.getElementById("heroHoldings");
    if (holdingsEl) {
      holdingsEl.innerHTML = solHtml(wallet.tokens_worth_sol, 4);
    }
    const holdingsCountEl = document.getElementById("heroHoldingsCount");
    if (holdingsCountEl) {
      const n = wallet.token_count || 0;
      // A held token we cannot price contributes 0 to the worth. Say so rather than
      // quietly reporting a headline that is short by an unknown amount.
      const unpriced = wallet.unpriced_token_count || 0;
      const label = n > 0 ? `${n} token${n === 1 ? "" : "s"}` : "";
      holdingsCountEl.textContent = unpriced > 0 ? `${label} · ${unpriced} unpriced` : label;
      holdingsCountEl.title =
        unpriced > 0
          ? `${unpriced} held token${unpriced === 1 ? " has" : "s have"} no price available and count as 0 in the total`
          : "";
    }

    // Open P&L tile — unrealized, from the positions snapshot.
    const openPnlEl = document.getElementById("heroOpenPnl");
    if (openPnlEl && data.positions) {
      const v = data.positions.unrealized_pnl_sol || 0;
      const pct = data.positions.unrealized_pnl_percent || 0;
      openPnlEl.innerHTML = `${signedSolHtml(
        v
      )} <span class="hero-stat-sub">${Utils.formatPercent(pct, { decimals: 1 })}</span>`;
      openPnlEl.className = `hero-stat-value ${pnlClass(v)}`;
    }

    // Realized Today — banked net P&L today, from trader analytics.
    const realizedEl = document.getElementById("heroRealizedToday");
    if (realizedEl && data.trader && data.trader.today) {
      const v = data.trader.today.net_pnl_sol || 0;
      realizedEl.innerHTML = signedSolHtml(v);
      realizedEl.className = `hero-stat-value ${pnlClass(v)}`;
    }

    // Balance-trend sparkline.
    renderSparkline(wallet.balance_history);
  }

  // Update positions statistics
  function updatePositionsStats(positions) {
    if (!positions) return;

    const countEl = document.getElementById("positionsCount");
    const investedEl = document.getElementById("positionsInvested");
    const avgSizeEl = document.getElementById("positionsAvgSize");
    const avgHoldEl = document.getElementById("positionsAvgHold");
    const bestEl = document.getElementById("positionsBest");
    const worstEl = document.getElementById("positionsWorst");

    if (countEl) animateValue(countEl, positions.open_count);
    if (investedEl)
      investedEl.textContent = Utils.formatSol(positions.total_invested_sol, {
        decimals: 4,
      });
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
        const pnl = positions.best_performer.pnl_percent || 0;
        bestEl.textContent = `${positions.best_performer.symbol} ${Utils.formatPercent(pnl, {
          decimals: 1,
        })}`;
        bestEl.className = `position-value ${pnlClass(pnl)}`;
      } else {
        bestEl.textContent = "—";
        bestEl.className = "position-value";
      }
    }
    if (worstEl) {
      if (positions.worst_performer) {
        const pnl = positions.worst_performer.pnl_percent || 0;
        worstEl.textContent = `${positions.worst_performer.symbol} ${Utils.formatPercent(pnl, {
          decimals: 1,
        })}`;
        worstEl.className = `position-value ${pnlClass(pnl)}`;
      } else {
        worstEl.textContent = "—";
        worstEl.className = "position-value";
      }
    }
  }

  // Update system statistics
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

    const currentValue =
      Number(element.dataset.numericValue ?? element.textContent.replaceAll(",", "")) || 0;
    if (currentValue === targetValue) return;
    element.dataset.numericValue = String(targetValue);

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
        element.textContent = Utils.formatNumber(targetValue, 0);
        clearInterval(interval);
        const idx = animationIntervals.indexOf(interval);
        if (idx !== -1) animationIntervals.splice(idx, 1);
      } else {
        element.textContent = Utils.formatNumber(Math.round(current), 0);
      }
    }, stepDuration);

    animationIntervals.push(interval);
  }

  return {
    init: (ctx) => {
      console.log("[Home] Initializing dashboard");
      // Dashboard fetcher is NOT latestOnly — fetchData's in-flight guard
      // already prevents overlap, and latestOnly would abort a slow in-flight
      // request on the next poll tick (the stuck-loading bug).
      scopedFetch = createScopedFetcher(ctx);
      calendarFetch = createScopedFetcher(ctx, { latestOnly: true });

      // Note: Loading state is already applied via HTML classes
      // Data fetch happens in activate() to avoid double call

      // Portfolio calendar.
      calendar = createCalendar(calendarFetch);
      calendar.mount();
    },

    activate: (ctx) => {
      console.log("[Home] Activating dashboard");

      if (!scopedFetch) {
        scopedFetch = createScopedFetcher(ctx);
      }
      if (!calendarFetch) {
        calendarFetch = createScopedFetcher(ctx, { latestOnly: true });
      }

      if (!calendar) {
        calendar = createCalendar(calendarFetch);
        calendar.mount();
      }

      // If we have cached data from a previous visit, show it immediately
      // This provides instant feedback while fresh data loads
      if (cachedData) {
        updateUI(cachedData);
        markLoaded();
      }

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(
            () => {
              fetchData();
              calendar?.refresh();
            },
            {
              label: "HomeDashboard",
              getInterval: () => 5000,
            }
          )
        );
      }

      poller.start({ silent: true });
      fetchData();

      // Show billboard promotional row
      showBillboardRow();
    },

    deactivate: () => {
      console.log("[Home] Deactivating dashboard");

      // Hide billboard row when leaving page
      hideBillboardRow();

      if (poller) {
        poller.stop({ silent: true });
        poller = null;
      }
    },

    dispose: () => {
      console.log("[Home] Disposing dashboard");
      scopedFetch = null;
      calendarFetch = null;

      calendar?.dispose();
      calendar = null;

      // Note: cachedData is deliberately kept — it lets a revisit paint real
      // numbers immediately instead of flashing the skeleton again.

      // Clear all animation intervals
      animationIntervals.forEach((interval) => clearInterval(interval));
      animationIntervals.length = 0;
    },
  };
}

registerPage("home", createLifecycle());
