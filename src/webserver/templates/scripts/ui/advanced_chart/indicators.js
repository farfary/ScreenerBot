/**
 * Chart Indicators Module
 * Technical indicator calculations for AdvancedChart
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
     * Calculate Simple Moving Average
     */
    sma(data, period) {
      const result = [];
      for (let i = 0; i < data.length; i++) {
        if (i < period - 1) {
          result.push({ time: data[i].time, value: null });
          continue;
        }
        let sum = 0;
        for (let j = 0; j < period; j++) {
          sum += data[i - j].close;
        }
        result.push({ time: data[i].time, value: sum / period });
      }
      return result;
    },

    /**
     * Calculate Exponential Moving Average
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

    /**
     * Calculate Relative Strength Index
     */
    rsi(data, period = 14) {
      const result = [];
      const gains = [];
      const losses = [];

      for (let i = 0; i < data.length; i++) {
        if (i === 0) {
          result.push({ time: data[i].time, value: null });
          continue;
        }

        const change = data[i].close - data[i - 1].close;
        gains.push(change > 0 ? change : 0);
        losses.push(change < 0 ? Math.abs(change) : 0);

        if (i < period) {
          result.push({ time: data[i].time, value: null });
          continue;
        }

        const avgGain = gains.slice(-period).reduce((a, b) => a + b, 0) / period;
        const avgLoss = losses.slice(-period).reduce((a, b) => a + b, 0) / period;

        const rs = avgLoss === 0 ? 100 : avgGain / avgLoss;
        const rsi = 100 - 100 / (1 + rs);

        result.push({ time: data[i].time, value: rsi });
      }
      return result;
    },

    /**
     * Calculate MACD (Moving Average Convergence Divergence)
     */
    macd(data, fastPeriod = 12, slowPeriod = 26, signalPeriod = 9) {
      const fastEMA = this.ema(data, fastPeriod);
      const slowEMA = this.ema(data, slowPeriod);

      const macdLine = [];
      for (let i = 0; i < data.length; i++) {
        const fast = fastEMA[i].value;
        const slow = slowEMA[i].value;
        macdLine.push({
          time: data[i].time,
          value: fast !== null && slow !== null ? fast - slow : null,
          close: fast !== null && slow !== null ? fast - slow : 0,
        });
      }

      const signalLine = this.ema(macdLine, signalPeriod);
      const histogram = [];

      for (let i = 0; i < data.length; i++) {
        const macd = macdLine[i].value;
        const signal = signalLine[i].value;
        histogram.push({
          time: data[i].time,
          value: macd !== null && signal !== null ? macd - signal : null,
        });
      }

      return { macdLine, signalLine, histogram };
    },

    /**
     * Calculate Bollinger Bands
     */
    bollinger(data, period = 20, stdDev = 2) {
      const sma = this.sma(data, period);
      const upper = [];
      const lower = [];

      for (let i = 0; i < data.length; i++) {
        if (sma[i].value === null) {
          upper.push({ time: data[i].time, value: null });
          lower.push({ time: data[i].time, value: null });
          continue;
        }

        // Calculate standard deviation
        let sumSquared = 0;
        for (let j = 0; j < period; j++) {
          const diff = data[i - j].close - sma[i].value;
          sumSquared += diff * diff;
        }
        const std = Math.sqrt(sumSquared / period);

        upper.push({ time: data[i].time, value: sma[i].value + stdDev * std });
        lower.push({ time: data[i].time, value: sma[i].value - stdDev * std });
      }

      return { upper, middle: sma, lower };
    },
  };

  // ==========================================================================
  // EXPORTS
  // ==========================================================================

  // Export to window for use by AdvancedChart
  window.ChartIndicators = Indicators;
})();
