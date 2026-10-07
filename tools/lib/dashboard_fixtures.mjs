// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Registry of recorded `/api` responses for the dashboard stability suite.
 *
 * Each directory under `tools/tests/fixtures/dashboard/` (`shell` plus one per
 * page) owns an `index.mjs` exporting:
 *   - `endpoints`: the responses the page reads. An entry is
 *     `{ method, path, fixture, rust, query?, empty?, record? }`:
 *       `path`     route pattern, `{name}` matches one segment;
 *       `query`    parameters a request must carry for the entry to match;
 *       `fixture`  JSON file served as the populated response;
 *       `empty`    JSON file served instead in the empty variant;
 *       `rust`     `src/...rs::Type`, the type the route serializes;
 *       `contentType` response type when not JSON (the action stream);
 *       `record`   URL the recorder fetches (default `path`), `null` when the
 *                  endpoint must not be fetched by a recorder (writes, `{name}`
 *                  routes without a sample id).
 *   - `views`: what to assert (see `tools/tests/dashboard_stability.test.mjs`).
 *
 * Consumers: `tools/tests/dashboard_stability.test.mjs`, `tools/record_dashboard_fixtures.mjs`.
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { REPO_ROOT } from "./dashboard_ui.mjs";

export const FIXTURES_ROOT = resolve(REPO_ROOT, "tools/tests/fixtures/dashboard");

export async function loadIndex(name) {
  const module = await import(pathToFileURL(resolve(FIXTURES_ROOT, name, "index.mjs")).href);
  return { name, endpoints: module.endpoints ?? [], views: module.views ?? [] };
}

function pathMatcher(pattern) {
  const source = pattern
    .split("/")
    .map((part) => (/^\{\w+\}$/.test(part) ? "[^/]+" : part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")))
    .join("/");
  return new RegExp(`^${source}$`);
}

export function fixtureFile(index, endpoint, variant) {
  const file = variant === "empty" && endpoint.empty ? endpoint.empty : endpoint.fixture;
  return resolve(FIXTURES_ROOT, index.name, file);
}

/**
 * `{{THEME}}` in a fixture becomes the theme the session opens with, because the
 * dashboard reads its saved theme from the API.
 *
 * Answer `/api` requests from the indexes in `indexes` (first match wins, so the
 * page index comes before the shell index). Requests no entry matches are
 * recorded in `unmocked` and answered 501.
 */
export function createApiHandler(indexes, variant, { theme = "dark" } = {}) {
  const entries = indexes.flatMap((index) =>
    index.endpoints.map((endpoint) => ({ index, endpoint, matcher: pathMatcher(endpoint.path) }))
  );
  const unmocked = [];
  const served = new Set();
  async function onApi({ method, url }) {
    const match = entries.find(
      ({ endpoint, matcher }) =>
        endpoint.method === method &&
        matcher.test(url.pathname) &&
        Object.entries(endpoint.query ?? {}).every(
          ([key, value]) => url.searchParams.get(key) === String(value)
        )
    );
    if (!match) {
      unmocked.push(`${method} ${url.pathname}${url.search}`);
      return { status: 501, body: JSON.stringify({ error: "no fixture for this request" }) };
    }
    served.add(`${match.index.name}:${method} ${match.endpoint.path}`);
    return {
      body: readFileSync(fixtureFile(match.index, match.endpoint, variant), "utf8").replaceAll(
        "{{THEME}}",
        theme
      ),
      contentType: match.endpoint.contentType,
    };
  }
  return { onApi, unmocked, served };
}
