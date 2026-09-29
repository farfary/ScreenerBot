/**
 * Every dashboard script and stylesheet must be embedded in the binary.
 *
 * Assets are compiled in with `include_str!` in `src/webserver/embeds.rs` and
 * served by explicit match arms, so a file that exists on disk but is not
 * registered is a 404 at runtime while every Node test still passes.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..", "..");
const TEMPLATES = path.join(ROOT, "src/webserver/templates");
const EMBEDS = fs.readFileSync(path.join(ROOT, "src/webserver/embeds.rs"), "utf8");

function walk(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    return entry.isDirectory() ? walk(full) : [full];
  });
}

test("every dashboard script and stylesheet is embedded", () => {
  const embedded = new Set(
    [...EMBEDS.matchAll(/include_str!\(\s*"templates\/([^"]+)"\s*\)/g)].map((m) => m[1])
  );
  const missing = walk(TEMPLATES)
    .filter((file) => file.endsWith(".js") || file.endsWith(".css"))
    .map((file) => path.relative(TEMPLATES, file).split(path.sep).join("/"))
    .filter((relative) => !embedded.has(relative))
    .sort();
  assert.deepEqual(missing, [], `not embedded in src/webserver/embeds.rs: ${missing.join(", ")}`);
});

test("every routed dashboard script has a serving arm", () => {
  const handlers = fs.readFileSync(
    path.join(ROOT, "src/webserver/routes/asset_serving/handlers.rs"),
    "utf8"
  );
  // /scripts/{core,pages,ui,promo}/<file> are served by match arms keyed on the
  // path below that directory.
  const missing = walk(path.join(TEMPLATES, "scripts"))
    .filter((file) => file.endsWith(".js"))
    .map((file) => path.relative(path.join(TEMPLATES, "scripts"), file).split(path.sep))
    .filter((parts) => parts.length > 1 && ["core", "pages", "ui", "promo"].includes(parts[0]))
    .map((parts) => parts.slice(1).join("/"))
    .filter((served) => !handlers.includes(`"${served}"`))
    .sort();
  assert.deepEqual(missing, [], `no serving arm in asset_serving/handlers.rs: ${missing.join(", ")}`);
});
