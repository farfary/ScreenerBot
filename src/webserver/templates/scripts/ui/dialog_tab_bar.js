/**
 * Shared tab row for full-screen details dialogs.
 *
 * Dialogs own only their tab definitions, content loaders, and optional trailing
 * actions. This module owns markup, switching, keyboard navigation, and ARIA.
 */

import { playTabSwitch } from "../core/sounds.js";

function escapeTabValue(value) {
  if (value === null || value === undefined) return "";
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

export function renderDialogTabRow({ tabs, activeTab, idPrefix, ariaLabel, actionsHtml = "" }) {
  const safePrefix = escapeTabValue(idPrefix);
  const buttons = tabs
    .map((tab) => {
      const safeId = escapeTabValue(tab.id);
      const isActive = tab.id === activeTab;
      const icon = tab.icon ? `<i class="${escapeTabValue(tab.icon)}" aria-hidden="true"></i>` : "";
      const badge =
        tab.badge !== undefined
          ? `<span class="details-tab-badge"${
              tab.badgeId ? ` id="${escapeTabValue(tab.badgeId)}"` : ""
            }>${escapeTabValue(tab.badge)}</span>`
          : "";

      return `
        <button
          class="details-tab${isActive ? " active" : ""}"
          id="${safePrefix}-tab-${safeId}"
          type="button"
          role="tab"
          data-dialog-tab="${safeId}"
          aria-controls="${safePrefix}-panel-${safeId}"
          aria-selected="${isActive}"
          tabindex="${isActive ? "0" : "-1"}"
        >
          ${icon}
          <span class="details-tab-label">${escapeTabValue(tab.label)}</span>
          ${badge}
        </button>
      `;
    })
    .join("");

  return `
    <div class="details-tab-row">
      <div
        class="details-tab-list"
        data-dialog-tab-list
        data-dialog-tab-prefix="${safePrefix}"
        role="tablist"
        aria-label="${escapeTabValue(ariaLabel)}"
        aria-orientation="horizontal"
      >
        ${buttons}
      </div>
      ${actionsHtml ? `<div class="details-tab-actions">${actionsHtml}</div>` : ""}
    </div>
  `;
}

export class DialogTabBar {
  constructor({
    root,
    tabs,
    activeTab,
    beforeChange,
    onChange,
    panelSelector = ".dialog-body > [data-tab-content]",
  }) {
    this.root = root;
    this.tabs = Array.isArray(tabs) ? tabs : [];
    this.tabIds = new Set(this.tabs.map((tab) => tab.id));
    this.activeTab = null;
    this.beforeChange = beforeChange || (() => true);
    this.onChange = onChange || (() => {});
    this.list = this.root?.querySelector("[data-dialog-tab-list]") || null;
    this.panels = this.root ? Array.from(this.root.querySelectorAll(panelSelector)) : [];

    if (!this.root || !this.list) {
      throw new Error("[DialogTabBar] Tab row not found");
    }

    this.idPrefix = this.list.dataset.dialogTabPrefix || "details";
    this._clickHandler = (event) => this._handleClick(event);
    this._keyHandler = (event) => this._handleKeyboard(event);
    this.list.addEventListener("click", this._clickHandler);
    this.list.addEventListener("keydown", this._keyHandler);
    this.setActive(activeTab || this.tabs[0]?.id, { silent: true, force: true });
  }

  _getButtons() {
    return Array.from(this.list.querySelectorAll("[data-dialog-tab]"));
  }

  _handleClick(event) {
    const button = event.target.closest("[data-dialog-tab]");
    if (!button || !this.list.contains(button)) return;
    this.setActive(button.dataset.dialogTab);
  }

  _handleKeyboard(event) {
    const buttons = this._getButtons();
    const currentIndex = buttons.findIndex((button) => button.dataset.dialogTab === this.activeTab);
    if (currentIndex < 0 || buttons.length === 0) return;

    let targetIndex = currentIndex;
    switch (event.key) {
      case "ArrowLeft":
        targetIndex = currentIndex > 0 ? currentIndex - 1 : buttons.length - 1;
        break;
      case "ArrowRight":
        targetIndex = currentIndex < buttons.length - 1 ? currentIndex + 1 : 0;
        break;
      case "Home":
        targetIndex = 0;
        break;
      case "End":
        targetIndex = buttons.length - 1;
        break;
      default:
        return;
    }

    event.preventDefault();
    const target = buttons[targetIndex];
    this.setActive(target.dataset.dialogTab);
    target.focus();
  }

  setActive(tabId, { silent = false, force = false } = {}) {
    if (!tabId || !this.tabIds.has(tabId)) {
      console.warn(`[DialogTabBar] Invalid tab ID: ${tabId}`);
      return false;
    }
    if (!force && tabId === this.activeTab) return true;

    const previousTab = this.activeTab;
    if (!silent && this.beforeChange(tabId, previousTab) === false) return false;

    this.activeTab = tabId;
    this._syncDom();

    if (!silent) {
      playTabSwitch();
      this.onChange(tabId, previousTab);
    }
    return true;
  }

  _syncDom() {
    const buttons = this._getButtons();
    buttons.forEach((button) => {
      const isActive = button.dataset.dialogTab === this.activeTab;
      button.classList.toggle("active", isActive);
      button.setAttribute("aria-selected", String(isActive));
      button.setAttribute("tabindex", isActive ? "0" : "-1");
    });

    this.panels.forEach((panel) => {
      const tabId = panel.dataset.tabContent;
      const isActive = tabId === this.activeTab;
      const tabButton = buttons.find((button) => button.dataset.dialogTab === tabId);
      panel.id = `${this.idPrefix}-panel-${tabId}`;
      panel.classList.toggle("active", isActive);
      panel.hidden = !isActive;
      panel.setAttribute("role", "tabpanel");
      panel.setAttribute("aria-hidden", String(!isActive));
      if (tabButton) {
        panel.setAttribute("aria-labelledby", tabButton.id);
      } else {
        panel.removeAttribute("aria-labelledby");
      }
    });
  }

  destroy() {
    if (this.list) {
      this.list.removeEventListener("click", this._clickHandler);
      this.list.removeEventListener("keydown", this._keyHandler);
    }
    this.root = null;
    this.list = null;
    this.panels = [];
    this.tabs = [];
    this.tabIds.clear();
    this.activeTab = null;
  }
}
