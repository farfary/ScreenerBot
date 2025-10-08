(function () {
    const global = window;

    // Ensure shared namespace exists for per-page realtime configs
    global.PageRealtime = global.PageRealtime || {};

    // WebSocket Hub - Centralized real-time updates (globally accessible)
    global.WsHub = {
        conn: null,
        enabled: true,
        attempts: 0,
        maxAttempts: 5,
        isConnecting: false,
        subscriptions: new Set(),
        listeners: {},

        connect() {
            if (this.isConnecting) return;
            if (this.conn && (this.conn.readyState === WebSocket.OPEN || this.conn.readyState === WebSocket.CONNECTING)) {
                return;
            }

            const proto = location.protocol === 'https:' ? 'wss' : 'ws';
            const url = `${proto}://${location.host}/api/ws`;

            console.log('[WsHub] Connecting:', url);

            try {
                this.isConnecting = true;
                this.conn = new WebSocket(url);

                this.conn.onopen = () => {
                    console.log('[WsHub] Connected');
                    this.attempts = 0;
                    this.isConnecting = false;

                    for (const channel of this.subscriptions) {
                        this.send({ type: 'subscribe', channel });
                    }

                    this.emit('_connected', {});
                };

                this.conn.onmessage = (event) => {
                    try {
                        const msg = JSON.parse(event.data);
                        this.handleMessage(msg);
                    } catch (err) {
                        console.error('[WsHub] Message parse error:', err);
                    }
                };

                this.conn.onclose = () => {
                    console.log('[WsHub] Closed');
                    this.isConnecting = false;
                    this.conn = null;
                    this.emit('_disconnected', {});
                    this.reconnect();
                };

                this.conn.onerror = (err) => {
                    console.error('[WsHub] Error:', err);
                    this.isConnecting = false;
                    try {
                        if (this.conn) {
                            this.conn.close();
                        }
                    } catch (_) {}
                    this.conn = null;
                    this.reconnect();
                };
            } catch (err) {
                console.error('[WsHub] Creation failed:', err);
                this.isConnecting = false;
                this.reconnect();
            }
        },

        reconnect() {
            this.attempts += 1;
            const delay = Math.min(1000 * Math.pow(2, this.attempts), 15000);

            console.log(`[WsHub] Reconnect attempt ${this.attempts}, delay: ${delay}ms`);

            if (
                this.conn &&
                (this.conn.readyState === WebSocket.OPEN ||
                    this.conn.readyState === WebSocket.CONNECTING ||
                    this.isConnecting)
            ) {
                return;
            }

            if (this.maxAttempts > 0 && this.attempts > this.maxAttempts) {
                console.warn('[WsHub] Reconnect still failing, continuing retries with max backoff');
                this.attempts = this.maxAttempts;
                this.emit('_warning', {
                    channel: 'ws',
                    message: 'Realtime connection degraded, retrying',
                    recommendation: 'http_catchup',
                });
            }

            setTimeout(() => this.connect(), delay);
        },

        handleMessage(msg) {
            switch (msg.type) {
                case 'data':
                    this.emit(msg.channel, msg.data, msg.timestamp);
                    break;
                case 'subscribed':
                    console.log('[WsHub] Subscribed to', msg.channel);
                    break;
                case 'unsubscribed':
                    console.log('[WsHub] Unsubscribed from', msg.channel);
                    break;
                case 'error':
                    console.error('[WsHub] Error:', msg.message, msg.code);
                    this.emit('_error', msg);
                    break;
                case 'warning':
                    console.warn('[WsHub] Warning:', msg.channel, msg.message);
                    this.emit('_warning', msg);
                    break;
                case 'pong':
                    break;
                default:
                    console.warn('[WsHub] Unknown message type:', msg.type);
            }
        },

        subscribe(channel, callback) {
            if (!this.listeners[channel]) {
                this.listeners[channel] = [];
            }
            this.listeners[channel].push(callback);
            const isInternal = typeof channel === 'string' && channel.startsWith('_');
            if (!isInternal) {
                this.subscriptions.add(channel);
            }

            if (!isInternal && this.conn && this.conn.readyState === WebSocket.OPEN) {
                this.send({ type: 'subscribe', channel });
            }
        },

        unsubscribe(channel, callback) {
            if (!this.listeners[channel]) {
                return;
            }

            this.listeners[channel] = this.listeners[channel].filter((cb) => cb !== callback);
            if (this.listeners[channel].length === 0) {
                delete this.listeners[channel];
                const isInternal = typeof channel === 'string' && channel.startsWith('_');
                if (!isInternal) {
                    this.subscriptions.delete(channel);
                }

                if (!isInternal && this.conn && this.conn.readyState === WebSocket.OPEN) {
                    this.send({ type: 'unsubscribe', channel });
                }
            }
        },

        emit(channel, data, timestamp) {
            if (!this.listeners[channel]) {
                return;
            }

            for (const callback of this.listeners[channel]) {
                try {
                    callback(data, timestamp);
                } catch (err) {
                    console.error('[WsHub] Listener error:', err);
                }
            }
        },

        send(msg) {
            if (this.conn && this.conn.readyState === WebSocket.OPEN) {
                this.conn.send(JSON.stringify(msg));
            } else {
                console.warn('[WsHub] Not connected, cannot send:', msg);
            }
        },

        startHeartbeat() {
            setInterval(() => {
                if (this.conn && this.conn.readyState === WebSocket.OPEN) {
                    this.send({ type: 'ping' });
                }
            }, 30000);
        },

        getStatus() {
            if (!this.conn) return 'disconnected';

            switch (this.conn.readyState) {
                case WebSocket.CONNECTING:
                    return 'connecting';
                case WebSocket.OPEN:
                    return 'connected';
                case WebSocket.CLOSING:
                    return 'closing';
                case WebSocket.CLOSED:
                    return 'disconnected';
                default:
                    return 'unknown';
            }
        },

        isConnected() {
            return this.conn && this.conn.readyState === WebSocket.OPEN;
        },
    };

    const globalSubscriptions = [];

    // Persistent subscriptions with client-side filtering (never unsubscribe on tab switch)
    const persistentSubscriptions = {
        status: (data) => {
            // Always handle status (global header)
            if (typeof global.renderStatusBadgesFromSnapshot === 'function') {
                global.renderStatusBadgesFromSnapshot(data);
            }
        },
        services: (data) => {
            // Only process if services page is active
            if (realtime.activePage === 'services' && 
                global.PageRealtime?.services?.channels?.services) {
                global.PageRealtime.services.channels.services(data);
            }
        },
        events: (data) => {
            // Only process if events page is active
            if (realtime.activePage === 'events' && 
                global.PageRealtime?.events?.channels?.events) {
                global.PageRealtime.events.channels.events(data);
            }
        },
        positions: (data) => {
            // Only process if positions page is active
            if (realtime.activePage === 'positions' && 
                global.PageRealtime?.positions?.channels?.positions) {
                global.PageRealtime.positions.channels.positions(data);
            }
        },
        prices: (data) => {
            // Only process if prices page is active (tokens page)
            if (realtime.activePage === 'tokens' && 
                global.PageRealtime?.tokens?.channels?.prices) {
                global.PageRealtime.tokens.channels.prices(data);
            }
        },
    };

    // Initialize persistent subscriptions once
    function initializePersistentSubscriptions() {
        if (!hasWsHub() || global.__persistentSubsInitialized) {
            return;
        }
        
        for (const [channel, handler] of Object.entries(persistentSubscriptions)) {
            global.WsHub.subscribe(channel, handler);
        }
        
        global.__persistentSubsInitialized = true;
        console.log('[Realtime] Persistent subscriptions initialized');
    }

    const realtime = {
        activePage: null,
        activeConfig: null,
        activeSubscriptions: [],
        hasInitialized: false,

        ensureHubInitialized() {
            if (!hasWsHub()) {
                return;
            }

            if (!global.__wsHubInitialized) {
                global.__wsHubInitialized = true;
                global.WsHub.connect();
                global.WsHub.startHeartbeat();
                initializePersistentSubscriptions();
            } else if (!global.WsHub.isConnected()) {
                global.WsHub.connect();
            }
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
                if (typeof global.setWsBadge === 'function') {
                    global.setWsBadge('disconnected', '🔌 N/A');
                }
                return;
            }

            const renderSnapshot =
                typeof global.renderStatusBadgesFromSnapshot === 'function'
                    ? global.renderStatusBadgesFromSnapshot
                    : null;
            const stopPolling =
                typeof global.stopStatusPolling === 'function' ? global.stopStatusPolling : null;
            const startPolling =
                typeof global.startStatusPolling === 'function' ? global.startStatusPolling : null;
            const fetchSnapshot =
                typeof global.fetchStatusSnapshot === 'function' ? global.fetchStatusSnapshot : null;

            const handleStatusUpdate = (snapshot) => {
                if (!snapshot) return;
                if (stopPolling) stopPolling();
                if (renderSnapshot) renderSnapshot(snapshot);
            };

            const handleStatusDisconnect = () => {
                if (typeof global.setWsBadge === 'function') {
                    global.setWsBadge('disconnected', '🔌 Offline');
                }
                if (startPolling) startPolling();
            };

            const handleStatusReconnect = () => {
                if (typeof global.setWsBadge === 'function') {
                    global.setWsBadge('connected', '🔌 Connected');
                }
                if (stopPolling) stopPolling();
                if (fetchSnapshot) fetchSnapshot();
            };

            this.addGlobalSubscription('status', handleStatusUpdate);
            this.addGlobalSubscription('_disconnected', handleStatusDisconnect);
            this.addGlobalSubscription('_failed', handleStatusDisconnect);
            this.addGlobalSubscription('_connected', handleStatusReconnect);

            if (typeof global.setWsBadge === 'function') {
                const status = typeof global.WsHub.getStatus === 'function' ? global.WsHub.getStatus() : 'unknown';
                if (status === 'connected') {
                    global.setWsBadge('connected', '🔌 Connected');
                } else if (status === 'connecting') {
                    global.setWsBadge('connecting', '🔌 Connecting');
                } else {
                    global.setWsBadge('disconnected', '🔌 Offline');
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

        activate(pageName) {
            if (!pageName) {
                return;
            }

            // Call onExit for previous page if exists
            const prevPage = this.activePage;
            if (prevPage && this.activeConfig && typeof this.activeConfig.onExit === 'function') {
                try {
                    this.activeConfig.onExit();
                } catch (err) {
                    console.error('[Realtime] onExit handler failed:', err);
                }
            }

            // Update active page state (no unsubscribe/resubscribe!)
            this.activePage = pageName;

            const config = global.PageRealtime ? global.PageRealtime[pageName] : undefined;
            this.activeConfig = config || null;

            if (!config) {
                console.log(`[Realtime] Activated page: ${pageName} (no config, was: ${prevPage || 'none'})`);
                return;
            }

            if (!hasWsHub()) {
                if (typeof config.onUnavailable === 'function') {
                    try {
                        config.onUnavailable();
                    } catch (err) {
                        console.error('[Realtime] onUnavailable handler failed:', err);
                    }
                }
                return;
            }

            const status = typeof global.WsHub.getStatus === 'function' ? global.WsHub.getStatus() : 'unknown';

            if (typeof config.onInitial === 'function') {
                try {
                    config.onInitial(status);
                } catch (err) {
                    console.error('[Realtime] onInitial handler failed:', err);
                }
            }

            if (typeof config.onEnter === 'function') {
                try {
                    config.onEnter(status);
                } catch (err) {
                    console.error('[Realtime] onEnter handler failed:', err);
                }
            }

            console.log(`[Realtime] Activated page: ${pageName} (was: ${prevPage || 'none'}) - using persistent subscriptions`);
        },

        deactivateCurrent() {
            // Only call onExit, do NOT unsubscribe (keep persistent subscriptions)
            if (this.activeConfig && typeof this.activeConfig.onExit === 'function') {
                try {
                    this.activeConfig.onExit();
                } catch (err) {
                    console.error('[Realtime] onExit handler failed:', err);
                }
            }

            // Only clear local state, keep subscriptions active
            this.activeConfig = null;
            this.activePage = null;
        },
    };

    function hasWsHub() {
        return typeof global.WsHub !== 'undefined' && global.WsHub && global.WsHub.enabled !== false;
    }

    document.addEventListener('DOMContentLoaded', () => {
        realtime.ensureHubInitialized();
        realtime.initializeOnce();

        const initialPage =
            (global.Router && Router.currentPage) ||
            (global.location
                ? global.location.pathname === '/'
                    ? 'home'
                    : global.location.pathname.replace(/^\//, '')
                : null);

        if (initialPage) {
            realtime.activate(initialPage);
        }
    });

    window.addEventListener('beforeunload', () => {
        realtime.deactivateCurrent();

        if (hasWsHub()) {
            const subs = globalSubscriptions.splice(0);
            for (const sub of subs) {
                try {
                    global.WsHub.unsubscribe(sub.channel, sub.handler);
                } catch (err) {
                    console.warn('[Realtime] Failed to clean global subscription', sub.channel, err);
                }
            }
        }
    });

    global.Realtime = realtime;
})();
