// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Electron backend token header tests - the exact backend-origin check and the session listener that adds X-ScreenerBot-Token.

import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const {
  TOKEN_HEADER,
  isBackendUrl,
  installBackendTokenHeader,
  tokenHeaderFor,
} = require("../../electron/src/backend_token.js");

const BACKEND = { host: "127.0.0.1", port: 49152, token: "test-session-token" };

test("the backend origin predicate requires plain HTTP, exact host and explicit port", () => {
  assert.equal(isBackendUrl("http://127.0.0.1:49152/api/wallets?x=1", BACKEND), true);
  assert.equal(isBackendUrl("http://127.0.0.1:49152/", { ...BACKEND, port: "49152" }), true);
  assert.equal(isBackendUrl("http://127.0.0.1:49153/media/clip.mp4", BACKEND), false);
  assert.equal(isBackendUrl("http://127.0.0.1/api/health", BACKEND), false);
  assert.equal(isBackendUrl("http://192.168.1.10:49152/api/health", BACKEND), false);
  assert.equal(isBackendUrl("http://example.com:49152/", BACKEND), false);
  assert.equal(isBackendUrl("http://localhost:49152/api/health", BACKEND), false);
  assert.equal(isBackendUrl("https://127.0.0.1:49152/api/health", BACKEND), false);
  assert.equal(isBackendUrl("ws://127.0.0.1:49152/", BACKEND), false);
  assert.equal(isBackendUrl("file:///Applications/index.html", BACKEND), false);
  assert.equal(isBackendUrl("not a url", BACKEND), false);
  assert.equal(isBackendUrl(undefined, BACKEND), false);
  assert.equal(isBackendUrl("http://127.0.0.1:49152/", { ...BACKEND, port: null }), false);
  assert.equal(isBackendUrl("http://127.0.0.1:49152/", null), false);
});

test("the exact backend origin receives the token header", () => {
  assert.deepEqual(
    tokenHeaderFor("http://127.0.0.1:49152/api/wallets?x=1", BACKEND),
    { [TOKEN_HEADER]: BACKEND.token },
  );
});

test("no token, an empty token or no backend yields no header", () => {
  const url = "http://127.0.0.1:49152/api/health";
  assert.equal(tokenHeaderFor(url, { ...BACKEND, token: null }), null);
  assert.equal(tokenHeaderFor(url, { ...BACKEND, token: "" }), null);
  assert.equal(tokenHeaderFor(url, { ...BACKEND, token: 42 }), null);
  assert.equal(tokenHeaderFor(url, { ...BACKEND, port: null }), null);
  assert.equal(tokenHeaderFor(url, null), null);
});

function fakeSession() {
  const registered = [];
  return {
    registered,
    webRequest: {
      onBeforeSendHeaders(...args) {
        registered.push(args);
      },
    },
  };
}

function send(listener, url, requestHeaders, resourceType) {
  const responses = [];
  listener({ url, requestHeaders, resourceType }, (response) => responses.push(response));
  return responses;
}

test("passive subresources never receive the token, including at the backend origin", () => {
  const session = fakeSession();
  installBackendTokenHeader(session, () => BACKEND);
  const [listener] = session.registered[0];

  for (const resourceType of ["image", "media", "font", "stylesheet", "script", "object", "ping", "cspReport"]) {
    const [response] = send(listener, "http://127.0.0.1:49152/assets/logo.png", {}, resourceType);
    assert.equal(TOKEN_HEADER in response.requestHeaders, false, resourceType);
  }
});

test("fetch, XHR and unclassified request types receive the token only at the backend origin", () => {
  const session = fakeSession();
  installBackendTokenHeader(session, () => BACKEND);
  const [listener] = session.registered[0];

  for (const resourceType of ["xhr", "other", "mainFrame", undefined]) {
    const [local] = send(listener, "http://127.0.0.1:49152/api/health", {}, resourceType);
    assert.equal(local.requestHeaders[TOKEN_HEADER], BACKEND.token, String(resourceType));
    const [external] = send(listener, "https://example.com/api/health", {}, resourceType);
    assert.equal(TOKEN_HEADER in external.requestHeaders, false, String(resourceType));
  }
});

test("the listener is registered once and reads the backend per request", () => {
  const session = fakeSession();
  let backend = { ...BACKEND, token: null };
  installBackendTokenHeader(session, () => backend);

  assert.equal(session.registered.length, 1);
  const [listener] = session.registered[0];
  assert.equal(typeof listener, "function");

  const before = send(listener, "http://127.0.0.1:49152/api/health", { Accept: "*/*" });
  assert.deepEqual(before, [{ requestHeaders: { Accept: "*/*" } }]);

  backend = BACKEND;
  const after = send(listener, "http://127.0.0.1:49152/api/health", { Accept: "*/*" });
  assert.deepEqual(after, [{
    requestHeaders: { Accept: "*/*", [TOKEN_HEADER]: BACKEND.token },
  }]);
});

test("requests to another origin pass through unchanged and the callback always runs", () => {
  const session = fakeSession();
  installBackendTokenHeader(session, () => BACKEND);
  const [listener] = session.registered[0];

  const headers = { Accept: "text/html" };
  const responses = send(listener, "http://127.0.0.1:50000/media/a.png", headers);
  assert.equal(responses.length, 1);
  assert.equal(responses[0].requestHeaders, headers);
  assert.equal(TOKEN_HEADER in responses[0].requestHeaders, false);

  const unparsable = send(listener, "::", headers);
  assert.equal(unparsable.length, 1);
  assert.equal(unparsable[0].requestHeaders, headers);
});
