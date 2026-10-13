// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Deterministic scroll framing for promotional stills.

import { dirSign } from "../core/dom.js";

const TAB_ROWS = ".tab-scroll-area, [role='tablist'], [data-dialog-tab-list], .copy-steps";
const SELECTED_TAB =
  ".tab.active, .nav-more-toggle.active, .sub-tab.active, [role='tab'][aria-selected='true'], [aria-current='step']";
const EXCLUDED = "input, textarea, select, [contenteditable], .promo-layer";
const TOLERANCE = 1;

function visible(element) {
  return (
    !element.closest("[hidden], [aria-hidden='true'], [inert]") &&
    element.checkVisibility({ opacityProperty: true, visibilityProperty: true })
  );
}

function scrollers() {
  return [...new Set([document.scrollingElement, ...document.body.querySelectorAll("*")])].filter(
    (element) => {
      if (!element || element.closest(EXCLUDED) || !visible(element)) return false;
      if (element === document.scrollingElement || element.matches(TAB_ROWS)) return true;
      const style = getComputedStyle(element);
      return (
        element.scrollTop !== 0 ||
        element.scrollLeft !== 0 ||
        (/(auto|scroll)/.test(style.overflowY) && element.scrollHeight > element.clientHeight) ||
        (/(auto|scroll)/.test(style.overflowX) && element.scrollWidth > element.clientWidth)
      );
    }
  );
}

function selectorOf(element) {
  if (element.id) return `#${CSS.escape(element.id)}`;
  if (!element.parentElement) return element.tagName.toLowerCase();
  const index = [...element.parentElement.children].indexOf(element) + 1;
  return `${selectorOf(element.parentElement)} > ${element.tagName.toLowerCase()}:nth-child(${index})`;
}

function requireVisible(selector) {
  const element = document.querySelector(selector);
  if (!element || !visible(element)) throw new Error(`Framing target is not visible: ${selector}`);
  return element;
}

function bounds(element) {
  const rect = element.getBoundingClientRect();
  return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
}

function clamp(value, max) {
  return Math.max(0, Math.min(value, Math.max(0, max)));
}

function selectedTab(element) {
  if (!element.matches(TAB_ROWS)) return null;
  return [...element.querySelectorAll(SELECTED_TAB)].find(visible) || null;
}

function tabLeft(element, tab) {
  if (!tab) return 0;
  const style = getComputedStyle(element);
  const rtl = dirSign() < 0;
  const row = element.getBoundingClientRect();
  const box = tab.getBoundingClientRect();
  const start = row.left + element.clientLeft;
  const end = start + element.clientWidth;
  const padding = parseFloat(rtl ? style.scrollPaddingLeft : style.scrollPaddingRight) || 0;
  const tabEnd = rtl ? end - box.left : box.right - start;
  const distance = clamp(
    tabEnd - element.clientWidth + padding,
    element.scrollWidth - element.clientWidth
  );
  return rtl ? -distance : distance;
}

function axisTarget(element, value, axis) {
  const vertical = axis === "top";
  const max = vertical
    ? element.scrollHeight - element.clientHeight
    : element.scrollWidth - element.clientWidth;
  const sign = vertical ? 1 : dirSign();
  if (value === undefined || value === "start" || value === "top") return 0;
  if (value === "end" || value === "bottom") return sign * Math.max(0, max);
  if (typeof value === "number" && Number.isFinite(value)) return sign * clamp(value, max);
  if (vertical && typeof value === "string") {
    const anchor = requireVisible(value);
    if (!element.contains(anchor))
      throw new Error(`Framing anchor is outside its scroller: ${value}`);
    const padding = parseFloat(getComputedStyle(element).scrollPaddingTop) || 0;
    return clamp(
      element.scrollTop +
        anchor.getBoundingClientRect().top -
        element.getBoundingClientRect().top -
        element.clientTop -
        padding,
      max
    );
  }
  throw new Error(`Invalid framing ${axis} target: ${value}`);
}

