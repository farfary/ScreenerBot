// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/* global */
import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { withApprox, withUsdSymbol } from "../core/format.js";
import * as Utils from "../core/utils.js";
import { requestManager, createScopedFetcher } from "../core/request_manager.js";
import { showFeaturedRow, hideFeaturedRow } from "../ui/featured_row.js";
import { renderAddress } from "../ui/token_identity.js";
import { PositionDetailsDialog } from "../ui/position_details_dialog.js";
import { notifyClientReady } from "../core/client_ready.js";
import { WALLET_CHANGE_PERCENT_DECIMALS } from "../core/header_metrics.js";
import { closeMenu, openMenu } from "../core/menu_manager.js";
import { paintHtml, paintText } from "../core/dom.js";
import { createCalendar } from "./home/portfolio_calendar.js";
import { createUpdateNotice } from "./home/update_notice.js";

const ABSENT = "—";

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
  let updateNotice = null;
  let walletAddress = "";
  let walletQrOpen = false;
  let walletIdentityCleanup = null;
  let sparkRoomCleanup = null;
  let exposureCleanup = null;
  let positionDialog = null;
  // Animation intervals tracking
  const animationIntervals = [];

  const walletQrMenu = {
    close: () => closeWalletQr(),
    owns: (target) => document.getElementById("homeWalletIdentity")?.contains(target) || false,
  };

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

  function closeWalletQr() {
    closeMenu(walletQrMenu);
    walletQrOpen = false;
    const popover = document.getElementById("homeWalletQrPopover");
    const button = document.getElementById("homeWalletQrButton");
    if (popover) {
      popover.classList.remove("open");
      popover.hidden = true;
    }
    if (button) {
      button.classList.remove("active");
      button.setAttribute("aria-expanded", "false");
    }
  }

  async function loadWalletQr(address) {
    const image = document.getElementById("homeWalletQrImage");
    const status = document.getElementById("homeWalletQrStatus");
    if (!image || !status || !address || image.dataset.address === address) return;

    image.hidden = true;
    status.hidden = false;
    status.textContent = I18n.t("home-wallet-qr-preparing");

    try {
      const data = await scopedFetch(`/api/wallet/qr/${encodeURIComponent(address)}`, {
        priority: "high",
        cache: "no-store",
      });
      if (walletAddress !== address) return;
      image.src = data.qr_data_url;
      image.dataset.address = address;
      image.hidden = false;
      status.hidden = true;
    } catch (error) {
      if (error?.name === "AbortError") return;
      status.textContent = I18n.t("home-wallet-qr-unavailable");
    }
  }

  function openWalletQr() {
    if (!walletAddress) return;
    const popover = document.getElementById("homeWalletQrPopover");
    const button = document.getElementById("homeWalletQrButton");
    if (!popover || !button) return;

    openMenu(walletQrMenu);
    walletQrOpen = true;
    popover.hidden = false;
    popover.classList.add("open");
    button.classList.add("active");
    button.setAttribute("aria-expanded", "true");
    loadWalletQr(walletAddress);
  }

  function mountWalletIdentity() {
    const qrButton = document.getElementById("homeWalletQrButton");
    const closeButton = document.getElementById("homeWalletQrClose");
    if (!qrButton || !closeButton) return;

    const toggleQr = (event) => {
      event.stopPropagation();
      if (walletQrOpen) closeWalletQr();
      else openWalletQr();
    };
    const closeQr = (event) => {
      event.stopPropagation();
      closeWalletQr();
      qrButton.focus();
    };

    qrButton.addEventListener("click", toggleQr);
    closeButton.addEventListener("click", closeQr);
    walletIdentityCleanup = () => {
      qrButton.removeEventListener("click", toggleQr);
      closeButton.removeEventListener("click", closeQr);
      closeWalletQr();
    };
  }

  function updateWalletIdentity(address) {
    const identity = document.getElementById("homeWalletIdentity");
    const addressEl = document.getElementById("homeWalletAddress");
    const qrAddress = document.getElementById("homeWalletQrAddress");
    const copyButton = document.getElementById("homeWalletCopy");
    const image = document.getElementById("homeWalletQrImage");
    if (!identity || !addressEl || !qrAddress || !copyButton || !image) return;

    const nextAddress = address || "";
    const changed = nextAddress !== walletAddress;
    walletAddress = nextAddress;
    identity.hidden = !nextAddress;
    if (!nextAddress) {
      closeWalletQr();
      return;
    }

    if (changed) {
      addressEl.innerHTML = renderAddress(nextAddress, { plain: true, compact: true });
      qrAddress.innerHTML = renderAddress(nextAddress, { plain: true });
    }
    copyButton.dataset.copy = nextAddress;

    if (changed) {
      image.hidden = true;
      image.removeAttribute("src");
      delete image.dataset.address;
      closeWalletQr();
    }
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

  // Number + a muted "SOL" unit span. Suppress formatSol's built-in " SOL" suffix so
  // the unit isn't doubled. An unknown amount is a bare dash.
  function solHtml(value) {
    if (!Number.isFinite(value)) return ABSENT;
    return `${Utils.formatSol(value, { suffix: "" })}<span class="hero-unit">SOL</span>`;
  }

  // Signed variant of solHtml for the P&L stats.
  function signedSolHtml(value) {
    if (!Number.isFinite(value)) return ABSENT;
    return `${Utils.formatSignedSol(value, { unit: false })}<span class="hero-unit">SOL</span>`;
  }

  const PNL_CLASSES = Object.freeze({ positive: "profit", negative: "loss", neutral: "flat" });

  // Profit/loss/flat semantic class for a signed value as shown at `decimals` (the SOL
  // formatters' precision when omitted): a value that rounds to zero is flat.
  function pnlClass(value, decimals) {
    return PNL_CLASSES[Utils.signedTone(value, decimals)];
  }

  // The sparkline shares the headline row with the balance, which never shrinks. It
  // takes the room the balance leaves, and is dropped when that room is below its CSS
  // `min-width`, so it is never squeezed into an unreadable sliver.
  function mountSparkRoom() {
    const headline = document.querySelector(".portfolio-headline");
    const primary = headline?.querySelector(".portfolio-primary");
    const svg = document.getElementById("heroSpark");
    if (!headline || !primary || !svg || typeof ResizeObserver !== "function") return;
    const fit = () => {
      const style = getComputedStyle(headline);
      const room =
        headline.clientWidth -
        parseFloat(style.paddingInlineStart) -
        parseFloat(style.paddingInlineEnd) -
        parseFloat(style.columnGap) -
        primary.getBoundingClientRect().width;
      svg.classList.toggle("no-room", room < parseFloat(getComputedStyle(svg).minWidth));
    };
    const observer = new ResizeObserver(fit);
    observer.observe(headline);
    observer.observe(primary);
    sparkRoomCleanup = () => observer.disconnect();
  }

  // Render the day's worth trend from an oldest-first array of SOL values. The series
  // starts at the start-of-day worth and ends at the live worth (see the home route), and
  // the line takes the tone of the day change the text beside it shows.
  function renderSparkline(history, tone) {
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

    // Close the same path down to the baseline for the faint area fill.
    if (area) {
      area.setAttribute("points", `${pad.toFixed(1)},${H} ${coords} ${(W - pad).toFixed(1)},${H}`);
    }

    for (const cls of Object.values(PNL_CLASSES)) svg.classList.toggle(cls, cls === tone);
  }

  // Populate the portfolio overview from the full dashboard payload.
  function updateHeroCard(data) {
    const wallet = data.wallet;
    if (!wallet) return;

    updateWalletIdentity(wallet.wallet_address);

    // Headline: total equity (cash + holdings).
    const balanceEl = document.getElementById("walletBalance");
    if (balanceEl) {
      paintHtml(balanceEl, solHtml(wallet.total_equity_native));
    }

    // Approximate USD value of total equity.
    const usdEl = document.getElementById("walletUsd");
    if (usdEl) {
      const usd = wallet.total_equity_native * wallet.native_price_usd;
      const shown = Number.isFinite(usd) && usd > 0;
      usdEl.hidden = !shown;
      if (shown) paintText(usdEl, withApprox(withUsdSymbol(Utils.formatNumber(usd, 2))));
    }

    // Day change: equity against the start-of-day worth, measured only like for like.
    // Without a baseline, or when the start of the day priced other holdings than now,
    // it is a dash. The incomparable case explains itself on hover and on keyboard
    // focus through the shared tooltip, and to assistive technology as its description.
    const known = wallet.change_status === "known" && Number.isFinite(wallet.change_native);
    const tone = known ? pnlClass(wallet.change_native) : "flat";
    const changeEl = document.getElementById("homeWalletChange");
    if (changeEl) {
      changeEl.className = `hero-change ${tone}`;
      const why =
        wallet.change_status === "incomparable"
          ? I18n.attr("home-portfolio-change-incomparable", "title")
          : "";
      if (why) {
        changeEl.dataset.tooltip = why;
        changeEl.setAttribute("aria-description", why);
        changeEl.tabIndex = 0;
      } else {
        delete changeEl.dataset.tooltip;
        changeEl.removeAttribute("aria-description");
        changeEl.removeAttribute("tabindex");
      }
      const percent =
        known && Number.isFinite(wallet.change_percent)
          ? `<span class="change-percent ${tone}">(${Utils.formatPercent(wallet.change_percent, {
              decimals: WALLET_CHANGE_PERCENT_DECIMALS,
            })})</span>`
          : "";
      paintHtml(
        changeEl,
        `<span class="hero-change-value change-value ${tone}">${
          known ? Utils.formatSignedSol(wallet.change_native) : ABSENT
        }</span>${percent ? ` ${percent}` : ""}`
      );
    }

    // Cash tile — free SOL available to trade.
    const cashEl = document.getElementById("heroCash");
    if (cashEl) {
      paintHtml(cashEl, solHtml(wallet.current_balance_native));
    }

    // Holdings tile — SOL value of held tokens, with the token count under it.
    const holdingsEl = document.getElementById("heroHoldings");
    if (holdingsEl) {
      paintHtml(holdingsEl, solHtml(wallet.tokens_worth_native));
    }
    const holdingsCountEl = document.getElementById("heroHoldingsCount");
    if (holdingsCountEl) {
      const n = wallet.token_count ?? 0;
      // A held token we cannot price contributes 0 to the worth. Say so rather than
      // quietly reporting a headline that is short by an unknown amount.
      const unpriced = wallet.unpriced_token_count ?? 0;
      const label = n > 0 ? I18n.t("home-holdings-token-count", { count: n }) : "";
      paintText(
        holdingsCountEl,
        unpriced > 0
          ? I18n.t("home-holdings-with-unpriced", { tokens: label, count: unpriced })
          : label
      );
      holdingsCountEl.title =
        unpriced > 0 ? I18n.t("home-holdings-unpriced-note", { count: unpriced }) : "";
    }

    // Without a wallet nothing trades, so the P&L tiles have no value to show; the
    // Explore banner explains why.
    const hasWallet = Boolean(wallet.wallet_address);

    // Open P&L tile — unrealized, from the positions snapshot, with its percent under it.
    const openPnlEl = document.getElementById("heroOpenPnl");
    if (openPnlEl && data.positions) {
      const v = hasWallet ? data.positions.unrealized_pnl_native : null;
      paintHtml(openPnlEl, signedSolHtml(v));
      openPnlEl.className = `hero-stat-value ${pnlClass(v)}`;
    }
    const openPnlPercentEl = document.getElementById("heroOpenPnlPercent");
    if (openPnlPercentEl && data.positions) {
      const pct = hasWallet ? data.positions.unrealized_pnl_percent : null;
      paintText(
        openPnlPercentEl,
        Number.isFinite(pct) ? Utils.formatPercent(pct, { decimals: 1 }) : ""
      );
    }

    // Realized Today — banked net P&L today, from trader analytics.
    const realizedEl = document.getElementById("heroRealizedToday");
    if (realizedEl && data.trader && data.trader.today) {
      const v = hasWallet ? data.trader.today.net_pnl_native : null;
      paintHtml(realizedEl, signedSolHtml(v));
      realizedEl.className = `hero-stat-value ${pnlClass(v)}`;
    }

    // The day's worth trend, in the day change's tone.
    renderSparkline(wallet.balance_history, tone);
  }

  // A position's symbol as a button that opens its details dialog.
  function positionButton(performer) {
    if (!performer) return ABSENT;
    const key = performer.id != null ? `data-position-id="${performer.id}"` : "";
    return `<button type="button" class="exposure-position" ${key} data-mint="${Utils.escapeHtml(
      performer.mint
    )}">${Utils.escapeHtml(performer.symbol)}</button>`;
  }

  // A performer as "SYMBOL +3.3%", the symbol opening the position.
  function performerHtml(performer) {
    if (!performer) return ABSENT;
    const pnl = performer.pnl_percent || 0;
    return `${positionButton(performer)} <span class="${pnlClass(pnl, 1)}">${Utils.formatPercent(
      pnl,
      { decimals: 1 }
    )}</span>`;
  }

  // Open the details dialog of the position a ledger button names.
  function onExposureClick(event) {
    const button = event.target.closest(".exposure-position");
    if (!button) return;
    const id = Number(button.dataset.positionId);
    positionDialog ??= new PositionDetailsDialog();
    positionDialog.show(Number.isFinite(id) && id > 0 ? { id } : { mint: button.dataset.mint });
  }

  function mountExposure() {
    const ledger = document.getElementById("exposureLedger");
    if (!ledger) return;
    ledger.addEventListener("click", onExposureClick);
    exposureCleanup = () => ledger.removeEventListener("click", onExposureClick);
  }

  // The exposure ledger. Averages and best/worst only mean something across two or more
  // positions; a single position is named instead, with its own hold time.
  function updatePositionsStats(positions) {
    if (!positions) return;

    // The count names the positions the ledger lists, so it is written with them, never
    // counted up through values the ledger contradicts.
    const count = positions.open_count ?? 0;
    paintText(document.getElementById("positionsCount"), Utils.formatNumber(count, 0));

    const emptyEl = document.getElementById("exposureEmpty");
    const ledger = document.getElementById("exposureLedger");
    if (emptyEl) emptyEl.hidden = count > 0;
    if (ledger) {
      ledger.hidden = count === 0;
      ledger.querySelectorAll("[data-scope]").forEach((row) => {
        row.hidden = row.dataset.scope === "single" ? count !== 1 : count < 2;
      });
    }
    if (count === 0) return;

    const setText = (id, text) => paintText(document.getElementById(id), text);
    const setHtml = (id, html) => paintHtml(document.getElementById(id), html);

    setText(
      "positionsInvested",
      Utils.formatSol(positions.total_invested_native, { fallback: ABSENT })
    );
    setText(
      "positionsValue",
      Utils.formatSol(positions.current_value_native, { fallback: ABSENT })
    );

    const pnl = positions.unrealized_pnl_native;
    const pnlPercent = Number.isFinite(positions.unrealized_pnl_percent)
      ? ` <span class="exposure-percent">${Utils.formatPercent(positions.unrealized_pnl_percent, {
          decimals: 1,
        })}</span>`
      : "";
    const pnlEl = document.getElementById("positionsPnl");
    if (pnlEl) {
      paintHtml(pnlEl, `${Utils.formatSignedSol(pnl, { fallback: ABSENT })}${pnlPercent}`);
      pnlEl.className = `position-value ${pnlClass(pnl)}`;
    }

    const mins = positions.avg_hold_duration_mins;
    const hold = Number.isFinite(mins)
      ? Utils.formatUptime(mins * 60, { style: "compact" })
      : ABSENT;
    if (count === 1) {
      setHtml("positionsOnly", performerHtml(positions.best_performer));
      setText("positionsHeld", hold);
      return;
    }
    setText(
      "positionsAvgSize",
      Utils.formatSol(positions.avg_position_size_native, { fallback: ABSENT })
    );
    setText("positionsAvgHold", hold);
    setHtml("positionsBest", performerHtml(positions.best_performer));
    setHtml("positionsWorst", performerHtml(positions.worst_performer));
  }

  // The market pipeline: a funnel that narrows in order (tracked, monitored, passed),
  // then the monitored tokens' other counts. Rejected is a normal outcome, not an alarm.
  function updateTokenStats(tokens) {
    if (!tokens) return;

    const count = (id, value) => {
      const el = document.getElementById(id);
      if (el) animateValue(el, value);
    };
    count("tokensTotal", tokens.total_in_database);
    count("tokensMonitored", tokens.monitored);
    count("tokensPassed", tokens.passed_filters);
    count("tokensRejected", tokens.rejected_filters);
    count("tokensBlacklisted", tokens.blacklisted);
    count("tokensWithPrices", tokens.with_prices);
    count("tokensOhlcv", tokens.with_ohlcv);

    const passedEl = document.getElementById("tokensPassed");
    if (passedEl) passedEl.className = Utils.countTone(tokens.passed_filters, "success");
  }

  // Animate number value changes
  function animateValue(element, targetValue) {
    if (!element) return;

    // A null target is a value the backend does not have yet — the filtering counters are
    // null until the first snapshot exists. There is nothing to count towards, so show the
    // absent marker and leave no stale numeric value behind for the next animation to
    // interpolate from.
    if (targetValue === null || targetValue === undefined || !Number.isFinite(targetValue)) {
      delete element.dataset.numericValue;
      element.textContent = Utils.formatNumber(targetValue, 0);
      return;
    }

    // With no number on screen yet (the absent marker) there is nothing to count from:
    // starting at 0 would show a zero the backend never reported.
    if (element.dataset.numericValue === undefined) {
      element.dataset.numericValue = String(targetValue);
      element.textContent = Utils.formatNumber(targetValue, 0);
      return;
    }
    const currentValue = Number(element.dataset.numericValue);
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
      updateNotice = createUpdateNotice(Utils);
      mountWalletIdentity();
      mountSparkRoom();
      mountExposure();
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
      if (!updateNotice) {
        updateNotice = createUpdateNotice(Utils);
      }
      if (!walletIdentityCleanup) {
        mountWalletIdentity();
      }
      if (!sparkRoomCleanup) {
        mountSparkRoom();
      }
      if (!exposureCleanup) {
        mountExposure();
      }

      // If we have cached data from a previous visit, show it immediately
      // This provides instant feedback while fresh data loads
      if (cachedData) {
        updateUI(cachedData);
        markLoaded();
      }

      if (!poller) {
        poller = new Poller(
          () => {
            fetchData();
            calendar?.refresh();
            updateNotice?.refresh();
          },
          {
            label: "HomeDashboard", // l10n-ignore: internal poller name
            getInterval: () => 5000,
          }
        );
      }

      ctx.managePoller(poller);
      poller.start({ silent: true });
      fetchData();
      updateNotice.refresh();

      // Show featured promotional row
      showFeaturedRow();
    },

    deactivate: () => {
      console.log("[Home] Deactivating dashboard");

      // Hide featured row when leaving page
      hideFeaturedRow();

      // Managed pollers are stopped centrally and retained for cached re-entry.
    },

    dispose: () => {
      console.log("[Home] Disposing dashboard");
      scopedFetch = null;
      calendarFetch = null;

      calendar?.dispose();
      calendar = null;
      updateNotice?.dispose();
      updateNotice = null;
      walletIdentityCleanup?.();
      walletIdentityCleanup = null;
      sparkRoomCleanup?.();
      sparkRoomCleanup = null;
      exposureCleanup?.();
      exposureCleanup = null;
      positionDialog?.destroy();
      positionDialog = null;
      walletAddress = "";

      // Note: cachedData is deliberately kept — it lets a revisit paint real
      // numbers immediately instead of flashing the skeleton again.
      poller = null;

      // Clear all animation intervals
      animationIntervals.forEach((interval) => clearInterval(interval));
      animationIntervals.length = 0;
    },
  };
}

registerPage("home", createLifecycle());
