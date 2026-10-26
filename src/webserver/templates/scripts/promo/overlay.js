// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Promo capture overlay layer.
 *
 * The annotation vocabulary a promotional screenshot or video needs: a pointed
 * callout anchored to real UI, a spotlight that dims everything else, a synthetic
 * cursor that travels and clicks, a lower-third caption, and a full-screen title
 * card. Everything is built from the dashboard's own surface, type and spacing
 * tokens so an annotated frame still looks like the product.
 *
 * Every function resolves on the Web Animations API's `finished` promise, so the
 * driver knows an annotation is fully on screen — it never waits out a duration
 * it guessed. That promise only settles while the renderer produces frames, so
 * every wait is also bounded by the animation's own end time plus a grace period
 * and rejects with the reason when the page has stopped painting.
 */

const LAYER_ID = "promoCaptureLayer";
const CURSOR_ID = "promoCaptureCursor";
const SVG_NS = "http://www.w3.org/2000/svg";

/**
 * The cursor's macOS-style arrow in CSS px, drawn with its tip at (0, 0). The
 * tip is the hotspot: the cursor element's origin is the aimed point, so every
 * move, hover and press lands the tip exactly there.
 */
const CURSOR_ARROW = {
  path: "M0 0V17L4.2 13.1L7 19.6L9.9 18.4L7.2 12H12.6Z",
  width: 13,
  height: 20,
};

/**
 * How long past its scheduled end an animation may run before the wait fails.
 * Generous against a busy main thread; a page that is not painting at all never
 * catches up, whatever the margin.
 */
const ANIMATION_GRACE_MS = 4000;

/** Cursor travel time in ms: a base plus a per-pixel share, clamped to a natural range. */
const CURSOR_TRAVEL = { baseMs: 380, perPxMs: 0.45, minMs: 420, maxMs: 800 };

/** Sideways bow of the travel path as a share of its length, capped in CSS px. */
const CURSOR_ARC = { share: 0.1, maxPx: 48 };

/** Keyframes sampled along the curved travel path. */
const CURSOR_PATH_STEPS = 16;

const registry = new Map();
let sequence = 0;

function motionScale() {
  const raw = getComputedStyle(document.documentElement).getPropertyValue("--promo-motion-scale");
  const value = Number.parseFloat(raw);
  return Number.isFinite(value) && value >= 0 ? value : 1;
}

function layer() {
  let el = document.getElementById(LAYER_ID);
  if (!el) {
    el = document.createElement("div");
    el.id = LAYER_ID;
    el.className = "promo-layer";
    el.setAttribute("aria-hidden", "true");
    document.body.appendChild(el);
  }
  return el;
}

function nextId(prefix) {
  sequence += 1;
  return `${prefix}-${sequence}`;
}

/**
 * Why the renderer is not delivering frames. A hidden document (the window is
 * minimised, fully covered or on another Space) gets no animation frames, so
 * neither `requestAnimationFrame` nor the animation timeline advances.
 */
export function renderStallReason() {
  return document.visibilityState === "hidden"
    ? "the page is hidden (window minimised, covered or on another Space), so the renderer is not painting"
    : "the renderer delivered no animation frames in time";
}

/**
 * Await an animation, tolerating a cancel so a cleared annotation never hangs,
 * and rejecting with `what` and the stall reason once the animation is overdue.
 */
async function settled(animation, what) {
  const endTime = animation.effect?.getComputedTiming().endTime;
  const budget = Math.round((Number.isFinite(endTime) ? endTime : 0) + ANIMATION_GRACE_MS);
  let timer = null;
  const overdue = new Promise((_, reject) => {
    timer = setTimeout(
      () => reject(new Error(`${what} did not finish within ${budget}ms: ${renderStallReason()}`)),
      budget
    );
  });
  try {
    // A cancel (clear() removing the annotation) rejects `finished`; the caller
    // only needs to stop waiting, so it counts as settled.
    await Promise.race([animation.finished.catch(() => null), overdue]);
  } finally {
    clearTimeout(timer);
  }
}

function rectOf(target) {
  if (target && typeof target === "object" && "width" in target) return target;
  const el = document.querySelector(target);
  if (!el) throw new Error(`Overlay target not found: ${target}`);
  const rect = el.getBoundingClientRect();
  return { top: rect.top, left: rect.left, width: rect.width, height: rect.height };
}

