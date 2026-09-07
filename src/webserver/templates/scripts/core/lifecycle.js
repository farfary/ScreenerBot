// Page Lifecycle Registry - centralized init/activate/deactivate/dispose flows
import { closeAllMenus } from "./menu_manager.js";

const lifecycles = new Map();
const contexts = new Map();
let visibilityHandlerAdded = false;
const activePollers = new Set();

const noop = () => {};

const defaultLifecycle = {
  init: noop,
  activate: noop,
  deactivate: noop,
  dispose: noop,
};

async function runHook(phase, pageName, hook, context) {
  if (typeof hook !== "function") {
    return;
  }
  try {
    await hook(context);
  } catch (error) {
    console.error(`[PageLifecycle] ${phase} failed for ${pageName}`, error);
    throw error;
  }
}

const safeInvokeAll = (callbacks) => {
  callbacks.forEach((callback) => {
    try {
      callback();
    } catch (error) {
      console.error("[PageLifecycle] Cleanup handler failed", error);
    }
  });
};

const createContext = (pageName) => {
  const deactivateCleanups = new Set();
  const disposeCleanups = new Set();
  // Managed resources belong to the page context for its entire cached
  // lifetime. Keep iterable identity sets so every deactivation can stop/hide
  // them centrally; callers cannot accidentally lose cleanup by forgetting to
  // re-register an object on a later activation.
  const managedPollers = new Set();
  const managedTabBars = new Set();
  const managedActionBars = new Set();
  const abortControllerReleases = new WeakMap();
  let active = false;

  const register = (store, callback) => {
    if (typeof callback !== "function") {
      return () => {};
    }
    store.add(callback);
    return () => store.delete(callback);
  };

  const context = {
    pageName,
    data: {},
    isActive: () => active,
    __initPromise: null,
    onDeactivate(callback) {
      return register(deactivateCleanups, callback);
    },
    onDispose(callback) {
      return register(disposeCleanups, callback);
    },
    managePoller(poller) {
      if (!poller || typeof poller !== "object") {
        return poller;
      }

      if (!managedPollers.has(poller)) {
        managedPollers.add(poller);
        activePollers.add(poller);
        this.onDispose(() => {
          managedPollers.delete(poller);
          activePollers.delete(poller);
          if (typeof poller.cleanup === "function") {
            poller.cleanup();
          }
        });
      }
      return poller;
    },
    manageTabBar(tabBar) {
      if (!tabBar || typeof tabBar !== "object") {
        return tabBar;
      }
      if (!managedTabBars.has(tabBar)) {
        managedTabBars.add(tabBar);
        this.onDispose(() => {
          managedTabBars.delete(tabBar);
          if (typeof tabBar.destroy === "function") {
            tabBar.destroy();
          }
        });
      }
      return tabBar;
    },
    manageActionBar(actionBar) {
      if (!actionBar || typeof actionBar !== "object") {
        return actionBar;
      }
      if (!managedActionBars.has(actionBar)) {
        managedActionBars.add(actionBar);
        this.onDispose(() => {
          managedActionBars.delete(actionBar);
          if (typeof actionBar.dispose === "function") {
            actionBar.dispose();
          }
        });
      }
      return actionBar;
    },
    createAbortController() {
      const controller = new AbortController();
      let release = () => {};
      const abort = () => {
        try {
          controller.abort();
        } catch (error) {
          console.error("[PageLifecycle] Abort controller cleanup failed", error);
        } finally {
          release();
        }
      };
      const unregisterDeactivate = this.onDeactivate(abort);
      const unregisterDispose = this.onDispose(abort);
      release = () => {
        unregisterDeactivate();
        unregisterDispose();
        abortControllerReleases.delete(controller);
      };
      abortControllerReleases.set(controller, release);
      return controller;
    },
    releaseAbortController(controller) {
      abortControllerReleases.get(controller)?.();
    },
    __setActive(value) {
      active = Boolean(value);
    },
    __runDeactivateCleanups() {
      safeInvokeAll(Array.from(managedPollers, (poller) => () => poller.stop?.({ silent: true })));
      safeInvokeAll(Array.from(managedTabBars, (tabBar) => () => tabBar.hide?.({ silent: true })));
      safeInvokeAll(Array.from(managedActionBars, (actionBar) => () => actionBar.clear?.()));

      const callbacks = Array.from(deactivateCleanups);
      deactivateCleanups.clear();
      safeInvokeAll(callbacks);
    },
    __runDisposeCleanups() {
      if (!disposeCleanups.size) {
        return;
      }
      const callbacks = Array.from(disposeCleanups);
      disposeCleanups.clear();
      safeInvokeAll(callbacks);
    },
  };

  return context;
};

