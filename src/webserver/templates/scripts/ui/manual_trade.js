/**
 * Manual Trade - the ONE way the dashboard performs a manual buy / add / sell.
 *
 * This flow (open TradeActionDialog -> POST the manual endpoint -> toast) was
 * copy-pasted into the context menu, the token details dialog, the tokens page and
 * the featured dialog. The copies had already drifted apart, in ways that were silent and
 * wrong:
 *
 *   - Only the tokens page forwarded the position ownership choice from the dialog's
 *     checkbox. Buying from the context menu or token details therefore DISCARDED
 *     the user's choice, so a manual buy could be auto-sold by the trader --
 *     manual/force buys are supposed to be protected from auto-sell.
 *   - "Add to position" POSTed to /api/trader/manual/add from the tokens page but
 *     to /api/trader/manual/buy from the context menu.
 *
 * Every caller now goes through here, so the payload is built in exactly one place.
 */

import * as Utils from "../core/utils.js";
import { requestManager } from "../core/request_manager.js";
import { TradeActionDialog } from "./trade_action_dialog.js";

// One dialog instance shared by every caller — it is modal, so there is never a
// reason for two of them to exist.
let tradeDialog = null;

function getDialog() {
  if (!tradeDialog) tradeDialog = new TradeActionDialog();
  return tradeDialog;
}

/**
 * Current wallet balance in SOL, read live at the moment it is needed.
 *
 * NEVER accept a balance a calling view already had: the tokens page fetched one on
 * tab activate and passed it in, so a wallet funded after that (the exact case: the
 * app started at 0 SOL, SOL arrived, the header updated) still opened the trade dialog
 * at 0 — with every buy amount rejected as "insufficient balance" while the header
 * showed the real figure. The endpoint serves the same live snapshot the header does.
 *
 * @returns {Promise<number>}
 */
export async function fetchWalletBalance() {
  try {
    const data = await requestManager.fetch("/api/wallet/balance", { priority: "high" });
    const balance = Number(data?.sol_balance);
    if (Number.isFinite(balance)) return balance;
  } catch {
    // fall through
  }
  return 0;
}

/**
 * The token's open position, or null when it has none.
 *
 * This is the authoritative source for holdings/decimals/size — NOT whatever the
 * calling view happened to have on its row. Callers used to pass these themselves
 * and got them wrong in ways nobody could see:
 *
 *   - token details passed `fullTokenData.holdings`, a field the token API does not
 *     even return, so every sell opened with holdings = 0 and no basis to size the
 *     percentage against;
 *   - the context menu passed holdings but NOT decimals, so the dialog printed the
 *     RAW on-chain amount ("1.2B tokens" for 1,200 tokens at 6 decimals);
 *   - nobody ever passed `currentSize`, so the "Current Position" row in the add
 *     dialog was dead code that never rendered.
 *
 * Resolving it here once means every entry point is correct by construction.
 */
async function fetchPosition(mint) {
  try {
    const data = await requestManager.fetch(
      `/api/positions/${encodeURIComponent(mint)}/details`,
      { priority: "high" }
    );

    // /details returns PositionDetailResponse directly; `position` flattens the
    // summary fields onto itself (no {success,data} envelope).
    const position = data?.position;
    if (!position) return null;

    return {
      // remaining_token_amount reflects partial exits; token_amount is the original.
      holdings: position.remaining_token_amount ?? position.token_amount ?? 0,
      decimals: position.token_decimals ?? data?.token_info?.decimals ?? null,
      currentSize: position.total_size_sol ?? position.entry_size_sol ?? null,
      symbol: position.symbol || null,
      name: position.name || null,
      logo: position.logo_url || data?.token_info?.image_url || null,
      positionManagement: position.management || "auto_trader",
    };
  } catch {
    // No position, or the lookup failed — the trade still proceeds.
    return null;
  }
}

/**
 * Token identity (symbol / name / logo) for the dialog header.
 * Only called when the position did not already supply it.
 */