/**
 * A viewport point inside an element: `anchor` gives the fractions of its width
 * and height (the centre by default), so a scene aims at real geometry rather
 * than at pixels measured on one display.
 */
export function pointIn(selector, anchor = null) {
  const rect = rectOf(selector);
  const ax = Number.isFinite(anchor?.x) ? anchor.x : 0.5;
  const ay = Number.isFinite(anchor?.y) ? anchor.y : 0.5;
  return { x: Math.round(rect.left + rect.width * ax), y: Math.round(rect.top + rect.height * ay) };
}

function enter(el, { from = "translateY(6px)" } = {}) {
  return el.animate(
    [
      { opacity: 0, transform: from },
      { opacity: 1, transform: "translateY(0)" },
    ],
    { duration: 260 * motionScale(), easing: "cubic-bezier(0.22, 1, 0.36, 1)", fill: "both" }
  );
}

// ---------------------------------------------------------------------------
// Callout
// ---------------------------------------------------------------------------

/**
 * Place the panel beside its target, flipping to the opposite side when the
 * preferred one would leave the viewport and clamping along the free axis. The
 * pointer is then aimed at the target's centre independently of that clamping,
 * so a callout pushed sideways still points at the thing it describes.
 */
function place(panel, rect, placement, gap = 14) {
  const panelRect = panel.getBoundingClientRect();
  const margin = 16;
  const vw = window.innerWidth;
  const vh = window.innerHeight;

  const room = {
    top: rect.top,
    bottom: vh - (rect.top + rect.height),
    left: rect.left,
    right: vw - (rect.left + rect.width),
  };
  const needed = {
    top: panelRect.height + gap + margin,
    bottom: panelRect.height + gap + margin,
    left: panelRect.width + gap + margin,
    right: panelRect.width + gap + margin,
  };

  let side = placement;
  if (room[side] < needed[side]) {
    const opposite = { top: "bottom", bottom: "top", left: "right", right: "left" }[side];
    if (room[opposite] >= needed[opposite]) side = opposite;
    else side = Object.keys(room).sort((a, b) => room[b] - needed[b] - (room[a] - needed[a]))[0];
  }

  let top;
  let left;
  if (side === "top" || side === "bottom") {
    top = side === "top" ? rect.top - panelRect.height - gap : rect.top + rect.height + gap;
    left = rect.left + rect.width / 2 - panelRect.width / 2;
    left = Math.min(Math.max(margin, left), vw - panelRect.width - margin);
  } else {
    left = side === "left" ? rect.left - panelRect.width - gap : rect.left + rect.width + gap;
    top = rect.top + rect.height / 2 - panelRect.height / 2;
    top = Math.min(Math.max(margin, top), vh - panelRect.height - margin);
  }

  panel.style.top = `${Math.round(top)}px`;
  panel.style.left = `${Math.round(left)}px`;
  panel.dataset.side = side;

  const pointer = panel.querySelector(".promo-callout-pointer");
  if (pointer) {
    if (side === "top" || side === "bottom") {
      const x = rect.left + rect.width / 2 - left;
      pointer.style.left = `${Math.min(Math.max(18, x), panelRect.width - 18)}px`;
      pointer.style.top = "";
    } else {
      const y = rect.top + rect.height / 2 - top;
      pointer.style.top = `${Math.min(Math.max(18, y), panelRect.height - 18)}px`;
      pointer.style.left = "";
    }
  }
}

/**
 * A pointed annotation panel anchored to a live element.
 *
 * @param {string|object} target  selector, or a viewport rect
 * @param {string} [eyebrow]      small label above the title (e.g. "Step 2")
 * @param {string} title
 * @param {string} [body]
 * @param {"top"|"bottom"|"left"|"right"} [placement]
 * @param {string} [tone]         "neutral" | "buy" | "sell"
 * @param {boolean} [follow]      keep the panel pinned while the target moves
 */
