/**
 * Chart Formatters Module
 * Theme configurations, timeframes, and price formatting utilities for AdvancedChart
 *
 * Exports:
 * - window.ChartFormatters: Object containing formatting functions and constants
 */

(function () {
  "use strict";

  // ==========================================================================
  // CONSTANTS & THEME DEFINITIONS
  // ==========================================================================

  const CHART_THEMES = {
    dark: {
      background: "#0d1117",
      textColor: "#8b949e",
      gridColor: "#21262d",
      borderColor: "#30363d",
      crosshairColor: "#58a6ff",
      upColor: "#3fb950",
      downColor: "#f85149",
      volumeUpColor: "rgba(63, 185, 80, 0.3)",
      volumeDownColor: "rgba(248, 81, 73, 0.3)",
      wickUpColor: "#3fb950",
      wickDownColor: "#f85149",
      overlayBackground: "rgba(13, 17, 23, 0.9)",
      tooltipBackground: "#161b22",
      tooltipBorder: "#30363d",
      indicatorColors: {
        ema9: "#f59e0b",
        ema21: "#8b5cf6",
        sma50: "#06b6d4",
        sma200: "#ec4899",
        rsi: "#58a6ff",
        macdLine: "#3fb950",
        macdSignal: "#f85149",
        macdHistogramUp: "rgba(63, 185, 80, 0.5)",
        macdHistogramDown: "rgba(248, 81, 73, 0.5)",
        bollingerUpper: "rgba(88, 166, 255, 0.5)",
        bollingerLower: "rgba(88, 166, 255, 0.5)",
        bollingerMiddle: "#58a6ff",
      },
      positionColors: {
        entry: "#3fb950",
        exit: "#f85149",
        dca: "#f59e0b",
        stopLoss: "#ef4444",
        takeProfit: "#10b981",
      },
    },
    light: {
      background: "#ffffff",
      textColor: "#374151",
      gridColor: "#e5e7eb",
      borderColor: "#d1d5db",
      crosshairColor: "#1565c0",
      upColor: "#10b981",
      downColor: "#ef4444",
      volumeUpColor: "rgba(16, 185, 129, 0.3)",
      volumeDownColor: "rgba(239, 68, 68, 0.3)",
      wickUpColor: "#10b981",
      wickDownColor: "#ef4444",
      overlayBackground: "rgba(255, 255, 255, 0.95)",
      tooltipBackground: "#ffffff",
      tooltipBorder: "#d1d5db",
      indicatorColors: {
        ema9: "#d97706",
        ema21: "#7c3aed",
        sma50: "#0891b2",
        sma200: "#db2777",
        rsi: "#1565c0",
        macdLine: "#059669",
        macdSignal: "#dc2626",
        macdHistogramUp: "rgba(16, 185, 129, 0.5)",
        macdHistogramDown: "rgba(239, 68, 68, 0.5)",
        bollingerUpper: "rgba(21, 101, 192, 0.4)",
        bollingerLower: "rgba(21, 101, 192, 0.4)",
        bollingerMiddle: "#1565c0",
      },
      positionColors: {
        entry: "#059669",
        exit: "#dc2626",
        dca: "#d97706",
        stopLoss: "#dc2626",
        takeProfit: "#059669",
      },
    },
  };

  const TIMEFRAMES = {
    "1m": { label: "1M", seconds: 60 },
    "5m": { label: "5M", seconds: 300 },
    "15m": { label: "15M", seconds: 900 },
    "30m": { label: "30M", seconds: 1800 },
    "1h": { label: "1H", seconds: 3600 },
    "4h": { label: "4H", seconds: 14400 },
    "12h": { label: "12H", seconds: 43200 },
    "1d": { label: "1D", seconds: 86400 },
  };

  // ==========================================================================
  // PRICE FORMATTING UTILITIES
  // ==========================================================================

  /**
   * Format price with intelligent notation for very small numbers
   * Uses subscript notation: 0.0₉12345 means 0.000000000012345 (9 zeros after decimal)
   */
  function formatPriceSubscript(price, precision = 9) {
    if (price === 0) return "0";
    if (!Number.isFinite(price)) return "—";

    const absPrice = Math.abs(price);
    const sign = price < 0 ? "-" : "";

    // Handle normal-sized numbers (>= 0.0001)
    if (absPrice >= 0.0001) {
      const formatted = absPrice.toFixed(precision);
      return sign + formatted.replace(/\.?0+$/, "");
    }

    // Count leading zeros after decimal
    const str = absPrice.toFixed(20);
    const match = str.match(/^0\.0*/);
    if (!match) return sign + absPrice.toPrecision(precision);

    const leadingZeros = match[0].length - 2; // Subtract "0."

    // Get significant digits after zeros
    const significantPart = str.substring(match[0].length);
    const significant = significantPart.substring(0, Math.min(5, significantPart.length));

    // Use subscript for zero count
    const subscriptDigits = "₀₁₂₃₄₅₆₇₈₉";
    let subscript = "";
    const zeroStr = leadingZeros.toString();
    for (const char of zeroStr) {
      subscript += subscriptDigits[parseInt(char, 10)];
    }

    return `${sign}0.0${subscript}${significant}`;
  }

  /**
   * Format price based on magnitude for chart display
   */
  function formatPriceAuto(price, precision = 9) {
    if (price === 0) return "0";
    if (!Number.isFinite(price)) return "—";

    const absPrice = Math.abs(price);

    // Tiny prices (more than 3 leading zeros after the decimal, i.e. < 0.0001):
    // subscript notation, never scientific — "0.0₅36" reads far better than
    // "3.6000e-6" on axis labels, tooltips and legends. formatPriceSubscript
    // renders the leading-zero count as a small subscript.
    if (absPrice < 0.0001) {
      return formatPriceSubscript(price, precision);
    }

    // Normal prices
    if (absPrice < 1) {
      const formatted = price.toFixed(precision);
      return formatted.replace(/\.?0+$/, "");
    }

    // Larger prices
    if (absPrice < 1000) {
      return price.toFixed(Math.min(4, precision));
    }

    // Very large prices
    return price.toLocaleString("en-US", {
      maximumFractionDigits: 2,
    });
  }

  // ==========================================================================
  // EXPORTS
  // ==========================================================================

  // Export to window for use by AdvancedChart
  window.ChartFormatters = {
    CHART_THEMES,
    TIMEFRAMES,
    formatPriceSubscript,
    formatPriceAuto,
  };
})();