async function fetchTokenIdentity(mint) {
  try {
    const token = await requestManager.fetch(`/api/tokens/${encodeURIComponent(mint)}`, {
      priority: "high",
    });
    if (!token?.mint) return null;

    return {
      symbol: token.symbol || null,
      name: token.name || null,
      logo: token.logo_url || null,
      decimals: token.decimals ?? null,
    };
  } catch {
    return null;
  }
}

/**
 * The trader's configured sizing, used to build the "Add to Position" presets.
 *
 * `trade_size_sol` x `dca_size_percentage` is exactly what the BACKEND adds when no
 * size is sent, so showing it as a preset means the user sees the amount they are
 * about to commit instead of confirming an invisible default. `entry_sizes` is the
 * configured quick-amount list. All read from config — never hardcoded, and resolved
 * HERE so no page carries its own copy of the defaults (the tokens and positions pages
 * each fetched the section themselves and fell back to a hardcoded
 * `[0.005, 0.01, 0.02, 0.05]`, which silently drifts from the real config).
 *
 * Not cached: config is editable while the app runs, and a dialog that shows a
 * superseded trade size is the same class of bug as one that shows a stale balance.
 * requestManager dedupes concurrent calls, so opening a dialog costs one request.
 */
async function fetchTraderDefaults() {
  try {
    // Config GET routes wrap the section in { data, timestamp }.
    const body = await requestManager.fetch("/api/config/trader", { priority: "high" });
    const tradeSize = Number(body?.data?.trade_size_sol);
    const dcaPct = Number(body?.data?.dca_size_percentage);
    const entrySizes = body?.data?.entry_sizes;

    return {
      tradeSize: Number.isFinite(tradeSize) && tradeSize > 0 ? tradeSize : null,
      dcaSize:
        Number.isFinite(tradeSize) && tradeSize > 0
          ? Number.isFinite(dcaPct) && dcaPct > 0
            ? tradeSize * (dcaPct / 100)
            : tradeSize
          : null,
      entrySizes: Array.isArray(entrySizes)
        ? entrySizes.filter((size) => Number.isFinite(size) && size > 0)
        : [],
    };
  } catch {
    // The dialog still opens — the user just types an amount instead of picking one.
    return { tradeSize: null, dcaSize: null, entrySizes: [] };
  }
}

const ENDPOINTS = {
  buy: "/api/trader/manual/buy",
  add: "/api/trader/manual/add",
  sell: "/api/trader/manual/sell",
};

const SUCCESS_MESSAGES = {
  buy: "Buy order placed!",
  add: "Added to position!",
  sell: "Sell order placed!",
};

const FAILURE_MESSAGES = {
  buy: "Buy failed",
  add: "Add to position failed",
  sell: "Sell failed",
};

/**
 * Build the POST body for a completed dialog result.
 */
function buildBody(action, mint, result) {
  // Per-trade slippage override from the dialog. Absent = follow the configured
  // slippage, which is what the auto-trader always does.
  const slippage =
    typeof result.slippage_pct === "number" ? { slippage_pct: result.slippage_pct } : {};

  if (action === "sell") {
    // 100% is a full close, which the backend takes as `close_all` rather than a
    // percentage (avoids leaving dust behind).
    return result.percentage === 100
      ? { mint, close_all: true, ...slippage }
      : { mint, percentage: result.percentage, ...slippage };
  }

  return {
    mint,
    ...slippage,
    ...(result.amount ? { size_sol: result.amount } : {}),
    ...(typeof result.management === "string" ? { management: result.management } : {}),
  };
}

/**
 * Run a manual trade end to end: resolve the token and its position, confirm it in
 * the shared dialog, POST it, toast the outcome.
 *
 * Callers supply only what they know. Identity (symbol/name/logo), holdings,
 * decimals and current position size are all resolved HERE, so no view can pass
 * them wrongly or forget them.
 *
 * @param {Object} options
 * @param {"buy"|"add"|"sell"} options.action
 * @param {string} options.mint - captured by the CALLER before any await; a dialog's
 *   own token data can be nulled while the trade dialog is open.
 * @param {string} [options.symbol] - display hint; the resolved value wins
 * @param {string} [options.name]
 * @param {string} [options.logo]
 * @param {Object} [options.context] - extra fields merged into the dialog context
 * @param {HTMLElement} [options.btn] - disabled while the request is in flight
 * @returns {Promise<boolean>} true when a trade was actually placed (false = user
 *   cancelled, or it failed)
 */
