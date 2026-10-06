// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Shared dashboard utilities - formatting helpers, clipboard and common glue
// used across every page.

import {
  formatNumber,
  formatCompactNumber,
  formatBooleanFlag,
  formatCurrencyUSD,
  formatPriceSubscript,
  formatPriceSol,
  formatPercentValue,
  formatPercent,
  formatSol,
  formatSignedSol,
  formatSignedNumber,
  signedTone,
  formatPnL,
  formatTimeFromSeconds,
  formatTimestamp,
  formatDate,
  formatDatePart,
  formatCompactFixed,
  formatTimeSpan,
  formatTimeAgo,
  formatTimeUntil,
  formatUptime,
  formatBytes,
  formatDuration,
  formatSignatureCompact,
  formatAddressCompact,
  formatSecondsToTime,
  formatList,
  formatFixed,
  withPercentUnit,
  withSolUnit,
  withUsdSymbol,
} from "./format.js";

(function () {
  // Import toast manager for new toast system
  let toastManager = null;
  import("./toast.js").then((module) => {
    toastManager = module.toastManager;
  });

  // Characters that belong to the animated numeric run of a live value: digits (including
  // subscript digits), separators, signs, currency symbols, percent and the dash placeholder.
  const LIVE_NUMBER_NUMERIC = /^[\p{Nd}\p{No}\p{Sc}.,'%\u2030+\-\u2212\u066B\u066C\u2013\u2014]$/u;
  // Spaces and format marks join a numeric run only when a numeric character sits on both sides.
  const LIVE_NUMBER_NEUTRAL = /^[\s\p{Cf}]+$/u;

  /**
   * Split a formatted value into alternating runs: `digits` runs (animated per grapheme)
   * and `text` runs (a localized unit word or suffix, kept whole so its script shapes).
   */
  function liveNumberRuns(text) {
    const graphemes = I18n.graphemes(text);
    const kinds = graphemes.map((g) =>
      LIVE_NUMBER_NEUTRAL.test(g) ? null : LIVE_NUMBER_NUMERIC.test(g) ? "digits" : "text"
    );
    const resolved = kinds.map((kind, index) => {
      if (kind) return kind;
      let before = null;
      for (let i = index - 1; i >= 0 && !before; i -= 1) before = kinds[i];
      let after = null;
      for (let i = index + 1; i < kinds.length && !after; i += 1) after = kinds[i];
      return before === "digits" && after === "digits" ? "digits" : "text";
    });

    const runs = [];
    graphemes.forEach((grapheme, index) => {
      const kind = resolved[index];
      const last = runs[runs.length - 1];
      if (last && last.kind === kind) {
        last.chars.push(grapheme);
      } else {
        runs.push({ kind, chars: [grapheme] });
      }
    });
    return runs.map((run) => ({ kind: run.kind, chars: run.chars, text: run.chars.join("") }));
  }

  /** Replace only the changed characters of one digit run, keeping stable nodes in place. */
  function patchDigitRun(runElement, previousChars, nextChars, createCharacter) {
    const previousNodes = Array.from(runElement.children);
    const stable =
      previousNodes.length === previousChars.length &&
      previousNodes.every((node, index) => node.textContent === previousChars[index]);

    if (!stable) {
      const fragment = document.createDocumentFragment();
      nextChars.forEach((char) => fragment.appendChild(createCharacter(char)));
      runElement.replaceChildren(fragment);
      return;
    }

    if (previousChars.length === nextChars.length) {
      nextChars.forEach((char, index) => {
        if (previousChars[index] !== char) {
          previousNodes[index].replaceWith(createCharacter(char));
        }
      });
      return;
    }

    let sharedPrefix = 0;
    while (
      sharedPrefix < previousChars.length &&
      sharedPrefix < nextChars.length &&
      previousChars[sharedPrefix] === nextChars[sharedPrefix]
    ) {
      sharedPrefix += 1;
    }

    let sharedSuffix = 0;
    while (
      sharedSuffix < previousChars.length - sharedPrefix &&
      sharedSuffix < nextChars.length - sharedPrefix &&
      previousChars[previousChars.length - 1 - sharedSuffix] ===
        nextChars[nextChars.length - 1 - sharedSuffix]
    ) {
      sharedSuffix += 1;
    }

    const previousMiddleEnd = previousChars.length - sharedSuffix;
    const nextMiddleEnd = nextChars.length - sharedSuffix;
    const suffixAnchor = previousNodes[previousMiddleEnd] || null;

    for (let index = sharedPrefix; index < previousMiddleEnd; index += 1) {
      previousNodes[index].remove();
    }
    for (let index = sharedPrefix; index < nextMiddleEnd; index += 1) {
      runElement.insertBefore(createCharacter(nextChars[index]), suffixAnchor);
    }
  }

  /**
   * Update a live numeric label without repainting the stable characters.
   *
   * The value is split into runs. Each numeric run (digits, separators, sign, currency
   * symbol, percent) is a `.live-number-digits` LTR island holding one span per grapheme;
   * characters that are unchanged retain their DOM nodes and only changed characters are
   * replaced, which lets CSS animate the exact digits that moved. Any other text (a
   * localized unit word such as "مليار" or "लाख") is one plain `.live-number-text` run, so
   * its script joins and shapes, and the outer element keeps the page direction so number
   * and unit order follow the locale.
   *
   * @param {HTMLElement} element
   * @param {string} text - Already-formatted display value
   * @param {number|null|undefined} numericValue - Raw value used for direction
   * @param {Object} options
   * @param {boolean} options.animate
   * @returns {boolean} Whether the displayed text changed
   */
  function updateLiveNumber(element, text, numericValue, { animate = true } = {}) {
    if (!element) return false;

    const nextText = String(text ?? "—");
    const previous = element.__liveNumberState;

    element.classList.add("live-number");
    if (previous?.text === nextText) return false;

    const previousText = previous?.text || "";
    const previousRuns = previous?.runs || [];
    const nextRuns = liveNumberRuns(nextText);
    const previousValue = previous?.value;
    const nextValue =
      numericValue === null || numericValue === undefined || numericValue === ""
        ? Number.NaN
        : Number(numericValue);
    const hasDirection =
      Number.isFinite(previousValue) && Number.isFinite(nextValue) && previousValue !== nextValue;
    const direction = hasDirection ? (nextValue > previousValue ? "up" : "down") : null;
    const shouldAnimate = Boolean(animate && previousText && direction);

    const createCharacter = (char) => {
      const character = document.createElement("span");
      character.className = "live-number-char";
      character.textContent = char;
      if (shouldAnimate) {
        character.classList.add(`live-number-char--${direction}`);
      }
      return character;
    };

    const createRun = (run) => {
      const runElement = document.createElement("span");
      if (run.kind === "digits") {
        runElement.className = "live-number-digits";
        run.chars.forEach((char) => runElement.appendChild(createCharacter(char)));
      } else {
        runElement.className = "live-number-text";
        runElement.textContent = run.text;
      }
      return runElement;
    };

    // Stable nodes are reused only when the run layout is unchanged: the same kinds in
    // the same order and the same text runs. Otherwise the value is rebuilt.
    const runNodes = Array.from(element.children);
    const sameLayout =
      runNodes.length === previousRuns.length &&
      previousRuns.length === nextRuns.length &&
      nextRuns.every(
        (run, index) =>
          run.kind === previousRuns[index].kind &&
          (run.kind === "digits" || run.text === previousRuns[index].text)
      );

    if (sameLayout) {
      nextRuns.forEach((run, index) => {
        if (run.kind === "digits" && run.text !== previousRuns[index].text) {
          patchDigitRun(runNodes[index], previousRuns[index].chars, run.chars, createCharacter);
        }
      });
    } else {
      const fragment = document.createDocumentFragment();
      nextRuns.forEach((run) => fragment.appendChild(createRun(run)));
      element.replaceChildren(fragment);
    }

    element.__liveNumberState = {
      text: nextText,
      runs: nextRuns,
      value: Number.isFinite(nextValue) ? nextValue : null,
    };
    return true;
  }

  // Also escapes quotes: the result is interpolated into double- and single-quoted
  // attribute values as well as element text.
  function escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text ?? "";
    return div.innerHTML.replaceAll('"', "&quot;").replaceAll("'", "&#39;");
  }

  function setText(id, value) {
    const el = document.getElementById(id);
    if (el) {
      el.textContent = value;
    }
    return el;
  }

  function setHtml(id, value) {
    const el = document.getElementById(id);
    if (el) {
      el.innerHTML = value;
    }
    return el;
  }

  // Global external link handler for Electron
  // Intercepts clicks on external links (http/https) and routes through backend API
  // This is necessary because Electron's webview doesn't natively open external links in browser
  document.addEventListener(
    "click",
    async (e) => {
      const link = e.target.closest("a[href]");
      if (!link) return;

      const href = link.getAttribute("href");
      if (!href) return;

      // Only intercept external URLs (http/https)
      if (!href.startsWith("http://") && !href.startsWith("https://")) return;

      // Check if this is a same-origin link (internal navigation) - allow those to work normally
      try {
        const linkUrl = new URL(href, window.location.origin);
        if (linkUrl.origin === window.location.origin) return;
      } catch {
        // Invalid URL, skip
        return;
      }

      // Prevent default browser behavior and open externally via backend API
      e.preventDefault();
      e.stopPropagation();

      try {
        const response = await fetch("/api/system/open-url", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ url: href }),
        });

        if (!response.ok) {
          const errorData = await response.json().catch(() => ({}));
          console.warn("Backend open-url failed:", errorData);
          // Fallback to window.open
          window.open(href, "_blank", "noopener,noreferrer");
        }
      } catch (err) {
        console.warn("Backend open-url request failed:", err);
        window.open(href, "_blank", "noopener,noreferrer");
      }
    },
    true
  ); // Use capture phase to intercept before other handlers

  function freezeTableLayout(tableElement) {
    const table =
      tableElement instanceof HTMLTableElement
        ? tableElement
        : tableElement && typeof tableElement.closest === "function"
          ? tableElement.closest("table")
          : null;

    if (!(table instanceof HTMLTableElement)) {
      return () => {};
    }

    const headerCells = Array.from(table.querySelectorAll("thead th"));
    if (headerCells.length === 0) {
      return () => {};
    }

    const tableRect = table.getBoundingClientRect();
    if (!tableRect || tableRect.width === 0) {
      return () => {};
    }

    const state = {
      width: table.style.width,
      minWidth: table.style.minWidth,
      maxWidth: table.style.maxWidth,
      layout: table.style.tableLayout,
      cellStyles: headerCells.map((th) => ({
        el: th,
        width: th.style.width,
        minWidth: th.style.minWidth,
        maxWidth: th.style.maxWidth,
      })),
    };

    const tableWidthPx = `${Math.max(tableRect.width, 1)}px`;
    table.style.width = tableWidthPx;
    table.style.minWidth = tableWidthPx;
    table.style.maxWidth = tableWidthPx;
    table.style.tableLayout = "fixed";
    table.classList.add("table--layout-frozen");

    headerCells.forEach((th) => {
      const rect = th.getBoundingClientRect();
      const widthPx = `${Math.max(rect.width, 1)}px`;
      th.style.width = widthPx;
      th.style.minWidth = widthPx;
      th.style.maxWidth = widthPx;
    });

    let released = false;
    return function releaseTableLayout() {
      if (released) {
        return;
      }
      released = true;

      table.style.width = state.width;
      table.style.minWidth = state.minWidth;
      table.style.maxWidth = state.maxWidth;
      table.style.tableLayout = state.layout || "";
      table.classList.remove("table--layout-frozen");

      state.cellStyles.forEach((entry) => {
        entry.el.style.width = entry.width;
        entry.el.style.minWidth = entry.minWidth;
        entry.el.style.maxWidth = entry.maxWidth;
      });
    };
  }

  function preserveScrollPosition(container, callback) {
    if (!(container instanceof HTMLElement) || typeof callback !== "function") {
      return typeof callback === "function" ? callback() : undefined;
    }

    const top = container.scrollTop;
    const left = container.scrollLeft;

    let result;
    try {
      result = callback();
    } finally {
      container.scrollTop = top;
      container.scrollLeft = left;
    }

    if (result && typeof result.then === "function") {
      return result.finally(() => {
        container.scrollTop = top;
        container.scrollLeft = left;
      });
    }

    return result;
  }

  /**
   * Show a toast notice.
   *
   * Two shapes, one behaviour: `showToast("Saved", "success")` for a bare
   * outcome, or `showToast({ type, title, message, key })` when the notice has
   * detail or an identity. Passing a `key` makes repeat calls UPDATE the toast
   * already on screen instead of stacking another one — use it for anything
   * that can fire more than once (a poll failure, a step of a long operation).
   *
   * @param {string|Object} messageOrConfig
   * @param {string} [type] only read when the first argument is a string
   * @returns {{key:string,update:Function,dismiss:Function}|null}
   */
  function showToast(messageOrConfig, type = "success") {
    if (!toastManager) {
      console.warn("[Utils] Toast manager not loaded yet:", messageOrConfig);
      return null;
    }

    if (typeof messageOrConfig === "string") {
      return toastManager.show({ type, title: messageOrConfig });
    }

    return toastManager.show(messageOrConfig);
  }

  // Clipboard results all share one toast key, so copying five addresses in a
  // row replaces one notice instead of stacking five identical ones.
  const CLIPBOARD_TOAST_KEY = "clipboard";

  /** @param {string} label what was copied, e.g. "Mint address" */
  function notifyCopied(label) {
    return showToast({ key: CLIPBOARD_TOAST_KEY, type: "success", title: I18n.t("shell-toast-copied", { label }) });
  }

  function notifyCopyFailed(error) {
    return showToast({
      key: CLIPBOARD_TOAST_KEY,
      type: "error",
      title: I18n.t("shell-toast-copy-failed"),
      message: error ? String(error) : null,
    });
  }

  function copyToClipboard(value) {
    return navigator.clipboard.writeText(value);
  }

  function copyMint(mint) {
    return copyToClipboard(mint)
      .then(() => notifyCopied(I18n.t("links-mint-address")))
      .catch((err) => {
        notifyCopyFailed(err);
        throw err;
      });
  }

  function copyDebugValue(value, label) {
    return copyToClipboard(value)
      .then(() => notifyCopied(label))
      .catch((err) => {
        notifyCopyFailed(err);
        throw err;
      });
  }

  async function copyDebugInfo(mint, type) {
    try {
      const endpoint =
        type === "position" ? `/api/positions/${mint}/debug` : `/api/tokens/${mint}/debug`;
      const res = await fetch(endpoint);
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const data = await res.json();
      const text = generateDebugText(data, type);
      await copyToClipboard(text);
      notifyCopied(I18n.t("common-copied-debug-info"));
    } catch (err) {
      console.error("copyDebugInfo error:", err);
      notifyCopyFailed(err);
      throw err;
    }
  }

  // Debug dumps show whole counts with up to three decimals, as a person reads a market figure.
  const DEBUG_COUNT = { decimals: 0, maxDecimals: 3 };

  function generateDebugText(data, type) {
    const lines = [];
    const tokenInfo = data.token_info || {};
    const price = data.price_data || {};
    const market = data.market_data || {};
    const pools = Array.isArray(data.pools) ? data.pools : [];
    const security = data.security || {};
    const pos = data.position_data || {};

    lines.push("ScreenerBot Debug Info");
    lines.push(`Mint: ${data.mint || "N/A"}`);
    if (tokenInfo.symbol || tokenInfo.name) {
      lines.push(
        `Token: ${tokenInfo.symbol || "N/A"} ${tokenInfo.name ? "(" + tokenInfo.name + ")" : ""}`
      );
    }
    lines.push("");

    lines.push("[Token]");
    lines.push(`Symbol: ${tokenInfo.symbol ?? "N/A"}`);
    lines.push(`Name: ${tokenInfo.name ?? "N/A"}`);
    lines.push(`Decimals: ${tokenInfo.decimals ?? "N/A"}`);
    lines.push(`Website: ${tokenInfo.website ?? "N/A"}`);
    lines.push(`Verified: ${tokenInfo.is_verified ? "Yes" : "No"}`);
    const tags = Array.isArray(tokenInfo.tags) ? tokenInfo.tags.join(", ") : "None";
    lines.push(`Tags: ${tags}`);
    lines.push("");

    lines.push("[Price & Market]");
    lines.push(
      `Price (SOL): ${
        price.pool_price_native != null
          ? formatPriceSol(price.pool_price_native, { fallback: "N/A" })
          : "N/A"
      }`
    );
    lines.push(
      `Confidence: ${
        price.confidence != null ? formatPercentValue(Number(price.confidence) * 100, { decimals: 1, plus: "" }) : "N/A"
      }`
    );
    lines.push(
      `Last Updated: ${
        price.last_updated ? new Date(price.last_updated * 1000).toISOString() : "N/A"
      }`
    );
    lines.push(
      `Market Cap: ${
        market.market_cap != null ? withUsdSymbol(formatNumber(market.market_cap, DEBUG_COUNT)) : "N/A"
      }`
    );
    lines.push(`FDV: ${market.fdv != null ? withUsdSymbol(formatNumber(market.fdv, DEBUG_COUNT)) : "N/A"}`);
    lines.push(
      `Liquidity: ${
        market.liquidity_usd != null ? withUsdSymbol(formatNumber(market.liquidity_usd, DEBUG_COUNT)) : "N/A"
      }`
    );
    lines.push(
      `24h Volume: ${
        market.volume_24h != null ? withUsdSymbol(formatNumber(market.volume_24h, DEBUG_COUNT)) : "N/A"
      }`
    );
    lines.push("");

    lines.push("[Pools]");
    if (pools.length === 0) {
      lines.push("None");
    } else {
      pools.forEach((p, idx) => {
        lines.push(`Pool #${idx + 1}`);
        lines.push(`  Address: ${p.pool_address ?? "N/A"}`);
        lines.push(`  DEX: ${p.dex_name ?? "N/A"}`);
        lines.push(
          `  SOL Reserves: ${p.sol_reserves != null ? Number(p.sol_reserves).toFixed(2) : "N/A"}`
        );
        lines.push(
          `  Token Reserves: ${
            p.token_reserves != null ? Number(p.token_reserves).toFixed(2) : "N/A"
          }`
        );
        lines.push(
          `  Price (SOL): ${
            p.price_sol != null ? formatPriceSol(p.price_sol, { fallback: "N/A" }) : "N/A"
          }`
        );
      });
    }
    lines.push("");

    lines.push("[Security]");
    lines.push(`Score: ${security.score ?? "N/A"}`);
    lines.push(`Rugged: ${security.rugged ? "Yes" : "No"}`);
    lines.push(`Total Holders: ${security.total_holders ?? "N/A"}`);
    lines.push(
      `Top 10 Concentration: ${
        security.top_10_concentration != null
          ? formatPercentValue(security.top_10_concentration, { decimals: 2, plus: "" })
          : "N/A"
      }`
    );
    lines.push(`Mint Authority: ${security.mint_authority ?? "None"}`);
    lines.push(`Freeze Authority: ${security.freeze_authority ?? "None"}`);
    const risks = Array.isArray(security.risks) ? security.risks : [];
    if (risks.length) {
      lines.push("Risks:");
      risks.forEach((r) =>
        lines.push(`  - ${r.name || "Unknown"}: ${r.level || "N/A"} (${r.description || ""})`)
      );
    } else {
      lines.push("Risks: None");
    }
    lines.push("");

    if (type === "position") {
      lines.push("[Position]");
      if (pos && Object.keys(pos).length) {
        lines.push(`Open Positions: ${pos.open_position ? "1" : "0"}`);
        lines.push(`Closed Positions: ${pos.closed_positions_count ?? "0"}`);
        lines.push(
          `Total P&L: ${pos.total_pnl != null ? formatSol(pos.total_pnl, { decimals: 4 }) : "N/A"}`
        );
        lines.push(
          `Win Rate: ${pos.win_rate != null ? formatPercentValue(pos.win_rate, { decimals: 1, plus: "" }) : "N/A"}`
        );
        if (pos.open_position) {
          const o = pos.open_position;
          lines.push("Open Position:");
          lines.push(
            `  Entry Price: ${
              o.entry_price != null ? formatPriceSol(o.entry_price, { fallback: "N/A" }) : "N/A"
            }`
          );
          lines.push(
            `  Entry Size: ${
              o.entry_size_native != null ? formatSol(o.entry_size_native, { decimals: 4 }) : "N/A"
            }`
          );
          lines.push(
            `  Current Price: ${
              o.current_price != null ? formatPriceSol(o.current_price, { fallback: "N/A" }) : "N/A"
            }`
          );
          lines.push(
            `  Unrealized P&L: ${
              o.unrealized_pnl != null ? formatSol(o.unrealized_pnl, { decimals: 4 }) : "N/A"
            }`
          );
          lines.push(
            `  Unrealized P&L %: ${
              o.unrealized_pnl_percent != null
                ? formatPercentValue(o.unrealized_pnl_percent, { decimals: 2, plus: "" })
                : "N/A"
            }`
          );
        }
      } else {
        lines.push("No position data available");
      }
      lines.push("");
    }

    if (data.pool_debug) {
      const pd = data.pool_debug;
      lines.push("[Pool Debug]");
      if (pd.price_history && pd.price_history.length > 0) {
        lines.push(`Price History Points: ${pd.price_history.length}`);
        lines.push("Recent Prices (last 10):");
        pd.price_history.slice(0, 10).forEach((p, i) => {
          const date = new Date(p.timestamp * 1000).toISOString();
          const historyPrice =
            p.price_sol != null ? formatPriceSol(p.price_sol, { fallback: "N/A" }) : "N/A";
          lines.push(
            `  ${i + 1}. ${date} - ${withSolUnit(historyPrice)} (conf: ${formatPercentValue(p.confidence * 100, { decimals: 1, plus: "" })})`
          );
        });
      }
      if (pd.price_stats) {
        const ps = pd.price_stats;
        lines.push(
          `Min Price: ${withSolUnit(
            formatPriceSol(ps.min_price, { fallback: "N/A" })
          )}`
        );
        lines.push(
          `Max Price: ${withSolUnit(
            formatPriceSol(ps.max_price, { fallback: "N/A" })
          )}`
        );
        lines.push(
          `Avg Price: ${withSolUnit(
            formatPriceSol(ps.avg_price, { fallback: "N/A" })
          )}`
        );
        lines.push(`Volatility: ${formatPercentValue(ps.price_volatility, { decimals: 2, plus: "" })}`);
        lines.push(`Data Points: ${ps.data_points}`);
        lines.push(
          `Time Span: ${ps.time_span_seconds}s (${(ps.time_span_seconds / 60).toFixed(0)} min)`
        );
      }
      if (pd.all_pools && pd.all_pools.length > 0) {
        lines.push(`All Pools (${pd.all_pools.length}):`);
        pd.all_pools.forEach((pool, i) => {
          lines.push(`  Pool #${i + 1}: ${pool.pool_address}`);
          lines.push(`    DEX: ${pool.dex_name}`);
        });
      }
      if (pd.cache_stats) {
        lines.push(
          `Cache - Total: ${pd.cache_stats.total_tokens_cached}, Fresh: ${pd.cache_stats.fresh_prices}, History: ${pd.cache_stats.history_entries}`
        );
      }
      lines.push("");
    }

    if (data.token_debug) {
      const td = data.token_debug;
      lines.push("[Token Debug]");
      if (td.blacklist_status) {
        lines.push(`Blacklisted: ${td.blacklist_status.is_blacklisted ? "Yes" : "No"}`);
        if (td.blacklist_status.is_blacklisted && td.blacklist_status.reason) {
          lines.push(`  Reason: ${td.blacklist_status.reason}`);
          lines.push(`  Occurrences: ${td.blacklist_status.occurrence_count}`);
          lines.push(`  First Occurrence: ${td.blacklist_status.first_occurrence || "N/A"}`);
        }
      }
      if (td.ohlcv_availability) {
        const oa = td.ohlcv_availability;
        lines.push(
          `OHLCV: 1m=${oa.has_1m_data}, 5m=${oa.has_5m_data}, 15m=${oa.has_15m_data}, 1h=${oa.has_1h_data}`
        );
        lines.push(`Total Candles: ${oa.total_candles}`);
        if (oa.oldest_timestamp) {
          lines.push(`  Oldest: ${new Date(oa.oldest_timestamp * 1000).toISOString()}`);
        }
        if (oa.newest_timestamp) {
          lines.push(`  Newest: ${new Date(oa.newest_timestamp * 1000).toISOString()}`);
        }
      }
      if (td.decimals_info) {
        lines.push(
          `Decimals: ${td.decimals_info.decimals ?? "N/A"} (${
            td.decimals_info.source
          }, cached: ${td.decimals_info.cached})`
        );
      }
      lines.push("");
    }

    if (data.position_debug) {
      const pd = data.position_debug;
      lines.push("[Position Debug]");
      if (pd.transaction_details) {
        lines.push("Transactions:");
        lines.push(
          `  Entry: ${
            pd.transaction_details.entry_signature || "N/A"
          } (verified: ${pd.transaction_details.entry_verified})`
        );
        lines.push(
          `  Exit: ${
            pd.transaction_details.exit_signature || "N/A"
          } (verified: ${pd.transaction_details.exit_verified})`
        );
        if (pd.transaction_details.synthetic_exit) {
          lines.push("  Synthetic Exit: Yes");
        }
        if (pd.transaction_details.closed_reason) {
          lines.push(`  Closed Reason: ${pd.transaction_details.closed_reason}`);
        }
      }
      if (pd.fee_details) {
        lines.push("Fees:");
        lines.push(
          `  Entry: ${withSolUnit(formatFixed(pd.fee_details.entry_fee_native, { decimals: 6, fallback: "N/A" }))} (${
            pd.fee_details.entry_fee_raw || 0
          } lamports)`
        );
        lines.push(
          `  Exit: ${withSolUnit(formatFixed(pd.fee_details.exit_fee_native, { decimals: 6, fallback: "N/A" }))} (${
            pd.fee_details.exit_fee_raw || 0
          } lamports)`
        );
        lines.push(`  Total: ${withSolUnit(formatFixed(pd.fee_details.total_fees_sol, { decimals: 6 }))}`);
      }
      if (pd.profit_targets) {
        lines.push(
          `Profit Targets: Min ${withPercentUnit(
            pd.profit_targets.min_target_percent || "N/A"
          )}, Max ${withPercentUnit(pd.profit_targets.max_target_percent || "N/A")}`
        );
        lines.push(`Liquidity Tier: ${pd.profit_targets.liquidity_tier || "N/A"}`);
      }
      if (pd.price_tracking) {
        lines.push("Price Tracking:");
        lines.push(`  High: ${pd.price_tracking.price_highest}`);
        lines.push(`  Low: ${pd.price_tracking.price_lowest}`);
        lines.push(`  Current: ${pd.price_tracking.current_price || "N/A"}`);
        if (pd.price_tracking.drawdown_from_high) {
          lines.push(`  Drawdown from High: ${formatPercentValue(pd.price_tracking.drawdown_from_high, { decimals: 2, plus: "" })}`);
        }
        if (pd.price_tracking.gain_from_low) {
          lines.push(`  Gain from Low: ${formatPercentValue(pd.price_tracking.gain_from_low, { decimals: 2, plus: "" })}`);
        }
      }
      if (pd.phantom_details) {
        lines.push("Phantom:");
        lines.push(`  Remove Flag: ${pd.phantom_details.phantom_remove}`);
        lines.push(`  Confirmations: ${pd.phantom_details.phantom_confirmations}`);
        if (pd.phantom_details.phantom_first_seen) {
          lines.push(`  First Seen: ${pd.phantom_details.phantom_first_seen}`);
        }
      }
      if (pd.proceeds_metrics) {
        const pm = pd.proceeds_metrics;
        lines.push("Proceeds Metrics:");
        lines.push(
          `  Accepted: ${pm.accepted_quotes} (${pm.accepted_profit_quotes} profit, ${pm.accepted_loss_quotes} loss)`
        );
        lines.push(`  Rejected: ${pm.rejected_quotes}`);
        lines.push(`  Avg Shortfall: ${pm.average_shortfall_bps.toFixed(2)} bps`);
        lines.push(`  Worst Shortfall: ${pm.worst_shortfall_bps} bps`);
      }
      lines.push("");
    }

    return lines.join("\n");
  }

  /**
   * Opens a URL in the default system browser via backend API.
   * This works in Electron because the backend uses system commands (open/xdg-open/start).
   * Falls back to window.open if backend request fails.
   * @param {string} url - The URL to open
   */
  async function openExternal(url) {
    if (!url) return;

    try {
      const response = await fetch("/api/system/open-url", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        console.warn("Backend open-url failed:", errorData);
        // Fallback to window.open
        window.open(url, "_blank", "noopener,noreferrer");
      }
    } catch (err) {
      console.warn("Backend open-url request failed:", err);
      window.open(url, "_blank", "noopener,noreferrer");
    }
  }

  function openGMGN(mint) {
    openExternal(`https://gmgn.ai/sol/token/${mint}`);
  }

  function openDexScreener(mint) {
    openExternal(`https://dexscreener.com/solana/${mint}`);
  }

  function openSolscan(mint) {
    openExternal(`https://solscan.io/token/${mint}`);
  }

  // --- Solscan URL builders (single source of truth for explorer links) -------
  function solscanTokenUrl(mint) {
    return `https://solscan.io/token/${mint}`;
  }

  function solscanAccountUrl(address) {
    return `https://solscan.io/account/${address}`;
  }

  function solscanTxUrl(signature) {
    return `https://solscan.io/tx/${signature}`;
  }

  function openSolscanAccount(address) {
    openExternal(solscanAccountUrl(address));
  }

  // Copy an address and surface a toast (use for wallet/pool/authority addresses).
  function copyAddress(address) {
    return copyToClipboard(address)
      .then(() => notifyCopied(I18n.t("common-copied-address")))
      .catch((err) => {
        notifyCopyFailed(err);
        throw err;
      });
  }

  /**
   * Render a reusable address chip: the address itself links to Solscan and a
   * copy button sits beside it. Used across dialogs so every wallet / pool /
   * authority address is consistently clickable + copyable.
   * @param {string} address base58 account address (or signature when kind="tx")
   * @param {Object} [opts]
   * @param {boolean} [opts.full=false] show the whole address instead of a short form
   * @param {string} [opts.kind="account"] "account" | "tx" | "token"
   * @returns {string} HTML string
   */
  function renderAddressChip(address, opts = {}) {
    if (!address) return '<span class="addr-chip addr-chip-empty">—</span>';
    const { full = false, kind = "account" } = opts;
    const raw = String(address);
    const safe = escapeHtml(raw);
    const display = full ? safe : escapeHtml(formatAddressCompact(raw, { start: 6, end: 6 }));
    const url =
      kind === "tx"
        ? solscanTxUrl(raw)
        : kind === "token"
          ? solscanTokenUrl(raw)
          : solscanAccountUrl(raw);
    // base58 addresses contain no quotes/HTML-special chars, so inlining is safe.
    const onclick = `event.preventDefault();event.stopPropagation();Utils.copyAddress('${raw}')`;
    return `<span class="addr-chip${full ? " addr-chip-full" : ""}"><a class="addr-chip-link mono" dir="ltr" href="${url}" target="_blank" rel="noopener noreferrer" title="${safe} ${escapeHtml(I18n.t("shell-address-open-solscan"))}">${display}</a><button type="button" class="addr-chip-copy" title="${escapeHtml(I18n.t("shell-address-copy"))}" onclick="${onclick}"><i class="icon-copy"></i></button></span>`;
  }

  // DOM Helper Functions
  function el(id) {
    return document.getElementById(id);
  }

  function qs(selector, scope = document) {
    return scope.querySelector(selector);
  }

  function qsa(selector, scope = document) {
    return Array.from(scope.querySelectorAll(selector));
  }

  // Input Helper Functions
  function textFromInput(id) {
    const input = el(id);
    if (!input) return null;
    const value = input.value.trim();
    return value ? value : null;
  }

  function numberFromInput(id) {
    const input = el(id);
    if (!input) return null;
    const raw = input.value.trim();
    if (raw === "") return null;
    const parsed = parseFloat(raw);
    return Number.isFinite(parsed) ? parsed : null;
  }

  // String Helper Functions
  function toSlug(value) {
    if (!value) return "";
    return String(value)
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
  }

  // Timing Helper Functions

  /**
   * Creates a debounced version of a function that delays execution until
   * after a specified wait period has passed since the last call.
   * @param {Function} fn - The function to debounce
   * @param {number} wait - The number of milliseconds to delay
   * @returns {Function} The debounced function
   */
  function debounce(fn, wait) {
    let timeoutId = null;
    return function debounced(...args) {
      if (timeoutId !== null) {
        clearTimeout(timeoutId);
      }
      timeoutId = setTimeout(() => {
        timeoutId = null;
        fn.apply(this, args);
      }, wait);
    };
  }

  /**
   * Creates a throttled version of a function that only executes at most once
   * per specified time interval.
   * @param {Function} fn - The function to throttle
   * @param {number} limit - The minimum time between executions in milliseconds
   * @returns {Function} The throttled function
   */
  function throttle(fn, limit) {
    let lastCall = 0;
    let timeoutId = null;
    return function throttled(...args) {
      const now = Date.now();
      const timeSinceLastCall = now - lastCall;

      if (timeSinceLastCall >= limit) {
        // Enough time has passed, execute immediately
        lastCall = now;
        fn.apply(this, args);
      } else if (timeoutId === null) {
        // Schedule execution for when limit expires
        timeoutId = setTimeout(() => {
          lastCall = Date.now();
          timeoutId = null;
          fn.apply(this, args);
        }, limit - timeSinceLastCall);
      }
      // If timeout already scheduled, ignore this call
    };
  }

  /**
   * Creates a focus trap for modal dialogs
   * @param {HTMLElement} container - The dialog container
   * @returns {Object} - Object with activate() and deactivate() methods
   */
  function createFocusTrap(container) {
    const focusableSelectors = [
      'button:not([disabled]):not([tabindex="-1"])',
      'input:not([disabled]):not([tabindex="-1"])',
      'select:not([disabled]):not([tabindex="-1"])',
      'textarea:not([disabled]):not([tabindex="-1"])',
      '[href]:not([tabindex="-1"])',
      '[tabindex]:not([tabindex="-1"])',
    ].join(", ");

    let previousActiveElement = null;

    const getFocusableElements = () =>
      Array.from(container.querySelectorAll(focusableSelectors)).filter(
        (el) => el.offsetParent !== null
      );

    const handleKeydown = (e) => {
      if (e.key !== "Tab") return;

      const focusable = getFocusableElements();
      if (focusable.length === 0) return;

      const first = focusable[0];
      const last = focusable[focusable.length - 1];

      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };

    return {
      activate: () => {
        previousActiveElement = document.activeElement;
        container.addEventListener("keydown", handleKeydown);
        // Focus first focusable element
        const focusable = getFocusableElements();
        if (focusable.length > 0) {
          setTimeout(() => focusable[0].focus(), 50);
        }
      },
      deactivate: () => {
        container.removeEventListener("keydown", handleKeydown);
        // Restore focus
        if (previousActiveElement && previousActiveElement.focus) {
          previousActiveElement.focus();
        }
      },
    };
  }

  const Utils = {
    formatNumber,
    formatCompactNumber,
    updateLiveNumber,
    formatBooleanFlag,
    formatCurrencyUSD,
    formatPriceSubscript,
    formatPriceSol,
    formatPercentValue,
    formatPercent,
    formatSol,
    formatSignedSol,
    formatSignedNumber,
    signedTone,
    formatPnL,
    formatTimeFromSeconds,
    formatTimestamp,
    formatDate,
    formatDatePart,
    formatCompactFixed,
    formatTimeSpan,
    formatTimeAgo,
    formatTimeUntil,
    formatUptime,
    formatBytes,
    formatDuration,
    formatSignatureCompact,
    formatAddressCompact,
    formatSecondsToTime,
    formatList,
    escapeHtml,
    setText,
    setHtml,
    el,
    qs,
    qsa,
    textFromInput,
    numberFromInput,
    toSlug,
    freezeTableLayout,
    preserveScrollPosition,
    showToast,
    notifyCopied,
    notifyCopyFailed,
    copyToClipboard,
    copyMint,
    copyDebugValue,
    copyDebugInfo,
    generateDebugText,
    openExternal,
    openGMGN,
    openDexScreener,
    openSolscan,
    solscanTokenUrl,
    solscanAccountUrl,
    solscanTxUrl,
    openSolscanAccount,
    copyAddress,
    renderAddressChip,
    debounce,
    throttle,
    createFocusTrap,
  };

  // Keep window.Utils for legacy compatibility during migration
  if (typeof window !== "undefined") {
    window.Utils = Utils;
    window.showToast = showToast;
  }

  return Utils;
})();

