(function () {
  const global = window;

  const aliasToTopic = {
    status: "system.status",
    services: "services.metrics",
    events: "events.new",
    positions: "positions.update",
    prices: "prices.update",
    tokens: "tokens.update",
    ohlcvs: "ohlcvs.update",
    trader: "trader.state",
    wallet: "wallet.balances",
    transactions: "transactions.activity",
    security: "security.alerts",
  };

  const topicToAlias = Object.entries(aliasToTopic).reduce(
    (acc, [alias, topic]) => {
      if (!acc[topic]) {
        acc[topic] = alias;
      }
      return acc;
    },
    {}
  );

  function resolveTopicFromAlias(alias) {
    return aliasToTopic[alias] || alias;
  }

  function resolveAliasFromTopic(topic) {
    return topicToAlias[topic] || topic;
  }

  function ensureClientId() {
    const key = "screenerbot.ws_client_id";
    try {
      const storage = global.localStorage;
      let id = storage.getItem(key);
      if (!id) {
        id = `ws-${Math.random()
          .toString(36)
          .slice(2, 11)}-${Date.now().toString(36)}`;
        storage.setItem(key, id);
      }
      return id;
    } catch (_) {
      return `ws-${Math.random()
        .toString(36)
        .slice(2, 11)}-${Date.now().toString(36)}`;
    }
  }

  function normalizeEnvelope(msg) {
    const topic = msg.t || msg.topic || "unknown";
    return {
      topic,
      alias: resolveAliasFromTopic(topic),
      timestamp: msg.ts ?? null,
      seq: msg.seq ?? null,
      key: msg.key ?? null,
      data: msg.data,
      meta: msg.meta ?? null,
      raw: msg,
    };
  }

  global.PageRealtime = global.PageRealtime || {};

  global.WsHub = {
    conn: null,
    enabled: true,
    attempts: 0,
    maxAttempts: 5,
    isConnecting: false,
    listeners: {},
    clientId: ensureClientId(),
    heartbeatTimer: null,
    protocolVersion: null,

    connect() {
      if (this.isConnecting) return;
      if (
        this.conn &&
        (this.conn.readyState === WebSocket.OPEN ||
          this.conn.readyState === WebSocket.CONNECTING)
      ) {
        return;
      }

      const proto = location.protocol === "https:" ? "wss" : "ws";
      const url = `${proto}://${location.host}/api/ws`;

      console.log("[WsHub] Connecting:", url);

      try {
        this.isConnecting = true;
        const socket = new WebSocket(url);
        this.conn = socket;

        socket.onopen = () => {
          console.log("[WsHub] Connected");
          this.attempts = 0;
          this.isConnecting = false;
          this.sendHello();
          this.emit("_connected", { status: "connected" });

          if (
            global.Realtime &&
            typeof global.Realtime.onHubConnected === "function"
          ) {
            global.Realtime.onHubConnected();
          }
        };

        socket.onmessage = (event) => {
          try {
            const msg = JSON.parse(event.data);
            this.handleMessage(msg);
          } catch (err) {
            console.error("[WsHub] Message parse error:", err);
          }
        };

        socket.onclose = (event) => {
          console.log("[WsHub] Closed");
          this.isConnecting = false;
          this.conn = null;
          this.stopHeartbeat();
          this.emit("_disconnected", {
            status: "disconnected",
            code: event?.code,
          });
          if (
            global.Realtime &&
            typeof global.Realtime.onHubDisconnected === "function"
          ) {
            global.Realtime.onHubDisconnected();
          }
          this.reconnect();
        };

        socket.onerror = (err) => {
          console.error("[WsHub] Error:", err);
          this.isConnecting = false;
          this.emit("_failed", { error: err?.message || "unknown" });
          try {
            socket.close();
          } catch (_) {}
        };
      } catch (err) {
        console.error("[WsHub] Creation failed:", err);
        this.isConnecting = false;
        this.reconnect();
      }
    },

    reconnect() {
      this.attempts += 1;
      const delay = Math.min(1000 * Math.pow(2, this.attempts), 15000);

      console.log(
        `[WsHub] Reconnect attempt ${this.attempts}, delay: ${delay}ms`
      );

      if (
        this.conn &&
        (this.conn.readyState === WebSocket.OPEN ||
          this.conn.readyState === WebSocket.CONNECTING ||
          this.isConnecting)
      ) {
        return;
      }

      if (this.maxAttempts > 0 && this.attempts > this.maxAttempts) {
        this.attempts = this.maxAttempts;
        this.emit("_warning", {
          alias: "ws",
          channel: "ws",
          topic: "system.status",
          message: "Realtime connection degraded, retrying",
          recommendation: "http_catchup",
        });
      }

      setTimeout(() => this.connect(), delay);
    },

    sendHello() {
      const payload = {
        type: "hello",
        client_id: this.clientId,
        app_version: global.APP_VERSION || null,
        pages_supported: Object.keys(global.PageRealtime || {}),
      };
      this.send(payload);
    },

    handleMessage(msg) {
      switch (msg.type) {
        case "data": {
          const envelope = normalizeEnvelope(msg);
          this.protocolVersion = envelope.raw?.v ?? this.protocolVersion;
          this.emit(envelope.alias, envelope.data, envelope);
          this.emit(`topic:${envelope.topic}`, envelope.data, envelope);
          break;
        }
        case "ack": {
          if (msg.context?.protocol_version) {
            this.protocolVersion = msg.context.protocol_version;
          }
          this.emit("_ack", msg, msg);
          if (
            global.Realtime &&
            typeof global.Realtime.onHubAck === "function"
          ) {
            global.Realtime.onHubAck(msg);
          }
          break;
        }
        case "error": {
          const alias = resolveAliasFromTopic(
            msg.topic || msg.channel || "unknown"
          );
          const payload = {
            ...msg,
            alias,
            channel: alias,
          };
          this.emit("_error", payload, payload);
          break;
        }
        case "warning":
        case "backpressure": {
          const alias = resolveAliasFromTopic(
            msg.topic || msg.channel || "unknown"
          );
          const payload = {
            ...msg,
            alias,
            channel: alias,
          };
          this.emit("_warning", payload, payload);
          this.emit(`warning:${msg.topic || alias}`, payload, payload);
          break;
        }
        case "snapshot_begin": {
          const alias = resolveAliasFromTopic(msg.topic);
          const payload = {
            ...msg,
            alias,
          };
          this.emit(`snapshot_begin:${msg.topic}`, payload, payload);
          this.emit("_snapshot_begin", payload, payload);
          break;
        }
        case "snapshot_end": {
          const alias = resolveAliasFromTopic(msg.topic);
          const payload = {
            ...msg,
            alias,
          };
          this.emit(`snapshot_end:${msg.topic}`, payload, payload);
          this.emit("_snapshot_end", payload, payload);
          break;
        }
        case "pong":
          break;
        default:
          console.warn("[WsHub] Unknown message type:", msg.type);
      }
    },

    subscribe(channel, callback) {
      if (!this.listeners[channel]) {
        this.listeners[channel] = [];
      }
      this.listeners[channel].push(callback);
    },

    unsubscribe(channel, callback) {
      const callbacks = this.listeners[channel];
      if (!callbacks) {
        return;
      }

      this.listeners[channel] = callbacks.filter((cb) => cb !== callback);
      if (this.listeners[channel].length === 0) {
        delete this.listeners[channel];
      }
    },

    emit(channel, data, context) {
      const callbacks = this.listeners[channel];
      if (!callbacks) {
        return;
      }

      for (const callback of callbacks) {
        try {
          callback(data, context);
        } catch (err) {
          console.error("[WsHub] Listener error:", err);
        }
      }
    },

    send(msg) {
      if (this.conn && this.conn.readyState === WebSocket.OPEN) {
        this.conn.send(JSON.stringify(msg));
      } else {
        console.warn("[WsHub] Not connected, cannot send:", msg?.type || msg);
      }
    },

    sendSetFilters(topics) {
      if (!topics || Object.keys(topics).length === 0) {
        return;
      }
      this.send({ type: "set_filters", topics });
    },

    sendResync(topics) {
      if (!topics || Object.keys(topics).length === 0) {
        return;
      }
      this.send({ type: "resync", topics });
    },

    startHeartbeat() {
      if (this.heartbeatTimer) {
        return;
      }
      this.heartbeatTimer = setInterval(() => {
        if (this.conn && this.conn.readyState === WebSocket.OPEN) {
          this.send({ type: "ping" });
        }
      }, 30000);
    },

    stopHeartbeat() {
      if (this.heartbeatTimer) {
        clearInterval(this.heartbeatTimer);
        this.heartbeatTimer = null;
      }
    },

    getStatus() {
      if (!this.conn) return "disconnected";

      switch (this.conn.readyState) {
        case WebSocket.CONNECTING:
          return "connecting";
        case WebSocket.OPEN:
          return "connected";
        case WebSocket.CLOSING:
          return "closing";
        case WebSocket.CLOSED:
          return "disconnected";
        default:
          return "unknown";
      }
    },

    isConnected() {
      return this.conn && this.conn.readyState === WebSocket.OPEN;
    },
  };

  function hasWsHub() {
    return (
      typeof global.WsHub !== "undefined" &&
      global.WsHub &&
      global.WsHub.enabled !== false
    );
  }

  const persistentRealtimeConfigs = [
    {
      alias: "status",
      handler: null,
      includeInFilters: () => true,
      getFilters: () => ({}),
    },
    {
      alias: "services",
      handler(data, envelope) {
        if (
          global.Realtime?.activePage === "services" &&
          global.PageRealtime?.services?.channels?.services
        ) {
          global.PageRealtime.services.channels.services(data, envelope);
        }
      },
      includeInFilters: () => global.Realtime?.activePage === "services",
      getFilters: () => ({}),
    },
    {
      alias: "events",
      handler(data, envelope) {
        if (global.PageRealtime?.events?.channels?.events) {
          global.PageRealtime.events.channels.events(data, envelope);
        }
      },
      includeInFilters: () => global.Realtime?.activePage === "events",
      getFilters: () => ({}),
    },
    {
      alias: "positions",
      handler(data, envelope) {
        if (global.PageRealtime?.positions?.channels?.positions) {
          global.PageRealtime.positions.channels.positions(data, envelope);
        }
      },
      includeInFilters: () => global.Realtime?.activePage === "positions",
      getFilters: () => ({}),
    },
    {
      alias: "prices",
      handler(data, envelope) {
        if (global.PageRealtime?.tokens?.channels?.prices) {
          global.PageRealtime.tokens.channels.prices(data, envelope);
        }
      },
      includeInFilters: () => global.Realtime?.activePage === "tokens",
      getFilters: () => ({}),
    },
  ];

  const globalSubscriptions = [];
  const persistentFilterProviders = new Map();
  const persistentIncludeChecks = new Map();

  function initializePersistentSubscriptions() {
    if (!hasWsHub() || global.__persistentSubsInitialized) {
      return;
    }

    for (const config of persistentRealtimeConfigs) {
      if (typeof config.handler === "function") {
        global.WsHub.subscribe(config.alias, config.handler);
      }
      if (typeof config.getFilters === "function") {
        persistentFilterProviders.set(config.alias, config.getFilters);
      } else {
        persistentFilterProviders.set(config.alias, () => ({}));
      }
      if (typeof config.includeInFilters === "function") {
        persistentIncludeChecks.set(config.alias, config.includeInFilters);
      } else {
        persistentIncludeChecks.set(config.alias, () => true);
      }
    }

    global.__persistentSubsInitialized = true;
    console.log("[Realtime] Persistent subscriptions initialized");
  }

  const realtime = {
    activePage: null,
    activeConfig: null,
    hasInitialized: false,
    snapshotRequestAliases: new Set(),
    pendingFilterUpdate: false,
    lastSentFilters: null,
    pendingTopics: null,
    pausedAliases: new Set(),

    ensureHubInitialized() {
      if (!hasWsHub()) {
        return;
      }

      if (!global.__wsHubInitialized) {
        global.__wsHubInitialized = true;
        global.WsHub.connect();
        global.WsHub.startHeartbeat();
      } else if (!global.WsHub.isConnected() && !global.WsHub.isConnecting) {
        global.WsHub.connect();
      }

      initializePersistentSubscriptions();
    },

    initializeOnce() {
      if (this.hasInitialized) {
        return;
      }
      this.hasInitialized = true;
      this.setupGlobalStatusTelemetry();
    },

    setupGlobalStatusTelemetry() {
      if (!hasWsHub()) {
        if (typeof global.setWsBadge === "function") {
          global.setWsBadge("disconnected", "🔌 N/A");
        }
        return;
      }

      const renderSnapshot =
        typeof global.renderStatusBadgesFromSnapshot === "function"
          ? global.renderStatusBadgesFromSnapshot
          : null;
      const stopPolling =
        typeof global.stopStatusPolling === "function"
          ? global.stopStatusPolling
          : null;
      const startPolling =
        typeof global.startStatusPolling === "function"
          ? global.startStatusPolling
          : null;
      const fetchSnapshot =
        typeof global.fetchStatusSnapshot === "function"
          ? global.fetchStatusSnapshot
          : null;

      const handleStatusUpdate = (snapshot) => {
        if (!snapshot) return;
        if (stopPolling) stopPolling();
        if (renderSnapshot) renderSnapshot(snapshot);
      };

      const handleStatusDisconnect = () => {
        if (typeof global.setWsBadge === "function") {
          global.setWsBadge("disconnected", "🔌 Offline");
        }
        if (startPolling) startPolling();
      };

      const handleStatusReconnect = () => {
        if (typeof global.setWsBadge === "function") {
          global.setWsBadge("connected", "🔌 Connected");
        }
        if (stopPolling) stopPolling();
        if (fetchSnapshot) fetchSnapshot();
      };

      this.addGlobalSubscription("status", handleStatusUpdate);
      this.addGlobalSubscription("_disconnected", handleStatusDisconnect);
      this.addGlobalSubscription("_failed", handleStatusDisconnect);
      this.addGlobalSubscription("_connected", handleStatusReconnect);

      if (typeof global.setWsBadge === "function") {
        const status =
          typeof global.WsHub.getStatus === "function"
            ? global.WsHub.getStatus()
            : "unknown";
        if (status === "connected") {
          global.setWsBadge("connected", "🔌 Connected");
        } else if (status === "connecting") {
          global.setWsBadge("connecting", "🔌 Connecting");
        } else {
          global.setWsBadge("disconnected", "🔌 Offline");
        }
      }
    },

    addGlobalSubscription(channel, handler) {
      if (!hasWsHub()) {
        return;
      }
      global.WsHub.subscribe(channel, handler);
      globalSubscriptions.push({ channel, handler });
    },

    getFiltersForAlias(alias) {
      const filters = {};

      const persistentProvider = persistentFilterProviders.get(alias);
      if (typeof persistentProvider === "function") {
        const value = persistentProvider();
        if (value && typeof value === "object" && !Array.isArray(value)) {
          Object.assign(filters, value);
        }
      }

      if (
        this.activeConfig &&
        typeof this.activeConfig.getFilters === "function"
      ) {
        const activeFilters = this.activeConfig.getFilters() || {};
        if (activeFilters && typeof activeFilters === "object") {
          if (
            activeFilters[alias] &&
            typeof activeFilters[alias] === "object"
          ) {
            Object.assign(filters, activeFilters[alias]);
          } else {
            const topicKey = resolveTopicFromAlias(alias);
            if (
              activeFilters[topicKey] &&
              typeof activeFilters[topicKey] === "object"
            ) {
              Object.assign(filters, activeFilters[topicKey]);
            }
          }
        }
      }

      return filters;
    },

    getActiveAliases() {
      const aliases = new Set();

      for (const config of persistentRealtimeConfigs) {
        const includeFn = persistentIncludeChecks.get(config.alias);
        if (includeFn && !includeFn()) {
          continue;
        }
        aliases.add(config.alias);
      }

      if (this.activeConfig && Array.isArray(this.activeConfig.topics)) {
        for (const alias of this.activeConfig.topics) {
          aliases.add(alias);
        }
      }

      return aliases;
    },

    collectFilters(snapshotAliases) {
      const topics = {};

      const applyAlias = (alias) => {
        const topic = resolveTopicFromAlias(alias);
        const filters = this.getFiltersForAlias(alias);
        const existing = topics[topic] || {};
        topics[topic] = { ...existing, ...filters };
        if (snapshotAliases.has(alias)) {
          topics[topic] = { ...topics[topic], snapshot: true };
        }
      };

      for (const config of persistentRealtimeConfigs) {
        const includeFn = persistentIncludeChecks.get(config.alias);
        if (includeFn && !includeFn()) {
          continue;
        }
        applyAlias(config.alias);
      }

      if (this.activeConfig && Array.isArray(this.activeConfig.topics)) {
        for (const alias of this.activeConfig.topics) {
          applyAlias(alias);
        }
      }

      return topics;
    },

    updateFilters(options = {}) {
      if (options.snapshotTopics) {
        for (const alias of options.snapshotTopics) {
          if (alias) {
            this.snapshotRequestAliases.add(alias);
          }
        }
      }

      const snapshotAliases = new Set(this.snapshotRequestAliases);
      const topics = this.collectFilters(snapshotAliases);

      if (Object.keys(topics).length === 0) {
        this.lastSentFilters = null;
        this.pendingFilterUpdate = false;
        this.pendingTopics = null;
        return;
      }

      if (!hasWsHub()) {
        this.pendingFilterUpdate = true;
        this.pendingTopics = topics;
        return;
      }

      if (!global.WsHub.isConnected()) {
        this.pendingFilterUpdate = true;
        this.pendingTopics = topics;
        return;
      }

      global.WsHub.sendSetFilters(topics);
      this.lastSentFilters = topics;
      this.pendingFilterUpdate = false;
      this.pendingTopics = null;
      this.snapshotRequestAliases.clear();
    },

    requestSnapshotForAliases(aliases) {
      if (!Array.isArray(aliases) || aliases.length === 0) {
        return;
      }
      for (const alias of aliases) {
        if (alias) {
          this.snapshotRequestAliases.add(alias);
        }
      }
      this.updateFilters();
    },

    activate(pageName) {
      if (!pageName) {
        return;
      }

      const prevPage = this.activePage;
      if (
        prevPage &&
        this.activeConfig &&
        typeof this.activeConfig.onExit === "function"
      ) {
        try {
          this.activeConfig.onExit();
        } catch (err) {
          console.error("[Realtime] onExit handler failed:", err);
        }
      }

      this.activePage = pageName;
      const config = global.PageRealtime
        ? global.PageRealtime[pageName]
        : undefined;
      this.activeConfig = config || null;

      if (!config) {
        console.log(
          `[Realtime] Activated page: ${pageName} (no config, was: ${
            prevPage || "none"
          })`
        );
        this.updateFilters();
        return;
      }

      const status = hasWsHub() ? global.WsHub.getStatus() : "disconnected";

      if (typeof config.onInitial === "function") {
        try {
          config.onInitial(status);
        } catch (err) {
          console.error("[Realtime] onInitial handler failed:", err);
        }
      }

      if (typeof config.onEnter === "function") {
        try {
          config.onEnter(status);
        } catch (err) {
          console.error("[Realtime] onEnter handler failed:", err);
        }
      }

      if (Array.isArray(config.topics) && config.topics.length > 0) {
        this.requestSnapshotForAliases(config.topics);
      } else {
        this.updateFilters();
      }

      console.log(
        `[Realtime] Activated page: ${pageName} (was: ${prevPage || "none"})`
      );
    },

    deactivateCurrent() {
      if (this.activeConfig && typeof this.activeConfig.onExit === "function") {
        try {
          this.activeConfig.onExit();
        } catch (err) {
          console.error("[Realtime] onExit handler failed:", err);
        }
      }

      this.activeConfig = null;
      this.activePage = null;
    },

    onHubConnected() {
      if (this.pendingFilterUpdate && this.pendingTopics) {
        global.WsHub.sendSetFilters(this.pendingTopics);
        this.lastSentFilters = this.pendingTopics;
        this.pendingFilterUpdate = false;
        this.pendingTopics = null;
        this.snapshotRequestAliases.clear();
      } else {
        const aliases = Array.from(this.getActiveAliases());
        if (aliases.length > 0) {
          this.requestSnapshotForAliases(aliases);
        } else if (this.lastSentFilters) {
          global.WsHub.sendSetFilters(this.lastSentFilters);
        }
      }

      if (this.pausedAliases.size > 0) {
        const topics = Array.from(this.pausedAliases)
          .map((alias) => resolveTopicFromAlias(alias))
          .filter(Boolean);
        if (topics.length > 0) {
          global.WsHub.send({ type: "pause", topics });
        }
      }
    },

    onHubDisconnected() {
      this.pendingFilterUpdate = true;
      if (this.lastSentFilters) {
        this.pendingTopics = this.lastSentFilters;
      }
    },

    onHubAck() {
      // Reserved for future protocol negotiation
    },

    isAliasPaused(alias) {
      if (!alias) {
        return false;
      }
      return this.pausedAliases.has(alias);
    },

    setAliasPaused(alias, paused) {
      if (!alias) {
        return false;
      }

      const topic = resolveTopicFromAlias(alias);
      if (!topic) {
        console.warn("[Realtime] Unknown topic alias for pause:", alias);
        return false;
      }

      if (paused) {
        this.pausedAliases.add(alias);
      } else {
        this.pausedAliases.delete(alias);
      }

      if (!hasWsHub()) {
        return true;
      }

      const payload = {
        type: paused ? "pause" : "resume",
        topics: [topic],
      };

      if (global.WsHub.isConnected()) {
        global.WsHub.send(payload);
      }

      return true;
    },

    isEventsPaused(alias) {
      if (typeof alias !== "string") {
        return this.isAliasPaused("events");
      }
      return this.isAliasPaused(alias);
    },

    setEventsPaused(aliasOrPaused, maybePaused) {
      if (typeof maybePaused === "undefined") {
        return this.setAliasPaused("events", Boolean(aliasOrPaused));
      }
      return this.setAliasPaused(aliasOrPaused, Boolean(maybePaused));
    },
  };

  document.addEventListener("DOMContentLoaded", () => {
    realtime.ensureHubInitialized();
    realtime.initializeOnce();

    const initialPage =
      (global.Router && Router.currentPage) ||
      (global.location
        ? global.location.pathname === "/"
          ? "home"
          : global.location.pathname.replace(/^\//, "")
        : null);

    if (initialPage) {
      realtime.activate(initialPage);
    }
  });

  window.addEventListener("beforeunload", () => {
    realtime.deactivateCurrent();

    if (hasWsHub()) {
      const subs = globalSubscriptions.splice(0);
      for (const sub of subs) {
        try {
          global.WsHub.unsubscribe(sub.channel, sub.handler);
        } catch (err) {
          console.warn(
            "[Realtime] Failed to clean global subscription",
            sub.channel,
            err
          );
        }
      }
    }
  });

  global.Realtime = realtime;
})();