/** Reset every structural scroller, then position tabs and explicit per-shot anchors. */
export function frameScrollers({ scrolls = [] } = {}) {
  const elements = scrollers();
  const overrides = new Map();
  for (const override of scrolls) {
    const element = requireVisible(override.selector);
    if (element.closest(EXCLUDED))
      throw new Error(`Framing cannot scroll a control: ${override.selector}`);
    if (overrides.has(element)) throw new Error(`Duplicate framing scroller: ${override.selector}`);
    overrides.set(element, override);
    if (!elements.includes(element)) elements.push(element);
  }

  // Reset ancestors as well as panes. scrollIntoView can move overflow-hidden ancestors.
  for (const element of elements) element.scrollTo({ left: 0, top: 0, behavior: "instant" });

  return elements.map((element) => {
    const tab = selectedTab(element);
    const override = overrides.get(element);
    const left =
      override?.left !== undefined
        ? axisTarget(element, override.left, "left")
        : tabLeft(element, tab);
    const top = axisTarget(element, override?.top, "top");
    element.scrollTo({ left, top, behavior: "instant" });
    return {
      element,
      selector: selectorOf(element),
      left,
      top,
      tab,
      geometry: bounds(element),
      selectedGeometry: tab ? bounds(tab) : null,
      maxLeft: element.scrollWidth - element.clientWidth,
      maxTop: element.scrollHeight - element.clientHeight,
    };
  });
}

function sameBounds(a, b) {
  return Object.keys(a).every((key) => Math.abs(a[key] - b[key]) <= TOLERANCE);
}

function tabClipped(element, tab) {
  const style = getComputedStyle(element);
  const row = element.getBoundingClientRect();
  const box = tab.getBoundingClientRect();
  const distance = Math.abs(element.scrollLeft);
  const max = element.scrollWidth - element.clientWidth;
  const rtl = dirSign() < 0;
  const leftFade = (rtl ? distance < max - TOLERANCE : distance > TOLERANCE)
    ? parseFloat(style.scrollPaddingLeft) || 0
    : 0;
  const rightFade = (rtl ? distance > TOLERANCE : distance < max - TOLERANCE)
    ? parseFloat(style.scrollPaddingRight) || 0
    : 0;
  const left = row.left + element.clientLeft;
  return (
    box.left < left + leftFade - TOLERANCE ||
    box.right > left + element.clientWidth - rightFade + TOLERANCE
  );
}

/** The intended offsets and geometry must survive loading, focus and scroll anchoring. */
export function framingMismatch(plan) {
  const current = scrollers();
  if (current.some((element) => !plan.some((entry) => entry.element === element))) {
    return "scroll containers changed";
  }
  for (const entry of plan) {
    const { element, tab } = entry;
    if (!element.isConnected || !visible(element)) return `${entry.selector}: removed or hidden`;
    if (
      Math.abs(element.scrollLeft - entry.left) > TOLERANCE ||
      Math.abs(element.scrollTop - entry.top) > TOLERANCE
    )
      return `${entry.selector}: scroll position`;
    if (
      !sameBounds(bounds(element), entry.geometry) ||
      Math.abs(element.scrollWidth - element.clientWidth - entry.maxLeft) > TOLERANCE ||
      Math.abs(element.scrollHeight - element.clientHeight - entry.maxTop) > TOLERANCE
    ) {
      return `${entry.selector}: layout changed`;
    }
    if (selectedTab(element) !== tab || (tab && !sameBounds(bounds(tab), entry.selectedGeometry)))
      return `${entry.selector}: selected tab changed`;
    if (tab && tabClipped(element, tab)) return `${entry.selector}: selected tab clipped`;
  }
  return null;
}

/** Store observed positions in the capture manifest without retaining DOM objects. */
export function describeFraming(plan) {
  return plan.map(({ element, selector, tab }) => ({
    selector,
    left: element.scrollLeft,
    top: element.scrollTop,
    viewport: bounds(element),
    selected: tab ? { selector: selectorOf(tab), viewport: bounds(tab) } : null,
  }));
}
