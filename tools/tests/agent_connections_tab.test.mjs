/**
 * Tests for the Agent Connections settings tab (`settings/agent_connections_tab.js`).
 *
 * Only the pure config generators are exercised: the DOM builders and the loader
 * dynamically import `core/utils.js` / `ui/confirmation_dialog.js`, which need a
 * browser, so importing the module under node yields just the helpers.
 *
 * What matters here: every client gets its OWN native artifact (not one generic
 * JSON), the generated shell commands are safely quoted and carry the credential
 * in the environment only, the one-time secret appears exactly once per block,
 * and nothing references the removed `install-mcp.sh` configurator.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

const MODULE = "../../src/webserver/templates/scripts/ui/settings/agent_connections_tab.js";

async function mod() {
  return import(`${MODULE}?t=${Math.random()}`);
}

const CID = "11111111-2222-3333-4444-555555555555";
const SECRET = "Abc123_one-time-secret-value_do-not-log-0000";

test("module imports under node with only the pure helpers", async () => {
  const m = await mod();
  for (const name of [
    "shQuote",
    "exePath",
    "mcpServerEntry",
    "genericStdioJson",
    "codexToml",
    "hermesYaml",
    "claudeCodeCommand",
    "codexCommand",
    "openClawCommand",
    "clientSetup",
    "validateLabel",
  ]) {
    assert.equal(typeof m[name], "function", `${name} is exported`);
  }
});

test("exePath falls back to the marked placeholder, uses a real path verbatim", async () => {
  const { exePath, EXE_PLACEHOLDER } = await mod();
  assert.equal(exePath(null), EXE_PLACEHOLDER);
  assert.equal(exePath(""), EXE_PLACEHOLDER);
  assert.equal(exePath("   "), EXE_PLACEHOLDER);
  assert.equal(exePath("/opt/screener bot/screenerbot"), "/opt/screener bot/screenerbot");
});

test("shQuote wraps in single quotes and escapes embedded single quotes", async () => {
  const { shQuote } = await mod();
  assert.equal(shQuote("plain"), "'plain'");
  assert.equal(shQuote("a'b"), `'a'\\''b'`);
});

test("mcpServerEntry runs the native binary with the mcp serve args, credential in env", async () => {
  const { mcpServerEntry, EXE_PLACEHOLDER } = await mod();
  const entry = mcpServerEntry(null, CID, SECRET);
  assert.equal(entry.command, EXE_PLACEHOLDER);
  assert.deepEqual(entry.args, ["mcp", "serve"]);
  assert.deepEqual(entry.env, {
    SCREENERBOT_CLIENT_ID: CID,
    SCREENERBOT_PAIRING_SECRET: SECRET,
  });
  // No package / npx / node / installer anywhere.
  assert.ok(!JSON.stringify(entry).match(/npx|npm|@screenerbot|node_modules|install-mcp/i));
});

test("claudeCodeCommand is a native `claude mcp add` with quoted, env-only credential", async () => {
  const { claudeCodeCommand, EXE_PLACEHOLDER } = await mod();
  const cmd = claudeCodeCommand("/bin/screenerbot", CID, SECRET);
  assert.match(cmd, /^claude mcp add --scope user screenerbot /);
  assert.match(cmd, new RegExp(`-e 'SCREENERBOT_CLIENT_ID=${CID}'`));
  assert.match(cmd, new RegExp(`-e 'SCREENERBOT_PAIRING_SECRET=${esc(SECRET)}'`));
  assert.match(cmd, /-- '\/bin\/screenerbot' mcp serve$/);
  // The secret is only ever inside the -e value, never a bare CLI arg.
  assert.equal(cmd.split(SECRET).length - 1, 1);
  // Placeholder path is quoted too.
  assert.match(claudeCodeCommand(null, CID, SECRET), new RegExp(`-- '${esc(EXE_PLACEHOLDER)}' mcp serve$`));
});

test("codexCommand is a native `codex mcp add` with --env and quoting", async () => {
  const { codexCommand } = await mod();
  const cmd = codexCommand("/bin/screenerbot", CID, SECRET);
  assert.match(cmd, /^codex mcp add screenerbot /);
  assert.match(cmd, /--env 'SCREENERBOT_CLIENT_ID=/);
  assert.match(cmd, /--env 'SCREENERBOT_PAIRING_SECRET=/);
  assert.match(cmd, /-- '\/bin\/screenerbot' mcp serve$/);
});

test("shell commands survive an executable path with a single quote", async () => {
  const { claudeCodeCommand, codexCommand } = await mod();
  const nasty = "/home/o'brien/screenerbot";
  assert.match(claudeCodeCommand(nasty, CID, SECRET), /-- '\/home\/o'\\''brien\/screenerbot' mcp serve$/);
  assert.match(codexCommand(nasty, CID, SECRET), /-- '\/home\/o'\\''brien\/screenerbot' mcp serve$/);
});

test("generic and Claude Desktop use the mcpServers.screenerbot stdio JSON", async () => {
  const { genericStdioJson } = await mod();
  const parsed = JSON.parse(genericStdioJson(null, CID, SECRET));
  assert.deepEqual(Object.keys(parsed), ["mcpServers"]);
  assert.ok(parsed.mcpServers.screenerbot);
  assert.deepEqual(parsed.mcpServers.screenerbot.args, ["mcp", "serve"]);
  assert.equal(parsed.mcpServers.screenerbot.env.SCREENERBOT_PAIRING_SECRET, SECRET);
});

test("openClawCommand uses OpenClaw's native saved-server CLI", async () => {
  const { openClawCommand } = await mod();
  const cmd = openClawCommand("/bin/screenerbot", CID, SECRET);
  assert.match(cmd, /^openclaw mcp add screenerbot /);
  assert.match(cmd, /--command '\/bin\/screenerbot'/);
  assert.match(cmd, /--arg 'mcp' --arg 'serve'/);
  assert.match(cmd, /--env 'SCREENERBOT_CLIENT_ID=/);
  assert.match(cmd, /--env 'SCREENERBOT_PAIRING_SECRET=/);
  assert.equal(cmd.split(SECRET).length - 1, 1);
});

test("codex TOML declares [mcp_servers.screenerbot] with an env sub-table", async () => {
  const { codexToml } = await mod();
  const toml = codexToml("/bin/screenerbot", CID, SECRET);
  assert.match(toml, /^\[mcp_servers\.screenerbot\]$/m);
  assert.match(toml, /^\[mcp_servers\.screenerbot\.env\]$/m);
  assert.match(toml, /command = "\/bin\/screenerbot"/);
  assert.match(toml, /args = \["mcp", "serve"\]/);
  assert.match(toml, new RegExp(`SCREENERBOT_CLIENT_ID = "${CID}"`));
  assert.match(toml, new RegExp(`SCREENERBOT_PAIRING_SECRET = "${esc(SECRET)}"`));
});

test("TOML string escaping handles quotes/backslashes", async () => {
  const { codexToml } = await mod();
  const toml = codexToml('a"b\\c', CID, SECRET);
  assert.match(toml, /command = "a\\"b\\\\c"/);
});

test("Hermes YAML uses the documented mcp_servers shape, not Claude JSON", async () => {
  const { hermesYaml } = await mod();
  const yaml = hermesYaml("/bin/screenerbot", CID, SECRET);
  assert.match(yaml, /^mcp_servers:$/m);
  assert.match(yaml, /^ {2}screenerbot:$/m);
  assert.match(yaml, /^ {4}command: "\/bin\/screenerbot"$/m);
  assert.match(yaml, /^ {4}args: \["mcp", "serve"\]$/m);
  assert.match(yaml, /^ {6}SCREENERBOT_PAIRING_SECRET: "/m);
  // It must not be JSON.
  assert.throws(() => JSON.parse(yaml));
});

test("clientSetup gives Claude and Codex native commands, others a config artifact", async () => {
  const { clientSetup } = await mod();

  const claude = clientSetup("claude", null, CID, SECRET);
  assert.deepEqual(
    claude.blocks.map((b) => b.lang),
    ["sh", "json"]
  );
  assert.match(claude.blocks[0].body, /^claude mcp add /);
  assert.match(claude.blocks[1].body, /"mcpServers"/);

  const codex = clientSetup("codex", null, CID, SECRET);
  assert.deepEqual(
    codex.blocks.map((b) => b.lang),
    ["sh", "toml"]
  );
  assert.match(codex.blocks[0].body, /^codex mcp add /);
  assert.match(codex.blocks[1].body, /\[mcp_servers\.screenerbot\]/);

  assert.equal(clientSetup("hermes", null, CID, SECRET).blocks[0].lang, "yaml");
  const openclaw = clientSetup("openclaw", null, CID, SECRET);
  assert.equal(openclaw.blocks[0].lang, "sh");
  assert.match(openclaw.blocks[0].body, /^openclaw mcp add /);
  assert.equal(clientSetup("generic", null, CID, SECRET).blocks[0].lang, "json");
  // Unknown id falls back to the generic stdio object, never throws.
  assert.equal(clientSetup("something-else", null, CID, SECRET).blocks[0].lang, "json");
});

test("every clientSetup block carries the secret exactly once, and notes never do", async () => {
  const { clientSetup, SETUP_CLIENTS } = await mod();
  for (const { id } of SETUP_CLIENTS) {
    const setup = clientSetup(id, null, CID, SECRET);
    for (const block of setup.blocks) {
      assert.equal(block.body.split(SECRET).length - 1, 1, `${id}/${block.label}: secret once`);
    }
    for (const note of setup.notes) {
      assert.ok(!note.includes(SECRET), `${id}: note has no secret`);
    }
  }
});

test("placeholder is only a path-encoding fallback; a reported path needs no replacement", async () => {
  const { clientSetup, EXE_PLACEHOLDER } = await mod();
  const withPlaceholder = clientSetup("claude", null, CID, SECRET);
  assert.ok(withPlaceholder.notes.some((n) => n.includes(EXE_PLACEHOLDER)));
  assert.ok(withPlaceholder.notes.some((n) => /could not represent/.test(n)));
  const withPath = clientSetup("claude", "/usr/local/bin/screenerbot", CID, SECRET);
  assert.ok(!withPath.notes.some((n) => n.includes(EXE_PLACEHOLDER)));
});

test("setup notes cover the non-default data directory case", async () => {
  const { clientSetup } = await mod();
  const setup = clientSetup("generic", "/bin/screenerbot", CID, SECRET);
  assert.ok(setup.notes.some((n) => /SCREENERBOT_DATA_DIR/.test(n)));
});

test("nothing in the module references the removed install-mcp.sh configurator", async () => {
  const { readFile } = await import("node:fs/promises");
  const { fileURLToPath } = await import("node:url");
  const src = await readFile(
    fileURLToPath(new URL(MODULE, import.meta.url)),
    "utf8"
  );
  assert.ok(!/install-mcp/i.test(src));
});

test("least-privilege scope: read is the default and first option", async () => {
  const { SCOPE_OPTIONS, DEFAULT_SCOPE } = await mod();
  assert.equal(DEFAULT_SCOPE, "read");
  assert.equal(SCOPE_OPTIONS[0].value, "read");
  assert.deepEqual(
    SCOPE_OPTIONS.map((s) => s.value),
    ["read", "operate", "trade"]
  );
});

test("validateLabel matches the backend bounds (1..=64, no control chars)", async () => {
  const { validateLabel } = await mod();
  assert.equal(validateLabel("  My agent  ").value, "My agent");
  assert.equal(validateLabel("").ok, false);
  assert.equal(validateLabel("   ").ok, false);
  assert.equal(validateLabel("x".repeat(65)).ok, false);
  assert.equal(validateLabel("x".repeat(64)).ok, true);
  assert.equal(validateLabel("bad\u0007bell").ok, false);
});

// ── helpers ─────────────────────────────────────────────────────────────────

function esc(value) {
  return value.replace(/[-/\\^$*+?.()|[\]{}]/g, "\\$&");
}
