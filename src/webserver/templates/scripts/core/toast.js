/**
 * Toast manager — the dashboard's transient notice layer.
 *
 * ONE NOTICE PER SUBJECT. Every toast has a `key`; showing a toast whose key is
 * already on screen UPDATES that toast in place instead of stacking a second
 * one. When no key is given the key is derived from the type and title, so the
 * same notice fired repeatedly (a copy button pressed five times, one error per
 * failed poll) collapses into a single toast with a repeat count. This is the
 * whole reason the layer exists in this shape: a swap used to raise three
 * separate toasts that piled up on each other.
 *
 * The persistent record of anything that happened lives in the notification
 * center (`core/notifications.js`), NOT here. A toast is a transient nudge for
 * work the user is waiting on or an outcome they cannot otherwise see.
 */

import { Toast } from "../ui/toast.js";

const MAX_VISIBLE = 3;
const MAX_QUEUED = 8;

/** A `progress` toast is dismissed by its owner; the rest time out. */
const DURATIONS = {
  success: 3500,
  info: 3500,
  warning: 6000,
  error: 9000,
  progress: 0,
};

const ICONS = {
  success: "icon-circle-check",
  error: "icon-circle-x",
  warning: "icon-triangle-alert",
  info: "icon-info",
  progress: "icon-loader-circle",
};

/**
 * A `progress` toast waits on a backend operation. If that operation never
 * resolves (a hung swap, a dropped SSE stream) the toast would sit on screen
 * forever claiming work is in flight, so it degrades to an honest warning.
 */
const PROGRESS_MAX_MS = 120000;
const STALLED_DURATION_MS = 8000;

const EXIT_ANIMATION_MS = 220;

function normalize(config) {
  const type = ICONS[config?.type] ? config.type : "info";
  const title = String(config?.title ?? "").trim();
  const message = config?.message ? String(config.message) : null;

  return {
    type,
    title,
    message,
    icon: ICONS[type],
    key: config?.key ? String(config.key) : `${type}:${title}`,
    // An explicit key means "this is the same subject, update it". Without one
    // the key IS the content, so a second identical call is a repeat.
    keyed: Boolean(config?.key),
    duration: Number.isFinite(config?.duration) ? config.duration : DURATIONS[type],
    progress: Number.isFinite(config?.progress)
      ? Math.min(100, Math.max(0, config.progress))
      : null,
  };
}

class ToastManager {
  constructor() {
    this.entries = new Map(); // key -> entry
    this.visible = []; // keys, in display order
    this.queued = []; // keys waiting for a slot
    this.container = null;
  }

  /**
   * Show a toast, or update the one already showing under the same key.
   * @param {{type?:string,title:string,message?:string,key?:string,duration?:number,progress?:number}} config
   * @returns {{key:string,update:Function,dismiss:Function}} handle
   */
  show(config) {
    const next = normalize(config);
    const existing = this.entries.get(next.key);

    if (existing) {
      // Identical content re-fired under a derived key: count it rather than
      // repeat it. An explicitly keyed notice is an update, never a repeat.
      existing.repeat = next.keyed ? 1 : existing.repeat + 1;
      this._apply(existing, next);
      return this._handle(next.key);
    }

    const entry = {
      key: next.key,
      config: next,
      repeat: 1,
      view: null,
      timer: null,
      stallTimer: null,
      paused: false,
      remaining: 0,
      startedAt: 0,
    };
    this.entries.set(next.key, entry);

    if (this.visible.length < MAX_VISIBLE) {
      this._mount(entry);
    } else {
      this.queued.push(next.key);
      // A burst of notices must not build an unbounded backlog that keeps
      // popping long after the user has moved on: drop the oldest waiter.
      while (this.queued.length > MAX_QUEUED) {
        const dropped = this.queued.shift();
        this.entries.delete(dropped);
      }
    }

    return this._handle(next.key);
  }

