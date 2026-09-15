/**
 * ChartFraming - where an AdvancedChart puts its view, as pure functions.
 *
 * Two frames exist. "Latest" shows the newest candles at a fixed candle width with part of the
 * pane left empty after them, so the live edge has room to grow. "Range" shows a span of time
 * (a position's lifetime) and falls back to latest when the span is on the live edge and fits,
 * or when no loaded candle lies inside it at all.
 *
 * The empty space after the newest candle exists ONLY when the view reaches the newest candle.
 * A frame that ends in the past shows the real candles that followed, never invented whitespace.
 *
 * Loaded as a classic script before advanced_chart.js; exported on window.ChartFraming and,
 * for the Node tests, on module.exports.
 */

(function (root) {
  "use strict";

  /** Share of the pane left empty to the right of the newest candle. */
  const RIGHT_MARGIN = 0.2;

  /**
   * Floor on how few bars a range frame may show. A logical range derives candle WIDTH from the
   * span, so framing a position that lived three candles would stretch those three into
   * pane-wide slabs.
   */
  const MIN_FRAME_BARS = 40;

  /**
   * The latest view for a plot `width` pixels wide. `rightOffset` is counted in bar SLOTS, so it
   * only means RIGHT_MARGIN alongside the `barSpacing` it came from, and must be re-derived when
   * the pane is resized.
   */
  function latestView(width, barSpacing) {
    const plot = width > 0 ? width : 0;
    return { barSpacing, rightOffset: (plot * RIGHT_MARGIN) / barSpacing };
  }

  /**
   * Plan the frame for the span [from, to] over ascending bar times.
   *
   * @param {Object} input
   * @param {number[]} input.times - ascending bar open times, unix seconds
   * @param {number} input.from - span start, unix seconds
   * @param {number} input.to - span end, unix seconds
   * @param {number} input.width - plot width in pixels
   * @param {number} input.barSpacing - the chart's default candle width in pixels
   * @param {number} [input.barSeconds] - candle duration, so a candle that opened before `from`
   *   but contains it still counts as inside the span
   * @returns {{mode: "latest"} | {mode: "logical", from: number, to: number} | null}
   */
  function planRangeFrame({ times, from, to, width, barSpacing, barSeconds = 0 }) {
    const count = times?.length || 0;
    if (!count) return null;
    const last = count - 1;

    let start = -1;
    for (let i = 0; i < count; i++) {
      if (times[i] + barSeconds > from) {
        start = i;
        break;
      }
    }
    let end = -1;
    for (let i = last; i >= 0; i--) {
      if (times[i] <= to) {
        end = i;
        break;
      }
    }
    // No loaded candle lies inside the span (it is older than stored depth on this timeframe, or
    // sits in a gap): framing it would show the edge of unrelated data, so show what is current.
    if (start < 0 || end < start) return { mode: "latest" };

    const pad = Math.max(2, Math.round((end - start + 1) * 0.08));
    let lo = start - pad;
    let hi = end + pad;

    const shortfall = MIN_FRAME_BARS - (hi - lo + 1);
    if (shortfall > 0) {
      lo -= Math.ceil(shortfall / 2);
      hi += Math.ceil(shortfall / 2);
    }
    // Before the first loaded candle there is nothing to show: move that room to the right.
    if (lo < 0) {
      hi -= lo;
      lo = 0;
    }

    if (hi < last) return { mode: "logical", from: lo, to: hi };

    // The frame reaches the newest candle, so nothing real follows it: move the overshoot to the
    // left as context and leave RIGHT_MARGIN empty instead.
    lo = Math.max(0, lo - (hi - last));
    const visible = last - lo + 1;
    // Everything fits at the normal candle width: that IS the latest view.
    if (visible * barSpacing <= width * (1 - RIGHT_MARGIN)) return { mode: "latest" };
    return {
      mode: "logical",
      from: lo,
      to: last + (visible * RIGHT_MARGIN) / (1 - RIGHT_MARGIN),
    };
  }

  /**
   * Whether a reference price belongs in the price scale for the visible time range. A line is
   * relevant only while the view overlaps the window it describes: a closed position's average
   * entry from two months ago squashed today's candles into the bottom of the pane.
   * @param {{from: number, to: number}} window - unix seconds, either end may be infinite
   * @param {{from: number, to: number} | null} visible - the chart's visible time range
   */
  function referenceInView(window, visible) {
    if (!visible) return false;
    return window.from <= visible.to && window.to >= visible.from;
  }

  const api = { RIGHT_MARGIN, MIN_FRAME_BARS, latestView, planRangeFrame, referenceInView };
  if (typeof module === "object" && module.exports) module.exports = api;
  if (root) root.ChartFraming = api;
})(typeof window !== "undefined" ? window : null);
