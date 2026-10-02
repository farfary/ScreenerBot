// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The Activity tab: the task's whole decision history, paged from the server and
// filtered by kind or token. Consecutive skips for one reason fold into a group.
import {
  dateTime,
  duration,
  exitLabel,
  pct,
  price,
  seconds,
  segmented,
  signedPct,
  skipKey,
  skipLabel,
  sol,
} from "./format.js";
import { ensureIdentities, sharedSymbols, tokenInline } from "./tokens.js";
import { panelMessage } from "./overview.js";

const PAGE = 50;
const GROUP_MIN = 3;
const FILTERS = [
  {
    id: "all",
    get label() {
      return I18n.t("copy-filter-all");
    },
  },
  {
    id: "fills",
    get label() {
      return I18n.t("copy-kind-fills");
    },
  },
  {
    id: "exits",
    get label() {
      return I18n.t("copy-kind-exits");
    },
  },
  {
    id: "skips",
    get label() {
      return I18n.t("copy-kind-skips");
    },
  },
  {
    id: "errors",
    get label() {
      return I18n.t("copy-kind-errors");
    },
  },
];
// Outcome kinds serialized by `CopyOutcome` (src/trader/copy/types.rs).
const OUTCOME_LABELS = Object.freeze({
  paper_filled: "copy-outcome-paper-filled",
  live_submitted: "copy-outcome-live-submitted",
  live_confirmed: "copy-outcome-live-confirmed",
  live_failed: "copy-outcome-live-failed",
  paper_sell_observed: "copy-outcome-paper-sell-observed",
  live_sell_submitted: "copy-outcome-live-sell-submitted",
  live_sell_failed: "copy-outcome-live-sell-failed",
  skipped: "copy-outcome-skipped",
});
const TONES = {
  paper_filled: "is-fill",
  live_submitted: "is-fill",
  live_confirmed: "is-fill",
  live_failed: "is-error",
  paper_sell_observed: "is-exit",
  live_sell_submitted: "is-exit",
  live_sell_failed: "is-error",
  skipped: "is-skip",
};

/** Synthetic paper-exit and write-off keys are not chain transactions. */
const chainSignature = (value) =>
  typeof value === "string" && value && !value.startsWith("paper-") ? value : null;

function arrivalText(telemetry) {
  if (!telemetry?.target_block_time || !telemetry.detected_at) return "";
  const ms = new Date(telemetry.detected_at).getTime() - Number(telemetry.target_block_time) * 1000;
  if (!Number.isFinite(ms) || ms < 0) return "";
  const span = ms < 60_000 ? seconds(ms) : duration(ms / 1000);
  return telemetry.backfill
    ? I18n.t("copy-activity-arrival-replayed", {
        span,
      })
    : I18n.t("copy-activity-arrival-seen", {
        span,
      });
}

/** A skip reason's label followed by the figures that decided it. */
function skipDetail(reason) {
  const label = skipLabel(skipKey(reason));
  const withDetail = (detail) => I18n.t("copy-activity-skip-detail", { label, detail });
  switch (reason?.kind) {
    case "target_below_minimum":
      return withDetail(sol(reason.minimum_sol, 3));
    case "target_above_maximum":
      return withDetail(sol(reason.maximum_sol, 3));
    case "below_minimum_size":
      return withDetail(
        I18n.t("copy-activity-skip-minimum-size", { amount: sol(reason.minimum_sol, 4) })
      );
    case "invalid_slippage":
      return withDetail(
        I18n.t("copy-activity-skip-maximum", { value: pct(reason.maximum_pct, 1) })
      );
    case "stale_observation":
      return withDetail(
        I18n.t("copy-activity-skip-stale", {
          arrival: seconds(reason.arrival_ms),
          limit: seconds(reason.threshold_ms),
        })
      );
    case "latency_kill_switch":
      return withDetail(
        I18n.t("copy-activity-skip-latency", {
          average: seconds(reason.average_ms),
          limit: seconds(reason.threshold_ms),
        })
      );
    default:
      return label;
  }
}

