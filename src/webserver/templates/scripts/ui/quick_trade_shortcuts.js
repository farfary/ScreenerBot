/**
 * Cmd/Ctrl+B and Cmd/Ctrl+Shift+S — the keyboard quick trade.
 *
 * This lived as a ~130-line inline `<script type="module">` in base.html, which
 * made it the FOURTH copy of the manual-trade flow and gave it two problems
 * nothing could see:
 *
 *   - it POSTed its own body with `amount_sol`, a field `ManualBuyRequest` does
 *     not have, so every keyboard buy silently spent the configured default
 *     trade size instead of the amount typed into the dialog;
 *   - it raised its own "order submitted" toast on top of the action stream's
 *     notice for the same trade.
 *
 * As a module it shares one `submitTrade` with the rest of the dashboard, so
 * the payload and the notices have a single owner. It must be imported with a
 * plain relative specifier (see `core/header.js`): a `?v=` script tag would
 * load a SECOND copy of every module it touches, including a second trade
 * dialog.
 */

import { TradeActionDialog } from "./trade_action_dialog.js";
import { submitTrade, fetchWalletBalance } from "./manual_trade.js";
import { playAcknowledge, playError } from "../core/sounds.js";
import { hasOpenOverlay } from "../core/escape_stack.js";

/** Screens that own the whole window; a trade shortcut must not fire behind them. */
const SYSTEM_SCREEN_IDS = [
  "splash-screen",
  "setupScreen",
  "setupScreenWrapper",
  "onboardingScreen",
  "lockscreenOverlay",
];

let dialog = null;

function isSystemScreenActive() {
  for (const id of SYSTEM_SCREEN_IDS) {
    const el = document.getElementById(id);
    if (el && el.style.display !== "none" && !el.classList.contains("hidden")) {
      return true;
    }
  }
  return document.body.classList.contains("initialization-mode");
}

/**
 * Open the trade dialog in quick mode — the only mode that asks for the mint
 * itself — and submit whatever the user confirms.
 * @param {"buy"|"sell"} action
 */
export async function openQuickTrade(action) {
  if (!dialog) dialog = new TradeActionDialog();

  const context = {};
  if (action === "buy") {
    context.balance = await fetchWalletBalance();
  }

  const result = await dialog.open({ action, mode: "quick", symbol: null, context });
  if (!result) return;

  const mint = dialog.currentContext?.mint;
  if (!mint) {
    playError();
    window.showToast?.({ type: "error", title: "No token selected" });
    return;
  }

  // No success toast here: the backend registers an action for this trade and
  // `core/action_toasts.js` shows the one notice that follows it to its result.
  const placed = await submitTrade({ action, mint, result });
  if (placed) {
    playAcknowledge();
  } else {
    playError();
  }
}

function handleKeydown(event) {
  // Never steal a keystroke the user is typing into a field.
  if (
    event.target.matches('input, textarea, select, [contenteditable="true"]') &&
    !event.target.closest(".search-dialog")
  ) {
    return;
  }

  if (isSystemScreenActive() || hasOpenOverlay()) return;

  const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
  const modifier = isMac ? event.metaKey : event.ctrlKey;
  if (!modifier) return;

  const key = event.key.toLowerCase();

  if (!event.shiftKey && key === "b") {
    event.preventDefault();
    openQuickTrade("buy");
    return;
  }

  if (event.shiftKey && key === "s") {
    event.preventDefault();
    openQuickTrade("sell");
  }
}

document.addEventListener("keydown", handleKeydown);
