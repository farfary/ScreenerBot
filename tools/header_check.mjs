// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// File-header lint: every first-party source file must open with the standard
// license block, in the exact per-syntax shape, followed by a description
// comment. Vendored and generated files are excluded below; extend EXCLUDED,
// never the checks, when a new file legitimately cannot carry the header.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = join(dirname(fileURLToPath(import.meta.url)), "..");
const HOLDER = "Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)";
const SPDX = "SPDX-License-Identifier: BUSL-1.1";

// Vendored third-party libraries keep their upstream license headers; generated
// files are owned by their generators.
const EXCLUDED = [/^src\/webserver\/assets\//, /^src\/webserver\/templates\/styles\/base\/script_fonts\.css$/];

const SETS = {
  rs: ["src/*.rs", "tests/*.rs", "build.rs", "build/*.rs"],
  js: ["src/webserver/templates/scripts/*.js", "tools/*.mjs", "electron/*.js", "eslint.config.js"],
  css: ["src/webserver/templates/*.css"],
  html: ["src/webserver/templates/*.html", "electron/src/*.html"],
  hash: ["tools/*.py", "screenerbot.sh"],
};

function tracked(pattern) {
  return execFileSync("git", ["-C", REPO, "ls-files", "--", pattern], { encoding: "utf8" })
    .split("\n")
    .filter(Boolean);
}

function checkRust(lines) {
  return (
    lines[0] === `// ${HOLDER}` &&
    lines[1] === `// ${SPDX}` &&
    lines.slice(2, 6).some((l) => l.startsWith("//! "))
  );
}

function checkJs(lines) {
  const at = lines[0]?.startsWith("#!") ? 1 : 0; // a node shebang must stay on line 1
  if (lines[at] !== `// ${HOLDER}` || lines[at + 1] !== `// ${SPDX}` || lines[at + 2] !== "//") return false;
  // accepted descriptions: // lines, //! lines (some dashboard files use Rust-style docs), or a /* */ block
  return (
    lines[at + 3]?.startsWith("// ") ||
    lines[at + 3]?.startsWith("//!") ||
    (lines[at + 3] === "" && lines[at + 4]?.startsWith("/*"))
  );
}

function checkCss(lines) {
  // either a license-only block, or one merged block that continues into the description
  return lines[0] === `/* ${HOLDER}` && (lines[1] === ` * ${SPDX} */` || lines[1] === ` * ${SPDX}`);
}

function checkHtml(lines) {
  const at = lines[0]?.toLowerCase().startsWith("<!doctype") ? 1 : 0;
  return (
    lines[at] === `<!-- ${HOLDER}` &&
    (lines[at + 1] === `     ${SPDX} -->` || lines[at + 1] === `     ${SPDX}`)
  );
}

function checkHash(lines) {
  const at = lines[0]?.startsWith("#!") ? 1 : 0;
  return lines[at] === `# ${HOLDER}` && lines[at + 1] === `# ${SPDX}`;
}

const CHECKERS = { rs: checkRust, js: checkJs, css: checkCss, html: checkHtml, hash: checkHash };

const violations = [];
let checked = 0;
for (const [fam, patterns] of Object.entries(SETS)) {
  const files = [...new Set(patterns.flatMap(tracked))].sort();
  for (const rel of files) {
    if (EXCLUDED.some((re) => re.test(rel))) continue;
    checked += 1;
    const lines = readFileSync(join(REPO, rel), "utf8").split("\n");
    if (!CHECKERS[fam](lines)) violations.push(`${rel}: missing or malformed standard header`);
    else if (lines.filter((l) => l.includes(HOLDER)).length !== 1)
      violations.push(`${rel}: license block appears more than once`);
  }
}

if (violations.length > 0) {
  console.error(`header check: ${violations.length} of ${checked} files violate the standard\n`);
  console.error(violations.map((v) => `  ${v}`).join("\n"));
  process.exit(1);
}
console.log(`header check: OK, ${checked} files carry the standard header`);
