/**
 * Navigation Tab Module - Tab order and visibility customization
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";

/**
 * Get default tab configuration
 */
function getDefaultTabs() {
  return [
    { id: "home", label: "Home", icon: "icon-house", order: 0, enabled: true },
    { id: "ai", label: "Assistant", icon: "icon-bot-message-square", order: 1, enabled: true },
    {
      id: "positions",
      label: "Positions",
      icon: "icon-chart-candlestick",
      order: 2,
      enabled: true,
    },
    { id: "tokens", label: "Tokens", icon: "icon-coins", order: 3, enabled: true },
    { id: "filtering", label: "Filtering", icon: "icon-list-filter", order: 4, enabled: true },
    { id: "trader", label: "Auto Trader", icon: "icon-bot", order: 5, enabled: true },
    { id: "wallets", label: "Wallets", icon: "icon-wallet", order: 7, enabled: true },
    { id: "transactions", label: "Transactions", icon: "icon-activity", order: 8, enabled: true },
    { id: "tools", label: "Tools", icon: "icon-wrench", order: 9, enabled: true },
    { id: "services", label: "Services", icon: "icon-server", order: 10, enabled: true },
    { id: "events", label: "Events", icon: "icon-radio-tower", order: 11, enabled: true },
    { id: "config", label: "Config", icon: "icon-settings", order: 12, enabled: true },
  ];
}

/**
 * Fetch default tab configuration from backend (single source of truth)
 * Falls back to local defaults on failure
 */
async function fetchDefaultTabs() {
  try {
    const response = await fetch("/api/config/gui/defaults");
    if (response.ok) {
      const result = await response.json();
      if (result.success && result.data?.tabs) {
        return result.data.tabs;
      }
    }
  } catch (e) {
    console.warn("Failed to fetch default tabs from API, using local fallback", e);
  }
  return getDefaultTabs();
}

/**
 * Build Navigation tab HTML
 */
export function buildNavigationTab(settings) {
  const navigation = settings?.dashboard?.navigation || {};
  const tabs = navigation.tabs || getDefaultTabs();

  // Sort tabs by order for display
  const sortedTabs = [...tabs].sort((a, b) => a.order - b.order);

  const tabItems = sortedTabs
    .map(
      (tab, index) => `
      <div class="settings-nav-tab-item" 
           data-tab-id="${tab.id}" 
           data-order="${tab.order}"
           draggable="true">
        <div class="settings-nav-tab-position">${index + 1}</div>
        <div class="settings-nav-tab-handle" title="Drag to reorder">
          <i class="icon-grip-vertical"></i>
        </div>
        <div class="settings-nav-tab-icon">
          <i class="${tab.icon}"></i>
        </div>
        <div class="settings-nav-tab-info">
          <span class="settings-nav-tab-label">${tab.label}</span>
        </div>
        <div class="settings-nav-tab-status ${tab.enabled ? "enabled" : "disabled"}">
          ${tab.enabled ? '<i class="icon-eye"></i>' : '<i class="icon-eye-off"></i>'}
        </div>
        <div class="settings-nav-tab-toggle">
          <label class="toggle">
            <input type="checkbox" ${tab.enabled ? "checked" : ""} ${tab.id === "home" ? "disabled" : ""}>
            <span class="toggle-track"></span>
          </label>
        </div>
      </div>
    `
    )
    .join("");

  return `
    <div class="settings-section">
      <div class="settings-section-header">
        <div class="settings-section-header-left">
          <h3 class="settings-section-title">
            <i class="icon-layout-grid"></i>
            Navigation Tabs
          </h3>
          <p class="settings-section-hint">Drag items to reorder. Toggle visibility with the switch.</p>
        </div>
        <button class="btn btn-secondary btn-sm" id="resetNavTabs">
          <i class="icon-rotate-ccw"></i>
          Reset
        </button>
      </div>
      <div class="settings-nav-tabs-list" id="navTabsList">
        ${tabItems}
      </div>
      <div class="settings-nav-tabs-note">
        <i class="icon-info"></i>
        <span>Changes apply after saving. Refresh the page to see updates in the navigation bar.</span>
      </div>
    </div>
  `;
}

