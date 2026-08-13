/**
 * Demo capture runtime.
 *
 * Loaded only when the binary runs with `--demo-capture`. It turns the dashboard
 * into something an external driver can operate step by step, with one hard rule:
 * every command resolves on an OBSERVED signal, never on a guessed delay. A
 * command that cannot observe its own completion rejects on its deadline so a
 * capture run fails loudly instead of silently photographing a half-painted page.
 *
 * The driver talks to the Electron demo bridge, which forwards renderer-scope
 * commands here over IPC. `window.__SB_DEMO__` is also usable directly from the
 * console for authoring a scene by hand.
 */

const assetVersion = window.__ASSET_VERSION__ || "";
const assetQuery = assetVersion ? `?v=${encodeURIComponent(assetVersion)}` : "";

const overlay = await import(`./overlay.js${assetQuery}`);
const audio = await import(`./audio.js${assetQuery}`);
const { loadPage, getCurrentPage } = await import(`../core/router.js${assetQuery}`);
const { pauseAllPollers, resumeAllPollers, listPollers } = await import(
  `../core/poller.js${assetQuery}`
);

/** Longest any single command may wait for its completion signal. */
const DEFAULT_TIMEOUT_MS = 20000;

/** Ceiling for the one settle that has to absorb app boot — see freeze(). */
const FREEZE_SETTLE_TIMEOUT_MS = 60000;

/**
 * Consecutive animation frames the DOM, the network and the animation engine
 * must all be quiet for before the page counts as settled. Frames, not
 * milliseconds: it scales with the machine instead of encoding one Mac's speed.
 */
const QUIET_FRAMES = 6;

const state = {
  frozen: false,
  motionScale: 1,
  mutations: 0,
  observer: null,
};

// ---------------------------------------------------------------------------
// Quiescence
// ---------------------------------------------------------------------------

function installActivityTracking() {
  if (state.observer) return;
  state.observer = new MutationObserver((records) => {
    state.mutations += records.length;
  });
  state.observer.observe(document.body, {
    subtree: true,
    childList: true,
    attributes: true,
    characterData: true,
  });
}

/**
 * True while the page is still moving.
 *
 * Read from `document.getAnimations()` rather than counted from
 * transitionrun/animationend pairs: the event pairs are unbalanced for an
 * animation that is cancelled mid-flight by a re-render, and an infinite one
 * never sends its end at all — a counter built on them drifts upward until
 * nothing is ever considered settled again. Two classes are deliberately not
 * page activity: the capture layer's own annotations (they ARE the shot), and
 * infinite animations, which are ambient and never resolve.
 */
function animationsBusy() {
  return document.getAnimations().some((animation) => {
    if (animation.playState !== "running") return false;
    const target = animation.effect?.target;
    if (target?.closest?.(".demo-layer")) return false;
    return animation.effect?.getTiming?.().iterations !== Infinity;
  });
}

/**
 * In-flight `fetch` calls, counted by wrapping fetch itself.
 *
 * `window.__requestManager` only knows about requests routed through it, and most
 * of the dashboard does not route through it — the featured row, the boosts
 * loader and hundreds of other call sites call `fetch` directly. Trusting the
 * manager alone therefore reports "network idle" while a panel is still loading,
 * and the capture photographs empty placeholder cards. Wrapping fetch is one
 * signal that covers every call site without touching any of them.
 *
 * The wrapper settles when the response headers arrive, not when the body has
 * been parsed and rendered; the DOM mutations that follow are what the mutation
 * check and the quiet-frame requirement are for.
 */
let inFlightFetches = 0;

function installFetchTracking() {
  if (window.__SB_DEMO_FETCH_PATCHED__) return;
  const original = window.fetch;
  window.fetch = function trackedFetch(...args) {
    inFlightFetches += 1;
    return original.apply(this, args).finally(() => {
      inFlightFetches -= 1;
    });
  };
  window.__SB_DEMO_FETCH_PATCHED__ = true;
}

/** True while any request the page has issued is still outstanding. */
function networkBusy() {
  if (inFlightFetches > 0) return true;
  const manager = window.__requestManager;
  if (!manager) return false;
  const stats = manager.getStats();
  return stats.inFlight > 0 || stats.activeCount > 0 || stats.queued > 0;
}

