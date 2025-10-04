/// Route aggregation module
///
/// Combines all route modules into the main API router

use axum::{ response::{ Html, Json }, Router };
use serde_json::json;
use std::sync::Arc;

use crate::webserver::state::AppState;

pub mod status;

// Phase 2 routes (future)
// pub mod positions;
// pub mod tokens;
// pub mod transactions;

// Phase 3 routes (future)
// pub mod trading;
// pub mod analytics;
// pub mod config_api;

/// Create the main API router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(home_page))
        .route("/api", axum::routing::get(api_info))
        .nest("/api/v1", api_v1_routes())
        .with_state(state)
}

/// Home page handler - HTML dashboard
async fn home_page() -> Html<String> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ScreenerBot Dashboard</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
            color: #333;
        }
        
        .container {
            max-width: 1200px;
            margin: 0 auto;
        }
        
        .header {
            background: white;
            border-radius: 15px;
            padding: 30px;
            margin-bottom: 20px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
        }
        
        .header h1 {
            color: #667eea;
            font-size: 2.5em;
            margin-bottom: 10px;
        }
        
        .header p {
            color: #666;
            font-size: 1.1em;
        }
        
        .status-badge {
            display: inline-block;
            padding: 5px 15px;
            border-radius: 20px;
            font-size: 0.9em;
            font-weight: bold;
            margin-top: 10px;
        }
        
        .status-online {
            background: #10b981;
            color: white;
        }
        
        .status-loading {
            background: #f59e0b;
            color: white;
        }
        
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 20px;
        }
        
        .card {
            background: white;
            border-radius: 15px;
            padding: 25px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
            transition: transform 0.3s ease;
        }
        
        .card:hover {
            transform: translateY(-5px);
        }
        
        .card h2 {
            color: #667eea;
            margin-bottom: 15px;
            font-size: 1.5em;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        
        .card-icon {
            font-size: 1.2em;
        }
        
        .metric {
            display: flex;
            justify-content: space-between;
            padding: 10px 0;
            border-bottom: 1px solid #eee;
        }
        
        .metric:last-child {
            border-bottom: none;
        }
        
        .metric-label {
            color: #666;
            font-weight: 500;
        }
        
        .metric-value {
            font-weight: bold;
            color: #333;
        }
        
        .service-item {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 10px;
            background: #f9fafb;
            border-radius: 8px;
            margin-bottom: 8px;
        }
        
        .service-status {
            width: 12px;
            height: 12px;
            border-radius: 50%;
            display: inline-block;
        }
        
        .status-ready {
            background: #10b981;
            box-shadow: 0 0 5px #10b981;
        }
        
        .status-not-ready {
            background: #ef4444;
            box-shadow: 0 0 5px #ef4444;
        }
        
        .endpoint {
            background: #f9fafb;
            padding: 12px;
            border-radius: 8px;
            margin-bottom: 10px;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        .endpoint-method {
            background: #667eea;
            color: white;
            padding: 3px 10px;
            border-radius: 5px;
            font-weight: bold;
            font-size: 0.85em;
        }
        
        .endpoint-path {
            color: #333;
            flex: 1;
            margin: 0 15px;
        }
        
        .test-btn {
            background: #10b981;
            color: white;
            border: none;
            padding: 5px 12px;
            border-radius: 5px;
            cursor: pointer;
            font-size: 0.85em;
            transition: background 0.3s ease;
        }
        
        .test-btn:hover {
            background: #059669;
        }
        
        .footer {
            background: white;
            border-radius: 15px;
            padding: 20px;
            text-align: center;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
            color: #666;
        }
        
        .loading {
            color: #f59e0b;
            font-style: italic;
        }
        
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }
        
        .pulse {
            animation: pulse 2s ease-in-out infinite;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🤖 ScreenerBot Dashboard</h1>
            <p>Automated Solana DeFi Trading Bot - Phase 1: System Status</p>
            <span class="status-badge status-loading pulse" id="mainStatus">Loading...</span>
        </div>
        
        <div class="grid">
            <div class="card">
                <h2><span class="card-icon">📊</span> System Status</h2>
                <div class="metric">
                    <span class="metric-label">Version</span>
                    <span class="metric-value">0.1.0</span>
                </div>
                <div class="metric">
                    <span class="metric-label">Uptime</span>
                    <span class="metric-value loading" id="uptime">Loading...</span>
                </div>
                <div class="metric">
                    <span class="metric-label">All Services</span>
                    <span class="metric-value loading" id="allReady">Loading...</span>
                </div>
                <div class="metric">
                    <span class="metric-label">Last Update</span>
                    <span class="metric-value" id="lastUpdate">--:--:--</span>
                </div>
            </div>
            
            <div class="card">
                <h2><span class="card-icon">⚙️</span> Services</h2>
                <div id="servicesContainer" class="loading">Loading services...</div>
            </div>
            
            <div class="card">
                <h2><span class="card-icon">💻</span> System Metrics</h2>
                <div class="metric">
                    <span class="metric-label">Memory Usage</span>
                    <span class="metric-value loading" id="memory">Loading...</span>
                </div>
                <div class="metric">
                    <span class="metric-label">CPU Usage</span>
                    <span class="metric-value loading" id="cpu">Loading...</span>
                </div>
                <div class="metric">
                    <span class="metric-label">RPC Calls</span>
                    <span class="metric-value loading" id="rpcCalls">Loading...</span>
                </div>
                <div class="metric">
                    <span class="metric-label">Active Threads</span>
                    <span class="metric-value loading" id="threads">Loading...</span>
                </div>
            </div>
        </div>
        
        <div class="card">
            <h2><span class="card-icon">🔌</span> API Endpoints</h2>
            <div class="endpoint">
                <span class="endpoint-method">GET</span>
                <span class="endpoint-path">/api/v1/health</span>
                <button class="test-btn" onclick="testEndpoint('/api/v1/health')">Test</button>
            </div>
            <div class="endpoint">
                <span class="endpoint-method">GET</span>
                <span class="endpoint-path">/api/v1/status</span>
                <button class="test-btn" onclick="testEndpoint('/api/v1/status')">Test</button>
            </div>
            <div class="endpoint">
                <span class="endpoint-method">GET</span>
                <span class="endpoint-path">/api/v1/status/services</span>
                <button class="test-btn" onclick="testEndpoint('/api/v1/status/services')">Test</button>
            </div>
            <div class="endpoint">
                <span class="endpoint-method">GET</span>
                <span class="endpoint-path">/api/v1/status/metrics</span>
                <button class="test-btn" onclick="testEndpoint('/api/v1/status/metrics')">Test</button>
            </div>
        </div>
        
        <div class="footer">
            <p>📚 <a href="/api" style="color: #667eea; text-decoration: none;">API Documentation</a> | 
            🔗 Built with Axum & Rust | Auto-refreshing every 5 seconds</p>
        </div>
    </div>
    
    <script>
        async function fetchData() {
            try {
                // Fetch main status
                const statusRes = await fetch('/api/v1/status');
                const status = await statusRes.json();
                
                // Update main status badge
                const mainStatus = document.getElementById('mainStatus');
                if (status.all_ready) {
                    mainStatus.textContent = '● Online';
                    mainStatus.className = 'status-badge status-online';
                } else {
                    mainStatus.textContent = '● Starting...';
                    mainStatus.className = 'status-badge status-loading pulse';
                }
                
                // Update system metrics
                document.getElementById('uptime').textContent = formatUptime(status.uptime_seconds);
                document.getElementById('allReady').textContent = status.all_ready ? '✅ Ready' : '⏳ Starting';
                
                // Update services
                const servicesContainer = document.getElementById('servicesContainer');
                servicesContainer.innerHTML = '';
                servicesContainer.className = '';
                
                const serviceNames = {
                    tokens: 'Tokens System',
                    positions: 'Positions Manager',
                    pools: 'Pool Service',
                    transactions: 'Transactions',
                    security: 'Security Analyzer'
                };
                
                for (const [key, name] of Object.entries(serviceNames)) {
                    const serviceDiv = document.createElement('div');
                    serviceDiv.className = 'service-item';
                    const isReady = status.services[key] || false;
                    serviceDiv.innerHTML = `
                        <span>${name}</span>
                        <span class="service-status ${isReady ? 'status-ready' : 'status-not-ready'}"></span>
                    `;
                    servicesContainer.appendChild(serviceDiv);
                }
                
                // Fetch metrics
                const metricsRes = await fetch('/api/v1/status/metrics');
                const metrics = await metricsRes.json();
                
                document.getElementById('memory').textContent = `${metrics.memory_usage_mb} MB`;
                document.getElementById('cpu').textContent = `${metrics.cpu_usage_percent.toFixed(1)}%`;
                document.getElementById('rpcCalls').textContent = metrics.rpc_calls_total.toLocaleString();
                document.getElementById('threads').textContent = metrics.active_threads;
                
                // Update timestamp
                document.getElementById('lastUpdate').textContent = new Date().toLocaleTimeString();
                
            } catch (error) {
                console.error('Error fetching data:', error);
                document.getElementById('mainStatus').textContent = '● Error';
                document.getElementById('mainStatus').className = 'status-badge status-loading';
            }
        }
        
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
        
        function testEndpoint(path) {
            window.open(path, '_blank');
        }
        
        // Initial fetch
        fetchData();
        
        // Refresh every 5 seconds
        setInterval(fetchData, 5000);
    </script>
</body>
</html>"#.to_string()
    )
}

/// API info page - JSON format for programmatic access
async fn api_info() -> Json<serde_json::Value> {
    Json(
        json!({
        "name": "ScreenerBot API",
        "version": "0.1.0",
        "description": "Automated Solana DeFi trading bot dashboard API",
        "phase": "Phase 1 - System Status",
        "endpoints": {
            "health": "GET /api/v1/health",
            "status": "GET /api/v1/status",
            "services": "GET /api/v1/status/services",
            "metrics": "GET /api/v1/status/metrics"
        },
        "documentation": "See docs/webserver-dashboard-api.md for full API documentation",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })
    )
}

/// API v1 routes
fn api_v1_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Phase 1: System status
        .merge(status::routes())
    // Phase 2: Data access (future)
    // .nest("/positions", positions::routes())
    // .nest("/tokens", tokens::routes())
    // .nest("/transactions", transactions::routes())
    // Phase 3: Operations (future)
    // .nest("/trading", trading::routes())
    // .nest("/analytics", analytics::routes())
    // .nest("/config", config_api::routes())
}
