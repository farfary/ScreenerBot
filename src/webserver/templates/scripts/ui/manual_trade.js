/**
 * Manual Trade - the ONE way the dashboard performs a manual buy / add / sell.
 *
 * This flow (open TradeActionDialog -> POST the manual endpoint -> toast) was
 * copy-pasted into the context menu, the token details dialog, the tokens page and
 * the billboard. The copies had already drifted apart, in ways that were silent and
 * wrong:
 *
 *   - Only the tokens page forwarded `manual_management` from the dialog's
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
 * Current wallet balance in SOL. Best-effort: the dialog still opens without it,
 * just with nothing to size the order against.
 * @returns {Promise<number>}
 */
export async function fetchWalletBalance() {
  try {
    const data = await requestManager.fetch("/api/wallet/balance", { priority: "low" });
    const balance = Number(data?.sol_balance);
    if (Number.isFinite(balance)) return balance;
  } catch {
    // fall through
  }
  return 0;
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
  if (action === "sell") {
    // 100% is a full close, which the backend takes as `close_all` rather than a
    // percentage (avoids leaving dust behind).
    return result.percentage === 100
      ? { mint, close_all: true }
      : { mint, percentage: result.percentage };
  }

  return {
    mint,
    ...(result.amount ? { size_sol: result.amount } : {}),
    // The dialog's manual-management checkbox (default true). Must be forwarded:
    // dropping it silently opts a manual buy back into auto-sell.
    ...(typeof result.manual_management === "boolean"
      ? { manual_management: result.manual_management }
      : {}),
  };
}

/**
 * Run a manual trade end to end: confirm it in the shared dialog, POST it, toast
 * the outcome.
 *
 * @param {Object} options
 * @param {"buy"|"add"|"sell"} options.action
 * @param {string} options.mint - captured by the CALLER before any await; a dialog's
 *   own token data can be nulled while the trade dialog is open.
 * @param {string} [options.symbol]
 * @param {number} [options.balance] - fetched when omitted (buy/add only)
 * @param {number} [options.holdings] - required to size a sell
 * @param {number} [options.decimals]
 * @param {Object} [options.context] - extra fields merged into the dialog context
 *   (e.g. the tokens page's `entrySize` / `entrySizes` DCA presets for "add")
 * @param {HTMLElement} [options.btn] - disabled while the request is in flight
 * @returns {Promise<boolean>} true when a trade was actually placed (false = user
 *   cancelled, or it failed)
 */
export async function manualTrade({
  action,
  mint,
  symbol = "?",
  balance,
  holdings,
  decimals,
  context = {},
  btn = null,
}) {
  if (!action || !mint) {
    Utils.showToast("No mint address available", "error");
    return false;
  }

  try {
    const dialogContext = { mint, ...context };
    if (action === "sell") {
      dialogContext.holdings = holdings ?? 0;
      if (decimals != null) dialogContext.decimals = decimals;
    } else {
      dialogContext.balance = balance ?? (await fetchWalletBalance());
    }

    const result = await getDialog().open({ action, mint, symbol, context: dialogContext });
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
