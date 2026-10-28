// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Strategies page sends and receives a strategy whole.
 *
 * The list's enable toggle once sent a body without the strategy's timeframe, so the
 * route reset a strategy saved on another timeframe to the default one. Import put
 * the parsed file into memory but never loaded its rules into the editor.
 *
 * - The roster switch changes only `enabled`, through the route that touches nothing
 *   else of the stored strategy; the editor's On switch takes the same route.
 * - A roster row opens its strategy in the editor in place of the roster, and the back
 *   control returns to the roster after unsaved changes are confirmed.
 * - Importing an export opens it as a new unsaved strategy, switched off, with its
 *   rules in the editor, and exporting that strategy writes the same file back.
 * - An import file whose condition does not apply to the strategy type, or whose
 *   priority is outside the evaluation order, is refused with the reason and opens
 *   nothing.
 * - Deleting a strategy from its roster row asks first and sends one DELETE.
 * - Before a stored strategy is saved the server is asked whether the save changes its
 *   rules. New rules on a live strategy are confirmed first and Cancel saves nothing; new
 *   rules on a paper strategy save at once and say that its paper evidence restarts.
 * - Adding a condition from the catalog opens it in place without a toast, and the
 *   catalog search is the shared search field: its clear control sits inside the input.
 * - A blank new strategy shows adding a condition as its next step, with no problem
 *   bar, and its Invert stays off until there is a condition to negate.
 * - With no strategy at all, the roster offers New Strategy in place.
 * - The editor's toolbar is one row of the name and the actions, also at the smallest
 *   window; the strategy settings, the description and the exit binding sit in the
 *   scrolling column above the rules, and the description stays closed until asked for,
 *   also for a strategy that has one.
 * - Testing against a picked token shows a spinner with the token's name in the footer
 *   line while the test runs, which can take as long as the server waits for the token's
 *   candles. The result then marks each rule by position with its verdict, an unknown
 *   condition with its reason, and the footer line reads the whole strategy's verdict
 *   under the same name. An edit takes the marks and the line away.
 * - An exit strategy names the entry strategies whose positions it manages: a checkbox per
 *   entry strategy plus a bound one deleted since, "All entry strategies" with none
 *   checked, and the checked ids saved; an entry strategy shows neither the button nor the
 *   note and choices, and the panel stays closed until its button opens it.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { branch, leaf } from "../lib/strategy_rules.mjs";

/** The condition schemas the trader fixtures serve, so catalog counts follow the catalog. */
const SCHEMAS = Object.values(
  JSON.parse(
    readFileSync(
      new URL("./fixtures/dashboard/trader/strategies_condition_schemas.json", import.meta.url),
      "utf8"
    )
  ).schemas
);

const ID = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c01";
const MISSING_COUNTS = {
  id: "strategies-error-missing-data",
  args: {
    data: { type: "nested", value: { id: "strategies-error-data-transaction-counts" } },
  },
};
/** The test outcome of RULES, in the shape of the saved tree. */
const OUTCOME = {
  verdict: "unknown",
  operator: "AND",
  reason: MISSING_COUNTS,
  children: [
    { verdict: "pass", condition_type: "Rsi" },
    { verdict: "off", condition_type: "VolumeSpike" },
    {
      verdict: "unknown",
      operator: "NOT",
      reason: MISSING_COUNTS,
      children: [
        {
          verdict: "unknown",
          operator: "OR",
          reason: MISSING_COUNTS,
          children: [
            { verdict: "unknown", condition_type: "CandleSize", reason: MISSING_COUNTS },
            { verdict: "fail", condition_type: "PriceToMA" },
          ],
        },
      ],
    },
  ],
};
const RULES = branch(
  "AND",
  leaf("Rsi", { period: 21, timeframe: "1h" }),
  { ...leaf("VolumeSpike"), disabled: true },
  branch("NOT", branch("OR", leaf("CandleSize"), leaf("PriceToMA")))
);
const STORED = {
  id: ID,
  name: "Momentum breakout",
  description: "Buys a 1h breakout.",
  strategy_type: "ENTRY",
  enabled: true,
  priority: 7,
  timeframe: "15m",
  rules: RULES,
  parameters: {},
  created_at: "2026-09-10T08:30:00+00:00",
  updated_at: "2026-10-01T12:15:00+00:00",
  author: null,
  version: 1,
  entry_strategy_ids: [],
};
/** A stored exit strategy bound to one listed entry strategy and one deleted since. */
const EXIT_ID = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c05";
const DIP_RECOVERY_ID = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c02";
const EXIT_STORED = {
  id: EXIT_ID,
  name: "Moving average cross exit",
  description: null,
  strategy_type: "EXIT",
  enabled: true,
  priority: 10,
  timeframe: "15m",
  rules: branch("AND", leaf("PositionHoldingTime")),
  parameters: {},
  created_at: "2026-09-10T08:30:00+00:00",
  updated_at: "2026-10-01T12:15:00+00:00",
  author: null,
  version: 3,
  entry_strategy_ids: [DIP_RECOVERY_ID, "retired-entry"],
};
/** The live entry strategy of the overview fixture, as its detail route answers. */
const DIP_STORED = {
  ...STORED,
  id: DIP_RECOVERY_ID,
  name: "Dip recovery",
  description: null,
  priority: 20,
  version: 2,
  mode: "live",
};
const EXPORTED = {
  format: "screenerbot.strategy",
  version: 1,
  strategy: {
    name: "Imported momentum",
    description: "Imported from a file.",
    strategy_type: "ENTRY",
    timeframe: "1h",
    priority: 12,
    rules: RULES,
    parameters: {},
  },
};