/**
 * Refresh the navigation list after reordering
 */
function refreshNavigationList(content, settings) {
  const listContainer = content.querySelector("#navTabsList");
  if (!listContainer) return;

  const tabs = settings?.dashboard?.navigation?.tabs || getDefaultTabs();
  const sortedTabs = [...tabs].sort((a, b) => a.order - b.order);

  const tabItems = sortedTabs
    .map(
      (tab, index) => `
      <div class="settings-nav-tab-item" 
           data-tab-id="${tab.id}" 
           data-order="${tab.order}"
           draggable="true">
        <div class="settings-nav-tab-position">${index + 1}</div>
        <div class="settings-nav-tab-handle" title="Drag to reorder">
          <i class="icon-grip-vertical"></i>
        </div>
        <div class="settings-nav-tab-icon">
          <i class="${tab.icon}"></i>
        </div>
        <div class="settings-nav-tab-info">
          <span class="settings-nav-tab-label">${tab.label}</span>
        </div>
        <div class="settings-nav-tab-status ${tab.enabled ? "enabled" : "disabled"}">
          ${tab.enabled ? '<i class="icon-eye"></i>' : '<i class="icon-eye-off"></i>'}
        </div>
        <div class="settings-nav-tab-toggle">
          <label class="toggle">
            <input type="checkbox" ${tab.enabled ? "checked" : ""} ${tab.id === "home" ? "disabled" : ""}>
            <span class="toggle-track"></span>
          </label>
        </div>
      </div>
    `
    )
    .join("");

  listContainer.innerHTML = tabItems;
}

/**
 * Attach handlers for Navigation tab
 */
