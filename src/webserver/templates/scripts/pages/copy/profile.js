// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Wallet profile: what this bot has seen of a wallet (watch status, trades
// observed through copy tasks, each task's results) and "Copy this wallet".
import { renderAddress } from "../../ui/token_identity.js";
import {
  dateTime,
  definitionRows,
  modeLabel,
  pct,
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
          I18n.t("copy-profile-watched"),
          I18n.t("format-no"),
          tasks.length
            ? I18n.t("copy-profile-watch-resume-hint")
            : I18n.t("copy-profile-watch-add-hint"),
        ],
      ];
    }
    return [
      [
        I18n.t("copy-profile-watched"),
        watch.enabled ? I18n.t("format-yes") : I18n.t("copy-state-paused"),
        watch.label || "",
      ],
      [
        I18n.t("copy-profile-stream"),
        watch.subscribed
          ? I18n.t("copy-profile-subscribed")
          : I18n.t("copy-profile-not-subscribed"),
        I18n.t("copy-profile-sources", { count: watch.sources }),
      ],
      [
        I18n.t("copy-profile-last-activity"),
        watch.last_activity_at ? timeAgo(watch.last_activity_at) : "—",
      ],
      watch.last_error ? [I18n.t("copy-profile-last-error"), I18n.text(watch.last_error)] : null,
    ];
  }

  function tasksTable(tasks) {
    if (!tasks.length) return "";
    const rows = tasks
      .map(
        (task) => `<tr>
          <td><button class="copy-token-link" type="button" data-profile-task="${task.task_id}">${esc(task.name)}</button></td>
          <td>${esc(task.enabled ? modeLabel(task.mode) : I18n.t("copy-mode-paused", { mode: modeLabel(task.mode) }))}</td>
          <td class="num">${task.rounds}</td>
          <td class="num">${esc(task.rounds ? pct(task.win_rate_pct, 0) : "—")}</td>
          <td class="num ${toneClass(task.realized_pnl_native)}">${esc(signedSol(task.realized_pnl_native))}</td>
          <td class="num">${esc(seconds(task.arrival_median_ms))}</td>
        </tr>`
      )
      .join("");
    return `<h4>${esc(I18n.t("copy-profile-tasks-title"))}</h4><div class="copy-table-wrap"><table class="copy-table"><thead><tr><th scope="col">${esc(I18n.t("copy-table-task"))}</th><th scope="col">${esc(I18n.t("copy-table-mode"))}</th><th scope="col" class="num">${esc(I18n.t("copy-table-rounds"))}</th><th scope="col" class="num">${esc(I18n.t("copy-metric-win-rate"))}</th><th scope="col" class="num">${esc(I18n.t("copy-table-realized"))}</th><th scope="col" class="num">${esc(I18n.t("copy-metric-median-arrival"))}</th></tr></thead><tbody>${rows}</tbody></table></div>`;
  }

  function render(profile) {
    const seen = profile.observations || {};
    const own = profile.own_wallet
      ? `<p class="copy-warning" role="alert"><i class="icon-triangle-alert" aria-hidden="true"></i>${esc(I18n.t("copy-profile-own-wallet"))}</p>`
      : "";
    const observed = seen.swaps
      ? definitionRows(
          [
            [
              I18n.t("copy-profile-swaps-seen"),
              String(seen.swaps),
              I18n.t("copy-profile-swaps-seen-note"),
            ],
            [
              I18n.t("copy-profile-buys-sells"),
              I18n.t("copy-profile-buys-sells-value", { buys: seen.buys, sells: seen.sells }),
            ],
            [I18n.t("copy-profile-tokens-traded"), String(seen.tokens)],
            [I18n.t("copy-profile-first-seen"), dateTime(seen.first_seen)],
            [I18n.t("copy-profile-last-seen"), dateTime(seen.last_seen)],
          ],
          esc
        )
      : "";
    return `<div class="copy-profile-address">${renderAddress(profile.address, { explorer: "account" })}</div>${own}
      <div class="copy-split">
        <section><h4>${esc(I18n.t("copy-profile-watch-title"))}</h4><dl class="copy-defs">${definitionRows(watchRows(profile.watch, profile.tasks || []), esc)}</dl></section>
        <section><h4>${esc(I18n.t("copy-profile-observed-title"))}</h4>${
          observed
            ? `<dl class="copy-defs">${observed}</dl>`
            : `<p class="copy-note">${esc(I18n.t("copy-profile-observed-none"))}</p>`
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
    paint(body, panelMessage(I18n.t("copy-profile-loading"), esc));
    dialogs.show("copy-profile");
    try {
      const profile = await api.profile(address);
      if (current?.address !== address) return;
      current.profile = profile;
      paint(body, render(profile));
      if (copy) {
        copy.disabled = profile.own_wallet;
        copy.textContent = profile.tasks?.length
          ? I18n.t("copy-profile-copy-other")
          : I18n.t("copy-profile-copy");
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