/**
 * True while the dashboard is still telling the user it is loading. These are the
 * dashboard's own loading affordances — the router placeholder, the `data-loading`
 * flag it sets on the content area, and the skeleton rows tables paint before
 * their first payload.
 */
function loadingVisible() {
  return Boolean(
    document.querySelector(
      ".page-loading, [data-loading='true'], .loading-spinner, .skeleton, .skeleton-row"
    )
  );
}

/**
 * How long a single image may hold up a capture before the runtime gives up on
 * it. The dashboard's token logos are third-party URLs — Arweave, IPFS gateways,
 * exchange CDNs — and a meaningful share of them never answer at all, so an
 * unbounded "wait for every image" is a wait that never ends.
 */
const IMAGE_BUDGET_MS = 4000;

/** Per-image deadline. Weak so a re-rendered table does not leak its old rows. */
const imageDeadlines = new WeakMap();
const imagesGivenUp = new WeakSet();
let imagesAbandoned = 0;

/**
 * True until every image that is meant to paint has decoded, or has used up its
 * budget. Giving up is sticky per element, so a logo that has already blown its
 * budget cannot re-block a later `waitStable` — the page renders its fallback and
 * the capture proceeds.
 *
 * Every image is examined on every pass rather than short-circuiting on the first
 * pending one: `.some()` would only start the second image's clock once the first
 * had resolved, so twenty dead logos would serialise into twenty budgets. Here
 * they all start their budget on the same pass and expire together.
 */
function imagesPending() {
  const now = performance.now();
  let pending = false;

  for (const img of document.images) {
    if (img.complete || !img.src || imagesGivenUp.has(img)) continue;

    let deadline = imageDeadlines.get(img);
    if (deadline === undefined) {
      deadline = now + IMAGE_BUDGET_MS;
      imageDeadlines.set(img, deadline);
    }

    if (now < deadline) {
      pending = true;
      continue;
    }

    imagesGivenUp.add(img);
    imagesAbandoned += 1;
  }

  return pending;
}

