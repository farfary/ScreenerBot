// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The Home Auto Trader and Copy Trading status cards.
 *
 * Each card shows its feature's state, a few live figures and a link to its page; while
 * the feature is unavailable (Explore Mode, or setup not complete) it says so and offers
 * setup instead of the figures. Every figure is the server's (`auto_trader` and
 * `copy_trading` in `/api/dashboard/home`): the page only formats it, and an absent
 * figure is a dash. Each card repaints only when what it shows changed.
 */
import { paintText } from "../../core/dom.js";

const ABSENT = "—";

const STATE_LABELS = Object.freeze({
  running: "home-status-running",
  stopped: "home-status-stopped",
  unavailable: "home-status-setup-required",
});

function count(value, Utils) {
  return Utils.formatNumber(value, { decimals: 0, fallback: ABSENT });
}

function stateLabel(state) {
  return Object.hasOwn(STATE_LABELS, state) ? I18n.label(STATE_LABELS, state) : ABSENT;
}

/**
 * What the Auto Trader card shows for the server's card. Pure: `Utils` supplies the
 * formatters, so it runs outside a browser.
 */
export function describeAutoTrader(card, Utils) {
  const state = card?.state ?? null;
  const used = card?.open_positions;
  const limit = card?.max_open_positions;
  return {
    state,
    stateText: stateLabel(state),
    unavailable: state === "unavailable",
    openPositions:
      used == null || limit == null
        ? ABSENT
        : I18n.t("home-status-capacity", {
            used: count(used, Utils),
            limit: count(limit, Utils),
          }),
    entriesToday: count(card?.entries_today, Utils),
    exitsToday: count(card?.exits_today, Utils),
  };
}

/** What the Copy Trading card shows for the server's card; pure, like `describeAutoTrader`. */
export function describeCopyTrading(card, Utils) {
  const state = card?.state ?? null;
  const active = card?.active_tasks;
  const total = card?.total_tasks;
  return {
    state,
    stateText: stateLabel(state),
    unavailable: state === "unavailable",
    followedWallets: count(card?.followed_wallets, Utils),
    activeTasks:
      active == null || total == null
        ? ABSENT
        : I18n.t("home-status-capacity", {
            used: count(active, Utils),
            limit: count(total, Utils),
          }),
    lastCopy: Utils.formatTimeAgo(card?.last_copy_at, { fallback: ABSENT }),
  };
}

/**
 * Bind both cards to the Home page DOM. `onSetup` opens the setup wizard from either
 * card's setup action.
 * @returns {{ update:(data:object)=>void, dispose:()=>void }}
 */
export function createStatusCards(Utils, onSetup) {
  const lastKeys = { trader: null, copy: null };
  const roots = ["homeTraderStatus", "homeCopyStatus"]
    .map((id) => document.getElementById(id))
    .filter(Boolean);

  const onClick = (event) => {
    if (event.target.closest("[data-status-setup]")) onSetup();
  };
  roots.forEach((root) => root.addEventListener("click", onClick));

  // The state line, then either the figures or the setup note. The key is the
  // description itself, so a relative time that moved repaints and nothing else does.
  function paintCard(name, prefix, view, figures) {
    const key = JSON.stringify(view);
    if (key === lastKeys[name]) return;
    lastKeys[name] = key;

    const root = document.getElementById(`home${prefix}Status`);
    const stateEl = document.getElementById(`${name}StatusState`);
    if (!root || !stateEl) return;
    root.dataset.state = view.state ?? "";
    paintText(stateEl, view.stateText);

    const ledger = document.getElementById(`${name}StatusLedger`);
    const setup = document.getElementById(`${name}StatusSetup`);
    if (ledger) ledger.hidden = view.unavailable;
    if (setup) setup.hidden = !view.unavailable;
    for (const [id, text] of figures) paintText(document.getElementById(id), text);
  }

  return {
    update(data) {
      if (!data) return;
      const trader = describeAutoTrader(data.auto_trader, Utils);
      paintCard("trader", "Trader", trader, [
        ["traderOpenPositions", trader.openPositions],
        ["traderEntriesToday", trader.entriesToday],
        ["traderExitsToday", trader.exitsToday],
      ]);
      const copy = describeCopyTrading(data.copy_trading, Utils);
      paintCard("copy", "Copy", copy, [
        ["copyFollowedWallets", copy.followedWallets],
        ["copyActiveTasks", copy.activeTasks],
        ["copyLastCopy", copy.lastCopy],
      ]);
    },

    dispose() {
      roots.forEach((root) => root.removeEventListener("click", onClick));
      lastKeys.trader = null;
      lastKeys.copy = null;
    },
  };
}