/** Holds the answer of a strategy test until a test case releases it. */
const testGate = { open: Promise.resolve() };
/**
 * What the server answers about a save: whether it changes the stored strategy's rules,
 * and the book the validation reports the strategy stored on (omitted while null).
 */
const answers = { validateRulesChanged: false, saveRulesChanged: false, validateMode: null };

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const fixtures = createApiHandler(
  [await loadIndex("trader"), await loadIndex("shell")],
  "populated"
);
const host = await serveDashboard(context, {
  locale: "en",
  onApi: async (request) => {
    if (request.url.pathname === "/api/strategies/validate") {
      const stored = request.url.searchParams.has("id");
      return {
        body: JSON.stringify({
          valid: true,
          errors: [],
          ...(stored ? { rules_changed: answers.validateRulesChanged } : {}),
          ...(stored && answers.validateMode ? { mode: answers.validateMode } : {}),
        }),
      };
    }
    if (request.url.pathname === `/api/strategies/${ID}`) {
      return {
        body: JSON.stringify(
          request.method === "GET"
            ? STORED
            : { id: ID, version: 2, rules_changed: answers.saveRulesChanged }
        ),
      };
    }
    if (request.url.pathname === `/api/strategies/${DIP_RECOVERY_ID}`) {
      return {
        body: JSON.stringify(
          request.method === "GET"
            ? DIP_STORED
            : { id: DIP_RECOVERY_ID, version: 3, rules_changed: answers.saveRulesChanged }
        ),
      };
    }
    if (request.url.pathname === `/api/strategies/${EXIT_ID}`) {
      return {
        body: JSON.stringify(
          request.method === "GET" ? EXIT_STORED : { id: EXIT_ID, version: 4 }
        ),
      };
    }
    if (request.url.pathname === `/api/strategies/${ID}/test`) {
      await testGate.open;
      return {
        body: JSON.stringify({
          strategy_id: ID,
          strategy_name: STORED.name,
          result: null,
          unknown_reason: MISSING_COUNTS,
          execution_time_ms: 3,
          rules: OUTCOME,
        }),
      };
    }
    if (request.url.pathname === `/api/strategies/${ID}/enabled`) {
      return { body: JSON.stringify({ id: ID, enabled: false, version: 2 }) };
    }
    return fixtures.onApi(request);
  },
});
after(async () => {
  await context.close();
  await host.close();
});
const page = await context.newPage();
page.setDefaultTimeout(15000);
await page.goto(`${host.origin}/trader`);
await page.waitForSelector(PAGE_READY);
await page.click('#subTabsContainer [data-tab-id="strategies"]');
await page.waitForSelector("#strategies-roster tbody tr[data-row-id]");

/** Text without the Unicode isolates Fluent places around arguments. */
const plain = (text) => text.replace(/[\u2068\u2069]/g, "");

/** Leave the editor for the roster, discarding unsaved changes when the page asks. */
async function backToRoster() {
  if (await page.locator("#strategy-editor").isHidden()) return;
  const dirty = (await page.locator("#editor-header.dirty").count()) > 0;
  await page.click("#strategy-back");
  if (dirty) {
    await page.getByText("Discard unsaved changes?").waitFor();
    await page.getByRole("button", { name: "Discard" }).click();
  }
  await page.locator("#strategies-roster").waitFor({ state: "visible" });
}

/** Open the stored strategy `id` named `name` from its row in the `kind` view. */
async function openStrategy(id, name, kind = "entry") {
  await backToRoster();
  await page.click(`#strategies-roster label:has(> input[data-seg-value="${kind}"])`);
  await page.click(`#strategies-roster tr[data-row-id="${id}"] [data-column-id="name"]`);
  await page.waitForFunction((expected) => document.querySelector("#strategy-name")?.value === expected, name);
  await page.locator("#strategies-roster").waitFor({ state: "hidden" });
}