function nextFrame() {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

/**
 * Resolve once the page has been completely still for QUIET_FRAMES frames.
 * Rejects on timeout with the reason it was still busy, which is what makes a
 * failing scene debuggable instead of mysterious.
 */
export async function waitStable({
  timeout = DEFAULT_TIMEOUT_MS,
  quietFrames = QUIET_FRAMES,
} = {}) {
  installActivityTracking();
  await document.fonts.ready;

  const deadline = performance.now() + timeout;
  let quiet = 0;
  let lastReason = "unknown";

  while (performance.now() < deadline) {
    state.mutations = 0;
    await nextFrame();

    const reason =
      (networkBusy() && "network") ||
      (loadingVisible() && "loading-ui") ||
      (animationsBusy() && "animations") ||
      (imagesPending() && "images") ||
      (state.mutations > 0 && "dom-mutations") ||
      null;

    if (reason) {
      lastReason = reason;
      quiet = 0;
      continue;
    }

    quiet += 1;
    if (quiet >= quietFrames) {
      // Two more frames so the compositor has certainly presented the final
      // paint before a screenshot is taken of it.
      await nextFrame();
      await nextFrame();
      return { settled: true };
    }
  }

  throw new Error(`waitStable timed out after ${timeout}ms (still busy: ${lastReason})`);
}

/**
 * Resolve once the window geometry has reached the renderer and stopped moving.
 *
 * This is what `window.configure` waits on, and it is deliberately NOT full
 * quiescence: configure runs while the app is still booting, when every page's
 * pollers are live and the dashboard is fetching its first payloads, so a
 * "zero requests in flight" bar there is one the app may never clear. Settling
 * is the screenshot's bar — and by the time a shot is taken the scene has frozen
 * the dashboard, which makes that bar reachable.
 */
export async function waitViewport({ timeout = DEFAULT_TIMEOUT_MS } = {}) {
  const deadline = performance.now() + timeout;
  let last = "";
  let steady = 0;

  while (performance.now() < deadline) {
    await nextFrame();
    const current = `${window.innerWidth}x${window.innerHeight}@${window.devicePixelRatio}`;
    steady = current === last ? steady + 1 : 0;
    last = current;
    if (steady >= 3) return { viewport: current };
  }

  throw new Error(`waitViewport timed out after ${timeout}ms`);
}

/**
 * Elements that must not be cut off in a capture. The main nav is the one that
 * bites: it is a single non-wrapping row of tabs, so on a laptop display the
 * last tabs sit outside the viewport and every screenshot looks like the product
 * is broken. `#navTabs` scrolls, so the page itself never reports overflow —
 * the element has to be measured directly.
 */
const CHROME_SELECTORS = ["#navTabs"];

/**
 * Report whether any of the chrome elements is clipped, and by how much. The
 * driver uses this to choose a zoom level by measurement rather than by a
 * hardcoded guess that would be wrong on the next display.
 */
export function measureChrome() {
  const viewport = document.documentElement.clientWidth;

  const overflow = CHROME_SELECTORS.map((selector) => {
    const el = document.querySelector(selector);
    if (!el) return { selector, found: false, overflowBy: 0 };

    const rect = el.getBoundingClientRect();

    // Three ways the same element can be cut off, and the nav uses the one a
    // naive check misses: `#navTabs` is `overflow: visible` and simply wider
    // than the window (1516 in a 1344 viewport, positioned at left -140), so
    // scrollWidth EQUALS clientWidth and an overflow-based test sees nothing.
    // What actually matters is how much of it lies outside the viewport.
    const overflowBy = Math.max(
      el.scrollWidth - el.clientWidth,
      el.scrollWidth - viewport,
      Math.max(0, -rect.left) + Math.max(0, rect.right - viewport)
    );

    // Sub-pixel layout rounding can leave a harmless 1px, so require more.
    return { selector, found: true, overflowBy: Math.max(0, overflowBy) };
  });

  return {
    overflow,
    clipped: overflow.some((entry) => entry.overflowBy > 1),
  };
}

// ---------------------------------------------------------------------------
// Element helpers
// ---------------------------------------------------------------------------

function isVisible(el) {
  if (!el || !el.isConnected) return false;
  const rect = el.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return false;
  const style = getComputedStyle(el);
  return style.visibility !== "hidden" && style.display !== "none" && style.opacity !== "0";
}

/**
 * Resolve when `selector` reaches `state`. Polled on animation frames rather
 * than a timer so it cannot resolve against a layout the browser has not
 * committed yet.
 */
export async function waitFor({
  selector,
  state: target = "visible",
  timeout = DEFAULT_TIMEOUT_MS,
} = {}) {
  const deadline = performance.now() + timeout;

  while (performance.now() < deadline) {
    const el = document.querySelector(selector);
    const satisfied =
      target === "absent"
        ? !el || !isVisible(el)
        : target === "present"
          ? Boolean(el)
          : isVisible(el);
    if (satisfied) return { selector, state: target };
    await nextFrame();
  }

  throw new Error(`waitFor("${selector}", "${target}") timed out after ${timeout}ms`);
}

function requireElement(selector) {
  const el = document.querySelector(selector);
  if (!el) throw new Error(`Element not found: ${selector}`);
  return el;
}

/** Center of an element in viewport coordinates — where the demo cursor aims. */
export function centerOf(el) {
  const rect = el.getBoundingClientRect();
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

// ---------------------------------------------------------------------------
// Navigation and interaction
// ---------------------------------------------------------------------------

/** Navigate to a main dashboard page and wait until it has finished painting. */
export async function goto({ page, timeout = DEFAULT_TIMEOUT_MS } = {}) {
  if (getCurrentPage() !== page) {
    await loadPage(page);
  }
  await waitStable({ timeout });
  return { page: getCurrentPage() };
}

/**
 * Click an element the way a person would: the demo cursor travels to it, presses,
 * and only then is the real click dispatched. `moveCursor: false` clicks silently,
 * which is what screenshot scenes want.
 */
export async function click({
  selector,
  moveCursor = true,
  travelMs = 650,
  settle = true,
  timeout = DEFAULT_TIMEOUT_MS,
} = {}) {
  await waitFor({ selector, state: "visible", timeout });
  const el = requireElement(selector);

  el.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" });
  await nextFrame();

  if (moveCursor) {
    await overlay.cursorTo({ selector, duration: travelMs * state.motionScale });
    await overlay.cursorPress();
  }

  el.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
  el.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
  el.click();
  el.dispatchEvent(new PointerEvent("pointerup", { bubbles: true }));
  el.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));

  if (settle) await waitStable({ timeout });
  return { selector };
}