export function createActivity(page, { rerender }) {
  const { Utils, api, on, notify } = page;
  const esc = Utils.escapeHtml;
  let taskId = null;
  let filter = "all";
  let mint = "";
  let rows = [];
  let nextBefore = null;
  let loaded = false;
  let error = null;
  let loadingOlder = false;
  let shared = new Set();
  const openGroups = new Set();

  function reset(id) {
    taskId = id;
    filter = "all";
    mint = "";
    clearRows();
    openGroups.clear();
  }

  function clearRows() {
    rows = [];
    nextBefore = null;
    loaded = false;
    error = null;
  }

  function requery() {
    clearRows();
    rerender();
    void refresh(taskId).then(rerender);
  }

  function setMint(value) {
    mint = String(value || "").trim();
    clearRows();
  }

  function setup(root) {
    on(root, "click", (event) => {
      const seg = event.target.closest('[data-seg="activity-filter"] [data-seg-value]');
      if (seg) {
        filter = seg.dataset.segValue;
        requery();
        return;
      }
      const token = event.target.closest("[data-activity-mint]");
      if (token) {
        setMint(token.dataset.activityMint);
        requery();
        return;
      }
      if (event.target.closest("[data-activity-clear]")) {
        setMint("");
        requery();
        return;
      }
      const older = event.target.closest("[data-activity-older]");
      if (older) void loadOlder(older);
    });
    on(root, "submit", (event) => {
      const form = event.target.closest("[data-activity-mint-form]");
      if (!form) return;
      event.preventDefault();
      setMint(form.elements.mint?.value);
      requery();
    });
    on(
      root,
      "toggle",
      (event) => {
        const group = event.target.closest?.("[data-group]");
        if (!group) return;
        if (group.open) openGroups.add(group.dataset.group);
        else openGroups.delete(group.dataset.group);
      },
      true
    );
  }

  async function refresh(id) {
    if (id !== taskId) reset(id);
    const asked = `${filter}|${mint}`;
    try {
      const response = await api.activity(id, { filter, mint, limit: PAGE });
      if (id !== taskId || asked !== `${filter}|${mint}`) return;
      const fresh = response.activity || [];
      const anchor = rows.length ? fresh.findIndex((row) => row.id === rows[0].id) : -1;
      if (anchor >= 0) {
        rows = [...fresh.slice(0, anchor), ...rows];
      } else {
        rows = fresh;
        nextBefore = response.next_before ?? null;
      }
      loaded = true;
      error = null;
    } catch (failure) {
      if (id === taskId) error = failure.detail;
    }
  }

  async function loadOlder(button) {
    if (!nextBefore || loadingOlder) return;
    loadingOlder = true;
    button.disabled = true;
    try {
      const response = await api.activity(taskId, {
        filter,
        mint,
        limit: PAGE,
        before: nextBefore,
      });
      const known = new Set(rows.map((row) => row.id));
      rows = rows.concat((response.activity || []).filter((row) => !known.has(row.id)));
      nextBefore = response.next_before ?? null;
    } catch (failure) {
      notify("error", I18n.t("copy-activity-older-failed"), failure.detail);
    } finally {
      loadingOlder = false;
      rerender();
    }
  }

  function title(outcome) {
    if (outcome.outcome === "paper_sell_observed" && outcome.exit_rule) {
      return I18n.t("copy-activity-paper-exit", { rule: exitLabel(outcome.exit_rule) });
    }
    return Object.hasOwn(OUTCOME_LABELS, outcome.outcome)
      ? I18n.label(OUTCOME_LABELS, outcome.outcome)
      : I18n.t("copy-activity-decision");
  }

  function detail(outcome) {
    switch (outcome.outcome) {
      case "paper_filled": {
        const fill = outcome.fill || {};
        const target = outcome.telemetry?.target_price_sol;
        const slip =
          fill.priced_from_pool && target > 0 && fill.fill_price_sol > 0
            ? (fill.fill_price_sol / target - 1) * 100
            : null;
        const values = {
          input: sol(fill.input_sol),
          price: price(fill.fill_price_sol),
          target: sol(outcome.target_size_sol, 3),
        };
        if (!fill.priced_from_pool) return I18n.t("copy-activity-filled-unpriced", values);
        return slip === null
          ? I18n.t("copy-activity-filled", values)
          : I18n.t("copy-activity-filled-slippage", { ...values, slippage: signedPct(slip, 2) });
      }
      case "live_submitted":
      case "live_confirmed":
      case "live_failed":
        return (
          outcome.error ||
          I18n.t("copy-activity-live-sized", {
            sized: sol(outcome.sized_sol),
            target: sol(outcome.target_size_sol, 3),
          })
        );
      case "paper_sell_observed": {
        const fill = outcome.paper_fill;
        if (!fill) {
          return I18n.t("copy-activity-sell-nothing", {
            amount: sol(outcome.target_sol_amount, 3),
          });
        }
        if (outcome.exit_rule === "manual" && !fill.fill_price_sol) {
          return I18n.t("copy-activity-written-off");
        }
        return I18n.t("copy-activity-sold", {
          tokens: Utils.formatNumber(fill.token_amount),
          proceeds: sol(fill.net_proceeds_sol),
          price: price(fill.fill_price_sol),
        });
      }
      case "live_sell_submitted":
      case "live_sell_failed":
        return (
          outcome.error ||
          (outcome.exit_percentage == null
            ? I18n.t("copy-activity-full-close")
            : I18n.t("copy-activity-partial-exit", { pct: pct(outcome.exit_percentage, 1) }))
        );
      case "skipped":
        return skipDetail(outcome.reason);
      default:
        return "";
    }
  }

  function links(outcome) {
    const target = chainSignature(outcome.signature || outcome.target_signature);
    const own = chainSignature(outcome.transaction_signature);
    const link = (signature, label) =>
      `<a class="copy-link" href="https://solscan.io/tx/${esc(signature)}" target="_blank" rel="noopener">${esc(label)} <i class="icon-external-link" aria-hidden="true"></i></a>`;
    return [
      target ? link(target, I18n.t("copy-activity-link-wallet-tx")) : "",
      own ? link(own, I18n.t("copy-activity-link-own-tx")) : "",
    ].join("");
  }

  function rowHtml(row) {
    const outcome = row.outcome || {};
    const at = outcome.telemetry?.decided_at || outcome.decided_at || row.created_at;
    const token = outcome.mint
      ? `<button class="copy-token-link" type="button" data-activity-mint="${esc(outcome.mint)}" title="${esc(`${I18n.t("copy-activity-only-token")} · ${outcome.mint}`)}">${tokenInline(outcome.mint, shared)}</button>`
      : "";
    const arrival = arrivalText(outcome.telemetry);
    return `<li class="copy-event ${TONES[outcome.outcome] || ""}">
      <time datetime="${esc(String(at || ""))}">${esc(dateTime(at))}</time>
      <div class="copy-event-main">
        <div class="copy-event-line"><strong>${esc(title(outcome))}</strong>${token}</div>
        <div class="copy-event-detail">${esc(detail(outcome))}</div>
        <div class="copy-event-meta">${arrival ? `<span>${esc(arrival)}</span>` : ""}${links(outcome)}</div>
      </div>
    </li>`;
  }

  function groupHtml(group) {
    if (group.rows.length < GROUP_MIN) return group.rows.map(rowHtml).join("");
    const first = group.rows[0];
    const last = group.rows[group.rows.length - 1];
    const tokens = new Set(group.rows.map((row) => row.outcome?.mint).filter(Boolean)).size;
    const id = String(first.id);
    const at = first.outcome?.decided_at || first.created_at;
    return `<li class="copy-event is-skip copy-event-group">
      <details data-group="${esc(id)}"${openGroups.has(id) ? " open" : ""}>
        <summary>
          <time datetime="${esc(String(at || ""))}">${esc(dateTime(at))}</time>
          <span class="copy-event-main"><span class="copy-event-line"><strong>${esc(I18n.t("copy-activity-skipped-group", { count: group.rows.length }))}</strong><span>${esc(skipLabel(group.key))}</span></span>
          <span class="copy-event-detail">${esc(I18n.t("copy-activity-group-detail", { tokens, since: dateTime(last.outcome?.decided_at || last.created_at) }))}</span></span>
        </summary>
        <ul class="copy-events">${group.rows.map(rowHtml).join("")}</ul>
      </details>
    </li>`;
  }

  function groups() {
    const result = [];
    rows.forEach((row) => {
      const key = row.outcome?.outcome === "skipped" ? skipKey(row.outcome.reason) : null;
      const last = result[result.length - 1];
      if (key && last?.key === key) last.rows.push(row);
      else result.push({ key, rows: [row] });
    });
    return result;
  }

  function html(ws) {
    if (ws.id !== taskId) reset(ws.id);
    ensureIdentities(
      rows.map((row) => row.outcome?.mint),
      rerender
    );
    const search = `<form class="copy-activity-search" data-activity-mint-form role="search"><input dir="ltr" type="search" name="mint" value="${esc(mint)}" placeholder="${esc(I18n.attr("copy-activity-mint-filter", "placeholder"))}" aria-label="${esc(I18n.attr("copy-activity-mint-filter", "aria-label"))}" spellcheck="false" autocomplete="off" />${mint ? `<button class="btn btn-ghost btn-sm" type="button" data-activity-clear>${esc(I18n.t("copy-activity-clear"))}</button>` : ""}</form>`;
    const head = `<div class="copy-panel-head"><h3>${esc(I18n.t("copy-activity-title"))}</h3><div class="copy-panel-tools">${segmented("activity-filter", FILTERS, filter, esc, I18n.t("copy-activity-filter-label"))}${search}</div></div>`;
    if (!loaded) {
      return (
        head +
        (error
          ? panelMessage(I18n.t("copy-activity-load-failed", { error }), esc, "is-error")
          : panelMessage(I18n.t("copy-activity-loading"), esc))
      );
    }
    if (!rows.length) {
      return (
        head +
        panelMessage(
          filter !== "all" || mint
            ? I18n.t("copy-activity-no-match")
            : I18n.t("copy-activity-empty"),
          esc
        )
      );
    }
    shared = sharedSymbols(rows.map((row) => row.outcome?.mint));
    const more = nextBefore
      ? `<div class="copy-more"><button class="btn btn-secondary btn-sm" type="button" data-activity-older>${esc(I18n.t("copy-activity-load-older"))}</button></div>`
      : `<p class="copy-note copy-end">${esc(I18n.t("copy-activity-start"))}</p>`;
    return `${head}<ul class="copy-events">${groups().map(groupHtml).join("")}</ul>${more}`;
  }

  return { setup, refresh, html, reset, setMint };
}
