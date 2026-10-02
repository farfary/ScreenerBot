// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/* global console, process */

/**
 * Localization audit for the desktop dashboard and its catalogs.
 *
 *   node tools/i18n/audit.mjs                  run every check
 *
 * Catalog parity, key usage, value-formatting errors (toLocale*String, Intl and the
 * "en-US" literal outside `core/format.js`), hardcoded user-visible strings and
 * physical-direction CSS all fail. The escapes are `// l10n-ignore: <reason>` for a
 * string and `/* rtl-ok: <reason> *\/` for a CSS declaration.
 */

import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { REPO_ROOT, TEMPLATES_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";
import { checkCatalogs, loadCatalogs } from "./catalogs.mjs";
import { scanCss } from "./css_direction.mjs";
import { scanFormatting } from "./formatting.mjs";
import { scanHtmlHardcoded, scanJsHardcoded } from "./hardcoded.mjs";
import { scanUsage } from "./usage.mjs";

/** `{ path: count }` for the files with a non-zero count. */
export function countsOf(results) {
  const counts = {};
  for (const [path, result] of Object.entries(results)) {
    if (result.items.length > 0) counts[path] = result.items.length;
  }
  return counts;
}

async function readAll(files) {
  return Promise.all(files.map(async (file) => ({ path: repoPath(file), source: await readFile(file, "utf8") })));
}

async function loadSources() {
  const templates = await walk(TEMPLATES_ROOT);
  const rust = await walk(resolve(REPO_ROOT, "src"));
  const shell = (await walk(resolve(REPO_ROOT, "electron/src"))).filter((file) => /\.(js|html)$/.test(file)).sort();
  const pick = (files, extension) => files.filter((file) => file.endsWith(extension)).sort();
  return {
    js: await readAll(pick(templates, ".js")),
    html: await readAll(pick(templates, ".html")),
    css: await readAll(pick(templates, ".css")),
    // Test modules do not count as usage: a key referenced only by tests is unused.
    shell: await readAll(shell),
    rust: await readAll(pick(rust, ".rs").filter((file) => !/(^|\/)tests?(_\w+)?\.rs$/.test(file))),
  };
}

function topFiles(counts, limit = 10) {
  return Object.entries(counts)
    .sort(([, a], [, b]) => b - a)
    .slice(0, limit);
}

const sum = (counts) => Object.values(counts).reduce((total, count) => total + count, 0);

/** Run every scan. Pure over the loaded sources and catalogs. */
export function analyze({ sources, catalogInput }) {
  const catalog = checkCatalogs(catalogInput);
  const usage = scanUsage({
    ids: catalog.sourceIds,
    js: sources.js,
    html: sources.html,
    rust: sources.rust,
    shell: sources.shell,
  });
  const errors = [...catalog.errors, ...usage.errors];
  /* A server-only domain is never in the dashboard payload, so a dashboard reference renders the raw id. */
  const serverOnly = catalogInput.serverOnly ?? new Set();
  for (const [id, file] of usage.dashboardUsed) {
    const domain = catalog.sourceDomains.get(id);
    if (serverOnly.has(domain)) {
      errors.push({ file, message: `"${id}" is used by the dashboard but ${domain}.ftl is server-only` });
    }
  }

  const hardcoded = {};
  const css = {};
  let ignores = 0;
  let rtlOk = 0;
  let formatOk = 0;
  const scan = (files, scanner, into, tally) => {
    for (const file of files) {
      const result = scanner({ ...file, ids: catalog.sourceIds });
      into[file.path] = result;
      errors.push(...result.errors);
      tally(result.ignores);
    }
  };
  scan(sources.js, scanJsHardcoded, hardcoded, (n) => (ignores += n));
  scan(sources.html, scanHtmlHardcoded, hardcoded, (n) => (ignores += n));
  scan(sources.css, scanCss, css, (n) => (rtlOk += n));
  for (const [path, result] of Object.entries(hardcoded)) {
    for (const item of result.items) {
      errors.push({
        file: path,
        line: item.line,
        message: `hardcoded user-visible string (${item.kind}): ${JSON.stringify(item.text)}; localize it or annotate // l10n-ignore: <reason>`,
      });
    }
  }
  for (const [path, result] of Object.entries(css)) {
    for (const item of result.items) {
      errors.push({
        file: path,
        line: item.line,
        message: `physical-direction CSS (${item.kind}): use the logical property or annotate /* rtl-ok: <reason> */`,
      });
    }
  }
  for (const file of sources.js) {
    const result = scanFormatting(file);
    errors.push(...result.errors);
    formatOk += result.escapes;
  }

  return {
    errors,
    info: catalog.info,
    completeness: catalog.completeness,
    ignores,
    rtlOk,
    formatOk,
    details: { hardcoded, cssDirection: css },
    current: { hardcoded: countsOf(hardcoded), cssDirection: countsOf(css) },
  };
}

export function formatSummary(result) {
  const lines = ["Localization audit", ""];
  const titles = { hardcoded: "Hardcoded user-visible strings", cssDirection: "Physical-direction CSS declarations" };
  for (const category of ["hardcoded", "cssDirection"]) {
    const counts = result.current[category];
    lines.push(`${titles[category]}: ${sum(counts)} in ${Object.keys(counts).length} files`);
    for (const [path, count] of topFiles(counts)) lines.push(`  ${String(count).padStart(5)}  ${path}`);
  }
  lines.push("", `l10n-ignore comments: ${result.ignores}`, `rtl-ok comments: ${result.rtlOk}`, `l10n-format-ok comments: ${result.formatOk}`);
  lines.push("Locale completeness:");
  for (const [code, percent] of result.completeness) lines.push(`  ${code}: ${percent}%`);
  for (const line of result.info) lines.push(`info: ${line}`);
  return lines.join("\n");
}

const place = (error) => (error.line ? `${error.file}:${error.line}` : error.file);

async function main() {
  const [sources, catalogInput] = await Promise.all([loadSources(), loadCatalogs()]);
  const result = analyze({ sources, catalogInput });
  console.log(formatSummary(result));

  const problems = result.errors.map((error) => `${place(error)}  ${error.message}`);
  if (problems.length > 0) {
    console.error(`\n${problems.length} problem lines:`);
    for (const problem of problems) console.error(`  ${problem}`);
    return 1;
  }
  console.log("\nPASS");
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = await main();
}