/** Hover an element (and move the demo cursor there) without clicking it. */
export async function hover({ selector, travelMs = 650, timeout = DEFAULT_TIMEOUT_MS } = {}) {
  await waitFor({ selector, state: "visible", timeout });
  const el = requireElement(selector);

  await overlay.cursorTo({ selector, duration: travelMs * state.motionScale });
  el.dispatchEvent(new PointerEvent("pointerover", { bubbles: true }));
  el.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  el.dispatchEvent(new MouseEvent("mouseenter", { bubbles: false }));
  await waitStable({ timeout });
  return { selector };
}

/**
 * Type into a field character by character so a recording shows the text being
 * written. Resolves when the last character has been dispatched and the page has
 * settled — the field's own listeners have therefore already run.
 */
export async function type({
  selector,
  text,
  charsPerSecond = 18,
  timeout = DEFAULT_TIMEOUT_MS,
} = {}) {
  await waitFor({ selector, state: "visible", timeout });
  const el = requireElement(selector);
  el.focus();

  const stepMs = 1000 / Math.max(1, charsPerSecond * state.motionScale);
  const setter = Object.getOwnPropertyDescriptor(
    el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype,
    "value"
  )?.set;

  for (const char of String(text)) {
    const next = `${el.value}${char}`;
    if (setter) setter.call(el, next);
    else el.value = next;
    el.dispatchEvent(new Event("input", { bubbles: true }));
    await sleepFrames(stepMs);
  }

  el.dispatchEvent(new Event("change", { bubbles: true }));
  await waitStable({ timeout });
  return { selector, text };
}

/**
 * Scroll a scroller (or the page) to a target and resolve when it has actually
 * stopped there — polled against the real scroll position, so smooth scrolling
 * is honoured without timing it.
 */
export async function scrollTo({
  selector = null,
  target = 0,
  behavior = "smooth",
  timeout = DEFAULT_TIMEOUT_MS,
} = {}) {
  const scroller = selector ? requireElement(selector) : document.scrollingElement;
  let top = target;

  if (typeof target === "string") {
    if (target === "bottom") top = scroller.scrollHeight;
    else if (target === "top") top = 0;
    else {
      const el = requireElement(target);
      const base = selector ? scroller.getBoundingClientRect().top : 0;
      top = scroller.scrollTop + el.getBoundingClientRect().top - base - 24;
    }
  }

  scroller.scrollTo({ top, behavior });

  const deadline = performance.now() + timeout;
  let last = Number.NaN;
  let stableFrames = 0;
  while (performance.now() < deadline) {
    await nextFrame();
    const current = scroller.scrollTop;
    stableFrames = current === last ? stableFrames + 1 : 0;
    last = current;
    if (stableFrames >= 3) {
      await waitStable({ timeout });
      return { top: current };
    }
  }

  throw new Error(`scrollTo timed out after ${timeout}ms`);
}