export async function callout({
  target,
  selector,
  eyebrow = "",
  title = "",
  body = "",
  placement = "bottom",
  tone = "neutral",
  follow = false,
  id = null,
} = {}) {
  const anchor = target || selector;
  const rect = rectOf(anchor);
  const panelId = id || nextId("callout");

  clear({ id: panelId });

  const panel = document.createElement("div");
  panel.className = "promo-callout";
  panel.dataset.tone = tone;
  panel.id = panelId;
  panel.innerHTML = `
    <span class="promo-callout-pointer" aria-hidden="true"></span>
    ${eyebrow ? '<p class="promo-callout-eyebrow"></p>' : ""}
    ${title ? '<p class="promo-callout-title"></p>' : ""}
    ${body ? '<p class="promo-callout-body"></p>' : ""}
  `;
  // Text is assigned rather than interpolated: scene copy is authored freely and
  // must never be able to inject markup into the captured frame.
  if (eyebrow) panel.querySelector(".promo-callout-eyebrow").textContent = eyebrow;
  if (title) panel.querySelector(".promo-callout-title").textContent = title;
  if (body) panel.querySelector(".promo-callout-body").textContent = body;

  layer().appendChild(panel);
  place(panel, rect, placement);

  const entry = { el: panel, kind: "callout", raf: null };
  registry.set(panelId, entry);

  if (follow && typeof anchor === "string") {
    const track = () => {
      if (!registry.has(panelId)) return;
      try {
        place(panel, rectOf(anchor), placement);
      } catch {
        /* target left the DOM; the panel stays where it was until cleared */
      }
      entry.raf = requestAnimationFrame(track);
    };
    entry.raf = requestAnimationFrame(track);
  }

  await settled(enter(panel), "callout entrance");
  return { id: panelId, side: panel.dataset.side };
}

// ---------------------------------------------------------------------------
// Spotlight
// ---------------------------------------------------------------------------

/**
 * Dim the whole window except one rectangle. Built from a single element with a
 * huge spread box-shadow, so there is no second full-screen paint to keep in sync
 * with the cut-out and no SVG mask to scale wrong on a HiDPI capture.
 */
export async function spotlight({
  target,
  selector,
  padding = 8,
  radius = 10,
  id = "promo-spotlight",
} = {}) {
  const rect = rectOf(target || selector);
  clear({ id });

  const hole = document.createElement("div");
  hole.className = "promo-spotlight";
  hole.id = id;
  hole.style.top = `${rect.top - padding}px`;
  hole.style.left = `${rect.left - padding}px`;
  hole.style.width = `${rect.width + padding * 2}px`;
  hole.style.height = `${rect.height + padding * 2}px`;
  hole.style.borderRadius = `${radius}px`;

  layer().appendChild(hole);
  registry.set(id, { el: hole, kind: "spotlight" });

  await settled(
    hole.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: 240 * motionScale(),
      easing: "ease-out",
      fill: "both",
    }),
    "spotlight fade-in"
  );
  return { id };
}

// ---------------------------------------------------------------------------
// Caption and title card
// ---------------------------------------------------------------------------

/** A lower-third (or upper-third) narration line for a recording. */
export async function caption({
  text = "",
  sub = "",
  position = "bottom",
  id = "promo-caption",
} = {}) {
  clear({ id });

  const el = document.createElement("div");
  el.className = "promo-caption";
  el.id = id;
  el.dataset.position = position;
  el.innerHTML = `<p class="promo-caption-text"></p>${sub ? '<p class="promo-caption-sub"></p>' : ""}`;
  el.querySelector(".promo-caption-text").textContent = text;
  if (sub) el.querySelector(".promo-caption-sub").textContent = sub;

  layer().appendChild(el);
  registry.set(id, { el, kind: "caption" });

  await settled(
    enter(el, { from: position === "bottom" ? "translateY(12px)" : "translateY(-12px)" }),
    "caption entrance"
  );
  return { id };
}

/** A full-screen card for the open and close of a promotional clip. */
export async function titleCard({ title = "", sub = "", id = "promo-title-card" } = {}) {
  clear({ id });

  const el = document.createElement("div");
  el.className = "promo-title-card";
  el.id = id;
  el.innerHTML = `<p class="promo-title-card-title"></p>${
    sub ? '<p class="promo-title-card-sub"></p>' : ""
  }`;
  el.querySelector(".promo-title-card-title").textContent = title;
  if (sub) el.querySelector(".promo-title-card-sub").textContent = sub;

  layer().appendChild(el);
  registry.set(id, { el, kind: "titleCard" });

  await settled(
    el.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: 420 * motionScale(),
      easing: "ease-out",
      fill: "both",
    }),
    "title card fade-in"
  );
  return { id };
}

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

