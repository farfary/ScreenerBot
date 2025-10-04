/// HTML templates for the webserver dashboard
///
/// This module contains all HTML/CSS templates organized by component.
/// Each function returns a String that can be used in Axum Html responses.

/// Base HTML template with navigation and common styles
pub fn base_template(title: &str, active_tab: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en" data-theme="light">
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
        <div class="header-controls">
            <button class="theme-toggle" id="themeToggle" aria-label="Toggle theme">
                <span id="themeIcon">🌙</span>
                <span id="themeText">Dark</span>
            </button>
            <div class="status-indicator">
                <span id="statusBadge" class="badge loading">⏳ Loading...</span>
            </div>
        </div>
    </div>
    
    <nav class="tabs">
        {nav_tabs}
    </nav>
    
    <main class="content">
        {content}
    </main>
    
    <!-- Debug Modal -->
    <div id="debugModal" class="modal-overlay" onclick="if(event.target === this) closeDebugModal()">
        <div class="modal-content" onclick="event.stopPropagation()">
            <div class="modal-header">
                <span class="modal-title">🐛 Debug Information</span>
                <button class="modal-close" onclick="closeDebugModal()" aria-label="Close">×</button>
            </div>
            <div class="modal-body">
                <div class="modal-tabs">
                    <button class="modal-tab active" onclick="switchDebugTab('token')">Token Info</button>
                    <button class="modal-tab" onclick="switchDebugTab('price')">Price & Market</button>
                    <button class="modal-tab" onclick="switchDebugTab('pool')">Pool Data</button>
                    <button class="modal-tab" onclick="switchDebugTab('security')">Security</button>
                    <button class="modal-tab" onclick="switchDebugTab('position')">Position History</button>
                </div>
                
                <!-- Token Info Tab -->
                <div id="tab-token" class="modal-tab-content active">
                    <div class="debug-section">
                        <h3 class="debug-section-title">Token Information</h3>
                        <div class="debug-row">
                            <span class="debug-label">Mint Address:</span>
                            <span class="debug-value" id="debugMintAddress">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Symbol:</span>
                            <span class="debug-value" id="tokenSymbol">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Name:</span>
                            <span class="debug-value" id="tokenName">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Decimals:</span>
                            <span class="debug-value" id="tokenDecimals">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Website:</span>
                            <span class="debug-value" id="tokenWebsite">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Verified:</span>
                            <span class="debug-value" id="tokenVerified">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Tags:</span>
                            <span class="debug-value" id="tokenTags">Loading...</span>
                        </div>
                    </div>
                </div>
                
                <!-- Price & Market Tab -->
                <div id="tab-price" class="modal-tab-content">
                    <div class="debug-section">
                        <h3 class="debug-section-title">Current Price</h3>
                        <div class="debug-row">
                            <span class="debug-label">Price (SOL):</span>
                            <span class="debug-value" id="priceSol">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Confidence:</span>
                            <span class="debug-value" id="priceConfidence">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Last Updated:</span>
                            <span class="debug-value" id="priceUpdated">Loading...</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Market Data</h3>
                        <div class="debug-row">
                            <span class="debug-label">Market Cap:</span>
                            <span class="debug-value" id="marketCap">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">FDV:</span>
                            <span class="debug-value" id="fdv">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Liquidity:</span>
                            <span class="debug-value" id="liquidity">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">24h Volume:</span>
                            <span class="debug-value" id="volume24h">Loading...</span>
                        </div>
                    </div>
                </div>
                
                <!-- Pool Data Tab -->
                <div id="tab-pool" class="modal-tab-content">
                    <div id="poolsList">
                        <p>Loading pool data...</p>
                    </div>
                </div>
                
                <!-- Security Tab -->
                <div id="tab-security" class="modal-tab-content">
                    <div class="debug-section">
                        <h3 class="debug-section-title">Security Overview</h3>
                        <div class="debug-row">
                            <span class="debug-label">Security Score:</span>
                            <span class="debug-value" id="securityScore">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Rugged:</span>
                            <span class="debug-value" id="securityRugged">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Total Holders:</span>
                            <span class="debug-value" id="securityHolders">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Top 10 Concentration:</span>
                            <span class="debug-value" id="securityTop10">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Mint Authority:</span>
                            <span class="debug-value" id="securityMintAuth">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Freeze Authority:</span>
                            <span class="debug-value" id="securityFreezeAuth">Loading...</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Risk Factors</h3>
                        <div id="securityRisks">
                            <p>Loading risks...</p>
                        </div>
                    </div>
                </div>
                
                <!-- Position History Tab -->
                <div id="tab-position" class="modal-tab-content">
                    <div class="debug-section">
                        <h3 class="debug-section-title">Position Summary</h3>
                        <div class="debug-row">
                            <span class="debug-label">Open Positions:</span>
                            <span class="debug-value" id="positionOpenPositions">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Closed Positions:</span>
                            <span class="debug-value" id="positionClosedCount">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Total P&L:</span>
                            <span class="debug-value" id="positionTotalPnL">Loading...</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Win Rate:</span>
                            <span class="debug-value" id="positionWinRate">Loading...</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Current Position</h3>
                        <div class="debug-row">
                            <span class="debug-label">Entry Price:</span>
                            <span class="debug-value" id="positionEntryPrice">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Entry Size:</span>
                            <span class="debug-value" id="positionEntrySize">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Current Price:</span>
                            <span class="debug-value" id="positionCurrentPrice">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Unrealized P&L:</span>
                            <span class="debug-value" id="positionUnrealizedPnL">N/A</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
    
    <script>
        {common_scripts}
        {theme_scripts}
    </script>
</body>
</html>"#,
        title = title,
        common_styles = common_styles(),
        nav_tabs = nav_tabs(active_tab),
        content = content,
        common_scripts = common_scripts(),
        theme_scripts = theme_scripts()
    )
}

