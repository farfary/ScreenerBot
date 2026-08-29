/**
 * The choice-control alignment contract (`components/form_controls.css`).
 *
 * A checkbox or radio is positioned against its label text in exactly one
 * place. Every rule here exists because the dashboard once had five answers to
 * the same question - `margin-top: 0.15em`, `0.2rem`, `2px`, `3px` and a
 * `--centered` modifier - so a control's height beside its text depended on
 * which file happened to render it.
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
const OWNER = resolve(stylesRoot, "components/form_controls.css");

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

test("the offset is derived from type metrics, not a magic number", async () => {
  const css = await readFile(OWNER, "utf8");
  for (const token of [
    "--control-line-height",
    "--control-text-ascent",
    "--control-text-descent",
    "--control-text-x-height",
    "--control-box-size",
    "--control-line-offset",
  ]) {
    assert.ok(css.includes(`${token}:`), `${token} must be defined`);
  }
  assert.match(css, /--control-line-offset:\s*calc\(/);
});

test("the control inherits the label's font size so the em math holds", async () => {
  const css = await readFile(OWNER, "utf8");
  for (const type of ["checkbox", "radio"]) {
    const start = css.indexOf(`\ninput[type="${type}"] {`);
    assert.ok(start > 0, `the base input[type="${type}"] rule must exist`);
    const body = css.slice(start, css.indexOf("}", start));
    assert.match(body, /font-size:\s*inherit/);
  }
});

test("an unclassed label row is aligned without opting in", async () => {
  const css = await readFile(OWNER, "utf8");
  assert.match(css, /:where\(\s*label:not\(\.toggle\):has\(> input\[type="checkbox"\]\)/);
  assert.match(css, /margin-block-start:\s*var\(--control-line-offset\)/);
});

test("no stylesheet outside form_controls.css places a choice control", async () => {
  const offenders = [];
  for (const file of await walk(stylesRoot)) {
    if (!file.endsWith(".css") || file === OWNER) continue;
    const css = (await readFile(file, "utf8")).replace(/\/\*[\s\S]*?\*\//g, "");
    for (const [, selector, body] of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      if (!/input(?![\w-])/.test(selector)) continue;
      if (/\[type=["']?(?:text|number|password|email|search|url|tel|date|time)/.test(selector)) {
        continue;
      }
      if (/^\s*(?:margin(?:-top|-block-start|-block)?|vertical-align|align-self)\s*:/m.test(body)) {
        offenders.push(`${relative(root, file)}: ${selector.trim()}`);
      }
    }
  }
  assert.deepEqual(offenders, []);
});

test("alignment is never expressed as a markup variant", async () => {
  const sources = [...(await walk(scriptsRoot)), ...(await walk(pagesRoot))].filter((file) =>
    /\.(?:js|html)$/.test(file)
  );
  const offenders = [];
  for (const file of sources) {
    const source = await readFile(file, "utf8");
    if (/checkbox-label--/.test(source)) offenders.push(relative(root, file));
  }
  assert.deepEqual(offenders, []);
});

test("no page styles `label` generically and unflexes a control row", async () => {
  const offenders = [];
  for (const file of await walk(stylesRoot)) {
    if (!file.endsWith(".css")) continue;
    const css = (await readFile(file, "utf8")).replace(/\/\*[\s\S]*?\*\//g, "");
    for (const [, selectors, body] of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      if (selectors.trim().startsWith("@")) continue;
      for (const selector of selectors.split(",")) {
        const compounds = selector
          .trim()
          .split(/[\s>+~](?![^(]*\))/)
          .filter(Boolean);
        if (compounds.length < 2) continue;
        const target = compounds[compounds.length - 1];
        if (!/^label(?![\w-])/.test(target)) continue;
        if (/\.checkbox-label/.test(target)) continue;
        if (/:has\(>\s*input\[type=["']?(?:checkbox|radio)/.test(target)) continue;
        if (/:not\([^)]*(?:checkbox-label|\[type=["']?(?:checkbox|radio))/.test(target)) continue;
        if (
          /^\s*(?:display|align-items|gap|margin(?:-[a-z]+)?|font-size|line-height|font-weight)\s*:/m.test(
            body
          )
        ) {
          offenders.push(`${relative(root, file)}: ${selector.trim()}`);
        }
      }
    }
  }
  assert.deepEqual(offenders, []);
});