// The formatters are the functions of core/format.js, re-exported unchanged.
export {
  formatNumber,
  formatCompactNumber,
  formatBooleanFlag,
  formatCurrencyUSD,
  formatPriceSubscript,
  formatPriceSol,
  formatPercentValue,
  formatPercent,
  formatSol,
  formatSignedSol,
  formatSignedNumber,
  signedTone,
  formatPnL,
  formatTimeFromSeconds,
  formatTimestamp,
  formatDate,
  formatDatePart,
  formatCompactFixed,
  formatTimeSpan,
  formatTimeAgo,
  formatTimeUntil,
  formatUptime,
  formatBytes,
  formatDuration,
  formatSignatureCompact,
  formatAddressCompact,
  formatSecondsToTime,
  formatList,
};

// Export the remaining helpers from the IIFE result
export const {
  updateLiveNumber,
  escapeHtml,
  setText,
  setHtml,
  el,
  qs,
  qsa,
  textFromInput,
  numberFromInput,
  toSlug,
  freezeTableLayout,
  preserveScrollPosition,
  showToast,
  notifyCopied,
  notifyCopyFailed,
  copyToClipboard,
  copyMint,
  copyDebugValue,
  copyDebugInfo,
  generateDebugText,
  openExternal,
  openGMGN,
  openDexScreener,
  openSolscan,
  solscanTokenUrl,
  solscanAccountUrl,
  solscanTxUrl,
  openSolscanAccount,
  copyAddress,
  renderAddressChip,
  debounce,
  throttle,
  createFocusTrap,
} = (function () {
  // Return Utils from IIFE above (it's in module scope)
  return (typeof window !== "undefined" && window.Utils) || {};
})();

