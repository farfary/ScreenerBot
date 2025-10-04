/// HTML templates for the webserver dashboard
///
/// This module contains all HTML/CSS templates organized by component.
/// Each function returns a String that can be used in Axum Html responses.

/// Base HTML template with navigation and common styles
pub fn base_template(title: &str, active_tab: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - ScreenerBot</title>
    <style>
        {common_styles}
    </style>
</head>
<body>
    <div class="header">
        <h1>🤖 ScreenerBot Dashboard</h1>
        <div class="status-indicator">
            <span id="statusBadge" class="badge loading">⏳ Loading...</span>
        </div>
    </div>
    
    <nav class="tabs">
        {nav_tabs}
    </nav>
    
    <main class="content">
        {content}
    </main>
    
    <footer class="footer">
        <p>ScreenerBot v0.1.0 | <a href="/api">API Docs</a> | Built with Rust & Axum</p>
    </footer>
    
    <script>
        {common_scripts}
    </script>
</body>
</html>"#,
        title = title,
        common_styles = common_styles(),
        nav_tabs = nav_tabs(active_tab),
        content = content,
        common_scripts = common_scripts()
    )
}

/// Common CSS styles
fn common_styles() -> &'static str {
    r#"
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f5f7fa;
            color: #2d3748;
            line-height: 1.6;
        }
        
        .header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px 30px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        
        .header h1 {
            font-size: 1.8em;
            font-weight: 600;
        }
        
        .status-indicator {
            display: flex;
            align-items: center;
            gap: 10px;
        }
        
        .badge {
            padding: 6px 16px;
            border-radius: 20px;
            font-size: 0.85em;
            font-weight: 600;
            display: inline-flex;
            align-items: center;
            gap: 6px;
        }
        
        .badge.online {
            background: #10b981;
            color: white;
        }
        
        .badge.loading {
            background: #f59e0b;
            color: white;
            animation: pulse 2s ease-in-out infinite;
        }
        
        .badge.error {
            background: #ef4444;
            color: white;
        }
        
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.7; }
        }
        
        .tabs {
            background: white;
            padding: 0;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            display: flex;
            overflow-x: auto;
        }
        
        .tab {
            padding: 15px 25px;
            text-decoration: none;
            color: #64748b;
            font-weight: 500;
            border-bottom: 3px solid transparent;
            transition: all 0.3s ease;
            white-space: nowrap;
        }
        
        .tab:hover {
            background: #f8fafc;
            color: #667eea;
        }
        
        .tab.active {
            color: #667eea;
            border-bottom-color: #667eea;
            background: #f8fafc;
        }
        
        .content {
            max-width: 1400px;
            margin: 0 auto;
            padding: 20px;
        }
        
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 15px;
            margin-bottom: 20px;
        }
        
        .card {
            background: white;
            border-radius: 8px;
            padding: 20px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            transition: box-shadow 0.3s ease;
        }
        
        .card:hover {
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        }
        
        .card-header {
            display: flex;
            align-items: center;
            gap: 8px;
            margin-bottom: 15px;
            padding-bottom: 10px;
            border-bottom: 2px solid #e2e8f0;
        }
        
        .card-title {
            font-size: 1.1em;
            font-weight: 600;
            color: #1e293b;
        }
        
        .card-icon {
            font-size: 1.3em;
        }
        
        .metric-row {
            display: flex;
            justify-content: space-between;
            padding: 8px 0;
            font-size: 0.9em;
        }
        
        .metric-label {
            color: #64748b;
        }
        
        .metric-value {
            font-weight: 600;
            color: #1e293b;
        }
        
        .service-list {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }
        
        .service-item {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 10px;
            background: #f8fafc;
            border-radius: 6px;
            font-size: 0.9em;
        }
        
        .status-dot {
            width: 10px;
            height: 10px;
            border-radius: 50%;
            display: inline-block;
        }
        
        .status-dot.ready {
            background: #10b981;
            box-shadow: 0 0 8px rgba(16, 185, 129, 0.6);
        }
        
        .status-dot.not-ready {
            background: #ef4444;
            box-shadow: 0 0 8px rgba(239, 68, 68, 0.6);
        }
        
        .table {
            width: 100%;
            border-collapse: collapse;
            font-size: 0.9em;
        }
        
        .table th {
            background: #f8fafc;
            padding: 10px;
            text-align: left;
            font-weight: 600;
            color: #475569;
            border-bottom: 2px solid #e2e8f0;
        }
        
        .table td {
            padding: 10px;
            border-bottom: 1px solid #e2e8f0;
        }
        
        .table tr:hover {
            background: #f8fafc;
        }
        
        .btn {
            padding: 6px 12px;
            border: none;
            border-radius: 5px;
            cursor: pointer;
            font-size: 0.85em;
            font-weight: 500;
            transition: all 0.3s ease;
        }
        
        .btn-primary {
            background: #667eea;
            color: white;
        }
        
        .btn-primary:hover {
            background: #5568d3;
        }
        
        .btn-success {
            background: #10b981;
            color: white;
        }
        
        .btn-success:hover {
            background: #059669;
        }
        
        .footer {
            background: white;
            padding: 15px;
            text-align: center;
            color: #64748b;
            font-size: 0.85em;
            margin-top: 20px;
            box-shadow: 0 -1px 3px rgba(0,0,0,0.1);
        }
        
        .footer a {
            color: #667eea;
            text-decoration: none;
        }
        
        .footer a:hover {
            text-decoration: underline;
        }
        
        .loading-text {
            color: #f59e0b;
            font-style: italic;
        }
        
        .empty-state {
            text-align: center;
            padding: 40px;
            color: #94a3b8;
        }
        
        .empty-state-icon {
            font-size: 3em;
            margin-bottom: 10px;
        }
    "#
}

