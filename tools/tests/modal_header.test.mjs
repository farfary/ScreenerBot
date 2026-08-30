/**
 * The modal header contract (`components.css`).
 *
 * `.modal-header` owns its own height and its title. The header once padded
 * itself with `--spacing-lg` on both edges while every other dialog header in
 * the app used `--spacing-md`, and it left the heading element unstyled, so a
 * modal's title size depended on whether its markup happened to use `h2` or
 * `h3` and an inline icon sat on the text baseline instead of beside it.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { resolve, relative } from "node:path";
import { fileURLToPath, URL } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const stylesRoot = resolve(root, "src/webserver/templates/styles");
const scriptsRoot = resolve(root, "src/webserver/templates/scripts");
const pagesRoot = resolve(root, "src/webserver/templates/pages");
const OWNER = resolve(stylesRoot, "components.css");

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(
    entries.map((entry) => {
      const path = resolve(directory, entry.name);
      return entry.isDirectory() ? walk(path) : [path];
    })
  );
  return files.flat();
}

function ruleBody(css, selectorStart) {
  const start = css.indexOf(selectorStart);
  assert.notEqual(start, -1, `${selectorStart} is missing`);
  return css.slice(start, css.indexOf("}", start));
}

test("the modal header pads itself like every other dialog header", async () => {
  const css = await readFile(OWNER, "utf8");
  const body = ruleBody(css, "\n.modal-header {");
  assert.match(body, /padding:\s*var\(--spacing-md\) var\(--spacing-lg\)/);
});

test("the header owns the title whatever element carries it", async () => {
  const css = await readFile(OWNER, "utf8");
  const body = ruleBody(css, ".modal-header > :where(h1, h2, h3, h4, h5, h6, .modal-title) {");
  assert.match(body, /margin:\s*0/);
  assert.match(body, /font-size:\s*1rem/);
  assert.match(body, /display:\s*inline-flex/);
  assert.match(body, /align-items:\s*center/);
  assert.match(body, /gap:\s*var\(--control-row-gap\)|gap:\s*var\(--spacing-sm\)/);
});

test("no other stylesheet re-styles a modal title", async () => {
  const offenders = [];
  for (const file of await walk(stylesRoot)) {
    if (!file.endsWith(".css") || file === OWNER) continue;
    const css = await readFile(file, "utf8");
    for (const [, selector] of css.matchAll(/(?:^|[};])\s*([^{}@;]+)\{/g)) {
      if (/\.modal-header\b[^{]*(?:[\s>]h[1-6]|[\s>]\.modal-title)\b/.test(selector)) {
        offenders.push(`${relative(stylesRoot, file)}: ${selector.trim()}`);
      }
    }
  }
  assert.deepEqual(offenders, []);
});

test("every modal close button carries the icon, never a literal glyph", async () => {
  const offenders = [];
  for (const directory of [scriptsRoot, pagesRoot]) {
    for (const file of await walk(directory)) {
      if (!/\.(?:js|html)$/.test(file)) continue;
      const source = await readFile(file, "utf8");
      for (const match of source.matchAll(/class="modal-close"[\s\S]{0,240}?<\/button>/g)) {
        if (!match[0].includes("icon-x")) offenders.push(relative(root, file));
      }
    }
  }
  assert.deepEqual(offenders, []);
});
