(function () {
  const global = window;

  function warnRealtimeDisabled(action) {
    if (global.__REALTIME_WARNED) {
      return;
    }
    global.__REALTIME_WARNED = true;
    console.warn(
      `[Realtime] WebSocket features are disabled; '${action}' is a no-op.`
    );
  }

  const realtime = {
    lastSnapshotRequestIds: {},
    isConnected() {
      return false;
    },
    requestSnapshotForAliases(aliases) {
      if (Array.isArray(aliases) && aliases.length > 0) {
        warnRealtimeDisabled("requestSnapshotForAliases");
      }
      return {};
    },
    updateFilters() {
      warnRealtimeDisabled("updateFilters");
      return {};
    },
    activate(pageName) {
      const config = global.PageRealtime?.[pageName];
      if (!config) {
        return;
      }

      if (typeof config.onInitial === "function") {
        try {
          config.onInitial("disabled");
        } catch (err) {
          console.warn("[Realtime] onInitial handler failed", err);
        }
      }

      if (typeof config.onUnavailable === "function") {
        try {
          config.onUnavailable();
        } catch (err) {
          console.warn("[Realtime] onUnavailable handler failed", err);
        }
      }
    },
    onHubConnected() {},
    onHubDisconnected() {},
    onHubAck() {},
  };

  global.Realtime = realtime;

  global.WsHub = {
    subscribe() {},
    unsubscribe() {},
    send() {
      warnRealtimeDisabled("send");
    },
    connect() {
      warnRealtimeDisabled("connect");
    },
    reconnect() {
      warnRealtimeDisabled("reconnect");
    },
    close() {},
    isConnected() {
      return false;
    },
    getStatus() {
      return "disabled";
    },
  };
})();