/** Frame-based sleep — the only wait in the runtime, and only for typing cadence. */
function sleepFrames(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Hold for a deliberate beat in a recording (a pause on a finished frame while
 * narration lands). Scenes must use this rather than sprinkling sleeps to "let
 * things load" — loading is what waitStable is for.
 */
export function beat({ ms = 800 } = {}) {
  return sleepFrames(ms * state.motionScale);
}

// ---------------------------------------------------------------------------
// Freeze
// ---------------------------------------------------------------------------

/**
 * Stop everything that would make two captures of the same screen differ:
 * pollers refetching, the caret blinking, transient overlays fading. The demo
 * data itself is already fixed server-side, and `--demo-freeze` pins the last
 * live value (SOL price) there.
 */
export async function freeze() {
  if (state.frozen) return { frozen: true, pollers: listPollers().length };
  pauseAllPollers();
  // The bottom status bar refreshes on its own timer rather than through the
  // poller registry, so it needs its own stop or it keeps rewriting uptime and
  // RPC counters between two shots of the same page.
  window.StatusBar?.pausePolling();
  document.documentElement.classList.add("demo-frozen");
  state.frozen = true;
  // Freeze is where a run absorbs the app's whole boot: it is the first point at
  // which requests can actually drain, because nothing is refetching any more.
  // That legitimately takes longer than a mid-scene settle, so it gets its own
  // ceiling rather than the per-command default.
  await waitStable({ timeout: FREEZE_SETTLE_TIMEOUT_MS });
  return { frozen: true, pollers: listPollers().length };
}

export async function thaw() {
  if (!state.frozen) return { frozen: false };
  resumeAllPollers();
  window.StatusBar?.resumePolling();
  document.documentElement.classList.remove("demo-frozen");
  state.frozen = false;
  return { frozen: false };
}

/**
 * Global motion scale for the whole runtime — 1 is real time, 2 is half speed for
 * a slow explanatory video, 0 collapses cursor travel for fast screenshot runs.
 */
export function setMotionScale({ scale = 1 } = {}) {
  state.motionScale = Math.max(0, Number(scale) || 0);
  document.documentElement.style.setProperty("--demo-motion-scale", String(state.motionScale));
  return { scale: state.motionScale };
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

const COMMANDS = {
  "wait.stable": waitStable,
  "wait.viewport": waitViewport,
  "demo.measureChrome": measureChrome,
  "wait.for": waitFor,
  "wait.beat": beat,
  "nav.goto": goto,
  "ui.click": click,
  "ui.hover": hover,
  "ui.type": type,
  "ui.scroll": scrollTo,
  "demo.freeze": freeze,
  "demo.thaw": thaw,
  "demo.motion": setMotionScale,
  "demo.state": () => ({
    page: getCurrentPage(),
    frozen: state.frozen,
    motionScale: state.motionScale,
    pollers: listPollers(),
    // Third-party logo URLs that never answered and were captured as fallbacks.
    imagesAbandoned,
    inFlightFetches,
    network: window.__requestManager ? window.__requestManager.getStats() : null,
    viewport: { width: window.innerWidth, height: window.innerHeight },
    devicePixelRatio: window.devicePixelRatio,
    theme: document.documentElement.getAttribute("data-theme"),
  }),
  "overlay.callout": overlay.callout,
  "overlay.spotlight": overlay.spotlight,
  "overlay.caption": overlay.caption,
  "overlay.titleCard": overlay.titleCard,
  "overlay.cursor": overlay.cursorTo,
  "overlay.cursorShow": overlay.cursorShow,
  "overlay.cursorHide": overlay.cursorHide,
  "overlay.clear": overlay.clear,
  "audio.music": audio.playMusic,
  "audio.volume": audio.setVolume,
  "audio.sfx": audio.playSfx,
  "audio.stop": audio.stopMusic,
  // Escape hatch for a scene that needs to read or poke something the command
  // vocabulary does not cover yet. Reachable only through the demo bridge, which
  // only exists under --demo-capture.
  eval: async ({ expression }) => {
    const AsyncFunction = Object.getPrototypeOf(async () => {}).constructor;
    return await new AsyncFunction(expression)();
  },
};

/** Run one driver command. Unknown names fail loudly rather than no-op. */
export async function runCommand(name, args = {}) {
  const handler = COMMANDS[name];
  if (!handler) throw new Error(`Unknown demo command: ${name}`);
  const result = await handler(args || {});
  return result === undefined ? null : result;
}

// The Electron demo bridge forwards driver commands here and reports the result
// back on the same request id. Registration is idempotent so a dashboard reload
// mid-scene reconnects the channel instead of orphaning the driver.
if (window.demoAPI && typeof window.demoAPI.onCommand === "function") {
  window.demoAPI.onCommand(async ({ id, name, args }) => {
    try {
      const result = await runCommand(name, args);
      window.demoAPI.sendResult({ id, ok: true, result });
    } catch (error) {
      window.demoAPI.sendResult({ id, ok: false, error: error?.message || String(error) });
    }
  });
}

installFetchTracking();
installActivityTracking();
setMotionScale({ scale: 1 });

window.__SB_DEMO__ = {
  runCommand,
  waitStable,
  waitViewport,
  measureChrome,
  waitFor,
  goto,
  click,
  hover,
  type,
  scrollTo,
  beat,
  freeze,
  thaw,
  setMotionScale,
  centerOf,
  overlay,
  audio,
};

// The bridge polls for this before sending the first command, so a scene never
// starts against a half-initialised runtime.
window.__SB_DEMO_READY__ = true;
document.documentElement.classList.add("demo-capture");
console.log("[DemoCapture] Runtime ready");