/// Navigation tabs
fn nav_tabs(active: &str) -> String {
    let tabs = vec![
        ("home", "🏠 Home"),
        ("status", "📊 Status"),
        ("positions", "💰 Positions"),
        ("tokens", "🪙 Tokens"),
        ("events", "📡 Events")
    ];

    tabs.iter()
        .map(|(name, label)| {
            let active_class = if *name == active { " active" } else { "" };
            format!(r#"<a href="/{}" class="tab{}">{}</a>"#, name, active_class, label)
        })
        .collect::<Vec<_>>()
        .join("\n        ")
}

/// Common JavaScript functions
fn common_scripts() -> &'static str {
    r#"
        // Update status badge
        async function updateStatusBadge() {
            try {
                const res = await fetch('/api/v1/status');
                const data = await res.json();
                const badge = document.getElementById('statusBadge');
                
                if (data.all_ready) {
                    badge.className = 'badge online';
                    badge.innerHTML = '✓ Online';
                } else {
                    badge.className = 'badge loading';
                    badge.innerHTML = '⏳ Starting...';
                }
            } catch (error) {
                const badge = document.getElementById('statusBadge');
                badge.className = 'badge error';
                badge.innerHTML = '✗ Error';
            }
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
        
        // Initialize
        updateStatusBadge();
        setInterval(updateStatusBadge, 5000);
    "#
}

/// Home page content
pub fn home_content() -> String {
    r#"
    <div class="grid">
        <div class="card">
            <div class="card-header">
                <span class="card-icon">📊</span>
                <span class="card-title">Quick Stats</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Version</span>
                <span class="metric-value">0.1.0</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Phase</span>
                <span class="metric-value">Phase 1 - Status</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Uptime</span>
                <span class="metric-value loading-text" id="homeUptime">Loading...</span>
            </div>
        </div>
        
        <div class="card">
            <div class="card-header">
                <span class="card-icon">🔌</span>
                <span class="card-title">API Endpoints</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Health Check</span>
                <a href="/api/v1/health" target="_blank" class="metric-value">GET /health</a>
            </div>
            <div class="metric-row">
                <span class="metric-label">System Status</span>
                <a href="/api/v1/status" target="_blank" class="metric-value">GET /status</a>
            </div>
            <div class="metric-row">
                <span class="metric-label">Metrics</span>
                <a href="/api/v1/status/metrics" target="_blank" class="metric-value">GET /metrics</a>
            </div>
        </div>
        
        <div class="card">
            <div class="card-header">
                <span class="card-icon">📚</span>
                <span class="card-title">Documentation</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Architecture</span>
                <span class="metric-value">docs/</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">API Reference</span>
                <a href="/api" target="_blank" class="metric-value">JSON</a>
            </div>
            <div class="metric-row">
                <span class="metric-label">Quick Start</span>
                <span class="metric-value">DASHBOARD_QUICKSTART.md</span>
            </div>
        </div>
    </div>
    
    <script>
        async function loadHomeData() {
            try {
                const res = await fetch('/api/v1/status');
                const data = await res.json();
                document.getElementById('homeUptime').textContent = formatUptime(data.uptime_seconds);
                document.getElementById('homeUptime').classList.remove('loading-text');
            } catch (error) {
                console.error('Failed to load home data:', error);
            }
        }
        
        loadHomeData();
        setInterval(loadHomeData, 5000);
    </script>
    "#.to_string()
}

/// Status page content
pub fn status_content() -> String {
    r#"
    <div class="grid">
        <div class="card">
            <div class="card-header">
                <span class="card-icon">💻</span>
                <span class="card-title">System Info</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Memory Usage</span>
                <span class="metric-value loading-text" id="memory">Loading...</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">CPU Usage</span>
                <span class="metric-value loading-text" id="cpu">Loading...</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Active Threads</span>
                <span class="metric-value loading-text" id="threads">Loading...</span>
            </div>
        </div>
        
        <div class="card">
            <div class="card-header">
                <span class="card-icon">📡</span>
                <span class="card-title">RPC Stats</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Total Calls</span>
                <span class="metric-value loading-text" id="rpcCalls">Loading...</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Success Rate</span>
                <span class="metric-value loading-text" id="successRate">Loading...</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">WebSocket Conns</span>
                <span class="metric-value loading-text" id="wsConns">Loading...</span>
            </div>
        </div>
    </div>
    
    <div class="card">
        <div class="card-header">
            <span class="card-icon">⚙️</span>
            <span class="card-title">Services Status</span>
        </div>
        <div class="service-list" id="servicesList">
            <div class="loading-text">Loading services...</div>
        </div>
    </div>
    
    <script>
        async function loadStatusData() {
            try {
                const [statusRes, metricsRes] = await Promise.all([
                    fetch('/api/v1/status'),
                    fetch('/api/v1/status/metrics')
                ]);
                
                const status = await statusRes.json();
                const metrics = await metricsRes.json();
                
                // Update metrics
                document.getElementById('memory').textContent = metrics.memory_usage_mb + ' MB';
                document.getElementById('cpu').textContent = metrics.cpu_usage_percent.toFixed(1) + '%';
                document.getElementById('threads').textContent = metrics.active_threads;
                document.getElementById('rpcCalls').textContent = formatNumber(metrics.rpc_calls_total);
                document.getElementById('successRate').textContent = metrics.rpc_success_rate.toFixed(1) + '%';
                document.getElementById('wsConns').textContent = metrics.ws_connections;
                
                // Remove loading class
                document.querySelectorAll('.loading-text').forEach(el => el.classList.remove('loading-text'));
                
                // Update services
                const servicesList = document.getElementById('servicesList');
                const serviceNames = {
                    tokens: 'Tokens System',
                    positions: 'Positions Manager',
                    pools: 'Pool Service',
                    transactions: 'Transactions',
                    security: 'Security Analyzer'
                };
                
                servicesList.innerHTML = Object.entries(serviceNames).map(([key, name]) => {
                    const isReady = status.services[key] || false;
                    const dotClass = isReady ? 'ready' : 'not-ready';
                    return `
                        <div class="service-item">
                            <span>${name}</span>
                            <span class="status-dot ${dotClass}"></span>
                        </div>
                    `;
                }).join('');
                
            } catch (error) {
                console.error('Failed to load status data:', error);
            }
        }
        
        loadStatusData();
        setInterval(loadStatusData, 5000);
    </script>
    "#.to_string()
}

/// Positions page content
pub fn positions_content() -> String {
    r#"
    <div class="card">
        <div class="card-header">
            <span class="card-icon">💰</span>
            <span class="card-title">Active Positions</span>
        </div>
        <div class="empty-state">
            <div class="empty-state-icon">📊</div>
            <p>Positions tracking coming in Phase 2</p>
            <p style="font-size: 0.85em; margin-top: 10px;">
                This section will show active trading positions, P&L, and position management.
            </p>
        </div>
    </div>
    "#.to_string()
}

/// Tokens page content
pub fn tokens_content() -> String {
    r#"
    <div class="card">
        <div class="card-header">
            <span class="card-icon">🪙</span>
            <span class="card-title">Token Database</span>
        </div>
        <div class="empty-state">
            <div class="empty-state-icon">🔍</div>
            <p>Token search and analysis coming in Phase 2</p>
            <p style="font-size: 0.85em; margin-top: 10px;">
                This section will show token information, security scores, and market data.
            </p>
        </div>
    </div>
    "#.to_string()
}

/// Events page content
pub fn events_content() -> String {
    r#"
    <div class="card">
        <div class="card-header">
            <span class="card-icon">📡</span>
            <span class="card-title">System Events</span>
        </div>
        <div class="empty-state">
            <div class="empty-state-icon">📋</div>
            <p>Event logs and monitoring coming in Phase 2</p>
            <p style="font-size: 0.85em; margin-top: 10px;">
                This section will show real-time events, trades, and system notifications.
            </p>
        </div>
    </div>
    "#.to_string()
}
