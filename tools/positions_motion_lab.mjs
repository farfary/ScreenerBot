// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Replays live position changes on the dashboard so the Positions table can be reviewed in a browser.
// Never run in CI: `node tools/positions_motion_lab.mjs [--port 8090] [--locale en]`, then open the printed URL.

/**
 * The dashboard is served from source by `tools/lib/dashboard_host.mjs` on recorded data, with
 * three endpoints replaced by a scripted market: the action stream (`/api/actions/stream`, SSE),
 * the active actions (`/api/actions/active`) and the open and closed position lists. A loop
 * walks the lifecycles the live book shows: a buy in flight that becomes a position, a sale of
 * a row in the middle of the list, an add (DCA), a failed buy, and a buy landing while another
 * row closes. Prices tick every second, as pool prices do in the app.
 *
 * Control: `GET /lab/pause` (fails the actions in flight), `GET /lab/resume` (starts a cycle
 * from its first step), `GET /lab/drop?ms=6000` (cuts the action stream and refuses it for that
 * long, so the updates in between are missed), `GET /lab/state`. Source files are read once
 * per run, so restart the lab after editing a template.
 */

import { createServer, request as httpRequest } from "node:http";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { serveDashboard } from "./lib/dashboard_host.mjs";
import { createApiHandler, loadIndex, FIXTURES_ROOT } from "./lib/dashboard_fixtures.mjs";

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const at = args.indexOf(name);
  return at >= 0 && args[at + 1] ? args[at + 1] : fallback;
};
const PORT = Number(option("--port", "8090"));
const LOCALE = option("--locale", "en");

const fixture = (name) =>
  JSON.parse(readFileSync(resolve(FIXTURES_ROOT, "positions", name), "utf8"));

const nowSec = () => Math.floor(Date.now() / 1000);
const iso = (ms = Date.now()) => new Date(ms).toISOString();

/** Tokens the scripted buys rotate through. */
const CANDIDATES = [
  {
    mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
    symbol: "JUP",
    name: "Jupiter",
    price: 0.0042,
  },
  {
    mint: "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
    symbol: "RAY",
    name: "Raydium",
    price: 0.0118,
  },
  {
    mint: "jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL",
    symbol: "JTO",
    name: "Jito",
    price: 0.0109,
  },
  {
    mint: "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3",
    symbol: "PYTH",
    name: "Pyth Network",
    price: 0.00097,
  },
  {
    mint: "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE",
    symbol: "ORCA",
    name: "Orca",
    price: 0.0128,
  },
  {
    mint: "MEW1gQWJ3nEXg2qgERiKu7FAFj79PHvQVREQUzScPP5",
    symbol: "MEW",
    name: "cat in a dogs world",
    price: 0.0000215,
  },
  {
    mint: "HeLp6NuQkmYB4pYWo2zYs22mESHXPQYzXbB8n4V98jwC",
    symbol: "AI16Z",
    name: "ai16z",
    price: 0.0019,
  },
  {
    mint: "2qEHjDLDLbuBgRYvsxhc5D6uDWAivNFZGan56P1tpump",
    symbol: "PNUT",
    name: "Peanut the Squirrel",
    price: 0.0016,
  },
];
const FAILED_BUY = {
  mint: "Df6yfrKC8kZE3KNkrHERKzAetSxbrWeniQfyJY4Jpump",
  symbol: "CHILLGUY",
  name: "Just a chill guy",
};

const template = fixture("positions_open.json")[0];

function positionRow({ id, mint, symbol, name, price, size, enteredSec }) {
  const tokens = Math.round((size / price) * 10 ** 6);
  return {
    ...template,
    id,
    mint,
    symbol,
    name,
    entry_price: price,
    effective_entry_price: price,
    average_entry_price: price,
    entry_time: enteredSec,
    entry_size_native: size,
    total_size_native: size,
    price_highest: price,
    price_lowest: price,
    current_price: price,
    current_price_updated: nowSec(),
    token_amount: tokens,
    remaining_token_amount: tokens,
    token_decimals: 6,
    dca_count: 0,
    unrealized_pnl: 0,
    unrealized_pnl_percent: 0,
    entry_transaction_signature: `lab-entry-${id}`,
  };
}

