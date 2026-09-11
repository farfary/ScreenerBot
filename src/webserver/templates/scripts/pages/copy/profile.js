// Wallet profile: what this bot has seen of a wallet (watch status, trades
// observed through copy tasks, each task's results) and "Copy this wallet".
import { renderAddress } from "../../ui/token_identity.js";
import {
  MODE_LABELS,
  dateTime,
  definitionRows,
  pct,
  plural,
  seconds,
  signedSol,
  timeAgo,
  toneClass,
} from "./format.js";
import { panelMessage } from "./overview.js";

export function createProfile(page) {
  const { $, Utils, api, on, paint, dialogs } = page;
  const esc = Utils.escapeHtml;
  let current = null;

  function setup() {
    on($("#copy-profile-copy"), "click", copyWallet);
    on($("#copy-profile-body"), "click", (event) => {
      const task = event.target.closest("[data-profile-task]");
      if (!task) return;
      dialogs.hide("copy-profile");
      page.select(Number(task.dataset.profileTask));
    });
  }

  function watchRows(watch, tasks) {
    if (!watch) {
      return [
        [
          "Watched",
          "No",
          tasks.length ? "Resuming a task watches it again" : "Adding a task starts watching it",
        ],
      ];
    }
    return [
      ["Watched", watch.enabled ? "Yes" : "Paused", watch.label || ""],
      [
        "Stream",
        watch.subscribed ? "Subscribed" : "Not subscribed",
        plural(watch.sources, "source"),
      ],
      ["Last activity", watch.last_activity_at ? timeAgo(watch.last_activity_at) : "—"],
      watch.last_error ? ["Last error", watch.last_error] : null,
    ];
  }

  function tasksTable(tasks) {
    if (!tasks.length) return "";
    const rows = tasks
      .map(
        (task) => `<tr>
          <td><button class="copy-token-link" type="button" data-profile-task="${task.task_id}">${esc(task.name)}</button></td>
          <td>${esc(MODE_LABELS[task.mode] || task.mode)}${task.enabled ? "" : " · paused"}</td>
          <td class="num">${task.rounds}</td>
          <td class="num">${esc(task.rounds ? pct(task.win_rate_pct, 0) : "—")}</td>
          <td class="num ${toneClass(task.realized_pnl_sol)}">${esc(signedSol(task.realized_pnl_sol))}</td>
          <td class="num">${esc(seconds(task.arrival_median_ms))}</td>
        </tr>`
      )
      .join("");
    return `<h4>Your tasks on this wallet</h4><div class="copy-table-wrap"><table class="copy-table"><thead><tr><th scope="col">Task</th><th scope="col">Mode</th><th scope="col" class="num">Rounds</th><th scope="col" class="num">Win rate</th><th scope="col" class="num">Realized</th><th scope="col" class="num">Median arrival</th></tr></thead><tbody>${rows}</tbody></table></div>`;
  }

  function render(profile) {
    const seen = profile.observations || {};
    const own = profile.own_wallet
      ? '<p class="copy-warning" role="alert"><i class="icon-triangle-alert" aria-hidden="true"></i>This is one of your own wallets; copying it is refused.</p>'
      : "";
    const observed = seen.swaps
      ? definitionRows(
          [
            ["Swaps seen", String(seen.swaps)],
            ["Buys / sells", `${seen.buys} / ${seen.sells}`],
            ["Tokens traded", String(seen.tokens)],
            ["First seen", dateTime(seen.first_seen)],
            ["Last seen", dateTime(seen.last_seen)],
          ],
          esc
        )
      : "";
    return `<div class="copy-profile-address">${renderAddress(profile.address, { explorer: "account" })}</div>${own}
      <div class="copy-split">
        <section><h4>Watch</h4><dl class="copy-defs">${definitionRows(watchRows(profile.watch, profile.tasks || []), esc)}</dl></section>
        <section><h4>Trades observed</h4>${
          observed
            ? `<dl class="copy-defs">${observed}</dl>`
            : '<p class="copy-note">No trades from this wallet in this bot yet. A Paper task observes it without spending SOL.</p>'
        }</section>
      </div>${tasksTable(profile.tasks || [])}`;
  }

  async function open(address, label = null) {
    current = { address, label, profile: null };
    const error = $("#copy-profile-error");
    if (error) error.textContent = "";
    const copy = $("#copy-profile-copy");
    if (copy) copy.disabled = true;
    const body = $("#copy-profile-body");
    paint(body, panelMessage("Loading wallet profile…", esc));
    dialogs.show("copy-profile");
    try {
      const profile = await api.profile(address);
      if (current?.address !== address) return;
      current.profile = profile;
      paint(body, render(profile));
      if (copy) {
        copy.disabled = profile.own_wallet;
        copy.textContent = profile.tasks?.length ? "Copy again" : "Copy this wallet";
      }
    } catch (failure) {
      if (current?.address !== address) return;
      paint(
        body,
        `<div class="copy-profile-address">${renderAddress(address, { explorer: "account" })}</div>`
      );
      if (error) error.textContent = failure.detail;
    }
  }

  function copyWallet() {
    if (!current?.profile || current.profile.own_wallet) return;
    const { address, label, profile } = current;
    dialogs.hide("copy-profile");
    page.editor.openCreate({
      target_address: address,
      label: label || profile.watch?.label || null,
    });
  }

  return { setup, open };
}