async function importFile(file) {
  await backToRoster();
  const [chooser] = await Promise.all([
    page.waitForEvent("filechooser"),
    page.click('#strategies-roster [data-btn-id="import-strategy"]'),
  ]);
  await chooser.setFiles({
    name: "strategy.json",
    mimeType: "application/json",
    buffer: Buffer.from(typeof file === "string" ? file : JSON.stringify(file)),
  });
}

test("the roster switch changes only enabled, through the enabled route", async () => {
  const writes = [];
  // Only the strategy resource counts: the shell also POSTs reads such as
  // `/api/ui-state/load`, which a slow page start can issue inside this window.
  const record = (request) => {
    if (
      request.method() !== "GET" &&
      new URL(request.url()).pathname.startsWith("/api/strategies")
    ) {
      writes.push(request);
    }
  };
  page.on("request", record);
  const [request] = await Promise.all([
    page.waitForRequest((candidate) => candidate.method() === "PATCH"),
    page.click(`#strategies-roster label:has(> input[data-enable-strategy="${ID}"])`),
  ]);
  // The roster reads the overview again once the switch is stored.
  await page.waitForResponse(
    (response) => new URL(response.url()).pathname === "/api/strategies/overview"
  );
  page.off("request", record);
  assert.equal(new URL(request.url()).pathname, `/api/strategies/${ID}/enabled`);
  assert.deepEqual(request.postDataJSON(), { enabled: false });
  assert.deepEqual(writes, [request], "no other write reaches the stored strategy");
});

test("an imported export opens as a new unsaved strategy and exports back unchanged", async () => {
  await importFile(EXPORTED);
  await page.waitForSelector("#editor-header.dirty");

  assert.equal(await page.inputValue("#strategy-name"), "Imported momentum");
  assert.equal(await page.inputValue("#strategy-priority"), "12");
  assert.equal(await page.inputValue("#strategy-timeframe"), "1h");
  assert.equal(await page.inputValue("#strategy-description"), "Imported from a file.");
  assert.equal(
    await page.isChecked("#strategy-enabled-toggle"),
    false,
    "an import arrives switched off"
  );
  assert.equal(await page.locator("#conditions-list .rule-row").count(), 4);
  assert.equal(await page.locator("#conditions-list .rule-row.is-off").count(), 1);
  assert.equal(await page.locator("#conditions-list .rule-group.is-inverted").count(), 1);

  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.click("#export-strategy"),
  ]);
  assert.equal(download.suggestedFilename(), "imported-momentum.json");
  assert.deepEqual(JSON.parse(readFileSync(await download.path(), "utf8")), EXPORTED);
});

test("an import the strategy type or the evaluation order refuses names the reason and opens nothing", async () => {
  const refused = JSON.parse(JSON.stringify(EXPORTED));
  refused.strategy.name = "Refused";
  refused.strategy.rules = branch("AND", leaf("PositionHoldingTime"));

  await importFile(refused);
  await page.getByText("cannot be used in this type of strategy").first().waitFor();
  assert.equal(await page.locator("#strategy-editor").isHidden(), true);

  for (const priority of [0, 101, 2.5]) {
    const outOfOrder = JSON.parse(JSON.stringify(EXPORTED));
    outOfOrder.strategy.priority = priority;
    await importFile(outOfOrder);
    await page
      .getByText(/The priority in the file must be a whole number from \W*1\W* to \W*100/)
      .first()
      .waitFor();
    assert.equal(await page.locator("#strategy-editor").isHidden(), true, `priority ${priority}`);
  }

  await importFile("not json");
  await page.getByText("The file is not valid JSON").first().waitFor();
});

