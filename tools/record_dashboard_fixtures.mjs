// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Refreshes the dashboard stability fixtures from a running dashboard (for example Explore Mode).
// Never run in CI: `npm run record:dashboard -- http://localhost:<port> [page,page]`.

import { mkdir, writeFile } from "node:fs/promises";
import { dirname } from "node:path";

import { PAGE_IDS } from "./lib/dashboard_host.mjs";
import { fixtureFile, loadIndex } from "./lib/dashboard_fixtures.mjs";

const [base, only] = process.argv.slice(2);
if (!base || !/^https?:\/\//.test(base)) {
  console.error("usage: node tools/record_dashboard_fixtures.mjs <dashboard url> [page,page]");
  process.exit(2);
}

const names = ["shell", ...(only ? only.split(",") : PAGE_IDS)];
let failed = 0;

for (const name of names) {
  const index = await loadIndex(name);
  for (const endpoint of index.endpoints) {
    if (endpoint.method !== "GET" || endpoint.record === null) continue;
    const url = new URL(endpoint.record ?? endpoint.path, base);
    try {
      const response = await fetch(url, { headers: { accept: "application/json" } });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const file = fixtureFile(index, endpoint, "populated");
      await mkdir(dirname(file), { recursive: true });
      const body = endpoint.contentType
        ? await response.text()
        : `${JSON.stringify(await response.json(), null, 2)}\n`;
      await writeFile(file, body);
      console.log(`recorded ${name}: ${endpoint.record ?? endpoint.path}`);
    } catch (error) {
      failed += 1;
      console.error(`failed ${name}: ${url.pathname}${url.search}: ${error.message}`);
    }
  }
}

process.exit(failed ? 1 : 0);
