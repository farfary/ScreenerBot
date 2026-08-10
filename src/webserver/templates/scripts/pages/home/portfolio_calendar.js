// Portfolio calendar — month grid of daily realized P&L + end-of-day portfolio value.
import * as Utils from "../../core/utils.js";

const MONTH_NAMES = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

/**
 * Create a portfolio calendar controller bound to the home page DOM.
 * @param {(url:string, opts?:object)=>Promise<any>} fetcher scoped fetch
 * @returns {{ mount:Function, refresh:Function, dispose:Function }}
 */
export function createCalendar(fetcher) {
  const now = new Date();
  let year = now.getUTCFullYear();
  let month = now.getUTCMonth() + 1; // 1-12
  let lastKey = null;
  let lastData = null;
  const cleanups = [];
  // Per-day detail lookup (date -> day object) + shared hover popover element.
  const dayMap = new Map();
  let popoverEl = null;

  const grid = () => document.getElementById("portfolioCalendarGrid");
  const monthLabel = () => document.getElementById("calendarMonthLabel");

  function track(el, evt, handler) {
    if (!el) return;
    el.addEventListener(evt, handler);
    cleanups.push(() => el.removeEventListener(evt, handler));
  }

  function isCurrentMonth() {
    const n = new Date();
    return year === n.getUTCFullYear() && month === n.getUTCMonth() + 1;
  }

  // Compute weekday offset + day count for a month, client-side (UTC).
  function monthMeta(y, m) {
    return {
      firstWeekday: new Date(Date.UTC(y, m - 1, 1)).getUTCDay(),
      daysInMonth: new Date(Date.UTC(y, m, 0)).getUTCDate(),
    };
  }

  // Render an immediate structural skeleton (same grid shape as loaded data)
  // so the card never shows as an empty/confusing box while the fetch is in flight.
  function renderSkeleton() {
    const gridEl = grid();
    if (!gridEl) return;
    const { firstWeekday, daysInMonth } = monthMeta(year, month);

    const labelEl = monthLabel();
    if (labelEl) labelEl.textContent = `${MONTH_NAMES[month - 1]} ${year}`;

    const todayIso = new Date().toISOString().slice(0, 10);
    const cells = [];
    for (let i = 0; i < firstWeekday; i++) {
      cells.push('<div class="calendar-cell blank"></div>');
    }
    for (let d = 1; d <= daysInMonth; d++) {
      const date = `${year}-${String(month).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
      let cls = "calendar-cell skeleton";
      if (date === todayIso) cls += " today";
      else if (date > todayIso) cls += " future";
      cells.push(`<div class="${cls}"><span class="cell-day">${d}</span></div>`);
    }
    gridEl.innerHTML = cells.join("");
  }

  async function load() {
    if (typeof fetcher !== "function") return;
    try {
      const data = await fetcher(
        `/api/dashboard/portfolio-calendar?year=${year}&month=${month}`,
        { priority: "low", cache: "no-store" }
      );
      render(data);
    } catch (error) {
      if (error?.name === "AbortError") return;
      console.error("[Calendar] fetch failed:", error);
    }
  }

  function render(data) {
    const gridEl = grid();
    if (!gridEl || !data) return;
    lastData = data;

    // Skip redraw when nothing changed (avoids flicker / scroll reset). Theme is
    // part of the key so a light/dark switch re-tints the cells (intensity differs).
    const theme = document.documentElement.getAttribute("data-theme") || "dark";
    const key = `${theme}|${JSON.stringify(data)}`;
    if (key === lastKey) return;
    lastKey = key;
    const isLight = theme === "light";

    const labelEl = monthLabel();
    if (labelEl) labelEl.textContent = `${MONTH_NAMES[data.month - 1]} ${data.year}`;

    // Heatmap scale: largest absolute daily P&L in the month.
    let maxAbs = 0;
    for (const d of data.days) {
      const a = Math.abs(d.net_pnl_sol || 0);
      if (a > maxAbs) maxAbs = a;
    }

    const todayIso = new Date().toISOString().slice(0, 10);
    const cells = [];
    dayMap.clear();

    // Leading blanks so the 1st lands on the correct weekday.
    for (let i = 0; i < data.first_weekday; i++) {
      cells.push('<div class="calendar-cell blank"></div>');
    }

    for (const d of data.days) {
      const pnl = d.net_pnl_sol || 0;
      const isFuture = d.date > todayIso;
      const isToday = d.date === todayIso;
      let cls = "calendar-cell";
      if (isFuture) cls += " future";
      if (isToday) cls += " today";
      if (!d.has_data && !isFuture) cls += " empty";
      if (pnl > 0) cls += " profit";
      else if (pnl < 0) cls += " loss";

      // Heatmap tint intensity relative to the month's largest absolute P&L.
      // Uses a sqrt curve + a solid floor so even a faint day reads as clearly
      // green/red rather than a muddy near-black tint. Light theme needs a much
      // higher floor: the tint sits over a WHITE card, so a low alpha yields a
      // pale pastel that white cell text can't sit on. Keep it saturated so the
      // white text (matching dark theme) always reads.
      let style = "";
      if (d.has_data && maxAbs > 0 && pnl !== 0) {
        const ratio = Math.sqrt(Math.abs(pnl) / maxAbs);
        const intensity = isLight
          ? Math.min(0.96, 0.78 + ratio * 0.18)
          : Math.min(0.9, 0.35 + ratio * 0.55);
        const rgb = pnl > 0 ? "63, 185, 80" : "248, 81, 73";
        style = ` style="background: rgba(${rgb}, ${intensity.toFixed(3)});"`;
      }

      const pnlText = d.has_data
        ? `${pnl > 0 ? "+" : ""}${Utils.formatSol(pnl, { decimals: 3, suffix: "" })}`
        : "";
      const valText =
        d.portfolio_value_sol != null
          ? `${Utils.formatSol(d.portfolio_value_sol, { decimals: 2, suffix: "" })}`
          : "";

      // Only days with trade activity are hoverable; expose them for the popover.
      const detailAttr = d.has_data ? ` data-date="${d.date}"` : "";
      if (d.has_data) dayMap.set(d.date, d);

      cells.push(
        `<div class="${cls}"${style}${detailAttr}>` +
          `<span class="cell-day">${d.day}</span>` +
          (pnlText ? `<span class="cell-pnl">${pnlText}</span>` : "") +
          (valText ? `<span class="cell-value">${valText}</span>` : "") +
          "</div>"
      );
    }

    hidePopover(); // avoid a stale popover pinned to a cell we're about to replace
    gridEl.innerHTML = cells.join("");

    const pnlEl = document.getElementById("calendarMonthPnl");
    if (pnlEl) {
      const mp = data.month_net_pnl_sol || 0;
      const cls = mp > 0 ? "profit" : mp < 0 ? "loss" : "flat";
      pnlEl.textContent = `${mp > 0 ? "+" : ""}${Utils.formatSol(mp, {
        decimals: 3,
        suffix: "",
      })} SOL`;
      pnlEl.className = `calendar-summary-value ${cls}`;
    }
    const tradesEl = document.getElementById("calendarMonthTrades");
    if (tradesEl) tradesEl.textContent = Utils.formatNumber(data.month_trades ?? 0, 0);
  }

  function shiftMonth(delta) {
    month += delta;
    if (month > 12) {
      month = 1;
      year += 1;
    } else if (month < 1) {
      month = 12;
      year -= 1;
    }
    lastKey = null; // force redraw for the new month
    renderSkeleton(); // show the new month's structure immediately
    load();
  }

  function updateNavState() {
    const nextBtn = document.getElementById("calendarNext");
    if (nextBtn) nextBtn.disabled = isCurrentMonth();
  }

  // ===== Hover detail popover (traded days only) =====

  function ensurePopover() {
    if (popoverEl) return popoverEl;
    popoverEl = document.createElement("div");
    popoverEl.className = "cal-day-popover";
    popoverEl.setAttribute("role", "tooltip");
    popoverEl.hidden = true;
    document.body.appendChild(popoverEl);
    return popoverEl;
  }

  function fmtSol(v, decimals = 3) {
    return Utils.formatSol(v, { decimals, suffix: " SOL" });
  }

  function popoverRow(label, value, cls = "") {
    return (
      `<div class="cal-pop-row"><span class="cal-pop-label">${label}</span>` +
      `<span class="cal-pop-val ${cls}">${value}</span></div>`
    );
  }

  function buildPopoverHtml(d) {
    const pnl = d.net_pnl_sol || 0;
    const pnlCls = pnl > 0 ? "profit" : pnl < 0 ? "loss" : "";
    const dt = new Date(`${d.date}T00:00:00Z`);
    const dateStr = dt.toLocaleDateString(undefined, {
      weekday: "short",
      month: "short",
      day: "numeric",
      timeZone: "UTC",
    });
    const trades = d.trades || 0;
    const wins = d.wins || 0;
    const losses = Math.max(0, trades - wins);
    const winRate = trades > 0 ? Math.round((wins / trades) * 100) : 0;

    const rows = [
      popoverRow("Net P&L", `${pnl >= 0 ? "+" : ""}${fmtSol(pnl)}`, pnlCls),
      popoverRow("Trades", String(trades)),
      popoverRow("Win rate", `${winRate}% · ${wins}W / ${losses}L`),
    ];
    if (d.profit_sol) rows.push(popoverRow("Gross profit", `+${fmtSol(d.profit_sol)}`, "profit"));
    if (d.loss_sol) rows.push(popoverRow("Gross loss", `-${fmtSol(d.loss_sol)}`, "loss"));
    if (d.portfolio_value_sol != null) {
      rows.push(popoverRow("End balance", fmtSol(d.portfolio_value_sol)));
    }

    return (
      `<div class="cal-pop-head"><span class="cal-pop-date">${dateStr}</span>` +
      `<span class="cal-pop-year">${dt.getUTCFullYear()}</span></div>` +
      `<div class="cal-pop-body">${rows.join("")}</div>`
    );
  }

  function positionPopover(cell) {
    const el = ensurePopover();
    const m = 8;
    const vw = document.documentElement.clientWidth;
    const vh = document.documentElement.clientHeight;
    const rect = cell.getBoundingClientRect();
    const pw = el.offsetWidth;
    const ph = el.offsetHeight;

    // Prefer above the cell; flip below if it doesn't fit.
    let top = rect.top - ph - m;
    if (top < m) top = rect.bottom + m;
    if (top + ph > vh - m) top = Math.max(m, vh - ph - m);

    // Center horizontally over the cell, clamped to the viewport.
    let left = rect.left + rect.width / 2 - pw / 2;
    left = Math.max(m, Math.min(left, vw - pw - m));

    el.style.top = `${Math.round(top)}px`;
    el.style.left = `${Math.round(left)}px`;
  }

  function showPopover(cell) {
    const day = dayMap.get(cell.dataset.date);
    if (!day) return;
    const el = ensurePopover();
    el.innerHTML = buildPopoverHtml(day);
    el.hidden = false;
    positionPopover(cell);
    // Force reflow so the entrance transition runs, then reveal.
    void el.offsetWidth;
    el.classList.add("cal-day-popover--visible");
  }

  function hidePopover() {
    if (!popoverEl) return;
    popoverEl.classList.remove("cal-day-popover--visible");
    popoverEl.hidden = true;
  }

  function onGridOver(e) {
    const cell = e.target.closest(".calendar-cell[data-date]");
    if (cell) showPopover(cell);
  }

  function onGridOut(e) {
    const cell = e.target.closest(".calendar-cell[data-date]");
    if (!cell) return;
    // Ignore moves that stay inside the same cell.
    if (e.relatedTarget && cell.contains(e.relatedTarget)) return;
    hidePopover();
  }

  return {
    mount() {
      track(document.getElementById("calendarPrev"), "click", () => {
        shiftMonth(-1);
        updateNavState();
      });
      track(document.getElementById("calendarNext"), "click", () => {
        if (isCurrentMonth()) return;
        shiftMonth(1);
        updateNavState();
      });
      // Delegated hover on the grid container (survives per-render innerHTML swaps).
      const gridEl = grid();
      track(gridEl, "mouseover", onGridOver);
      track(gridEl, "mouseout", onGridOut);

      // Re-tint cells when the theme changes (intensity floor differs per theme).
      track(window, "screenerbot:theme", () => {
        if (lastData) render(lastData);
      });

      updateNavState();
      renderSkeleton();
      load();
    },

    // Called by the home poller — only refresh live (current) month.
    refresh() {
      if (isCurrentMonth()) load();
    },

    dispose() {
      cleanups.forEach((fn) => fn());
      cleanups.length = 0;
      lastKey = null;
      lastData = null;
      hidePopover();
      dayMap.clear();
      if (popoverEl) {
        popoverEl.remove();
        popoverEl = null;
      }
    },
  };
}
