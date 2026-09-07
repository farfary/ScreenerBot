/**
 * Navigation/loading architecture contract.
 *
 * Main-page transitions have one viewport owner, page init hooks paint before
 * remote I/O, and table/subtab loads use the shared loading component. These
 * assertions guard the exact boundaries that previously let an outgoing table
 * share half the viewport with an incoming loader.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath, URL } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const templates = resolve(root, "src/webserver/templates");

async function source(relativePath) {
  return readFile(resolve(templates, relativePath), "utf8");
}

test("the router replaces the viewport with one canonical transition loader", async () => {
  const router = await source("scripts/core/router.js");
  const begin = router.slice(
    router.indexOf("function beginPageTransition"),
    router.indexOf("function finishPageTransition")
  );
  const load = router.slice(
    router.indexOf("export async function loadPage"),
    router.indexOf("function isConnectionError")
  );

  assert.match(begin, /mainContent\.replaceChildren\(loadingEl\)/);
  assert.doesNotMatch(begin, /mainContent\.appendChild\(loadingEl\)/);
  assert.match(begin, /TabBarManager\?\.hideAll\(\)/);
  assert.match(begin, /ActionBarManager\?\.hideAll\(\)/);

  const beginCall = load.indexOf("beginPageTransition(mainContent, navigationId)");
  const cacheBranch = load.indexOf("if (!pageEl)");
  assert.ok(beginCall !== -1 && beginCall < cacheBranch, "cached and uncached pages use one path");
});

test("CSS makes a split outgoing-page/loading state structurally impossible", async () => {
  const layout = await source("styles/layout.css");
  assert.match(
    layout,
    /\.content\[data-loading\] > :not\(\.page-loading\)\s*\{\s*display: none !important;/
  );
});

test("main page init hooks never block first paint", async () => {
  const pages = [
    "home",
    "tokens",
    "positions",
    "events",
    "services",
    "transactions",
    "filtering",
    "wallets",
    "tools",
    "assistant",
    "config",
    "trader",
  ];

  for (const page of pages) {
    const pageSource = await source(`scripts/pages/${page}.js`);
    assert.doesNotMatch(pageSource, /\basync\s+init\s*\(/, `${page} has an async init hook`);
    assert.doesNotMatch(pageSource, /\binit\s*:\s*async\b/, `${page} has an async init hook`);
  }
});

test("token subtabs clear stale rows before requesting the next view", async () => {
  const tokens = await source("scripts/pages/tokens.js");
  const switchView = tokens.slice(
    tokens.indexOf("const switchView = (view) =>"),
    tokens.indexOf("const handleLinkAction")
  );

  assert.match(switchView, /showTokensLoadingState\(\)/);
  assert.match(switchView, /preserveData: false/);
  assert.doesNotMatch(switchView, /preserveData: true/);
});

test("DataTable loading states use the shared spinner and no private animation", async () => {
  const table = await source("scripts/ui/data_table.js");
  const styles = await source("styles/ui/data_table/core.css");

  assert.match(table, /data-table-blocking-state__spinner loading-spinner inline/);
  assert.match(table, /dt-scroll-loader__indicator loading-spinner inline/);
  assert.match(table, /<div class="loading-spinner">\$\{this\.options\.loadingMessage\}<\/div>/);
  assert.doesNotMatch(table, /dt-loading-spinner/);
  assert.doesNotMatch(styles, /dt-loading-spinner|dt-loading-spin|data-table-blocking-spin/);
});

test("router waits for in-flight styles and lifecycle resource registration stays bounded", async () => {
  const router = await source("scripts/core/router.js");
  const lifecycle = await source("scripts/core/lifecycle.js");
  const requests = await source("scripts/core/request_manager.js");

  assert.match(router, /pageStyleLoads: new Map\(\)/);
  assert.match(router, /return waitForPageStylesheet\(pageName, existing\)/);
  assert.match(lifecycle, /const managedPollers = new Set\(\)/);
  assert.match(lifecycle, /if \(!managedPollers\.has\(poller\)\)/);
  assert.match(lifecycle, /Array\.from\(managedPollers,[\s\S]*?poller\.stop/);
  assert.match(lifecycle, /releaseAbortController\(controller\)/);
  assert.match(requests, /\.finally\(\(\) => \{[\s\S]*?releaseAbortController/);
});

test("tab pollers use the Poller lifecycle state and declared cadence", async () => {
  const trader = await source("scripts/pages/trader.js");
  const wallets = await source("scripts/pages/wallets.js");

  assert.doesNotMatch(
    trader,
    /Poller[^\n]*\.running|\b(?:stats|walletCopy|strategies)Poller\??\.running/
  );
  assert.match(wallets, /\{ label: "Wallets", intervalMs: POLL_INTERVAL \}/);
});

test("Watched wallets creates its DataTable only after its panel is active", async () => {
  const watched = await source("scripts/pages/wallets/watched.js");
  const setup = watched.slice(
    watched.indexOf("function setup()"),
    watched.indexOf("function showAddModal")
  );

  assert.doesNotMatch(setup, /ensureTable\(\)/);
  assert.match(watched, /async function load[\s\S]*?const t = ensureTable\(\)/);
});

test("token special-view resources have one owner and obey page lifecycle cleanup", async () => {
  const tokens = await source("scripts/pages/tokens.js");
  const ohlcv = await source("scripts/pages/tokens/ohlcv.js");
  const favorites = await source("scripts/pages/tokens/favorites.js");

  assert.doesNotMatch(tokens, /let (?:ohlcvTable|ohlcvPoller|favoritesTable)\s*=/);
  assert.match(tokens, /deps\.ohlcvTable\.destroy\(\)/);
  assert.match(tokens, /deps\.favoritesTable\.destroy\(\)/);
  assert.match(ohlcv, /deps\.managePoller\?\.\(deps\.ohlcvPoller\)/);
  assert.doesNotMatch(ohlcv, /\.pause\(\)/);
  assert.doesNotMatch(favorites, /\.pause\(\)/);
});
