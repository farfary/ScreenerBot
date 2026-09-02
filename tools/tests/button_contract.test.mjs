/**
 * The dashboard button contract.
 *
 * Settings > About shipped two buttons wearing the browser's own grey bevelled
 * chrome for months. Nothing was broken enough to notice in code review: the
 * updates tab had renamed `.settings-update-btn` to `.updates-btn`, About kept
 * writing the old name, and the single rule that still matched it set a
 * background and nothing else. Cargo does not parse CSS, ESLint does not read
 * class attributes, and stylelint does not know which classes exist, so every
 * gate the repo had was happy.
 *
 * These tests close that gap. They read the markup the dashboard emits (page
 * templates AND the JS template literals that produce most of it) and the
 * stylesheets that are concatenated into one document, and they check that
 * every button is fully drawn in every state it can be in: at rest, hovered,
 * selected, and disabled.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { CHECKS, loadButtonModel } from "../lib/button_contract.mjs";
import { STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";

const model = await loadButtonModel();

function report(violations) {
  return violations.map((violation) => `${violation.file}:${violation.line}: ${violation.message}`);
}

/* A check that stops matching anything passes silently for the wrong reason,
   so pin down what the model is expected to have found. */
test("the audit still sees the dashboard it is auditing", () => {
  assert.ok(model.buttons.length > 400, `only ${model.buttons.length} buttons found`);
  assert.ok(model.skinRules.length > 3000, `only ${model.skinRules.length} skin rules found`);
  assert.ok(model.stateRules.length > 500, `only ${model.stateRules.length} state rules found`);
  assert.ok(model.disabledRules.length > 30, `only ${model.disabledRules.length} disabled rules`);
  assert.ok(
    model.buttons.filter((button) => button.disabledInMarkup).length > 30,
    "no buttons were recognised as disable-able"
  );
  assert.ok(
    model.buttons.some((button) => button.conditional.length),
    "no conditional state classes were read out of the markup"
  );
});

for (const [name, check] of Object.entries(CHECKS)) {
  test(name, () => {
    assert.deepEqual(report(check(model)), []);
  });
}

/**
 * The one button property that is NOT checked per button, because it is fixed
 * once at the source. Form controls do not inherit type: without this rule
 * every button, input and textarea renders in the OS control font next to our
 * Inter copy, which is why two dozen components had each written
 * `font-family: inherit` for themselves before anyone named the cause.
 */
test("the dashboard never lets a control wear the OS font", async () => {
  const foundation = await readFile(resolve(STYLES_ROOT, "foundation.css"), "utf8");
  const rule = rulesIn(foundation).find(
    (candidate) =>
      candidate.selector === "button" && /font-family\s*:\s*inherit/.test(candidate.body)
  );
  assert.ok(rule, "foundation.css must give button/input/select/textarea font-family: inherit");
});
