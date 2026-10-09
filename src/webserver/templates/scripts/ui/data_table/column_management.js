// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Column Management Mixin for DataTable
 * Handles column width calculations, resizing, and auto-sizing
 */

/* global NodeFilter */

import { dirSign } from "../../core/dom.js";

/**
 * The one-line extent of a cell's content plus its inline padding and borders.
 * Text runs and inline boxes are measured, never a block wrapper, whose box
 * reports the column's width rather than what the value needs.
 */
function naturalCellWidth(cell, range) {
  let start = Infinity;
  let end = -Infinity;
  const include = (rect) => {
    if (!rect.width) return;
    start = Math.min(start, rect.left);
    end = Math.max(end, rect.right);
  };
  const walker = document.createTreeWalker(cell, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (!node.textContent.trim()) continue;
    range.selectNodeContents(node);
    Array.from(range.getClientRects()).forEach(include);
  }
  for (const element of cell.querySelectorAll("*")) {
    if (getComputedStyle(element).display.startsWith("inline")) {
      include(element.getBoundingClientRect());
    }
  }
  if (end <= start) return 0;
  const style = getComputedStyle(cell);
  const frame = [
    "paddingInlineStart",
    "paddingInlineEnd",
    "borderInlineStartWidth",
    "borderInlineEndWidth",
  ].reduce((sum, key) => sum + (parseFloat(style[key]) || 0), 0);
  return Math.ceil(end - start + frame);
}

