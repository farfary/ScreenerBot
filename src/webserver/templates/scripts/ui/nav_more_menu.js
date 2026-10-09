// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The main navigation's More-menu layout. While `<html data-nav-overflow="menu">`, the tabs
 * that do not fit the row are hidden and listed under a More disclosure at the row's end;
 * in the default "scroll" layout every tab stays in the row and the tab scroll strip
 * (`attachTabScrollStrip`) owns the overflow.
 *
 * The More control lives inside `#navTabs` as its last child, so it is measured in the
 * same row as the tabs and the travelling indicator can sit under it while the current
 * page is one of the hidden tabs. The settings dialog rebuilds `#navTabs` wholesale; the
 * control is re-attached and the row laid out again after every rebuild.
 */

import { pushEscapeHandler } from "../core/escape_stack.js";

const MENU = "menu";
const SCROLL = "scroll";

/** The overflow layout in force: "menu" or the default "scroll". */
export function navOverflowLayout() {
  return document.documentElement.dataset.navOverflow === MENU ? MENU : SCROLL;
}

/** Switch the overflow layout; the attached menu lays the row out again. */
export function setNavOverflowLayout(layout) {
  document.documentElement.dataset.navOverflow = layout === MENU ? MENU : SCROLL;
}

function buildMoreControl() {
  const more = document.createElement("div");
  more.className = "nav-more";
  more.hidden = true;
  more.innerHTML = `
    <button type="button" class="nav-more-toggle" id="navMoreToggle"
            aria-expanded="false" aria-controls="navMoreList">
      <i class="icon-ellipsis" aria-hidden="true"></i>
      <span data-l10n-id="shell-nav-more"></span>
    </button>
    <div class="dropdown-menu nav-more-list" id="navMoreList" data-align="right"></div>`;
  I18n.localizeTree(more);
  return more;
}

/**
 * Lay the main navigation out in the chosen overflow layout and keep it laid out as the
 * row resizes, the tabs are rebuilt or the layout setting changes.
 *
 * @param {HTMLElement} navTabs `#navTabs`, the flex row of main tabs.
 */
export function attachNavMoreMenu(navTabs) {
  const scroller = navTabs.parentElement;
  const strip = scroller.closest(".tab-scroll-wrapper");
  const more = buildMoreControl();
  const toggle = more.querySelector(".nav-more-toggle");
  const list = more.querySelector(".nav-more-list");
  let releaseEscape = null;

  const rowTabs = () => [...navTabs.querySelectorAll(":scope > a.tab")];
  const items = () => [...list.querySelectorAll(".dropdown-item")];
  const isOpen = () => toggle.getAttribute("aria-expanded") === "true";

  const close = ({ restoreFocus = false } = {}) => {
    if (!isOpen()) return;
    toggle.setAttribute("aria-expanded", "false");
    list.classList.remove("open");
    releaseEscape?.();
    releaseEscape = null;
    if (restoreFocus) toggle.focus();
  };

  // The list is rebuilt from the hidden tabs on every open, so it always carries their
  // current labels and the router's current page.
  const open = ({ focus = null } = {}) => {
    if (!isOpen()) {
      list.replaceChildren(
        ...rowTabs()
          .filter((tab) => tab.hidden)
          .map((tab) => {
            const item = tab.cloneNode(true);
            item.hidden = false;
            item.className = "dropdown-item";
            item.querySelector("i")?.classList.add("icon");
            item.querySelector("span")?.classList.add("label");
            return item;
          })
      );
      list.classList.add("open");
      toggle.setAttribute("aria-expanded", "true");
      releaseEscape = pushEscapeHandler(() => close({ restoreFocus: true }));
    }
    const entries = items();
    if (focus === "first") entries[0]?.focus();
    if (focus === "last") entries.at(-1)?.focus();
  };

  const layout = () => {
    close();
    if (more.parentElement !== navTabs) navTabs.appendChild(more);
    const tabs = rowTabs();
    tabs.forEach((tab) => {
      tab.hidden = false;
    });
    more.hidden = true;
    const folding = navOverflowLayout() === MENU;
    strip?.classList.toggle("is-folded", folding);

    if (folding) {
      const style = getComputedStyle(navTabs);
      const gap = parseFloat(style.columnGap) || 0;
      const room =
        scroller.clientWidth -
        parseFloat(style.paddingInlineStart) -
        parseFloat(style.paddingInlineEnd);
      const widths = tabs.map((tab) => tab.getBoundingClientRect().width);
      const needed = widths.reduce((sum, width) => sum + width, 0) + gap * (tabs.length - 1);
      if (needed > room + 0.5) {
        more.hidden = false;
        let used = more.getBoundingClientRect().width;
        let fitting = 0;
        while (fitting < widths.length && used + gap + widths[fitting] <= room + 0.5) {
          used += gap + widths[fitting];
          fitting += 1;
        }
        tabs.slice(fitting).forEach((tab) => {
          tab.hidden = true;
        });
      }
    }
    markCurrent();
  };

  // While the current page is one of the hidden tabs, the More control is the current
  // entry of the row: it takes the active state and the indicator moves under it.
  const markCurrent = () => {
    const current = rowTabs().some((tab) => tab.hidden && tab.classList.contains("active"));
    toggle.classList.toggle("active", current);
  };

  let pending = false;
  const schedule = () => {
    if (pending) return;
    pending = true;
    requestAnimationFrame(() => {
      pending = false;
      layout();
    });
  };

  toggle.addEventListener("click", (event) => {
    event.stopPropagation();
    if (isOpen()) close();
    else open({ focus: event.detail === 0 ? "first" : null });
  });

  toggle.addEventListener("keydown", (event) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    open({ focus: event.key === "ArrowDown" ? "first" : "last" });
  });

  list.addEventListener("keydown", (event) => {
    const entries = items();
    const current = entries.indexOf(document.activeElement);
    let next = null;
    if (event.key === "ArrowDown") next = (current + 1) % entries.length;
    else if (event.key === "ArrowUp") next = (current - 1 + entries.length) % entries.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = entries.length - 1;
    if (next === null) return;
    event.preventDefault();
    entries[next]?.focus();
  });

  // A chosen page closes the list before the router's own click handler runs.
  list.addEventListener("click", () => close(), true);
  more.addEventListener("focusout", (event) => {
    if (!more.contains(event.relatedTarget)) close();
  });
  document.addEventListener("click", (event) => {
    if (isOpen() && !more.contains(event.target)) close();
  });

  // The router toggles `.active` on the tabs; the settings dialog replaces them. Changes
  // inside the More control are its own and never lay the row out again.
  new MutationObserver((records) => {
    const outside = records.filter((record) => !more.contains(record.target));
    if (outside.some((record) => record.type === "childList")) schedule();
    else if (outside.length) markCurrent();
  }).observe(navTabs, { childList: true, subtree: true, attributeFilter: ["class"] });
  new MutationObserver(schedule).observe(document.documentElement, {
    attributeFilter: ["data-nav-overflow"],
  });
  new ResizeObserver(schedule).observe(scroller);
  document.fonts?.ready.then(schedule).catch(() => {});
  layout();
}