export async function manualTrade({ action, mint, symbol, name, logo, context = {}, btn = null }) {
  if (!action || !mint) {
    Utils.showToast("No mint address available", "error");
    return false;
  }

  try {
    // The position is authoritative for holdings/decimals/size AND already carries
    // the token's identity, so it doubles as the identity lookup when one exists.
    const [position, walletBalance] = await Promise.all([
      fetchPosition(mint),
      action === "sell" ? Promise.resolve(null) : fetchWalletBalance(),
    ]);

    // Only pay for a second lookup when the position did not supply identity.
    const identity =
      position || (symbol && logo) ? null : await fetchTokenIdentity(mint);

    // A buy OPENS a position, so buying a token that is already held would create a
    // second position for the same mint — which the backend now rejects outright
    // (positions resolve by mint, so duplicates corrupt each other). Adding to the
    // existing position is what the user means, so switch to it: they get the "Add to
    // Position" dialog with their current size, instead of an error after confirming.
    if (action === "buy" && position) {
      action = "add";
    }

    const dialogContext = {
      ...context,
      mint,
      symbol: symbol || position?.symbol || identity?.symbol || "?",
      name: name || position?.name || identity?.name || null,
      logo: logo || position?.logo || identity?.logo || null,

      // Position awareness: every action gets it, so a buy can warn that the token
      // is already held and an add/sell can size against the real amount.
      hasPosition: position != null,
      holdings: position?.holdings ?? 0,
      decimals: position?.decimals ?? identity?.decimals ?? null,
      currentSize: position?.currentSize ?? null,
      positionManagement: position?.positionManagement ?? null,
    };

    if (action !== "sell") {
      dialogContext.balance = walletBalance ?? 0;
    }

    // "Add to Position" presets. Only the tokens page ever passed its own, so every
    // other entry point (positions page, context menu, token details) opened the DCA
    // dialog with ZERO preset buttons and no preselected amount — a blank form.
    // Resolve them here, from the position and from config, so every caller gets them.
    if (action === "add") {
      const defaults = await fetchTraderDefaults();

      // The multiplier presets (1.0x / 1.5x / 2.0x) are sized off the money already in
      // the position — the natural basis for averaging in. Fall back to the configured
      // trade size when the position's size could not be resolved.
      dialogContext.entrySize = position?.currentSize ?? defaults.tradeSize ?? null;

      // Flat presets: the configured DCA size (the exact amount the backend adds when
      // no size_sol is sent) plus the configured entry sizes, deduplicated.
      const flat = [];
      if (defaults.dcaSize) flat.push(Number(defaults.dcaSize.toFixed(4)));
      defaults.entrySizes.forEach((size) => {
        if (!flat.includes(size)) flat.push(size);
      });
      dialogContext.entrySizes = flat;
    }

    const result = await getDialog().open({
      action,
      mint,
      symbol: dialogContext.symbol,
      context: dialogContext,
    });
    if (!result) return false; // user cancelled

    if (btn) btn.disabled = true;
    try {
      await requestManager.fetch(ENDPOINTS[action], {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(buildBody(action, mint, result)),
        priority: "high",
      });
    } finally {
      if (btn) btn.disabled = false;
    }

    Utils.showToast(SUCCESS_MESSAGES[action], "success");
    return true;
  } catch (error) {
    Utils.showToast(await describeError(error, action), "error");
    return false;
  }
}

/**
 * requestManager throws a generic `HTTP 400: Bad Request` on a failed response, so
 * the backend's actual reason ("insufficient balance", "force stop active", ...)
 * would be lost. It attaches the Response, so read the real message back off it.
 */
async function describeError(error, action) {
  try {
    const body = await error?.response?.json();
    const message = body?.error?.message || body?.message;
    if (message) return message;
  } catch {
    // no JSON body — fall back below
  }
  return error?.message || FAILURE_MESSAGES[action];
}
