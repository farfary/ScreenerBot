// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Clears the shell session's HTTP cache and V8 code caches before the first page loads, so a core that replaced the previous one never runs against dashboard assets or compiled scripts cached from it.

'use strict';

// The dashboard is served by the core on a fixed local origin. A cached response
// or a cached compilation of a script from the previous core would otherwise be
// reused after an update. This module never imports `electron`; the caller passes
// the session, so plain node tests can exercise it.

/**
 * Clear `session`'s HTTP cache and code caches. Never throws: a cache that cannot
 * be cleared is logged and the shell starts anyway, since the assets are still
 * revalidated against the core.
 *
 * @param {{ clearCache: () => Promise<void>, clearCodeCaches: (options: object) => Promise<void> }} session
 * @param {{ warn: (...args: unknown[]) => void }} [log]
 * @returns {Promise<{ httpCache: boolean, codeCaches: boolean }>} which caches were cleared
 */
async function clearShellCaches(session, log = console) {
  const [httpCache, codeCaches] = await Promise.allSettled([
    Promise.resolve().then(() => session.clearCache()),
    Promise.resolve().then(() => session.clearCodeCaches({})),
  ]);
  if (httpCache.status === 'rejected') {
    log.warn('[Electron] HTTP cache could not be cleared:', httpCache.reason?.message ?? httpCache.reason);
  }
  if (codeCaches.status === 'rejected') {
    log.warn('[Electron] Code caches could not be cleared:', codeCaches.reason?.message ?? codeCaches.reason);
  }
  return {
    httpCache: httpCache.status === 'fulfilled',
    codeCaches: codeCaches.status === 'fulfilled',
  };
}

module.exports = { clearShellCaches };
