import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";

export function createLifecycle() {
  // State
  let currentStrategy = null;
  let strategies = [];
  let templates = [];
  let conditionSchemas = null;

  // Canvas state
  let canvasNodes = [];
  let nextNodeId = 1;
  let selectedNodeId = null;
  let isDraggingNode = false;
  let draggedNode = null;
  let dragOffset = { x: 0, y: 0 };
  let canvasZoom = 1.0;
  let canvasPan = { x: 0, y: 0 };
  let isDraggingCanvas = false;
  let canvasDragStart = { x: 0, y: 0 };
  let canvasPanStart = { x: 0, y: 0 };

  // Pollers
  let strategiesPoller = null;
  let templatesPoller = null;

  return {
    async init(_ctx) {
      console.log("[Strategies] Initializing page");

      // Setup tab switching
      setupTabs();

      // Setup filters
      setupFilters();

      // Setup sidebar actions
      setupSidebarActions();

      // Setup canvas controls
      setupCanvasControls();

      // Setup toolbar actions
      setupToolbarActions();

      // Setup search
      setupSearch();

      // Load condition schemas
      await loadConditionSchemas();

      // Initialize condition library
      initializeConditionLibrary();

      // Setup canvas interactions
      setupCanvasInteractions();
    },

    async activate(ctx) {
      console.log("[Strategies] Activating page");

      // Create pollers
      strategiesPoller = ctx.managePoller(
        new Poller(async () => {
          await loadStrategies();
        }, { label: "Strategies" })
      );

      templatesPoller = ctx.managePoller(
        new Poller(async () => {
          await loadTemplates();
        }, { label: "Templates" })
      );

      // Start pollers
      strategiesPoller.start();
      templatesPoller.start();

      // Initial load
      await loadStrategies();
      await loadTemplates();
    },

    deactivate() {
      console.log("[Strategies] Deactivating page");
      // Pollers stopped automatically by lifecycle
    },

    dispose() {
      console.log("[Strategies] Disposing page");
      // Cleanup if needed
      currentStrategy = null;
      strategies = [];
      templates = [];
      canvasNodes = [];
      nextNodeId = 1;
      selectedNodeId = null;
    },
  };

  // Tab System
  function setupTabs() {
    const tabButtons = $$(".sidebar-tabs .tab-btn");
    const tabContents = $$(".tab-content");

    tabButtons.forEach((button) => {
      button.addEventListener("click", () => {
        const targetTab = button.dataset.tab;

        // Update buttons
        tabButtons.forEach((btn) => btn.classList.remove("active"));
        button.classList.add("active");

        // Update content
        tabContents.forEach((content) => {
          if (content.dataset.tab === targetTab) {
            content.classList.add("active");
          } else {
            content.classList.remove("active");
          }
        });
      });
    });
  }

  // Filters
  function setupFilters() {
    // Strategy type filters
    const filterButtons = $$(".strategy-filters .filter-btn");
    filterButtons.forEach((button) => {
      button.addEventListener("click", () => {
        filterButtons.forEach((btn) => btn.classList.remove("active"));
        button.classList.add("active");

        const filter = button.dataset.filter;
        filterStrategies(filter);
      });
    });

    // Template filters
    const categorySelect = $("#template-category");
    const riskSelect = $("#template-risk");

    if (categorySelect) {
      categorySelect.addEventListener("change", () => {
        filterTemplates();
      });
    }

    if (riskSelect) {
      riskSelect.addEventListener("change", () => {
        filterTemplates();
      });
    }
  }

  // Sidebar Actions
  function setupSidebarActions() {
    // Create new strategy
    const createBtn = $("#create-strategy");
    if (createBtn) {
      createBtn.addEventListener("click", () => {
        createNewStrategy();
      });
    }

    // Import strategy
    const importBtn = $("#import-strategy");
    if (importBtn) {
      importBtn.addEventListener("click", () => {
        importStrategy();
      });
    }

    // Refresh strategies
    const refreshBtn = $("#refresh-strategies");
    if (refreshBtn) {
      refreshBtn.addEventListener("click", async () => {
        await loadStrategies();
        Utils.showToast("Strategies refreshed", "success");
      });
    }
  }

  // Canvas Controls
  function setupCanvasControls() {
    const zoomInBtn = $("#zoom-in");
    const zoomOutBtn = $("#zoom-out");
    const resetViewBtn = $("#reset-view");
    const fitViewBtn = $("#fit-view");
    const autoLayoutBtn = $("#auto-layout");
    const toggleGridBtn = $("#toggle-grid");

    if (zoomInBtn) zoomInBtn.addEventListener("click", () => zoomCanvas(1.2));
    if (zoomOutBtn) zoomOutBtn.addEventListener("click", () => zoomCanvas(0.8));
    if (resetViewBtn) resetViewBtn.addEventListener("click", resetCanvasView);
    if (fitViewBtn) fitViewBtn.addEventListener("click", fitCanvasToView);
    if (autoLayoutBtn) autoLayoutBtn.addEventListener("click", autoLayoutNodes);
    if (toggleGridBtn) toggleGridBtn.addEventListener("click", toggleGrid);

    // Empty state actions
    const addRootBtn = $("#add-root-node");
    const loadTemplateBtn = $("#load-template");

    if (addRootBtn) {
      addRootBtn.addEventListener("click", () => {
        addRootCondition();
      });
    }

    if (loadTemplateBtn) {
      loadTemplateBtn.addEventListener("click", () => {
        // Switch to templates tab
        const templatesTab = $(".tab-btn[data-tab='templates']");
        if (templatesTab) templatesTab.click();
      });
    }
  }

  // Toolbar Actions
  function setupToolbarActions() {
    const saveBtn = $("#save-strategy");
    const saveAsBtn = $("#save-as-strategy");
    const duplicateBtn = $("#duplicate-strategy");
    const validateBtn = $("#validate-strategy");
    const testBtn = $("#test-strategy");
    const deployBtn = $("#deploy-strategy");

    if (saveBtn) {
      saveBtn.addEventListener("click", async () => {
        await saveStrategy();
      });
    }

    if (saveAsBtn) {
      saveAsBtn.addEventListener("click", async () => {
        await saveStrategyAs();
      });
    }

    if (duplicateBtn) {
      duplicateBtn.addEventListener("click", () => {
        duplicateStrategy();
      });
    }

    if (validateBtn) {
      validateBtn.addEventListener("click", async () => {
        await validateStrategy();
      });
    }

    if (testBtn) {
      testBtn.addEventListener("click", async () => {
        await testStrategy();
      });
    }

    if (deployBtn) {
      deployBtn.addEventListener("click", async () => {
        await deployStrategy();
      });
    }
  }

  // Search
  function setupSearch() {
    const searchInput = $("#condition-search");
    const clearBtn = $("#clear-search");

    if (searchInput) {
      searchInput.addEventListener("input", (e) => {
        const query = e.target.value.toLowerCase();
        filterConditions(query);
      });
    }

    if (clearBtn) {
      clearBtn.addEventListener("click", () => {
        if (searchInput) {
          searchInput.value = "";
          filterConditions("");
        }
      });
    }
  }

  // Load Data
  async function loadStrategies() {
    try {
      const response = await fetch("/api/strategies");
      if (!response.ok) throw new Error("Failed to load strategies");

      const data = await response.json();
      const items = data.items || [];
      strategies = items.map((s) => ({
        id: s.id,
        name: s.name,
        description: s.description || null,
        type: s.strategy_type,
        enabled: !!s.enabled,
        priority: s.priority,
        created_at: s.created_at,
        updated_at: s.updated_at,
        author: s.author || null,
        version: s.version,
      }));

      renderStrategies();
    } catch (error) {
      console.error("Failed to load strategies:", error);
      Utils.showToast("Failed to load strategies", "error");
    }
  }

  async function loadTemplates() {
    try {
      const response = await fetch("/api/strategies/templates");
      if (!response.ok) throw new Error("Failed to load templates");

  const data = await response.json();
  templates = data.items || [];

      renderTemplates();
    } catch (error) {
      console.error("Failed to load templates:", error);
      // Don't show error toast for templates as they're optional
    }
  }

  async function loadConditionSchemas() {
    try {
  const response = await fetch("/api/strategies/conditions/schemas");
      if (!response.ok) throw new Error("Failed to load condition schemas");

      const data = await response.json();
      conditionSchemas = data.schemas || {};
    } catch (error) {
      console.error("Failed to load condition schemas:", error);
      conditionSchemas = {};
    }
  }

  // Render Functions
  function renderStrategies() {
    const listContainer = $("#strategy-list");
    if (!listContainer) return;

    if (strategies.length === 0) {
      listContainer.innerHTML = `
        <div class="empty-state">
          <span class="icon">📝</span>
          <p>No strategies yet</p>
          <small>Create your first strategy</small>
        </div>
      `;
      return;
    }

    listContainer.innerHTML = strategies
      .map(
        (strategy) => `
      <div class="strategy-item ${currentStrategy?.id === strategy.id ? "active" : ""}" 
           data-strategy-id="${strategy.id}">
        <div class="strategy-item-header">
          <div class="strategy-item-title">
            <span class="icon">${strategy.type === "ENTRY" ? "📈" : "📉"}</span>
            ${Utils.escapeHtml(strategy.name)}
          </div>
          <div class="strategy-item-actions">
            <button class="btn-icon" data-action="edit" title="Edit">✏️</button>
            <button class="btn-icon" data-action="toggle" title="${strategy.enabled ? "Disable" : "Enable"}">
              ${strategy.enabled ? "✓" : "○"}
            </button>
            <button class="btn-icon" data-action="delete" title="Delete">🗑️</button>
          </div>
        </div>
        <div class="strategy-item-meta">
          <span class="strategy-badge ${strategy.type.toLowerCase()}">${strategy.type}</span>
          <span class="strategy-badge ${strategy.enabled ? "enabled" : "disabled"}">
            ${strategy.enabled ? "Enabled" : "Disabled"}
          </span>
          ${strategy.priority ? `<span class="strategy-badge">Priority: ${strategy.priority}</span>` : ""}
        </div>
      </div>
    `
      )
      .join("");

    // Attach event listeners
    $$(".strategy-item").forEach((item) => {
      const strategyId = item.dataset.strategyId;

      item.addEventListener("click", (e) => {
        if (!e.target.closest(".btn-icon")) {
          loadStrategyToCanvas(strategyId);
        }
      });

      // Action buttons
      const editBtn = item.querySelector("[data-action='edit']");
      const toggleBtn = item.querySelector("[data-action='toggle']");
      const deleteBtn = item.querySelector("[data-action='delete']");

      if (editBtn) {
        editBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          loadStrategyToCanvas(strategyId);
        });
      }

      if (toggleBtn) {
        toggleBtn.addEventListener("click", async (e) => {
          e.stopPropagation();
          await toggleStrategyEnabled(strategyId);
        });
      }

      if (deleteBtn) {
        deleteBtn.addEventListener("click", async (e) => {
          e.stopPropagation();
          await deleteStrategy(strategyId);
        });
      }
    });
  }

  function renderTemplates() {
    const listContainer = $("#template-list");
    if (!listContainer) return;

    if (templates.length === 0) {
      listContainer.innerHTML = `
        <div class="empty-state">
          <span class="icon">📦</span>
          <p>No templates available</p>
        </div>
      `;
      return;
    }

    listContainer.innerHTML = templates
      .map(
        (template) => `
      <div class="template-item" data-template-id="${template.id}">
        <div class="template-item-header">
          <div class="template-item-title">${Utils.escapeHtml(template.name)}</div>
          <span class="risk-badge ${template.risk_level || "medium"}">
            ${(template.risk_level || "medium").toUpperCase()}
          </span>
        </div>
        <div class="template-item-description">
          ${Utils.escapeHtml(template.description || "No description")}
        </div>
        <div class="template-item-footer">
          <span class="strategy-badge">${template.category || "General"}</span>
          <button class="btn" data-action="use">Use Template</button>
        </div>
      </div>
    `
      )
      .join("");

    // Attach event listeners
    $$(".template-item").forEach((item) => {
      const useBtn = item.querySelector("[data-action='use']");
      if (useBtn) {
        useBtn.addEventListener("click", () => {
          const templateId = item.dataset.templateId;
          useTemplate(templateId);
        });
      }
    });
  }

  function initializeConditionLibrary() {
    const container = $("#condition-categories");
    if (!container || !conditionSchemas) return;

    // Group conditions by category
    const categories = {
      "Price Patterns": [],
      "Technical Indicators": [],
      "Market Context": [],
      "Position & Performance": [],
    };

    // Categorize conditions
    Object.entries(conditionSchemas).forEach(([type, schema]) => {
      const condition = { type, ...schema };

      if (type.includes("Price")) {
        categories["Price Patterns"].push(condition);
      } else if (type.includes("MA") || type.includes("RSI")) {
        categories["Technical Indicators"].push(condition);
      } else if (type.includes("Liquidity") || type.includes("Time")) {
        categories["Market Context"].push(condition);
      } else {
        categories["Position & Performance"].push(condition);
      }
    });

    // Render categories
    container.innerHTML = Object.entries(categories)
      .map(
        ([category, conditions]) => `
      <div class="condition-category">
        <div class="category-header" data-category="${category}">
          <div class="category-title">
            <span class="icon">${getCategoryIcon(category)}</span>
            ${category}
          </div>
          <span class="category-toggle">▼</span>
        </div>
        <div class="category-items">
          ${conditions.map((condition) => renderConditionItem(condition)).join("")}
        </div>
      </div>
    `
      )
      .join("");

    // Setup category toggle
    $$(".category-header").forEach((header) => {
      header.addEventListener("click", () => {
        const items = header.nextElementSibling;
        const toggle = header.querySelector(".category-toggle");

        if (items.classList.contains("collapsed")) {
          items.classList.remove("collapsed");
          toggle.textContent = "▼";
        } else {
          items.classList.add("collapsed");
          toggle.textContent = "▶";
        }
      });
    });

    // Setup drag and drop
    setupDragAndDrop();
  }

  function renderConditionItem(condition) {
    return `
      <div class="condition-item" draggable="true" data-condition-type="${condition.type}">
        <div class="condition-item-header">
          <span class="condition-icon">${getConditionIcon(condition.type)}</span>
          <span class="condition-name">${condition.name || condition.type}</span>
        </div>
        <div class="condition-description">
          ${condition.description || "No description available"}
        </div>
      </div>
    `;
  }

  // Helper Functions
  function getCategoryIcon(category) {
    const icons = {
      "Price Patterns": "💰",
      "Technical Indicators": "📊",
      "Market Context": "🌐",
      "Position & Performance": "📈",
    };
    return icons[category] || "📌";
  }

  function getConditionIcon(type) {
    const icons = {
      PriceThreshold: "🎯",
      PriceMovement: "📈",
      RelativeToMA: "📉",
      LiquidityDepth: "💧",
      PositionAge: "⏱️",
    };
    return icons[type] || "🔹";
  }

  function filterStrategies(filter) {
    const items = $$(".strategy-item");
    items.forEach((item) => {
      const strategyId = item.dataset.strategyId;
      const strategy = strategies.find((s) => s.id === strategyId);

      if (!strategy) {
        item.style.display = "none";
        return;
      }

      if (filter === "all") {
        item.style.display = "";
      } else if (filter === "entry") {
        item.style.display = strategy.type === "ENTRY" ? "" : "none";
      } else if (filter === "exit") {
        item.style.display = strategy.type === "EXIT" ? "" : "none";
      }
    });
  }

  function filterTemplates() {
    const categorySelect = $("#template-category");
    const riskSelect = $("#template-risk");

    if (!categorySelect || !riskSelect) return;

    const category = categorySelect.value;
    const risk = riskSelect.value;

    const items = $$(".template-item");
    items.forEach((item) => {
      const templateId = item.dataset.templateId;
      const template = templates.find((t) => t.id === templateId);

      if (!template) {
        item.style.display = "none";
        return;
      }

  const matchesCategory = category === "all" || (template.category || "").toLowerCase() === category;
  const matchesRisk = risk === "all" || (template.risk_level || "").toLowerCase() === risk;

      item.style.display = matchesCategory && matchesRisk ? "" : "none";
    });
  }

  function filterConditions(query) {
    const items = $$(".condition-item");
    items.forEach((item) => {
      const name = item.querySelector(".condition-name")?.textContent.toLowerCase() || "";
      const description =
        item.querySelector(".condition-description")?.textContent.toLowerCase() || "";

      const matches = name.includes(query) || description.includes(query);
      item.style.display = matches ? "" : "none";
    });

    // Hide empty categories
    $$(".condition-category").forEach((cat) => {
      const items = Array.from(cat.querySelectorAll(".condition-item"));
      const visibleItems = items.filter((el) => el.style.display !== "none").length;
      cat.style.display = visibleItems > 0 ? "" : "none";
    });
  }

  // Drag and Drop
  function setupDragAndDrop() {
    const canvas = $("#strategy-canvas");
    if (!canvas) return;

    // Draggable items
    $$(".condition-item").forEach((item) => {
      item.addEventListener("dragstart", (e) => {
        const conditionType = item.dataset.conditionType;
        e.dataTransfer.setData("conditionType", conditionType);
        item.classList.add("dragging");
      });

      item.addEventListener("dragend", () => {
        item.classList.remove("dragging");
      });
    });

    // Drop zone
    canvas.addEventListener("dragover", (e) => {
      e.preventDefault();
    });

    canvas.addEventListener("drop", (e) => {
      e.preventDefault();
      const conditionType = e.dataTransfer.getData("conditionType");

      if (conditionType) {
        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        addConditionToCanvas(conditionType, x, y);
      }
    });

    // Canvas panning - mousedown on canvas background
    canvas.addEventListener("mousedown", (e) => {
      // Only start panning if clicking on the canvas itself, not on a node
      if (e.target === canvas) {
        isDraggingCanvas = true;
        canvasDragStart = { x: e.clientX, y: e.clientY };
        canvasPanStart = { x: canvasPan.x, y: canvasPan.y };
        canvas.style.cursor = "grabbing";
        e.preventDefault();
      }
    });
  }

  // Canvas Operations
  function addConditionToCanvas(conditionType, x, y) {
    console.log(`Adding condition ${conditionType} at (${x}, ${y})`);

    // Get condition schema
    const schema = conditionSchemas[conditionType];
    if (!schema) {
      Utils.showToast("Unknown condition type", "error");
      return;
    }

    // Create new node
    const node = {
      id: `node-${nextNodeId++}`,
      type: "condition",
      conditionType,
      name: schema.name || conditionType,
      parameters: createDefaultParameters(schema),
      position: { x: x / canvasZoom - canvasPan.x, y: y / canvasZoom - canvasPan.y },
    };

    canvasNodes.push(node);
    renderCanvas();
    selectNode(node.id);

    Utils.showToast(`Added ${node.name}`, "success");
  }

  function createDefaultParameters(schema) {
    const params = {};
    if (schema.parameters) {
      Object.entries(schema.parameters).forEach(([key, paramSchema]) => {
        params[key] = paramSchema.default || null;
      });
    }
    return params;
  }

  function renderCanvas() {
    const canvas = $("#strategy-canvas");
    if (!canvas) return;

    // Hide empty state if we have nodes
    const emptyState = canvas.querySelector(".canvas-empty-state");
    if (emptyState) {
      emptyState.style.display = canvasNodes.length > 0 ? "none" : "";
    }

    // Remove existing nodes (except empty state)
    canvas.querySelectorAll(".canvas-node").forEach((node) => node.remove());
    canvas.querySelectorAll(".canvas-connection").forEach((conn) => conn.remove());

    // Render nodes
    canvasNodes.forEach((node) => {
      renderNode(node);
    });

    // Update rule tree structure
    updateRuleTreeFromCanvas();
  }

  function renderNode(node) {
    const canvas = $("#strategy-canvas");
    if (!canvas) return;

    const nodeEl = document.createElement("div");
    nodeEl.className = `canvas-node ${node.type} ${selectedNodeId === node.id ? "selected" : ""}`;
    nodeEl.dataset.nodeId = node.id;
    nodeEl.style.left = `${node.position.x * canvasZoom + canvasPan.x}px`;
    nodeEl.style.top = `${node.position.y * canvasZoom + canvasPan.y}px`;
    nodeEl.style.transform = `scale(${canvasZoom})`;
    nodeEl.style.transformOrigin = "top left";

    const icon = getConditionIcon(node.conditionType);
    const paramCount = Object.keys(node.parameters || {}).length;

    nodeEl.innerHTML = `
      <div class="node-header">
        <span class="node-icon">${icon}</span>
        <span class="node-title">${Utils.escapeHtml(node.name)}</span>
        <button class="node-delete" title="Delete node">✕</button>
      </div>
      <div class="node-body">
        <div class="node-type">${Utils.escapeHtml(node.conditionType)}</div>
        ${paramCount > 0 ? `<div class="node-params">${paramCount} parameters</div>` : ""}
      </div>
    `;

    // Attach event listeners
    nodeEl.addEventListener("mousedown", (e) => {
      if (e.target.classList.contains("node-delete")) {
        deleteNode(node.id);
        e.stopPropagation();
        return;
      }
      startNodeDrag(node.id, e);
    });

    nodeEl.addEventListener("click", (e) => {
      if (!e.target.classList.contains("node-delete")) {
        selectNode(node.id);
      }
    });

    canvas.appendChild(nodeEl);
  }

  function selectNode(nodeId) {
    selectedNodeId = nodeId;
    renderCanvas();

    // Open parameter editor modal instead of switching tabs
    const node = canvasNodes.find((n) => n.id === nodeId);
    if (node) {
      openParameterEditor(node);
    }
  }

  function deleteNode(nodeId) {
    canvasNodes = canvasNodes.filter((n) => n.id !== nodeId);
    if (selectedNodeId === nodeId) {
      selectedNodeId = null;
      renderPropertiesPanel(null);
    }
    renderCanvas();
    Utils.showToast("Node deleted", "success");
  }

  function startNodeDrag(nodeId, e) {
    const node = canvasNodes.find((n) => n.id === nodeId);
    if (!node) return;

    isDraggingNode = true;
    draggedNode = node;
    dragOffset = {
      x: e.clientX / canvasZoom - node.position.x,
      y: e.clientY / canvasZoom - node.position.y,
    };

    e.preventDefault();
  }

  function updateRuleTreeFromCanvas() {
    if (canvasNodes.length === 0) {
      if (currentStrategy) {
        currentStrategy.rules = null;
      }
      return;
    }

    // For now, create a simple AND rule with all conditions
    // In the future, this could support complex tree structures
    const conditions = canvasNodes
      .filter((n) => n.type === "condition")
      .map((n) => {
        const schema = conditionSchemas[n.conditionType] || { parameters: {} };
        const params = {};
        Object.keys(schema.parameters || {}).forEach((k) => {
          const v = n.parameters[k];
          if (v && typeof v === "object" && "value" in v) {
            params[k] = v;
          } else {
            const defv = schema.parameters[k]?.default;
            params[k] = { value: v, default: defv };
          }
        });
        return { condition: { type: n.conditionType, parameters: params } };
      });

    if (conditions.length === 0) {
      if (currentStrategy) currentStrategy.rules = null;
    } else if (conditions.length === 1) {
      if (currentStrategy) currentStrategy.rules = conditions[0];
    } else {
      if (currentStrategy) {
        currentStrategy.rules = {
          operator: "AND",
          conditions,
        };
      }
    }
  }

  function renderPropertiesPanel(node) {
    const editor = $("#property-editor");
    if (!editor) return;

    if (!node) {
      editor.innerHTML = `
        <div class="empty-state">
          <span class="icon">🎯</span>
          <p>No selection</p>
          <small>Select a node to edit properties</small>
        </div>
      `;
      return;
    }

    const schema = conditionSchemas[node.conditionType];
    if (!schema) {
      editor.innerHTML = `
        <div class="empty-state">
          <span class="icon">⚠️</span>
          <p>Unknown condition type</p>
        </div>
      `;
      return;
    }

    let html = `
      <div class="property-group">
        <div class="property-group-title">Condition Details</div>
        <div class="property-field">
          <label class="property-label">Name</label>
          <input type="text" class="property-input" id="node-name" value="${Utils.escapeHtml(node.name)}">
        </div>
        <div class="property-field">
          <label class="property-label">Type</label>
          <input type="text" class="property-input" value="${Utils.escapeHtml(node.conditionType)}" disabled>
        </div>
      </div>
    `;

    if (schema.parameters && Object.keys(schema.parameters).length > 0) {
      html += `
        <div class="property-group">
          <div class="property-group-title">Parameters</div>
      `;

      Object.entries(schema.parameters).forEach(([key, paramSchema]) => {
        const value = node.parameters[key] ?? paramSchema.default ?? "";
        html += renderParameterField(key, paramSchema, value);
      });

      html += "</div>";
    }

    editor.innerHTML = html;

    // Attach event listeners
    const nameInput = $("#node-name");
    if (nameInput) {
      nameInput.addEventListener("input", (e) => {
        node.name = e.target.value;
        renderCanvas();
      });
    }

    // Parameter inputs
    if (schema.parameters) {
      Object.keys(schema.parameters).forEach((key) => {
        const input = $(`#param-${key}`);
        if (input) {
          const handler = (e) => {
            const paramSchema = schema.parameters[key];
            let value = e.target.value;

            // Convert to appropriate type
            if (paramSchema.type === "number") {
              value = parseFloat(value) || 0;
            } else if (paramSchema.type === "boolean") {
              value = e.target.checked;
            }

            const existing = node.parameters[key];
            const defaultVal = paramSchema.default !== undefined ? paramSchema.default : null;
            if (existing && typeof existing === "object" && "value" in existing) {
              node.parameters[key] = { ...existing, value };
            } else {
              node.parameters[key] = { value, default: defaultVal };
            }
            updateRuleTreeFromCanvas();
          };
          input.addEventListener("input", handler);
          input.addEventListener("change", handler);
        }
      });
    }
  }

  function renderParameterField(key, schema, value) {
    const type = schema.type || "string";
    const effectiveValue = value && typeof value === "object" && "value" in value ? value.value : value;
    const description = schema.description
      ? `<div class="property-description">${Utils.escapeHtml(schema.description)}</div>`
      : "";

    let inputHtml = "";

    switch (type) {
      case "number":
        inputHtml = `<input type="number" class="property-input" id="param-${key}" value="${effectiveValue}" 
          ${schema.min !== undefined ? `min="${schema.min}"` : ""} 
          ${schema.max !== undefined ? `max="${schema.max}"` : ""} 
          ${schema.step !== undefined ? `step="${schema.step}"` : ""}>`;
        break;

      case "boolean":
        inputHtml = `<input type="checkbox" id="param-${key}" ${effectiveValue ? "checked" : ""}>`;
        break;

      case "enum":
        {
          const opts = (schema.values && Array.isArray(schema.values)) ? schema.values : (schema.options || []);
          if (opts && Array.isArray(opts) && opts.length > 0) {
            inputHtml = `<select class="property-input" id="param-${key}">
              ${opts.map((v) => `<option value="${v}" ${v === effectiveValue ? "selected" : ""}>${v}</option>`).join("")}
            </select>`;
          } else {
            inputHtml = `<input type="text" class="property-input" id="param-${key}" value="${Utils.escapeHtml(String(effectiveValue))}">`;
          }
        }
        break;

      default:
        if (schema.options && Array.isArray(schema.options) && schema.options.length > 0) {
          inputHtml = `<select class="property-input" id="param-${key}">
            ${schema.options.map((v) => `<option value="${v}" ${v === effectiveValue ? "selected" : ""}>${v}</option>`).join("")}
          </select>`;
        } else {
          inputHtml = `<input type="text" class="property-input" id="param-${key}" value="${Utils.escapeHtml(String(effectiveValue))}">`;
        }
    }

    return `
      <div class="property-field">
        <label class="property-label">${schema.name || key}</label>
        ${inputHtml}
        ${description}
      </div>
    `;
  }

  // Parameter Editor Modal Functions
  function openParameterEditor(node) {
    const modal = $("#parameter-editor-modal");
    const body = $("#parameter-editor-body");
    if (!modal || !body) return;

    const schema = conditionSchemas[node.conditionType];
    if (!schema) {
      Utils.showToast("Unknown condition type", "error");
      return;
    }

    let html = `
      <div class="property-group">
        <div class="property-group-title">Condition Details</div>
        <div class="property-field">
          <label class="property-label">Name</label>
          <input type="text" class="property-input" id="modal-node-name" value="${Utils.escapeHtml(node.name)}">
        </div>
        <div class="property-field">
          <label class="property-label">Type</label>
          <input type="text" class="property-input" value="${Utils.escapeHtml(node.conditionType)}" disabled>
        </div>
      </div>
    `;

    if (schema.parameters && Object.keys(schema.parameters).length > 0) {
      html += `
        <div class="property-group">
          <div class="property-group-title">Parameters</div>
      `;

      Object.entries(schema.parameters).forEach(([key, paramSchema]) => {
        const value = node.parameters[key] ?? paramSchema.default ?? "";
        html += renderParameterField(key, paramSchema, value);
      });

      html += "</div>";
    }

    body.innerHTML = html;

    // Store reference to the current node being edited
    modal.dataset.editingNodeId = node.id;

    // Show modal
    modal.classList.add("active");

    // Setup event listeners
    setupParameterEditorListeners(node, schema);
  }

  function setupParameterEditorListeners(node, schema) {
    const modal = $("#parameter-editor-modal");
    const closeBtn = $("#close-parameter-editor");
    const cancelBtn = $("#cancel-parameter-edit");
    const applyBtn = $("#apply-parameter-edit");

    // Close handlers
    const closeModal = () => {
      modal.classList.remove("active");
      selectedNodeId = null;
      renderCanvas();
    };

    closeBtn.onclick = closeModal;
    cancelBtn.onclick = closeModal;

    // Click outside to close
    modal.onclick = (e) => {
      if (e.target === modal) {
        closeModal();
      }
    };

    // Apply changes
    applyBtn.onclick = () => {
      // Update node name
      const nameInput = $("#modal-node-name");
      if (nameInput) {
        node.name = nameInput.value;
      }

      // Update parameters
      if (schema.parameters) {
        Object.keys(schema.parameters).forEach((key) => {
          const input = $(`#param-${key}`);
          if (input) {
            const paramSchema = schema.parameters[key];
            let value = input.value;

            // Convert to appropriate type
            if (paramSchema.type === "number") {
              value = parseFloat(value) || 0;
            } else if (paramSchema.type === "boolean") {
              value = input.checked;
            }

            const existing = node.parameters[key];
            const defaultVal = paramSchema.default !== undefined ? paramSchema.default : null;
            if (existing && typeof existing === "object" && "value" in existing) {
              node.parameters[key] = { ...existing, value };
            } else {
              node.parameters[key] = { value, default: defaultVal };
            }
          }
        });
      }

      updateRuleTreeFromCanvas();
      renderCanvas();
      closeModal();
      Utils.showToast("Parameters updated", "success");
    };

    // ESC key to close
    const escHandler = (e) => {
      if (e.key === "Escape") {
        closeModal();
        document.removeEventListener("keydown", escHandler);
      }
    };
    document.addEventListener("keydown", escHandler);
  }

  function zoomCanvas(factor) {
    canvasZoom = Math.max(0.5, Math.min(2.0, canvasZoom * factor));
    renderCanvas();
    Utils.showToast(`Zoom: ${Math.round(canvasZoom * 100)}%`, "info");
  }

  function resetCanvasView() {
    canvasZoom = 1.0;
    canvasPan = { x: 0, y: 0 };
    renderCanvas();
    Utils.showToast("View reset", "success");
  }

  function fitCanvasToView() {
    if (canvasNodes.length === 0) {
      resetCanvasView();
      return;
    }

    // Calculate bounding box
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;

    canvasNodes.forEach((node) => {
      minX = Math.min(minX, node.position.x);
      minY = Math.min(minY, node.position.y);
      maxX = Math.max(maxX, node.position.x + 200); // Approximate node width
      maxY = Math.max(maxY, node.position.y + 100); // Approximate node height
    });

    const canvas = $("#strategy-canvas");
    if (!canvas) return;

    const width = maxX - minX;
    const height = maxY - minY;
    const canvasWidth = canvas.clientWidth;
    const canvasHeight = canvas.clientHeight;

    const scaleX = canvasWidth / width;
    const scaleY = canvasHeight / height;
    canvasZoom = Math.min(scaleX, scaleY, 1.0) * 0.9; // 90% to add padding

    canvasPan = {
      x: (canvasWidth / canvasZoom - width) / 2 - minX,
      y: (canvasHeight / canvasZoom - height) / 2 - minY,
    };

    renderCanvas();
    Utils.showToast("Fit to view", "success");
  }

  function autoLayoutNodes() {
    if (canvasNodes.length === 0) return;

    // Simple grid layout
    const spacing = { x: 250, y: 150 };
    const columns = Math.ceil(Math.sqrt(canvasNodes.length));

    canvasNodes.forEach((node, index) => {
      const col = index % columns;
      const row = Math.floor(index / columns);
      node.position = {
        x: col * spacing.x + 50,
        y: row * spacing.y + 50,
      };
    });

    renderCanvas();
    Utils.showToast("Auto layout applied", "success");
  }

  function toggleGrid() {
    const canvas = $("#strategy-canvas");
    if (canvas) {
      canvas.classList.toggle("no-grid");
      Utils.showToast("Grid toggled", "success");
    }
  }

  function addRootCondition() {
    const canvas = $("#strategy-canvas");
    if (!canvas) return;

    // Add a condition at the center
    const rect = canvas.getBoundingClientRect();
    const x = rect.width / 2;
    const y = rect.height / 2;

    // Use the first available condition type
    const firstConditionType = Object.keys(conditionSchemas)[0];
    if (firstConditionType) {
      addConditionToCanvas(firstConditionType, x, y);
    } else {
      Utils.showToast("No condition types available", "warning");
    }
  }

  // Canvas interaction handlers
  function setupCanvasInteractions() {
    const canvas = $("#strategy-canvas");
    if (!canvas) return;

    // Mouse move for dragging nodes and panning canvas
    document.addEventListener("mousemove", (e) => {
      if (isDraggingNode && draggedNode) {
        draggedNode.position = {
          x: e.clientX / canvasZoom - dragOffset.x,
          y: e.clientY / canvasZoom - dragOffset.y,
        };
        renderCanvas();
      } else if (isDraggingCanvas) {
        const deltaX = e.clientX - canvasDragStart.x;
        const deltaY = e.clientY - canvasDragStart.y;
        canvasPan = {
          x: canvasPanStart.x + deltaX,
          y: canvasPanStart.y + deltaY,
        };
        renderCanvas();
      }
    });

    // Mouse up to stop dragging
    document.addEventListener("mouseup", () => {
      if (isDraggingNode) {
        isDraggingNode = false;
        draggedNode = null;
      }
      if (isDraggingCanvas) {
        isDraggingCanvas = false;
        const canvas = $("#strategy-canvas");
        if (canvas) canvas.style.cursor = "grab";
      }
    });
  }

  // Strategy Operations
  function createNewStrategy() {
    currentStrategy = {
      id: null,
      name: "New Strategy",
      type: "ENTRY",
      enabled: true,
      priority: 10,
      rules: null,
      parameters: {},
    };

    // Update UI
    const nameInput = $("#strategy-name");
    const typeSelect = $("#strategy-type");

    if (nameInput) nameInput.value = currentStrategy.name;
    if (typeSelect) typeSelect.value = currentStrategy.type;

    // Clear canvas
    canvasNodes = [];
    nextNodeId = 1;
    selectedNodeId = null;
    renderCanvas();
    renderPropertiesPanel(null);

    Utils.showToast("New strategy created", "success");
  }

  async function loadStrategyToCanvas(strategyId) {
    try {
      const response = await fetch(`/api/strategies/${strategyId}`);
      if (!response.ok) throw new Error("Failed to load strategy");

      const data = await response.json();
      currentStrategy = {
        id: data.id,
        name: data.name,
        description: data.description || null,
        type: data.strategy_type,
        enabled: !!data.enabled,
        priority: data.priority,
        rules: data.rules || null,
        parameters: data.parameters || {},
        created_at: data.created_at,
        updated_at: data.updated_at,
        author: data.author || null,
        version: data.version,
      };

      // Update UI
      const nameInput = $("#strategy-name");
      const typeSelect = $("#strategy-type");

      if (nameInput) nameInput.value = currentStrategy.name;
      if (typeSelect) typeSelect.value = currentStrategy.type;

      // Render strategy on canvas
      renderStrategyOnCanvas(currentStrategy.rules);

      // Update active state in list
      $$(".strategy-item").forEach((item) => {
        if (item.dataset.strategyId === strategyId) {
          item.classList.add("active");
        } else {
          item.classList.remove("active");
        }
      });

      Utils.showToast(`Loaded strategy: ${currentStrategy.name}`, "success");
    } catch (error) {
      console.error("Failed to load strategy:", error);
      Utils.showToast("Failed to load strategy", "error");
    }
  }

  function renderStrategyOnCanvas(rules) {
    // Clear existing nodes
    canvasNodes = [];
    nextNodeId = 1;
    selectedNodeId = null;

    if (!rules) {
      renderCanvas();
      return;
    }

    // Parse rule tree and create nodes
    const spacing = { x: 250, y: 150 };
    let nodeIndex = 0;

    function addRuleNode(rule, depth = 0, index = 0) {
      const x = index * spacing.x + 50;
      const y = depth * spacing.y + 50;

      if (rule.operator) {
        // It's a logical operator node
        if (rule.conditions && Array.isArray(rule.conditions)) {
          rule.conditions.forEach((condition) => {
            addRuleNode(condition, depth, nodeIndex++);
          });
        }
      } else if (rule.condition) {
        // It's a condition node
        const cond = rule.condition;
        const schema = conditionSchemas[cond.type];
        const node = {
          id: `node-${nextNodeId++}`,
          type: "condition",
          conditionType: cond.type,
          name: schema?.name || cond.type,
          parameters: cond.parameters || {},
          position: { x, y },
        };
        canvasNodes.push(node);
      }
    }

    addRuleNode(rules);

    // Auto-layout for better visualization
    if (canvasNodes.length > 0) {
      autoLayoutNodes();
    } else {
      renderCanvas();
    }
  }

  async function saveStrategy() {
    if (!currentStrategy) {
      Utils.showToast("No strategy to save", "warning");
      return;
    }

    try {
      // Get current values from UI
      const nameInput = $("#strategy-name");
      const typeSelect = $("#strategy-type");

      if (nameInput) currentStrategy.name = nameInput.value;
      if (typeSelect) currentStrategy.type = typeSelect.value;

      const body = {
        name: currentStrategy.name,
        description: currentStrategy.description || null,
        strategy_type: currentStrategy.type,
        enabled: !!currentStrategy.enabled,
        priority: currentStrategy.priority ?? 10,
        rules: currentStrategy.rules || null,
        parameters: currentStrategy.parameters || {},
        author: currentStrategy.author || null,
      };

      const method = currentStrategy.id ? "PUT" : "POST";
      const url = currentStrategy.id ? `/api/strategies/${currentStrategy.id}` : "/api/strategies";

      const response = await fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (!response.ok) throw new Error("Failed to save strategy");

      const data = await response.json();
      if (!currentStrategy.id && data.id) {
        currentStrategy.id = data.id;
      }

      await loadStrategies();
      Utils.showToast("Strategy saved successfully", "success");
    } catch (error) {
      console.error("Failed to save strategy:", error);
      Utils.showToast("Failed to save strategy", "error");
    }
  }

  async function saveStrategyAs() {
    if (!currentStrategy) {
      Utils.showToast("No strategy to save", "warning");
      return;
    }

    // eslint-disable-next-line no-undef
    const newName = prompt("Enter new strategy name:", `${currentStrategy.name} (Copy)`);
    if (!newName) return;
    const newStrategy = { ...currentStrategy, id: null, name: newName };
    currentStrategy = newStrategy;

    await saveStrategy();
  }

  function duplicateStrategy() {
    if (!currentStrategy) {
      Utils.showToast("No strategy to duplicate", "warning");
      return;
    }

    currentStrategy = {
      ...currentStrategy,
      id: null,
      name: `${currentStrategy.name} (Copy)`,
    };

    const nameInput = $("#strategy-name");
    if (nameInput) nameInput.value = currentStrategy.name;

    Utils.showToast("Strategy duplicated", "success");
  }

  async function validateStrategy() {
    if (!currentStrategy) {
      Utils.showToast("No strategy to validate", "warning");
      return;
    }

    try {
      const response = await fetch(`/api/strategies/${currentStrategy.id}/validate`, {
        method: "POST",
      });

      if (!response.ok) throw new Error("Validation failed");

      const data = await response.json();

      if (data.valid) {
        updateValidationStatus(true, "Strategy is valid");
        Utils.showToast("Strategy is valid", "success");
      } else {
        updateValidationStatus(false, data.errors?.join(", ") || "Invalid strategy");
        Utils.showToast("Strategy has errors", "error");
      }
    } catch (error) {
      console.error("Validation failed:", error);
      updateValidationStatus(false, error.message);
      Utils.showToast("Validation failed", "error");
    }
  }

  async function testStrategy() {
    if (!currentStrategy?.id) {
      Utils.showToast("Please save the strategy first", "warning");
      return;
    }

    try {
      const response = await fetch(`/api/strategies/${currentStrategy.id}/test`, {
        method: "POST",
      });

      if (!response.ok) throw new Error("Test failed");

      const data = await response.json();
      Utils.showToast(
        `Test result: ${data.result ? "Passed" : "Failed"}`,
        data.result ? "success" : "error"
      );
    } catch (error) {
      console.error("Test failed:", error);
      Utils.showToast("Test failed", "error");
    }
  }

  async function deployStrategy() {
    if (!currentStrategy?.id) {
      Utils.showToast("Please save the strategy first", "warning");
      return;
    }

    // eslint-disable-next-line no-undef
    const confirmed = confirm(
      `Deploy strategy "${currentStrategy.name}"? This will enable it for live trading.`
    );
    if (!confirmed) return;

    try {
      const response = await fetch(`/api/strategies/${currentStrategy.id}/deploy`, {
        method: "POST",
      });

      if (!response.ok) throw new Error("Deploy failed");

      await loadStrategies();
      Utils.showToast("Strategy deployed successfully", "success");
    } catch (error) {
      console.error("Deploy failed:", error);
      Utils.showToast("Deploy failed", "error");
    }
  }

  async function toggleStrategyEnabled(strategyId) {
    try {
      const strategy = strategies.find((s) => s.id === strategyId);
      if (!strategy) return;
      // Fetch full detail to avoid missing fields
      const detailRes = await fetch(`/api/strategies/${strategyId}`);
      if (!detailRes.ok) throw new Error("Failed to load strategy detail");
      const detail = await detailRes.json();

      const body = {
        name: detail.name,
        description: detail.description || null,
        strategy_type: detail.strategy_type,
        enabled: !strategy.enabled,
        priority: detail.priority,
        rules: detail.rules || null,
        parameters: detail.parameters || {},
        author: detail.author || null,
      };

      const response = await fetch(`/api/strategies/${strategyId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (!response.ok) throw new Error("Failed to toggle strategy");

      await loadStrategies();
      Utils.showToast(`Strategy ${strategy.enabled ? "disabled" : "enabled"}`, "success");
    } catch (error) {
      console.error("Failed to toggle strategy:", error);
      Utils.showToast("Failed to toggle strategy", "error");
    }
  }

  async function deleteStrategy(strategyId) {
    const strategy = strategies.find((s) => s.id === strategyId);
    if (!strategy) return;

    // eslint-disable-next-line no-undef
    const confirmed = confirm(`Delete strategy "${strategy.name}"? This action cannot be undone.`);
    if (!confirmed) return;

    try {
      const response = await fetch(`/api/strategies/${strategyId}`, {
        method: "DELETE",
      });

      if (!response.ok) throw new Error("Failed to delete strategy");

      if (currentStrategy?.id === strategyId) {
        currentStrategy = null;
        createNewStrategy();
      }

      await loadStrategies();
      Utils.showToast("Strategy deleted", "success");
    } catch (error) {
      console.error("Failed to delete strategy:", error);
      Utils.showToast("Failed to delete strategy", "error");
    }
  }

  function importStrategy() {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";

    input.onchange = async (e) => {
      const file = e.target.files[0];
      if (!file) return;

      try {
        const text = await file.text();
        const strategy = JSON.parse(text);

        currentStrategy = { ...strategy, id: null };

        const nameInput = $("#strategy-name");
        const typeSelect = $("#strategy-type");

        if (nameInput) nameInput.value = currentStrategy.name;
        if (typeSelect) typeSelect.value = currentStrategy.type;

        Utils.showToast("Strategy imported", "success");
      } catch (error) {
        console.error("Failed to import strategy:", error);
        Utils.showToast("Failed to import strategy", "error");
      }
    };

    input.click();
  }

  function useTemplate(templateId) {
    const template = templates.find((t) => t.id === templateId);
    if (!template) return;

    currentStrategy = {
      id: null,
      name: template.name,
      type: "ENTRY",
      enabled: false,
      priority: 10,
      rules: template.rules,
      parameters: template.parameters || {},
    };

    // Update UI
    const nameInput = $("#strategy-name");
    const typeSelect = $("#strategy-type");

    if (nameInput) nameInput.value = currentStrategy.name;
    if (typeSelect) typeSelect.value = currentStrategy.type;

  // Render on canvas
  renderStrategyOnCanvas(currentStrategy.rules);

  // Switch to strategies tab
    const strategiesTab = $(".tab-btn[data-tab='strategies']");
    if (strategiesTab) strategiesTab.click();

    Utils.showToast(`Loaded template: ${template.name}`, "success");
  }

  function updateValidationStatus(valid, message) {
    const status = $("#validation-status");
    if (!status) return;

    const icon = status.querySelector(".status-icon");
    const text = status.querySelector(".status-text");

    if (valid) {
      status.classList.remove("invalid");
      status.classList.add("valid");
      if (icon) icon.textContent = "✓";
    } else {
      status.classList.remove("valid");
      status.classList.add("invalid");
      if (icon) icon.textContent = "✗";
    }

    if (text) text.textContent = message;
  }
}

// Register page so router can init/activate it
registerPage("strategies", createLifecycle());
