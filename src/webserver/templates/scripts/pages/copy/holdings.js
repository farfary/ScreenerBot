// The Holdings tab: open paper holdings marked at the pool price with where each
// exit rule acts, closed rounds with their result, and the paper-book actions.
import { loadPage } from "../../core/router.js";
import { getIdentity } from "../../ui/token_identity.js";
import {
  EXIT_LABELS,
  dateTime,
  duration,
  price,
  segmented,
  shortAddress,
  signedPct,
  signedSol,
  sol,
  taskName,
  toneClass,
} from "./format.js";
import { ensureIdentities, openTokenDetails, tokenInline } from "./tokens.js";
import { panelMessage } from "./overview.js";

const relative = (trigger, entry) =>
  trigger != null && entry > 0 ? (Number(trigger) / Number(entry) - 1) * 100 : null;

function untilText(value) {
  if (!value) return "";
  const seconds = (new Date(value).getTime() - Date.now()) / 1000;
  return seconds > 0 ? ` in ${duration(seconds)}` : "";
}

export function createHoldings(page, { rerender, showActivityFor }) {
  const { Utils, api, on, toast, confirm } = page;
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

  function tokenCell(mint) {
    return `<button class="copy-token-link" type="button" data-holding-action="details" data-mint="${esc(mint)}" title="${esc(`Open token details · ${mint}`)}">${tokenInline(mint)}</button>`;
  }

  function exitCell(holding, ws) {
    const watch = holding.exit_watch;
    if (!watch) return '<span class="copy-muted">Wallet sells only</span>';
    const entry = holding.entry_price_sol;
    const items = [];
    if (watch.stop_loss_price_sol != null) {
      items.push([
        `Stop ${signedPct(relative(watch.stop_loss_price_sol, entry))}${untilText(watch.stop_loss_armed_at)}`,
        watch.stop_loss_price_sol,
      ]);
    }
    if (watch.take_profit_price_sol != null) {
      items.push([
        `Take ${signedPct(relative(watch.take_profit_price_sol, entry))}`,
        watch.take_profit_price_sol,
      ]);
    }
    if (watch.trailing_armed && watch.trailing_stop_price_sol != null) {
      items.push([
        `Trail ${signedPct(relative(watch.trailing_stop_price_sol, entry))}`,
        watch.trailing_stop_price_sol,
      ]);
    } else if (watch.trailing_activation_price_sol != null) {
      items.push([
        `Trail arms ${signedPct(relative(watch.trailing_activation_price_sol, entry))}`,
        watch.trailing_activation_price_sol,
      ]);
    }
    if (watch.time_rule_price_sol != null) {
      items.push([
        `Time ≤ ${signedPct(relative(watch.time_rule_price_sol, entry))}${untilText(watch.time_rule_from)}`,
        watch.time_rule_price_sol,
      ]);
    }
    if (ws.exit_mode === "hybrid") items.push(["Wallet sells", null]);
    if (!items.length) return '<span class="copy-warning-text">No exit rule</span>';
    return `<span class="copy-exit-list">${items
      .map(
        ([text, at]) =>
          `<span${at != null ? ` title="${esc(`${price(at)} SOL`)}"` : ""}>${esc(text)}</span>`
      )
      .join("")}</span>`;
  }

  function openTable(holdings, ws) {
    if (!holdings.length) {
      return panelMessage("No open paper holdings. Buys copied from the wallet appear here.", esc);
    }
    const rows = holdings
      .map((holding) => {
        const priced = holding.mark_price_sol != null;
        const pnl = priced
          ? `<span class="${toneClass(holding.unrealized_pnl_sol)}">${esc(signedSol(holding.unrealized_pnl_sol))}</span><small>${esc(signedPct(holding.unrealized_pnl_pct))}</small>`
          : '<span class="copy-warning-text">No pool price</span>';
        return `<tr>
          <td>${tokenCell(holding.mint)}</td>
          <td class="num">${esc(sol(holding.cost_basis_sol))}</td>
          <td class="num">${esc(price(holding.entry_price_sol))}</td>
          <td class="num">${esc(price(holding.mark_price_sol))}</td>
          <td class="num">${esc(price(holding.peak_price_sol))}</td>
          <td class="num copy-cell-stack">${pnl}</td>
          <td>${exitCell(holding, ws)}</td>
          <td class="num" title="${esc(`Opened ${dateTime(holding.opened_at)}`)}">${esc(duration(holding.held_seconds))}</td>
          <td class="copy-row-actions">
            <button class="btn btn-secondary btn-sm" type="button" data-holding-action="close" data-mint="${esc(holding.mint)}">${priced ? "Close" : "Write off"}</button>
            <button class="btn btn-ghost btn-sm" type="button" data-holding-action="activity" data-mint="${esc(holding.mint)}">Activity</button>
          </td>
        </tr>`;
      })
      .join("");
    return `<div class="copy-table-wrap"><table class="copy-table"><thead><tr><th scope="col">Token</th><th scope="col" class="num">Cost</th><th scope="col" class="num">Entry</th><th scope="col" class="num">Mark</th><th scope="col" class="num">Peak</th><th scope="col" class="num">P&amp;L</th><th scope="col">Exit rules</th><th scope="col" class="num">Held</th><th scope="col"><span class="sr-only">Actions</span></th></tr></thead><tbody>${rows}</tbody></table></div>
    <p class="copy-note">Prices are SOL per token at the pool. Exit levels are relative to the entry price; hover one for its price.</p>`;
  }

  function closedTable(insights, error) {
    if (!insights) {
      return error
        ? panelMessage(`Closed rounds could not be loaded: ${error}`, esc, "is-error")
        : panelMessage("Loading closed rounds…", esc);
    }
    const rounds = insights.recent_rounds || [];
    if (!rounds.length) return panelMessage("No closed rounds yet.", esc);
    const rows = rounds
      .map(
        (round) => `<tr>
          <td>${tokenCell(round.mint)}</td>
          <td class="num">${esc(sol(round.invested_sol))}</td>
          <td class="num">${esc(sol(round.proceeds_sol))}</td>
          <td class="num copy-cell-stack"><span class="${toneClass(round.pnl_sol)}">${esc(signedSol(round.pnl_sol))}</span><small>${esc(signedPct(round.pnl_pct))}</small></td>
          <td>${esc(EXIT_LABELS[round.exit] || round.exit)}</td>
          <td class="num">${esc(duration(round.hold_seconds))}</td>
          <td>${esc(dateTime(round.closed_at))}</td>
          <td class="copy-row-actions"><button class="btn btn-ghost btn-sm" type="button" data-holding-action="activity" data-mint="${esc(round.mint)}">Activity</button></td>
        </tr>`
      )
      .join("");
    const shown =
      rounds.length < insights.rounds
        ? `<p class="copy-note">Latest ${rounds.length} of ${insights.rounds} rounds.</p>`
        : "";
    return `<div class="copy-table-wrap"><table class="copy-table"><thead><tr><th scope="col">Token</th><th scope="col" class="num">Invested</th><th scope="col" class="num">Proceeds</th><th scope="col" class="num">P&amp;L</th><th scope="col">Exit</th><th scope="col" class="num">Held</th><th scope="col">Closed</th><th scope="col"><span class="sr-only">Actions</span></th></tr></thead><tbody>${rows}</tbody></table></div>${shown}`;
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
      { id: "open", label: `Open (${live ? (ws.stats?.open_positions ?? 0) : open.length})` },
      { id: "closed", label: `Closed rounds (${insights ? insights.rounds : "…"})` },
    ];
    const reset = live
      ? ""
      : '<button class="btn btn-ghost btn-sm copy-danger-action" type="button" data-holding-action="reset"><i class="icon-rotate-ccw" aria-hidden="true"></i> Reset paper book</button>';
    const head = `<div class="copy-panel-head"><h3>Holdings</h3><div class="copy-panel-tools">${segmented("holdings-view", views, view, esc, "Holdings view")}${reset}</div></div>`;
    if (view === "closed") return head + closedTable(insights, error);
    if (live) {
      return `${head}<div class="copy-panel-message">Live copies are real positions.<button class="btn btn-secondary btn-sm" type="button" data-holding-action="positions">Open Positions</button></div>`;
    }
    return head + openTable(open, ws);
  }

  async function close(mint, button) {
    const holding = lastWs?.paper_holdings?.find((item) => item.mint === mint && item.open);
    if (!holding) return;
    const name = tokenName(mint);
    const priced = holding.mark_price_sol != null;
    const result = await confirm(
      priced
        ? {
            title: "Close paper holding",
            message: `Sell ${name} in the paper book at the pool price (${price(holding.mark_price_sol)} SOL) with the task's slippage and fees.`,
            confirmLabel: "Close holding",
            cancelLabel: "Keep",
            variant: "warning",
          }
        : {
            title: "Write off paper holding",
            message: `${name} has no pool price to sell at. Writing it off closes it at zero and books its ${sol(holding.cost_basis_sol)} cost as a loss.`,
            confirmLabel: "Write off",
            cancelLabel: "Keep",
            variant: "danger",
          }
    );
    if (!result.confirmed) return;
    button.disabled = true;
    try {
      const closed = await api.closeHolding(lastWs.id, mint);
      toast(
        "success",
        closed.written_off ? `${name} written off` : `${name} closed`,
        closed.written_off
          ? "Closed at zero proceeds"
          : `Sold at ${price(closed.mark_price_sol)} SOL`
      );
      await page.reload();
    } catch (error) {
      toast("error", "Holding could not be closed", error.detail);
      if (button.isConnected) button.disabled = false;
    }
  }

  async function reset(button) {
    if (!lastWs) return;
    const result = await confirm({
      title: "Reset paper book",
      message: `Start “${taskName(lastWs)}” over: its paper holdings, spend, fills, exits and skips are removed. The rules and the wallet stay.`,
      confirmLabel: "Reset paper book",
      cancelLabel: "Keep history",
      variant: "danger",
    });
    if (!result.confirmed) return;
    button.disabled = true;
    try {
      const outcome = await api.reset(lastWs.id);
      toast("success", "Paper book reset", `${outcome.removed_decisions} decisions removed`);
      await page.reload();
    } catch (error) {
      toast("error", "Paper book could not be reset", error.detail);
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
