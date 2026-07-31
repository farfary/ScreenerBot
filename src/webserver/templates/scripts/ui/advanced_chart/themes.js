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
      tooltipBackground: "#161b22",
      indicatorColors: {
        ema9: "#f59e0b",
        ema21: "#8b5cf6",
      },
      positionColors: {
        entry: "#3fb950",
        exit: "#f85149",
        dca: "#f59e0b",
        avgEntry: "#58a6ff",
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
      tooltipBackground: "#ffffff",
      indicatorColors: {
        ema9: "#d97706",
        ema21: "#7c3aed",
      },
      positionColors: {
        entry: "#059669",
        exit: "#dc2626",
        dca: "#d97706",
        avgEntry: "#1565c0",
      },
    },
  };

  // ==========================================================================
  // EXPORTS
  // ==========================================================================

  // Export to window for use by AdvancedChart
  window.ChartThemes = { CHART_THEMES };
})();