test("a condition is added from the catalog, checked as it is edited and kept out of Save while invalid", async () => {
  const saves = [];
  page.on("request", (request) => {
    if (request.method() === "POST" && new URL(request.url()).pathname === "/api/strategies") {
      saves.push(request);
    }
  });

  await page.click('#strategies-roster [data-btn-id="create-strategy"]');
  await page.click('.type-card[data-type="ENTRY"]');
  await page.waitForSelector("#editor-header.dirty");
  assert.equal(
    await page.locator("#rule-problems").isHidden(),
    true,
    "a blank strategy is not a problem yet"
  );
  assert.match(plain(await page.textContent("#conditions-list .state-view")), /No conditions yet/);
  assert.equal(await page.locator("#conditions-list .rule-invert").isDisabled(), true);

  await page.click("#add-condition");
  assert.equal(await page.evaluate(() => document.activeElement?.id), "condition-search");
  assert.equal(await page.locator("#catalog-list .catalog-entry").count(), SCHEMAS.length);
  assert.equal(
    await page.locator("#catalog-list .catalog-entry:disabled").count(),
    SCHEMAS.filter((schema) => !schema.applies_to.includes("ENTRY")).length,
    "an entry strategy cannot use the conditions that read a position"
  );
  assert.equal(
    await page.locator('.catalog-entry[data-condition-type="PositionProfit"]').isDisabled(),
    true
  );
  await page.fill("#condition-search", "relative strength");
  const search = await page.evaluate(() => {
    const input = document.getElementById("condition-search").getBoundingClientRect();
    const clear = document.getElementById("clear-search");
    const box = clear.getBoundingClientRect();
    return {
      shape: clear.closest(".search-wrapper") !== null && clear.classList.contains("search-clear"),
      visible: getComputedStyle(clear).display !== "none" && box.width > 0,
      inside:
        box.left >= input.left &&
        box.right <= input.right &&
        box.top >= input.top &&
        box.bottom <= input.bottom,
    };
  });
  assert.deepEqual(search, { shape: true, visible: true, inside: true });
  // Toasts from earlier cases may still be on screen; only a new one counts.
  const toastsBefore = await page.locator(".toast").allInnerTexts();
  await page.keyboard.press("Enter");
  await page.locator("#condition-catalog-modal").waitFor({ state: "hidden" });
  assert.deepEqual(
    (await page.locator(".toast").allInnerTexts()).filter((text) => !toastsBefore.includes(text)),
    [],
    "adding a condition raises no toast"
  );

  const row = page.locator("#conditions-list .rule-row.is-open");
  assert.equal(await row.count(), 1);
  assert.equal(
    await page.evaluate(() => document.activeElement?.closest(".rule-body") !== null),
    true
  );
  assert.equal(await page.locator("#rule-problems").isHidden(), true);
  assert.equal(await page.locator("#conditions-list .rule-invert").isDisabled(), false);

  await page.fill('[data-key="period"]', "");
  assert.equal(await page.getAttribute('[data-key="period"]', "aria-invalid"), "true");
  assert.equal(await row.locator(".param-field-error:not(:empty)").textContent(), "Enter a number");
  assert.match(
    plain(await page.textContent("#rule-problems")),
    /1 problem to fix: .*RSI Period: Enter a number/
  );

  await page.click("#save-strategy");
  assert.equal(saves.length, 0, "Save does not call the API while a field is invalid");
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.key), "period");

  await page.fill('[data-key="period"]', "500");
  assert.match(plain(await page.textContent("#rule-problems")), /Must be between 2 and 100/);
  await page.fill('[data-key="period"]', "14");
  assert.equal(await page.locator("#rule-problems").isHidden(), true);

  await page.locator('#conditions-list [data-action="duplicate"]').first().click();
  assert.equal(await page.locator("#conditions-list .rule-row").count(), 2);
  await page
    .locator("#conditions-list .rule-row")
    .first()
    .locator('[data-action="move-down"]')
    .click();
  assert.deepEqual(
    await page.evaluate(() => {
      const active = document.activeElement;
      const rows = [...document.querySelectorAll("#conditions-list .rule-row")];
      return [active?.dataset.action, rows.indexOf(active?.closest(".rule-row"))];
    }),
    ["move-up", 1],
    "focus stays on the moved row, on the button that is still enabled"
  );

  await page.click("#add-group");
  const group = page.locator("#conditions-list .rule-group.is-nested");
  assert.equal(await group.count(), 1);
  assert.match(plain(await group.textContent()), /^\s*Match[\s\S]*of these/);
  await group.locator('[data-action="add-condition"]').click();
  await page.fill("#condition-search", "buy pressure");
  await page.keyboard.press("Enter");
  assert.equal(await group.locator(".rule-row").count(), 1);
  assert.equal(
    await page.locator("#conditions-list > .rule-group > .rule-children > .rule-row").count(),
    2
  );
});

test("the back control confirms unsaved changes and returns to the roster", async () => {
  assert.equal(await page.locator("#editor-header.dirty").count(), 1, "the new strategy is unsaved");
  await page.click("#strategy-back");
  await page.getByText("Discard unsaved changes?").waitFor();
  await page.locator('.confirmation-dialog [data-action="cancel"]').click();
  assert.equal(await page.locator("#strategy-editor").isVisible(), true, "Cancel stays in the editor");
  assert.equal(await page.locator("#editor-header.dirty").count(), 1);

  await backToRoster();
  assert.equal(await page.locator("#strategy-editor").isHidden(), true);
});

