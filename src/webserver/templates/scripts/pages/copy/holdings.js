// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The Holdings tab: open paper holdings marked at the pool price with where each
// exit rule acts, closed rounds with their result, and the paper-book actions.
import { loadPage } from "../../core/router.js";
import { getIdentity } from "../../ui/token_identity.js";
import {
  dateTime,
  duration,
  exitLabel,
  price,
  priceSol,
  segmented,
  shortAddress,
  signedPct,
  signedSol,
  sol,
  taskName,
  toneClass,
} from "./format.js";
import { ensureIdentities, openTokenDetails, sharedSymbols, tokenInline } from "./tokens.js";
import { panelMessage } from "./overview.js";

const relative = (trigger, entry) =>
  trigger != null && entry > 0 ? (Number(trigger) / Number(entry) - 1) * 100 : null;

/** The time until a rule acts, or an empty string once it can act or has no time. */
function untilSpan(value) {
  if (!value) return "";
  const seconds = (new Date(value).getTime() - Date.now()) / 1000;
  return seconds > 0 ? duration(seconds) : "";
}

/** A stop rule's level, with its wait when it is not armed yet. */
function stopText(level, armedAt) {
  const span = untilSpan(armedAt);
  return span
    ? I18n.t("copy-holdings-watch-stop-until", { level, span })
    : I18n.t("copy-holdings-watch-stop", { level });
}

/** A time rule's level, with its wait when it does not act yet. */
function timeText(level, from) {
  const span = untilSpan(from);
  return span
    ? I18n.t("copy-holdings-watch-time-until", { level, span })
    : I18n.t("copy-holdings-watch-time", { level });
}

