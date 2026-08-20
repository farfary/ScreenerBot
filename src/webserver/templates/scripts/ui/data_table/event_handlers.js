/**
 * Event Handlers Mixin for DataTable
 * Handles all event listener attachments for the DataTable component
 */

import { openMenu, closeMenu, trackAnchoredMenu } from "../../core/menu_manager.js";
import { copyToClipboard, notifyCopied, notifyCopyFailed } from "../../core/utils.js";

export function applyEventHandlersMixin(DataTable) {
  const proto = DataTable.prototype;

  /**
   * Attach event listeners
   */
  proto._attachEvents = function () {
    // Remove old listeners first to prevent duplicates
    this._removeEventListeners();

    const toolbarRoot = this.elements.toolbar;

    // Search input
    const searchInput = toolbarRoot?.querySelector(".dt-search-input");
    const searchClear = toolbarRoot?.querySelector(".table-toolbar-search__clear");
    if (searchInput) {
      const searchConfig = this.toolbarView?.getItem("search") || {};

      const handler = (e) => {
        this.state.searchQuery = e.target.value;
        if (searchClear) {
          searchClear.hidden = !(e.target.value || "").length;
        }

        // Call custom onChange if provided
        if (typeof searchConfig.onChange === "function") {
          searchConfig.onChange(e.target.value, searchInput);
        }

        // Only apply filters if not using custom onChange (server-side search)
        if (!searchConfig.onChange) {
          this._applyFilters();
        }
        this._saveState();
      };
      this._addEventListener(searchInput, "input", handler);

      const keyHandler = (e) => {
        if (e.key === "Escape" && searchInput.value) {
          searchInput.value = "";
          this.state.searchQuery = "";
          if (searchClear) {
            searchClear.hidden = true;
          }

          // Call custom onChange if provided
          if (typeof searchConfig.onChange === "function") {
            searchConfig.onChange("", searchInput);
          }
          if (typeof searchConfig.onSubmit === "function") {
            searchConfig.onSubmit("", searchInput);
          }

          // Only apply filters if not using custom onChange
          if (!searchConfig.onChange) {
            this._applyFilters();
          }
          this._saveState();
        } else if (e.key === "Escape") {
          searchInput.blur();
        } else if (e.key === "Enter") {
          // Call custom onSubmit if provided
          if (typeof searchConfig.onSubmit === "function") {
            searchConfig.onSubmit(searchInput.value, searchInput);
          }
        }
      };
      this._addEventListener(searchInput, "keydown", keyHandler);
    }

    if (searchClear) {
      const searchConfig = this.toolbarView?.getItem("search") || {};
      searchClear.hidden = !(this.state.searchQuery || "").length;
      const clearHandler = () => {
        if (searchInput) {
          searchInput.value = "";
          searchInput.focus();
        }
        this.state.searchQuery = "";
        searchClear.hidden = true;

        // Call custom onChange if provided
        if (typeof searchConfig.onChange === "function") {
          searchConfig.onChange("", searchInput);
        }
        if (typeof searchConfig.onSubmit === "function") {
          searchConfig.onSubmit("", searchInput);
        }

        // Only apply filters if not using custom onChange
        if (!searchConfig.onChange) {
          this._applyFilters();
        }
        this._saveState();
      };
      this._addEventListener(searchClear, "click", clearHandler);
    }

    // Filter dropdowns
    const filterSelects = toolbarRoot?.querySelectorAll(".dt-filter") || [];
    filterSelects.forEach((select) => {
      const filterId = select.dataset.filterId;
      if (!filterId) {
        return;
      }

      const isCheckbox = select.type === "checkbox";

      if (!(filterId in this.state.filters)) {
        this.state.filters[filterId] = isCheckbox ? select.checked : select.value;
      }

      const handler = (e) => {
        const input = e.target;
        const isSwitch = input.type === "checkbox";
        const value = isSwitch ? input.checked : input.value;
        this.state.filters[filterId] = value;
        if (isSwitch) {
          const status = input.closest(".toggle")?.querySelector(".toggle-state");
          if (status) {
            const onLabel = status.dataset.onLabel || "On";
            const offLabel = status.dataset.offLabel || "All";
            status.textContent = value ? onLabel : offLabel;
          }
        }
        // Look the item up through the toolbar's flat index — a filter may live in
        // `filters`, in `controls`, or nested inside a group.
        const filterConfig = this.toolbarView?.getItem(filterId);

        const autoApply = filterConfig?.autoApply !== false;
        if (autoApply) {
          this._applyFilters();
        }
        this._saveState();

        if (typeof filterConfig?.onChange === "function") {
          filterConfig.onChange(value, input);
        }
      };
      this._addEventListener(select, "change", handler);
    });

    // Custom controls (text inputs, etc.)
    const controlInputs =
      toolbarRoot?.querySelectorAll(".table-toolbar-input[data-control-id]") || [];
    controlInputs.forEach((input) => {
      const controlId = input.dataset.controlId;
      if (!controlId) {
        return;
      }

      const controlConfig = this.toolbarView?.getItem(controlId);

      if (!(controlId in this.state.customControls)) {
        this.state.customControls[controlId] = input.value ?? "";
      } else if (input.value !== this.state.customControls[controlId]) {
        input.value = this.state.customControls[controlId];
      }

      const updateClearButton = () => {
        const wrapper = input.closest("[data-control-wrapper]");
        if (!wrapper) {
          return;
        }
        const clearBtn = wrapper.querySelector(".table-toolbar-input__clear");
        if (clearBtn) {
          clearBtn.hidden = !(input.value || "").length;
        }
      };

      updateClearButton();

      const inputHandler = (e) => {
        const value = e.target.value;
        this.state.customControls[controlId] = value;
        updateClearButton();
        this._saveState();
        if (typeof controlConfig?.onChange === "function") {
          controlConfig.onChange(value, input);
        }
      };
      this._addEventListener(input, "input", inputHandler);

      const keyHandler = (e) => {
        if (e.key === "Enter") {
          if (typeof controlConfig?.onSubmit === "function") {
            controlConfig.onSubmit(input.value, input);
          }
        }
      };
      this._addEventListener(input, "keydown", keyHandler);

      const wrapper = input.closest("[data-control-wrapper]");
      if (wrapper) {
        const clearBtn = wrapper.querySelector(".table-toolbar-input__clear");
        if (clearBtn) {
          const clearHandler = () => {
            input.value = "";
            this.state.customControls[controlId] = "";
            updateClearButton();
            this._saveState();
            if (typeof controlConfig?.onChange === "function") {
              controlConfig.onChange("", input);
            }
            if (typeof controlConfig?.onClear === "function") {
              controlConfig.onClear(input);
            }
            input.focus();
          };
          this._addEventListener(clearBtn, "click", clearHandler);
        }
      }
    });

    // Toolbar buttons — matched on `data-btn-id` alone so a button behaves the same
    // whether it sits in the visible strip or inside the overflow menu.
    const buttons = toolbarRoot?.querySelectorAll("[data-btn-id]") || [];
    buttons.forEach((btn) => {
      const handler = () => {
        const btnConfig = this.toolbarView?.getItem(btn.dataset.btnId);
        if (typeof btnConfig?.onClick === "function") {
          btnConfig.onClick(btn, this);
        }
        if (btn.closest(".table-toolbar-menu")) {
          this._closeToolbarOverflow?.();
        }
      };
      this._addEventListener(btn, "click", handler);
    });

    this._attachToolbarSegmentedEvents(toolbarRoot);
    this._attachToolbarOverflowEvents(toolbarRoot);
    this._attachToolbarIdentityEvents(toolbarRoot);

    // Column visibility toggle - NEW: Using settings dialog instead of dropdown
    const columnBtn = toolbarRoot?.querySelector(".dt-btn-columns");
    if (columnBtn) {
      const settingsHandler = () => {
        this._openSettingsDialog();
      };
      this._addEventListener(columnBtn, "click", settingsHandler);
    }

    // Sortable headers - use event delegation on thead to survive innerHTML updates
    // This is critical because _renderTable() replaces thead.innerHTML, destroying direct th handlers
    const sortHandler = (e) => {
      // Ignore clicks on resize handles
      if (e.target.classList.contains("dt-resize-handle")) {
        return;
      }

      // Find the closest sortable th element
      const th = e.target.closest("th.sortable[data-column-id]");
      if (!th) return;

      const columnId = th.dataset.columnId;
      if (!columnId) return;

      if (this._getSortingMode() === "server") {
        const currentDirection =
          this.state.sortColumn === columnId ? this.state.sortDirection : null;
        // First click on a column uses the column's initial direction (desc by
        // default → newest/highest first); subsequent clicks toggle.
        const nextDirection = currentDirection
          ? currentDirection === "asc"
            ? "desc"
            : "asc"
          : this._getInitialSortDirection(columnId);
        this.state.sortColumn = columnId;
        this.state.sortDirection = nextDirection;

        this._saveState();

        // For server-side sorting, only update header sort icons - don't re-render body
        // The body will be re-rendered when new data arrives from the server
        this._updateHeaderSortIndicators();

        const sortingConfig = this.options.sorting || {};
        if (typeof sortingConfig.onChange === "function") {
          sortingConfig.onChange({
            column: columnId,
            direction: nextDirection,
            table: this,
          });
        }
        return;
      }

      if (this.state.sortColumn === columnId) {
        this.state.sortDirection = this.state.sortDirection === "asc" ? "desc" : "asc";
      } else {
        this.state.sortColumn = columnId;
        this.state.sortDirection = this._getInitialSortDirection(columnId);
      }

      this._applySort();
      this._saveState();
      // Force render since this is a user-initiated action (even if hovering)
      this._renderTable({ force: true });
    };
    this._addEventListener(this.elements.thead, "click", sortHandler);

    // Header right-click context menu (pin/unpin, hide). Delegated on thead so it
    // survives header re-renders. Attached in the CAPTURE phase and stops
    // propagation so the table's own menu wins over any global context menu, and
    // so clicks on the resize handle don't also open it.
    const headerContextHandler = (e) => {
      const th = e.target.closest("th[data-column-id]");
      if (!th || !this.elements.thead.contains(th)) {
        return;
      }
      // Never hijack the resize handle's own interactions.
      if (e.target.classList.contains("dt-resize-handle")) {
        return;
      }
      e.preventDefault();
      e.stopPropagation();
      this._openHeaderColumnMenu(th.dataset.columnId, e.clientX, e.clientY);
    };
    this.elements.thead.addEventListener("contextmenu", headerContextHandler, true);
    // Track for cleanup with the same capture flag used to register it.
    this.eventHandlers.set(`contextmenu_header_${Date.now()}_${Math.random()}`, {
      element: this.elements.thead,
      event: "contextmenu",
      handler: headerContextHandler,
      capture: true,
    });

    // Column resizing - use event delegation on thead to survive innerHTML updates
    const resizeHandler = (e) => {
      // Only handle mousedown on resize handles
      if (!e.target.classList.contains("dt-resize-handle")) return;

      e.preventDefault();
      e.stopPropagation();

      const handle = e.target;
      const th = handle.closest("th[data-column-id]");
      if (!th) return;

      const columnId = th.dataset.columnId;
      const minWidth = this._getColumnMinWidth(columnId);

      this.resizing = {
        columnId,
        startX: e.pageX,
        startWidth: th.offsetWidth,
        minWidth,
        leftHeader: th,
        handle,
      };

      th.classList.add("dt-resizing");
      handle.classList.add("active");
      document.body.classList.add("dt-column-resizing");

      // Remove any existing listeners before adding new ones to prevent duplicates
      document.removeEventListener("mousemove", this._handleResize);
      document.removeEventListener("mouseup", this._handleResizeEnd);
      document.addEventListener("mousemove", this._handleResize);
      document.addEventListener("mouseup", this._handleResizeEnd);
    };
    this._addEventListener(this.elements.thead, "mousedown", resizeHandler);

    // Image click handlers
    const imageContainers = this.elements.tbody.querySelectorAll(
      ".dt-image-clickable[data-image-click]"
    );
    imageContainers.forEach((container) => {
      const handler = (e) => {
        e.stopPropagation(); // Prevent row click
        const columnId = container.dataset.imageClick;
        const tr = container.closest("tr");
        if (tr && tr.dataset.rowId) {
          const rowId = tr.dataset.rowId;
          const row = this.state.filteredData.find(
            (r) => String(r[this.options.rowIdField]) === String(rowId)
          );
          if (row) {
            const column = this.options.columns.find((c) => c.id === columnId);
            if (column?.image?.onClick) {
              try {
                column.image.onClick(row, e);
              } catch (error) {
                this._log("error", `Image click handler failed for column ${columnId}`, error);
              }
            }
          }
        }
      };
      this._addEventListener(container, "click", handler);
    });

    // Action button click handlers
    const actionButtons = this.elements.tbody.querySelectorAll(".dt-action-btn[data-action-id]");
    actionButtons.forEach((btn) => {
      const handler = (e) => {
        e.stopPropagation(); // Prevent row click
        const actionId = btn.dataset.actionId;
        const td = btn.closest("td");
        if (td && td.dataset.rowId) {
          const rowId = td.dataset.rowId;
          const row = this.state.filteredData.find(
            (r) => String(r[this.options.rowIdField]) === String(rowId)
          );
          if (row) {
            const columnId = td.dataset.columnId;
            const column = this.options.columns.find((c) => c.id === columnId);
            if (column?.actions?.buttons) {
              const action = column.actions.buttons.find((a) => a.id === actionId);
              if (action?.onClick) {
                try {
                  action.onClick(row, e);
                } catch (error) {
                  this._log("error", `Action button handler failed for action ${actionId}`, error);
                }
              }
            }
          }
        }
      };
      this._addEventListener(btn, "click", handler);
    });

    // Dropdown toggle handlers
    // Shared descriptor so the global menu coordinator treats this table's open
    // row-action dropdown like any other dropdown: opening a select/menu anywhere
    // (or clicking an input/checkbox, changing page, etc.) auto-closes it.
    const closeAllActionDropdowns = (reason) => {
      const activeMenu = this._activeActionDropdown?.menu;
      const activeDropdown = this._activeActionDropdown?.dropdown;
      const activeTrigger = this._activeActionDropdown?.trigger;
      this._stopActionDropdownTracking?.();
      this._stopActionDropdownTracking = null;
      activeMenu?.classList.remove("open");
      activeMenu?.setAttribute("aria-hidden", "true");
      activeTrigger?.setAttribute("aria-expanded", "false");

      const allMenus = this.elements.tbody?.querySelectorAll(".dt-actions-dropdown-menu") || [];
      const allTriggers =
        this.elements.tbody?.querySelectorAll(".dt-actions-dropdown-trigger") || [];
      allMenus.forEach((m) => {
        m.style.display = "none";
        m.classList.remove("open");
        m.setAttribute("aria-hidden", "true");
      });
      allTriggers.forEach((t) => {
        t.classList.remove("active");
        t.setAttribute("aria-expanded", "false");
      });
      this._activeActionDropdown = null;
      closeMenu(this._actionDropdownHandle);
      if (reason === "escape") {
        activeTrigger?.focus({ preventScroll: true });
      }

      const finish = () => {
        if (activeMenu && activeDropdown?.isConnected) {
          activeDropdown.appendChild(activeMenu);
        } else {
          activeMenu?.remove();
        }
        activeMenu?.classList.remove("is-portaled", "menu-above");
        activeTrigger?.classList.remove("active", "menu-above");
        if (activeMenu) {
          activeMenu.style.display = "none";
          activeMenu.style.position = "";
          activeMenu.style.left = "";
          activeMenu.style.top = "";
          activeMenu.style.maxHeight = "";
          activeMenu.style.overflowY = "";
        }
      };
      if (
        !activeMenu ||
        [
          "superseded",
          "outside-pointer",
          "focus-left",
          "document-hidden",
          "navigation",
          "dialog-open",
        ].includes(reason)
      ) {
        finish();
      } else {
        setTimeout(finish, 140);
      }
    };
    this._actionDropdownHandle = {
      close: closeAllActionDropdowns,
      owns: (target) =>
        !!(
          target &&
          (this._activeActionDropdown?.menu?.contains(target) ||
            this._activeActionDropdown?.trigger?.contains(target))
        ),
    };

    const dropdownTriggers = this.elements.tbody.querySelectorAll(
      ".dt-actions-dropdown-trigger[data-action='dropdown-toggle']"
    );
    dropdownTriggers.forEach((trigger) => {
      const handler = (e) => {
        e.stopPropagation(); // Prevent row click
        const dropdown = trigger.closest(".dt-actions-dropdown");
        const menu =
          this._activeActionDropdown?.trigger === trigger
            ? this._activeActionDropdown.menu
            : dropdown.querySelector(".dt-actions-dropdown-menu");

        if (this._activeActionDropdown?.trigger !== trigger) {
          closeAllActionDropdowns("superseded");
        }

        // Close all other dropdowns first
        const allMenus = this.elements.tbody.querySelectorAll(".dt-actions-dropdown-menu");
        allMenus.forEach((m) => {
          if (m !== menu) {
            m.style.display = "none";
            m.classList.remove("open");
            m.setAttribute("aria-hidden", "true");
            const t = m
              .closest(".dt-actions-dropdown")
              ?.querySelector(".dt-actions-dropdown-trigger");
            if (t) t.setAttribute("aria-expanded", "false");
          }
        });

        // Toggle current menu
        if (menu) {
          const isOpen = menu.classList.contains("open") || menu.style.display === "block";
          if (isOpen) {
            closeAllActionDropdowns();
          } else {
            // Register first so any other open menu elsewhere is dismissed.
            openMenu(this._actionDropdownHandle);
            document.body.appendChild(menu);
            menu.style.display = "block";
            menu.classList.add("is-portaled");
            menu.setAttribute("aria-hidden", "false");
            trigger.classList.add("active");
            trigger.setAttribute("aria-expanded", "true");
            this._activeActionDropdown = { dropdown, menu, trigger };
            this._stopActionDropdownTracking = trackAnchoredMenu({
              trigger,
              menu,
              align: menu.classList.contains("menu-left") ? "start" : "end",
              onDetach: () => closeAllActionDropdowns(),
            });
            requestAnimationFrame(() => {
              if (this._activeActionDropdown?.menu === menu) menu.classList.add("open");
            });

            // Focus first interactive item for keyboard users
            const items = Array.from(
              menu.querySelectorAll(".dt-actions-dropdown-item:not(.disabled)")
            );
            if (items.length) {
              items.forEach((it) => it.setAttribute("tabindex", "-1"));
              const first = items[0];
              first.setAttribute("tabindex", "0");
              first.focus();
            }
          }
        }
      };
      this._addEventListener(trigger, "click", handler);

      // Keyboard toggle and navigation from trigger
      const keyHandler = (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          trigger.click();
          return;
        }
        if (e.key === "ArrowDown") {
          e.preventDefault();
          // open and focus first
          trigger.click();
          return;
        }
        if (e.key === "Escape") {
          const menu = this._activeActionDropdown?.trigger === trigger
            ? this._activeActionDropdown.menu
            : null;
          if (menu && menu.classList.contains("open")) {
            closeAllActionDropdowns("escape");
          }
        }
      };
      this._addEventListener(trigger, "keydown", keyHandler);
    });

    // Dropdown item click handlers
    const dropdownItems = this.elements.tbody.querySelectorAll(
      ".dt-actions-dropdown-item[data-action-id]"
    );
    dropdownItems.forEach((item) => {
      const handler = (e) => {
        e.stopPropagation(); // Prevent row click
        const actionId = item.dataset.actionId;
        const dropdown = item.closest(".dt-actions-dropdown") || this._activeActionDropdown?.dropdown;
        const rowId = dropdown?.dataset.rowId;

        if (rowId) {
          const row = this.state.filteredData.find(
            (r) => String(r[this.options.rowIdField]) === String(rowId)
          );
          if (row) {
            const td = dropdown.closest("td");
            const columnId = td?.dataset.columnId;
            const column = this.options.columns.find((c) => c.id === columnId);
            if (column?.actions?.items) {
              const action = column.actions.items.find((a) => a.id === actionId);
              if (action?.onClick) {
                try {
                  action.onClick(row, e);

                  // Close dropdown after action
                  const trigger = this._activeActionDropdown?.trigger;
                  closeAllActionDropdowns();
                  trigger?.focus({ preventScroll: true });
                } catch (error) {
                  this._log(
                    "error",
                    `Dropdown action handler failed for action ${actionId}`,
                    error
                  );
                }
              }
            }
          }
        }
      };
      this._addEventListener(item, "click", handler);

      // Keyboard support for menu items
      const itemKeyHandler = (e) => {
        const menu = item.closest(".dt-actions-dropdown-menu");
        const items = Array.from(menu.querySelectorAll(".dt-actions-dropdown-item:not(.disabled)"));
        const idx = items.indexOf(item);
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          item.click();
          return;
        }
        if (e.key === "ArrowDown") {
          e.preventDefault();
          const next = items[(idx + 1) % items.length];
          if (next) {
            items.forEach((it) => it.setAttribute("tabindex", "-1"));
            next.setAttribute("tabindex", "0");
            next.focus();
          }
          return;
        }
        if (e.key === "ArrowUp") {
          e.preventDefault();
          const prev = items[(idx - 1 + items.length) % items.length];
          if (prev) {
            items.forEach((it) => it.setAttribute("tabindex", "-1"));
            prev.setAttribute("tabindex", "0");
            prev.focus();
          }
          return;
        }
        if (e.key === "Home") {
          e.preventDefault();
          const first = items[0];
          if (first) {
            items.forEach((it) => it.setAttribute("tabindex", "-1"));
            first.setAttribute("tabindex", "0");
            first.focus();
          }
          return;
        }
        if (e.key === "End") {
          e.preventDefault();
          const last = items[items.length - 1];
          if (last) {
            items.forEach((it) => it.setAttribute("tabindex", "-1"));
            last.setAttribute("tabindex", "0");
            last.focus();
          }
          return;
        }
        if (e.key === "Escape") {
          e.preventDefault();
          closeAllActionDropdowns("escape");
        }
      };
      this._addEventListener(item, "keydown", itemKeyHandler);
    });

    // Row click
    if (this.options.onRowClick) {
      const handler = (e) => {
        const tr = e.target.closest("tr");
        if (tr && tr.dataset.rowId) {
          const rowId = tr.dataset.rowId;
          const row = this.state.filteredData.find(
            (r) => String(r[this.options.rowIdField]) === String(rowId)
          );
          if (row) {
            this.options.onRowClick(row, e);
          }
        }
      };
      this._addEventListener(this.elements.tbody, "click", handler);
    }

    // Hover tracking for render skipping during user interaction
    // This prevents table re-renders while user is hovering rows
    const mouseEnterHandler = () => {
      this._isHovering = true;
      if (this._hoverTimeout) {
        clearTimeout(this._hoverTimeout);
        this._hoverTimeout = null;
      }
    };
    const mouseLeaveHandler = () => {
      // Small delay before allowing re-renders to prevent flicker on quick mouse movements
      if (this._hoverTimeout) {
        clearTimeout(this._hoverTimeout);
      }
      this._hoverTimeout = setTimeout(() => {
        this._isHovering = false;
        this._hoverTimeout = null;
      }, 150);
    };
    this._addEventListener(this.elements.scrollContainer, "mouseenter", mouseEnterHandler);
    this._addEventListener(this.elements.scrollContainer, "mouseleave", mouseLeaveHandler);

    // Scroll position tracking (throttled to avoid excessive saves)
    const scrollHandler = () => {
      if (!this.elements.scrollContainer) {
        return;
      }
      this.state.scrollPosition = this.elements.scrollContainer.scrollTop;
      this._handlePaginationScroll();

      // Throttle state saves to once per 500ms
      if (this.scrollThrottle) {
        clearTimeout(this.scrollThrottle);
      }
      this.scrollThrottle = setTimeout(() => {
        this._saveState();
        this.scrollThrottle = null;
      }, 500);
    };
    this._addEventListener(this.elements.scrollContainer, "scroll", scrollHandler);

    // Sync horizontal scroll from body scroll container to the fixed header container
    if (this.elements.headerContainer && this.elements.scrollContainer) {
      const hScrollSync = () => {
        const scrollLeft = this.elements.scrollContainer.scrollLeft;
        this.elements.headerContainer.scrollLeft = scrollLeft;
        // Toggle a wrapper class so CSS can add an elevation/shadow to pinned
        // (floating-left) columns once the body has scrolled horizontally.
        if (this.elements.wrapper) {
          this.elements.wrapper.classList.toggle("is-pinned-scrolled", scrollLeft > 0);
        }
      };
      this._addEventListener(this.elements.scrollContainer, "scroll", hScrollSync);
    }

    // Attach client pagination events
    this._attachClientPaginationEvents();

    // Attach hybrid pagination mode toggle and server pagination events
    this._attachModeToggleEvents();
    this._attachServerPaginationEvents();
  };

  /**
   * Segmented view switchers in the toolbar.
   * The active value is owned by the page (via `onChange`), not by table state —
   * a view switch usually changes the query, which the page must drive itself.
   */
  proto._attachToolbarSegmentedEvents = function (toolbarRoot) {
    const segments = toolbarRoot?.querySelectorAll(".table-toolbar-seg") || [];
    segments.forEach((seg) => {
      const segId = seg.dataset.segId;
      if (!segId) {
        return;
      }
      seg.querySelectorAll(".table-toolbar-seg__btn").forEach((btn) => {
        const handler = () => {
          const value = btn.dataset.segValue;
          const config = this.toolbarView?.getItem(segId);
          if (config?.value === value) {
            return;
          }
          this.setToolbarSegmentValue(segId, value);
          if (typeof config?.onChange === "function") {
            config.onChange(value, btn, this);
          }
        };
        this._addEventListener(btn, "click", handler);
      });
    });
  };

  /**
   * The "More actions" overflow menu. Registered with the global menu manager so it
   * obeys the single-open rule and the Escape stack like every other dashboard menu.
   */
  proto._attachToolbarOverflowEvents = function (toolbarRoot) {
    const overflow = toolbarRoot?.querySelector(".table-toolbar-overflow");
    if (!overflow) {
      this._toolbarOverflowHandle = null;
      this._closeToolbarOverflow = null;
      if (this._toolbarOverflowCloseTimer !== null) {
        clearTimeout(this._toolbarOverflowCloseTimer);
        this._toolbarOverflowCloseTimer = null;
      }
      return;
    }

    const trigger = overflow.querySelector(".table-toolbar-overflow__trigger");
    const menu = overflow.querySelector(".table-toolbar-menu");
    if (!trigger || !menu) {
      return;
    }

    let isOpen = false;
    const close = (reason) => {
      isOpen = false;
      menu.classList.remove("open");
      trigger.setAttribute("aria-expanded", "false");
      closeMenu(this._toolbarOverflowHandle);
      if (reason === "escape") trigger.focus({ preventScroll: true });

      if (this._toolbarOverflowCloseTimer !== null) {
        clearTimeout(this._toolbarOverflowCloseTimer);
      }
      const finish = () => {
        this._toolbarOverflowCloseTimer = null;
        if (menu.classList.contains("open")) return;
        menu.hidden = true;
        trigger.classList.remove("active");
      };
      if (
        [
          "superseded",
          "outside-pointer",
          "focus-left",
          "document-hidden",
          "navigation",
          "dialog-open",
        ].includes(reason)
      ) {
        finish();
      } else {
        this._toolbarOverflowCloseTimer = setTimeout(finish, 220);
      }
    };

    this._toolbarOverflowHandle = {
      close,
      owns: (t) => !!(t && t.closest && t.closest(".table-toolbar-overflow")),
    };
    this._closeToolbarOverflow = close;

    const toggleHandler = (e) => {
      e.stopPropagation();
      if (!isOpen) {
        isOpen = true;
        if (this._toolbarOverflowCloseTimer !== null) {
          clearTimeout(this._toolbarOverflowCloseTimer);
          this._toolbarOverflowCloseTimer = null;
        }
        openMenu(this._toolbarOverflowHandle);
        menu.hidden = false;
        trigger.classList.add("active");
        trigger.setAttribute("aria-expanded", "true");
        requestAnimationFrame(() => {
          if (!isOpen || menu.hidden) return;
          menu.classList.add("open");
          menu.querySelector(".table-toolbar-menu__item:not(:disabled)")?.focus();
        });
      } else {
        close();
      }
    };
    this._addEventListener(trigger, "click", toggleHandler);

    const menuKeyHandler = (event) => {
      const items = Array.from(menu.querySelectorAll(".table-toolbar-menu__item:not(:disabled)"));
      const index = items.indexOf(document.activeElement);
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const direction = event.key === "ArrowDown" ? 1 : -1;
        const nextIndex =
          index < 0
            ? direction > 0
              ? 0
              : items.length - 1
            : (index + direction + items.length) % items.length;
        items[nextIndex]?.focus();
      } else if (event.key === "Home" || event.key === "End") {
        event.preventDefault();
        items[event.key === "Home" ? 0 : items.length - 1]?.focus();
      }
    };
    this._addEventListener(menu, "keydown", menuKeyHandler);
  };

  /**
   * Copy affordance on the toolbar identity address.
   */
  proto._attachToolbarIdentityEvents = function (toolbarRoot) {
    const copyBtn = toolbarRoot?.querySelector("[data-toolbar-copy]");
    if (!copyBtn) {
      return;
    }
    const handler = (e) => {
      e.stopPropagation();
      const value = copyBtn.dataset.toolbarCopy;
      if (!value) {
        return;
      }
      copyToClipboard(value)
        .then(() => notifyCopied("Address"))
        .catch(notifyCopyFailed);
    };
    this._addEventListener(copyBtn, "click", handler);
  };
}
