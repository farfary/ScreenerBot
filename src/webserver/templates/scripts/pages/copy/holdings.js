// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The Holdings tab: the task's open paper rounds marked at their latest price with where
// each exit rule acts, closed rounds with their result, and the paper-run actions.
import { loadPage } from "../../core/router.js";
import { getIdentity } from "../../ui/token_identity.js";
import {
  dateTime,
  duration,
  exitLabel,
  price,
  priceSol,
  seconds,
  segmented,
  signedPct,
  signedSolCell,
  skipKey,
  skipLabel,
  solCell,
  taskName,
  toneClass,
} from "./format.js";
import { ensureIdentities, openTokenDetails, tokenWithMint } from "./tokens.js";
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

/** The stagnation exit's flat band, with its wait when it does not act yet. */
function flatText(low, high, from) {
  const span = untilSpan(from);
  return span
    ? I18n.t("copy-holdings-watch-flat-until", { low, high, span })
    : I18n.t("copy-holdings-watch-flat", { low, high });
}

/**
 * One round of one task's holding. `opened_at` is kept as the opaque string the backend
 * sent: a reopened position of the same mint is a different round with a different value.
 */
const roundKey = (taskId, mint, openedAt) => JSON.stringify([taskId, mint, openedAt]);

/** Signature prefixes of the skipped intents a manual close records (workspace/book.rs). */
const MANUAL_SKIP_PREFIXES = [
  "manual-close-skip:",
  "manual-close-queue:",
  "manual-close-unavailable:",
];

const isManualSale = (outcome) =>
  outcome.outcome === "paper_sell_observed" &&
  outcome.exit_rule === "manual" &&
  Boolean(outcome.paper_fill);

const isManualSkip = (outcome) =>
  outcome.outcome === "skipped" &&
  outcome.paper?.model === "quote" &&
  typeof outcome.signature === "string" &&
  MANUAL_SKIP_PREFIXES.some((prefix) => outcome.signature.startsWith(prefix));

/** The newest activity id of a workspace: later rows were recorded after it was read. */
const newestActivityId = (ws) =>
  (ws?.activity || []).reduce((newest, row) => Math.max(newest, Number(row.id) || 0), 0);

