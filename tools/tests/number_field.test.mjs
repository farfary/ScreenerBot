/**
 * Tests for the numeric field's step arithmetic (`ui/number_field.js`).
 *
 * The stepper cannot delegate to `HTMLInputElement.stepUp()`: it throws on
 * `step="any"` and on an empty value, and float addition walks a trade size to
 * 0.30000000000000004. Since a stepped field writes a real config value — a
 * trade size, a slippage, a stop loss — the arithmetic is asserted here rather
 * than trusted to a browser API we do not use.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { stepNumber } from "../../src/webserver/templates/scripts/ui/number_field.js";

test("steps by the field's own step, in both directions", () => {
  assert.equal(stepNumber("5", { step: "0.5" }, 1), 5.5);
  assert.equal(stepNumber("5", { step: "0.5" }, -1), 4.5);
});

test("a field without a step moves by one", () => {
  assert.equal(stepNumber("41", {}, 1), 42);
  assert.equal(stepNumber("41", { step: "any" }, -1), 40);
});

test("keeps the step's precision instead of accumulating float dust", () => {
  assert.equal(stepNumber("0.1", { step: "0.2" }, 1), 0.3);
  assert.equal(stepNumber("0.005", { step: "0.001" }, 1), 0.006);
});

test("clamps to the field's own bounds", () => {
  assert.equal(stepNumber("100", { step: "1", max: "100" }, 1), 100);
  assert.equal(stepNumber("0.5", { step: "1", min: "0.5" }, -1), 0.5);
});

test("an empty field starts at its floor, not at zero", () => {
  assert.equal(stepNumber("", { step: "0.001", min: "0.001" }, 1), 0.001);
  assert.equal(stepNumber("", { step: "1" }, 1), 0);
  assert.equal(stepNumber("", { step: "1", max: "-5" }, 1), -5);
});

test("an unparseable value is treated as unset rather than as NaN", () => {
  assert.equal(stepNumber("abc", { step: "1", min: "10" }, 1), 10);
  assert.equal(Number.isFinite(stepNumber("abc", { step: "1" }, -1)), true);
});

test("a zero or negative step attribute cannot freeze the field", () => {
  assert.equal(stepNumber("7", { step: "0" }, 1), 8);
  assert.equal(stepNumber("7", { step: "-3" }, 1), 8);
});