test("deleting a strategy from its roster row asks first and sends one DELETE", async () => {
  await backToRoster();
  const deletes = [];
  const record = (request) => {
    if (request.method() === "DELETE") deletes.push(request);
  };
  page.on("request", record);
  try {
    const row = page.locator(`#strategies-roster tr[data-row-id="${ID}"]`);
    assert.equal(
      plain(await row.locator('[data-action-id="delete"]').getAttribute("aria-label")),
      "Delete Momentum breakout"
    );
    await row.locator('[data-action-id="delete"]').click();
    await page.locator('.confirmation-dialog [data-action="cancel"]').click();
    assert.equal(deletes.length, 0, "Cancel deletes nothing");

    const [request] = await Promise.all([
      page.waitForRequest((candidate) => candidate.method() === "DELETE"),
      (async () => {
        await row.locator('[data-action-id="delete"]').click();
        await page.locator(".confirmation-dialog").getByRole("button", { name: "Delete" }).click();
      })(),
    ]);
    assert.equal(new URL(request.url()).pathname, `/api/strategies/${ID}`);
    assert.equal(deletes.length, 1);
    assert.equal(await page.locator("#strategies-roster").isVisible(), true);
    assert.equal(await page.locator("#strategy-editor").isHidden(), true);
  } finally {
    page.off("request", record);
  }
});

test("the toolbar is one row of actions; settings and panels scroll with the rules", async () => {
  await openStrategy(ID, STORED.name);

  const placement = await page.evaluate(() => {
    const inside = (selector, owner) =>
      document.querySelector(owner).contains(document.querySelector(selector));
    return {
      toolbar: [
        "#strategy-back",
        "#strategy-name",
        "#test-strategy",
        "#validate-strategy",
        "#export-strategy",
        "#save-strategy",
        "#strategy-enabled-toggle",
      ].every((selector) => inside(selector, "#editor-header")),
      noSettingsInToolbar: [
        "#strategy-timeframe",
        "#strategy-priority",
        "#toggle-description",
        "#toggle-entry-binding",
        "#strategy-description-panel",
        "#strategy-entry-binding-panel",
      ].every((selector) => !inside(selector, "#editor-header")),
      scrolling: [
        "#editor-settings",
        "#strategy-description-panel",
        "#strategy-entry-binding-panel",
      ].every((selector) => inside(selector, "#editor-scroll-area")),
    };
  });
  assert.deepEqual(placement, { toolbar: true, noSettingsInToolbar: true, scrolling: true });

  assert.equal(
    await page.locator("#strategy-description-panel").isHidden(),
    true,
    "a strategy with a description opens with it closed"
  );
  assert.equal(await page.getAttribute("#toggle-description", "aria-expanded"), "false");
  await page.click("#toggle-description");
  assert.equal(await page.locator("#strategy-description-panel").isVisible(), true);
  await page.click("#toggle-description");

  // The smallest window: the toolbar stays one row and leaves the rules the editor's height.
  await page.setViewportSize({ width: 1200, height: 700 });
  try {
    const layout = await page.evaluate(() => {
      const toolbar = document.querySelector("#editor-header");
      const middles = [...toolbar.querySelectorAll(".editor-identity, .editor-controls > *")]
        .filter((element) => element.getClientRects().length > 0)
        .map((element) => element.getBoundingClientRect())
        .map((box) => box.top + box.height / 2);
      const footer = document.querySelector("#editor-footer").getBoundingClientRect();
      return {
        height: toolbar.getBoundingClientRect().height,
        fixed: toolbar.getBoundingClientRect().height + footer.height,
        oneRow: Math.max(...middles) - Math.min(...middles) <= 2,
        overflow: toolbar.scrollWidth > toolbar.clientWidth,
        rules: document.querySelectorAll("#editor-scroll-area .rule-row").length > 0,
      };
    });
    assert.ok(layout.height <= 52, `toolbar height ${layout.height}`);
    assert.ok(layout.fixed <= 160, `toolbar and footer take ${layout.fixed}px`);
    assert.deepEqual(
      { oneRow: layout.oneRow, overflow: layout.overflow, rules: layout.rules },
      { oneRow: true, overflow: false, rules: true }
    );
    // The editor now spans the sub-tab, so which labels fit depends on the window; the
    // order in which they give way does not.
    const labels = await page.evaluate(() =>
      Object.fromEntries(
        ["#strategy-back", "#test-strategy", "#validate-strategy", "#export-strategy"].map(
          (selector) => [
            selector,
            document.querySelector(`${selector} .editor-action-label`).getBoundingClientRect().width > 1,
          ]
        )
      )
    );
    assert.equal(labels["#validate-strategy"], labels["#export-strategy"], "Validate and Export collapse together");
    assert.equal(labels["#strategy-back"], labels["#validate-strategy"], "the back control collapses with them");
    assert.ok(
      labels["#test-strategy"] || !labels["#validate-strategy"],
      "Test keeps its label while any other action shows one"
    );
    assert.equal(
      await page.getAttribute("#validate-strategy", "title"),
      "Validate",
      "an action shown as its glyph keeps its label as tooltip"
    );
    assert.equal(
      await page.getByRole("button", { name: "Validate" }).count(),
      1,
      "and as its accessible name"
    );
  } finally {
    await page.setViewportSize({ width: 1440, height: 900 });
  }
});

