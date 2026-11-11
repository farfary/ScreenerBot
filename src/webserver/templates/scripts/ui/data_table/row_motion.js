// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Row motion for DataTable: how rows enter, leave and move when live data changes the body.
 *
 * A live render (`renderOptions.live`: rows arrived from a poll or `setData`, not a sort,
 * filter or view switch by the viewer) that adds, removes or reorders a few rows animates
 * them. A row that moved slides from where it was drawn (FLIP), a new row fades in under a
 * short arrival wash, and a removed row fades where it stands and then closes its gap so the
 * rows below slide up. Bulk replacements, hidden tables and reduced motion change at once.
 *
 * Rows keep their DOM nodes: placement moves only the rows outside the longest run that kept
 * its order, so a live update never detaches the whole body. A row that leaves while another
 * with the same `rowContinuityKey` arrives is the same subject under a new id (a pending buy
 * that became a position) and keeps its node, so it changes in place instead of fading out
 * beside its successor.
 *
 * The row the viewer is about to click never moves. A pointer guards its row while it is
 * moving (it moved within `POINTER_AIM_MS`) or rests on a control in the body (a row action,
 * a link, a checkbox). A live render that would insert, remove or reorder a row at or above a
 * guarding pointer is held, while the cells of the rows on screen keep updating; closing the
 * gap of a row removed above it waits the same way. The held change lands, with its motion,
 * as soon as the pointer stops guarding or leaves the table, so a pointer parked over the
 * rows never hides new ones.
 */

/** A live change larger than this many entering or leaving rows is applied without motion. */
const MOTION_MAX_CHANGED_ROWS = 6;
/** Bodies longer than this are never measured for motion. */
const MOTION_MAX_BODY_ROWS = 1000;
/** A pointer that moved within this long is aiming at a row; one at rest longer is reading. */
const POINTER_AIM_MS = 1500;
/** How often a held change looks again whether the pointer still guards its row. */
const HELD_RECHECK_MS = 400;
/** Controls in the body a resting pointer may be about to press. */
const ROW_CONTROL_SELECTOR = "button, a[href], [role='button'], input, select, label";
/**
 * Slides follow the theme's `--transition-normal` (300ms ease). A curve with a steeper
 * start covers half the distance in the first frames, so one late frame after a render
 * reads as a jump instead of a slide.
 */
const MOVE_MS = 300;
const MOVE_EASING = "ease";
const ENTER_MS = 220;
const EXIT_MS = 220;
const ARRIVAL_MS = 1600;

/** Running slide of each row, cancelled before the row is measured again. */
const moves = new WeakMap();
/** Running fade of each leaving row, cancelled when the row comes back. */
const exits = new WeakMap();

/** A row fading out or holding its gap: no longer part of the table's rows. */
export const isLeavingRow = (tr) => tr.classList.contains("dt-row-leaving");

/**
 * Positions in `sequence` that form its longest strictly increasing run of non-negative
 * values. `sequence[i]` is the old index of the row placed at `i`, or -1 for a new row; the
 * rows of that run keep their place and every other row is moved.
 * @param {number[]} sequence
 * @returns {Set<number>}
 */
export function longestIncreasingRun(sequence) {
  const tails = [];
  const previous = new Array(sequence.length).fill(-1);
  for (let index = 0; index < sequence.length; index += 1) {
    const value = sequence[index];
    if (value < 0) continue;
    let low = 0;
    let high = tails.length;
    while (low < high) {
      const middle = (low + high) >> 1;
      if (sequence[tails[middle]] < value) low = middle + 1;
      else high = middle;
    }
    if (low > 0) previous[index] = tails[low - 1];
    tails[low] = index;
  }
  const kept = new Set();
  let at = tails.length > 0 ? tails[tails.length - 1] : -1;
  while (at >= 0) {
    kept.add(at);
    at = previous[at];
  }
  return kept;
}