const market = {
  open: [],
  closed: fixture("positions_closed.json"),
  actions: new Map(),
  nextId: 300,
  nextAction: 1,
  candidate: 0,
  paused: false,
  /** Until this time (ms) the stream refuses connections, as a lost connection would. */
  droppedUntil: 0,
};

// Six open positions to start with: the three recorded ones and three older ones.
market.open = [
  ...fixture("positions_open.json"),
  positionRow({ id: 90, ...CANDIDATES[5], size: 0.01, enteredSec: 1791200000 }),
  positionRow({ id: 91, ...CANDIDATES[6], size: 0.02, enteredSec: 1791150000 }),
  positionRow({ id: 92, ...CANDIDATES[7], size: 0.01, enteredSec: 1791100000 }),
];

// ---- action stream -------------------------------------------------------------------

const streams = new Set();

function broadcast(updateType, action) {
  const payload = JSON.stringify({
    action_id: action.id,
    update_type: updateType,
    timestamp: iso(),
    data: {},
    action,
  });
  for (const response of streams) response.write(`data: ${payload}\n\n`);
}

function inProgressState(action) {
  const index = action.current_step_index;
  return {
    status: "in_progress",
    current_step: action.steps[index].name,
    current_step_index: index,
    total_steps: action.steps.length,
    progress_pct: Math.round((index / action.steps.length) * 100),
  };
}

function startAction(actionType, token, metadata = {}) {
  const codes =
    actionType === "position_open"
      ? ["validate", "quote", "swap", "verify"]
      : ["quote", "swap", "verify"];
  const action = {
    id: `lab-action-${market.nextAction++}`,
    action_type: actionType,
    entity_id: token.mint,
    state: null,
    steps: codes.map((name, index) => ({
      step_id: `step-${index}`,
      name,
      status: index === 0 ? "in_progress" : "pending",
      started_at: index === 0 ? iso() : null,
      completed_at: null,
      error: null,
      metadata: {},
    })),
    current_step_index: 0,
    started_at: iso(),
    completed_at: null,
    read: false,
    dismissed: false,
    metadata: { mint: token.mint, symbol: token.symbol, operation: "auto", ...metadata },
  };
  action.state = inProgressState(action);
  market.actions.set(action.id, action);
  broadcast("action_started", action);
  return action;
}

function advanceAction(action) {
  const step = action.steps[action.current_step_index];
  step.status = "completed";
  step.completed_at = iso();
  action.current_step_index += 1;
  const next = action.steps[action.current_step_index];
  next.status = "in_progress";
  next.started_at = iso();
  action.state = inProgressState(action);
  broadcast("step_completed", action);
}

function completeAction(action) {
  for (const step of action.steps) {
    if (step.status !== "completed") {
      step.status = "completed";
      step.started_at ??= iso();
      step.completed_at = iso();
    }
  }
  action.current_step_index = action.steps.length - 1;
  action.state = { status: "completed" };
  action.completed_at = iso();
  broadcast("action_completed", action);
}

function failAction(action, details) {
  const step = action.steps[action.current_step_index];
  const error = { text: { id: "swap-error-generic" }, details };
  step.status = "failed";
  step.completed_at = iso();
  step.error = error;
  action.state = { status: "failed", error };
  action.completed_at = iso();
  broadcast("action_failed", action);
}

// ---- scripted market -----------------------------------------------------------------

const byNewest = () => [...market.open].sort((a, b) => b.entry_time - a.entry_time);

function openPosition(token, size) {
  const row = positionRow({ id: market.nextId++, ...token, size, enteredSec: nowSec() });
  market.open.push(row);
  return row;
}

function markClosing(row) {
  row.exit_transaction_signature = `lab-exit-${row.id}`;
  row.transaction_exit_verified = false;
}

function closePosition(row) {
  market.open = market.open.filter((open) => open.id !== row.id);
  const received = row.total_size_native * (1 + (row.unrealized_pnl_percent || 0) / 100);
  market.closed.unshift({
    ...row,
    status: "closed",
    exit_time: nowSec(),
    exit_price: row.current_price,
    effective_exit_price: row.current_price,
    transaction_exit_verified: true,
    sol_received: received,
    pnl: received - row.total_size_native,
    pnl_percent: row.unrealized_pnl_percent,
    closed_reason: "take_profit",
    unrealized_pnl: null,
    unrealized_pnl_percent: null,
  });
}

