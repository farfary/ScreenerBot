/**
 * Chart Themes Module
 * Palette definitions for AdvancedChart (dark + light).
 *
 * Price formatting is NOT here: every price in the dashboard goes through
 * Utils.formatPriceSubscript so the axis, tooltip, headers and tables can never
 * disagree about the same number.
 *
 * Exports:
 * - window.ChartThemes: { CHART_THEMES }
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

  // ==========================================================================
  // EXPORTS
  // ==========================================================================

  // Export to window for use by AdvancedChart
  window.ChartThemes = { CHART_THEMES };
})();