export function attachNavigationHandlers(dialog, content) {
  const list = content.querySelector("#navTabsList");
  if (!list) return;

  // Ensure navigation config exists
  if (!dialog.settings.dashboard) dialog.settings.dashboard = {};
  if (!dialog.settings.dashboard.navigation) {
    dialog.settings.dashboard.navigation = { tabs: getDefaultTabs() };
  }

  const getTabs = () => dialog.settings.dashboard.navigation.tabs;
  const setTabs = (tabs) => {
    dialog.settings.dashboard.navigation.tabs = tabs;
    dialog._checkForChanges();
  };

  // Drag and drop state
  let draggedItem = null;
  let draggedTabId = null;

  // Drag start
  list.addEventListener("dragstart", (e) => {
    const item = e.target.closest(".settings-nav-tab-item");
    if (!item) return;

    draggedItem = item;
    draggedTabId = item.dataset.tabId;
    item.classList.add("dragging");

    // Set drag image and data
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", draggedTabId);

    // Delay adding drag class for smooth animation
    requestAnimationFrame(() => {
      item.style.opacity = "0.5";
    });
  });

  // Drag end
  list.addEventListener("dragend", (e) => {
    const item = e.target.closest(".settings-nav-tab-item");
    if (item) {
      item.classList.remove("dragging");
      item.style.opacity = "";
    }
    draggedItem = null;
    draggedTabId = null;

    // Remove all drag-over states
    list.querySelectorAll(".settings-nav-tab-item").forEach((el) => {
      el.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
    });
  });

  // Drag over
  list.addEventListener("dragover", (e) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";

    const item = e.target.closest(".settings-nav-tab-item");
    if (!item || item === draggedItem) return;

    // Auto-scroll when near edges
    const listRect = list.getBoundingClientRect();
    const scrollZone = 50;
    const scrollSpeed = 8;

    if (e.clientY < listRect.top + scrollZone) {
      list.scrollTop -= scrollSpeed;
    } else if (e.clientY > listRect.bottom - scrollZone) {
      list.scrollTop += scrollSpeed;
    }

    // Calculate position for visual feedback
    const rect = item.getBoundingClientRect();
    const midY = rect.top + rect.height / 2;

    // Remove previous indicators
    list.querySelectorAll(".settings-nav-tab-item").forEach((el) => {
      if (el !== item) {
        el.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
      }
    });

    // Add indicator based on mouse position
    item.classList.add("drag-over");
    if (e.clientY < midY) {
      item.classList.add("drag-over-top");
      item.classList.remove("drag-over-bottom");
    } else {
      item.classList.add("drag-over-bottom");
      item.classList.remove("drag-over-top");
    }
  });

  // Drag leave
  list.addEventListener("dragleave", (e) => {
    const item = e.target.closest(".settings-nav-tab-item");
    if (item && !item.contains(e.relatedTarget)) {
      item.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
    }
  });

  // Drop
  list.addEventListener("drop", (e) => {
    e.preventDefault();

    const dropTarget = e.target.closest(".settings-nav-tab-item");
    if (!dropTarget || !draggedTabId || dropTarget.dataset.tabId === draggedTabId) {
      // Clean up
      list.querySelectorAll(".settings-nav-tab-item").forEach((el) => {
        el.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
      });
      return;
    }

    const tabs = getTabs();

    // Get current sorted order
    const sortedTabs = [...tabs].sort((a, b) => a.order - b.order);
    const draggedIdx = sortedTabs.findIndex((t) => t.id === draggedTabId);
    const dropIdx = sortedTabs.findIndex((t) => t.id === dropTarget.dataset.tabId);

    if (draggedIdx === -1 || dropIdx === -1) return;

    // Calculate insert position based on mouse
    const rect = dropTarget.getBoundingClientRect();
    const insertBefore = e.clientY < rect.top + rect.height / 2;

    // Remove from old position
    const [movedTab] = sortedTabs.splice(draggedIdx, 1);

    // Calculate new position (accounting for removal)
    let insertIdx = dropIdx;
    if (draggedIdx < dropIdx) {
      // Dragging down - dropIdx shifted by 1 after removal
      insertIdx = insertBefore ? dropIdx - 1 : dropIdx;
    } else {
      // Dragging up
      insertIdx = insertBefore ? dropIdx : dropIdx + 1;
    }

    // Clamp to valid range
    insertIdx = Math.max(0, Math.min(insertIdx, sortedTabs.length));

    // Insert at new position
    sortedTabs.splice(insertIdx, 0, movedTab);

    // Update order values in original tabs array
    sortedTabs.forEach((tab, idx) => {
      const originalTab = tabs.find((t) => t.id === tab.id);
      if (originalTab) originalTab.order = idx;
    });

    // Save and refresh
    setTabs(tabs);
    refreshNavigationList(content, dialog.settings);

    // Clean up drag states
    list.querySelectorAll(".settings-nav-tab-item").forEach((el) => {
      el.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
    });
  });

  // Toggle handler
  list.addEventListener("change", (e) => {
    if (e.target.type === "checkbox") {
      const item = e.target.closest(".settings-nav-tab-item");
      const tabId = item.dataset.tabId;
      if (tabId !== "home") {
        const tabs = getTabs();
        const tab = tabs.find((t) => t.id === tabId);
        if (tab) {
          tab.enabled = e.target.checked;
          // Update status icon
          const statusEl = item.querySelector(".settings-nav-tab-status");
          if (statusEl) {
            statusEl.className = `settings-nav-tab-status ${tab.enabled ? "enabled" : "disabled"}`;
            statusEl.innerHTML = tab.enabled
              ? '<i class="icon-eye"></i>'
              : '<i class="icon-eye-off"></i>';
          }
          setTabs(tabs);
        }
      }
    }
  });

  // Reset button handler
  const resetBtn = content.querySelector("#resetNavTabs");
  if (resetBtn) {
    resetBtn.addEventListener("click", async () => {
      // Fetch defaults from backend (single source of truth)
      const defaultTabs = await fetchDefaultTabs();
      dialog.settings.dashboard.navigation.tabs = defaultTabs;
      dialog._checkForChanges();
      refreshNavigationList(content, dialog.settings);
      Utils.showToast({
        type: "info",
        title: "Navigation reset to defaults",
      });
    });
  }
}
