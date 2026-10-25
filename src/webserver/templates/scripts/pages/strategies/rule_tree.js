// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Strategy rule tree model - parses a saved rule tree into editable groups and conditions, serializes it back and checks it against the condition schemas.

/**
 * The editor model of a strategy's rules.
 *
 * - group: `{ kind: "group", id, match: "all" | "any", inverted, disabled, children }`
 * - condition: `{ kind: "condition", id, type, params, raw, inverted, disabled, known }`
 *
 * `params` holds the value of every parameter (`null` for an unset optional one or an
 * empty number field); `raw` holds the saved parameter object each value came from, so
 * constraints and keys the schema does not list survive a round trip. Ids exist only in
 * the editor and are never serialized.
 *
 * The saved tree writes a NOT as a branch of its own with one child. The model shows it
 * as `inverted` on the node it wraps, and `serializeRules` writes it back. Levels follow
 * the saved tree, which is what the engine limits: the root group is level 1, a node
 * directly under it level 2, and the NOT wrapper of an inverted node takes a level.
 *
 * Pure: no DOM, no localization.
 */

/** Deepest level of a saved rule tree (the engine's `MAX_RULE_DEPTH`). */
export const MAX_DEPTH = 4;

const OPERATORS = Object.freeze({ all: "AND", any: "OR" });
const NUMBER_TYPES = new Set(["number", "percent", "sol"]);

/** Deep copy of a model node, whose values are all JSON. */
const clone = (value) => JSON.parse(JSON.stringify(value));

let sequence = 0;
const nextId = () => {
  sequence += 1;
  return sequence;
};

function newGroup(match = "all") {
  return { kind: "group", id: nextId(), match, inverted: false, disabled: false, children: [] };
}

function isParamObject(saved) {
  return saved !== null && typeof saved === "object" && !Array.isArray(saved) && "value" in saved;
}

function defaultOf(spec) {
  return spec?.default ?? null;
}

function parseCondition(condition, schemas) {
  const schema = schemas?.[condition.type];
  const saved = condition.parameters ?? {};
  const params = {};
  const raw = {};
  for (const [key, spec] of Object.entries(schema?.parameters ?? {})) {
    if (isParamObject(saved[key])) {
      params[key] = saved[key].value;
      raw[key] = { ...saved[key] };
    } else {
      params[key] = saved[key] === undefined ? defaultOf(spec) : saved[key];
    }
  }
  for (const [key, value] of Object.entries(saved)) {
    if (key in params) continue;
    params[key] = isParamObject(value) ? value.value : value;
    raw[key] = isParamObject(value) ? { ...value } : value;
  }
  return {
    kind: "condition",
    id: nextId(),
    type: condition.type,
    params,
    raw,
    inverted: false,
    disabled: false,
    known: Boolean(schema),
  };
}

function parseNode(node, schemas) {
  if (node === null || typeof node !== "object") return newGroup();
  if (node.condition) {
    const leaf = parseCondition(node.condition, schemas);
    leaf.disabled = Boolean(node.disabled);
    return leaf;
  }
  const operator = String(node.operator ?? "AND").toUpperCase();
  const children = (Array.isArray(node.conditions) ? node.conditions : []).map((child) =>
    parseNode(child, schemas)
  );
  if (operator === "NOT") {
    const inner = children.length === 1 ? children[0] : Object.assign(newGroup(), { children });
    inner.inverted = !inner.inverted;
    inner.disabled = inner.disabled || Boolean(node.disabled);
    return inner;
  }
  const group = newGroup(operator === "OR" ? "any" : "all");
  group.children = children;
  group.disabled = Boolean(node.disabled);
  return group;
}

/**
 * Editable model of a saved rule tree. `null` is an empty root; a bare condition becomes
 * the only child of the root.
 * @param {object|null} rules Saved rule tree.
 * @param {object} schemas Condition schemas by type.
 * @returns {object} Root group.
 */
export function parseRules(rules, schemas) {
  if (!rules) return newGroup();
  const node = parseNode(rules, schemas);
  const root = node.kind === "group" ? node : Object.assign(newGroup(), { children: [node] });
  root.disabled = false;
  return root;
}