// Setup global visibility change handler for all pollers
const setupVisibilityHandler = () => {
  if (visibilityHandlerAdded) {
    return;
  }

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      // Pause all active pollers when tab hidden
      activePollers.forEach((poller) => {
        if (poller.isActive && poller.isActive() && typeof poller.pause === "function") {
          poller.pause();
        }
      });
    } else {
      // Resume all active pollers when tab visible
      activePollers.forEach((poller) => {
        if (poller.isActive && poller.isActive() && typeof poller.resume === "function") {
          poller.resume();
        }
      });
    }
  });

  visibilityHandlerAdded = true;
  console.log("[PageLifecycle] Global visibility handler initialized");
};

// Initialize visibility handler immediately
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", setupVisibilityHandler);
} else {
  setupVisibilityHandler();
}

const getLifecycle = (pageName) => {
  const lifecycle = lifecycles.get(pageName);
  if (!lifecycle) {
    return null;
  }
  return {
    init: lifecycle.init || noop,
    activate: lifecycle.activate || noop,
    deactivate: lifecycle.deactivate || noop,
    dispose: lifecycle.dispose || noop,
  };
};

const getOrCreateContext = (pageName) => {
  if (!contexts.has(pageName)) {
    contexts.set(pageName, createContext(pageName));
  }
  return contexts.get(pageName);
};

export const PageLifecycleRegistry = {
  register(pageName, lifecycle = {}) {
    if (typeof pageName !== "string" || !pageName.trim()) {
      throw new Error("PageLifecycleRegistry.register requires a page id");
    }
    const normalized = {
      ...defaultLifecycle,
      ...lifecycle,
    };
    lifecycles.set(pageName, normalized);
  },

  has(pageName) {
    return lifecycles.has(pageName);
  },

  getContext(pageName) {
    return getOrCreateContext(pageName);
  },

  async init(pageName) {
    const lifecycle = getLifecycle(pageName);
    if (!lifecycle) {
      return;
    }
    const context = getOrCreateContext(pageName);
    if (context.__initialized) {
      return;
    }
    if (context.__initPromise) {
      return context.__initPromise;
    }
    context.__initPromise = (async () => {
      await runHook("init", pageName, lifecycle.init, context);
      context.__initialized = true;
    })().finally(() => {
      context.__initPromise = null;
    });
    return context.__initPromise;
  },

  async activate(pageName) {
    const lifecycle = getLifecycle(pageName);
    if (!lifecycle) {
      return;
    }
    const context = getOrCreateContext(pageName);
    await this.init(pageName);
    context.__setActive(true);
    await runHook("activate", pageName, lifecycle.activate, context);
  },

  async deactivate(pageName) {
    const lifecycle = getLifecycle(pageName);
    if (!lifecycle) {
      return;
    }
    const context = getOrCreateContext(pageName);
    if (!context.__initialized || !context.isActive()) {
      return;
    }
    // Leaving a page must never leave a stray dropdown/menu floating over the next.
    closeAllMenus("navigation");
    context.__setActive(false);
    try {
      await runHook("deactivate", pageName, lifecycle.deactivate, context);
    } finally {
      context.__runDeactivateCleanups();
    }
  },

  async dispose(pageName) {
    const lifecycle = getLifecycle(pageName);
    if (!lifecycle) {
      return;
    }
    const context = getOrCreateContext(pageName);
    if (!context.__initialized) {
      lifecycles.delete(pageName);
      contexts.delete(pageName);
      return;
    }
    try {
      await runHook("dispose", pageName, lifecycle.dispose, context);
    } finally {
      context.__runDeactivateCleanups();
      context.__runDisposeCleanups();
      lifecycles.delete(pageName);
      contexts.delete(pageName);
    }
  },
};

export function registerPage(pageName, lifecycle) {
  PageLifecycleRegistry.register(pageName, lifecycle);
}
