// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Session-token header for the backend - isBackendUrl() owns the exact-origin rule, tokenHeaderForRequest() keeps passive subresources from receiving the token, and installBackendTokenHeader() adds X-ScreenerBot-Token to eligible requests in the shell's own session, so no served page has to carry the token.

'use strict';

const TOKEN_HEADER = 'X-ScreenerBot-Token';

/** Whether `url` uses the backend's exact plain-http host and explicit port. */
function isBackendUrl(url, backend) {
  if (!backend) return false;
  if (backend.port === null || backend.port === undefined) return false;

  let parsed;
  try {
    parsed = new URL(url);
  } catch (_) {
    return false;
  }

  return parsed.protocol === 'http:'
    && parsed.hostname === backend.host
    && parsed.port === String(backend.port);
}

/**
 * The token header for `url`, or null. Only the exact backend origin receives
 * it: any other origin, including another loopback port, is outside the backend.
 */
function tokenHeaderFor(url, backend) {
  if (!backend || typeof backend.token !== 'string' || backend.token === '') return null;
  if (!isBackendUrl(url, backend)) return null;

  return { [TOKEN_HEADER]: backend.token };
}

const PASSIVE_RESOURCE_TYPES = new Set([
  'image',
  'media',
  'font',
  'stylesheet',
  'script',
  'object',
  'ping',
  'cspReport',
]);

/**
 * Passive loads can use URLs supplied by token metadata, including logos and
 * lightbox images. Dashboard API calls use fetch or XHR; denying known passive
 * types keeps those URLs outside the token path even if Electron reports fetch
 * with a resource type that changes between versions.
 */
function tokenHeaderForRequest(details, backend) {
  if (PASSIVE_RESOURCE_TYPES.has(details.resourceType)) return null;
  return tokenHeaderFor(details.url, backend);
}

/**
 * Register the one `onBeforeSendHeaders` listener of `session`. `getBackend()`
 * is read per request, so a backend restart on a new port and token needs no
 * reinstall. Electron keeps only the last listener attached to this event, and
 * its URL filter syntax is not specified, so the listener takes every request
 * and `tokenHeaderForRequest` applies the resource-type and origin checks.
 */
function installBackendTokenHeader(session, getBackend) {
  session.webRequest.onBeforeSendHeaders((details, callback) => {
    const requestHeaders = details.requestHeaders || {};
    const header = tokenHeaderForRequest(details, getBackend());
    callback({ requestHeaders: header ? { ...requestHeaders, ...header } : requestHeaders });
  });
}

module.exports = {
  TOKEN_HEADER,
  isBackendUrl,
  installBackendTokenHeader,
  tokenHeaderFor,
  tokenHeaderForRequest,
};