function serializeParams(node, schemas) {
  const specs = schemas?.[node.type]?.parameters ?? {};
  const parameters = {};
  for (const [key, value] of Object.entries(node.params)) {
    const raw = node.raw[key];
    const base = isParamObject(raw) ? raw : {};
    parameters[key] =
      key in specs ? { ...base, value, default: defaultOf(specs[key]) } : { ...base, value };
  }
  return parameters;
}

function serializeNode(node, schemas) {
  let out;
  if (node.kind === "group") {
    out = {
      operator: OPERATORS[node.match],
      conditions: node.children.map((child) => serializeNode(child, schemas)),
    };
  } else {
    // A condition type the schemas do not list is written back exactly as it was read.
    const parameters = node.known ? serializeParams(node, schemas) : clone(node.raw);
    out = { condition: { type: node.type, parameters } };
  }
  if (node.inverted) out = { operator: "NOT", conditions: [out] };
  if (node.disabled) out.disabled = true;
  return out;
}

/**
 * Saved rule tree of a model: `null` for a root without children, else a group node.
 * @param {object} root Root group.
 * @param {object} schemas Condition schemas by type.
 * @returns {object|null}
 */
export function serializeRules(root, schemas) {
  if (!root.children.length) return null;
  return serializeNode({ ...root, disabled: false }, schemas);
}

function locate(group, id, parent = null) {
  if (group.id === id) return { node: group, parent };
  if (group.kind !== "group") return null;
  for (const child of group.children) {
    const found = locate(child, id, group);
    if (found) return found;
  }
  return null;
}

/** The node with `id`, or `null`. */
export function findNode(root, id) {
  return locate(root, id)?.node ?? null;
}

/** The group holding the node with `id`, or `null` for the root and for an unknown id. */
export function parentOf(root, id) {
  return locate(root, id)?.parent ?? null;
}

/** Level of a node in the saved tree (see the module comment); `0` for an unknown id. */
export function depthOf(root, id) {
  const level = (group, base) => {
    const here = base + (group.inverted ? 1 : 0);
    if (group.id === id) return here;
    if (group.kind !== "group") return 0;
    for (const child of group.children) {
      const found = level(child, here + 1);
      if (found) return found;
    }
    return 0;
  };
  return level(root, 1);
}

/** Levels the subtree of `node` spans below and including itself, wrappers counted. */
function heightOf(node) {
  const own = node.inverted ? 2 : 1;
  if (node.kind !== "group") return own;
  return own + Math.max(0, ...node.children.map(heightOf));
}

/** Whether a condition can still be added to `groupId`. */
export function canAddCondition(root, groupId) {
  const group = findNode(root, groupId);
  return group?.kind === "group" && depthOf(root, groupId) + 1 <= MAX_DEPTH;
}

/** Whether a group added to `groupId` can still hold a condition. */
export function canAddGroup(root, groupId) {
  const group = findNode(root, groupId);
  return group?.kind === "group" && depthOf(root, groupId) + 2 <= MAX_DEPTH;
}

/** Whether inverting the node would push its subtree below the deepest level. */
export function canInvert(root, id) {
  const node = findNode(root, id);
  if (!node) return false;
  if (node.inverted) return true;
  return depthOf(root, id) + heightOf(node) <= MAX_DEPTH;
}

/** Add a condition with the schema defaults to a group; returns its id or `false`. */
export function addCondition(root, groupId, type, schemas) {
  const schema = schemas?.[type];
  if (!schema || !canAddCondition(root, groupId)) return false;
  const params = {};
  for (const [key, spec] of Object.entries(schema.parameters ?? {})) params[key] = defaultOf(spec);
  const node = {
    kind: "condition",
    id: nextId(),
    type,
    params,
    raw: {},
    inverted: false,
    disabled: false,
    known: true,
  };
  findNode(root, groupId).children.push(node);
  return node.id;
}

/** Add an empty `all` group to a group; returns its id or `false`. */
export function addGroup(root, groupId) {
  if (!canAddGroup(root, groupId)) return false;
  const group = newGroup();
  findNode(root, groupId).children.push(group);
  return group.id;
}

/** Remove a node; the root cannot be removed. */
export function removeNode(root, id) {
  const found = locate(root, id);
  if (!found?.parent) return false;
  found.parent.children.splice(found.parent.children.indexOf(found.node), 1);
  return id;
}

function cloneWithFreshIds(node) {
  const copy = clone(node);
  const renumber = (current) => {
    current.id = nextId();
    if (current.kind === "group") current.children.forEach(renumber);
  };
  renumber(copy);
  return copy;
}

