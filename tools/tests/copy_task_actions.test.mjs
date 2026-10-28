// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Copy Trading actions speak one language.
 *
 * A task's action row uses one button style: every action is an outline button, only the
 * action that resumes copying may be the primary button, and Delete differs by its danger
 * colour alone.
 *
 * A watch paused over its signature limit is restored from the notice under the tab row:
 * its Restore wallet watch action opens one dialog with both ways back (catch up from saved
 * progress, or skip ahead and resume from now), and the action row offers neither.
 *
 * The toolbar shows at most one bulk action, chosen by the processing switch and the task
 * states: Pause copying while a task copies, Resume copying while processing is off, and
 * Resume all tasks while processing is on, no task copies and a paused task can be resumed.
 * Resume all tasks asks first only while a live task would resume. Add wallet is the one
 * primary button, and a page without a task has no toolbar action but Settings.
 *
 * The wallet list marks the selected task as the current item without a border, and an
 * unnamed task shows its short address in the name slot.
 *
 * The arm gate and the settings take the width before they scroll: at a laptop height the
 * gate names its task in the header and sets readiness beside exposure with nothing to
 * scroll, and the settings lay three fields a row.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const fixture = (name) => JSON.parse(readFileSync(`${FIXTURES_ROOT}/copy/${name}`, "utf8"));
const strip = (text) => text.replace(/[⁨⁩]/g, "").trim();
const isResume = (request) => request.url().endsWith("/api/copy-trading/tasks/resume");

/** Wait until the status chip reads `expected` (a pattern): it says "Loading" until the overview answers. */
const chipReads = (page, expected) =>
  page.waitForFunction(
    (pattern) =>
      new RegExp(pattern).test(
        document.querySelector('#copy-summary [data-summary-id="status"]')?.textContent ?? ""
      ),
    expected
  );