export function createHoldings(page, { rerender, showActivityFor }) {
  const { Utils, api, on, notify, confirm } = page;
  const esc = Utils.escapeHtml;
  let view = "open";
  let lastWs = null;
  /**
   * Manual closes this page started, by round key. `requesting` is true while the HTTP call
   * is open; afterwards the entry stays until the workspace no longer reports the close as
   * pending and the recorded result has been shown.
   */
  const closing = new Map();
  /** Round keys (and reset task keys) whose confirmation dialog is open. */
  const confirming = new Set();

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
      const { holdingAction: action, mint, openedAt } = button.dataset;
      if (action === "details") openTokenDetails(mint);
      else if (action === "activity") showActivityFor(mint);
      else if (action === "positions") loadPage("positions");
      else if (action === "close") void close(mint, openedAt);
      else if (action === "reset") void reset(button);
    });
  }

  function tokenName(mint) {
    return getIdentity(mint).symbol || I18n.t("format-unknown");
  }

  function tokenCell(mint) {
    return `<button class="copy-token-link" type="button" data-holding-action="details" data-mint="${esc(mint)}" title="${esc(`${I18n.t("copy-holdings-token-details")} · ${mint}`)}">${tokenWithMint(mint)}</button>`;
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
        stopText(
          signedPct(relative(watch.stop_loss_price_native, entry)),
          watch.stop_loss_armed_at
        ),
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
    if (watch.break_even_floor_price_native != null) {
      const level = signedPct(relative(watch.break_even_floor_price_native, entry));
      items.push([
        watch.break_even_armed || watch.break_even_activation_price_native == null
          ? I18n.t("copy-holdings-watch-floor", { level })
          : I18n.t("copy-holdings-watch-floor-arms", {
              level,
              arms: signedPct(relative(watch.break_even_activation_price_native, entry)),
            }),
        watch.break_even_floor_price_native,
      ]);
    }
    if (watch.time_rule_price_native != null) {
      items.push([
        timeText(signedPct(relative(watch.time_rule_price_native, entry)), watch.time_rule_from),
        watch.time_rule_price_native,
      ]);
    }
    if (
      watch.stagnation_band_low_price_native != null &&
      watch.stagnation_band_high_price_native != null
    ) {
      items.push([
        flatText(
          signedPct(relative(watch.stagnation_band_low_price_native, entry)),
          signedPct(relative(watch.stagnation_band_high_price_native, entry)),
          watch.stagnation_from
        ),
        null,
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
    const rows = holdings
      .map((holding) => {
        const priced = holding.mark_price_native != null;
        const pending =
          holding.close_pending || closing.has(roundKey(ws.id, holding.mint, holding.opened_at));
        const pnl = priced
          ? `<span class="${toneClass(holding.unrealized_pnl_native)}">${esc(signedSolCell(holding.unrealized_pnl_native))}</span><small>${esc(signedPct(holding.unrealized_pnl_pct))}</small>`
          : `<span class="copy-warning-text">${esc(I18n.t("copy-holdings-no-price"))}</span>`;
        return `<tr>
          <td>${tokenCell(holding.mint)}</td>
          <td class="num">${esc(solCell(holding.cost_basis_native))}</td>
          <td class="num">${esc(price(holding.entry_price_native))}</td>
          <td class="num">${esc(price(holding.mark_price_native))}</td>
          ${relativeCell(holding.peak_price_native, holding.entry_price_native)}
          <td class="num copy-cell-stack">${pnl}</td>
          <td>${exitCell(holding, ws)}</td>
          <td class="num" title="${esc(I18n.t("copy-holdings-opened", { time: dateTime(holding.opened_at) }))}">${esc(duration(holding.held_seconds))}</td>
          <td class="copy-row-actions">
            <button class="btn btn-secondary btn-sm" type="button" data-holding-action="close" data-mint="${esc(holding.mint)}" data-opened-at="${esc(holding.opened_at)}"${pending ? " disabled" : ""}>${esc(pending ? I18n.t("copy-holdings-close-pending") : I18n.t("copy-holdings-close"))}</button>
            <button class="btn btn-ghost btn-sm" type="button" data-holding-action="activity" data-mint="${esc(holding.mint)}">${esc(I18n.t("copy-holdings-activity"))}</button>
          </td>
        </tr>`;
      })
      .join("");
    return `<div class="copy-table-wrap"><table class="copy-table"><thead><tr><th scope="col">${esc(I18n.t("copy-holdings-col-token"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-cost"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-entry"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-mark"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-peak"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-pnl"))}</th><th scope="col">${esc(I18n.t("copy-holdings-col-exit-rules"))}</th><th scope="col" class="num">${esc(I18n.t("copy-holdings-col-held"))}</th><th scope="col"><span class="sr-only">${esc(I18n.t("copy-holdings-col-actions"))}</span></th></tr></thead><tbody>${rows}</tbody></table></div>
    ${pausedNote(ws)}${exitMonitorNote(ws)}<p class="copy-note">${esc(I18n.t("copy-holdings-round-note"))}</p>`;
  }

  /** Exits of paper rounds run in the Auto Trader's exit monitor; say when it is stopped. */
  function exitMonitorNote(ws) {
    if (ws.exit_rules_running !== false) return "";
    const note =
      ws.exit_mode === "buy_only"
        ? I18n.t("copy-holdings-trader-stopped-rules")
        : I18n.t("copy-holdings-trader-stopped-wallet");
    return `<p class="copy-note copy-warning-text">${esc(note)}</p>`;
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
    const rows = rounds
      .map(
        (round) => `<tr>
          <td>${tokenCell(round.mint)}</td>
          <td class="num">${esc(solCell(round.invested_native))}</td>
          <td class="num">${esc(solCell(round.proceeds_native))}</td>
          <td class="num copy-cell-stack"><span class="${toneClass(round.pnl_sol)}">${esc(signedSolCell(round.pnl_sol))}</span><small>${esc(signedPct(round.pnl_pct))}</small></td>
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
      : `<button class="btn btn-ghost btn-sm copy-danger-action" type="button" data-holding-action="reset"><i class="icon-rotate-ccw" aria-hidden="true"></i> ${esc(I18n.t("copy-holdings-restart"))}</button>`;
    const head = `<div class="copy-panel-head"><h3>${esc(I18n.t("copy-holdings-title"))}</h3><div class="copy-panel-tools">${segmented("holdings-view", views, view, esc, I18n.t("copy-holdings-view-label"))}${reset}</div></div>`;
    if (view === "closed") return head + closedTable(insights, error);
    if (live) {
      return `${head}<div class="copy-panel-message">${esc(I18n.t("copy-holdings-live-note"))}<button class="btn btn-secondary btn-sm" type="button" data-holding-action="positions">${esc(I18n.t("copy-holdings-open-positions"))}</button></div>`;
    }
    return head + openTable(open, ws);
  }

  /** Rows this close recorded for its task and mint, newest first. */
  async function recordedRows(entry) {
    const response = await api.activity(entry.taskId, {
      filter: "all",
      mint: entry.mint,
      limit: 10,
    });
    return (response.activity || [])
      .filter(
        (row) =>
          Number(row.id) > entry.baseline &&
          row.outcome?.task_id === entry.taskId &&
          row.outcome?.mint === entry.mint
      )
      .sort((a, b) => Number(b.id) - Number(a.id));
  }

  /**
   * How the workspace shows the closed round and its mint: whether that round is still
   * open, was seen closed, and whether a different round of the mint is open.
   */
  function roundState(ws, entry) {
    const holdings = (ws.paper_holdings || []).filter((item) => item.mint === entry.mint);
    const same = holdings.filter((item) => item.opened_at === entry.openedAt);
    return {
      original: same.find((item) => item.open) || null,
      closed: same.some((item) => !item.open),
      other: holdings.some((item) => item.open && item.opened_at !== entry.openedAt),
    };
  }

  /**
   * The factual result of a close the page no longer waits on. Only the manual sale or
   * manual skip recorded for this task and mint answers it, and a different round of the
   * mint is never read as the answer for the round that was closed.
   */
  function reportClose({ entry, round, rows }) {
    const token = tokenName(entry.mint);
    const manual = rows.find((row) => isManualSale(row.outcome) || isManualSkip(row.outcome));
    if (entry.failure?.status !== undefined) {
      notify("error", I18n.t("copy-holdings-close-failed"), entry.failure.detail);
    } else if (manual && isManualSale(manual.outcome) && !round.original && !round.other) {
      notify(
        "success",
        I18n.t("copy-holdings-closed", { token }),
        I18n.t("copy-holdings-sold-at", {
          price: priceSol(manual.outcome.paper_fill.fill_price_sol),
        })
      );
    } else if (manual && isManualSkip(manual.outcome) && round.original) {
      notify(
        "error",
        I18n.t("copy-holdings-close-failed"),
        skipLabel(skipKey(manual.outcome.reason))
      );
    } else if (round.closed && !round.original && !round.other) {
      notify(
        "info",
        I18n.t("copy-holdings-closed", { token }),
        I18n.t("copy-holdings-closed-check-activity")
      );
    } else {
      notify(
        "warning",
        I18n.t("copy-holdings-close-unconfirmed"),
        I18n.t("copy-holdings-close-unconfirmed-detail")
      );
    }
  }

  /**
   * Reconciles a close whose request has ended with the backend's state. While the
   * workspace reports it pending the entry stays; nothing is inferred from elapsed time.
   */
  async function settle(entry) {
    if (entry.settling) return;
    entry.settling = true;
    try {
      const ws = await api.workspace(entry.taskId);
      const round = roundState(ws, entry);
      if (round.original?.close_pending) {
        if (!entry.noticed) {
          entry.noticed = true;
          notify(
            "info",
            I18n.t("copy-holdings-close-pending"),
            I18n.t("copy-holdings-close-pending-detail")
          );
        }
        return;
      }
      const rows = await recordedRows(entry);
      if (closing.get(entry.key) === entry) closing.delete(entry.key);
      reportClose({ entry, round, rows });
      await page.reload();
    } catch {
      // The workspace or activity could not be read; the close stays tracked as pending.
    } finally {
      entry.settling = false;
    }
  }

  /** Settles every ended request of this task whose round the workspace no longer reports pending. */
  function settleFinished(ws) {
    closing.forEach((entry) => {
      if (entry.requesting || entry.settling || entry.taskId !== ws.id) return;
      if (!roundState(ws, entry).original?.close_pending) void settle(entry);
    });
  }

  /** Takes the latest workspace read on every poll, whichever tab is shown. */
  function sync(ws) {
    lastWs = ws;
    settleFinished(ws);
  }

  async function close(mint, openedAt) {
    const ws = lastWs;
    const holding = ws?.paper_holdings?.find(
      (item) => item.mint === mint && item.opened_at === openedAt && item.open
    );
    if (!holding || holding.close_pending) return;
    const key = roundKey(ws.id, mint, holding.opened_at);
    if (closing.has(key) || confirming.has(key)) return;
    const token = tokenName(mint);
    const entry = {
      key,
      taskId: ws.id,
      mint,
      openedAt: holding.opened_at,
      baseline: newestActivityId(ws),
      requesting: true,
      settling: false,
      noticed: false,
      failure: null,
    };
    confirming.add(key);
    let result;
    try {
      result = await confirm({
        title: I18n.t("copy-holdings-close-title"),
        message: I18n.t("copy-holdings-close-quote-message", {
          token,
          delay: seconds(ws.paper_execution?.landing_delay_ms, { precise: true }),
        }),
        confirmLabel: I18n.t("copy-holdings-close-confirm"),
        cancelLabel: I18n.t("copy-holdings-keep"),
        variant: "warning",
      });
    } finally {
      confirming.delete(key);
    }
    if (!result.confirmed || closing.has(key)) return;
    closing.set(key, entry);
    rerender();
    void page.reload();
    try {
      const closed = await api.closeHolding(entry.taskId, mint, entry.openedAt);
      if (closing.get(key) === entry) closing.delete(key);
      notify(
        "success",
        I18n.t("copy-holdings-closed", { token }),
        I18n.t("copy-holdings-sold-at", { price: priceSol(closed.fill_price_native) })
      );
      await page.reload();
    } catch (error) {
      entry.requesting = false;
      entry.failure = error;
      await page.reload();
      await settle(entry);
    }
  }

  async function reset(button) {
    const ws = lastWs;
    if (!ws) return;
    const { id } = ws;
    const key = JSON.stringify(["reset", id]);
    if (confirming.has(key)) return;
    confirming.add(key);
    let result;
    try {
      result = await confirm({
        title: I18n.t("copy-holdings-restart"),
        message: I18n.t("copy-holdings-restart-message", { name: taskName(ws) }),
        confirmLabel: I18n.t("copy-holdings-restart"),
        cancelLabel: I18n.t("copy-holdings-restart-cancel"),
        variant: "danger",
      });
    } finally {
      confirming.delete(key);
    }
    if (!result.confirmed) return;
    button.disabled = true;
    try {
      const outcome = await api.reset(id);
      notify(
        "success",
        I18n.t("copy-holdings-restart-done"),
        I18n.t("copy-holdings-restart-detail", { count: outcome.closed_rounds })
      );
      await page.reload();
    } catch (error) {
      notify("error", I18n.t("copy-holdings-restart-failed"), error.detail);
      if (button.isConnected) button.disabled = false;
    }
  }

  return {
    setup,
    html,
    sync,
    reset: () => {
      view = "open";
    },
  };
}