let cursorPosition = { x: -100, y: -100 };

/** The arrow as inline SVG; the ring is a sibling drawn beneath it, centred on the tip. */
function cursorArrow() {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("class", "promo-cursor-arrow");
  svg.setAttribute("width", String(CURSOR_ARROW.width));
  svg.setAttribute("height", String(CURSOR_ARROW.height));
  svg.setAttribute("viewBox", `0 0 ${CURSOR_ARROW.width} ${CURSOR_ARROW.height}`);
  const path = document.createElementNS(SVG_NS, "path");
  path.setAttribute("d", CURSOR_ARROW.path);
  svg.appendChild(path);
  return svg;
}

function cursorEl() {
  let el = document.getElementById(CURSOR_ID);
  if (!el) {
    el = document.createElement("div");
    el.id = CURSOR_ID;
    el.className = "promo-cursor";
    const ring = document.createElement("span");
    ring.className = "promo-cursor-ring";
    el.append(ring, cursorArrow());
    el.style.translate = cursorTranslate(cursorPosition);
    layer().appendChild(el);
  }
  return el;
}

/**
 * The cursor is positioned with the `translate` property, never `transform`.
 * An element's matrix is translate · rotate · scale · transform, so a `scale`
 * multiplies whatever `transform` holds: with the position inside `transform`,
 * the press dip scaled the position too and the cursor darted toward the
 * window's top-left corner on every press. Held in `translate`, the position is
 * applied after the dip, which then scales the arrow about its
 * `transform-origin` — the tip — so the hotspot never moves during a press.
 */
function cursorTranslate({ x, y }) {
  return `${x}px ${y}px`;
}

/** Resolve a cursor destination from a selector (plus anchor) or an x/y pair. */
function cursorPoint({ selector = null, anchor = null, x = null, y = null }) {
  if (selector) return pointIn(selector, anchor);
  if (Number.isFinite(x) && Number.isFinite(y)) return { x, y };
  return null;
}

/** Natural travel time for a distance: short hops are quick, long sweeps cap out. */
export function cursorTravelMs(distance) {
  if (distance < 1) return 0;
  const { baseMs, perPxMs, minMs, maxMs } = CURSOR_TRAVEL;
  return Math.round(Math.min(maxMs, Math.max(minMs, baseMs + distance * perPxMs)));
}

/**
 * Keyframes along a gently bowed path from `from` to `to`. A hand moving a mouse
 * pivots at the wrist, so a straight ruler line reads as synthetic; a quadratic
 * curve with a small sideways bow reads as a person.
 */
function travelKeyframes(from, to) {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy);
  const bow = Math.min(length * CURSOR_ARC.share, CURSOR_ARC.maxPx);
  const control = {
    x: (from.x + to.x) / 2 + (dy / length) * bow,
    y: (from.y + to.y) / 2 - (dx / length) * bow,
  };
  return Array.from({ length: CURSOR_PATH_STEPS + 1 }, (_, step) => {
    const t = step / CURSOR_PATH_STEPS;
    const u = 1 - t;
    return {
      translate: cursorTranslate({
        x: u * u * from.x + 2 * u * t * control.x + t * t * to.x,
        y: u * u * from.y + 2 * u * t * control.y + t * t * to.y,
      }),
    };
  });
}

/**
 * Show the cursor. A hidden cursor is placed at the point (an element via
 * `selector`/`anchor`, or x/y) before it fades in, so it never appears in one
 * place and then jumps; a cursor that is already visible travels there instead.
 *
 * Animations here never use a forward fill: the end state is written to the
 * element's own style first, so nothing left behind overrides a later placement.
 */
export async function cursorShow({ selector = null, anchor = null, x = null, y = null } = {}) {
  const el = cursorEl();
  const point = cursorPoint({ selector, anchor, x, y });

  if (el.classList.contains("is-visible")) {
    if (point) await cursorTo({ x: point.x, y: point.y });
    return { ...cursorPosition };
  }

  if (point) {
    cursorPosition = point;
    el.style.translate = cursorTranslate(point);
  }
  el.classList.add("is-visible");
  await settled(
    el.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: 180 * motionScale(),
      easing: "ease-out",
      fill: "backwards",
    }),
    "cursor fade-in"
  );
  return { ...cursorPosition };
}

