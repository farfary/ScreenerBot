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

const EN =
  "errors-strategies-already-exists = Strategy with ID '{ $id }' already exists\n" +
  "errors-with-details = { $message }: { $details }\n";
const FA =
  "errors-strategies-already-exists = Strategy { $id } exists (fa)\n" +
  "errors-with-details = { $message } | { $details } (fa)\n";

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
const { apiErrorMessage, apiErrorTitle, apiErrorDetails } = await import(
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

const withoutDetails = { error: { ...envelope.error, details: null } };

test("renders the catalog text in the active locale", () => {
  const message = apiErrorMessage(withoutDetails, "fallback");
  assert.match(message, /Strategy .*a.* exists \(fa\)/);
  assert.notEqual(message, envelope.error.message);
});

test("frames the message with its technical cause when details are present", () => {
  const message = apiErrorMessage(envelope, "fallback");
  assert.match(message, /Strategy .*a.* exists \(fa\).* \| .*raw cause.* \(fa\)$/);
});

test("empty or non-string details leave the message unframed", () => {
  for (const details of ["", null, undefined, 5]) {
    const body = { error: { ...envelope.error, details } };
    assert.match(apiErrorMessage(body, "fallback"), /^Strategy .*a.* exists \(fa\)$/);
  }
});

test("a legacy envelope with details keeps its English message", () => {
  const legacy = { error: { message: "Legacy message", details: "cause" } };
  assert.equal(apiErrorMessage(legacy, "fallback"), "Legacy message");
});

test("the title omits details", () => {
  assert.match(apiErrorTitle(envelope, "fallback"), /^Strategy .*a.* exists \(fa\)$/);
  const legacy = { error: { message: "Legacy message", details: "cause" } };
  assert.equal(apiErrorTitle(legacy, "fallback"), "Legacy message");
  assert.equal(apiErrorTitle({ message: "Top level" }, "fallback"), "Top level");
  assert.equal(apiErrorTitle(null, "fallback"), "fallback");
  assert.equal(apiErrorTitle({}, "fallback"), "fallback");
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
  assert.equal(globalThis.window.RequestManagerErrors.apiErrorTitle, apiErrorTitle);
});
