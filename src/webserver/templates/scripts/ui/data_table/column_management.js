/**
 * Column Management Mixin for DataTable
 * Handles column width calculations, resizing, and auto-sizing
 */

export function applyColumnManagementMixin(DataTable) {
  const proto = DataTable.prototype;

  proto._getColumnConfig = function (columnId) {
    return this.options.columns.find((col) => col.id === columnId);
  };

  proto._getColumnMinWidth = function (columnId) {
    const column = this._getColumnConfig(columnId);
    if (!column) {
      return 80;
    }
    if (typeof column.minWidth === "number" && column.minWidth >= 0) {
      return column.minWidth;
    }
    return 80;
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

    this._autoSizeColumnsFromContent();
    this._snapshotColumnWidths();
    this._applyStoredColumnWidths();

    // Fit ONCE (prevents double-fitting); the flag is set only when the fit
    // actually measured something, so a deferred fit stays pending.
    if (this.options.fitToContainer !== false && !this.state.hasAutoFitted) {
      this.state.hasAutoFitted = this._fitColumnsToContainer() === true;
    }
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

      const headerCell = this.elements.thead.querySelector(`th[data-column-id="${columnId}"]`);

      let maxWidth = headerCell ? Math.ceil(headerCell.scrollWidth) : 0;

      sampleRows.forEach((row) => {
        const cell = row.querySelector(`td[data-column-id="${columnId}"]`);
        if (!cell) {
          return;
        }
        const cellWidth = Math.ceil(cell.scrollWidth);
        if (cellWidth > maxWidth) {
          maxWidth = cellWidth;
        }
      });

      if (maxWidth === 0 && headerCell) {
        maxWidth = Math.ceil(headerCell.offsetWidth);
      }

      const minWidth = this._getColumnMinWidth(columnId);
      const maxWidthLimit = this._getColumnMaxWidth(columnId);
      let finalWidth = Math.max(minWidth, maxWidth + padding);
      const previous = this.state.columnWidths[columnId];

      if (Number.isFinite(previous)) {
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
      // Snap to target to avoid 1px rounding horizontal scrollbars
      this.state.tableWidth = targetWidth;
      this._applyTableWidth();

      this._log("info", "Columns fitted to container", {
        originalWidth: totalWidth,
        containerWidth: targetWidth,
        resultingWidth: recomputed,
      });
    };

    // Proportional scale when overflowing
    if (totalWidth > targetWidth) {
      const scaleFactor = targetWidth / totalWidth;
      let runningTotal = 0;
      const lastIdx = visibleColumns.length - 1;

      visibleColumns.forEach((col, idx) => {
        const currentWidth = this.state.columnWidths[col.id];
        if (typeof currentWidth === "number" && !Number.isNaN(currentWidth)) {
          // Skip user-resized columns - preserve their width
          if (this.state.userResizedColumns?.[col.id]) {
            runningTotal += currentWidth;
            return;
          }

          const minWidth = this._getColumnMinWidth(col.id);
          const maxWidth = this._getColumnMaxWidth(col.id);
          // Round down to avoid overflow accumulation, we'll fix remainder on last column
          let scaled = Math.max(minWidth, Math.floor(currentWidth * scaleFactor));

          if (Number.isFinite(maxWidth)) {
            scaled = Math.min(maxWidth, scaled);
          }

          // On last column, absorb remainder so total matches targetWidth exactly (or as close as min allows)
          if (idx === lastIdx) {
            const remainder = targetWidth - runningTotal;
            // If remainder is less than minWidth, respect minWidth but it may still overflow in extreme cases
            const capped = Number.isFinite(maxWidth) ? Math.min(maxWidth, remainder) : remainder;
            scaled = Math.max(minWidth, capped);
          }

          runningTotal += scaled;
          this.state.columnWidths[col.id] = scaled;
          this._applyColumnWidth(col.id, scaled);
        }
      });

      applyFinal();
      return true;
    }

    // If under target, expand last non-user-resized column to fill remaining gap for exact fit
    if (totalWidth < targetWidth) {
      // Choose the last visible column that wasn't manually resized to absorb the gap
      let lastCol = null;
      for (let i = visibleColumns.length - 1; i >= 0; i--) {
        if (!this.state.userResizedColumns?.[visibleColumns[i].id]) {
          lastCol = visibleColumns[i];
          break;
        }
      }

      if (lastCol) {
        const currentWidth = this.state.columnWidths[lastCol.id];
        if (typeof currentWidth === "number" && !Number.isNaN(currentWidth)) {
          const gap = targetWidth - totalWidth;
          const minWidth = this._getColumnMinWidth(lastCol.id);
          const maxWidth = this._getColumnMaxWidth(lastCol.id);
          const unclamped = currentWidth + gap;
          const newWidth = Math.min(maxWidth, Math.max(minWidth, unclamped));
          this.state.columnWidths[lastCol.id] = newWidth;
          this._applyColumnWidth(lastCol.id, newWidth);
        }
      }

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
      let diff = e.pageX - startX;

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
