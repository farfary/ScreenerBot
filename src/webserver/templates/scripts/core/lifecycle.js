// Page Lifecycle Registry - centralized init/activate/deactivate/dispose flows
const lifecycles = new Map();
const contexts = new Map();

const noop = () => {};

const defaultLifecycle = {
  init: noop,
  activate: noop,
  deactivate: noop,
  dispose: noop,
};

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
      this.onDeactivate(() => {
        if (typeof poller.stop === "function") {
          poller.stop({ silent: true });
        }
      });
      this.onDispose(() => {
        if (typeof poller.cleanup === "function") {
          poller.cleanup();
        }
      });
      return poller;
    },
    manageTabBar(tabBar) {
      if (!tabBar || typeof tabBar !== "object") {
        return tabBar;
      }
      this.onDeactivate(() => {
        if (typeof tabBar.hide === "function") {
          tabBar.hide({ silent: true });
        }
      });
      this.onDispose(() => {
        if (typeof tabBar.destroy === "function") {
          tabBar.destroy();
        }
      });
      return tabBar;
    },
    createAbortController() {
      const controller = new AbortController();
      const abort = () => {
        try {
          controller.abort();
        } catch (error) {
          console.error(
            "[PageLifecycle] Abort controller cleanup failed",
            error
          );
        }
      };
      this.onDeactivate(abort);
      this.onDispose(abort);
      return controller;
    },
    __setActive(value) {
      active = Boolean(value);
    },
    __runDeactivateCleanups() {
      if (!deactivateCleanups.size) {
        return;
      }
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

  init(pageName) {
    const lifecycle = getLifecycle(pageName);
    if (!lifecycle) {
      return;
    }
    const context = getOrCreateContext(pageName);
    if (context.__initialized) {
      return;
    }
    context.__initialized = true;
    try {
      lifecycle.init(context);
    } catch (error) {
      console.error(`[PageLifecycle] init failed for ${pageName}`, error);
    }
  },

  activate(pageName) {
    const lifecycle = getLifecycle(pageName);
    if (!lifecycle) {
      return;
    }
    const context = getOrCreateContext(pageName);
    this.init(pageName);
    context.__setActive(true);
    try {
      lifecycle.activate(context);
    } catch (error) {
      console.error(`[PageLifecycle] activate failed for ${pageName}`, error);
    }
  },

  deactivate(pageName) {
    const lifecycle = getLifecycle(pageName);
    if (!lifecycle) {
      return;
    }
    const context = getOrCreateContext(pageName);
    if (!context.__initialized) {
      return;
    }
    try {
      lifecycle.deactivate(context);
    } catch (error) {
      console.error(`[PageLifecycle] deactivate failed for ${pageName}`, error);
    } finally {
      context.__setActive(false);
      context.__runDeactivateCleanups();
    }
  },

  dispose(pageName) {
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
      lifecycle.dispose(context);
    } catch (error) {
      console.error(`[PageLifecycle] dispose failed for ${pageName}`, error);
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