  /**
   * Update a live toast in place. Unknown keys are ignored — the owner of a
   * long-running notice does not have to track whether the user dismissed it.
   */
  update(key, patch) {
    const entry = this.entries.get(key);
    if (!entry) return;
    this._apply(entry, normalize({ ...entry.config, ...patch, key }));
  }

  /** Dismiss a toast by key. */
  dismiss(key) {
    const entry = this.entries.get(key);
    if (!entry) return;

    this._clearTimers(entry);
    this.entries.delete(key);

    const queuedIndex = this.queued.indexOf(key);
    if (queuedIndex !== -1) {
      this.queued.splice(queuedIndex, 1);
      return;
    }

    const visibleIndex = this.visible.indexOf(key);
    if (visibleIndex !== -1) {
      this.visible.splice(visibleIndex, 1);
    }

    const view = entry.view;
    if (!view) {
      this._promoteQueued();
      return;
    }

    view.startExit();
    setTimeout(() => {
      view.destroy();
      this._promoteQueued();
    }, EXIT_ANIMATION_MS);
  }

  _handle(key) {
    return {
      key,
      update: (patch) => this.update(key, patch),
      dismiss: () => this.dismiss(key),
    };
  }

  _apply(entry, config) {
    entry.config = config;

    if (entry.view) {
      entry.view.update(config, entry.repeat);
      this._arm(entry);
    }
  }

  _getContainer() {
    if (!this.container) {
      this.container = document.createElement("div");
      this.container.className = "toast-container";
      this.container.setAttribute("role", "region");
      this.container.setAttribute("aria-label", "Notifications");
      document.body.appendChild(this.container);
    }
    return this.container;
  }

  _mount(entry) {
    entry.view = new Toast(entry.config, entry.repeat, {
      onClose: () => this.dismiss(entry.key),
      onHoverChange: (hovered) => this._onHoverChange(entry, hovered),
    });
    this._getContainer().appendChild(entry.view.element);
    this.visible.push(entry.key);
    entry.view.enter();
    this._arm(entry);
  }

  _promoteQueued() {
    while (this.queued.length > 0 && this.visible.length < MAX_VISIBLE) {
      const key = this.queued.shift();
      const entry = this.entries.get(key);
      if (entry) this._mount(entry);
    }
  }

  /** (Re)start whichever timer this toast's type calls for. */
  _arm(entry) {
    this._clearTimers(entry);

    if (entry.config.type === "progress") {
      entry.stallTimer = setTimeout(() => {
        this._apply(
          entry,
          normalize({
            ...entry.config,
            type: "warning",
            message: "Still running — check the notification center",
            duration: STALLED_DURATION_MS,
            progress: null,
          })
        );
      }, PROGRESS_MAX_MS);
      return;
    }

    if (entry.config.duration > 0) {
      entry.paused = false;
      entry.remaining = entry.config.duration;
      entry.startedAt = Date.now();
      entry.timer = setTimeout(() => this.dismiss(entry.key), entry.remaining);
    }
  }

  /** Reading a toast must not race its own timer. */
  _onHoverChange(entry, hovered) {
    if (hovered) {
      if (!entry.timer) return;
      clearTimeout(entry.timer);
      entry.timer = null;
      entry.paused = true;
      entry.remaining = Math.max(0, entry.remaining - (Date.now() - entry.startedAt));
      return;
    }

    if (!entry.paused) return;
    entry.paused = false;

    // The toast may have been dismissed, or re-armed by an update, while the
    // pointer rested on it.
    if (!this.entries.has(entry.key) || entry.timer) return;

    entry.startedAt = Date.now();
    entry.timer = setTimeout(() => this.dismiss(entry.key), Math.max(entry.remaining, 600));
  }

  _clearTimers(entry) {
    if (entry.timer) {
      clearTimeout(entry.timer);
      entry.timer = null;
    }
    if (entry.stallTimer) {
      clearTimeout(entry.stallTimer);
      entry.stallTimer = null;
    }
  }
}

export const toastManager = new ToastManager();
export { ToastManager };