function nextCandidate() {
  const held = new Set(market.open.map((row) => row.mint));
  for (let tries = 0; tries < CANDIDATES.length; tries += 1) {
    const token = CANDIDATES[market.candidate++ % CANDIDATES.length];
    if (!held.has(token.mint)) return token;
  }
  return CANDIDATES[0];
}

/** Steps of one buy: four steps about a second apart, then the row lands. */
function scriptBuy(at, steps, size = 0.01) {
  let token;
  let action;
  steps.push([
    at,
    () => {
      token = nextCandidate();
      action = startAction("position_open", token, { size_sol: size });
    },
  ]);
  for (let index = 1; index <= 3; index += 1) {
    steps.push([at + index * 1100, () => advanceAction(action)]);
  }
  steps.push([at + 4400, () => completeAction(action)]);
  steps.push([at + 5300, () => openPosition(token, size)]);
}

/** Steps of one full sale of the row `pick` chooses from the newest-first list. */
function scriptSale(at, steps, pick) {
  let row;
  let action;
  steps.push([
    at,
    () => {
      row = pick(byNewest());
      if (row) action = startAction("position_close", row);
    },
  ]);
  for (let index = 1; index <= 2; index += 1) {
    steps.push([at + index * 1100, () => action && advanceAction(action)]);
  }
  steps.push([
    at + 3300,
    () => {
      if (!action) return;
      completeAction(action);
      markClosing(row);
    },
  ]);
  steps.push([at + 5200, () => row && closePosition(row)]);
}

function scriptCycle() {
  const steps = [];
  scriptBuy(1000, steps);
  scriptSale(9000, steps, (rows) => rows[Math.min(3, rows.length - 1)]);

  // An add to the newest position: its size grows when the add is verified.
  let added;
  let addAction;
  steps.push([
    17000,
    () => {
      added = byNewest()[0];
      if (added) addAction = startAction("position_dca", added, { size_sol: 0.005 });
    },
  ]);
  steps.push([18100, () => addAction && advanceAction(addAction)]);
  steps.push([19200, () => addAction && advanceAction(addAction)]);
  steps.push([20300, () => addAction && completeAction(addAction)]);
  steps.push([
    21500,
    () => {
      if (!added) return;
      added.total_size_native += 0.005;
      added.dca_count += 1;
    },
  ]);

  // A buy that fails at the swap and lingers as a failed row.
  let failed;
  steps.push([
    24000,
    () => {
      failed = startAction("position_open", FAILED_BUY, { size_sol: 0.01 });
    },
  ]);
  steps.push([25100, () => advanceAction(failed)]);
  steps.push([26200, () => advanceAction(failed)]);
  steps.push([27300, () => failAction(failed, "Slippage tolerance exceeded")]);

  // A buy landing while the oldest row closes.
  scriptBuy(36000, steps);
  scriptSale(38500, steps, (rows) => rows[rows.length - 1]);
  return { steps, length: 50000 };
}

let timers = [];
function runCycle() {
  const { steps, length } = scriptCycle();
  timers = steps.map(([at, step]) =>
    setTimeout(() => {
      try {
        step();
      } catch (error) {
        console.error("[lab] step failed:", error);
      }
    }, at)
  );
  timers.push(setTimeout(() => !market.paused && runCycle(), length));
}

// Pool prices tick every second.
setInterval(() => {
  for (const row of market.open) {
    const drift = 1 + (Math.random() - 0.48) * 0.006;
    row.current_price *= drift;
    row.current_price_updated = nowSec();
    row.price_highest = Math.max(row.price_highest, row.current_price);
    row.price_lowest = Math.min(row.price_lowest, row.current_price);
    const basis = row.average_entry_price || row.entry_price;
    row.unrealized_pnl_percent = (row.current_price / basis - 1) * 100;
    row.unrealized_pnl = (row.total_size_native * row.unrealized_pnl_percent) / 100;
  }
}, 1000);

// ---- servers ---------------------------------------------------------------------------

