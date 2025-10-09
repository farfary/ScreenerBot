(function () {
  const global = window;
  // Enable verbose realtime logging by setting localStorage.debugRealtime = '1'
  if (typeof global.__DEBUG_REALTIME === "undefined") {
    try {
      global.__DEBUG_REALTIME =
        (global.localStorage &&
          global.localStorage.getItem("debugRealtime") === "1") ||
        false;
    } catch (_) {
      global.__DEBUG_REALTIME = false;
    }
  }

  function dbg(...args) {
    if (global.__DEBUG_REALTIME) {
      try {
        console.log("[RealtimeDBG]", ...args);
      } catch (_) {}
    }
  }

  const aliasToTopic = {
    status: "system.status",
    services: "services.metrics",
    events: "events.new",
    positions: "positions.update",
    tokens: "tokens.update",
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

  if (typeof global.__statusTelemetryRequired === "undefined") {
    global.__statusTelemetryRequired = false;
  }

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

  // CRITICAL: Only create WsHub once - reuse across page navigations to avoid connection leaks
  if (!global.WsHub) {
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
      connectTimeoutTimer: null,
      watchdogTimer: null,
      lastPongAt: 0,
      lastPingAt: 0,
      lastPingId: 0,
      rttMs: null,
      pingSeq: 0,
    };
  }

  // Ensure WsHub methods exist (in case they were lost during hot reload or page nav)
  Object.assign(global.WsHub, {
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

        // Guard against a hung CONNECTING state by enforcing a timeout
        if (this.connectTimeoutTimer) {
          clearTimeout(this.connectTimeoutTimer);
        }
        this.connectTimeoutTimer = setTimeout(() => {
          if (
            this.conn === socket &&
            socket &&
            socket.readyState === WebSocket.CONNECTING
          ) {
            try {
              console.warn("[WsHub] Connect timeout, forcing close");
              socket.close();
            } catch (_) {}
          }
        }, 10000);

        socket.onopen = () => {
          console.log("[WsHub] Connected");
          this.attempts = 0;
          this.isConnecting = false;
          if (this.connectTimeoutTimer) {
            clearTimeout(this.connectTimeoutTimer);
            this.connectTimeoutTimer = null;
          }
          // Ensure heartbeat restarts on every successful (re)connect
          this.startHeartbeat();
          this.startWatchdog();
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
          if (this.connectTimeoutTimer) {
            clearTimeout(this.connectTimeoutTimer);
            this.connectTimeoutTimer = null;
          }
          this.stopHeartbeat();
          this.stopWatchdog();
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
          if (this.connectTimeoutTimer) {
            clearTimeout(this.connectTimeoutTimer);
            this.connectTimeoutTimer = null;
          }
          this.stopWatchdog();
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
      const base = Math.min(1000 * Math.pow(2, this.attempts), 15000);
      // Add jitter (0.5x–1.5x) to avoid thundering herd
      const jitter = 0.5 + Math.random();
      const delay = Math.floor(base * jitter);

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
      dbg("sendHello", payload);
      this.send(payload);
    },

    handleMessage(msg) {
      switch (msg.type) {
        case "data": {
          const envelope = normalizeEnvelope(msg);
          this.protocolVersion = envelope.raw?.v ?? this.protocolVersion;
          // Note: Per-item logging removed to avoid console spam
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
        case "pong": {
          // Update lastPongAt and compute RTT if possible
          const now = Date.now();
          this.lastPongAt = now;
          const pongId = typeof msg.id === "number" ? msg.id : null;
          if (
            pongId !== null &&
            pongId === this.lastPingId &&
            this.lastPingAt
          ) {
            this.rttMs = Math.max(0, now - this.lastPingAt);
          }
          this.emit("_pong", { rttMs: this.rttMs, ts: now }, msg);
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
          dbg("recv:snapshot_begin", payload);
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
          dbg("recv:snapshot_end", payload);
          this.emit(`snapshot_end:${msg.topic}`, payload, payload);
          this.emit("_snapshot_end", payload, payload);
          break;
        }
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
        dbg("ws:send", msg);
        this.conn.send(JSON.stringify(msg));
      } else {
        console.warn("[WsHub] Not connected, cannot send:", msg?.type || msg);
      }
    },

    sendSetFilters(topics) {
      if (!topics || typeof topics !== "object") {
        return;
      }
      // Allow empty object to indicate clearing filters on the server
      dbg("ws:set_filters", topics);
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
          this.sendPing();
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

    sendPing() {
      this.pingSeq = (this.pingSeq + 1) >>> 0; // uint wrap
      this.lastPingId = this.pingSeq;
      this.lastPingAt = Date.now();
      // Server expects a string ID; send as string to avoid schema errors
      this.send({ type: "ping", id: String(this.lastPingId) });
    },

    startWatchdog() {
      if (this.watchdogTimer) return;
      // Check every 10s; if no pong for >75s, consider degraded and reconnect
      const thresholdMs = 75_000;
      this.watchdogTimer = setInterval(() => {
        if (!this.isConnected()) return;
        const now = Date.now();
        const last = this.lastPongAt || 0;
        if (last > 0 && now - last > thresholdMs) {
          this.emit("_warning", {
            alias: "ws",
            channel: "ws",
            topic: "system.status",
            message: "No pong received; reconnecting",
            recommendation: "http_catchup",
          });
          try {
            this.conn.close();
          } catch (_) {}
        }
      }, 10_000);
    },

    stopWatchdog() {
      if (this.watchdogTimer) {
        clearInterval(this.watchdogTimer);
        this.watchdogTimer = null;
      }
    },
  });

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
      includeInFilters: () => global.__statusTelemetryRequired === true,
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
    activeSubscriptions: [],
    snapshotContextsByTopic: new Map(),
    snapshotContextsByAlias: new Map(),
    lastSnapshotRequestIds: {},
    snapshotListenersBound: false,

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
      this.bindSnapshotLifecycleListeners();
    },

    setupGlobalStatusTelemetry() {
      if (!hasWsHub()) {
        if (typeof global.setWsBadge === "function") {
          global.setWsBadge("disconnected", "🔌 N/A");
        }
        return;
      }

      global.__statusTelemetryRequired = true;

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

    bindSnapshotLifecycleListeners() {
      if (this.snapshotListenersBound) {
        return;
      }

      this.addGlobalSubscription("_snapshot_begin", (payload) => {
        this.handleSnapshotBegin(payload);
      });
      this.addGlobalSubscription("_snapshot_end", (payload) => {
        this.handleSnapshotCompletion(payload);
      });

      this.snapshotListenersBound = true;
    },

    addGlobalSubscription(channel, handler) {
      if (!hasWsHub()) {
        return;
      }
      global.WsHub.subscribe(channel, handler);
      globalSubscriptions.push({ channel, handler });
    },

    handleSnapshotBegin(payload) {
      if (!payload || !payload.topic) {
        return;
      }
      const topic = payload.topic;
      const requestId =
        payload.context?.request_id || payload.context?.requestId || null;
      const context = this.snapshotContextsByTopic.get(topic);
      if (context) {
        context.startedAt = Date.now();
        if (requestId && context.requestId !== requestId) {
          context.requestId = requestId;
          if (context.aliases) {
            context.aliases.forEach((alias) => {
              this.snapshotContextsByAlias.set(alias, context);
              this.lastSnapshotRequestIds[alias] = requestId;
            });
          }
          this.lastSnapshotRequestIds[topic] = requestId;
        }
      } else if (requestId) {
        const aliasSet = new Set([topic]);
        const ctx = {
          topic,
          requestId,
          aliases: aliasSet,
          createdAt: Date.now(),
          startedAt: Date.now(),
        };
        this.snapshotContextsByTopic.set(topic, ctx);
        this.snapshotContextsByAlias.set(topic, ctx);
        this.lastSnapshotRequestIds[topic] = requestId;
      }
    },

    handleSnapshotCompletion(payload) {
      if (!payload || !payload.topic) {
        return;
      }
      const topic = payload.topic;
      const requestId =
        payload.context?.request_id || payload.context?.requestId || null;
      const context = this.snapshotContextsByTopic.get(topic);
      if (!context) {
        return;
      }
      if (requestId && context.requestId && context.requestId !== requestId) {
        return;
      }
      this.snapshotContextsByTopic.delete(topic);
      if (context.aliases) {
        context.aliases.forEach((alias) => {
          this.snapshotContextsByAlias.delete(alias);
        });
      }
    },

    prepareSnapshotContexts(snapshotAliases, willSendNow) {
      const contexts = new Map();
      const aliasRequestIds = {};

      if (!willSendNow || snapshotAliases.size === 0) {
        return { contexts, aliasRequestIds };
      }

      const now = Date.now();
      const timestampPart = now.toString(36);

      for (const alias of snapshotAliases) {
        if (!alias) continue;
        const topic = resolveTopicFromAlias(alias);
        let context = contexts.get(topic);
        if (!context) {
          const requestId = `snap-${topic
            .replace(/[^a-z0-9]+/gi, "-")
            .replace(/-+/g, "-")}-${timestampPart}-${Math.random()
            .toString(36)
            .slice(2, 8)}`;
          context = {
            topic,
            requestId,
            aliases: new Set(),
            createdAt: now,
          };
          contexts.set(topic, context);
        }
        context.aliases.add(alias);
        aliasRequestIds[alias] = context.requestId;
        aliasRequestIds[topic] = context.requestId;
      }

      contexts.forEach((ctx) => {
        this.snapshotContextsByTopic.set(ctx.topic, ctx);
        ctx.aliases.forEach((alias) => {
          this.snapshotContextsByAlias.set(alias, ctx);
        });
      });

      this.lastSnapshotRequestIds = { ...aliasRequestIds };

      return { contexts, aliasRequestIds };
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

    collectFilters(snapshotAliases, snapshotContexts) {
      const topics = {};

      const applyAlias = (alias) => {
        const topic = resolveTopicFromAlias(alias);
        const filters = this.getFiltersForAlias(alias);
        const existing = topics[topic] || {};
        const merged = { ...existing, ...filters };
        const context =
          snapshotContexts.get(topic) ||
          this.snapshotContextsByTopic.get(topic);
        if (snapshotAliases.has(alias)) {
          merged.snapshot = true;
          if (context && context.requestId) {
            merged.request_id = context.requestId;
          }
        }
        topics[topic] = merged;
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

      dbg("collectFilters", {
        snapshotAliases: Array.from(snapshotAliases),
        topics,
      });
      return topics;
    },

    updateFilters(options = {}) {
      let aliasRequestIds = {};
      if (options.snapshotTopics) {
        for (const alias of options.snapshotTopics) {
          if (alias) {
            this.snapshotRequestAliases.add(alias);
          }
        }
      }

      const snapshotAliases = new Set(this.snapshotRequestAliases);
      const hubAvailable = hasWsHub();
      const isConnected = hubAvailable && global.WsHub.isConnected();
      const { contexts: snapshotContexts, aliasRequestIds: generatedIds } =
        this.prepareSnapshotContexts(
          snapshotAliases,
          hubAvailable && isConnected
        );
      if (Object.keys(generatedIds).length > 0) {
        aliasRequestIds = generatedIds;
      }

      const topics = this.collectFilters(snapshotAliases, snapshotContexts);

      if (Object.keys(topics).length === 0) {
        const hadPrev = !!this.lastSentFilters;
        this.lastSentFilters = null;
        this.pendingFilterUpdate = false;
        this.pendingTopics = null;
        this.snapshotRequestAliases.clear();

        // Proactively clear server-side filters to avoid stale subscriptions
        if (hubAvailable) {
          if (isConnected) {
            if (hadPrev) {
              dbg("updateFilters:clearing_server_filters");
              global.WsHub.sendSetFilters({});
            }
          } else if (hadPrev) {
            // Defer clearing until connection is available
            this.pendingFilterUpdate = true;
            this.pendingTopics = {};
          }
        }
        return aliasRequestIds;
      }

      if (!hubAvailable) {
        dbg("updateFilters:hub_unavailable", topics);
        this.pendingFilterUpdate = true;
        this.pendingTopics = topics;
        return aliasRequestIds;
      }

      if (!isConnected) {
        dbg("updateFilters:ws_disconnected", topics);
        this.pendingFilterUpdate = true;
        this.pendingTopics = topics;
        return aliasRequestIds;
      }

      // Avoid redundant updates if nothing changed and no snapshot was requested
      const snapshotRequested = snapshotAliases.size > 0;
      if (
        !snapshotRequested &&
        deepEqualObjects(topics, this.lastSentFilters)
      ) {
        dbg("updateFilters:skipped_redundant", { topics });
        this.pendingFilterUpdate = false;
        this.pendingTopics = null;
        this.snapshotRequestAliases.clear();
        return aliasRequestIds;
      }

      dbg("updateFilters:sending", {
        topics,
        snapshotAliases: Array.from(snapshotAliases),
      });
      global.WsHub.sendSetFilters(topics);
      this.lastSentFilters = topics;
      this.pendingFilterUpdate = false;
      this.pendingTopics = null;
      this.snapshotRequestAliases.clear();
      return aliasRequestIds;
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
      return this.updateFilters();
    },

    isConnected() {
      return hasWsHub() && global.WsHub.isConnected();
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

      // Unsubscribe previous page-specific channels
      this.unbindActiveChannels();

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
      dbg("activate", { page: pageName, topics: config.topics });

      const status = hasWsHub() ? global.WsHub.getStatus() : "disconnected";

      if (typeof config.onInitial === "function") {
        try {
          config.onInitial(status);
        } catch (err) {
          console.error("[Realtime] onInitial handler failed:", err);
        }
      }

      // Bind channels BEFORE triggering onEnter/snapshot to avoid missing early messages
      this.bindActiveChannels();

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
      dbg("hub:connected");
      if (this.pendingFilterUpdate && this.pendingTopics) {
        global.WsHub.sendSetFilters(this.pendingTopics);
        this.lastSentFilters = this.pendingTopics;
        this.pendingFilterUpdate = false;
        this.pendingTopics = null;
        this.snapshotRequestAliases.clear();
      } else {
        const aliases = Array.from(this.getActiveAliases());
        dbg("hub:connected:active_aliases", aliases);
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

      // Ensure active channels bound after reconnect (in case subscriptions cleared)
      this.bindActiveChannels();
    },

    onHubDisconnected() {
      dbg("hub:disconnected");
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

    bindActiveChannels() {
      if (!hasWsHub()) return;
      const cfg = this.activeConfig;
      if (!cfg || !cfg.channels || typeof cfg.channels !== "object") return;
      for (const [channel, handler] of Object.entries(cfg.channels)) {
        if (typeof handler !== "function") continue;
        global.WsHub.subscribe(channel, handler);
        this.activeSubscriptions.push({ channel, handler });
      }
    },

    unbindActiveChannels() {
      if (!hasWsHub()) {
        this.activeSubscriptions = [];
        return;
      }
      const subs = this.activeSubscriptions.splice(0);
      for (const { channel, handler } of subs) {
        try {
          global.WsHub.unsubscribe(channel, handler);
        } catch (err) {
          console.warn("[Realtime] Failed to unbind channel", channel, err);
        }
      }
    },
  };

  document.addEventListener("DOMContentLoaded", () => {
    realtime.ensureHubInitialized();
    realtime.initializeOnce();

    // Network awareness: recover immediately when network returns; quiesce on offline
    if (typeof window !== "undefined") {
      const onOnline = () => {
        if (hasWsHub()) {
          global.WsHub.attempts = 0; // reset backoff
          global.WsHub.isConnecting = false;
          // If not connected, attempt immediate reconnect
          if (!global.WsHub.isConnected()) {
            global.WsHub.connect();
          } else {
            global.WsHub.startHeartbeat();
          }
        }
      };
      const onOffline = () => {
        if (hasWsHub()) {
          global.WsHub.emit("_warning", {
            alias: "ws",
            channel: "ws",
            topic: "system.status",
            message: "Network offline; realtime paused",
            recommendation: "http_catchup",
          });
          global.WsHub.stopHeartbeat();
          try {
            if (global.WsHub.conn) {
              global.WsHub.conn.close();
            }
          } catch (_) {}
        }
      };
      window.addEventListener("online", onOnline, { once: false });
      window.addEventListener("offline", onOffline, { once: false });
    }

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

  // Shallow-deep equality for plain objects and arrays
  function deepEqualObjects(a, b) {
    if (a === b) return true;
    if (typeof a !== typeof b) return false;
    if (a == null || b == null) return false;
    if (typeof a !== "object") return a === b;
    if (Array.isArray(a)) {
      if (!Array.isArray(b) || a.length !== b.length) return false;
      for (let i = 0; i < a.length; i++) {
        if (!deepEqualObjects(a[i], b[i])) return false;
      }
      return true;
    }
    const aKeys = Object.keys(a).sort();
    const bKeys = Object.keys(b).sort();
    if (!deepEqualObjects(aKeys, bKeys)) return false;
    for (const k of aKeys) {
      if (!deepEqualObjects(a[k], b[k])) return false;
    }
    return true;
  }
})();