test("the editor's On switch stores a saved strategy through the enabled route alone", async () => {
  await openStrategy(ID, STORED.name);
  const writes = [];
  const record = (request) => {
    if (request.method() !== "GET" && new URL(request.url()).pathname.startsWith("/api/strategies")) {
      writes.push(request);
    }
  };
  page.on("request", record);
  try {
    const [request] = await Promise.all([
      page.waitForRequest((candidate) => candidate.method() === "PATCH"),
      page.click("label.editor-active"),
    ]);
    assert.equal(new URL(request.url()).pathname, `/api/strategies/${ID}/enabled`);
    assert.deepEqual(request.postDataJSON(), { enabled: false });
    assert.deepEqual(writes, [request], "the switch saves nothing else");
    assert.equal(await page.locator("#editor-header.dirty").count(), 0, "nothing is left unsaved");
  } finally {
    page.off("request", record);
  }
});

test("a test against a picked token marks each rule and reads the strategy in the footer", async () => {
  await openStrategy(ID, STORED.name);
  await page.evaluate(() => {
    window.openSearchDialog = ({ onPick }) =>
      onPick({ mint: "TrendMint1111111111111111111111111111111111", symbol: "TREND", name: "Trend" });
  });
  let release;
  testGate.open = new Promise((resolve) => {
    release = resolve;
  });

  await page.click("#test-strategy");
  const running = page.locator("#strategy-test-result .strategy-test-running.loading-spinner");
  await running.waitFor();
  assert.equal(plain(await running.textContent()), "Testing against TREND…");
  assert.equal(
    await page.evaluate(() =>
      document
        .querySelector("#editor-footer")
        .contains(document.querySelector("#strategy-test-line"))
    ),
    true,
    "the outcome is a footer line"
  );

  release();
  const title = page.locator("#strategy-test-result .strategy-test-title");
  await title.waitFor();
  assert.equal(plain(await title.textContent()), "Test against TREND");
  assert.equal(await running.count(), 0);
  assert.match(
    plain(await page.textContent("#strategy-test-result")),
    /Test against TREND\s+Unknown\s+Transaction counts is not available/
  );

  const marked = await page.evaluate(() =>
    [
      ...document.querySelectorAll(
        "#conditions-list .rule-row, #conditions-list .rule-group.is-nested"
      ),
    ].map((node) => {
      const head = node.querySelector(":scope > .rule-head, :scope > .rule-group-head");
      const reason = node.querySelector(":scope > .strategy-test-reason");
      return [
        node.dataset.kind,
        head.querySelector(":scope > .strategy-test-verdict")?.dataset.verdict ?? null,
        reason ? reason.textContent.replace(/[\u2068\u2069]/g, "").trim() : null,
        reason?.getAttribute("role") ?? null,
      ];
    })
  );
  assert.deepEqual(marked, [
    ["condition", "pass", null, null],
    ["condition", "off", null, null],
    ["group", "unknown", null, null],
    ["condition", "unknown", "Transaction counts is not available", null],
    ["condition", "fail", null, null],
  ]);
  assert.equal(
    await page.locator("#strategy-test-panel, .strategy-test-tree").count(),
    0,
    "no second tree of the rules"
  );

  await page.fill("#strategy-priority", "8");
  await page.waitForSelector("#editor-header.dirty");
  assert.equal(await page.locator("#conditions-list [data-test-mark]").count(), 0);
  assert.equal(await page.locator("#strategy-test-line").isHidden(), true);

  await page.click("#save-strategy");
  await page.waitForSelector("#editor-header:not(.dirty)");
});

/** Whether a save request reached the strategy resource with `method`. */
const isSave = (request, method, id) =>
  request.method() === method && new URL(request.url()).pathname === `/api/strategies/${id}`;