export function createHoldings(page, { rerender, showActivityFor }) {
  const { Utils, api, on, notify, confirm } = page;
  const esc = Utils.escapeHtml;
  let view = "open";
  let lastWs = null;

  function setup(root) {
    on(root, "click", (event) => {
      const seg = event.target.closest('[data-seg="holdings-view"] [data-seg-value]');
      if (seg) {
        view = seg.dataset.segValue;
        rerender();
        return;
      }
      const button = event.target.closest("[data-holding-action]");
      if (!button) return;
      const { holdingAction: action, mint } = button.dataset;
      if (action === "details") openTokenDetails(mint);
      else if (action === "activity") showActivityFor(mint);
      else if (action === "positions") loadPage("positions");
      else if (action === "close") void close(mint, button);
      else if (action === "reset") void reset(button);
    });
  }

  function tokenName(mint) {
    return getIdentity(mint).symbol || shortAddress(mint);
  }

  function tokenCell(mint, shared) {
    return `<button class="copy-token-link" type="button" data-holding-action="details" data-mint="${esc(mint)}" title="${esc(`${I18n.t("copy-holdings-token-details")} · ${mint}`)}">${tokenInline(mint, shared)}</button>`;
  }

  /** A price relative to the entry, with the price itself on hover. */
  function relativeCell(value, entry) {
    if (value == null) return '<td class="num">—</td>';
    return `<td class="num" title="${esc(priceSol(value))}">${esc(signedPct(relative(value, entry)))}</td>`;
  }

  function exitCell(holding, ws) {
    const watch = holding.exit_watch;
    if (!watch) {
      return `<span class="copy-muted">${esc(I18n.t("copy-rules-wallet-sells-only"))}</span>`;
    }
    const entry = holding.entry_price_native;
    const items = [];
    if (watch.stop_loss_price_native != null) {
      items.push([
        stopText(signedPct(relative(watch.stop_loss_price_native, entry)), watch.stop_loss_armed_at),
        watch.stop_loss_price_native,
      ]);
    }
    if (watch.take_profit_price_native != null) {
      items.push([
        I18n.t("copy-holdings-watch-take", {
          level: signedPct(relative(watch.take_profit_price_native, entry)),
        }),
        watch.take_profit_price_native,
      ]);
    }
    if (watch.trailing_armed && watch.trailing_stop_price_native != null) {
      items.push([
        I18n.t("copy-holdings-watch-trail", {
          level: signedPct(relative(watch.trailing_stop_price_native, entry)),
        }),
        watch.trailing_stop_price_native,
      ]);
    } else if (watch.trailing_activation_price_native != null) {
      items.push([
        I18n.t("copy-holdings-watch-trail-arms", {
          level: signedPct(relative(watch.trailing_activation_price_native, entry)),
        }),
        watch.trailing_activation_price_native,
      ]);
    }
    if (watch.time_rule_price_native != null) {
      items.push([
        timeText(signedPct(relative(watch.time_rule_price_native, entry)), watch.time_rule_from),
        watch.time_rule_price_native,
      ]);
    }
    if (ws.exit_mode === "hybrid") items.push([I18n.t("copy-holdings-watch-wallet-sells"), null]);
    if (!items.length) {
      return `<span class="copy-warning-text">${esc(I18n.t("copy-holdings-no-exit-rule"))}</span>`;
    }
    return `<span class="copy-exit-list">${items
      .map(
        ([text, at]) =>
          `<span${at != null ? ` title="${esc(priceSol(at))}"` : ""}>${esc(text)}</span>`
      )
      .join("")}</span>`;
  }

  function openTable(holdings, ws) {
    if (!holdings.length) {
      return panelMessage(I18n.t("copy-holdings-empty"), esc);
    }
    const shared = sharedSymbols(holdings.map((holding) => holding.mint));
    const rows = holdings
      .map((holding) => {
        const priced = holding.mark_price_native != null;
        const pnl = priced
          ? `<span class="${toneClass(holding.unrealized_pnl_native)}">${esc(signedSol(holding.unrealized_pnl_native))}</span><small>${esc(signedPct(holding.unrealized_pnl_pct))}</small>`
          : `<span class="copy-warning-text">${esc(I18n.t("copy-holdings-no-pool-price"))}</span>`;
        return `<tr>
          <td>${tokenCell(holding.mint, shared)}</td>
          <td class="num">${esc(sol(holding.cost_basis_native))}</td>
          <td class="num">${esc(price(holding.entry_price_native))}</td>
          <td class="num">${esc(price(holding.mark_price_native))}</td>
          ${relativeCell(holding.peak_price_native, holding.entry_price_native)}
          <td class="num copy-cell-stack">${pnl}</td>
          <td>${exitCell(holding, ws)}</td>
          <td class="num" title="${esc(I18n.t("copy-holdings-opened", { time: dateTime(holding.opened_at) }))}">${esc(duration(holding.held_seconds))}</td>
          <td class="copy-row-actions">
            <button class="btn btn-secondary btn-sm" type="button" data-holding-action="close" data-mint="${esc(holding.mint)}">${esc(priced ? I18n.t("copy-holdings-close") : I18n.t("copy-holdings-write-off"))}</button>
            <button class="btn btn-ghost btn-sm" type="button" data-holding-action="activity" data-mint="${esc(holding.mint)}">${esc(I18n.t("copy-holdings-activity"))}</button>
          </td>
        </tr>`;
      })
      .join("");
    return `<div class="copy-table-wrap"><table class="copy-table"><thead><tr><th scope="col">${esc(I18n.t("copy-holdings-col-token"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-cost"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-entry"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-mark"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-peak"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-pnl"))}</th><th scope="col">${esc(I18n.t("copy-holdings-col-exit-rules"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-held"))}</th><th scope="col"><span class="sr-only">${esc(I18n.t("copy-holdings-col-actions"))}</span></th></tr></thead><tbody>${rows}</tbody></table></div>
    ${pausedNote(ws)}<p class="copy-note">${esc(I18n.t("copy-holdings-price-note"))}</p>`;
  }

  /** A paused task still closes what it holds; say by what. */
  function pausedNote(ws) {
    if (ws.enabled) return "";
    const note =
      ws.exit_mode === "buy_only"
        ? I18n.t("copy-holdings-paused-rules")
        : ws.exit_mode === "mirror"
          ? I18n.t("copy-holdings-paused-mirror")
          : I18n.t("copy-holdings-paused-hybrid");
    return `<p class="copy-note">${esc(note)}</p>`;
  }

  function closedTable(insights, error) {
    if (!insights) {
      return error
        ? panelMessage(I18n.t("copy-holdings-closed-load-failed", { error }), esc, "is-error")
        : panelMessage(I18n.t("copy-holdings-closed-loading"), esc);
    }
    const rounds = insights.recent_rounds || [];
    if (!rounds.length) return panelMessage(I18n.t("copy-holdings-closed-empty"), esc);
    const shared = sharedSymbols(rounds.map((round) => round.mint));
    const rows = rounds
      .map(
        (round) => `<tr>
          <td>${tokenCell(round.mint, shared)}</td>
          <td class="num">${esc(sol(round.invested_native))}</td>
          <td class="num">${esc(sol(round.proceeds_native))}</td>
          <td class="num copy-cell-stack"><span class="${toneClass(round.pnl_sol)}">${esc(signedSol(round.pnl_sol))}</span><small>${esc(signedPct(round.pnl_pct))}</small></td>
          <td>${esc(exitLabel(round.exit))}</td>
          <td class="num">${esc(duration(round.hold_seconds))}</td>
          <td>${esc(dateTime(round.closed_at))}</td>
          <td class="copy-row-actions"><button class="btn btn-ghost btn-sm" type="button" data-holding-action="activity" data-mint="${esc(round.mint)}">${esc(I18n.t("copy-holdings-activity"))}</button></td>
        </tr>`
      )
      .join("");
    const shown =
      rounds.length < insights.rounds
        ? `<p class="copy-note">${esc(I18n.t("copy-holdings-closed-latest", { shown: rounds.length, total: insights.rounds }))}</p>`
        : "";
    return `<div class="copy-table-wrap"><table class="copy-table"><thead><tr><th scope="col">${esc(I18n.t("copy-holdings-col-token"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-invested"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-proceeds"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-pnl"))}</th><th scope="col">${esc(I18n.t("copy-holdings-col-exit"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-held"))}</th><th scope="col">${esc(I18n.t("copy-holdings-col-closed"))}</th><th scope="col"><span class="sr-only">${esc(I18n.t("copy-holdings-col-actions"))}</span></th></tr></thead><tbody>${rows}</tbody></table></div>${shown}`;
  }

  function html({ ws, insights, error }) {
    lastWs = ws;
    const live = ws.mode === "live";
    const open = (ws.paper_holdings || []).filter((holding) => holding.open);
    ensureIdentities(
      [...open, ...(insights?.recent_rounds || [])].map((item) => item.mint),
      rerender
    );
    const views = [
      {
        id: "open",
        label: I18n.t("copy-holdings-view-open", {
          count: live ? (ws.stats?.open_positions ?? 0) : open.length,
        }),
      },
      {
        id: "closed",
        label: I18n.t("copy-holdings-view-closed", { count: insights ? insights.rounds : "…" }),
      },
    ];
    const reset = live
      ? ""
      : `<button class="btn btn-ghost btn-sm copy-danger-action" type="button" data-holding-action="reset"><i class="icon-rotate-ccw" aria-hidden="true"></i> ${esc(I18n.t("copy-holdings-reset"))}</button>`;
    const head = `<div class="copy-panel-head"><h3>${esc(I18n.t("copy-holdings-title"))}</h3><div class="copy-panel-tools">${segmented("holdings-view", views, view, esc, I18n.t("copy-holdings-view-label"))}${reset}</div></div>`;
    if (view === "closed") return head + closedTable(insights, error);
    if (live) {
      return `${head}<div class="copy-panel-message">${esc(I18n.t("copy-holdings-live-note"))}<button class="btn btn-secondary btn-sm" type="button" data-holding-action="positions">${esc(I18n.t("copy-holdings-open-positions"))}</button></div>`;
    }
    return head + openTable(open, ws);
  }

  async function close(mint, button) {
    const holding = lastWs?.paper_holdings?.find((item) => item.mint === mint && item.open);
    if (!holding) return;
    const name = tokenName(mint);
    const priced = holding.mark_price_native != null;
    const result = await confirm(
      priced
        ? {
            title: I18n.t("copy-holdings-close-title"),
            message: I18n.t("copy-holdings-close-message", {
              token: name,
              price: priceSol(holding.mark_price_native),
            }),
            confirmLabel: I18n.t("copy-holdings-close-confirm"),
            cancelLabel: I18n.t("copy-holdings-keep"),
            variant: "warning",
          }
        : {
            title: I18n.t("copy-holdings-write-off-title"),
            message: I18n.t("copy-holdings-write-off-message", {
              token: name,
              cost: sol(holding.cost_basis_native),
            }),
            confirmLabel: I18n.t("copy-holdings-write-off"),
            cancelLabel: I18n.t("copy-holdings-keep"),
            variant: "danger",
          }
    );
    if (!result.confirmed) return;
    button.disabled = true;
    try {
      const closed = await api.closeHolding(lastWs.id, mint);
      notify(
        "success",
        closed.written_off
          ? I18n.t("copy-holdings-written-off", {
              token: name,
            })
          : I18n.t("copy-holdings-closed", {
              token: name,
            }),
        closed.written_off
          ? I18n.t("copy-holdings-written-off-detail")
          : I18n.t("copy-holdings-sold-at", { price: priceSol(closed.mark_price_native) })
      );
      await page.reload();
    } catch (error) {
      notify("error", I18n.t("copy-holdings-close-failed"), error.detail);
      if (button.isConnected) button.disabled = false;
    }
  }

  async function reset(button) {
    if (!lastWs) return;
    const result = await confirm({
      title: I18n.t("copy-holdings-reset"),
      message: I18n.t("copy-holdings-reset-message", { name: taskName(lastWs) }),
      confirmLabel: I18n.t("copy-holdings-reset"),
      cancelLabel: I18n.t("copy-holdings-reset-cancel"),
      variant: "danger",
    });
    if (!result.confirmed) return;
    button.disabled = true;
    try {
      const outcome = await api.reset(lastWs.id);
      notify(
        "success",
        I18n.t("copy-holdings-reset-done"),
        I18n.t("copy-holdings-reset-detail", { count: outcome.removed_decisions })
      );
      await page.reload();
    } catch (error) {
      notify("error", I18n.t("copy-holdings-reset-failed"), error.detail);
      if (button.isConnected) button.disabled = false;
    }
  }

  return {
    setup,
    html,
    reset: () => {
      view = "open";
      lastWs = null;
    },
  };
}