export async function cursorHide() {
  const el = document.getElementById(CURSOR_ID);
  if (!el || !el.classList.contains("is-visible")) return { hidden: true };
  const fade = el.animate([{ opacity: 1 }, { opacity: 0 }], {
    duration: 180 * motionScale(),
    easing: "ease-out",
    fill: "forwards",
  });
  try {
    await settled(fade, "cursor fade-out");
  } finally {
    // The class and the fill are dropped together so no frame shows the cursor
    // again, and the next show starts from a clean element.
    el.classList.remove("is-visible");
    fade.cancel();
  }
  return { hidden: true };
}

/**
 * Move the promo cursor to an element (its centre, or `anchor` fractions of its
 * rect) or to an x/y point along an eased, gently curved path. `duration` in ms
 * overrides the distance-based travel time; both scale with the motion setting.
 * The travel is one Web Animation, so the driver awaits the real end of the move.
 */
export async function cursorTo({
  selector = null,
  anchor = null,
  x = null,
  y = null,
  duration = null,
} = {}) {
  const destination = cursorPoint({ selector, anchor, x, y });
  if (!destination) throw new Error("cursorTo needs a selector or an x/y pair");

  const el = cursorEl();
  if (!el.classList.contains("is-visible")) await cursorShow();
  el.getAnimations().forEach((animation) => {
    if (animation.effect?.getKeyframes().some((frame) => frame.translate)) animation.cancel();
  });

  const from = cursorPosition;
  const distance = Math.hypot(destination.x - from.x, destination.y - from.y);
  const travelMs =
    Math.max(0, Number.isFinite(duration) ? duration : cursorTravelMs(distance)) * motionScale();

  cursorPosition = destination;
  el.style.translate = cursorTranslate(destination);
  if (distance >= 1 && travelMs > 0) {
    await settled(
      el.animate(travelKeyframes(from, destination), {
        duration: travelMs,
        easing: "cubic-bezier(0.3, 0, 0.2, 1)",
        fill: "backwards",
      }),
      "cursor travel"
    );
  }
  return { ...destination, duration: Math.round(travelMs) };
}

/**
 * The press: the arrow shrinks slightly toward its tip and a ring expands from
 * the tip, like a real tap. Only the arrow scales, about its tip, so the hotspot
 * stays on the target throughout. It resolves when the dip has released, which
 * is when a real click lands, so the caller dispatches the click while the ring
 * is still expanding on its own. The ring holds most of its opacity until it is
 * well out, so it still reads after video compression on any surface.
 */
export async function cursorPress() {
  const el = cursorEl();
  const ring = el.querySelector(".promo-cursor-ring");
  const arrow = el.querySelector(".promo-cursor-arrow");
  const duration = 420 * motionScale();

  const ripple = ring.animate(
    [
      { transform: "scale(0.2)", opacity: 0.85 },
      { transform: "scale(0.8)", opacity: 0.5, offset: 0.6 },
      { transform: "scale(1)", opacity: 0 },
    ],
    { duration, easing: "cubic-bezier(0.22, 1, 0.36, 1)" }
  );
  const dip = arrow.animate([{ scale: "1" }, { scale: "0.86" }, { scale: "1" }], {
    duration: duration * 0.45,
    easing: "ease-out",
  });

  // The ring is not awaited: it is the shot's subject, not a completion signal,
  // and a cancel by a later command must not fail this one.
  ripple.finished.catch(() => null);
  await settled(dip, "cursor press");
  return { pressed: true, ...cursorPosition };
}

// ---------------------------------------------------------------------------
// Teardown
// ---------------------------------------------------------------------------

/** Remove one annotation, or every annotation when called without an id. */
export function clear({ id = null } = {}) {
  const remove = (key, entry) => {
    if (entry.raf) cancelAnimationFrame(entry.raf);
    entry.el.getAnimations().forEach((animation) => animation.cancel());
    entry.el.remove();
    registry.delete(key);
  };

  if (id) {
    const entry = registry.get(id);
    if (entry) remove(id, entry);
    return { cleared: id };
  }

  Array.from(registry.entries()).forEach(([key, entry]) => remove(key, entry));
  return { cleared: "all" };
}