test("new rules on a live strategy are confirmed first, and Cancel saves nothing", async () => {
  await openStrategy(DIP_RECOVERY_ID, DIP_STORED.name);
  answers.validateRulesChanged = true;
  answers.saveRulesChanged = true;
  const puts = [];
  const record = (request) => {
    if (isSave(request, "PUT", DIP_RECOVERY_ID)) puts.push(request);
  };
  page.on("request", record);
  try {
    await page.fill("#strategy-priority", "21");
    await page.waitForSelector("#editor-header.dirty");
    const [check] = await Promise.all([
      page.waitForRequest(
        (candidate) => new URL(candidate.url()).pathname === "/api/strategies/validate"
      ),
      page.click("#save-strategy"),
    ]);
    assert.equal(new URL(check.url()).searchParams.get("id"), DIP_RECOVERY_ID);
    assert.equal(check.postDataJSON().priority, 21, "the check reads the edited strategy");

    const dialog = page.locator(".confirmation-dialog");
    await dialog.getByText(/Change the rules of live/).waitFor();
    assert.match(
      plain(await dialog.textContent()),
      /The new rules trade real SOL from the next check, and the paper evidence of this strategy restarts from this save\./
    );
    await dialog.locator('[data-action="cancel"]').click();
    await dialog.waitFor({ state: "hidden" });
    assert.deepEqual(puts, [], "Cancel saves nothing");
    assert.equal(await page.locator("#editor-header.dirty").count(), 1, "the edit stays unsaved");
    assert.equal(await page.locator("#strategy-editor").isVisible(), true);

    const [put] = await Promise.all([
      page.waitForRequest((candidate) => isSave(candidate, "PUT", DIP_RECOVERY_ID)),
      (async () => {
        await page.click("#save-strategy");
        await dialog.getByRole("button", { name: "Save and trade live" }).click();
      })(),
    ]);
    assert.equal(put.postDataJSON().priority, 21);
    await page.waitForSelector("#editor-header:not(.dirty)");
    assert.equal(
      await page.getByText(/paper evidence restarts from now/).count(),
      0,
      "the confirmed dialog already said the evidence restarts"
    );
  } finally {
    page.off("request", record);
    answers.validateRulesChanged = false;
    answers.saveRulesChanged = false;
  }
});

test("new rules on a paper strategy save at once and say its paper evidence restarts", async () => {
  await openStrategy(ID, STORED.name);
  answers.validateRulesChanged = true;
  answers.saveRulesChanged = true;
  try {
    await page.fill("#strategy-priority", "9");
    await page.waitForSelector("#editor-header.dirty");
    const [put] = await Promise.all([
      page.waitForRequest((candidate) => isSave(candidate, "PUT", ID)),
      page.click("#save-strategy"),
    ]);
    assert.equal(put.postDataJSON().priority, 9);
    await page
      .getByText(/The rules of [“\u2068]+Momentum breakout[\u2069”]+ changed; its paper evidence restarts from now/)
      .first()
      .waitFor();
    assert.equal(
      await page.locator(".confirmation-dialog:visible").count(),
      0,
      "a paper strategy asks nothing before new rules"
    );
  } finally {
    answers.validateRulesChanged = false;
    answers.saveRulesChanged = false;
  }
});

test("new rules are confirmed when the server stores the strategy live though the roster shows paper", async () => {
  await openStrategy(ID, STORED.name);
  answers.validateRulesChanged = true;
  answers.validateMode = "live";
  const puts = [];
  const record = (request) => {
    if (isSave(request, "PUT", ID)) puts.push(request);
  };
  page.on("request", record);
  try {
    await page.fill("#strategy-priority", "13");
    await page.waitForSelector("#editor-header.dirty");
    await page.click("#save-strategy");
    const dialog = page.locator(".confirmation-dialog");
    await dialog.getByText(/Change the rules of live/).waitFor();
    await dialog.locator('[data-action="cancel"]').click();
    await dialog.waitFor({ state: "hidden" });
    assert.deepEqual(puts, [], "the unconfirmed save sends nothing");
    assert.equal(await page.locator("#editor-header.dirty").count(), 1, "the edit stays unsaved");

    await Promise.all([
      page.waitForRequest((candidate) => isSave(candidate, "PUT", ID)),
      (async () => {
        await page.click("#save-strategy");
        await dialog.getByRole("button", { name: "Save and trade live" }).click();
      })(),
    ]);
    await page.waitForSelector("#editor-header:not(.dirty)");
  } finally {
    page.off("request", record);
    answers.validateRulesChanged = false;
    answers.validateMode = null;
  }
});

test("a save the server calls unchanged in rules says only that it saved", async () => {
  await openStrategy(ID, STORED.name);
  await page.fill("#strategy-priority", "11");
  await page.waitForSelector("#editor-header.dirty");
  await Promise.all([
    page.waitForRequest((candidate) => isSave(candidate, "PUT", ID)),
    page.click("#save-strategy"),
  ]);
  await page.getByText(/["\u2068]+Momentum breakout[\u2069"]+ saved successfully/).first().waitFor();
});