/// Common CSS styles
fn common_styles() -> &'static str {
    r#"
        /* CSS Variables for Theming */
        :root {
            /* Light mode (default) */
            --bg-primary: #f5f7fa;
            --bg-secondary: #ffffff;
            --bg-card: #ffffff;
            --bg-card-hover: #f8fafc;
            --text-primary: #2d3748;
            --text-secondary: #718096;
            --text-muted: #a0aec0;
            --border-color: #e2e8f0;
            --header-bg: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            --header-text: #ffffff;
            --link-color: #667eea;
            --link-hover: #5568d3;
            --badge-online: #10b981;
            --badge-error: #ef4444;
            --badge-loading: #f59e0b;
            --shadow-sm: rgba(0,0,0,0.05);
            --shadow-md: rgba(0,0,0,0.1);
            --shadow-lg: rgba(0,0,0,0.2);
            --table-header-bg: #f8fafc;
            --table-header-text: #475569;
            --service-item-bg: #f8fafc;
        }
        
        /* Dark mode */
        [data-theme="dark"] {
            --bg-primary: #1a202c;
            --bg-secondary: #2d3748;
            --bg-card: #2d3748;
            --bg-card-hover: #374151;
            --text-primary: #e2e8f0;
            --text-secondary: #cbd5e0;
            --text-muted: #718096;
            --border-color: #4a5568;
            --header-bg: linear-gradient(135deg, #4c51bf 0%, #553c9a 100%);
            --header-text: #ffffff;
            --link-color: #818cf8;
            --link-hover: #a5b4fc;
            --badge-online: #34d399;
            --badge-error: #f87171;
            --badge-loading: #fbbf24;
            --shadow-sm: rgba(0,0,0,0.2);
            --shadow-md: rgba(0,0,0,0.3);
            --shadow-lg: rgba(0,0,0,0.5);
            --table-header-bg: #374151;
            --table-header-text: #cbd5e0;
            --service-item-bg: #374151;
        }
        
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
            transition: background-color 0.3s ease, color 0.3s ease, border-color 0.3s ease;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.6;
            height: 100vh;
            display: flex;
            flex-direction: column;
        }
        
        .header {
            background: var(--header-bg);
            color: var(--header-text);
            padding: 8px 14px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            box-shadow: 0 2px 10px var(--shadow-md);
        }
        
        .header h1 {
            font-size: 1.1em;
            font-weight: 600;
        }
        
        .header-controls {
            display: flex;
            align-items: center;
            gap: 10px;
        }
        
        .status-indicator {
            display: flex;
            align-items: center;
            gap: 10px;
        }
        
        /* Theme Toggle Button */
        .theme-toggle {
            background: rgba(255,255,255,0.2);
            border: none;
            color: var(--header-text);
            padding: 4px 10px;
            border-radius: 14px;
            cursor: pointer;
            font-size: 0.9em;
            display: flex;
            align-items: center;
            gap: 8px;
            transition: background 0.3s ease;
        }
        
        .theme-toggle:hover {
            background: rgba(255,255,255,0.3);
        }
        
        .theme-toggle:active {
            transform: scale(0.95);
        }
        
        .badge {
            padding: 4px 10px;
            border-radius: 14px;
            font-size: 0.8em;
            font-weight: 600;
            display: inline-flex;
            align-items: center;
            gap: 6px;
        }
        
        .badge.online {
            background: var(--badge-online);
            color: white;
        }
        
        .badge.loading {
            background: var(--badge-loading);
            color: white;
            animation: pulse 2s ease-in-out infinite;
        }
        
        .badge.error {
            background: var(--badge-error);
            color: white;
        }
        
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.7; }
        }
        
        .tabs {
            background: var(--bg-secondary);
            padding: 0 4px;
            box-shadow: 0 1px 3px var(--shadow-md);
            display: flex;
            overflow-x: auto;
        }
        
        .tab {
            padding: 8px 12px;
            text-decoration: none;
            color: var(--text-secondary);
            font-weight: 500;
            border-bottom: 3px solid transparent;
            transition: all 0.3s ease;
            white-space: nowrap;
            font-size: 0.95em;
        }
        
        .tab:hover {
            background: var(--bg-card-hover);
            color: var(--link-color);
        }
        
        .tab.active {
            color: var(--link-color);
            border-bottom-color: var(--link-color);
            background: var(--bg-card-hover);
        }
        
        .content {
            flex: 1;
            display: flex;
            flex-direction: column;
            min-height: 0; /* allow child to size/scroll */
            padding: 10px;
        }
        
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 15px;
            margin-bottom: 20px;
        }
        
        .card {
            background: var(--bg-card);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 20px;
            box-shadow: 0 1px 3px var(--shadow-sm);
            transition: box-shadow 0.3s ease;
        }
        
        .card:hover {
            box-shadow: 0 4px 12px var(--shadow-md);
        }
        
        .card-header {
            display: flex;
            align-items: center;
            gap: 8px;
            margin-bottom: 15px;
            padding-bottom: 10px;
            border-bottom: 2px solid var(--border-color);
        }
        
        .card-title {
            font-size: 1.1em;
            font-weight: 600;
            color: var(--text-primary);
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
            color: var(--text-secondary);
        }
        
        .metric-value {
            font-weight: 600;
            color: var(--text-primary);
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
            background: var(--service-item-bg);
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
            background: var(--badge-online);
            box-shadow: 0 0 8px rgba(16, 185, 129, 0.6);
        }
        
        .status-dot.not-ready {
            background: var(--badge-error);
            box-shadow: 0 0 8px rgba(239, 68, 68, 0.6);
        }
        
        .table {
            width: 100%;
            border-collapse: collapse;
            font-size: 0.9em;
        }
        
        .table th {
            background: var(--table-header-bg);
            padding: 10px;
            text-align: left;
            font-weight: 600;
            color: var(--table-header-text);
            border-bottom: 2px solid var(--border-color);
        }
        
        .table td {
            padding: 10px;
            border-bottom: 1px solid var(--border-color);
        }
        
        .table tr:hover {
            background: var(--bg-card-hover);
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
            background: var(--link-color);
            color: white;
        }
        
        .btn-primary:hover {
            background: var(--link-hover);
        }
        
        .btn-success {
            background: var(--badge-online);
            color: white;
        }
        
        .btn-success:hover {
            background: #059669;
        }
        

        
        .loading-text {
            color: var(--badge-loading);
            font-style: italic;
        }
        
        .empty-state {
            text-align: center;
            padding: 40px;
            color: var(--text-muted);
        }
        
        .empty-state-icon {
            font-size: 3em;
            margin-bottom: 10px;
        }
        
        /* Dropdown Menu Styles */
        .dropdown-container {
            position: relative;
            display: inline-block;
        }
        
        .dropdown-btn {
            background: var(--bg-card);
            border: 1px solid var(--border-color);
            border-radius: 4px;
            padding: 4px 10px;
            font-size: 1.2em;
            font-weight: bold;
            cursor: pointer;
            color: var(--text-primary);
            transition: all 0.2s ease;
        }
        
        .dropdown-btn:hover {
            background: var(--bg-card-hover);
            border-color: var(--link-color);
        }
        
        .dropdown-menu {
            display: none;
            position: absolute;
            right: 0;
            top: 100%;
            margin-top: 4px;
            background: var(--bg-card);
            border: 1px solid var(--border-color);
            border-radius: 6px;
            box-shadow: 0 4px 12px var(--shadow-lg);
            min-width: 200px;
            z-index: 1000;
            overflow: hidden;
        }
        
        .dropdown-menu.show {
            display: block;
        }
        
        .dropdown-item {
            display: block;
            width: 100%;
            padding: 10px 16px;
            text-align: left;
            background: var(--bg-card);
            border: none;
            border-bottom: 1px solid var(--border-color);
            color: var(--text-primary);
            cursor: pointer;
            font-size: 0.9em;
            transition: background 0.2s ease;
        }
        
        .dropdown-item:last-child {
            border-bottom: none;
        }
        
        .dropdown-item:hover {
            background: var(--bg-card-hover);
            color: var(--link-color);
        }
        
        /* Modal Styles */
        .modal-overlay {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0, 0, 0, 0.7);
            z-index: 2000;
            backdrop-filter: blur(4px);
        }
        
        .modal-overlay.show {
            display: flex;
            align-items: center;
            justify-content: center;
        }
        
        .modal-content {
            background: var(--bg-card);
            border-radius: 12px;
            width: 90%;
            max-width: 900px;
            max-height: 90vh;
            overflow: hidden;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
            animation: modalSlideIn 0.3s ease-out;
        }
        
        @keyframes modalSlideIn {
            from {
                opacity: 0;
                transform: translateY(-50px);
            }
            to {
                opacity: 1;
                transform: translateY(0);
            }
        }
        
        .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 20px 24px;
            border-bottom: 2px solid var(--border-color);
            background: var(--table-header-bg);
        }
        
        .modal-title {
            font-size: 1.4em;
            font-weight: 600;
            color: var(--text-primary);
        }
        
        .modal-close {
            background: none;
            border: none;
            font-size: 1.8em;
            color: var(--text-secondary);
            cursor: pointer;
            padding: 0;
            width: 32px;
            height: 32px;
            display: flex;
            align-items: center;
            justify-content: center;
            border-radius: 4px;
            transition: all 0.2s ease;
        }
        
        .modal-close:hover {
            background: var(--bg-card-hover);
            color: var(--text-primary);
        }
        
        .modal-body {
            padding: 24px;
            max-height: calc(90vh - 140px);
            overflow-y: auto;
        }
        
        .modal-tabs {
            display: flex;
            gap: 8px;
            margin-bottom: 20px;
            border-bottom: 2px solid var(--border-color);
        }
        
        .modal-tab {
            padding: 10px 20px;
            background: none;
            border: none;
            border-bottom: 3px solid transparent;
            color: var(--text-secondary);
            cursor: pointer;
            font-size: 0.95em;
            font-weight: 500;
            transition: all 0.2s ease;
        }
        
        .modal-tab:hover {
            color: var(--text-primary);
            background: var(--bg-card-hover);
        }
        
        .modal-tab.active {
            color: var(--link-color);
            border-bottom-color: var(--link-color);
        }
        
        .modal-tab-content {
            display: none;
        }
        
        .modal-tab-content.active {
            display: block;
        }
        
        .debug-section {
            margin-bottom: 24px;
        }
        
        .debug-section-title {
            font-size: 1.1em;
            font-weight: 600;
            color: var(--text-primary);
            margin-bottom: 12px;
            padding-bottom: 8px;
            border-bottom: 1px solid var(--border-color);
        }
        
        .debug-row {
            display: flex;
            justify-content: space-between;
            padding: 8px 0;
            border-bottom: 1px solid var(--border-color);
        }
        
        .debug-row:last-child {
            border-bottom: none;
        }
        
        .debug-label {
            font-weight: 500;
            color: var(--text-secondary);
            flex: 0 0 40%;
        }
        
        .debug-value {
            color: var(--text-primary);
            font-family: 'Courier New', monospace;
            flex: 1;
            text-align: right;
            word-break: break-all;
            display: flex;
            align-items: center;
            justify-content: flex-end;
            gap: 8px;
        }
        
        .debug-value-text {
            flex: 1;
            text-align: right;
        }
        
        .copy-btn-small {
            background: var(--bg-card-hover);
            border: 1px solid var(--border-color);
            border-radius: 4px;
            padding: 2px 6px;
            font-size: 0.75em;
            cursor: pointer;
            color: var(--text-secondary);
            transition: all 0.2s ease;
            white-space: nowrap;
            flex-shrink: 0;
        }
        
        .copy-btn-small:hover {
            background: var(--link-color);
            color: white;
            border-color: var(--link-color);
        }
        
        /* Toast Notification Styles */
        .toast-container {
            position: fixed;
            top: 80px;
            right: 20px;
            z-index: 3000;
            display: flex;
            flex-direction: column;
            gap: 10px;
        }
        
        .toast {
            background: var(--bg-card);
            border: 1px solid var(--border-color);
            border-left: 4px solid var(--badge-online);
            border-radius: 6px;
            padding: 12px 16px;
            box-shadow: 0 4px 12px var(--shadow-lg);
            min-width: 300px;
            animation: toastSlideIn 0.3s ease-out;
        }
        
        .toast.error {
            border-left-color: var(--badge-error);
        }
        
        @keyframes toastSlideIn {
            from {
                opacity: 0;
                transform: translateX(100px);
            }
            to {
                opacity: 1;
                transform: translateX(0);
            }
        }
        
        .toast-message {
            color: var(--text-primary);
            font-size: 0.9em;
        }

        /* Full-height page utilities */
        .page-section {
            flex: 1;
            display: flex;
            flex-direction: column;
            min-height: 0; /* allow child scrolling */
        }
        .toolbar {
            display: flex;
            gap: 8px;
            align-items: center;
            flex-wrap: wrap;
            padding: 6px 8px;
            border: 1px solid var(--border-color);
            border-radius: 6px;
            background: var(--bg-secondary);
            margin-bottom: 8px;
        }
        .toolbar .spacer { flex: 1; }
        .table-scroll {
            flex: 1;
            overflow: auto;
            border: 1px solid var(--border-color);
            border-radius: 8px;
            background: var(--bg-card);
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
        // State Manager - Browser Storage
        const AppState = {
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
        
        // Save active tab on navigation
        document.addEventListener('DOMContentLoaded', () => {
            const currentPath = window.location.pathname;
            const tab = currentPath === '/' ? 'home' : currentPath.substring(1);
            AppState.save('lastTab', tab);
        });
        
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
        
    // Initialize (refresh silently every 1s)
    updateStatusBadge();
    setInterval(updateStatusBadge, 1000);
        
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
                const endpoint = type === 'position' ? `/api/v1/positions/${mint}/debug` : `/api/v1/tokens/${mint}/debug`;
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
                `/api/v1/positions/${mint}/debug` : 
                `/api/v1/tokens/${mint}/debug`;
            
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
    "#
}

/// Theme management JavaScript
fn theme_scripts() -> &'static str {
    r#"
        // Theme Management
        (function() {
            const html = document.documentElement;
            const themeToggle = document.getElementById('themeToggle');
            const themeIcon = document.getElementById('themeIcon');
            const themeText = document.getElementById('themeText');
            
            // Load saved theme or default to light
            const savedTheme = localStorage.getItem('theme') || 'light';
            setTheme(savedTheme);
            
            // Theme toggle click handler
            themeToggle.addEventListener('click', () => {
                const currentTheme = html.getAttribute('data-theme');
                const newTheme = currentTheme === 'light' ? 'dark' : 'light';
                setTheme(newTheme);
                localStorage.setItem('theme', newTheme);
            });
            
            function setTheme(theme) {
                html.setAttribute('data-theme', theme);
                if (theme === 'dark') {
                    themeIcon.textContent = '☀️';
                    themeText.textContent = 'Light';
                } else {
                    themeIcon.textContent = '🌙';
                    themeText.textContent = 'Dark';
                }
            }
        })();
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
    <style>
        .pnl-positive { color: #10b981; font-weight: 600; }
        .pnl-negative { color: #ef4444; font-weight: 600; }
        .pnl-neutral { color: #6b7280; }
        .status-badge {
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 0.8em;
            font-weight: 600;
            display: inline-block;
        }
        .status-open { background: #10b98120; color: #10b981; }
        .status-closed { background: #64748b20; color: #64748b; }
        .status-synthetic { background: #f59e0b20; color: #f59e0b; }
    </style>
    <div class="page-section">
        <div class="toolbar">
            <span style="font-weight:600;">💰 Positions</span>
            <select id="statusFilter" onchange="loadPositions()" style="padding: 6px 8px; border: 1px solid var(--border-color); border-radius: 6px; font-size: 0.9em; background: var(--bg-primary); color: var(--text-primary);">
                <option value="all">All</option>
                <option value="open" selected>Open</option>
                <option value="closed">Closed</option>
            </select>
            <input type="text" id="searchInput" placeholder="Search symbol, name, or mint" 
                   style="flex: 1; min-width: 200px; padding: 6px 8px; border: 1px solid var(--border-color); border-radius: 6px; font-size: 0.9em; background: var(--bg-primary); color: var(--text-primary);">
            <div class="spacer"></div>
            <span id="positionCount" style="color: var(--text-secondary); font-size: 0.9em;">Loading...</span>
            <button onclick="loadPositions()" class="btn btn-primary">🔄 Refresh</button>
        </div>
        <div class="table-scroll">
            <table class="table" id="positionsTable">
                <thead>
                    <tr>
                        <th style="min-width: 80px;">Status</th>
                        <th style="min-width: 100px;">Symbol</th>
                        <th style="min-width: 150px;">Name</th>
                        <th style="min-width: 120px;">Entry Price</th>
                        <th style="min-width: 120px;">Current/Exit</th>
                        <th style="min-width: 100px;">Size (SOL)</th>
                        <th style="min-width: 120px;">P&L</th>
                        <th style="min-width: 120px;">P&L %</th>
                        <th style="min-width: 150px;">Entry Time</th>
                        <th style="min-width: 100px;">Actions</th>
                    </tr>
                </thead>
                <tbody id="positionsTableBody">
                    <tr>
                        <td colspan="10" style="text-align: center; padding: 20px; color: #64748b;">
                            Loading positions...
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>

    <script>
    let autoRefreshInterval = null;

        // Format number with decimals
        function formatNumber(num, decimals = 2) {
            if (num === null || num === undefined) return '-';
            return Number(num).toFixed(decimals);
        }

        // Format SOL amount
        function formatSOL(amount) {
            if (amount === null || amount === undefined) return '-';
            return formatNumber(amount, 4) + ' SOL';
        }

        // Format percentage
        function formatPercent(percent) {
            if (percent === null || percent === undefined) return '-';
            const formatted = formatNumber(percent, 2) + '%';
            if (percent > 0) {
                return '<span class="pnl-positive">+' + formatted + '</span>';
            } else if (percent < 0) {
                return '<span class="pnl-negative">' + formatted + '</span>';
            }
            return '<span class="pnl-neutral">' + formatted + '</span>';
        }

        // Format P&L with color
        function formatPnL(pnl) {
            if (pnl === null || pnl === undefined) return '-';
            const formatted = formatSOL(pnl);
            if (pnl > 0) {
                return '<span class="pnl-positive">+' + formatted + '</span>';
            } else if (pnl < 0) {
                return '<span class="pnl-negative">' + formatted + '</span>';
            }
            return '<span class="pnl-neutral">' + formatted + '</span>';
        }

        // Format timestamp
        function formatTime(timestamp) {
            if (!timestamp) return '-';
            const date = new Date(timestamp * 1000);
            return date.toLocaleString('en-US', { 
                month: 'short', 
                day: 'numeric', 
                hour: '2-digit', 
                minute: '2-digit' 
            });
        }

        // Truncate address
        function truncateAddress(address) {
            if (!address) return '-';
            return address.substring(0, 8) + '...' + address.substring(address.length - 6);
        }

        // Stats removed in compact layout

        // Load positions
        async function loadPositions() {
            const statusFilter = document.getElementById('statusFilter').value;
            const searchInput = document.getElementById('searchInput').value.toLowerCase();
            
            try {
                const response = await fetch(`/api/v1/positions?status=${statusFilter}&limit=1000`);
                const positions = await response.json();
                
                // Filter by search input
                const filteredPositions = positions.filter(pos => {
                    if (!searchInput) return true;
                    return pos.symbol.toLowerCase().includes(searchInput) ||
                           pos.name.toLowerCase().includes(searchInput) ||
                           pos.mint.toLowerCase().includes(searchInput);
                });

                const tbody = document.getElementById('positionsTableBody');
                
                if (filteredPositions.length === 0) {
                    tbody.innerHTML = `
                        <tr>
                            <td colspan="10" style="text-align: center; padding: 20px; color: #64748b;">
                                No positions found
                            </td>
                        </tr>
                    `;
                    document.getElementById('positionCount').textContent = '0 positions';
                    return;
                }

                tbody.innerHTML = filteredPositions.map(pos => {
                    const isOpen = !pos.transaction_exit_verified;
                    const statusBadge = pos.synthetic_exit 
                        ? '<span class="status-badge status-synthetic">SYNTHETIC</span>'
                        : (isOpen 
                            ? '<span class="status-badge status-open">OPEN</span>'
                            : '<span class="status-badge status-closed">CLOSED</span>');
                    
                    const currentOrExitPrice = isOpen 
                        ? (pos.current_price ? formatNumber(pos.current_price, 8) : '-')
                        : (pos.effective_exit_price || pos.exit_price ? formatNumber(pos.effective_exit_price || pos.exit_price, 8) : '-');
                    
                    const pnl = isOpen ? pos.unrealized_pnl : pos.pnl;
                    const pnlPercent = isOpen ? pos.unrealized_pnl_percent : pos.pnl_percent;
                    
                    return `
                        <tr>
                            <td>${statusBadge}</td>
                            <td><strong>${pos.symbol}</strong></td>
                            <td style="font-size: 0.85em;">${pos.name}</td>
                            <td>${formatNumber(pos.effective_entry_price || pos.entry_price, 8)}</td>
                            <td>${currentOrExitPrice}</td>
                            <td>${formatSOL(pos.entry_size_sol)}</td>
                            <td>${formatPnL(pnl)}</td>
                            <td>${formatPercent(pnlPercent)}</td>
                            <td style="font-size: 0.85em;">${formatTime(pos.entry_time)}</td>
                            <td>
                                <div class="dropdown-container">
                                    <button class="dropdown-btn" onclick="toggleDropdown(event)" aria-label="Actions">
                                        ⋮
                                    </button>
                                    <div class="dropdown-menu">
                                        <button onclick="copyDebugInfo('${pos.mint}', 'position')" class="dropdown-item">
                                            📋 Copy Debug Info
                                        </button>
                                        <button onclick="copyMint('${pos.mint}')" class="dropdown-item">
                                            📋 Copy Mint
                                        </button>
                                        <button onclick="openGMGN('${pos.mint}')" class="dropdown-item">
                                            🔗 Open GMGN
                                        </button>
                                        <button onclick="openDexScreener('${pos.mint}')" class="dropdown-item">
                                            📊 Open DexScreener
                                        </button>
                                        <button onclick="openSolscan('${pos.mint}')" class="dropdown-item">
                                            🔍 Open Solscan
                                        </button>
                                        <button onclick="showDebugModal('${pos.mint}', 'position')" class="dropdown-item">
                                            🐛 Debug Info
                                        </button>
                                    </div>
                                </div>
                            </td>
                        </tr>
                    `;
                }).join('');

                document.getElementById('positionCount').textContent = `${filteredPositions.length} positions`;
                
            } catch (error) {
                console.error('Failed to load positions:', error);
                const tbody = document.getElementById('positionsTableBody');
                tbody.innerHTML = `
                    <tr>
                        <td colspan="10" style="text-align: center; padding: 20px; color: #ef4444;">
                            ⚠️ Failed to load positions
                        </td>
                    </tr>
                `;
            }
        }

        // Setup auto-refresh
        function setupAutoRefresh() {
            if (autoRefreshInterval) {
                clearInterval(autoRefreshInterval);
            }
            
            autoRefreshInterval = setInterval(() => {
                loadPositions();
            }, 1000);
        }

        // Search on input
        document.addEventListener('DOMContentLoaded', () => {
            const searchInput = document.getElementById('searchInput');
            searchInput.addEventListener('input', () => {
                loadPositions();
            });

            // Initial load and silent refresh
            loadPositions();
            setupAutoRefresh();
        });
    </script>
    "#.to_string()
}

/// Tokens page content
pub fn tokens_content() -> String {
    r#"
    <div class="page-section">
        <div class="toolbar">
            <span style="font-weight:600;">🪙 Tokens</span>
            <input type="text" id="searchInput" placeholder="Search by symbol or mint" 
                   style="flex: 1; min-width: 200px; padding: 6px 8px; border: 1px solid var(--border-color); border-radius: 6px; font-size: 0.9em; background: var(--bg-primary); color: var(--text-primary);">
            <div class="spacer"></div>
            <span id="tokenCount" style="color: var(--text-secondary); font-size: 0.9em;">Loading...</span>
            <button onclick="loadTokens()" class="btn btn-primary">🔄 Refresh</button>
        </div>
        <div class="table-scroll">
            <table class="table" id="tokensTable">
                <thead>
                    <tr>
                        <th style="min-width: 80px;">Symbol</th>
                        <th style="min-width: 160px;">Price (SOL)</th>
                        <th style="min-width: 120px;">Updated</th>
                        <th style="min-width: 100px;">Actions</th>
                    </tr>
                </thead>
                <tbody id="tokensTableBody">
                    <tr>
                        <td colspan="4" style="text-align: center; padding: 20px; color: #94a3b8;">
                            <div class="loading-text">Loading tokens...</div>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>
    
    <script>
        let allTokensData = [];
    let tokensRefreshInterval = null;
        
        async function loadTokens() {
            try {
                const res = await fetch('/api/v1/tokens');
                const data = await res.json();
                
                allTokensData = data.tokens || [];
                
                document.getElementById('tokenCount').textContent = 
                    `${allTokensData.length} tokens with available prices`;
                
                renderTokens(allTokensData);
                
            } catch (error) {
                console.error('Failed to load tokens:', error);
                document.getElementById('tokensTableBody').innerHTML = 
                    '<tr><td colspan="4" style="text-align: center; padding: 20px; color: #ef4444;">Failed to load tokens</td></tr>';
            }
        }
        
        function renderTokens(tokens) {
            const tbody = document.getElementById('tokensTableBody');
            
            if (tokens.length === 0) {
                tbody.innerHTML = 
                    '<tr><td colspan="4" style="text-align: center; padding: 20px; color: #94a3b8;">No tokens with available prices</td></tr>';
                return;
            }
            
            tbody.innerHTML = tokens.map(token => {
                const timeAgo = formatTimeAgo(token.updated_at);
                const priceDisplay = token.price_sol < 0.000001 ? 
                    token.price_sol.toExponential(4) : 
                    token.price_sol.toFixed(9);
                
                return `
                    <tr>
                        <td style="font-weight: 600; color: #667eea;">${escapeHtml(token.symbol)}</td>
                        <td style="font-family: 'Courier New', monospace; font-weight: 600;">${priceDisplay}</td>
                        <td style="font-size: 0.85em; color: #64748b;">${timeAgo}</td>
                        <td>
                            <div class="dropdown-container">
                                <button class="dropdown-btn" onclick="toggleDropdown(event)" aria-label="Actions">
                                    ⋮
                                </button>
                                <div class="dropdown-menu">
                                    <button onclick="copyDebugInfo('${token.mint}', 'token')" class="dropdown-item">
                                        📋 Copy Debug Info
                                    </button>
                                    <button onclick="copyMint('${token.mint}')" class="dropdown-item">
                                        📋 Copy Mint
                                    </button>
                                    <button onclick="openGMGN('${token.mint}')" class="dropdown-item">
                                        🔗 Open GMGN
                                    </button>
                                    <button onclick="openDexScreener('${token.mint}')" class="dropdown-item">
                                        📊 Open DexScreener
                                    </button>
                                    <button onclick="openSolscan('${token.mint}')" class="dropdown-item">
                                        🔍 Open Solscan
                                    </button>
                                    <button onclick="showDebugModal('${token.mint}', 'token')" class="dropdown-item">
                                        🐛 Debug Info
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                `;
            }).join('');
        }
        
        function formatTimeAgo(timestamp) {
            const seconds = Math.floor(Date.now() / 1000) - timestamp;
            
            if (seconds < 60) return `${seconds}s ago`;
            if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
            if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
            return `${Math.floor(seconds / 86400)}d ago`;
        }
        
        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }
        
        function copyToClipboard(text) {
            navigator.clipboard.writeText(text).then(() => {
                // Could show a toast notification here
                console.log('Copied:', text);
            });
        }
        
        // Search functionality
        document.getElementById('searchInput').addEventListener('input', (e) => {
            const searchTerm = e.target.value.toLowerCase();
            
            if (searchTerm === '') {
                renderTokens(allTokensData);
            } else {
                const filtered = allTokensData.filter(token => 
                    token.symbol.toLowerCase().includes(searchTerm) ||
                    token.mint.toLowerCase().includes(searchTerm)
                );
                renderTokens(filtered);
            }
        });
        
        // Silent auto-refresh every second
        function startTokensRefresh() {
            if (tokensRefreshInterval) clearInterval(tokensRefreshInterval);
            tokensRefreshInterval = setInterval(() => {
                loadTokens();
            }, 1000);
        }
        
        loadTokens();
        startTokensRefresh();
    </script>
    "#.to_string()
}

/// Events page content
pub fn events_content() -> String {
    r#"
    <div class="page-section">
        <div class="toolbar">
            <span style="font-weight:600;">📡 Events</span>
            <input type="text" id="eventSearch" placeholder="🔍 Search events..." 
                   style="flex: 1; min-width: 200px; padding: 6px 8px; border: 1px solid var(--border-color); border-radius: 6px; background: var(--bg-primary); color: var(--text-primary);">
            <select id="categoryFilter" 
                    style="padding: 6px 8px; border: 1px solid var(--border-color); border-radius: 6px; background: var(--bg-primary); color: var(--text-primary);">
                <option value="">All Categories</option>
                <option value="swap">Swap</option>
                <option value="transaction">Transaction</option>
                <option value="pool">Pool</option>
                <option value="token">Token</option>
                <option value="position">Position</option>
                <option value="security">Security</option>
                <option value="entry">Entry</option>
                <option value="system">System</option>
            </select>
            <select id="severityFilter" 
                    style="padding: 6px 8px; border: 1px solid var(--border-color); border-radius: 6px; background: var(--bg-primary); color: var(--text-primary);">
                <option value="">All Severity</option>
                <option value="info">Info</option>
                <option value="warn">Warning</option>
                <option value="error">Error</option>
                <option value="debug">Debug</option>
            </select>
            <div class="spacer"></div>
            <span id="eventsCountText" style="color: var(--text-secondary); font-size: 0.9em;">Loading...</span>
            <button id="refreshEvents" class="btn btn-primary">🔄 Refresh</button>
        </div>
        <div class="table-scroll">
            <table class="table" id="eventsTable">
                <thead>
                    <tr>
                        <th>Time</th>
                        <th>Category</th>
                        <th>Subtype</th>
                        <th>Severity</th>
                        <th>Message</th>
                        <th>Reference</th>
                    </tr>
                </thead>
                <tbody id="eventsTableBody">
                    <tr>
                        <td colspan="6" style="text-align: center; padding: 20px; color: var(--text-muted);">
                            Loading events...
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>
    
    <script>
        let allEventsData = [];
        let eventsRefreshInterval = null;
        
        // Load events from API
        async function loadEvents() {
            try {
                const category = document.getElementById('categoryFilter').value;
                const severity = document.getElementById('severityFilter').value;
                
                let url = '/api/v1/events?limit=200';
                if (category) url += `&category=${category}`;
                if (severity) url += `&severity=${severity}`;
                
                const res = await fetch(url);
                const data = await res.json();
                
                allEventsData = data.events;
                renderEvents(allEventsData);
                
                document.getElementById('eventsCountText').textContent = `${data.count} events`;
                
                // Save filter state
                AppState.save('events_category', category);
                AppState.save('events_severity', severity);
                
            } catch (error) {
                console.error('Failed to load events:', error);
                document.getElementById('eventsTableBody').innerHTML = `
                    <tr>
                        <td colspan="6" style="text-align: center; padding: 40px; color: var(--badge-error);">
                            ❌ Failed to load events
                        </td>
                    </tr>
                `;
                document.getElementById('eventsCountText').textContent = 'Error loading events';
            }
        }
        
        // Render events in table
        function renderEvents(events) {
            const tbody = document.getElementById('eventsTableBody');
            
            if (events.length === 0) {
                tbody.innerHTML = `
                    <tr>
                        <td colspan="6" style="text-align: center; padding: 40px; color: var(--text-muted);">
                            📋 No events found
                        </td>
                    </tr>
                `;
                return;
            }
            
            tbody.innerHTML = events.map(event => {
                const time = formatTimeAgo(new Date(event.event_time));
                const severityColor = getSeverityColor(event.severity);
                const shortRef = event.reference_id 
                    ? event.reference_id.substring(0, 8) + '...' 
                    : '-';
                
                return `
                    <tr style="border-bottom: 1px solid var(--border-color);">
                        <td style="padding: 10px; white-space: nowrap;">${time}</td>
                        <td style="padding: 10px;">
                            <span style="background: var(--badge-loading); color: white; padding: 2px 8px; border-radius: 4px; font-size: 0.85em;">
                                ${event.category}
                            </span>
                        </td>
                        <td style="padding: 10px; font-size: 0.9em;">${event.subtype || '-'}</td>
                        <td style="padding: 10px;">
                            <span style="background: ${severityColor}; color: white; padding: 2px 8px; border-radius: 4px; font-size: 0.85em;">
                                ${event.severity}
                            </span>
                        </td>
                        <td style="padding: 10px; max-width: 400px; overflow: hidden; text-overflow: ellipsis;">
                            ${escapeHtml(event.message)}
                        </td>
                        <td style="padding: 10px; font-family: monospace; font-size: 0.85em;">${shortRef}</td>
                    </tr>
                `;
            }).join('');
        }
        
        // Get color for severity
        function getSeverityColor(severity) {
            const colors = {
                'Info': 'var(--badge-online)',
                'Warn': 'var(--badge-loading)',
                'Error': 'var(--badge-error)',
                'Debug': '#6b7280'
            };
            return colors[severity] || '#6b7280';
        }
        
        // Search events
        document.getElementById('eventSearch').addEventListener('input', (e) => {
            const searchTerm = e.target.value.toLowerCase();
            AppState.save('events_search', searchTerm);
            
            const filtered = allEventsData.filter(event => 
                event.category.toLowerCase().includes(searchTerm) ||
                (event.subtype && event.subtype.toLowerCase().includes(searchTerm)) ||
                event.message.toLowerCase().includes(searchTerm) ||
                (event.reference_id && event.reference_id.toLowerCase().includes(searchTerm))
            );
            renderEvents(filtered);
        });
        
        // Filter by category
        document.getElementById('categoryFilter').addEventListener('change', loadEvents);
        
        // Filter by severity
        document.getElementById('severityFilter').addEventListener('change', loadEvents);
        
        // Refresh button
        document.getElementById('refreshEvents').addEventListener('click', loadEvents);
        
        // Time formatting
        function formatTimeAgo(date) {
            const seconds = Math.floor((new Date() - date) / 1000);
            if (seconds < 60) return `${seconds}s ago`;
            const minutes = Math.floor(seconds / 60);
            if (minutes < 60) return `${minutes}m ago`;
            const hours = Math.floor(minutes / 60);
            if (hours < 24) return `${hours}h ago`;
            const days = Math.floor(hours / 24);
            return `${days}d ago`;
        }
        
        // HTML escape
        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }
        
        // Silent refresh every second
        function startEventsRefresh() {
            if (eventsRefreshInterval) clearInterval(eventsRefreshInterval);
            eventsRefreshInterval = setInterval(() => {
                loadEvents();
            }, 1000);
        }
        
        // Restore saved filters
        const savedCategory = AppState.load('events_category', '');
        const savedSeverity = AppState.load('events_severity', '');
        const savedSearch = AppState.load('events_search', '');
        
        if (savedCategory) document.getElementById('categoryFilter').value = savedCategory;
        if (savedSeverity) document.getElementById('severityFilter').value = savedSeverity;
        if (savedSearch) document.getElementById('eventSearch').value = savedSearch;
        
        // Initial load
        loadEvents();
        startEventsRefresh();
    </script>
    "#.to_string()
}