export function applyColumnManagementMixin(DataTable) {
  const proto = DataTable.prototype;

  proto._getColumnConfig = function (columnId) {
    return this.options.columns.find((col) => col.id === columnId);
  };

  /**
   * A column's minimum width: its configured `minWidth` (80px by default), and never
   * less than its header needs. A header's text may overflow into the cell's end
   * padding without growing its scroll width, so neither content sizing nor the
   * proportional fit would see it; the measured header floor keeps every column at
   * least as wide as its label and sort mark. A column whose cells are never cut
   * (see `_measureContentFloors`) is also never narrower than its widest value.
   */
  proto._getColumnMinWidth = function (columnId) {
    const column = this._getColumnConfig(columnId);
    const floor = Math.max(
      this._headerFloors?.[columnId] || 0,
      this._contentFloors?.[columnId] || 0
    );
    if (!column) {
      return Math.max(80, floor);
    }
    if (typeof column.minWidth === "number" && column.minWidth >= 0) {
      return Math.max(column.minWidth, floor);
    }
    return Math.max(80, floor);
  };

  /**
   * Measure the widest value of every column whose cells are never cut: the
   * value-typed columns (their cells carry `data-type` and do not wrap, so an amount
   * and its unit stay on one line) and any column declared `minWidth: "content"`
   * (a short label such as a mode with its paused mark). The floor is the value's
   * one-line extent plus the cell's inline padding, so the proportional fit can never
   * squeeze such a column below what it shows.
   *
   * Before the one-time fit every sampled cell is measured precisely. After it, a
   * render (every poll) only reads whether a cell overflows: a cell that does not
   * overflow already fits, and one that does reports its full need as its scroll
   * width, since it cannot wrap. Between fits floors only grow, so a column never
   * narrows under a value it has shown.
   */
  proto._measureContentFloors = function () {
    const precise = !this.state.hasAutoFitted;
    // A refit (new columns, density, locale) measures afresh.
    const floors = precise ? {} : { ...this._contentFloors };
    const rows = Array.from(this.elements.tbody?.querySelectorAll("tr[data-row-id]") || []).slice(
      0,
      this.options.autoSizeSample
    );
    const range = precise ? document.createRange() : null;
    for (const column of this._getOrderedColumns()) {
      if (!this._isColumnVisible(column.id)) continue;
      let widest = floors[column.id] || 0;
      for (const row of rows) {
        const cell = row.querySelector(`td[data-column-id="${column.id}"]`);
        if (!cell || (column.minWidth !== "content" && !cell.hasAttribute("data-type"))) continue;
        if (precise) widest = Math.max(widest, naturalCellWidth(cell, range));
        else if (cell.scrollWidth > cell.clientWidth) widest = Math.max(widest, cell.scrollWidth);
      }
      if (widest > 0) floors[column.id] = widest;
    }
    this._contentFloors = floors;
  };

  /**
   * Widen every column that sits below its minimum. The fit to the container runs
   * once, so a floor that grows with later rows is applied here on every sizing
   * pass; a column the user resized keeps the user's width.
   */
  proto._raiseColumnsToFloors = function () {
    let raised = false;
    for (const column of this._getOrderedColumns()) {
      const width = this.state.columnWidths[column.id];
      if (typeof width !== "number" || this.state.userResizedColumns?.[column.id]) continue;
      const floor = this._getColumnMinWidth(column.id);
      if (width >= floor) continue;
      this.state.columnWidths[column.id] = floor;
      this._applyColumnWidth(column.id, floor);
      raised = true;
    }
    if (!raised) return;
    const total = this._computeTableWidthFromState();
    if (typeof total === "number" && total > (this.state.tableWidth || 0)) {
      this.state.tableWidth = total;
      this._applyTableWidth();
    }
  };

  /**
   * Measure each header's natural width: its label and sort mark at the header
   * font, plus the cell's own inline padding and borders. Read on every sizing
   * pass, so a locale change, a sort mark or a density change is reflected.
   */
  proto._measureHeaderFloors = function () {
    const floors = {};
    this.elements.thead?.querySelectorAll("th[data-column-id]").forEach((th) => {
      const content = th.querySelector(".dt-header-content");
      if (!content) return;
      // The header row and its label stretch to the cell, so their boxes report the
      // column's width, not its need: measure each part's own text extent (a Range
      // over its contents) and add the gaps between them.
      const range = document.createRange();
      const parts = [...content.children];
      const natural = parts.reduce((sum, part) => {
        if (!part.firstChild) return sum + part.getBoundingClientRect().width;
        range.selectNodeContents(part);
        return sum + range.getBoundingClientRect().width;
      }, 0);
      const gap =
        (parseFloat(getComputedStyle(content).columnGap) || 0) * Math.max(0, parts.length - 1);
      const style = getComputedStyle(th);
      const frame = [
        "paddingInlineStart",
        "paddingInlineEnd",
        "borderInlineStartWidth",
        "borderInlineEndWidth",
      ].reduce((sum, key) => sum + (parseFloat(style[key]) || 0), 0);
      floors[th.dataset.columnId] = Math.ceil(natural + gap + frame);
    });
    this._headerFloors = floors;
  };

  proto._getColumnMaxWidth = function (columnId) {
    const column = this._getColumnConfig(columnId);
    if (!column) {
      return Number.POSITIVE_INFINITY;
    }
    if (typeof column.maxWidth === "number" && column.maxWidth > 0) {
      return column.maxWidth;
    }
    return Number.POSITIVE_INFINITY;
  };

  proto._markColumnAsUserResized = function (columnId) {
    if (!columnId) {
      return;
    }
    if (!this.state.userResizedColumns) {
      this.state.userResizedColumns = {};
    }
    this.state.userResizedColumns[columnId] = true;
  };

  /**
   * Apply column width by updating the <col> element
   * This is much more efficient than updating every <td> element
   * The browser handles the column layout automatically
   */
  proto._applyColumnWidth = function (columnId, widthPx) {
    if (!columnId || !Number.isFinite(widthPx)) return;

    const minWidth = this._getColumnMinWidth(columnId);
    const maxWidth = this._getColumnMaxWidth(columnId);
    const w = Math.min(maxWidth, Math.max(minWidth, Math.round(widthPx)));

    const applyTo = (colEl) => {
      if (!colEl) return;
      colEl.style.width = `${w}px`;
      colEl.style.minWidth = `${w}px`;
      colEl.style.maxWidth = `${w}px`;
    };

    // Resolve the <col> elements LIVE from the STABLE table elements each call.
    // `this.elements.table` / `.headerTable` are created once and never replaced
    // (only their colgroup/thead contents are swapped), whereas the cached
    // `this.elements.cols` map and `headerColgroup` reference can point at a
    // DETACHED colgroup after an `outerHTML` swap. Writing a width to a detached
    // <col> silently does nothing — which is exactly how the HEADER column could
    // stay stuck at its old width while the BODY column (resolved via a freshly
    // rebuilt map) updated, desyncing the two during a live resize. Querying live
    // from the stable tables keeps header and body column widths in lockstep.
    const bodyCol =
      this.elements.table?.querySelector(`colgroup col[data-column-id="${columnId}"]`) ||
      this.elements.cols?.[columnId];
    applyTo(bodyCol);

    const headerCol = this.elements.headerTable?.querySelector(
      `colgroup col[data-column-id="${columnId}"]`
    );
    applyTo(headerCol);

    // Update header <th> too for a correct pointer-events hit-box on the handle.
    const th = this.elements.thead?.querySelector(`th[data-column-id="${columnId}"]`);
    if (th) {
      th.style.width = `${w}px`;
    }
  };

  proto._applyTableWidth = function () {
    const w = typeof this.state.tableWidth === "number" ? `${this.state.tableWidth}px` : "";
    if (this.elements.table) this.elements.table.style.width = w;
    if (this.elements.headerTable) this.elements.headerTable.style.width = w;
  };

  proto._applyStoredColumnWidths = function () {
    if (!this.elements.table) {
      return;
    }

    Object.entries(this.state.columnWidths).forEach(([columnId, width]) => {
      if (typeof width === "number" && !Number.isNaN(width)) {
        this._applyColumnWidth(columnId, width);
      }
    });

    this._applyTableWidth();
  };

  /**
   * Is the table actually laid out? A table inside a `display: none` ancestor -
   * a tab panel not yet unhidden, a page mid-route-switch - reports every width
   * as 0. Measuring it stores garbage (every column at its minimum) that the
   * oscillation guards then defend, so no measurement may be taken or recorded
   * until this is true.
   */
  proto._isLaidOut = function () {
    return (this.elements.scrollContainer?.clientWidth || 0) > 0;
  };

  /**
   * The one column-width pass: measure content, capture anything missing, write
   * it to the DOM, then fit the result to the container exactly once. Called on
   * every render, and again from the wrapper ResizeObserver when a table that
   * rendered while hidden finally gets a real width.
   */
  proto._sizeColumns = function () {
    if (!this._isLaidOut()) return;

    this._measureHeaderFloors();
    this._measureContentFloors();
    this._autoSizeColumnsFromContent();
    this._snapshotColumnWidths();
    this._applyStoredColumnWidths();

    // Fit ONCE (prevents double-fitting); the flag is set only when the fit
    // actually measured something, so a deferred fit stays pending. A table with no
    // rows yet (still loading) has measured only its headers: it fits for now and
    // fits again, against its content floors, once rows arrive.
    if (this.options.fitToContainer !== false && !this.state.hasAutoFitted) {
      const fitted = this._fitColumnsToContainer() === true;
      this.state.hasAutoFitted = fitted && this._hasMeasurableRows();
    }
    this._raiseColumnsToFloors();
  };

  proto._hasMeasurableRows = function () {
    return Boolean(this.elements.tbody?.querySelector("tr[data-row-id]"));
  };

  proto._autoSizeColumnsFromContent = function () {
    if (this.options.autoSizeColumns === false) {
      return;
    }
    if (!this.elements.thead || !this.elements.tbody) {
      return;
    }

    const visibleColumns = this._getOrderedColumns();
    if (!visibleColumns || visibleColumns.length === 0) {
      return;
    }

    // Skip if columns are locked and all have sizes
    if (this.options.lockColumnWidths && this.state.columnWidthsLocked) {
      const needsSizing = visibleColumns.some(
        (col) => typeof this.state.columnWidths[col.id] !== "number"
      );
      if (!needsSizing) {
        return;
      }
    }

    // Skip content-based sizing if we've already auto-fitted and have stable widths
    // This prevents oscillation during rapid data updates
    if (this.state.hasAutoFitted && this._allColumnsHaveWidths()) {
      return;
    }

    const allRows = Array.from(this.elements.tbody.querySelectorAll("tr[data-row-id]"));
    const sampleSize = Math.min(this.options.autoSizeSample, allRows.length);
    const sampleRows = sampleSize > 0 ? allRows.slice(0, sampleSize) : [];
    const padding = this.options.autoSizePadding;
    const range = document.createRange();

    let didChange = false;

    visibleColumns.forEach((col) => {
      const columnId = col.id;
      if (!columnId || !this._isColumnVisible(columnId)) {
        return;
      }

      const hasFixedWidth =
        col.autoWidth !== true &&
        col.width !== undefined &&
        col.width !== null &&
        !(typeof col.width === "string" && col.width.trim().toLowerCase() === "auto");

      if (hasFixedWidth) {
        const minWidth = this._getColumnMinWidth(columnId);
        const maxWidth = this._getColumnMaxWidth(columnId);
        const fixed = Math.min(maxWidth, Math.max(minWidth, Number(col.width)));
        if (
          typeof fixed === "number" &&
          !Number.isNaN(fixed) &&
          this.state.columnWidths[columnId] !== fixed
        ) {
          this.state.columnWidths[columnId] = fixed;
          this._applyColumnWidth(columnId, fixed);
          didChange = true;
        }
        return;
      }

      if (this.state.userResizedColumns?.[columnId]) {
        return;
      }

      // What the header and the values NEED, not the boxes they sit in: a cell's
      // scroll width is the column's laid-out width whenever its content is
      // narrower, so measuring it handed every column a share of the table's
      // spare width (a 50px badge in a 228px column).
      let maxWidth = this._headerFloors?.[columnId] || 0;

      sampleRows.forEach((row) => {
        const cell = row.querySelector(`td[data-column-id="${columnId}"]`);
        if (!cell) {
          return;
        }
        const cellWidth = naturalCellWidth(cell, range);
        if (cellWidth > maxWidth) {
          maxWidth = cellWidth;
        }
      });

      const minWidth = this._getColumnMinWidth(columnId);
      const maxWidthLimit = this._getColumnMaxWidth(columnId);
      let finalWidth = Math.max(minWidth, maxWidth + padding);
      const previous = this.state.columnWidths[columnId];

      // Before the one-time fit a width on record is only an earlier pass's guess (an
      // empty table's header-only layout), so the measurement replaces it; after the
      // fit, the guard below keeps polled rows from jittering the columns.
      if (Number.isFinite(previous) && this.state.hasAutoFitted) {
        // Prevent oscillation: only allow width changes if content significantly changed
        // Use a higher threshold to prevent micro-adjustments from causing visual jitter
        const growthThreshold = 4;
        const shrinkThreshold = 8;

        // Calculate the difference between new measured width and stored width
        const widthDiff = finalWidth - previous;

        if (widthDiff > growthThreshold) {
          // Content grew significantly, allow increase
          // finalWidth stays as calculated
        } else if (widthDiff < -shrinkThreshold) {
          // Content shrank significantly, but only shrink if user hasn't interacted
          // Keep previous width to prevent shrinking on data updates
          finalWidth = previous;
        } else {
          // Within threshold, keep stable
          finalWidth = previous;
        }
      }

      if (!Number.isFinite(finalWidth)) {
        return;
      }

      finalWidth = Math.min(maxWidthLimit, finalWidth);

      if (!Number.isFinite(previous) || Math.abs(previous - finalWidth) > 1) {
        this.state.columnWidths[columnId] = finalWidth;
        this._applyColumnWidth(columnId, finalWidth);
        didChange = true;
      }
    });

    if (didChange) {
      const total = this._computeTableWidthFromState();
      if (typeof total === "number") {
        this.state.tableWidth = total;
      }
      // Don't reset hasAutoFitted here - content-based sizing shouldn't trigger container fit
    }

    if (this.options.lockColumnWidths) {
      const allSized = visibleColumns.every(
        (col) => typeof this.state.columnWidths[col.id] === "number"
      );
      if (allSized) {
        this.state.columnWidthsLocked = true;
      }
    }
    // Note: fitToContainer is now called separately in _renderTable to avoid double-fitting
  };

  // Snapshot current natural widths for visible columns into state if missing
  // This function ONLY captures widths - fitting is done separately
  proto._snapshotColumnWidths = function () {
    if (!this.elements.thead) return;
    const headers = this.elements.thead.querySelectorAll("th[data-column-id]");
    headers.forEach((th) => {
      const id = th.dataset.columnId;
      if (!id) return;
      if (typeof this.state.columnWidths[id] !== "number") {
        const w = th.offsetWidth;
        if (w && !Number.isNaN(w)) this.state.columnWidths[id] = Math.round(w);
      }
    });

    // Update table width sum (fitting and DOM application done by caller)
    const sum = this._computeTableWidthFromState();
    if (typeof sum === "number") {
      this.state.tableWidth = sum;
    }
  };

  // Check if all visible columns have stored widths
  proto._allColumnsHaveWidths = function () {
    const visibleColumns = this._getOrderedColumns();
    if (!visibleColumns || visibleColumns.length === 0) return false;
    return visibleColumns.every((col) => typeof this.state.columnWidths[col.id] === "number");
  };

  proto._computeTableWidthFromState = function () {
    const cols = this._getOrderedColumns();
    if (!cols || cols.length === 0) return null;
    let total = 0;
    cols.forEach((c) => {
      if (!this._isColumnVisible(c.id)) return;
      const w = this.state.columnWidths[c.id];
      if (typeof w === "number" && !Number.isNaN(w)) total += w;
    });
    return Math.max(0, Math.round(total));
  };

  /**
   * Fit columns proportionally to container width if they would overflow.
   * Returns true only when a real measurement was taken and applied, so the
   * caller knows whether the one-shot fit is actually done.
   */
  proto._fitColumnsToContainer = function () {
    if (!this.elements.scrollContainer) return false;

    // Use clientWidth which excludes vertical scrollbar width
    const containerWidth = this.elements.scrollContainer.clientWidth;
    const totalWidth = this._computeTableWidthFromState();
    if (!totalWidth || totalWidth <= 0) return false;

    // A zero-width container is not a measurement - it means the table is
    // rendering inside a `display: none` ancestor (a tab panel that has not been
    // unhidden yet, a page mid-route-switch). Fitting against it scales every
    // column to its minimum and pins `table.style.width` to 0px, which the
    // stylesheet's `width: 100%` cannot undo; the table then stays collapsed for
    // the rest of its life. Defer instead: the caller leaves the fit pending and
    // the next render, or the wrapper ResizeObserver, redoes it with real widths.
    if (containerWidth <= 0) return false;

    const visibleColumns = this._getOrderedColumns();
    if (!visibleColumns || visibleColumns.length === 0) return false;

    // We always attempt to match the container exactly on init-fit
    const targetWidth = Math.max(0, Math.floor(containerWidth));

    // Helper to apply final widths and snap table width to target
    const applyFinal = () => {
      // After adjusting individual columns, recompute and set table width
      const recomputed = this._computeTableWidthFromState();
      // Snap to target to avoid 1px rounding horizontal scrollbars; minimums that
      // exceed the container keep their sum, and the table scrolls.
      this.state.tableWidth = Math.max(targetWidth, recomputed ?? 0);
      this._applyTableWidth();

      this._log("info", "Columns fitted to container", {
        originalWidth: totalWidth,
        containerWidth: targetWidth,
        resultingWidth: recomputed,
      });
    };

    // Overflow: every column gives up width in proportion to what it holds above its
    // minimum, so a column already at its minimum never pushes the rest further out.
    // The table overflows (and scrolls) only by what the minimums themselves exceed.
    if (totalWidth > targetWidth) {
      const shrinkable = visibleColumns
        .filter(
          (col) =>
            !this.state.userResizedColumns?.[col.id] &&
            Number.isFinite(this.state.columnWidths[col.id])
        )
        .map((col) => {
          const width = this.state.columnWidths[col.id];
          return { col, width, slack: Math.max(0, width - this._getColumnMinWidth(col.id)) };
        })
        .filter((entry) => entry.slack > 0);
      const totalSlack = shrinkable.reduce((sum, entry) => sum + entry.slack, 0);
      let deficit = Math.min(totalWidth - targetWidth, totalSlack);
      shrinkable.forEach((entry, idx) => {
        const remainingSlack = shrinkable.slice(idx).reduce((sum, e) => sum + e.slack, 0);
        const cut =
          idx === shrinkable.length - 1
            ? deficit
            : Math.min(entry.slack, Math.ceil((deficit * entry.slack) / remainingSlack));
        deficit -= cut;
        const newWidth = entry.width - cut;
        this.state.columnWidths[entry.col.id] = newWidth;
        this._applyColumnWidth(entry.col.id, newWidth);
      });

      applyFinal();
      return true;
    }

    // Spare width goes to the name columns (`grow: true`: the token, the wallet, the
    // task), which are the only cells that read better wider; a badge or a number
    // column keeps the width its content needs. A table without a name column gives
    // the spare to its last column.
    if (totalWidth < targetWidth) {
      const adjustable = visibleColumns.filter(
        (col) =>
          !this.state.userResizedColumns?.[col.id] &&
          Number.isFinite(this.state.columnWidths[col.id])
      );
      const growing = adjustable.filter((col) => col.grow === true);
      const receivers = growing.length > 0 ? growing : adjustable.slice(-1);
      let gap = targetWidth - totalWidth;
      receivers.forEach((col, idx) => {
        const currentWidth = this.state.columnWidths[col.id];
        const share =
          idx === receivers.length - 1 ? gap : Math.floor(gap / (receivers.length - idx));
        const maxWidth = this._getColumnMaxWidth(col.id);
        const newWidth = Math.min(maxWidth, currentWidth + share);
        gap -= newWidth - currentWidth;
        this.state.columnWidths[col.id] = newWidth;
        this._applyColumnWidth(col.id, newWidth);
      });

      applyFinal();
      return true;
    }

    // Already exactly matching
    applyFinal();
    return true;
  };

  /**
   * Handle column resize drag with RAF throttling for smooth performance
   */
  proto._handleResize = function (e) {
    if (!this.resizing) return;
    e.preventDefault();

    // Throttle updates with requestAnimationFrame
    if (this._pendingRAF) return;

    this._pendingRAF = requestAnimationFrame(() => {
      this._pendingRAF = null;

      if (!this.resizing) return;

      const { columnId, startX, startWidth, minWidth } = this.resizing;

      const effectiveMin = typeof minWidth === "number" ? minWidth : 50;
      let diff = dirSign() * (e.pageX - startX);

      // Prevent shrinking beyond min width
      const maxDecrease = startWidth - effectiveMin;
      if (diff < -maxDecrease) {
        diff = -maxDecrease;
      }

      const newWidth = Math.max(effectiveMin, Math.round(startWidth + diff));
      this._markColumnAsUserResized(columnId);
      this.state.columnWidths[columnId] = newWidth;
      this._applyColumnWidth(columnId, newWidth);

      // Grow table width - don't shrink other columns
      const total = this._computeTableWidthFromState();
      if (typeof total === "number") {
        this.state.tableWidth = total;
        this._applyTableWidth();
      }

      // When the column being dragged is pinned/floating, the cumulative left
      // offsets of every pinned column to its right change continuously as its
      // width changes. Recompute them on each animation frame so the rest of the
      // pinned group tracks the drag smoothly and stays aligned, instead of being
      // stuck at the old offset (overlapping) until mouse-up.
      if (
        Array.isArray(this.state.floatingColumns) &&
        this.state.floatingColumns.includes(columnId) &&
        typeof this._updateStickyOffsets === "function"
      ) {
        this._updateStickyOffsets();
      }
    });
  };

  /**
   * Handle resize end
   *
   * Always clears `this.resizing` and tears down the document listeners, even on
   * an unexpected/duplicate call. A stuck `this.resizing` would otherwise wedge
   * `_isUserInteracting()` and block all subsequent sorting/renders, so the
   * cleanup below runs unconditionally.
   */
  proto._handleResizeEnd = function () {
    // Cancel any pending RAF
    if (this._pendingRAF) {
      cancelAnimationFrame(this._pendingRAF);
      this._pendingRAF = null;
    }

    if (this.resizing) {
      const { leftHeader, handle } = this.resizing;
      if (leftHeader) {
        leftHeader.classList.remove("dt-resizing");
      }
      if (handle) {
        handle.classList.remove("active");
      }

      this._saveState();
    }

    // Defensively clear the flag regardless of whether `this.resizing` was set,
    // so the interaction guard can never get permanently stuck.
    this.resizing = null;

    document.body.classList.remove("dt-column-resizing");
    document.removeEventListener("mousemove", this._handleResize);
    document.removeEventListener("mouseup", this._handleResizeEnd);

    // A pinned column may have been resized — recompute the cumulative left
    // offsets so subsequent pinned columns stay correctly aligned.
    if (typeof this._updateStickyOffsets === "function") {
      this._updateStickyOffsets();
    }
  };
}