test("with no strategy the roster offers New Strategy", async () => {
  const empty = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const emptyFixtures = createApiHandler(
    [await loadIndex("trader"), await loadIndex("shell")],
    "empty"
  );
  const emptyHost = await serveDashboard(empty, { locale: "en", onApi: emptyFixtures.onApi });
  try {
    const tab = await empty.newPage();
    tab.setDefaultTimeout(15000);
    await tab.goto(`${emptyHost.origin}/trader`);
    await tab.waitForSelector(PAGE_READY);
    await tab.click('#subTabsContainer [data-tab-id="strategies"]');
    const action = tab.locator('#strategies-roster .state-view-empty [data-action="table-empty-action"]');
    await action.waitFor();
    assert.equal((await action.textContent()).trim(), "New Strategy");
    assert.match(plain(await tab.textContent("#strategies-roster")), /No strategies yet/);
    await action.click();
    await tab.locator("#create-strategy-modal").waitFor({ state: "visible" });
    assert.equal(
      await tab.getAttribute('#create-strategy-modal .type-card[data-type="ENTRY"]', "aria-pressed"),
      "true",
      "the Entry view preselects an entry strategy"
    );
  } finally {
    await empty.close();
    await emptyHost.close();
  }
});

test("an exit strategy picks the entry strategies it manages and saves their ids", async () => {
  await openStrategy(EXIT_ID, EXIT_STORED.name, "exit");
  const toggle = page.locator("#toggle-entry-binding");
  const summary = page.locator("#entry-binding-summary");
  const choices = page.locator("#strategy-entry-binding input[type=checkbox]");
  const panel = page.locator("#strategy-entry-binding-panel");
  assert.equal(await toggle.isVisible(), true);
  assert.equal(plain(await page.locator("#entry-binding-label").textContent()), "Manages");

  const pickerTab = await page.evaluate(() => {
    let options = null;
    const open = window.openSearchDialog;
    window.openSearchDialog = (given) => {
      options = given;
    };
    document.querySelector("#test-strategy").click();
    window.openSearchDialog = open;
    return options?.tab;
  });
  assert.equal(pickerTab, "positions", "an exit strategy is tested against a held token");
  assert.equal(await panel.isHidden(), true, "the binding panel opens only from its button");
  assert.equal(plain(await summary.textContent()), "Dip recovery and retired-entry (deleted)");

  await toggle.click();
  assert.equal(await toggle.getAttribute("aria-expanded"), "true");
  assert.equal(await panel.isVisible(), true);
  assert.equal(await choices.count(), 4, "three entry strategies and the deleted one");
  const checked = await choices.evaluateAll((boxes) =>
    boxes.filter((box) => box.checked).map((box) => box.value)
  );
  assert.deepEqual(checked, [DIP_RECOVERY_ID, "retired-entry"]);

  await page.locator(`#strategy-entry-binding input[value="${DIP_RECOVERY_ID}"]`).uncheck();
  await page.locator('#strategy-entry-binding input[value="retired-entry"]').uncheck();
  assert.equal(plain(await summary.textContent()), "All entry strategies");
  await page.locator(`#strategy-entry-binding input[value="${ID}"]`).check();
  assert.equal(plain(await summary.textContent()), "Momentum breakout");
  await page.waitForSelector("#editor-header.dirty");

  const [request] = await Promise.all([
    page.waitForRequest(
      (candidate) =>
        candidate.method() === "PUT" && new URL(candidate.url()).pathname === `/api/strategies/${EXIT_ID}`
    ),
    page.click("#save-strategy"),
  ]);
  assert.deepEqual(request.postDataJSON().entry_strategy_ids, [ID]);

  await page.waitForSelector("#editor-header:not(.dirty)");
  await openStrategy(ID, STORED.name);
  assert.equal(await toggle.isHidden(), true, "an entry strategy manages no positions");
  assert.equal(await panel.isHidden(), true, "nor shows the binding note or its choices");

  await backToRoster();
  await page.click(`#strategies-roster label:has(> input[data-seg-value="exit"])`);
  await page.click('#strategies-roster [data-btn-id="create-strategy"]');
  assert.equal(
    await page.getAttribute('#create-strategy-modal .type-card[data-type="EXIT"]', "aria-pressed"),
    "true",
    "the Exit view preselects an exit strategy"
  );
  await page.click('#create-strategy-modal .type-card[data-type="EXIT"]');
  await page.waitForSelector("#strategy-type-badge.exit");
  assert.equal(await toggle.isVisible(), true, "a new exit strategy offers the binding");
  assert.equal(plain(await summary.textContent()), "All entry strategies");
});