/** A dashboard on the Copy page whose API answers `answers` by path before the fixtures. */
async function openCopy(t, answers = {}, viewport = { width: 1440, height: 900 }) {
  const context = await browser.newContext({ viewport });
  const api = createApiHandler([await loadIndex("copy"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const answer = answers[request.url.pathname];
      if (typeof answer === "function") return answer(request);
      return answer ? { body: answer } : api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  return { page, origin: host.origin };
}

test("the task action row uses one button style", async (t) => {
  const { page, origin } = await openCopy(t);
  await page.goto(`${origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector("#copy-ws-actions [data-ws-action='delete']");
  const buttons = await page.$$eval("#copy-ws-actions .btn", (nodes) =>
    nodes.map((node) => {
      const style = getComputedStyle(node);
      return {
        action: node.dataset.wsAction,
        primary: node.classList.contains("btn-primary"),
        outline: node.classList.contains("btn-outline"),
        border: style.borderTopStyle !== "none" && parseFloat(style.borderTopWidth) > 0,
        background: style.backgroundColor,
      };
    })
  );
  assert.ok(buttons.length >= 4, JSON.stringify(buttons));
  for (const button of buttons) {
    if (button.primary) {
      assert.equal(button.action, "resume", button.action);
      continue;
    }
    assert.ok(button.outline && button.border, `${button.action} is an outline button`);
    assert.equal(button.background, "rgba(0, 0, 0, 0)", `${button.action} has no fill`);
  }
});

test("a watch over its limit is restored from the notice under the tab row", async (t) => {
  const task = fixture("copy_workspace.json");
  task.enabled = false;
  task.recovery = "restore_watch";
  task.pause_reason = { kind: "watch_budget_exceeded", page_budget: 5, signatures_checked: 500 };
  const watch = fixture("wallets_watch_status.json");
  watch.target.enabled = false;
  watch.catch_up_options = [{ provider: "helius", available: true }];
  const { page, origin } = await openCopy(t, {
    [`/api/copy-trading/tasks/${task.id}/workspace`]: JSON.stringify(task),
    [`/api/wallets/watch/${watch.target.id}/status`]: JSON.stringify(watch),
  });
  await page.goto(`${origin}/copy`);
  await page.waitForSelector(PAGE_READY);

  const notice = page.locator("#copy-ws-notices .notice");
  const restore = notice.locator("[data-action='restore-watch']");
  await restore.waitFor();
  assert.equal(strip(await restore.textContent()), "Restore wallet watch");
  assert.equal(await page.locator("#copy-ws-actions [data-ws-action='resume']").count(), 0);
  assert.equal(await page.locator("#copy-ws-actions [data-ws-action='resume-budget']").count(), 0);
  const layout = await page.evaluate(() => {
    const box = (selector) => document.querySelector(selector).getBoundingClientRect();
    return { tabs: box("#copy-ws-tabs"), notices: box("#copy-ws-notices") };
  });
  assert.ok(layout.tabs.bottom <= layout.notices.top, "the notice sits below the tab row");

  await restore.click();
  const dialog = page.locator("#copy-watch-restore:not([hidden])");
  await dialog.locator("[data-restore-action='approve-helius']").waitFor();
  const choices = await dialog.locator("[data-restore-action]").evaluateAll((nodes) =>
    nodes.map((node) => {
      const style = getComputedStyle(node);
      return {
        action: node.dataset.restoreAction,
        variant: ["btn-primary", "btn-outline"].find((name) => node.classList.contains(name)),
        legible: style.color !== style.backgroundColor,
      };
    })
  );
  assert.deepEqual(
    choices.map(({ action, variant }) => `${action}:${variant}`),
    ["approve-helius:btn-primary", "resume-budget:btn-outline"]
  );
  for (const choice of choices) assert.ok(choice.legible, `${choice.action} is legible`);

  const resume = dialog.locator("[data-restore-action='resume-budget']");
  assert.ok(await resume.isDisabled(), "resuming from now waits for the acknowledgement");
  await dialog.locator("[data-watch-resume-ack]").check();
  assert.ok(await resume.isEnabled(), "the acknowledgement enables resuming from now");
});

test("the toolbar offers one bulk action that follows the task states", async (t) => {
  const overview = fixture("copy_overview.json");
  const status = (overview.data ?? overview).status;
  const posts = [];
  const { page, origin } = await openCopy(t, {
    "/api/copy-trading/overview": () => ({ body: JSON.stringify(overview) }),
    "/api/copy-trading/tasks/resume": () => ({
      body: JSON.stringify({ resumed: [3], kept_paused: [] }),
    }),
  });
  page.on("request", (request) => {
    if (isResume(request)) posts.push(request.postDataJSON());
  });
  const global = page.locator("#copy-global-action");
  const resumeTasks = page.locator("#copy-resume-tasks");
  const statusChip = page.locator('#copy-summary [data-summary-id="status"]');

  const states = [
    [{ enabled: true, active_tasks: 2 }, "Pause copying", "Processing"],
    [{ enabled: false, active_tasks: 0 }, "Resume copying", "Paused"],
  ];
  for (const [patch, label, chip] of states) {
    Object.assign(status, patch);
    await page.goto(`${origin}/copy`);
    await page.waitForSelector(PAGE_READY);
    await statusChip.waitFor();
    await chipReads(page, chip);
    assert.match(strip(await statusChip.textContent()), new RegExp(chip));
    assert.equal(strip(await global.textContent()), label);
    assert.equal(await global.isVisible(), true, JSON.stringify(patch));
    assert.equal(await resumeTasks.isVisible(), false, JSON.stringify(patch));
  }

  Object.assign(status, {
    enabled: true,
    active_tasks: 0,
    resumable_tasks: 1,
    resumable_live_tasks: 0,
  });
  await page.goto(`${origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await resumeTasks.waitFor();
  await chipReads(page, "All tasks paused");
  assert.match(strip(await statusChip.textContent()), /All tasks paused/);
  assert.equal(strip(await resumeTasks.textContent()), "Resume all tasks");
  assert.equal(await global.isVisible(), false, "one bulk action at a time");
  await Promise.all([page.waitForRequest(isResume), resumeTasks.click()]);
  assert.deepEqual(posts, [{ include_live: false }], "paper tasks resume without asking");
  assert.equal(await page.locator(".confirmation-dialog").count(), 0);

  Object.assign(status, { resumable_tasks: 2, resumable_live_tasks: 1 });
  await page.goto(`${origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await resumeTasks.waitFor();
  await resumeTasks.click();
  const dialog = page.locator(".confirmation-dialog");
  await dialog.waitFor();
  assert.match(
    strip(await dialog.locator(".confirmation-dialog__message").textContent()),
    /live task will submit real swaps/
  );
  await Promise.all([
    page.waitForRequest(isResume),
    dialog.locator('[data-action="confirm"]').click(),
  ]);
  assert.deepEqual(posts.at(-1), { include_live: true }, "a confirmed resume includes live");

  Object.assign(status, { resumable_tasks: 0, resumable_live_tasks: 0 });
  await page.goto(`${origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await statusChip.waitFor();
  assert.equal(await resumeTasks.isVisible(), false, "nothing to resume, no bulk action");
  assert.equal(await global.isVisible(), false);

  (overview.data ?? overview).tasks = [];
  Object.assign(status, { total_tasks: 0, active_tasks: 0, paper_tasks: 0, live_tasks: 0 });
  for (const enabled of [true, false]) {
    status.enabled = enabled;
    await page.goto(`${origin}/copy`);
    await page.waitForSelector(PAGE_READY);
    await page.locator("#copy-settings-open").waitFor();
    assert.equal(await statusChip.count(), 0, `no chips without a task, enabled: ${enabled}`);
    assert.equal(await global.isVisible(), false, "no bulk action without a task");
    assert.equal(await page.locator("#copy-add").isVisible(), false);
  }
});

test("Add wallet is the one primary, and the list marks its current task plainly", async (t) => {
  const overview = fixture("copy_overview.json");
  const unnamed = (overview.data ?? overview).tasks.find((task) => !task.label);
  assert.ok(unnamed, "the fixture keeps an unnamed task");
  const { page, origin } = await openCopy(t);
  await page.goto(`${origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector('.copy-row[aria-current="true"]');

  const primaries = await page.$$eval(".copy-toolbar .btn-primary", (nodes) =>
    nodes.filter((node) => !node.hidden).map((node) => node.id)
  );
  assert.deepEqual(primaries, ["copy-add"]);

  assert.equal(await page.locator(".copy-row[aria-pressed]").count(), 0);
  assert.equal(await page.locator('.copy-row[aria-current="true"]').count(), 1);
  const selected = await page.$eval('.copy-row[aria-current="true"]', (node) => {
    const style = getComputedStyle(node);
    return { border: parseFloat(style.borderTopWidth), background: style.backgroundColor };
  });
  assert.equal(selected.border, 0, "the selected row has no border");
  assert.notEqual(selected.background, "rgba(0, 0, 0, 0)", "the selected row is tinted");

  const name = strip(
    await page.locator(`.copy-row[data-task-id="${unnamed.id}"] .copy-row-name`).textContent()
  );
  const address = unnamed.target_address;
  assert.ok(name.startsWith(address.slice(0, 4)), name);
  assert.ok(name.endsWith(address.slice(-4)), name);
  assert.ok(name.length < address.length, "the name slot shows the short address");
});

test("at a laptop height the arm gate sets readiness beside exposure, unscrolled", async (t) => {
  const task = fixture("copy_workspace.json");
  const { page, origin } = await openCopy(t, {}, { width: 1280, height: 666 });
  await page.goto(`${origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  const arm = page.locator("#copy-ws-panel [data-ws-action='arm']");
  await arm.waitFor();
  await arm.click();
  await page.locator("#copy-arm:not([hidden]) #copy-arm-body .arm-gate-ack").first().waitFor();

  const gate = await page.evaluate(() => {
    const body = document.querySelector("#copy-arm-body");
    const [readiness, exposure] = [...body.querySelectorAll(".copy-split > section")].map(
      (section) => section.getBoundingClientRect()
    );
    return {
      overflows: body.scrollHeight > body.clientHeight + 1,
      width: document.querySelector(".copy-arm-panel").offsetWidth,
      sideBySide: Math.abs(readiness.top - exposure.top) < 1 && readiness.right <= exposure.left,
      exposure: body.querySelectorAll(".copy-split .copy-def").length,
      subtitle: document.querySelector("#copy-arm-sub").textContent,
    };
  });
  assert.equal(gate.overflows, false, "every check, figure and acknowledgement is visible");
  assert.ok(gate.width > 860, `the gate takes the width (${gate.width}px)`);
  assert.ok(gate.sideBySide, "readiness and exposure share a row");
  assert.ok(gate.exposure >= 6, "the exposure figures sit beside the readiness");
  assert.ok(strip(gate.subtitle).includes((task.data ?? task).label), gate.subtitle);
  await page.keyboard.press("Escape");
  await page.locator("#copy-arm").waitFor({ state: "hidden" });
});

test("the settings lay three fields a row on the wide panel", async (t) => {
  const { page, origin } = await openCopy(t);
  await page.goto(`${origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await page.locator("#copy-settings-open").click();
  await page.locator("#copy-settings:not([hidden]) #copy-settings-readiness").waitFor();

  const sheet = await page.evaluate(() => {
    const body = document.querySelector("#copy-settings .modal-body");
    const tops = [...body.querySelectorAll(".copy-fields > .copy-field")].map((field) =>
      Math.round(field.getBoundingClientRect().top)
    );
    const note = document
      .querySelector("#copy-settings-paper-delay-shared")
      .getBoundingClientRect();
    const fields = body.querySelector(".copy-fields").getBoundingClientRect();
    return {
      overflows: body.scrollHeight > body.clientHeight + 1,
      width: document.querySelector(".copy-settings-panel").offsetWidth,
      rows: [...new Set(tops)].length,
      noteSpans: Math.abs(note.width - fields.width) < 1,
    };
  });
  assert.equal(sheet.overflows, false, "the settings fit the window unscrolled");
  assert.ok(sheet.width > 860, `the settings take the width (${sheet.width}px)`);
  assert.equal(sheet.rows, 2, "six fields in two rows of three");
  assert.ok(sheet.noteSpans, "the shared paper-delay note spans its own row");
});