const indexes = [await loadIndex("positions"), await loadIndex("shell")];
const api = createApiHandler(indexes, "populated");
const inner = await serveDashboard(
  { route: async () => {} },
  { locale: LOCALE, onApi: api.onApi, startupPage: "positions" }
);
const innerUrl = new URL(inner.origin);

const json = (response, body) => {
  response.writeHead(200, { "content-type": "application/json", "cache-control": "no-store" });
  response.end(JSON.stringify(body));
};

const outer = createServer((request, response) => {
  const url = new URL(request.url, "http://localhost");
  const { pathname } = url;

  if (pathname === "/lab/pause") {
    market.paused = true;
    timers.forEach(clearTimeout);
    // The steps that would finish them are gone, so actions in flight fail now instead of
    // leaving their rows mid-step.
    for (const action of market.actions.values()) {
      if (action.state.status === "in_progress") failAction(action, "Lab paused");
    }
    return json(response, { paused: true });
  }
  if (pathname === "/lab/resume") {
    if (market.paused) {
      market.paused = false;
      runCycle();
    }
    return json(response, { paused: false });
  }
  if (pathname === "/lab/state") {
    return json(response, {
      paused: market.paused,
      open: byNewest().map((row) => `${row.id} ${row.symbol}`),
      actions: [...market.actions.values()].map(
        (a) => `${a.id} ${a.action_type} ${a.state.status}`
      ),
    });
  }
  if (pathname === "/lab/drop") {
    const ms = Math.min(Number(url.searchParams.get("ms")) || 6000, 60000);
    market.droppedUntil = Date.now() + ms;
    for (const stream of streams) stream.destroy();
    streams.clear();
    return json(response, { droppedForMs: ms });
  }
  if (pathname === "/api/actions/stream") {
    if (Date.now() < market.droppedUntil) {
      response.writeHead(503);
      response.end();
      return undefined;
    }
    response.writeHead(200, {
      "content-type": "text/event-stream",
      "cache-control": "no-store",
      connection: "keep-alive",
    });
    response.write(": connected\n\n");
    streams.add(response);
    request.on("close", () => streams.delete(response));
    return undefined;
  }
  if (
    request.method === "POST" &&
    /^\/api\/actions\/(read-all|dismiss-all|[^/]+\/(dismiss|read))$/.test(pathname)
  ) {
    return json(response, { success: true, unread: 0 });
  }
  if (pathname === "/api/actions/active") {
    const active = [...market.actions.values()].filter((a) => a.state.status === "in_progress");
    return json(response, {
      actions: active,
      count: active.length,
      in_progress: active.length,
      completed: 0,
      failed: 0,
      unread: 0,
    });
  }
  const actionPath = /^\/api\/actions\/([^/]+)$/.exec(pathname);
  if (
    actionPath &&
    request.method === "GET" &&
    !["all", "history", "subscribers"].includes(actionPath[1])
  ) {
    const action = market.actions.get(decodeURIComponent(actionPath[1]));
    if (!action) {
      response.writeHead(404, { "content-type": "application/json" });
      response.end(JSON.stringify({ success: false }));
      return undefined;
    }
    return json(response, { success: true, action });
  }
  if (pathname === "/api/positions" && request.method === "GET") {
    const status = url.searchParams.get("status");
    if (status === "open") return json(response, market.open);
    if (status === "closed") return json(response, market.closed);
  }

  const forwarded = httpRequest(
    {
      hostname: innerUrl.hostname,
      port: innerUrl.port,
      path: request.url,
      method: request.method,
      headers: { ...request.headers, host: innerUrl.host },
    },
    (answer) => {
      response.writeHead(answer.statusCode ?? 502, answer.headers);
      answer.pipe(response);
    }
  );
  forwarded.on("error", () => {
    response.writeHead(502);
    response.end();
  });
  request.pipe(forwarded);
  return undefined;
});

await new Promise((done) => outer.listen(PORT, "127.0.0.1", done));
runCycle();
console.log(`positions motion lab: http://127.0.0.1:${PORT}/positions (Ctrl-C to stop)`);

const stop = () => {
  outer.close();
  for (const response of streams) response.end();
  inner.close().then(() => process.exit(0));
};
process.on("SIGINT", stop);
process.on("SIGTERM", stop);
