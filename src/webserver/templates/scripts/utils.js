(function () {
    const global = window;

    function coerceNumber(value) {
        if (value === null || value === undefined || value === '') {
            return Number.NaN;
        }
        const num = Number(value);
        return Number.isFinite(num) ? num : Number.NaN;
    }

    function formatNumber(value, decimalsOrOptions = 2, maybeOptions = {}) {
        let decimals = decimalsOrOptions;
        let options = maybeOptions;

        if (typeof decimalsOrOptions === 'object' && decimalsOrOptions !== null) {
            options = decimalsOrOptions;
            decimals = options.decimals ?? 2;
        }

        const {
            fallback = '—',
            useGrouping = true,
            locale = 'en-US',
        } = options || {};

        const num = coerceNumber(value);
        if (!Number.isFinite(num)) {
            return fallback;
        }

        return num.toLocaleString(locale, {
            minimumFractionDigits: decimals,
            maximumFractionDigits: decimals,
            useGrouping,
        });
    }

    function formatCompactNumber(value, digits = 2, fallback = '—') {
        const num = coerceNumber(value);
        if (!Number.isFinite(num)) {
            return fallback;
        }

        return Intl.NumberFormat('en-US', {
            notation: 'compact',
            maximumFractionDigits: digits,
        }).format(num);
    }

    function formatBooleanFlag(value, unknownLabel = 'Unknown') {
        if (value === true) return 'Yes';
        if (value === false) return 'No';
        return unknownLabel;
    }

    function formatCurrencyUSD(value, { fallback = '—' } = {}) {
        const num = coerceNumber(value);
        if (!Number.isFinite(num)) {
            return fallback;
        }

        const abs = Math.abs(num);
        let scaled = num;
        let suffix = '';

        if (abs >= 1_000_000_000) {
            scaled = num / 1_000_000_000;
            suffix = 'B';
        } else if (abs >= 1_000_000) {
            scaled = num / 1_000_000;
            suffix = 'M';
        } else if (abs >= 1_000) {
            scaled = num / 1_000;
            suffix = 'K';
        }

        return `$${scaled.toFixed(2)}${suffix}`;
    }

    function formatPriceSol(price, { fallback = 'N/A' } = {}) {
        const num = coerceNumber(price);
        if (!Number.isFinite(num)) {
            return fallback;
        }
        if (num === 0) return '0';
        return Math.abs(num) < 0.000001 ? num.toExponential(4) : num.toFixed(8);
    }

    function formatPercentValue(value, {
        fallback = '—',
        decimals = 2,
        includeSign = true,
    } = {}) {
        const num = coerceNumber(value);
        if (!Number.isFinite(num)) {
            return fallback;
        }

        const magnitude = Math.abs(num).toFixed(decimals);
        if (!includeSign) {
            return `${magnitude}%`;
        }

        if (num > 0) return `+${magnitude}%`;
        if (num < 0) return `-${magnitude}%`;
        return `${magnitude}%`;
    }

    function formatPercent(value, {
        style = 'plain',
        decimals = 2,
        fallback = '-',
    } = {}) {
        const num = coerceNumber(value);
        if (!Number.isFinite(num)) {
            if (style === 'token') {
                return `<span>${fallback}</span>`;
            }
            return fallback;
        }

        if (style === 'token') {
            const color = num > 0 ? '#16a34a' : num < 0 ? '#ef4444' : 'inherit';
            const sign = num > 0 ? '+' : '';
            return `<span style="color:${color};">${sign}${num.toFixed(decimals)}%</span>`;
        }

        if (style === 'pnl') {
            const magnitude = Math.abs(num).toFixed(decimals);
            if (num > 0) {
                return `<span class="pnl-positive">+${magnitude}%</span>`;
            }
            if (num < 0) {
                return `<span class="pnl-negative">-${magnitude}%</span>`;
            }
            return `<span class="pnl-neutral">${magnitude}%</span>`;
        }

        const sign = num > 0 ? '+' : num < 0 ? '-' : '';
        return `${sign}${Math.abs(num).toFixed(decimals)}%`;
    }

    function formatSol(amount, {
        decimals = 4,
        fallback = '-',
        suffix = ' SOL',
    } = {}) {
        const num = coerceNumber(amount);
        if (!Number.isFinite(num)) {
            return fallback;
        }
        const formatted = num.toFixed(decimals);
        return `${formatted}${suffix}`;
    }

    function formatPnL(value, {
        decimals = 4,
        fallback = '-',
    } = {}) {
        const num = coerceNumber(value);
        if (!Number.isFinite(num)) {
            return fallback;
        }

        const formatted = formatSol(Math.abs(num), { decimals, fallback: fallback === '-' ? '-' : fallback });
        if (formatted === fallback) {
            return fallback;
        }

        if (num > 0) {
            return `<span class="pnl-positive">+${formatted}</span>`;
        }
        if (num < 0) {
            return `<span class="pnl-negative">-${formatted}</span>`;
        }
        return `<span class="pnl-neutral">${formatted}</span>`;
    }

    function formatTimeFromSeconds(timestamp, {
        fallback = '-',
        locale = 'en-US',
        includeSeconds = false,
    } = {}) {
        const num = coerceNumber(timestamp);
        if (!Number.isFinite(num)) {
            return fallback;
        }
        const date = new Date(num * 1000);
        if (Number.isNaN(date.getTime())) {
            return fallback;
        }
        const options = {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        };
        if (includeSeconds) {
            options.second = '2-digit';
        }
        return date.toLocaleString(locale, options);
    }

    function toDate(value) {
        if (value instanceof Date) {
            return Number.isNaN(value.getTime()) ? null : value;
        }

        if (typeof value === 'number') {
            const num = value > 1e12 ? value : value * 1000;
            const date = new Date(num);
            return Number.isNaN(date.getTime()) ? null : date;
        }

        if (typeof value === 'string' && value.trim() !== '') {
            const numeric = Number(value);
            if (Number.isFinite(numeric)) {
                const num = numeric > 1e12 ? numeric : numeric * 1000;
                const date = new Date(num);
                if (!Number.isNaN(date.getTime())) {
                    return date;
                }
            }
            const parsed = new Date(value);
            return Number.isNaN(parsed.getTime()) ? null : parsed;
        }

        return null;
    }

    function formatTimestamp(value, {
        fallback = 'N/A',
        includeSeconds = true,
        locale = 'en-US',
    } = {}) {
        const date = toDate(value);
        if (!date) {
            return fallback;
        }
        const options = {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        };
        if (includeSeconds) {
            options.second = '2-digit';
        }
        return date.toLocaleString(locale, options);
    }

    function formatTimeAgo(value, { fallback = '-' } = {}) {
        const date = toDate(value);
        if (!date) {
            return fallback;
        }
        const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
        if (seconds < 0) {
            return '0s ago';
        }
        if (seconds < 60) return `${seconds}s ago`;
        const minutes = Math.floor(seconds / 60);
        if (minutes < 60) return `${minutes}m ago`;
        const hours = Math.floor(minutes / 60);
        if (hours < 24) return `${hours}h ago`;
        const days = Math.floor(hours / 24);
        return `${days}d ago`;
    }

    function formatUptime(seconds, {
        fallback = '0s',
        style = 'detailed',
    } = {}) {
        const num = coerceNumber(seconds);
        if (!Number.isFinite(num) || num < 0) {
            return fallback;
        }

        const total = Math.floor(num);
        const days = Math.floor(total / 86400);
        const hours = Math.floor((total % 86400) / 3600);
        const minutes = Math.floor((total % 3600) / 60);
        const remainingSeconds = total % 60;

        if (style === 'compact') {
            if (total < 60) return `${total}s`;
            if (total < 3600) return `${Math.floor(total / 60)}m`;
            if (total < 86400) return `${Math.floor(total / 3600)}h ${Math.floor((total % 3600) / 60)}m`;
            return `${days}d ${hours}h`;
        }

        if (days > 0) return `${days}d ${hours}h ${minutes}m`;
        if (hours > 0) return `${hours}h ${minutes}m ${remainingSeconds}s`;
        if (minutes > 0) return `${minutes}m ${remainingSeconds}s`;
        return `${remainingSeconds}s`;
    }

    function formatBytes(bytes, fallback = '0 B') {
        const num = coerceNumber(bytes);
        if (!Number.isFinite(num) || num < 0) {
            return fallback;
        }
        if (num < 1024) return `${num} B`;
        if (num < 1_048_576) return `${(num / 1024).toFixed(1)} KB`;
        if (num < 1_073_741_824) return `${(num / 1_048_576).toFixed(1)} MB`;
        return `${(num / 1_073_741_824).toFixed(2)} GB`;
    }

    function formatDuration(nanos, fallback = '0ns') {
        const num = coerceNumber(nanos);
        if (!Number.isFinite(num) || num < 0) {
            return fallback;
        }
        if (num < 1_000) return `${num}ns`;
        if (num < 1_000_000) return `${(num / 1_000).toFixed(1)}µs`;
        if (num < 1_000_000_000) return `${(num / 1_000_000).toFixed(1)}ms`;
        return `${(num / 1_000_000_000).toFixed(2)}s`;
    }

    function escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text ?? '';
        return div.innerHTML;
    }

    function setText(id, value) {
        const el = document.getElementById(id);
        if (el) {
            el.textContent = value;
        }
        return el;
    }

    function setHtml(id, value) {
        const el = document.getElementById(id);
        if (el) {
            el.innerHTML = value;
        }
        return el;
    }

    function closeDropdownMenus() {
        document.querySelectorAll('.dropdown-menu.show').forEach((menu) => {
            menu.classList.remove('show');
            menu.style.position = '';
            menu.style.top = '';
            menu.style.left = '';
            menu.style.right = '';
            menu.style.width = '';
        });
    }

    function toggleDropdown(event) {
        event.stopPropagation();
        const btn = event.currentTarget;
        if (!btn) return;
        const menu = btn.nextElementSibling;
        if (!menu) return;

        const isOpen = menu.classList.contains('show');
        closeDropdownMenus();
        if (isOpen) {
            return;
        }

        const rect = btn.getBoundingClientRect();
        const menuWidth = Math.max(200, menu.offsetWidth || 200);
        const viewportWidth = window.innerWidth;
        const rightSpace = viewportWidth - rect.right;

        menu.classList.add('show');
        menu.style.position = 'fixed';
        menu.style.top = `${Math.round(rect.bottom + 4)}px`;
        if (rightSpace < menuWidth) {
            menu.style.left = `${Math.max(8, Math.round(rect.right - menuWidth))}px`;
            menu.style.right = '';
        } else {
            menu.style.left = `${Math.round(rect.left)}px`;
            menu.style.right = '';
        }
        menu.style.width = `${menuWidth}px`;
    }

    document.addEventListener('click', closeDropdownMenus);
    ['scroll', 'resize'].forEach((evt) => {
        window.addEventListener(evt, closeDropdownMenus, { passive: true });
    });

    function ensureToastContainer() {
        let container = document.getElementById('toastContainer');
        if (!container) {
            container = document.createElement('div');
            container.id = 'toastContainer';
            container.className = 'toast-container';
            document.body.appendChild(container);
        }
        return container;
    }

    function showToast(message, type = 'success') {
        const container = ensureToastContainer();
        const toast = document.createElement('div');
        toast.className = 'toast' + (type === 'error' ? ' error' : type === 'info' ? ' info' : '');
        toast.innerHTML = `<div class="toast-message">${message}</div>`;
        container.appendChild(toast);
        setTimeout(() => {
            toast.style.opacity = '0';
            setTimeout(() => toast.remove(), 300);
        }, 3000);
    }

    function showNotification(message, type = 'info') {
        showToast(message, type);
    }

    function copyToClipboard(value) {
        return navigator.clipboard.writeText(value);
    }

    function copyMint(mint) {
        return copyToClipboard(mint)
            .then(() => showToast('✅ Mint address copied to clipboard!'))
            .catch((err) => {
                showToast(`❌ Failed to copy: ${err}`, 'error');
                throw err;
            });
    }

    function copyDebugValue(value, label) {
        return copyToClipboard(value)
            .then(() => showToast(`✅ ${label} copied to clipboard!`))
            .catch((err) => {
                showToast(`❌ Failed to copy: ${err}`, 'error');
                throw err;
            });
    }

    async function copyDebugInfo(mint, type) {
        try {
            const endpoint = type === 'position' ? `/api/positions/${mint}/debug` : `/api/tokens/${mint}/debug`;
            const res = await fetch(endpoint);
            if (!res.ok) {
                throw new Error(`HTTP ${res.status}`);
            }
            const data = await res.json();
            const text = generateDebugText(data, type);
            await copyToClipboard(text);
            showToast('✅ Debug info copied to clipboard!');
        } catch (err) {
            console.error('copyDebugInfo error:', err);
            showToast(`❌ Failed to copy debug info: ${err}`, 'error');
            throw err;
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

        lines.push('ScreenerBot Debug Info');
        lines.push(`Mint: ${data.mint || 'N/A'}`);
        if (tokenInfo.symbol || tokenInfo.name) {
            lines.push(`Token: ${tokenInfo.symbol || 'N/A'} ${tokenInfo.name ? '(' + tokenInfo.name + ')' : ''}`);
        }
        lines.push('');

        lines.push('[Token]');
        lines.push(`Symbol: ${tokenInfo.symbol ?? 'N/A'}`);
        lines.push(`Name: ${tokenInfo.name ?? 'N/A'}`);
        lines.push(`Decimals: ${tokenInfo.decimals ?? 'N/A'}`);
        lines.push(`Website: ${tokenInfo.website ?? 'N/A'}`);
        lines.push(`Verified: ${tokenInfo.is_verified ? 'Yes' : 'No'}`);
        const tags = Array.isArray(tokenInfo.tags) ? tokenInfo.tags.join(', ') : 'None';
        lines.push(`Tags: ${tags}`);
        lines.push('');

        lines.push('[Price & Market]');
        lines.push(`Price (SOL): ${price.pool_price_sol != null ? Number(price.pool_price_sol).toPrecision(10) : 'N/A'}`);
        lines.push(`Confidence: ${price.confidence != null ? (Number(price.confidence) * 100).toFixed(1) + '%' : 'N/A'}`);
        lines.push(`Last Updated: ${price.last_updated ? new Date(price.last_updated * 1000).toISOString() : 'N/A'}`);
        lines.push(`Market Cap: ${market.market_cap != null ? '$' + Number(market.market_cap).toLocaleString() : 'N/A'}`);
        lines.push(`FDV: ${market.fdv != null ? '$' + Number(market.fdv).toLocaleString() : 'N/A'}`);
        lines.push(`Liquidity: ${market.liquidity_usd != null ? '$' + Number(market.liquidity_usd).toLocaleString() : 'N/A'}`);
        lines.push(`24h Volume: ${market.volume_24h != null ? '$' + Number(market.volume_24h).toLocaleString() : 'N/A'}`);
        lines.push('');

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
            risks.forEach((r) => lines.push(`  - ${r.name || 'Unknown'}: ${r.level || 'N/A'} (${r.description || ''})`));
        } else {
            lines.push('Risks: None');
        }
        lines.push('');

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

        if (data.pool_debug) {
            const pd = data.pool_debug;
            lines.push('[Pool Debug]');
            if (pd.price_history && pd.price_history.length > 0) {
                lines.push(`Price History Points: ${pd.price_history.length}`);
                lines.push('Recent Prices (last 10):');
                pd.price_history.slice(0, 10).forEach((p, i) => {
                    const date = new Date(p.timestamp * 1000).toISOString();
                    lines.push(`  ${i + 1}. ${date} - ${Number(p.price_sol).toPrecision(10)} SOL (conf: ${(p.confidence * 100).toFixed(1)}%)`);
                });
            }
            if (pd.price_stats) {
                const ps = pd.price_stats;
                lines.push(`Min Price: ${Number(ps.min_price).toPrecision(10)} SOL`);
                lines.push(`Max Price: ${Number(ps.max_price).toPrecision(10)} SOL`);
                lines.push(`Avg Price: ${Number(ps.avg_price).toPrecision(10)} SOL`);
                lines.push(`Volatility: ${Number(ps.price_volatility).toFixed(2)}%`);
                lines.push(`Data Points: ${ps.data_points}`);
                lines.push(`Time Span: ${ps.time_span_seconds}s (${(ps.time_span_seconds / 60).toFixed(0)} min)`);
            }
            if (pd.all_pools && pd.all_pools.length > 0) {
                lines.push(`All Pools (${pd.all_pools.length}):`);
                pd.all_pools.forEach((pool, i) => {
                    lines.push(`  Pool #${i + 1}: ${pool.pool_address}`);
                    lines.push(`    DEX: ${pool.dex_name}`);
                });
            }
            if (pd.cache_stats) {
                lines.push(`Cache - Total: ${pd.cache_stats.total_tokens_cached}, Fresh: ${pd.cache_stats.fresh_prices}, History: ${pd.cache_stats.history_entries}`);
            }
            lines.push('');
        }

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

        if (data.position_debug) {
            const pd = data.position_debug;
            lines.push('[Position Debug]');
            if (pd.transaction_details) {
                lines.push('Transactions:');
                lines.push(`  Entry: ${pd.transaction_details.entry_signature || 'N/A'} (verified: ${pd.transaction_details.entry_verified})`);
                lines.push(`  Exit: ${pd.transaction_details.exit_signature || 'N/A'} (verified: ${pd.transaction_details.exit_verified})`);
                if (pd.transaction_details.synthetic_exit) {
                    lines.push('  Synthetic Exit: Yes');
                }
                if (pd.transaction_details.closed_reason) {
                    lines.push(`  Closed Reason: ${pd.transaction_details.closed_reason}`);
                }
            }
            if (pd.fee_details) {
                lines.push('Fees:');
                lines.push(`  Entry: ${pd.fee_details.entry_fee_sol?.toFixed(6) || 'N/A'} SOL (${pd.fee_details.entry_fee_lamports || 0} lamports)`);
                lines.push(`  Exit: ${pd.fee_details.exit_fee_sol?.toFixed(6) || 'N/A'} SOL (${pd.fee_details.exit_fee_lamports || 0} lamports)`);
                lines.push(`  Total: ${pd.fee_details.total_fees_sol.toFixed(6)} SOL`);
            }
            if (pd.profit_targets) {
                lines.push(`Profit Targets: Min ${pd.profit_targets.min_target_percent || 'N/A'}%, Max ${pd.profit_targets.max_target_percent || 'N/A'}%`);
                lines.push(`Liquidity Tier: ${pd.profit_targets.liquidity_tier || 'N/A'}`);
            }
            if (pd.price_tracking) {
                lines.push('Price Tracking:');
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
                lines.push('Phantom:');
                lines.push(`  Remove Flag: ${pd.phantom_details.phantom_remove}`);
                lines.push(`  Confirmations: ${pd.phantom_details.phantom_confirmations}`);
                if (pd.phantom_details.phantom_first_seen) {
                    lines.push(`  First Seen: ${pd.phantom_details.phantom_first_seen}`);
                }
            }
            if (pd.proceeds_metrics) {
                const pm = pd.proceeds_metrics;
                lines.push('Proceeds Metrics:');
                lines.push(`  Accepted: ${pm.accepted_quotes} (${pm.accepted_profit_quotes} profit, ${pm.accepted_loss_quotes} loss)`);
                lines.push(`  Rejected: ${pm.rejected_quotes}`);
                lines.push(`  Avg Shortfall: ${pm.average_shortfall_bps.toFixed(2)} bps`);
                lines.push(`  Worst Shortfall: ${pm.worst_shortfall_bps} bps`);
            }
            lines.push('');
        }

        return lines.join('\n');
    }

    function openGMGN(mint) {
        window.open(`https://gmgn.ai/sol/token/${mint}`, '_blank');
    }

    function openDexScreener(mint) {
        window.open(`https://dexscreener.com/solana/${mint}`, '_blank');
    }

    function openSolscan(mint) {
        window.open(`https://solscan.io/token/${mint}`, '_blank');
    }

    const Utils = {
        formatNumber,
        formatCompactNumber,
        formatBooleanFlag,
        formatCurrencyUSD,
        formatPriceSol,
        formatPercentValue,
        formatPercent,
        formatSol,
        formatPnL,
        formatTimeFromSeconds,
        formatTimestamp,
        formatTimeAgo,
        formatUptime,
        formatBytes,
        formatDuration,
        escapeHtml,
        setText,
        setHtml,
        toggleDropdown,
        closeDropdownMenus,
        showToast,
        showNotification,
        copyToClipboard,
        copyMint,
        copyDebugValue,
        copyDebugInfo,
        generateDebugText,
        openGMGN,
        openDexScreener,
        openSolscan,
    };

    global.Utils = Object.assign(global.Utils || {}, Utils);
})();
