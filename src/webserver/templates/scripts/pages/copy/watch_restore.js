// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The Restore wallet watch dialog: a task paused because its wallet outran the watch
// budget comes back by catching up through Helius from saved progress, or by skipping
// the unchecked activity and resuming the watch from now at a higher signature limit.
// Either way only the watch is restored; the copy task stays paused until resumed.

const DIALOG_ID = "copy-watch-restore";

// Watch checks per poll the dialog accepts, in whole 100-signature pages.
const WATCH_SIGNATURES = Object.freeze({ min: 500, max: 5000, step: 100 });
const SIGNATURES_PER_PAGE = 100;

/** A failed wallet watch step whose explanation is a catalog message. */
function watchError(key) {
  const error = new Error(I18n.t(key));
  error.detail = error.message;
  return error;
}

/** The wallet watch target of `address`; a missing target fails with its own message. */
export async function watchTarget(requestManager, address) {
  const { targets = [] } = await requestManager.fetch("/api/wallets/watch");
  const target = targets.find((item) => item.address === address);
  if (!target) throw watchError("copy-watch-target-missing");
  return target;
}

/** A write to one wallet watch target (`/api/wallets/watch/{id}/{action}`), never replayed. */
export function postWatch(requestManager, targetId, action, body) {
  return requestManager.fetch(`/api/wallets/watch/${targetId}/${action}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    priority: "high",
    skipDedup: true,
  });
}

export function createWatchRestore(page) {
  const { $, Utils, requestManager, on, notify, confirm, dialogs } = page;
  const esc = Utils.escapeHtml;
  let task = null;
  let status = null;

  const body = () => $("#copy-watch-restore-body");

  function setup() {
    const root = body();
    on(root, "input", sync);
    on(root, "change", sync);
    on(root, "click", (event) => {
      const button = event.target.closest("[data-restore-action]");
      if (!button || !task) return;
      if (button.dataset.restoreAction === "approve-helius") void approveHelius(button);
      else if (button.dataset.restoreAction === "resume-budget") void resumeBudget(button);
    });
  }

  /** Opens the dialog for a paused task; `watchStatus` is the watch's last read status. */
  function open(nextTask, watchStatus) {
    task = nextTask;
    status = watchStatus || null;
    page.paint(body(), html());
    sync();
    dialogs.show(DIALOG_ID, {
      onClose: () => {
        task = null;
        status = null;
      },
    });
  }

  function html() {
    const helius = status?.catch_up_options?.find((option) => option.provider === "helius");
    const catchUp = helius?.available
      ? `<button class="btn btn-primary" type="button" data-restore-action="approve-helius">${esc(I18n.t("copy-watch-approve"))}</button><small>${esc(I18n.t("copy-watch-approve-help"))}</small>`
      : `<small>${esc(I18n.t(helius ? "copy-watch-approve-unavailable" : "copy-watch-no-provider"))}</small>`;
    const currentLimit = (Number(task.pause_reason?.page_budget) || 5) * SIGNATURES_PER_PAGE;
    const suggested = Math.min(WATCH_SIGNATURES.max, currentLimit + WATCH_SIGNATURES.min);
    return `<p class="copy-note">${esc(I18n.t("copy-watch-recovery-budget-intro"))}</p>
      <section class="copy-watch-option">${catchUp}</section>
      <section class="copy-watch-option">
        <label class="copy-field" for="copy-watch-page-budget">
          <span>${esc(I18n.t("copy-watch-budget-label"))}</span>
          <span class="copy-number-row"><input id="copy-watch-page-budget" data-watch-page-budget type="number" min="${WATCH_SIGNATURES.min}" max="${WATCH_SIGNATURES.max}" step="${WATCH_SIGNATURES.step}" value="${suggested}" aria-describedby="copy-watch-budget-hint" /></span>
          <small id="copy-watch-budget-hint">${esc(I18n.t("copy-watch-budget-hint", WATCH_SIGNATURES))}</small>
        </label>
        <label class="checkbox-label"><input type="checkbox" data-watch-resume-ack /><span>${esc(I18n.t("copy-watch-ack"))}</span></label>
        <button class="btn btn-outline" type="button" data-restore-action="resume-budget" disabled>${esc(I18n.t("copy-action-resume-from-now"))}</button>
      </section>`;
  }

  /** The signature limit as watch pages, or null while it is not a whole page in range. */
  function pageBudget() {
    const signatures = Number($("[data-watch-page-budget]")?.value);
    if (!Number.isInteger(signatures / SIGNATURES_PER_PAGE)) return null;
    if (signatures < WATCH_SIGNATURES.min || signatures > WATCH_SIGNATURES.max) return null;
    return signatures / SIGNATURES_PER_PAGE;
  }

  /** Resuming from now waits for a valid limit and the acknowledgement. */
  function sync() {
    const button = $("[data-restore-action='resume-budget']");
    if (!button) return;
    const input = $("[data-watch-page-budget]");
    button.disabled =
      !input?.value || pageBudget() === null || !$("[data-watch-resume-ack]")?.checked;
  }

  async function run(button, work, success, failure) {
    const address = task.target_address;
    button.disabled = true;
    try {
      await work(address);
      page.workspace.forgetWatch(address);
      dialogs.hide(DIALOG_ID);
      notify("success", success);
      await page.reload();
    } catch (error) {
      notify("error", failure, error.detail);
    } finally {
      if (button.isConnected) {
        button.disabled = false;
        sync();
      }
    }
  }

  async function resumeBudget(button) {
    const budget = pageBudget();
    if (budget === null) {
      notify("error", I18n.t("copy-watch-toast-range", WATCH_SIGNATURES));
      return;
    }
    if (!$("[data-watch-resume-ack]")?.checked) {
      notify("error", I18n.t("copy-watch-toast-ack"));
      return;
    }
    await run(
      button,
      async (address) => {
        const target = await watchTarget(requestManager, address);
        if (!target.enabled) {
          await postWatch(requestManager, target.id, "resume", {
            page_budget: budget,
            acknowledge_missed_activity: true,
          });
        } else if (target.page_budget !== budget) {
          await postWatch(requestManager, target.id, "budget", { page_budget: budget });
        }
      },
      I18n.t("copy-watch-resumed"),
      I18n.t("copy-watch-resume-failed")
    );
  }

  async function approveHelius(button) {
    const options = status?.catch_up_options;
    if (!options?.some((option) => option.provider === "helius" && option.available)) return;
    const confirmation = await confirm({
      title: I18n.t("copy-watch-approve-title"),
      message: I18n.t("copy-watch-approve-message"),
      confirmLabel: I18n.t("copy-watch-approve-confirm"),
      cancelLabel: I18n.t("copy-keep-paused"),
      variant: "warning",
    });
    if (!confirmation.confirmed || !task) return;
    await run(
      button,
      async (address) => {
        const target = await watchTarget(requestManager, address);
        await postWatch(requestManager, target.id, "high-activity-approval", {
          approved: true,
          acknowledge_provider_usage: true,
        });
        const restored = await requestManager.fetch("/api/wallets/watch");
        if (!restored.targets?.find((item) => item.id === target.id)?.enabled)
          throw watchError("copy-watch-not-restored");
      },
      I18n.t("copy-watch-approved"),
      I18n.t("copy-watch-restore-failed")
    );
  }

  return { setup, open };
}
