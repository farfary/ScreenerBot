/**
 * Tests for the API error envelope helpers in `core/request_manager.js`.
 *
 * The dashboard localization runtime runs in an isolated vm context, as in
 * `i18n_runtime.test.mjs`; the request manager module is imported with a stub
 * `window` that carries that runtime.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const read = (rel) =>
  fs.readFileSync(new URL(`../../src/webserver/${rel}`, import.meta.url), "utf8");

const EN = "errors-strategies-already-exists = Strategy with ID '{ $id }' already exists\n";
const FA = "errors-strategies-already-exists = Strategy { $id } exists (fa)\n";

function loadI18n() {
  const context = { console, Intl, Date, JSON, fetch: async () => ({ ok: true }) };
  context.window = context;
  context.__SCREENERBOT_L10N__ = {
    locale: "fa",
    intlLocale: "fa-u-nu-latn",
    dir: "rtl",
    source: "en",
    catalogs: [
      { locale: "en", ftl: EN },
      { locale: "fa", ftl: FA },
    ],
  };
  vm.createContext(context);
  vm.runInContext(read("assets/fluent-bundle.js"), context);
  vm.runInContext(read("templates/scripts/core/i18n.js"), context);
  return context.I18n;
}

const I18n = loadI18n();
globalThis.window = { I18n, location: { origin: "http://localhost" } };
const { apiErrorMessage, apiErrorDetails } = await import(
  new URL("../../src/webserver/templates/scripts/core/request_manager.js", import.meta.url)
);

const envelope = {
  error: {
    code: "CONFLICT",
    message: "Strategy with ID 'a' already exists",
    text: { id: "errors-strategies-already-exists", args: { id: { type: "text", value: "a" } } },
    details: "raw cause",
  },
};

test("renders the catalog text in the active locale", () => {
  const message = apiErrorMessage(envelope, "fallback");
  assert.match(message, /Strategy .*a.* exists \(fa\)/);
  assert.notEqual(message, envelope.error.message);
});

test("uses the English message when the envelope has no text", () => {
  const legacy = { error: { code: "X", message: "Legacy message" } };
  assert.equal(apiErrorMessage(legacy, "fallback"), "Legacy message");
  assert.equal(apiErrorMessage({ message: "Top level" }, "fallback"), "Top level");
});

test("falls back when the body is missing or carries no message", () => {
  assert.equal(apiErrorMessage(null, "fallback"), "fallback");
  assert.equal(apiErrorMessage(undefined, "fallback"), "fallback");
  assert.equal(apiErrorMessage({}, "fallback"), "fallback");
  assert.equal(apiErrorMessage({ error: "plain string" }, "fallback"), "fallback");
});

test("exposes technical details or null", () => {
  assert.equal(apiErrorDetails(envelope), "raw cause");
  assert.equal(apiErrorDetails({ error: { details: null } }), null);
  assert.equal(apiErrorDetails(null), null);
  assert.equal(globalThis.window.RequestManagerErrors.apiErrorMessage, apiErrorMessage);
});