/**
 * Normalize any provider-supplied image URL, or return null if an <img> could
 * not load it (blank, relative, data:, ipfs://, ...).
 *
 * The scheme test is case-insensitive on purpose: providers do ship capitalized
 * schemes (Jupiter serves ANSEM's icon as "Https://www.blackbullsol.com/..."),
 * and a case-sensitive check silently discarded those.
 */
export function normalizeImageUrl(raw) {
  if (typeof raw !== "string") return null;

  const url = raw.trim();
  if (!url) return null;

  return /^https?:\/\//i.test(url) ? url : null;
}

/**
 * Resolve a token's displayable logo URL, or null when there is none to show.
 *
 * Accepts whichever field the source used (`logo` on featured cards, `icon`
 * straight from a provider, `image_url` on our own token rows).
 */
export function resolveTokenLogoUrl(token) {
  if (!token) return null;
  return normalizeImageUrl(token.logo_url || token.logo || token.icon || token.image_url);
}

/**
 * Resolve a token's wide banner/header image, or null when it has none.
 * Only DexScreener supplies these (1500x500), so most tokens have none and the
 * banner must simply be absent rather than showing a placeholder.
 */
export function resolveTokenBannerUrl(token) {
  if (!token) return null;
  // `banner` is the featured card's field; `header_image_url` is the token detail
  // API's. Both must be accepted -- matching only one silently renders every card
  // bannerless (it falls through to the accent gradient, so it LOOKS designed).
  return normalizeImageUrl(token.banner || token.header_image_url || token.header);
}

/**
 * Global `[data-copy]` delegation: any element carrying `data-copy="<text>"`
 * copies that text on click, from anywhere in the dashboard.
 *
 * It lives here, once, because the markup that uses it is shared (token identity
 * chips, mint addresses, signatures) and a per-dialog listener meant the button
 * silently did nothing on any surface that had not loaded that dialog's module.
 * A container-scoped handler that calls stopPropagation (position activity) still
 * wins -- the event never reaches this one.
 */
if (typeof document !== "undefined" && !window.__copyDelegationInstalled) {
  window.__copyDelegationInstalled = true;
  document.addEventListener("click", (event) => {
    const trigger = event.target.closest("[data-copy]");
    if (!trigger) return;
    const text = trigger.dataset.copy;
    if (!text) return;
    event.preventDefault();
    copyToClipboard(text)
      .then(() => notifyCopied(I18n.t("common-copied-value")))
      .catch(notifyCopyFailed);
  });
}