export function applyRowMotionMixin(DataTable) {
  const proto = DataTable.prototype;

  /** The rows that belong to the table, in DOM order (leaving rows excluded). */
  proto._liveRows = function () {
    const tbody = this.elements?.tbody;
    if (!tbody) return [];
    return Array.from(tbody.children).filter(
      (tr) => tr.hasAttribute("data-row-id") && !isLeavingRow(tr)
    );
  };

  /** Can rows animate right now: enabled, motion allowed, table laid out and on screen. */
  proto._motionEnabled = function () {
    if (this.options.rowMotion === false || this._prefersReducedMotion()) return false;
    if (typeof document !== "undefined" && document.hidden) return false;
    const tbody = this.elements?.tbody;
    if (!tbody || tbody.children.length > MOTION_MAX_BODY_ROWS) return false;
    return tbody.getClientRects().length > 0;
  };

  /** Viewport top of every row on screen or within one screen of it, for the slides. */
  proto._captureRowTops = function () {
    const tops = new Map();
    const view = this.elements?.scrollContainer?.getBoundingClientRect();
    const margin = view ? view.height : 0;
    for (const tr of this._liveRows()) {
      const rect = tr.getBoundingClientRect();
      if (view && (rect.bottom < view.top - margin || rect.top > view.bottom + margin)) continue;
      tops.set(tr, rect.top);
    }
    return tops;
  };

  /** The `rowContinuityKey` of a row, or null when the table has none. */
  proto._continuityKey = function (row) {
    const fn = this.options.rowContinuityKey;
    if (typeof fn !== "function") return null;
    try {
      const key = fn(row);
      return key === null || key === undefined || key === "" ? null : String(key);
    } catch (error) {
      this._log("error", "rowContinuityKey function failed", error);
      return null;
    }
  };

  /** Keep `data-dt-continuity` on a row in step with its data. */
  proto._syncRowContinuity = function (tr, row) {
    const key = this._continuityKey(row);
    if (key === null) tr.removeAttribute("data-dt-continuity");
    else if (tr.getAttribute("data-dt-continuity") !== key) {
      tr.setAttribute("data-dt-continuity", key);
    }
  };

  /**
   * Put `desired` (the rows in their new order) into the body with the fewest moves. Rows of
   * the longest run that kept its order stay where they are, leaving rows stay beside their
   * old neighbours, and only the rest are inserted.
   */
  proto._placeRows = function (desired) {
    const tbody = this.elements.tbody;
    const oldIndex = new Map(this._liveRows().map((tr, index) => [tr, index]));
    const kept = longestIncreasingRun(desired.map((tr) => oldIndex.get(tr) ?? -1));
    let anchor = null;
    for (let index = desired.length - 1; index >= 0; index -= 1) {
      const tr = desired[index];
      if (!kept.has(index)) tbody.insertBefore(tr, anchor);
      anchor = tr;
    }
  };

  /**
   * Take a row out of the table. With motion it fades where it stands, then its gap closes;
   * without, it is removed at once.
   */
  proto._removeRow = function (tr, animate) {
    if (!animate) {
      tr.remove();
      return;
    }
    const id = tr.getAttribute("data-row-id");
    tr.removeAttribute("data-row-id");
    if (id !== null) tr.setAttribute("data-dt-leaving-id", id);
    tr.setAttribute("aria-hidden", "true");
    tr.classList.add("dt-row-leaving");
    const fade = tr.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: EXIT_MS,
      easing: "ease-in",
      fill: "forwards",
    });
    exits.set(tr, fade);
    fade.finished.then(
      () => this._closeRowGap(tr),
      () => {}
    );
  };

  /** A leaving row whose id is back in the data returns to the table instead of a twin. */
  proto._reviveRow = function (tr, id) {
    exits.get(tr)?.cancel();
    exits.delete(tr);
    this._heldGaps?.delete(tr);
    tr.classList.remove("dt-row-leaving");
    tr.removeAttribute("data-dt-leaving-id");
    tr.removeAttribute("aria-hidden");
    tr.setAttribute("data-row-id", id);
  };

  /** Leaving rows of the body by the id they had. */
  proto._leavingRowsById = function () {
    const rows = new Map();
    const tbody = this.elements?.tbody;
    if (!tbody) return rows;
    for (const tr of tbody.children) {
      if (isLeavingRow(tr)) rows.set(tr.getAttribute("data-dt-leaving-id"), tr);
    }
    return rows;
  };

  /** Is the pointer aiming at a row, or resting on a control in the body? */
  proto._pointerGuardsRow = function () {
    if (!this._isHovering || !Number.isFinite(this._pointerY)) return false;
    if (performance.now() - this._pointerMovedAt < POINTER_AIM_MS) return true;
    if (!Number.isFinite(this._pointerX)) return false;
    const target = document.elementFromPoint(this._pointerX, this._pointerY);
    return Boolean(target?.closest(ROW_CONTROL_SELECTOR) && this.elements?.tbody?.contains(target));
  };

  /** Does the pointer guard a row at or below `tr`, so closing its gap would move it? */
  proto._pointerAtOrBelow = function (tr) {
    return this._pointerGuardsRow() && this._pointerY >= tr.getBoundingClientRect().top;
  };

  proto._closeRowGap = function (tr) {
    if (!tr.isConnected || !isLeavingRow(tr)) return;
    if (this._pointerAtOrBelow(tr)) {
      (this._heldGaps ??= new Set()).add(tr);
      this._scheduleHeldMotionRelease();
      return;
    }
    this._closeGaps([tr]);
  };

  /** Remove faded rows; the rows below slide up into the space they left. */
  proto._closeGaps = function (rows) {
    const connected = rows.filter((tr) => tr.isConnected);
    if (connected.length === 0) return;
    const tops = this._motionEnabled() ? this._captureRowTops() : null;
    connected.forEach((tr) => {
      exits.delete(tr);
      tr.remove();
    });
    if (tops) this._playMoves(tops);
  };

  /** Slide every measured row from the top it had to where layout put it. */
  proto._playMoves = function (tops) {
    tops.forEach((_, tr) => moves.get(tr)?.cancel());
    const slides = [];
    tops.forEach((top, tr) => {
      if (!tr.isConnected || isLeavingRow(tr)) return;
      const delta = top - tr.getBoundingClientRect().top;
      if (Math.abs(delta) >= 1) slides.push([tr, delta]);
    });
    for (const [tr, delta] of slides) {
      moves.set(
        tr,
        tr.animate([{ translate: `0 ${delta}px` }, { translate: "0 0" }], {
          duration: MOVE_MS,
          easing: MOVE_EASING,
        })
      );
    }
  };

  /** Wash a row that just arrived, fading to its own surface (`--dt-row-arrival`). */
  proto._playArrival = function (tr) {
    const wash = getComputedStyle(tr).getPropertyValue("--dt-row-arrival").trim();
    if (!wash) return;
    tr.animate([{ "--dt-row-wash": wash, offset: 0 }], {
      duration: ARRIVAL_MS,
      easing: "ease-out",
    });
  };

  /** Play what the last body update planned: slides, new rows fading in, arrivals. */
  proto._playRowMotion = function () {
    const plan = this._rowMotion;
    this._rowMotion = null;
    if (!plan) return;
    this._playMoves(plan.tops);
    plan.entered.forEach((tr) => {
      tr.animate([{ opacity: 0 }, { opacity: 1 }], { duration: ENTER_MS, easing: "ease-out" });
      this._playArrival(tr);
    });
    plan.arrived.forEach((tr) => this._playArrival(tr));
  };

  /** Will a live update to `rows` change any row at or above the pointer? */
  proto._movesPointerRow = function (rows) {
    if (!this._pointerGuardsRow()) return false;
    const live = this._liveRows();
    for (let index = 0; index < live.length; index += 1) {
      const tr = live[index];
      if (tr.getBoundingClientRect().top > this._pointerY) return false;
      const row = rows[index];
      if (row === undefined) return true;
      if (tr.getAttribute("data-row-id") === String(this._getRowId(row, index))) continue;
      const key = this._continuityKey(row);
      if (key === null || tr.getAttribute("data-dt-continuity") !== key) return true;
    }
    return false;
  };

  /**
   * Hold a render while the pointer guards its row or a control is busy, keeping the cells
   * of the rows on screen current so a held update never freezes the values.
   */
  proto._holdRender = function (renderOptions, rows) {
    this._pendingRenderOptions = { ...(this._pendingRenderOptions || {}), ...renderOptions };
    this._scheduleHeldMotionRelease();
    const byId = new Map();
    rows.forEach((row, index) => {
      const rowId = this._getRowId(row, index);
      byId.set(String(rowId), { row, rowId });
    });
    const visibleColumns = this._getOrderedColumns();
    for (const tr of this._liveRows()) {
      const entry = byId.get(tr.getAttribute("data-row-id"));
      if (entry) this._updateRowCells(tr, entry.row, visibleColumns, entry.rowId);
    }
  };

  /** Look again shortly whether what is held may land; it lands once nothing guards it. */
  proto._scheduleHeldMotionRelease = function () {
    if (this._heldMotionTimer) return;
    this._heldMotionTimer = setTimeout(() => this._releaseHeldMotion(), HELD_RECHECK_MS);
  };

  proto._releaseHeldMotion = function () {
    this._heldMotionTimer = null;
    if (!this._pendingRenderOptions && !this._heldGaps?.size) return;
    if (this._pointerGuardsRow() || this._isEditingOrMenuOpen()) {
      this._scheduleHeldMotionRelease();
      return;
    }
    this._flushHeldMotion();
  };

  /** Apply the held render, then close the held gaps. */
  proto._flushHeldMotion = function () {
    const renderOptions = this._pendingRenderOptions;
    this._pendingRenderOptions = null;
    if (renderOptions) this._renderTable(renderOptions);
    const gaps = this._heldGaps;
    this._heldGaps = null;
    if (gaps?.size) this._closeGaps([...gaps]);
  };

  /** End all row motion now: leaving rows are removed and held gaps closed without a slide. */
  proto._settleRowMotion = function () {
    this._resetRowMotion();
    const tbody = this.elements?.tbody;
    if (!tbody) return;
    for (const tr of Array.from(tbody.children)) {
      if (isLeavingRow(tr)) tr.remove();
    }
  };

  /** Forget motion state that refers to rows of a body that was replaced. */
  proto._resetRowMotion = function () {
    this._rowMotion = null;
    this._heldGaps = null;
    clearTimeout(this._heldMotionTimer);
    this._heldMotionTimer = null;
  };

  /** How many entering or leaving rows a live change may have and still animate. */
  proto._motionChangeFits = function (entering, leaving) {
    return entering <= MOTION_MAX_CHANGED_ROWS && leaving <= MOTION_MAX_CHANGED_ROWS;
  };
}
