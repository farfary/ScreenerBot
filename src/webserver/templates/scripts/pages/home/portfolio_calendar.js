// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Portfolio calendar — month grid of daily realized P&L and weekly totals, with a per-day detail popover.
import { firstDayOfWeek, formatMonthYear, formatWeekday } from "../../core/format.js";
import * as Utils from "../../core/utils.js";

// The grid is a Gregorian month, so its title and day labels name Gregorian dates in
// every locale (fa would otherwise name the Persian month over Gregorian days).
const GRID_CALENDAR = "gregory";

const PNL_CLASSES = Object.freeze({ positive: "profit", negative: "loss", neutral: "flat" });
const POPOVER_PNL_CLASSES = Object.freeze({ positive: "profit", negative: "loss", neutral: "" });
const POPOVER_ID = "homeCalendarDayPopover";
const DAYS_PER_WEEK = 7;
// Tint of a traded day, as a share of the profit or loss colour mixed into the cell:
// the faintest day starts at the floor, the month's largest reaches floor + span.
const TINT_FLOOR = 8;
const TINT_SPAN = 18;

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
  const cleanups = [];
  // Per-day detail lookup (date -> day object) + shared hover popover element.
  const dayMap = new Map();
  let popoverEl = null;
  // The day cell whose detail the popover shows.
  let activeCell = null;

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

  // Blank cells before the 1st: its weekday (0 = Sunday) counted from the locale's week start.
  function leadingBlanks(weekday) {
    return (weekday - firstDayOfWeek() + 7) % 7;
  }

  // Compute weekday offset + day count for a month, client-side (UTC).
  function monthMeta(y, m) {
    return {
      firstWeekday: new Date(Date.UTC(y, m - 1, 1)).getUTCDay(),
      daysInMonth: new Date(Date.UTC(y, m, 0)).getUTCDate(),
    };
  }

  // Lay a month out in week rows: blanks before the 1st and after the last day, so
  // every row holds seven cells, each row closed by its week cell.
  function weekRows(leading, dayCells, weekCell) {
    const slots = [
      ...Array.from({ length: leading }, () => null),
      ...dayCells,
    ];
    while (slots.length % DAYS_PER_WEEK !== 0) slots.push(null);
    const html = [];
    for (let start = 0; start < slots.length; start += DAYS_PER_WEEK) {
      const week = slots.slice(start, start + DAYS_PER_WEEK);
      for (const cell of week) html.push(cell ? cell.html : '<div class="calendar-cell blank"></div>');
      html.push(weekCell(week.filter(Boolean)));
    }
    return html.join("");
  }

  // Render an immediate structural skeleton (same grid shape as loaded data)
  // so the card never shows as an empty/confusing box while the fetch is in flight.
  function renderSkeleton() {
    const gridEl = grid();
    if (!gridEl) return;
    const { firstWeekday, daysInMonth } = monthMeta(year, month);

    const labelEl = monthLabel();
    if (labelEl) labelEl.textContent = formatMonthYear(year, month, { calendar: GRID_CALENDAR });

    const todayIso = new Date().toISOString().slice(0, 10);
    const days = [];
    for (let d = 1; d <= daysInMonth; d++) {
      const date = `${year}-${String(month).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
      let cls = "calendar-cell skeleton";
      if (date === todayIso) cls += " today";
      else if (date > todayIso) cls += " future";
      days.push({ html: `<div class="${cls}"><span class="cell-day">${d}</span></div>` });
    }
    gridEl.innerHTML = weekRows(
      leadingBlanks(firstWeekday),
      days,
      () => '<div class="calendar-week skeleton"></div>'
    );
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

  // A week's total: the realized P&L of its traded days, blank for a week without trades.
  function weekCellHtml(days) {
    const traded = days.filter((cell) => cell.day.has_data);
    if (traded.length === 0) return '<div class="calendar-week"></div>';
    const total = traded.reduce((sum, cell) => sum + (cell.day.net_pnl_native || 0), 0);
    const tone = PNL_CLASSES[Utils.signedTone(total)];
    return (
      `<div class="calendar-week ${tone}">` +
      `<span class="cell-pnl">${Utils.formatSignedSol(total, { unit: false })}</span></div>`
    );
  }

  function render(data) {
    const gridEl = grid();
    if (!gridEl || !data) return;

    // Skip redraw when nothing changed (avoids flicker and a lost keyboard focus).
    const key = JSON.stringify(data);
    if (key === lastKey) return;
    lastKey = key;

    const labelEl = monthLabel();
    if (labelEl) {
      labelEl.textContent = formatMonthYear(data.year, data.month, { calendar: GRID_CALENDAR });
    }

    // Heatmap scale: largest absolute daily P&L in the month.
    let maxAbs = 0;
    for (const d of data.days) {
      const a = Math.abs(d.net_pnl_native || 0);
      if (a > maxAbs) maxAbs = a;
    }

    const todayIso = new Date().toISOString().slice(0, 10);
    const focusedDate = gridEl.contains(document.activeElement)
      ? document.activeElement.dataset.date
      : null;
    dayMap.clear();

    const days = data.days.map((d) => {
      const pnl = d.net_pnl_native || 0;
      const isFuture = d.date > todayIso;
      let cls = "calendar-cell";
      if (isFuture) cls += " future";
      if (d.date === todayIso) cls += " today";
      // A past day without trades is a quiet day, not an empty one.
      if (!d.has_data && !isFuture) cls += " no-trades";
      const tone = d.has_data ? Utils.signedTone(pnl) : "neutral";
      if (tone !== "neutral") cls += ` ${PNL_CLASSES[tone]}`;

      // Tint intensity relative to the month's largest absolute P&L, on a sqrt curve
      // so a small day still reads as green or red.
      const style =
        tone !== "neutral" && maxAbs > 0
          ? ` style="--cell-tint: ${(TINT_FLOOR + Math.sqrt(Math.abs(pnl) / maxAbs) * TINT_SPAN).toFixed(1)}%"`
          : "";

      // Every day with a detail (trades, or an end-of-day worth) opens it on hover,
      // keyboard focus or a tap.
      const hasDetail = d.has_data || d.portfolio_value_native != null;
      if (hasDetail) dayMap.set(d.date, d);
      const detailAttr = hasDetail
        ? ` data-date="${d.date}" tabindex="0" aria-label="${Utils.escapeHtml(dayLabel(d))}"`
        : "";
      const pnlText = d.has_data ? Utils.formatSignedSol(pnl, { unit: false }) : "";

      return {
        day: d,
        html:
          `<div class="${cls}"${style}${detailAttr}>` +
          `<span class="cell-day">${d.day}</span>` +
          (pnlText ? `<span class="cell-pnl">${pnlText}</span>` : "") +
          "</div>",
      };
    });

    // A refresh (today's worth moves the day's end balance) rebuilds the cells; an open
    // day detail moves to its new cell with fresh figures instead of closing.
    const openDate =
      activeCell && popoverEl && !popoverEl.hidden ? activeCell.dataset.date : null;
    gridEl.innerHTML = weekRows(leadingBlanks(data.first_weekday), days, weekCellHtml);
    activeCell = null;
    if (focusedDate) gridEl.querySelector(`[data-date="${focusedDate}"]`)?.focus();
    const reopened = openDate ? gridEl.querySelector(`[data-date="${openDate}"]`) : null;
    if (reopened) showPopover(reopened);
    else hidePopover();

    const pnlEl = document.getElementById("calendarMonthPnl");
    if (pnlEl) {
      const mp = data.month_net_pnl_native || 0;
      pnlEl.textContent = Utils.formatSignedSol(mp);
      pnlEl.className = `calendar-summary-value ${PNL_CLASSES[Utils.signedTone(mp)]}`;
    }
    const setCount = (id, value) => {
      const el = document.getElementById(id);
      if (el) el.textContent = Utils.formatNumber(value ?? 0, 0);
    };
    setCount("calendarMonthTrades", data.month_trades);
    setCount("calendarWinDays", data.month_win_days);
    setCount("calendarLossDays", data.month_loss_days);
    const bestEl = document.getElementById("calendarBestDay");
    if (bestEl) {
      const best = data.best_day;
      bestEl.className = `calendar-summary-value ${best ? "profit" : ""}`;
      bestEl.innerHTML = best
        ? `<span class="calendar-best-date">${Utils.escapeHtml(dayDate(best.date))}</span> ` +
          Utils.escapeHtml(Utils.formatSignedSol(best.net_pnl_native))
        : "—";
    }
  }

  // "Mon, Oct 5" in UTC, the calendar's own day.
  function dayDate(date, weekday = false) {
    return Utils.formatDate(new Date(`${date}T00:00:00Z`), {
      includeYear: false,
      weekday,
      utc: true,
      calendar: GRID_CALENDAR,
    });
  }

  // The accessible name of a day cell: its date, and its P&L when it traded.
  function dayLabel(d) {
    const date = dayDate(d.date, true);
    return d.has_data
      ? I18n.t("home-calendar-day-label", {
          date,
          pnl: Utils.formatSignedSol(d.net_pnl_native || 0),
        })
      : date;
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
    hidePopover(); // its day is not in the month about to show
    renderSkeleton(); // show the new month's structure immediately
    load();
  }

  function updateNavState() {
    const nextBtn = document.getElementById("calendarNext");
    if (nextBtn) nextBtn.disabled = isCurrentMonth();
  }

  // ===== Day detail popover (hover, keyboard focus or tap) =====

  function ensurePopover() {
    if (popoverEl) return popoverEl;
    popoverEl = document.createElement("div");
    popoverEl.className = "cal-day-popover";
    popoverEl.id = POPOVER_ID;
    popoverEl.setAttribute("role", "tooltip");
    popoverEl.hidden = true;
    document.body.appendChild(popoverEl);
    return popoverEl;
  }

  function popoverRow(label, value, cls = "") {
    return (
      `<div class="cal-pop-row"><span class="cal-pop-label">${Utils.escapeHtml(label)}</span>` +
      `<span class="cal-pop-val ${cls}">${value}</span></div>`
    );
  }

  function buildPopoverHtml(d) {
    const pnl = d.net_pnl_native || 0;
    const pnlCls = POPOVER_PNL_CLASSES[Utils.signedTone(pnl)];
    const dt = new Date(`${d.date}T00:00:00Z`);
    const trades = d.trades || 0;
    const wins = d.wins || 0;
    const losses = Math.max(0, trades - wins);
    const winRate = trades > 0 ? Math.round((wins / trades) * 100) : 0;

    const rows = d.has_data
      ? [
          popoverRow(I18n.t("home-calendar-pop-net-pnl"), Utils.formatSignedSol(pnl), pnlCls),
          popoverRow(I18n.t("home-calendar-trades"), String(trades)),
          popoverRow(
            I18n.t("home-calendar-pop-win-rate"),
            I18n.t("home-calendar-pop-win-rate-value", {
              rate: Utils.formatPercentValue(winRate, { decimals: 0, includeSign: false }),
              wins,
              losses,
            })
          ),
        ]
      : [popoverRow(I18n.t("home-calendar-trades"), Utils.formatNumber(0, { decimals: 0 }))];
    if (d.profit_native) {
      rows.push(
        popoverRow(
          I18n.t("home-calendar-pop-gross-profit"),
          Utils.formatSignedSol(d.profit_native),
          POPOVER_PNL_CLASSES[Utils.signedTone(d.profit_native)]
        )
      );
    }
    if (d.loss_native) {
      const loss = -Math.abs(d.loss_native);
      rows.push(
        popoverRow(
          I18n.t("home-calendar-pop-gross-loss"),
          Utils.formatSignedSol(loss),
          POPOVER_PNL_CLASSES[Utils.signedTone(loss)]
        )
      );
    }
    if (d.portfolio_value_native != null) {
      rows.push(popoverRow(I18n.t("home-calendar-pop-end-balance"), Utils.formatSol(d.portfolio_value_native)));
    }

    return (
      `<div class="cal-pop-head"><span class="cal-pop-date">${dayDate(d.date, true)}</span>` +
      `<span class="cal-pop-year">${dt.getUTCFullYear()}</span></div>` +
      `<div class="cal-pop-body">${rows.join("")}</div>`
    );
  }

  // Above the cell when it fits below the weekday row, otherwise below it, so the
  // popover never covers the month navigation; clamped to the viewport.
  function positionPopover(cell) {
    const el = ensurePopover();
    const m = 8;
    const vw = document.documentElement.clientWidth;
    const vh = document.documentElement.clientHeight;
    const rect = cell.getBoundingClientRect();
    const ceiling = Math.max(m, grid()?.getBoundingClientRect().top ?? m);
    const pw = el.offsetWidth;
    const ph = el.offsetHeight;

    let top = rect.top - ph - m;
    if (top < ceiling) top = rect.bottom + m;
    if (top + ph > vh - m) top = Math.max(m, vh - ph - m);

    let left = rect.left + rect.width / 2 - pw / 2;
    left = Math.max(m, Math.min(left, vw - pw - m));

    el.style.top = `${Math.round(top)}px`;
    el.style.left = `${Math.round(left)}px`;
  }

  function showPopover(cell) {
    const day = dayMap.get(cell.dataset.date);
    if (!day) return;
    if (activeCell && activeCell !== cell) activeCell.removeAttribute("aria-describedby");
    activeCell = cell;
    cell.setAttribute("aria-describedby", POPOVER_ID);
    const el = ensurePopover();
    const wasVisible = !el.hidden;
    el.innerHTML = buildPopoverHtml(day);
    el.hidden = false;
    positionPopover(cell);
    if (wasVisible) return;
    // Force reflow so the entrance transition runs, then reveal.
    void el.offsetWidth;
    el.classList.add("cal-day-popover--visible");
  }

  function hidePopover() {
    activeCell?.removeAttribute("aria-describedby");
    activeCell = null;
    if (!popoverEl) return;
    popoverEl.classList.remove("cal-day-popover--visible");
    popoverEl.hidden = true;
  }

  const detailCell = (e) => e.target.closest(".calendar-cell[data-date]");

  function onGridOver(e) {
    const cell = detailCell(e);
    if (cell) showPopover(cell);
  }

  function onGridOut(e) {
    const cell = detailCell(e);
    if (!cell) return;
    // Ignore moves that stay inside the same cell, and keep a focused cell's popover.
    if (e.relatedTarget && cell.contains(e.relatedTarget)) return;
    if (cell === document.activeElement) return;
    hidePopover();
  }

  function onGridFocusIn(e) {
    const cell = detailCell(e);
    if (cell) showPopover(cell);
  }

  function onGridFocusOut(e) {
    if (!grid()?.contains(e.relatedTarget)) hidePopover();
  }

  // A tap opens the popover (on a touch screen it is the only way to); tapping
  // anywhere else blurs the cell, which closes it.
  function onGridClick(e) {
    const cell = detailCell(e);
    if (cell) showPopover(cell);
  }

  function onGridKey(e) {
    if (e.key === "Escape" && activeCell) {
      e.stopPropagation();
      hidePopover();
    }
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
      // Delegated on the grid container (survives per-render innerHTML swaps).
      const gridEl = grid();
      track(gridEl, "mouseover", onGridOver);
      track(gridEl, "mouseout", onGridOut);
      track(gridEl, "focusin", onGridFocusIn);
      track(gridEl, "focusout", onGridFocusOut);
      track(gridEl, "click", onGridClick);
      track(gridEl, "keydown", onGridKey);

      document.querySelectorAll(".calendar-weekdays span:not(.calendar-week-head)").forEach((el, index) => {
        el.textContent = formatWeekday((firstDayOfWeek() + index) % DAYS_PER_WEEK);
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
      hidePopover();
      dayMap.clear();
      if (popoverEl) {
        popoverEl.remove();
        popoverEl = null;
      }
    },
  };
}