/** Insert a deep copy of a node after it; returns the copy's id or `false`. */
export function duplicateNode(root, id) {
  const found = locate(root, id);
  if (!found?.parent) return false;
  const copy = cloneWithFreshIds(found.node);
  found.parent.children.splice(found.parent.children.indexOf(found.node) + 1, 0, copy);
  return copy.id;
}

/** Move a node by `delta` places among its siblings; returns its id or `false` at an end. */
export function moveNode(root, id, delta) {
  const found = locate(root, id);
  if (!found?.parent) return false;
  const siblings = found.parent.children;
  const from = siblings.indexOf(found.node);
  const to = from + delta;
  if (to < 0 || to >= siblings.length) return false;
  siblings.splice(from, 1);
  siblings.splice(to, 0, found.node);
  return id;
}

/** Switch a node off or on; the root group is always on. */
export function setDisabled(root, id, disabled) {
  const found = locate(root, id);
  if (!found?.parent) return false;
  found.node.disabled = Boolean(disabled);
  return id;
}

/** Invert a node; refused when its subtree would then nest too deep. */
export function setInverted(root, id, inverted) {
  const node = findNode(root, id);
  if (!node || (inverted && !node.inverted && !canInvert(root, id))) return false;
  node.inverted = Boolean(inverted);
  return id;
}

/** Set whether a group needs `all` or `any` of its children. */
export function setMatch(root, id, match) {
  const node = findNode(root, id);
  if (node?.kind !== "group" || !(match in OPERATORS)) return false;
  node.match = match;
  return id;
}

/** Set one parameter value of a condition. */
export function setParam(root, id, key, value) {
  const node = findNode(root, id);
  if (node?.kind !== "condition" || !(key in node.params)) return false;
  node.params[key] = value;
  return id;
}

const isUnset = (value) => value === null || value === undefined || value === "";

function paramProblems(node, schema) {
  const problems = [];
  for (const [key, spec] of Object.entries(schema.parameters ?? {})) {
    const value = node.params[key];
    if (NUMBER_TYPES.has(spec.type)) {
      if (isUnset(value) && spec.optional) continue;
      if (typeof value !== "number" || !Number.isFinite(value)) {
        problems.push({ nodeId: node.id, key, code: "required", args: {} });
      } else if (value < spec.min || value > spec.max) {
        problems.push({
          nodeId: node.id,
          key,
          code: "range",
          args: { min: spec.min, max: spec.max },
        });
      }
    } else if (spec.type === "enum" && Array.isArray(spec.options)) {
      if (isUnset(value) && spec.optional) continue;
      const listed = spec.options.some((option) => (option?.value ?? option) === value);
      if (!listed) problems.push({ nodeId: node.id, key, code: "invalid-option", args: {} });
    }
  }
  return problems;
}

/**
 * Everything that would stop a strategy from saving, in tree order. Switched-off rows are
 * checked too: the engine validates them as well.
 * @param {object} root Root group.
 * @param {object} schemas Condition schemas by type.
 * @param {string} strategyType `"ENTRY"` or `"EXIT"`.
 * @returns {{ nodeId: number, key?: string, code: string, args: object }[]}
 */
export function problemsOf(root, schemas, strategyType) {
  const problems = [];
  const visit = (node, level) => {
    const wrapped = level + (node.inverted ? 1 : 0);
    if (wrapped > MAX_DEPTH) {
      problems.push({ nodeId: node.id, code: "too-deep", args: { max: MAX_DEPTH } });
    }
    if (node.kind === "group") {
      const isRoot = node === root;
      if (!node.children.some((child) => !child.disabled)) {
        problems.push({ nodeId: node.id, code: isRoot ? "rules-empty" : "group-empty", args: {} });
      }
      for (const child of node.children) visit(child, wrapped + 1);
      return;
    }
    const schema = schemas?.[node.type];
    if (!schema) {
      problems.push({ nodeId: node.id, code: "unknown-condition", args: { type: node.type } });
      return;
    }
    if (Array.isArray(schema.applies_to) && !schema.applies_to.includes(strategyType)) {
      problems.push({ nodeId: node.id, code: "not-applicable", args: { type: node.type } });
    }
    problems.push(...paramProblems(node, schema));
  };
  visit(root, 1);
  return problems;
}
