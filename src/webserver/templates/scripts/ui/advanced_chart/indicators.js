/**
 * Chart Indicators Module
 * Technical indicator calculations for AdvancedChart.
 *
 * Only the indicators the dashboard actually offers live here (EMA 9/21 on the
 * position chart). Adding one means adding its toggle at the same time — an
 * indicator with no surface is dead weight that silently rots.
 *
 * Exports:
 * - window.ChartIndicators: Object containing all indicator calculation functions
 */

(function () {
  "use strict";

  // ==========================================================================
  // INDICATOR CALCULATIONS
  // ==========================================================================

  const Indicators = {
    /**
     * Exponential Moving Average, seeded with the SMA of the first `period`
     * closes. Bars before the series can be seeded carry a null value so the
     * caller can drop them rather than draw a fabricated early average.
     * @param {Array} data - Bars with { time, close }
     * @param {number} period
     * @returns {Array} [{ time, value|null }]
     */
    ema(data, period) {
      const result = [];
      const multiplier = 2 / (period + 1);
      let ema = null;

      for (let i = 0; i < data.length; i++) {
        if (i < period - 1) {
          result.push({ time: data[i].time, value: null });
          continue;
        }

        if (ema === null) {
          // Initialize with SMA
          let sum = 0;
          for (let j = 0; j < period; j++) {
            sum += data[i - j].close;
          }
          ema = sum / period;
        } else {
          ema = (data[i].close - ema) * multiplier + ema;
        }

        result.push({ time: data[i].time, value: ema });
      }
      return result;
    },
  };

  // ==========================================================================
  // EXPORTS
  // ==========================================================================

  // Export to window for use by AdvancedChart
  window.ChartIndicators = Indicators;
})();
