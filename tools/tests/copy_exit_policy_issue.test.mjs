// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a copy task whose stored exit overrides no longer fit the Trader's exit
 * rules carries a warning with the broken rule on its row in the wallet list, and a
 * task whose overrides fit carries none. The selected task's sheet repeats it as a
 * warning notice under the tab row whose Edit rules action opens the editor at its
 * exits step.
 *
 * The warning is the backend `exit_policy_issue` UiText; the page renders it as is.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex, FIXTURES_ROOT } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const strip = (text) => text.replace(/[⁨⁩]/g, "").replace(/\s+/g, " ").trim();

test("a task whose exit overrides no longer fit is flagged on its row", async (t) => {
  const raw = JSON.parse(await readFile(join(FIXTURES_ROOT, "copy/copy_overview.json"), "utf8"));
  const overview = raw.data ?? raw;
  const [flagged, ...others] = overview.tasks;
  flagged.exit_policy_issue = {
    id: "copy-exit-issue-warning",
    args: {
      reason: {
        type: "nested",
        value: {
          id: "copy-exit-issue-trailing-distance-above-activation",
          args: {
            distance: { type: "number", value: 12 },
            activation: { type: "number", value: 10 },
          },
        },
      },
    },
  };
  for (const task of others) delete task.exit_policy_issue;
  const workspace = JSON.parse(
    await readFile(join(FIXTURES_ROOT, "copy/copy_workspace.json"), "utf8")
  );
  const sheet = workspace.data ?? workspace;
  sheet.id = flagged.id;
  sheet.exit_policy_issue = flagged.exit_policy_issue;

  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("copy"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      if (request.url.pathname === "/api/copy-trading/overview") {
        return { body: JSON.stringify(raw) };
      }
      if (request.url.pathname === `/api/copy-trading/tasks/${flagged.id}/workspace`) {
        return { body: JSON.stringify(workspace) };
      }
      return api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector("#copy-main:not([hidden])");
  await page.waitForSelector(`#copy-list-rows [data-task-id="${flagged.id}"]`);

  const warning = page.locator(`#copy-list-rows [data-task-id="${flagged.id}"] .copy-row-warning`);
  const text = strip(await warning.textContent());
  assert.match(text, /Exit rules need attention/);
  assert.match(text, /Trailing distance 12% must be below the activation, now 10%/);
  assert.equal(await warning.locator("i.icon-triangle-alert").count(), 1);

  for (const task of others) {
    assert.equal(
      await page.locator(`#copy-list-rows [data-task-id="${task.id}"] .copy-row-warning`).count(),
      0,
      `task ${task.id} fits the Trader's rules`
    );
  }

  const notice = page.locator('#copy-ws-notices .notice[data-variant="warning"]');
  await notice.waitFor();
  assert.match(strip(await notice.textContent()), /Trailing distance 12% must be below/);
  const edit = notice.locator('[data-action="edit-rules"]');
  assert.equal(strip(await edit.textContent()), "Edit rules");
  await edit.click();
  const current = page.locator('#copy-editor:not([hidden]) .copy-step[aria-current="step"]');
  await current.waitFor();
  assert.match(strip(await current.textContent()), /Exits$/);
});
