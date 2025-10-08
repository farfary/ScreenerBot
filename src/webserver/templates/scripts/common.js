
        // State Manager - Browser Storage (globally accessible)
        window.AppState = {
            save(key, value) {
                try {
                    localStorage.setItem(`screenerbot_${key}`, JSON.stringify(value));
                } catch (e) {
                    console.warn('Failed to save state:', key, e);
                }
            },
            
            load(key, defaultValue = null) {
                try {
                    const item = localStorage.getItem(`screenerbot_${key}`);
                    return item ? JSON.parse(item) : defaultValue;
                } catch (e) {
                    console.warn('Failed to load state:', key, e);
                    return defaultValue;
                }
            },
            
            remove(key) {
                try {
                    localStorage.removeItem(`screenerbot_${key}`);
                } catch (e) {
                    console.warn('Failed to remove state:', key, e);
                }
            },
            
            clearAll() {
                try {
                    Object.keys(localStorage)
                        .filter(key => key.startsWith('screenerbot_'))
                        .forEach(key => localStorage.removeItem(key));
                } catch (e) {
                    console.warn('Failed to clear state:', e);
                }
            }
        };
        // Client-Side Router - SPA Architecture
        window.Router = {
            currentPage: null,
            _timeoutMs: 10000,
            cleanupHandlers: [],
            
            registerCleanup(handler) {
                if (typeof handler === 'function') {
                    this.cleanupHandlers.push(handler);
                }
                return handler;
            },

            runCleanupHandlers() {
                while (this.cleanupHandlers.length) {
                    const handler = this.cleanupHandlers.pop();
                    try {
                        handler();
                    } catch (err) {
                        console.error('[Router] Cleanup handler failed:', err);
                    }
                }
            },

            trackInterval(intervalId) {
                if (intervalId != null) {
                    this.registerCleanup(() => clearInterval(intervalId));
                }
                return intervalId;
            },

            trackTimeout(timeoutId) {
                if (timeoutId != null) {
                    this.registerCleanup(() => clearTimeout(timeoutId));
                }
                return timeoutId;
            },
            
            async loadPage(pageName) {
                console.log('[Router] Loading page:', pageName);
                
                // Update current page
                this.currentPage = pageName;

                // Run cleanup handlers for previous page before loading new content
                this.runCleanupHandlers();
                
                // Update active tab styling
                document.querySelectorAll('nav .tab').forEach(tab => {
                    const tabPage = tab.getAttribute('data-page');
                    if (tabPage === pageName) {
                        tab.classList.add('active');
                    } else {
                        tab.classList.remove('active');
                    }
                });
                
                const mainContent = document.querySelector('main');
                if (mainContent) {
                    mainContent.setAttribute('data-loading', 'true');
                    mainContent.innerHTML = `
                        <div class="page-loading" style="padding: 2rem; text-align: center;">
                            <div style="font-size: 1.1rem; color: var(--text-secondary);">
                                Loading ${pageName}...
                            </div>
                        </div>
                    `;
                }

                // Fetch page content from API with timeout protection
                try {
                    const html = await this.fetchPageContent(pageName, this._timeoutMs);

                    if (!mainContent) {
                        console.error('[Router] Main content container not found');
                        return;
                    }

                    mainContent.innerHTML = html;
                    this.executeEmbeddedScripts(mainContent);

                    // Re-initialize page-specific scripts
                    this.initPageScripts(pageName);

                    // Clean up sub-tabs and toolbar for pages that don't use them
                    cleanupTabContainers();

                    // Update browser history only if path actually changed
                    const targetUrl = pageName === 'home' ? '/' : `/${pageName}`;
                    if (window.location.pathname !== targetUrl) {
                        window.history.pushState({ page: pageName }, '', targetUrl);
                    }

                    // Save last visited tab
                    AppState.save('lastTab', pageName);

                    console.log('[Router] Page loaded successfully:', pageName);
                } catch (error) {
                    console.error('[Router] Failed to load page:', pageName, error);
                    
                    // Show error in main content
                    if (mainContent) {
                        mainContent.removeAttribute('data-loading');
                        mainContent.innerHTML = `
                            <div style="padding: 2rem; text-align: center;">
                                <h2 style="color: #ef4444;">⚠️ Failed to Load Page</h2>
                                <p style="color: #9ca3af; margin-top: 1rem;">
                                    ${error.message}
                                </p>
                                <button onclick="Router.loadPage('${pageName}')" 
                                    style="margin-top: 1rem; padding: 0.5rem 1rem; 
                                           background: #3b82f6; color: white; border: none; 
                                           border-radius: 0.5rem; cursor: pointer;">
                                    Retry
                                </button>
                            </div>
                        `;
                    }
                }
                
                if (mainContent) {
                    mainContent.removeAttribute('data-loading');
                }
            },
            
            async fetchPageContent(pageName, timeoutMs = 10000) {
                const controller = new AbortController();
                const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
                const url = `/api/pages/${pageName}`;

                try {
                    const response = await fetch(url, { signal: controller.signal, headers: { 'X-Requested-With': 'fetch' } });
                    if (!response.ok) {
                        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                    }
                    return await response.text();
                } catch (error) {
                    if (error.name === 'AbortError') {
                        throw new Error(`Request timed out after ${Math.round(timeoutMs / 1000)}s`);
                    }
                    throw error;
                } finally {
                    clearTimeout(timeoutId);
                }
            },
            
            executeEmbeddedScripts(container) {
                if (!container) return;

                const scripts = container.querySelectorAll('script');
                scripts.forEach((script) => {
                    const newScript = document.createElement('script');
                    Array.from(script.attributes).forEach(attr => newScript.setAttribute(attr.name, attr.value));

                    if (script.src) {
                        newScript.src = script.src;
                    } else {
                        newScript.textContent = script.textContent;
                    }

                    script.parentNode?.replaceChild(newScript, script);
                });
            },
            
            initPageScripts(pageName) {
                // Re-initialize page-specific functionality after dynamic load
                switch (pageName) {
                    case 'status':
                        if (typeof initStatusSubTabs === 'function') initStatusSubTabs();
                        if (typeof ensureStatusSubTabsVisible === 'function') ensureStatusSubTabsVisible();
                        break;
                    case 'tokens':
                        if (typeof initTokensPage === 'function') initTokensPage();
                        break;
                    case 'positions':
                        if (typeof initPositionsPage === 'function') initPositionsPage();
                        break;
                    case 'events':
                        if (typeof initEventsPage === 'function') initEventsPage();
                        break;
                    case 'config':
                        if (typeof initConfigPage === 'function') initConfigPage();
                        break;
                    case 'services':
                        if (typeof initServicesPage === 'function') initServicesPage();
                        break;
                }

                if (window.Realtime && typeof Realtime.activate === 'function') {
                    Realtime.activate(pageName);
                }
            }
        };
        
        // Initialize router on page load
        document.addEventListener('DOMContentLoaded', () => {
            // Set initial page from URL
            const currentPath = window.location.pathname;
            const initialPage = currentPath === '/' ? 'home' : currentPath.substring(1);
            Router.currentPage = initialPage;
            
            // Intercept all navigation link clicks
            document.addEventListener('click', (e) => {
                const target = e.target.closest('a[data-page]');
                if (target) {
                    e.preventDefault();
                    const pageName = target.getAttribute('data-page');
                    Router.loadPage(pageName);
                }
            });
            
            // Handle browser back/forward buttons
            window.addEventListener('popstate', (e) => {
                if (e.state && e.state.page) {
                    Router.loadPage(e.state.page);
                } else {
                    const path = window.location.pathname;
                    const page = path === '/' ? 'home' : path.substring(1);
                    Router.loadPage(page);
                }
            });
            
            // Save initial state
            AppState.save('lastTab', initialPage);
            cleanupTabContainers();
            
            console.log('[Router] Initialized - SPA mode active');
        });
        
        // Helper function to hide sub-tabs/toolbar containers
        function cleanupTabContainers() {
            const subTabsContainer = document.getElementById('subTabsContainer');
            const toolbarContainer = document.getElementById('toolbarContainer');
            
            // Only hide if not on a page that explicitly shows them
            // Pages can call initPageSubTabs() to show and populate them
            const currentPath = window.location.pathname;
            const pagesWithSubTabs = ['/tokens']; // Add more as needed
            
            if (!pagesWithSubTabs.includes(currentPath)) {
                if (subTabsContainer) {
                    subTabsContainer.style.display = 'none';
                    subTabsContainer.innerHTML = '';
                }
                if (toolbarContainer) {
                    toolbarContainer.style.display = 'none';
                    toolbarContainer.innerHTML = '';
                }
            }
        }
        
        let statusPollInterval = null;

        function setWsBadge(state, message) {
            const badge = document.getElementById('wsBadge');
            if (!badge) return;

            switch (state) {
                case 'connected':
                    badge.className = 'badge online';
                    badge.innerHTML = message || '🔌 Connected';
                    badge.title = 'WebSocket: Connected';
                    break;
                case 'disconnected':
                    badge.className = 'badge error';
                    badge.innerHTML = message || '🔌 Offline';
                    badge.title = 'WebSocket: Disconnected';
                    break;
                case 'connecting':
                    badge.className = 'badge loading';
                    badge.innerHTML = message || '🔌 Connecting';
                    badge.title = 'WebSocket: Connecting...';
                    break;
                default:
                    badge.className = 'badge loading';
                    badge.innerHTML = message || '🔌 WS';
                    badge.title = 'WebSocket: Unknown';
                    break;
            }
        }

        function setBotBadge(state, message) {
            const badge = document.getElementById('botBadge');
            if (!badge) return;

            switch (state) {
                case 'running':
                    badge.className = 'badge online';
                    badge.innerHTML = message || '🤖 Running';
                    badge.title = 'Bot: Running';
                    break;
                case 'stopped':
                    badge.className = 'badge error';
                    badge.innerHTML = message || '🤖 Stopped';
                    badge.title = 'Bot: Stopped';
                    break;
                case 'error':
                    badge.className = 'badge error';
                    badge.innerHTML = message || '🤖 Error';
                    badge.title = 'Bot: Error';
                    break;
                case 'starting':
                    badge.className = 'badge loading';
                    badge.innerHTML = message || '🤖 Starting';
                    badge.title = 'Bot: Starting...';
                    break;
                default:
                    badge.className = 'badge loading';
                    badge.innerHTML = message || '🤖 BOT';
                    badge.title = 'Bot: Unknown';
                    break;
            }
        }

        function deriveAllReady(payload) {
            if (!payload || typeof payload !== 'object') return null;

            if (typeof payload.all_ready === 'boolean') {
                return payload.all_ready;
            }

            if (payload.services) {
                if (typeof payload.services.all_ready === 'boolean') {
                    return payload.services.all_ready;
                }

                const serviceStatuses = Object.values(payload.services);
                if (serviceStatuses.length > 0) {
                    return serviceStatuses.every(status =>
                        typeof status === 'string' && status.toLowerCase().includes('healthy')
                    );
                }
            }

            return null;
        }

        function renderStatusBadgesFromSnapshot(snapshot) {
            if (!snapshot || typeof snapshot !== 'object') {
                setBotBadge('error', '🤖 Error');
                return;
            }

            // Update bot status badge
            const allReady = deriveAllReady(snapshot);
            const tradingEnabled = snapshot.trading_enabled;
            
            if (allReady === true) {
                if (tradingEnabled === true) {
                    setBotBadge('running', '🤖 Running');
                } else if (tradingEnabled === false) {
                    setBotBadge('stopped', '🤖 Stopped');
                } else {
                    setBotBadge('running', '🤖 Ready');
                }
            } else if (allReady === false) {
                setBotBadge('starting', '🤖 Starting');
            } else {
                setBotBadge('starting', '🤖 Connecting');
            }
        }

        async function fetchStatusSnapshot() {
            try {
                const res = await fetch('/api/status');
                if (!res.ok) throw new Error(`HTTP ${res.status}`);
                const data = await res.json();
                renderStatusBadgesFromSnapshot(data);
                return data;
            } catch (error) {
                console.warn('Failed to fetch status snapshot:', error);
                setBotBadge('error', '🤖 Error');
                return null;
            }
        }

        function startStatusPolling(intervalMs = 5000) {
            if (statusPollInterval) return;
            statusPollInterval = setInterval(fetchStatusSnapshot, intervalMs);
        }

        function stopStatusPolling() {
            if (!statusPollInterval) return;
            clearInterval(statusPollInterval);
            statusPollInterval = null;
        }
        
        // Format uptime
        function formatUptime(seconds) {
            const days = Math.floor(seconds / 86400);
            const hours = Math.floor((seconds % 86400) / 3600);
            const minutes = Math.floor((seconds % 3600) / 60);
            const secs = Math.floor(seconds % 60);
            
            if (days > 0) return `${days}d ${hours}h ${minutes}m`;
            if (hours > 0) return `${hours}h ${minutes}m ${secs}s`;
            if (minutes > 0) return `${minutes}m ${secs}s`;
            return `${secs}s`;
        }
        
        // Format large numbers
        function formatNumber(num) {
            return num.toLocaleString();
        }
        
    // Initialize (refresh silently every 1s)
    fetchStatusSnapshot();
    startStatusPolling();

    if (window.Router && typeof Router.registerCleanup === 'function') {
        Router.registerCleanup(() => {
            stopStatusPolling();
        });
    }

        // Dropdown Menu Functions
        function toggleDropdown(event) {
            event.stopPropagation();
            const btn = event.currentTarget;
            const menu = btn.nextElementSibling;
            
            // Close all other dropdowns
            document.querySelectorAll('.dropdown-menu.show').forEach(m => {
                if (m !== menu) {
                    m.classList.remove('show');
                    m.style.position = '';
                    m.style.top = '';
                    m.style.left = '';
                    m.style.right = '';
                    m.style.width = '';
                }
            });
            
            // Toggle visibility
            const willShow = !menu.classList.contains('show');
            if (!willShow) {
                menu.classList.remove('show');
                return;
            }
            
            // Compute viewport position for fixed menu to avoid clipping
            const rect = btn.getBoundingClientRect();
            const menuWidth = Math.max(200, menu.offsetWidth || 200);
            const viewportWidth = window.innerWidth;
            const rightSpace = viewportWidth - rect.right;
            
            menu.classList.add('show');
            menu.style.position = 'fixed';
            menu.style.top = `${Math.round(rect.bottom + 4)}px`;
            if (rightSpace < menuWidth) {
                // Align to right edge of button when near viewport edge
                menu.style.left = `${Math.max(8, Math.round(rect.right - menuWidth))}px`;
                menu.style.right = '';
            } else {
                // Default align to button's left
                menu.style.left = `${Math.round(rect.left)}px`;
                menu.style.right = '';
            }
            menu.style.width = `${menuWidth}px`;
        }
        
        // Close dropdowns when clicking outside
        document.addEventListener('click', () => {
            document.querySelectorAll('.dropdown-menu.show').forEach(m => {
                m.classList.remove('show');
                m.style.position = '';
                m.style.top = '';
                m.style.left = '';
                m.style.right = '';
                m.style.width = '';
            });
        });
        // Also close on scroll/resize to avoid desync
        ['scroll','resize'].forEach(evt => {
            window.addEventListener(evt, () => {
                document.querySelectorAll('.dropdown-menu.show').forEach(m => {
                    m.classList.remove('show');
                    m.style.position = '';
                    m.style.top = '';
                    m.style.left = '';
                    m.style.right = '';
                    m.style.width = '';
                });
            }, { passive: true });
        });
        
        // Copy Mint Address Function
        function copyMint(mint) {
            navigator.clipboard.writeText(mint).then(() => {
                showToast('✅ Mint address copied to clipboard!');
            }).catch(err => {
                showToast('❌ Failed to copy: ' + err, 'error');
            });
        }
        
        // Copy any debug value to clipboard
        function copyDebugValue(value, label) {
            navigator.clipboard.writeText(value).then(() => {
                showToast(`✅ ${label} copied to clipboard!`);
            }).catch(err => {
                showToast('❌ Failed to copy: ' + err, 'error');
            });
        }
        
        // Build and copy full debug info (single action)
        async function copyDebugInfo(mint, type) {
            try {
                const endpoint = type === 'position' ? `/api/positions/${mint}/debug` : `/api/tokens/${mint}/debug`;
                const res = await fetch(endpoint);
                const data = await res.json();
                const text = generateDebugText(data, type);
                await navigator.clipboard.writeText(text);
                showToast('✅ Debug info copied to clipboard!');
            } catch (err) {
                console.error('copyDebugInfo error:', err);
                showToast('❌ Failed to copy debug info: ' + err, 'error');
            }
        }

        function generateDebugText(data, type) {
            const lines = [];
            const tokenInfo = data.token_info || {};
            const price = data.price_data || {};
            const market = data.market_data || {};
            const pools = Array.isArray(data.pools) ? data.pools : [];
            const security = data.security || {};
            const pos = data.position_data || {};

            // Header
            lines.push('ScreenerBot Debug Info');
            lines.push(`Mint: ${data.mint || 'N/A'}`);
            if (tokenInfo.symbol || tokenInfo.name) {
                lines.push(`Token: ${tokenInfo.symbol || 'N/A'} ${tokenInfo.name ? '(' + tokenInfo.name + ')' : ''}`);
            }
            lines.push('');

            // Token Info
            lines.push('[Token]');
            lines.push(`Symbol: ${tokenInfo.symbol ?? 'N/A'}`);
            lines.push(`Name: ${tokenInfo.name ?? 'N/A'}`);
            lines.push(`Decimals: ${tokenInfo.decimals ?? 'N/A'}`);
            lines.push(`Website: ${tokenInfo.website ?? 'N/A'}`);
            lines.push(`Verified: ${tokenInfo.is_verified ? 'Yes' : 'No'}`);
            const tags = Array.isArray(tokenInfo.tags) ? tokenInfo.tags.join(', ') : 'None';
            lines.push(`Tags: ${tags}`);
            lines.push('');

            // Price & Market
            lines.push('[Price & Market]');
            lines.push(`Price (SOL): ${price.pool_price_sol != null ? Number(price.pool_price_sol).toPrecision(10) : 'N/A'}`);
            lines.push(`Confidence: ${price.confidence != null ? (Number(price.confidence) * 100).toFixed(1) + '%' : 'N/A'}`);
            lines.push(`Last Updated: ${price.last_updated ? new Date(price.last_updated * 1000).toISOString() : 'N/A'}`);
            lines.push(`Market Cap: ${market.market_cap != null ? '$' + Number(market.market_cap).toLocaleString() : 'N/A'}`);
            lines.push(`FDV: ${market.fdv != null ? '$' + Number(market.fdv).toLocaleString() : 'N/A'}`);
            lines.push(`Liquidity: ${market.liquidity_usd != null ? '$' + Number(market.liquidity_usd).toLocaleString() : 'N/A'}`);
            lines.push(`24h Volume: ${market.volume_24h != null ? '$' + Number(market.volume_24h).toLocaleString() : 'N/A'}`);
            lines.push('');

            // Pools
            lines.push('[Pools]');
            if (pools.length === 0) {
                lines.push('None');
            } else {
                pools.forEach((p, idx) => {
                    lines.push(`Pool #${idx + 1}`);
                    lines.push(`  Address: ${p.pool_address ?? 'N/A'}`);
                    lines.push(`  DEX: ${p.dex_name ?? 'N/A'}`);
                    lines.push(`  SOL Reserves: ${p.sol_reserves != null ? Number(p.sol_reserves).toFixed(2) : 'N/A'}`);
                    lines.push(`  Token Reserves: ${p.token_reserves != null ? Number(p.token_reserves).toFixed(2) : 'N/A'}`);
                    lines.push(`  Price (SOL): ${p.price_sol != null ? Number(p.price_sol).toPrecision(10) : 'N/A'}`);
                });
            }
            lines.push('');

            // Security
            lines.push('[Security]');
            lines.push(`Score: ${security.score ?? 'N/A'}`);
            lines.push(`Rugged: ${security.rugged ? 'Yes' : 'No'}`);
            lines.push(`Total Holders: ${security.total_holders ?? 'N/A'}`);
            lines.push(`Top 10 Concentration: ${security.top_10_concentration != null ? Number(security.top_10_concentration).toFixed(2) + '%' : 'N/A'}`);
            lines.push(`Mint Authority: ${security.mint_authority ?? 'None'}`);
            lines.push(`Freeze Authority: ${security.freeze_authority ?? 'None'}`);
            const risks = Array.isArray(security.risks) ? security.risks : [];
            if (risks.length) {
                lines.push('Risks:');
                risks.forEach(r => lines.push(`  - ${r.name || 'Unknown'}: ${r.level || 'N/A'} (${r.description || ''})`));
            } else {
                lines.push('Risks: None');
            }
            lines.push('');

            // Position
            if (type === 'position') {
                lines.push('[Position]');
                if (pos && Object.keys(pos).length) {
                    lines.push(`Open Positions: ${pos.open_position ? '1' : '0'}`);
                    lines.push(`Closed Positions: ${pos.closed_positions_count ?? '0'}`);
                    lines.push(`Total P&L: ${pos.total_pnl != null ? Number(pos.total_pnl).toFixed(4) + ' SOL' : 'N/A'}`);
                    lines.push(`Win Rate: ${pos.win_rate != null ? Number(pos.win_rate).toFixed(1) + '%' : 'N/A'}`);
                    if (pos.open_position) {
                        const o = pos.open_position;
                        lines.push('Open Position:');
                        lines.push(`  Entry Price: ${o.entry_price != null ? Number(o.entry_price).toPrecision(10) : 'N/A'}`);
                        lines.push(`  Entry Size: ${o.entry_size_sol != null ? Number(o.entry_size_sol).toFixed(4) + ' SOL' : 'N/A'}`);
                        lines.push(`  Current Price: ${o.current_price != null ? Number(o.current_price).toPrecision(10) : 'N/A'}`);
                        lines.push(`  Unrealized P&L: ${o.unrealized_pnl != null ? Number(o.unrealized_pnl).toFixed(4) + ' SOL' : 'N/A'}`);
                        lines.push(`  Unrealized P&L %: ${o.unrealized_pnl_percent != null ? Number(o.unrealized_pnl_percent).toFixed(2) + '%' : 'N/A'}`);
                    }
                } else {
                    lines.push('No position data available');
                }
                lines.push('');
            }

            // Pool Debug
            if (data.pool_debug) {
                const pd = data.pool_debug;
                lines.push('[Pool Debug]');
                
                // Price history
                if (pd.price_history && pd.price_history.length > 0) {
                    lines.push(`Price History Points: ${pd.price_history.length}`);
                    lines.push('Recent Prices (last 10):');
                    pd.price_history.slice(0, 10).forEach((p, i) => {
                        const date = new Date(p.timestamp * 1000).toISOString();
                        lines.push(`  ${i + 1}. ${date} - ${Number(p.price_sol).toPrecision(10)} SOL (conf: ${(p.confidence * 100).toFixed(1)}%)`);
                    });
                }
                
                // Price stats
                if (pd.price_stats) {
                    const ps = pd.price_stats;
                    lines.push(`Min Price: ${Number(ps.min_price).toPrecision(10)} SOL`);
                    lines.push(`Max Price: ${Number(ps.max_price).toPrecision(10)} SOL`);
                    lines.push(`Avg Price: ${Number(ps.avg_price).toPrecision(10)} SOL`);
                    lines.push(`Volatility: ${Number(ps.price_volatility).toFixed(2)}%`);
                    lines.push(`Data Points: ${ps.data_points}`);
                    lines.push(`Time Span: ${ps.time_span_seconds}s (${(ps.time_span_seconds / 60).toFixed(0)} min)`);
                }
                
                // All pools
                if (pd.all_pools && pd.all_pools.length > 0) {
                    lines.push(`All Pools (${pd.all_pools.length}):`);
                    pd.all_pools.forEach((pool, i) => {
                        lines.push(`  Pool #${i + 1}: ${pool.pool_address}`);
                        lines.push(`    DEX: ${pool.dex_name}`);
                    });
                }
                
                // Cache stats
                if (pd.cache_stats) {
                    lines.push(`Cache - Total: ${pd.cache_stats.total_tokens_cached}, Fresh: ${pd.cache_stats.fresh_prices}, History: ${pd.cache_stats.history_entries}`);
                }
                
                lines.push('');
            }

            // Token Debug
            if (data.token_debug) {
                const td = data.token_debug;
                lines.push('[Token Debug]');
                
                if (td.blacklist_status) {
                    lines.push(`Blacklisted: ${td.blacklist_status.is_blacklisted ? 'Yes' : 'No'}`);
                    if (td.blacklist_status.is_blacklisted && td.blacklist_status.reason) {
                        lines.push(`  Reason: ${td.blacklist_status.reason}`);
                        lines.push(`  Occurrences: ${td.blacklist_status.occurrence_count}`);
                        lines.push(`  First Occurrence: ${td.blacklist_status.first_occurrence || 'N/A'}`);
                    }
                }
                
                if (td.ohlcv_availability) {
                    const oa = td.ohlcv_availability;
                    lines.push(`OHLCV: 1m=${oa.has_1m_data}, 5m=${oa.has_5m_data}, 15m=${oa.has_15m_data}, 1h=${oa.has_1h_data}`);
                    lines.push(`Total Candles: ${oa.total_candles}`);
                    if (oa.oldest_timestamp) {
                        lines.push(`  Oldest: ${new Date(oa.oldest_timestamp * 1000).toISOString()}`);
                    }
                    if (oa.newest_timestamp) {
                        lines.push(`  Newest: ${new Date(oa.newest_timestamp * 1000).toISOString()}`);
                    }
                }
                
                if (td.decimals_info) {
                    lines.push(`Decimals: ${td.decimals_info.decimals ?? 'N/A'} (${td.decimals_info.source}, cached: ${td.decimals_info.cached})`);
                }
                
                lines.push('');
            }

            // Position Debug
            if (data.position_debug) {
                const pd = data.position_debug;
                lines.push('[Position Debug]');
                
                if (pd.transaction_details) {
                    lines.push('Transactions:');
                    lines.push(`  Entry: ${pd.transaction_details.entry_signature || 'N/A'} (verified: ${pd.transaction_details.entry_verified})`);
                    lines.push(`  Exit: ${pd.transaction_details.exit_signature || 'N/A'} (verified: ${pd.transaction_details.exit_verified})`);
                    if (pd.transaction_details.synthetic_exit) {
                        lines.push(`  Synthetic Exit: Yes`);
                    }
                    if (pd.transaction_details.closed_reason) {
                        lines.push(`  Closed Reason: ${pd.transaction_details.closed_reason}`);
                    }
                }
                
                if (pd.fee_details) {
                    lines.push(`Fees:`);
                    lines.push(`  Entry: ${pd.fee_details.entry_fee_sol?.toFixed(6) || 'N/A'} SOL (${pd.fee_details.entry_fee_lamports || 0} lamports)`);
                    lines.push(`  Exit: ${pd.fee_details.exit_fee_sol?.toFixed(6) || 'N/A'} SOL (${pd.fee_details.exit_fee_lamports || 0} lamports)`);
                    lines.push(`  Total: ${pd.fee_details.total_fees_sol.toFixed(6)} SOL`);
                }
                
                if (pd.profit_targets) {
                    lines.push(`Profit Targets: Min ${pd.profit_targets.min_target_percent || 'N/A'}%, Max ${pd.profit_targets.max_target_percent || 'N/A'}%`);
                    lines.push(`Liquidity Tier: ${pd.profit_targets.liquidity_tier || 'N/A'}`);
                }
                
                if (pd.price_tracking) {
                    lines.push(`Price Tracking:`);
                    lines.push(`  High: ${pd.price_tracking.price_highest}`);
                    lines.push(`  Low: ${pd.price_tracking.price_lowest}`);
                    lines.push(`  Current: ${pd.price_tracking.current_price || 'N/A'}`);
                    if (pd.price_tracking.drawdown_from_high) {
                        lines.push(`  Drawdown from High: ${pd.price_tracking.drawdown_from_high.toFixed(2)}%`);
                    }
                    if (pd.price_tracking.gain_from_low) {
                        lines.push(`  Gain from Low: ${pd.price_tracking.gain_from_low.toFixed(2)}%`);
                    }
                }
                
                if (pd.phantom_details) {
                    lines.push(`Phantom:`);
                    lines.push(`  Remove Flag: ${pd.phantom_details.phantom_remove}`);
                    lines.push(`  Confirmations: ${pd.phantom_details.phantom_confirmations}`);
                    if (pd.phantom_details.phantom_first_seen) {
                        lines.push(`  First Seen: ${pd.phantom_details.phantom_first_seen}`);
                    }
                }
                
                if (pd.proceeds_metrics) {
                    const pm = pd.proceeds_metrics;
                    lines.push(`Proceeds Metrics:`);
                    lines.push(`  Accepted: ${pm.accepted_quotes} (${pm.accepted_profit_quotes} profit, ${pm.accepted_loss_quotes} loss)`);
                    lines.push(`  Rejected: ${pm.rejected_quotes}`);
                    lines.push(`  Avg Shortfall: ${pm.average_shortfall_bps.toFixed(2)} bps`);
                    lines.push(`  Worst Shortfall: ${pm.worst_shortfall_bps} bps`);
                }
                
                lines.push('');
            }

            return lines.join('\n');
        }
        
        // Open External Links
        function openGMGN(mint) {
            window.open(`https://gmgn.ai/sol/token/${mint}`, '_blank');
        }
        
        function openDexScreener(mint) {
            window.open(`https://dexscreener.com/solana/${mint}`, '_blank');
        }
        
        function openSolscan(mint) {
            window.open(`https://solscan.io/token/${mint}`, '_blank');
        }
        
        // Show Debug Modal
        async function showDebugModal(mint, type) {
            const modal = document.getElementById('debugModal');
            const endpoint = type === 'position' ? 
                `/api/positions/${mint}/debug` : 
                `/api/tokens/${mint}/debug`;
            
            modal.classList.add('show');
            
            try {
                const response = await fetch(endpoint);
                const data = await response.json();
                populateDebugModal(data, type);
            } catch (error) {
                showToast('❌ Failed to load debug info: ' + error, 'error');
                console.error('Debug modal error:', error);
            }
        }
        
        // Close Debug Modal
        function closeDebugModal() {
            document.getElementById('debugModal').classList.remove('show');
        }
        
        // Switch Debug Modal Tabs
        function switchDebugTab(tabName) {
            // Update tab buttons
            document.querySelectorAll('.modal-tab').forEach(tab => {
                tab.classList.remove('active');
            });
            event.currentTarget.classList.add('active');
            
            // Update tab content
            document.querySelectorAll('.modal-tab-content').forEach(content => {
                content.classList.remove('active');
            });
            document.getElementById(`tab-${tabName}`).classList.add('active');
        }
        
        // Populate Debug Modal with Data (no per-field copy buttons)
        function populateDebugModal(data, type) {
            // Store mint for copying
            const mintAddress = data.mint;
            
            // Token Info Tab
            const tokenInfo = data.token_info || {};
            document.getElementById('tokenSymbol').textContent = tokenInfo.symbol || 'N/A';
            document.getElementById('tokenName').textContent = tokenInfo.name || 'N/A';
            document.getElementById('tokenDecimals').textContent = tokenInfo.decimals || 'N/A';
            document.getElementById('tokenWebsite').innerHTML = tokenInfo.website ? 
                `<a href="${tokenInfo.website}" target="_blank" style="color: var(--link-color);">${tokenInfo.website}</a>` : 
                '<span class="debug-value-text">N/A</span>';
            document.getElementById('tokenVerified').textContent = tokenInfo.is_verified ? '✅ Yes' : '❌ No';
            document.getElementById('tokenTags').textContent = tokenInfo.tags?.join(', ') || 'None';
            
            // Add mint address display at the top
            document.getElementById('debugMintAddress').textContent = mintAddress || 'N/A';
            
            // Price Data Tab
            const priceData = data.price_data || {};
            document.getElementById('priceSol').textContent = priceData.pool_price_sol ? priceData.pool_price_sol.toFixed(9) : 'N/A';
            document.getElementById('priceConfidence').textContent = priceData.confidence ? (priceData.confidence * 100).toFixed(1) + '%' : 'N/A';
            document.getElementById('priceUpdated').textContent = priceData.last_updated ? 
                new Date(priceData.last_updated * 1000).toLocaleString() : 'N/A';
            
            // Market Data
            const marketData = data.market_data || {};
            document.getElementById('marketCap').textContent = marketData.market_cap ? ('$' + marketData.market_cap.toLocaleString()) : 'N/A';
            document.getElementById('fdv').textContent = marketData.fdv ? ('$' + marketData.fdv.toLocaleString()) : 'N/A';
            document.getElementById('liquidity').textContent = marketData.liquidity_usd ? ('$' + marketData.liquidity_usd.toLocaleString()) : 'N/A';
            document.getElementById('volume24h').textContent = marketData.volume_24h ? ('$' + marketData.volume_24h.toLocaleString()) : 'N/A';
            
            // Pool Data Tab
            const poolsHtml = (data.pools || []).map(pool => `
                <div class="debug-section">
                    <div class="debug-row">
                        <span class="debug-label">Pool Address:</span>
                        <span class="debug-value"><span class="debug-value-text">${pool.pool_address}</span></span>
                    </div>
                    <div class="debug-row">
                        <span class="debug-label">DEX:</span>
                        <span class="debug-value"><span class="debug-value-text">${pool.dex_name}</span></span>
                    </div>
                    <div class="debug-row">
                        <span class="debug-label">SOL Reserves:</span>
                        <span class="debug-value"><span class="debug-value-text">${pool.sol_reserves.toFixed(2)}</span></span>
                    </div>
                    <div class="debug-row">
                        <span class="debug-label">Token Reserves:</span>
                        <span class="debug-value"><span class="debug-value-text">${pool.token_reserves.toFixed(2)}</span></span>
                    </div>
                    <div class="debug-row">
                        <span class="debug-label">Price (SOL):</span>
                        <span class="debug-value"><span class="debug-value-text">${pool.price_sol.toFixed(9)}</span></span>
                    </div>
                </div>
            `).join('');
            document.getElementById('poolsList').innerHTML = poolsHtml || '<p>No pool data available</p>';
            
            // Security Tab
            const security = data.security || {};
            document.getElementById('securityScore').textContent = security.score ?? 'N/A';
            document.getElementById('securityRugged').textContent = security.rugged ? '❌ Yes' : '✅ No';
            document.getElementById('securityHolders').textContent = security.total_holders ?? 'N/A';
            document.getElementById('securityTop10').textContent = security.top_10_concentration != null ? (security.top_10_concentration.toFixed(2) + '%') : 'N/A';
            document.getElementById('securityMintAuth').textContent = security.mint_authority ?? 'None';
            document.getElementById('securityFreezeAuth').textContent = security.freeze_authority ?? 'None';
            
            const risksHtml = (security.risks || []).map(risk => `
                <div class="debug-row">
                    <span class="debug-label">${risk.name}:</span>
                    <span class="debug-value">${risk.level} (${risk.description})</span>
                </div>
            `).join('');
            document.getElementById('securityRisks').innerHTML = risksHtml || '<p>No risks detected</p>';
            
            // Position-specific data
            if (type === 'position' && data.position_data) {
                const posData = data.position_data;
                document.getElementById('positionOpenPositions').textContent = posData.open_position ? '1 Open' : 'None';
                document.getElementById('positionClosedCount').textContent = posData.closed_positions_count;
                document.getElementById('positionTotalPnL').textContent = posData.total_pnl.toFixed(4) + ' SOL';
                document.getElementById('positionWinRate').textContent = posData.win_rate.toFixed(1) + '%';
                
                if (posData.open_position) {
                    const open = posData.open_position;
                    document.getElementById('positionEntryPrice').textContent = open.entry_price.toFixed(9);
                    document.getElementById('positionEntrySize').textContent = open.entry_size_sol.toFixed(4) + ' SOL';
                    document.getElementById('positionCurrentPrice').textContent = open.current_price ? 
                        open.current_price.toFixed(9) : 'N/A';
                    document.getElementById('positionUnrealizedPnL').textContent = open.unrealized_pnl ? 
                        open.unrealized_pnl.toFixed(4) + ' SOL (' + open.unrealized_pnl_percent.toFixed(2) + '%)' : 'N/A';
                }
            }
        }
        
        // Toast Notification
        function showToast(message, type = 'success') {
            const container = document.getElementById('toastContainer') || createToastContainer();
            const toast = document.createElement('div');
            toast.className = 'toast' + (type === 'error' ? ' error' : '');
            toast.innerHTML = `<div class="toast-message">${message}</div>`;
            
            container.appendChild(toast);
            
            setTimeout(() => {
                toast.style.opacity = '0';
                setTimeout(() => toast.remove(), 300);
            }, 3000);
        }
        
        function createToastContainer() {
            const container = document.createElement('div');
            container.id = 'toastContainer';
            container.className = 'toast-container';
            document.body.appendChild(container);
            return container;
        }
        
        // =============================================================================
        // TRADER CONTROL SYSTEM
        // =============================================================================
        
        let traderStatusPollInterval = null;
        let isReconnecting = false;
        
        // Initialize trader controls on page load
        function initializeTraderControls() {
            const traderToggle = document.getElementById('traderToggle');
            const rebootBtn = document.getElementById('rebootBtn');
            
            if (traderToggle) {
                traderToggle.addEventListener('click', toggleTrader);
            }
            
            if (rebootBtn) {
                rebootBtn.addEventListener('click', rebootSystem);
            }
            
            // Start polling trader status
            updateTraderStatus();
            traderStatusPollInterval = setInterval(updateTraderStatus, 2000);

            if (window.Router && typeof Router.registerCleanup === 'function') {
                Router.registerCleanup(() => {
                    if (traderStatusPollInterval) {
                        clearInterval(traderStatusPollInterval);
                        traderStatusPollInterval = null;
                    }
                });
            }
        }
        
        // Update trader status from API
        async function updateTraderStatus() {
            if (isReconnecting) return; // Skip during reconnect
            
            try {
                const res = await fetch('/api/trader/status');
                if (!res.ok) throw new Error(`HTTP ${res.status}`);
                
                const data = await res.json();
                const status = data.data || data;
                
                updateTraderUI(status.enabled, status.running);
            } catch (error) {
                console.warn('Failed to fetch trader status:', error);
                // Don't update UI on transient network errors
            }
        }
        
        // Update trader UI based on status
        function updateTraderUI(enabled, running) {
            const traderToggle = document.getElementById('traderToggle');
            const traderIcon = document.getElementById('traderIcon');
            const traderText = document.getElementById('traderText');
            
            if (!traderToggle || !traderIcon || !traderText) return;
            
            // Remove existing state classes
            traderToggle.classList.remove('running', 'stopped');
            
            if (enabled && running) {
                traderToggle.classList.add('running');
                traderIcon.textContent = '▶️';
                traderText.textContent = 'Trader Running';
            } else {
                traderToggle.classList.add('stopped');
                traderIcon.textContent = '⏸️';
                traderText.textContent = 'Trader Stopped';
            }
            
            traderToggle.disabled = false;
        }
        
        // Toggle trader on/off
        async function toggleTrader() {
            const traderToggle = document.getElementById('traderToggle');
            const traderIcon = document.getElementById('traderIcon');
            const traderText = document.getElementById('traderText');
            
            if (!traderToggle) return;
            
            // Determine current state from UI
            const isRunning = traderToggle.classList.contains('running');
            const endpoint = isRunning ? '/api/trader/stop' : '/api/trader/start';
            const action = isRunning ? 'Stopping' : 'Starting';
            
            // Disable button and show loading state
            traderToggle.disabled = true;
            traderIcon.textContent = '⏳';
            traderText.textContent = `${action}...`;
            
            try {
                const res = await fetch(endpoint, { method: 'POST' });
                const data = await res.json();
                
                if (!res.ok || !data.success) {
                    throw new Error(data.error || data.message || 'Request failed');
                }
                
                // Update UI based on response
                const status = data.status || data.data?.status || {};
                updateTraderUI(status.enabled, status.running);
                
                const message = data.message || data.data?.message || 
                    (isRunning ? 'Trader stopped successfully' : 'Trader started successfully');
                showToast(`✅ ${message}`);
                
                // Immediate status refresh
                setTimeout(updateTraderStatus, 500);
            } catch (error) {
                console.error('Trader toggle error:', error);
                showToast(`❌ Failed to ${isRunning ? 'stop' : 'start'} trader: ${error.message}`, 'error');
                
                // Restore previous state
                updateTraderUI(isRunning, isRunning);
            }
        }
        
        // Reboot the entire system
        async function rebootSystem() {
            const rebootBtn = document.getElementById('rebootBtn');
            if (!rebootBtn) return;
            
            // Confirm action
            if (!confirm('⚠️ Are you sure you want to reboot ScreenerBot? This will restart the entire process.')) {
                return;
            }
            
            // Disable button and show loading
            rebootBtn.disabled = true;
            const originalHTML = rebootBtn.innerHTML;
            rebootBtn.innerHTML = '<span>⏳</span><span>Rebooting...</span>';
            
            try {
                const res = await fetch('/api/system/reboot', { method: 'POST' });
                const data = await res.json();
                
                if (!res.ok || !data.success) {
                    throw new Error(data.error || 'Reboot request failed');
                }
                
                showToast('🔄 System reboot initiated. Reconnecting...', 'info');
                
                // Start reconnection attempts
                isReconnecting = true;
                if (traderStatusPollInterval) {
                    clearInterval(traderStatusPollInterval);
                    traderStatusPollInterval = null;
                }
                
                attemptReconnect();
            } catch (error) {
                console.error('Reboot error:', error);
                showToast(`❌ Failed to initiate reboot: ${error.message}`, 'error');
                
                // Restore button
                rebootBtn.disabled = false;
                rebootBtn.innerHTML = originalHTML;
            }
        }
        
        // Attempt to reconnect after reboot
        async function attemptReconnect() {
            const maxAttempts = 60; // 60 attempts = 2 minutes
            let attempt = 0;
            
            const checkConnection = async () => {
                attempt++;
                
                try {
                    const res = await fetch('/api/status', { 
                        cache: 'no-cache',
                        signal: AbortSignal.timeout(3000)
                    });
                    
                    if (res.ok) {
                        showToast('✅ System reconnected successfully!');
                        
                        // Reload the page to refresh all state
                        setTimeout(() => {
                            window.location.reload();
                        }, 1000);
                        return;
                    }
                } catch (error) {
                    // Connection failed, continue trying
                }
                
                if (attempt < maxAttempts) {
                    showToast(`🔄 Reconnecting... (${attempt}/${maxAttempts})`, 'info');
                    setTimeout(checkConnection, 2000);
                } else {
                    showToast('❌ Reconnection timeout. Please refresh the page manually.', 'error');
                    isReconnecting = false;
                    
                    // Re-enable reboot button
                    const rebootBtn = document.getElementById('rebootBtn');
                    if (rebootBtn) {
                        rebootBtn.disabled = false;
                        rebootBtn.innerHTML = '<span>🔄</span><span>Reboot</span>';
                    }
                }
            };
            
            // Wait 3 seconds before first attempt (give system time to restart)
            setTimeout(checkConnection, 3000);
        }
        
        // Show notification toast
        function showNotification(message, type = 'info') {
            showToast(message, type);
        }
        
        // Initialize trader controls when DOM is ready
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', initializeTraderControls);
        } else {
            initializeTraderControls();
        }
    