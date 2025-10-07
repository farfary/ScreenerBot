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
    <script src="https://unpkg.com/lightweight-charts@4.2.1/dist/lightweight-charts.standalone.production.js"></script>
    <style>
        {common_styles}
    </style>
</head>
<body>
    <div class="header">
        <h1>🤖 ScreenerBot Dashboard</h1>
        <div class="header-controls">
            <button class="control-btn trader-btn" id="traderToggle" aria-label="Toggle trader">
                <span id="traderIcon">⏸️</span>
                <span id="traderText">Stop Trader</span>
            </button>
            <button class="control-btn reboot-btn" id="rebootBtn" aria-label="Reboot system">
                <span>🔄</span>
                <span>Reboot</span>
            </button>
            <button class="theme-toggle" id="themeToggle" aria-label="Toggle theme">
                <span id="themeIcon">🌙</span>
                <span id="themeText">Dark</span>
            </button>
            <div class="status-indicator">
                <div style="display: flex; gap: 8px; align-items: center;">
                    <span id="wsBadge" class="badge loading" title="WebSocket Connection">🔌 WS</span>
                    <span id="botBadge" class="badge loading" title="Bot Status">🤖 BOT</span>
                </div>
            </div>
        </div>
    </div>
    
    <nav class="tabs">
        {nav_tabs}
    </nav>
    
    <!-- Sub-tabs container (populated dynamically per page) -->
    <div id="subTabsContainer" class="sub-tabs-container" style="display: none;">
        <!-- Sub-tabs will be injected here by page-specific JavaScript -->
    </div>
    
    <!-- Toolbar container (for action buttons below sub-tabs) -->
    <div id="toolbarContainer" class="toolbar-container" style="display: none;">
        <!-- Toolbar buttons will be injected here by page-specific JavaScript -->
    </div>
    
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
                    <button class="modal-tab" onclick="switchDebugTab('pool-debug')">Pool Debug</button>
                    <button class="modal-tab" onclick="switchDebugTab('token-debug')">Token Debug</button>
                    <button class="modal-tab" onclick="switchDebugTab('security')">Security</button>
                    <button class="modal-tab" onclick="switchDebugTab('position')">Position History</button>
                    <button class="modal-tab" onclick="switchDebugTab('position-debug')">Position Debug</button>
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
                
                <!-- Pool Debug Tab -->
                <div id="tab-pool-debug" class="modal-tab-content">
                    <div class="debug-section">
                        <h3 class="debug-section-title">Price History (Last 100)</h3>
                        <div id="priceHistoryChart" style="height: 200px; overflow-y: auto;">
                            <pre id="priceHistoryData">Loading...</pre>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Price Statistics</h3>
                        <div class="debug-row">
                            <span class="debug-label">Min Price:</span>
                            <span class="debug-value" id="statsMinPrice">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Max Price:</span>
                            <span class="debug-value" id="statsMaxPrice">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Avg Price:</span>
                            <span class="debug-value" id="statsAvgPrice">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Volatility:</span>
                            <span class="debug-value" id="statsVolatility">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Data Points:</span>
                            <span class="debug-value" id="statsDataPoints">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Time Span:</span>
                            <span class="debug-value" id="statsTimeSpan">N/A</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">All Discovered Pools</h3>
                        <div id="allPoolsList">Loading...</div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Cache Statistics</h3>
                        <div class="debug-row">
                            <span class="debug-label">Total Cached:</span>
                            <span class="debug-value" id="cacheTotal">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Fresh Prices:</span>
                            <span class="debug-value" id="cacheFresh">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">History Entries:</span>
                            <span class="debug-value" id="cacheHistory">N/A</span>
                        </div>
                    </div>
                </div>
                
                <!-- Token Debug Tab -->
                <div id="tab-token-debug" class="modal-tab-content">
                    <div class="debug-section">
                        <h3 class="debug-section-title">Blacklist Status</h3>
                        <div class="debug-row">
                            <span class="debug-label">Blacklisted:</span>
                            <span class="debug-value" id="blacklistStatus">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Reason:</span>
                            <span class="debug-value" id="blacklistReason">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Occurrences:</span>
                            <span class="debug-value" id="blacklistCount">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">First Occurrence:</span>
                            <span class="debug-value" id="blacklistFirstOccurrence">N/A</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">OHLCV Availability</h3>
                        <div class="debug-row">
                            <span class="debug-label">1m Data:</span>
                            <span class="debug-value" id="ohlcv1m">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">5m Data:</span>
                            <span class="debug-value" id="ohlcv5m">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">15m Data:</span>
                            <span class="debug-value" id="ohlcv15m">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">1h Data:</span>
                            <span class="debug-value" id="ohlcv1h">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Total Candles:</span>
                            <span class="debug-value" id="ohlcvTotal">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Oldest Timestamp:</span>
                            <span class="debug-value" id="ohlcvOldest">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Newest Timestamp:</span>
                            <span class="debug-value" id="ohlcvNewest">N/A</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Decimals Cache</h3>
                        <div class="debug-row">
                            <span class="debug-label">Decimals:</span>
                            <span class="debug-value" id="decimalsValue">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Cached:</span>
                            <span class="debug-value" id="decimalsCached">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Source:</span>
                            <span class="debug-value" id="decimalsSource">N/A</span>
                        </div>
                    </div>
                </div>
                
                <!-- Position Debug Tab -->
                <div id="tab-position-debug" class="modal-tab-content">
                    <div class="debug-section">
                        <h3 class="debug-section-title">Transaction Details</h3>
                        <div class="debug-row">
                            <span class="debug-label">Entry Signature:</span>
                            <span class="debug-value" id="entrySignature" style="word-break: break-all; font-size: 0.8em;">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Entry Verified:</span>
                            <span class="debug-value" id="entryVerified">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Exit Signature:</span>
                            <span class="debug-value" id="exitSignature" style="word-break: break-all; font-size: 0.8em;">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Exit Verified:</span>
                            <span class="debug-value" id="exitVerified">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Synthetic Exit:</span>
                            <span class="debug-value" id="syntheticExit">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Closed Reason:</span>
                            <span class="debug-value" id="closedReason">N/A</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Fee Details</h3>
                        <div class="debug-row">
                            <span class="debug-label">Entry Fee:</span>
                            <span class="debug-value" id="entryFee">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Exit Fee:</span>
                            <span class="debug-value" id="exitFee">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Total Fees:</span>
                            <span class="debug-value" id="totalFees">N/A</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Profit Targets</h3>
                        <div class="debug-row">
                            <span class="debug-label">Min Target:</span>
                            <span class="debug-value" id="minTarget">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Max Target:</span>
                            <span class="debug-value" id="maxTarget">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Liquidity Tier:</span>
                            <span class="debug-value" id="liquidityTier">N/A</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Price Tracking</h3>
                        <div class="debug-row">
                            <span class="debug-label">Highest:</span>
                            <span class="debug-value" id="priceHighest">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Lowest:</span>
                            <span class="debug-value" id="priceLowest">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Drawdown from High:</span>
                            <span class="debug-value" id="drawdown">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Gain from Low:</span>
                            <span class="debug-value" id="gainFromLow">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Current Price Updated:</span>
                            <span class="debug-value" id="currentPriceUpdated">N/A</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Phantom Details</h3>
                        <div class="debug-row">
                            <span class="debug-label">Phantom Remove:</span>
                            <span class="debug-value" id="phantomRemove">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Confirmations:</span>
                            <span class="debug-value" id="phantomConfirmations">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">First Seen:</span>
                            <span class="debug-value" id="phantomFirstSeen">N/A</span>
                        </div>
                    </div>
                    <div class="debug-section">
                        <h3 class="debug-section-title">Proceeds Metrics</h3>
                        <div class="debug-row">
                            <span class="debug-label">Accepted Quotes:</span>
                            <span class="debug-value" id="acceptedQuotes">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Rejected Quotes:</span>
                            <span class="debug-value" id="rejectedQuotes">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Profit Quotes:</span>
                            <span class="debug-value" id="profitQuotes">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Loss Quotes:</span>
                            <span class="debug-value" id="lossQuotes">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Avg Shortfall:</span>
                            <span class="debug-value" id="avgShortfall">N/A</span>
                        </div>
                        <div class="debug-row">
                            <span class="debug-label">Worst Shortfall:</span>
                            <span class="debug-value" id="worstShortfall">N/A</span>
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
            /* Light mode (default) - Professional Navy/Steel Blue */
            --bg-primary: #f8f9fb;
            --bg-secondary: #ffffff;
            --bg-card: #ffffff;
            --bg-card-hover: #f0f2f6;
            --text-primary: #1f2937;
            --text-secondary: #6b7280;
            --text-muted: #9ca3af;
            --border-color: #d1d5db;
            --header-bg: linear-gradient(135deg, #0f4c81 0%, #1565c0 100%);
            --header-text: #ffffff;
            --link-color: #1565c0;
            --link-hover: #0f4c81;
            --badge-online: #10b981;
            --badge-error: #d32f2f;
            --badge-loading: #f59e0b;
            --shadow-sm: rgba(0,0,0,0.05);
            --shadow-md: rgba(0,0,0,0.1);
            --shadow-lg: rgba(0,0,0,0.2);
            --table-header-bg: #f0f2f6;
            --table-header-text: #374151;
            --service-item-bg: #f0f2f6;
        }
        
        /* Dark mode - Professional Graphite/Cyan */
        [data-theme="dark"] {
            --bg-primary: #111827;
            --bg-secondary: #1f2937;
            --bg-card: #1f2937;
            --bg-card-hover: #273247;
            --text-primary: #e5e7eb;
            --text-secondary: #9ca3af;
            --text-muted: #6b7280;
            --border-color: #334155;
            --header-bg: linear-gradient(135deg, #0d324d 0%, #0e4e6d 100%);
            --header-text: #ffffff;
            --link-color: #38bdf8;
            --link-hover: #0ea5e9;
            --badge-online: #10b981;
            --badge-error: #ef4444;
            --badge-loading: #eab308;
            --shadow-sm: rgba(0,0,0,0.2);
            --shadow-md: rgba(0,0,0,0.3);
            --shadow-lg: rgba(0,0,0,0.5);
            --table-header-bg: #273247;
            --table-header-text: #d1d5db;
            --service-item-bg: #273247;
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
        
        /* Control Buttons */
        .control-btn {
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
            transition: background 0.3s ease, opacity 0.3s ease;
        }
        
        .control-btn:hover:not(:disabled) {
            background: rgba(255,255,255,0.3);
        }
        
        .control-btn:active:not(:disabled) {
            transform: scale(0.95);
        }
        
        .control-btn:disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }
        
        .trader-btn.running {
            background: rgba(16, 185, 129, 0.3);
        }
        
        .trader-btn.stopped {
            background: rgba(239, 68, 68, 0.3);
        }
        
        .reboot-btn:hover:not(:disabled) {
            background: rgba(239, 68, 68, 0.3);
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
        
        /* Sub-tabs navigation (second row below main tabs) */
        .sub-tabs-container {
            background: var(--bg-card);
            border-bottom: 1px solid var(--border-color);
            padding: 0 10px;
            display: flex;
            gap: 4px;
            overflow-x: auto;
        }
        
        .sub-tab {
            padding: 8px 16px;
            background: transparent;
            border: none;
            color: var(--text-secondary);
            font-weight: 500;
            font-size: 0.9em;
            cursor: pointer;
            border-bottom: 2px solid transparent;
            transition: all 0.2s ease;
            white-space: nowrap;
            position: relative;
        }
        
        .sub-tab:hover {
            color: var(--link-color);
            background: var(--bg-card-hover);
        }
        
        .sub-tab.active {
            color: var(--link-color);
            border-bottom-color: var(--link-color);
            font-weight: 600;
        }
        
        /* Toolbar container (third row for action buttons) */
        .toolbar-container {
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--border-color);
            padding: 8px 10px;
            display: flex;
            align-items: center;
            gap: 8px;
            flex-wrap: wrap;
        }
        
        .toolbar-container .spacer {
            flex: 1;
            min-width: 10px;
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

        /* Tokens table: flexible auto-fit columns with min/max constraints */
        #tokensTable {
            table-layout: auto;
            width: 100%;
        }
        #tokensTable th, #tokensTable td {
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }
        /* Dynamic column sizing with sensible constraints */
        #tokensTable th:nth-child(1), #tokensTable td:nth-child(1) { min-width: 140px; max-width: 200px; }
        #tokensTable th:nth-child(2), #tokensTable td:nth-child(2) { min-width: 110px; text-align: right; }
        #tokensTable th:nth-child(3), #tokensTable td:nth-child(3) { min-width: 90px; text-align: right; }
        #tokensTable th:nth-child(4), #tokensTable td:nth-child(4) { min-width: 90px; text-align: right; }
        #tokensTable th:nth-child(5), #tokensTable td:nth-child(5) { min-width: 90px; text-align: right; }
        #tokensTable th:nth-child(6), #tokensTable td:nth-child(6) { min-width: 90px; text-align: right; }
        #tokensTable th:nth-child(7), #tokensTable td:nth-child(7) { min-width: 60px; text-align: right; }
        #tokensTable th:nth-child(8), #tokensTable td:nth-child(8) { min-width: 60px; text-align: right; }
        #tokensTable th:nth-child(9), #tokensTable td:nth-child(9) { min-width: 70px; text-align: right; }
        #tokensTable th:nth-child(10), #tokensTable td:nth-child(10) { min-width: 80px; }
        #tokensTable th:nth-child(11), #tokensTable td:nth-child(11) { min-width: 70px; text-align: right; }
        #tokensTable th:nth-child(12), #tokensTable td:nth-child(12) { min-width: 50px; text-align: center; }
        
        .table th {
            background: var(--table-header-bg);
            padding: 6px;
            text-align: left;
            font-weight: 600;
            color: var(--table-header-text);
            border-bottom: 2px solid var(--border-color);
        }

        /* Sortable headers */
        #tokensTable th.sortable { cursor: pointer; user-select: none; }
        #tokensTable th .sort-label { display: inline-flex; align-items: center; gap: 6px; }
        #tokensTable th .sort-indicator { font-size: 0.9em; opacity: 0.6; }
        
        .table td {
            padding: 10px 8px;
            border-bottom: 1px solid var(--border-color);
            vertical-align: middle;
        }

        /* Numeric cells: compact, tabular digits for perfect alignment */
        #tokensTable td.num, #tokensTable th.num {
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
            font-variant-numeric: tabular-nums;
            letter-spacing: 0;
        }
        
        .table tr:hover {
            background: var(--bg-card-hover);
        }

        /* ===================================================================
           TOKEN DETAILS MODAL - Professional Trading Dashboard
           =================================================================== */
        
        .token-modal-overlay {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0, 0, 0, 0.85);
            backdrop-filter: blur(4px);
            z-index: 9999;
            animation: fadeIn 0.3s ease;
        }

        .token-modal-overlay.active {
            display: flex;
            align-items: stretch;
            justify-content: stretch;
        }

        .token-modal {
            width: 100vw;
            height: 100vh;
            background: #0f1419;
            border-radius: 0;
            display: flex;
            flex-direction: column;
            box-shadow: none;
            animation: slideUp 0.25s cubic-bezier(0.16, 1, 0.3, 1);
            overflow: hidden;
        }

        @keyframes fadeIn {
            from { opacity: 0; }
            to { opacity: 1; }
        }

        @keyframes slideUp {
            from {
                opacity: 0;
                transform: translateY(40px) scale(0.95);
            }
            to {
                opacity: 1;
                transform: translateY(0) scale(1);
            }
        }

        /* Modal Header */
        .token-modal .modal-header {
            background: linear-gradient(135deg, #1a1f2e 0%, #0f1419 100%);
            padding: 6px 16px;
            border-bottom: 1px solid #2d3748;
            display: flex;
            align-items: center;
            justify-content: space-between;
            flex-shrink: 0;
        }

        .token-modal .modal-header-left {
            display: flex;
            align-items: center;
            gap: 14px;
            flex: 1;
        }

        .token-modal .modal-token-logo {
            width: 32px;
            height: 32px;
            border-radius: 7px;
            box-shadow: 0 1px 4px rgba(102, 126, 234, 0.18);
        }

        .token-modal .modal-token-info {
            display: flex;
            flex-direction: column;
            gap: 2px;
        }

        .token-modal .modal-token-symbol {
            font-size: 1em;
            font-weight: 700;
            color: #e2e8f0;
        }

        .token-modal .modal-token-name {
            font-size: 0.8em;
            color: #94a3b8;
        }

        .token-modal .modal-price-info {
            display: flex;
            align-items: center;
            gap: 12px;
            margin-left: auto;
            margin-right: 16px;
        }

        .token-modal .modal-price {
            font-size: 1.1em;
            font-weight: 700;
            color: #e2e8f0;
            font-family: 'Courier New', monospace;
        }

        .token-modal .modal-price-change {
            padding: 3px 8px;
            border-radius: 6px;
            font-weight: 600;
            font-size: 0.8em;
        }

        .token-modal .modal-price-change.positive {
            background: rgba(16, 185, 129, 0.15);
            color: #10b981;
        }

        .token-modal .modal-price-change.negative {
            background: rgba(239, 68, 68, 0.15);
            color: #ef4444;
        }

        .token-modal .modal-close-btn {
            width: 34px;
            height: 34px;
            border-radius: 8px;
            background: #1e293b;
            border: 1px solid #334155;
            color: #94a3b8;
            cursor: pointer;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 1.1em;
            transition: all 0.2s;
        }

        .token-modal .modal-close-btn:hover {
            background: #334155;
            color: #e2e8f0;
            transform: scale(1.05);
        }

        /* Modal Body - Split Layout */
        .token-modal .token-detail-body {
            display: flex;
            flex: 1;
            min-height: 0;
            overflow: hidden;
            padding: 0;
            gap: 0;
            max-height: none;
            height: auto;
        }

        .token-modal .modal-main {
            flex: 1;
            display: flex;
            flex-direction: column;
            overflow: hidden;
            min-width: 0;
            min-height: 0;
        }

        .token-modal .modal-sidebar {
            width: 280px;
            min-width: 280px;
            flex: 0 0 280px;
            background: #1a1f2e;
            border-left: 1px solid #2d3748;
            display: flex;
            flex-direction: column;
            overflow-y: auto;
            align-self: stretch;
            min-height: 0;
        }

        /* Chart Area */
        .token-modal .modal-chart-container {
            flex: 1;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 12px 16px;
            background: #0a0e14;
            position: relative;
            min-height: 0;
            min-width: 0;
            overflow: hidden;
        }

        .token-modal .chart-canvas {
            width: 100%;
            height: 100%;
            max-width: 100%;
            max-height: 100%;
        }

        .token-modal .chart-loading {
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            color: #64748b;
            font-size: 1.1em;
        }

        /* Tabs Navigation */
        .token-modal .modal-tabs {
            display: flex;
            gap: 4px;
            padding: 6px 10px 0 10px;
            background: #0f1419;
            border-bottom: 1px solid #2d3748;
            flex-shrink: 0;
        }

        .token-modal .modal-tab {
            padding: 6px 10px;
            background: transparent;
            border: none;
            color: #94a3b8;
            font-size: 0.85em;
            font-weight: 600;
            cursor: pointer;
            border-radius: 8px 8px 0 0;
            transition: all 0.2s;
            position: relative;
        }

        .token-modal .modal-tab:hover {
            background: #1a1f2e;
            color: #e2e8f0;
        }

        .token-modal .modal-tab.active {
            background: #1a1f2e;
            color: #38bdf8;
        }

        .token-modal .modal-tab.active::after {
            content: '';
            position: absolute;
            bottom: 0;
            left: 0;
            right: 0;
            height: 2px;
            background: #38bdf8;
        }

        /* Tab Content */
        .token-modal .modal-content {
            flex: 1;
            display: flex;
            flex-direction: column;
            padding: 0;
            background: #0f1419;
            min-height: 0;
            overflow: hidden;
            position: relative;
            /* Override global .modal-content constraints */
            width: 100%;
            max-width: none;
            height: auto;
            max-height: none;
        }

        .token-modal .tab-pane {
            display: none;
            flex: 1;
            min-height: 0;
            min-width: 0; /* allow flex sizing to full width */
            overflow-y: auto;
            overflow-x: hidden;
            padding: 12px 14px;
            width: 100%;
        }

        .token-modal .tab-pane.active {
            display: flex;
            flex-direction: column;
            animation: fadeInContent 0.3s ease;
        }
        
        /* CRITICAL: Override generic modal-body height cap */
        .token-modal .token-detail-body {
            max-height: none !important;
            height: auto !important;
        }

        @keyframes fadeInContent {
            from {
                opacity: 0;
                transform: translateY(10px);
            }
            to {
                opacity: 1;
                transform: translateY(0);
            }
        }

        /* Metrics Grid */
        .token-modal .metrics-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
            gap: 10px;
            width: 100%;
        }

        @media (max-width: 1200px) {
            .token-modal .token-detail-body {
                flex-direction: column;
            }

            .token-modal .modal-sidebar {
                width: 100%;
                min-width: 0;
                flex: none;
                border-left: none;
                border-top: 1px solid #2d3748;
                max-height: 300px;
            }
        }

        /* Custom Scrollbar */
        .token-modal .modal-content::-webkit-scrollbar,
        .token-modal .modal-sidebar::-webkit-scrollbar,
        .token-modal .tab-pane::-webkit-scrollbar {
            width: 8px;
        }

        .token-modal .modal-content::-webkit-scrollbar-track,
        .token-modal .modal-sidebar::-webkit-scrollbar-track,
        .token-modal .tab-pane::-webkit-scrollbar-track {
            background: #1a1f2e;
        }

        .token-modal .modal-content::-webkit-scrollbar-thumb,
        .token-modal .modal-sidebar::-webkit-scrollbar-thumb,
        .token-modal .tab-pane::-webkit-scrollbar-thumb {
            background: #334155;
            border-radius: 4px;
        }

        .token-modal .modal-content::-webkit-scrollbar-thumb:hover,
        .token-modal .modal-sidebar::-webkit-scrollbar-thumb:hover,
        .token-modal .tab-pane::-webkit-scrollbar-thumb:hover {
            background: #475569;
        }

        .token-modal .metric-card {
            background: #1a1f2e;
            border: 1px solid #2d3748;
            border-radius: 6px;
            padding: 10px;
        }

        .token-modal .metric-card-header {
            font-size: 0.78em;
            color: #94a3b8;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 10px;
            font-weight: 600;
        }

        .token-modal .metric-row {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 6px 0;
            border-bottom: 1px solid #2d3748;
        }

        .token-modal .metric-row:last-child {
            border-bottom: none;
        }

        .token-modal .metric-label {
            color: #94a3b8;
            font-size: 0.9em;
        }

        .token-modal .metric-value {
            color: #e2e8f0;
            font-weight: 600;
            font-family: 'Courier New', monospace;
        }

        /* Ensure all tab-pane children expand to full width */
        .token-modal .tab-pane > div {
            width: 100%;
            box-sizing: border-box;
        }

        /* Security tab blocks should expand */
        .token-modal .security-badge,
        .token-modal .metric-card,
        .token-modal .risk-list,
        .token-modal .risk-item {
            width: 100%;
            box-sizing: border-box;
        }

        /* Overview grid spans full width */
        .token-modal .metrics-grid {
            width: 100%;
            min-width: 100%;
            box-sizing: border-box;
        }

        /* Chart container full width */
        .token-modal .modal-chart-container {
            width: 100%;
            box-sizing: border-box;
        }

        /* Transactions list full width */
        .token-modal #transactions-list {
            width: 100%;
            box-sizing: border-box;
        }

        /* Sidebar Metrics */
        .token-modal .sidebar-section {
            padding: 14px 12px;
            border-bottom: 1px solid #2d3748;
        }

        .token-modal .sidebar-section:last-child {
            border-bottom: none;
        }

        .token-modal .sidebar-section-title {
            font-size: 0.8em;
            color: #94a3b8;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 12px;
            font-weight: 600;
        }

        .token-modal .sidebar-metric {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 8px 0;
        }

        .token-modal .sidebar-metric-label {
            color: #94a3b8;
            font-size: 0.85em;
        }

        .token-modal .sidebar-metric-value {
            color: #e2e8f0;
            font-weight: 600;
            font-size: 0.9em;
        }

        /* Quick Actions */
        .token-modal .quick-actions {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }

        .token-modal .quick-action-btn {
            padding: 8px 12px;
            background: #1e293b;
            border: 1px solid #334155;
            border-radius: 6px;
            color: #e2e8f0;
            font-size: 0.85em;
            cursor: pointer;
            transition: all 0.2s;
            text-align: left;
        }

        .token-modal .quick-action-btn:hover {
            background: #334155;
            border-color: #38bdf8;
            transform: translateX(4px);
        }

        /* Security Score Badge */
        .token-modal .security-badge {
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 12px 20px;
            border-radius: 8px;
            font-weight: 700;
            font-size: 1.2em;
            margin-bottom: 20px;
        }

        .token-modal .security-badge.excellent {
            background: rgba(16, 185, 129, 0.15);
            color: #10b981;
            border: 2px solid rgba(16, 185, 129, 0.3);
        }

        .token-modal .security-badge.good {
            background: rgba(34, 197, 94, 0.15);
            color: #22c55e;
            border: 2px solid rgba(34, 197, 94, 0.3);
        }

        .token-modal .security-badge.warning {
            background: rgba(245, 158, 11, 0.15);
            color: #f59e0b;
            border: 2px solid rgba(245, 158, 11, 0.3);
        }

        .token-modal .security-badge.danger {
            background: rgba(239, 68, 68, 0.15);
            color: #ef4444;
            border: 2px solid rgba(239, 68, 68, 0.3);
        }

        /* Risk Factors List */
        .token-modal .risk-list {
            display: flex;
            flex-direction: column;
            gap: 12px;
        }

        .token-modal .risk-item {
            display: flex;
            align-items: center;
            gap: 12px;
            padding: 12px 16px;
            background: #1a1f2e;
            border-radius: 8px;
            border-left: 3px solid;
        }

        .token-modal .risk-item.low {
            border-left-color: #10b981;
        }

        .token-modal .risk-item.medium {
            border-left-color: #f59e0b;
        }

        .token-modal .risk-item.high {
            border-left-color: #ef4444;
        }

        .token-modal .risk-icon {
            font-size: 1.2em;
        }

        .token-modal .risk-content {
            flex: 1;
        }

        .token-modal .risk-title {
            color: #e2e8f0;
            font-weight: 600;
            font-size: 0.9em;
        }

        .token-modal .risk-description {
            color: #94a3b8;
            font-size: 0.8em;
            margin-top: 4px;
        }

        .token-modal .risk-score {
            font-weight: 700;
            font-family: 'Courier New', monospace;
        }

        /* Progress Bar */
        .progress-bar {
            height: 8px;
            background: #2d3748;
            border-radius: 4px;
            overflow: hidden;
            margin-top: 8px;
        }

        .progress-fill {
            height: 100%;
            background: linear-gradient(90deg, #1565c0 0%, #0f4c81 100%);
            transition: width 0.6s cubic-bezier(0.16, 1, 0.3, 1);
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

        /* Chart Styles */
        #token-chart {
            width: 100%;
            height: 400px;
            background: var(--bg-card);
            border-radius: 8px;
        }

        #chart-loading {
            display: flex;
            align-items: center;
            justify-content: center;
            height: 400px;
            color: var(--text-secondary);
            font-size: 1.1em;
        }

        #chart-loading::after {
            content: '...';
            animation: dots 1.5s steps(4, end) infinite;
        }

        @keyframes dots {
            0%, 20% { content: '.'; }
            40% { content: '..'; }
            60%, 100% { content: '...'; }
        }

        #chart-error {
            display: none;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 400px;
            gap: 16px;
        }

        #chart-error-message {
            color: var(--badge-error);
            font-size: 1em;
        }

        #chart-error button {
            padding: 8px 16px;
            background: var(--link-color);
            color: white;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.9em;
            transition: background 0.2s ease;
        }

        #chart-error button:hover {
            background: var(--link-hover);
        }

        .chart-controls {
            display: flex;
            gap: 16px;
            margin-bottom: 12px;
            padding: 12px;
            background: var(--table-header-bg);
            border-radius: 6px;
        }

        .timeframe-selector {
            display: flex;
            gap: 4px;
        }

        .timeframe-btn {
            padding: 6px 12px;
            background: var(--bg-card);
            border: 1px solid var(--border-color);
            border-radius: 4px;
            color: var(--text-secondary);
            cursor: pointer;
            font-size: 0.85em;
            transition: all 0.2s ease;
        }

        .timeframe-btn:hover {
            background: var(--bg-card-hover);
            color: var(--text-primary);
        }

        .timeframe-btn.active {
            background: var(--link-color);
            color: white;
            border-color: var(--link-color);
        }

        .indicator-toggles {
            display: flex;
            gap: 12px;
            align-items: center;
        }

        .indicator-toggles label {
            display: flex;
            align-items: center;
            gap: 6px;
            color: var(--text-secondary);
            font-size: 0.85em;
            cursor: pointer;
        }

        .indicator-toggles input[type="checkbox"] {
            cursor: pointer;
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
        .table-scroll.no-x-scroll {
            overflow-x: hidden;
            overflow-y: auto;
        }
    "#
}

/// Navigation tabs
fn nav_tabs(active: &str) -> String {
    let tabs = vec![
        ("home", "🏠 Home"),
        ("status", "📊 Status"),
        ("services", "🔧 Services"),
        ("positions", "💰 Positions"),
        ("tokens", "🪙 Tokens"),
        ("events", "📡 Events"),
        ("config", "⚙️ Config")
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
        
        // WebSocket Hub - Centralized real-time updates (globally accessible)
        window.WsHub = {
            conn: null,
            enabled: true,
            attempts: 0,
            maxAttempts: 5,
            subscriptions: new Set(),
            listeners: {}, // channel -> [callback, callback, ...]
            
            // Initialize connection
            connect() {
                if (this.conn && this.conn.readyState === WebSocket.OPEN) return;
                
                const proto = location.protocol === 'https:' ? 'wss' : 'ws';
                const url = `${proto}://${location.host}/api/ws`;
                
                console.log('[WsHub] Connecting:', url);
                
                try {
                    this.conn = new WebSocket(url);
                    
                    this.conn.onopen = () => {
                        console.log('[WsHub] Connected');
                        this.attempts = 0;
                        
                        // Resubscribe to all channels
                        for (const channel of this.subscriptions) {
                            this.send({ type: 'subscribe', channel });
                        }
                        
                        // Notify listeners
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
                        this.conn = null;
                        this.emit('_disconnected', {});
                        this.reconnect();
                    };
                    
                    this.conn.onerror = (err) => {
                        console.error('[WsHub] Error:', err);
                        try { this.conn && this.conn.close(); } catch {};
                        this.conn = null;
                        this.reconnect();
                    };
                } catch (err) {
                    console.error('[WsHub] Creation failed:', err);
                    this.reconnect();
                }
            },
            
            // Reconnect with exponential backoff
            reconnect() {
                this.attempts++;
                const delay = Math.min(1000 * Math.pow(2, this.attempts), 15000);
                
                console.log(`[WsHub] Reconnect attempt ${this.attempts}, delay: ${delay}ms`);
                
                if (this.attempts > this.maxAttempts) {
                    console.warn('[WsHub] Reconnect failed after max attempts, disabling');
                    this.enabled = false;
                    this.emit('_failed', {});
                    return;
                }
                
                setTimeout(() => this.connect(), delay);
            },
            
            // Handle incoming messages
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
                        // Keep-alive response
                        break;
                    default:
                        console.warn('[WsHub] Unknown message type:', msg.type);
                }
            },
            
            // Subscribe to channel
            subscribe(channel, callback) {
                if (!this.listeners[channel]) {
                    this.listeners[channel] = [];
                }
                this.listeners[channel].push(callback);
                const isInternal = typeof channel === 'string' && channel.startsWith('_');
                if (!isInternal) {
                    this.subscriptions.add(channel);
                }
                
                // Send subscribe message if connected
                if (!isInternal && this.conn && this.conn.readyState === WebSocket.OPEN) {
                    this.send({ type: 'subscribe', channel });
                }
            },
            
            // Unsubscribe from channel
            unsubscribe(channel, callback) {
                if (this.listeners[channel]) {
                    this.listeners[channel] = this.listeners[channel].filter(cb => cb !== callback);
                    if (this.listeners[channel].length === 0) {
                        delete this.listeners[channel];
                        const isInternal = typeof channel === 'string' && channel.startsWith('_');
                        if (!isInternal) {
                            this.subscriptions.delete(channel);
                        }
                        
                        // Send unsubscribe message if connected
                        if (!isInternal && this.conn && this.conn.readyState === WebSocket.OPEN) {
                            this.send({ type: 'unsubscribe', channel });
                        }
                    }
                }
            },
            
            // Emit event to listeners
            emit(channel, data, timestamp) {
                if (this.listeners[channel]) {
                    for (const callback of this.listeners[channel]) {
                        try {
                            callback(data, timestamp);
                        } catch (err) {
                            console.error('[WsHub] Listener error:', err);
                        }
                    }
                }
            },
            
            // Send message to server
            send(msg) {
                if (this.conn && this.conn.readyState === WebSocket.OPEN) {
                    this.conn.send(JSON.stringify(msg));
                } else {
                    console.warn('[WsHub] Not connected, cannot send:', msg);
                }
            },
            
            // Start heartbeat (ping every 30s)
            startHeartbeat() {
                setInterval(() => {
                    if (this.conn && this.conn.readyState === WebSocket.OPEN) {
                        this.send({ type: 'ping' });
                    }
                }, 30000);
            },
            
            // Get connection status
            getStatus() {
                if (!this.conn) return 'disconnected';
                switch (this.conn.readyState) {
                    case WebSocket.CONNECTING: return 'connecting';
                    case WebSocket.OPEN: return 'connected';
                    case WebSocket.CLOSING: return 'closing';
                    case WebSocket.CLOSED: return 'disconnected';
                    default: return 'unknown';
                }
            },
            
            // Check if connected
            isConnected() {
                return this.conn && this.conn.readyState === WebSocket.OPEN;
            }
        };
        
        // Initialize WebSocket Hub on page load
        document.addEventListener('DOMContentLoaded', () => {
            WsHub.connect();
            WsHub.startHeartbeat();
        });
        
        // Save active tab on navigation
        document.addEventListener('DOMContentLoaded', () => {
            const currentPath = window.location.pathname;
            const tab = currentPath === '/' ? 'home' : currentPath.substring(1);
            AppState.save('lastTab', tab);
            
            // Clean up sub-tabs and toolbar for pages that don't use them
            cleanupTabContainers();
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

        if (typeof WsHub !== 'undefined') {
            const handleStatusUpdate = (snapshot) => {
                if (!snapshot) return;
                stopStatusPolling();
                renderStatusBadgesFromSnapshot(snapshot);
            };

            const handleStatusDisconnect = () => {
                setWsBadge('disconnected', '🔌 Offline');
                startStatusPolling();
            };

            const handleStatusReconnect = () => {
                setWsBadge('connected', '🔌 Connected');
                stopStatusPolling();
                fetchStatusSnapshot();
            };
            
            const handleStatusConnecting = () => {
                setWsBadge('connecting', '🔌 Connecting');
            };

            WsHub.subscribe('status', handleStatusUpdate);
            WsHub.subscribe('_disconnected', handleStatusDisconnect);
            WsHub.subscribe('_failed', handleStatusDisconnect);
            WsHub.subscribe('_connected', handleStatusReconnect);
            
            // Set initial WebSocket badge state
            if (WsHub.isConnected()) {
                setWsBadge('connected', '🔌 Connected');
            } else {
                setWsBadge('connecting', '🔌 Connecting');
            }

            window.addEventListener('beforeunload', () => {
                WsHub.unsubscribe('status', handleStatusUpdate);
                WsHub.unsubscribe('_disconnected', handleStatusDisconnect);
                WsHub.unsubscribe('_failed', handleStatusDisconnect);
                WsHub.unsubscribe('_connected', handleStatusReconnect);
            });
        } else {
            // No WebSocket available
            setWsBadge('disconnected', '🔌 N/A');
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
        <!-- Trading Overview Card -->
        <div class="card">
            <div class="card-header">
                <span class="card-icon">�</span>
                <span class="card-title">Trading Overview</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Open Positions</span>
                <span class="metric-value loading-text" id="openPositions">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Total Invested</span>
                <span class="metric-value loading-text" id="totalInvested">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Total P&L</span>
                <span class="metric-value loading-text" id="totalPnl">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Win Rate</span>
                <span class="metric-value loading-text" id="winRate">--</span>
            </div>
        </div>
        
        <!-- Wallet Status Card -->
        <div class="card">
            <div class="card-header">
                <span class="card-icon">�</span>
                <span class="card-title">Wallet Status</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">SOL Balance</span>
                <span class="metric-value loading-text" id="solBalance">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Token Holdings</span>
                <span class="metric-value loading-text" id="tokenCount">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Last Updated</span>
                <span class="metric-value loading-text" id="walletUpdated">--</span>
            </div>
        </div>
        
        <!-- System Health Card -->
        <div class="card">
            <div class="card-header">
                <span class="card-icon">⚙️</span>
                <span class="card-title">System Health</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Services</span>
                <span class="metric-value loading-text" id="servicesStatus">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">RPC Calls/sec</span>
                <span class="metric-value loading-text" id="rpcRate">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">CPU (System)</span>
                <span class="metric-value loading-text" id="homeCpu">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Memory (Sys)</span>
                <span class="metric-value loading-text" id="homeMem">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Uptime</span>
                <span class="metric-value loading-text" id="systemUptime">--</span>
            </div>
        </div>
        
        <!-- Performance Summary Card -->
        <div class="card">
            <div class="card-header">
                <span class="card-icon">📊</span>
                <span class="card-title">All-Time Performance</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Total Positions</span>
                <span class="metric-value loading-text" id="totalPositions">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Closed Positions</span>
                <span class="metric-value loading-text" id="closedPositions">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">All-Time P&L</span>
                <span class="metric-value loading-text" id="allTimePnl">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Overall Win Rate</span>
                <span class="metric-value loading-text" id="overallWinRate">--</span>
            </div>
        </div>
        
        <!-- Monitoring Status Card -->
        <div class="card">
            <div class="card-header">
                <span class="card-icon">�</span>
                <span class="card-title">Monitoring</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Tokens Tracked</span>
                <span class="metric-value loading-text" id="tokensTracked">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Blacklisted</span>
                <span class="metric-value loading-text" id="blacklisted">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Entry Check</span>
                <span class="metric-value loading-text" id="entryInterval">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Position Check</span>
                <span class="metric-value loading-text" id="positionInterval">--</span>
            </div>
        </div>
        
        <!-- Trading Config Card -->
        <div class="card">
            <div class="card-header">
                <span class="card-icon">⚡</span>
                <span class="card-title">Trading Config</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Max Positions</span>
                <span class="metric-value loading-text" id="maxPositions">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Trade Size</span>
                <span class="metric-value loading-text" id="tradeSize">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Stop Loss</span>
                <span class="metric-value loading-text" id="stopLoss">--</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Min Profit</span>
                <span class="metric-value loading-text" id="minProfit">--</span>
            </div>
        </div>
    </div>
    
    <script>
        async function loadHomeData() {
            try {
                const res = await fetch('/api/dashboard/overview');
                const data = await res.json();
                
                // Trading Overview
                const openPos = data.positions.open_positions;
                const maxPos = 2; // Will be fetched from config
                document.getElementById('openPositions').textContent = `${openPos}/${maxPos}`;
                document.getElementById('openPositions').classList.remove('loading-text');
                
                document.getElementById('totalInvested').textContent = `${data.positions.total_invested_sol.toFixed(4)} SOL`;
                document.getElementById('totalInvested').classList.remove('loading-text');
                
                const pnl = data.positions.total_pnl;
                const pnlEl = document.getElementById('totalPnl');
                pnlEl.textContent = `${pnl >= 0 ? '+' : ''}${pnl.toFixed(4)} SOL`;
                pnlEl.style.color = pnl >= 0 ? '#00ff00' : '#ff4444';
                pnlEl.classList.remove('loading-text');
                
                document.getElementById('winRate').textContent = `${data.positions.win_rate.toFixed(1)}%`;
                document.getElementById('winRate').classList.remove('loading-text');
                
                // Wallet Status
                document.getElementById('solBalance').textContent = `${data.wallet.sol_balance.toFixed(4)} SOL`;
                document.getElementById('solBalance').classList.remove('loading-text');
                
                document.getElementById('tokenCount').textContent = `${data.wallet.total_tokens_count} tokens`;
                document.getElementById('tokenCount').classList.remove('loading-text');
                
                const walletTime = data.wallet.last_updated ? new Date(data.wallet.last_updated).toLocaleTimeString() : 'N/A';
                document.getElementById('walletUpdated').textContent = walletTime;
                document.getElementById('walletUpdated').classList.remove('loading-text');
                
                // System Health
                const allReady = data.system.all_services_ready;
                const servicesEl = document.getElementById('servicesStatus');
                servicesEl.textContent = allReady ? '●●●●● All Ready' : '○ Starting...';
                servicesEl.style.color = allReady ? '#00ff00' : '#ffaa00';
                servicesEl.classList.remove('loading-text');
                
                document.getElementById('rpcRate').textContent = `${data.rpc.calls_per_second.toFixed(1)}/sec`;
                document.getElementById('rpcRate').classList.remove('loading-text');
                
                // Fetch detailed system metrics for CPU/memory
                try {
                    const mres = await fetch('/api/status/metrics');
                    const m = await mres.json();
                    const cpuEl = document.getElementById('homeCpu');
                    const memEl = document.getElementById('homeMem');
                    cpuEl.textContent = `${m.cpu_system_percent.toFixed(1)}%`;
                    memEl.textContent = `${m.system_memory_used_mb} / ${m.system_memory_total_mb} MB`;
                    cpuEl.classList.remove('loading-text');
                    memEl.classList.remove('loading-text');
                } catch (e) {
                    // fallback: use simplified fields from overview
                    const cpuEl = document.getElementById('homeCpu');
                    const memEl = document.getElementById('homeMem');
                    cpuEl.textContent = `${data.system.cpu_percent.toFixed(1)}%`;
                    memEl.textContent = `${data.system.memory_mb.toFixed(0)} MB`;
                    cpuEl.classList.remove('loading-text');
                    memEl.classList.remove('loading-text');
                }
                
                document.getElementById('systemUptime').textContent = data.system.uptime_formatted;
                document.getElementById('systemUptime').classList.remove('loading-text');
                
                // Performance Summary
                document.getElementById('totalPositions').textContent = data.positions.total_positions;
                document.getElementById('totalPositions').classList.remove('loading-text');
                
                document.getElementById('closedPositions').textContent = data.positions.closed_positions;
                document.getElementById('closedPositions').classList.remove('loading-text');
                
                const allTimePnl = data.positions.total_pnl;
                const allTimePnlEl = document.getElementById('allTimePnl');
                allTimePnlEl.textContent = `${allTimePnl >= 0 ? '+' : ''}${allTimePnl.toFixed(4)} SOL`;
                allTimePnlEl.style.color = allTimePnl >= 0 ? '#00ff00' : '#ff4444';
                allTimePnlEl.classList.remove('loading-text');
                
                document.getElementById('overallWinRate').textContent = `${data.positions.win_rate.toFixed(1)}%`;
                document.getElementById('overallWinRate').classList.remove('loading-text');
                
                // Monitoring
                document.getElementById('tokensTracked').textContent = data.monitoring.tokens_tracked;
                document.getElementById('tokensTracked').classList.remove('loading-text');
                
                document.getElementById('blacklisted').textContent = data.blacklist.total_blacklisted;
                document.getElementById('blacklisted').classList.remove('loading-text');
                
                document.getElementById('entryInterval').textContent = `Every ${data.monitoring.entry_check_interval_secs}s`;
                document.getElementById('entryInterval').classList.remove('loading-text');
                
                document.getElementById('positionInterval').textContent = `Every ${data.monitoring.position_monitor_interval_secs}s`;
                document.getElementById('positionInterval').classList.remove('loading-text');
                
                // Load trading config
                const configRes = await fetch('/api/trading/config');
                const config = await configRes.json();
                
                document.getElementById('maxPositions').textContent = config.trading_limits.max_open_positions;
                document.getElementById('maxPositions').classList.remove('loading-text');
                
                document.getElementById('tradeSize').textContent = `${config.trading_limits.trade_size_sol} SOL`;
                document.getElementById('tradeSize').classList.remove('loading-text');
                
                document.getElementById('stopLoss').textContent = `${config.risk_management.stop_loss_percent}%`;
                document.getElementById('stopLoss').classList.remove('loading-text');
                
                document.getElementById('minProfit').textContent = `${config.profit_targets.base_min_profit_percent}%`;
                document.getElementById('minProfit').classList.remove('loading-text');
                
            } catch (error) {
                console.error('Failed to load home data:', error);
            }
        }
        
        loadHomeData();
        setInterval(loadHomeData, 1000);
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
            <div class="metric-row">
                <span class="metric-label">Process Memory</span>
                <span class="metric-value loading-text" id="procMem">Loading...</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Process CPU</span>
                <span class="metric-value loading-text" id="procCpu">Loading...</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">System Memory</span>
                <span class="metric-value loading-text" id="sysMem">Loading...</span>
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
        function escapeHtml(str) {
            if (typeof str !== 'string') return '';
            return str
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;')
                .replace(/'/g, '&#39;');
        }

        async function loadStatusData() {
            try {
                const [statusRes, metricsRes] = await Promise.all([
                    fetch('/api/status'),
                    fetch('/api/status/metrics')
                ]);

                if (!statusRes.ok) {
                    throw new Error(`Status request failed (${statusRes.status})`);
                }

                if (!metricsRes.ok) {
                    throw new Error(`Metrics request failed (${metricsRes.status})`);
                }

                const status = await statusRes.json();
                const metrics = await metricsRes.json();
                
                // Update metrics
                document.getElementById('memory').textContent = metrics.system_memory_used_mb + ' MB';
                document.getElementById('cpu').textContent = metrics.cpu_system_percent.toFixed(1) + '%';
                document.getElementById('threads').textContent = metrics.active_threads;
                document.getElementById('rpcCalls').textContent = formatNumber(metrics.rpc_calls_total);
                document.getElementById('successRate').textContent = metrics.rpc_success_rate.toFixed(1) + '%';
                document.getElementById('wsConns').textContent = metrics.ws_connections;

                // Detailed metrics
                document.getElementById('procMem').textContent = metrics.process_memory_mb + ' MB';
                document.getElementById('procCpu').textContent = metrics.cpu_process_percent.toFixed(1) + '%';
                document.getElementById('sysMem').textContent = metrics.system_memory_used_mb + ' / ' + metrics.system_memory_total_mb + ' MB';
                
                // Remove loading class
                document.querySelectorAll('.loading-text').forEach(el => el.classList.remove('loading-text'));
                
                // Update services
                const servicesList = document.getElementById('servicesList');
                const servicesData = (status && status.services) || {};
                const serviceEntries = [
                    { key: 'tokens_system', label: 'Tokens System' },
                    { key: 'positions_system', label: 'Positions Manager' },
                    { key: 'pool_service', label: 'Pool Service' },
                    { key: 'transactions_system', label: 'Transactions' },
                    { key: 'security_analyzer', label: 'Security Analyzer' }
                ];

                if (servicesList) {
                    const rendered = serviceEntries
                        .map(({ key, label }) => {
                            const svc = servicesData[key] || {};
                            const ready = svc.ready === true;
                            const dotClass = ready ? 'ready' : 'not-ready';
                            const tooltip = svc.error ? ` title="${escapeHtml(svc.error)}"` : '';
                            return `
                                <div class="service-item"${tooltip}>
                                    <span>${label}</span>
                                    <span class="status-dot ${dotClass}"></span>
                                </div>
                            `;
                        })
                        .join('');

                    servicesList.innerHTML = rendered || '<div class="loading-text">No service data available</div>';
                }
                
            } catch (error) {
                console.error('Failed to load status data:', error);
                const servicesList = document.getElementById('servicesList');
                if (servicesList) {
                    servicesList.innerHTML = '<div class="loading-text">Failed to load services</div>';
                }
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
            <select id="statusFilter" onchange="loadPositions({ reason: 'filter-change', force: true })" style="padding: 6px 8px; border: 1px solid var(--border-color); border-radius: 6px; font-size: 0.9em; background: var(--bg-primary); color: var(--text-primary);">
                <option value="all">All</option>
                <option value="open" selected>Open</option>
                <option value="closed">Closed</option>
            </select>
            <input type="text" id="searchInput" placeholder="Search symbol, name, or mint" 
                   style="flex: 1; min-width: 200px; padding: 6px 8px; border: 1px solid var(--border-color); border-radius: 6px; font-size: 0.9em; background: var(--bg-primary); color: var(--text-primary);">
            <div class="spacer"></div>
            <span id="positionCount" style="color: var(--text-secondary); font-size: 0.9em;">Loading...</span>
            <button onclick="loadPositions({ reason: 'manual-refresh', force: true })" class="btn btn-primary">🔄 Refresh</button>
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
    const positionsStore = new Map();
    let positionsLoading = false;
    let positionsInitialized = false;
    let positionsFallbackInterval = null;
    let positionsRequestController = null;

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
            if (!timestamp && timestamp !== 0) return '-';
            const date = new Date(Number(timestamp) * 1000);
            if (Number.isNaN(date.getTime())) return '-';
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

        function getCurrentStatusFilter() {
            const select = document.getElementById('statusFilter');
            return select ? select.value : 'all';
        }

        function getSearchTerm() {
            const input = document.getElementById('searchInput');
            return input ? input.value.toLowerCase().trim() : '';
        }

        function isDropdownOpen() {
            return Boolean(document.querySelector('.dropdown-menu.show'));
        }

        function parsePossibleTimestamp(value) {
            if (value === null || value === undefined) return null;
            if (typeof value === 'number' && Number.isFinite(value)) {
                return value;
            }
            if (typeof value === 'string' && value.trim() !== '') {
                const numeric = Number(value);
                if (Number.isFinite(numeric)) {
                    return numeric;
                }

                const parsed = Date.parse(value);
                if (Number.isFinite(parsed)) {
                    return Math.floor(parsed / 1000);
                }
            }
            return null;
        }

        function normalizePositionFromApi(raw) {
            if (!raw || typeof raw !== 'object') return raw;
            const position = { ...raw };

            position.entry_time = parsePossibleTimestamp(position.entry_time);
            position.exit_time = parsePossibleTimestamp(position.exit_time);
            position.current_price_updated = parsePossibleTimestamp(position.current_price_updated);

            return position;
        }

        function getPositionKey(position) {
            if (!position) return null;

            if (position.id !== undefined && position.id !== null) {
                return `id:${position.id}`;
            }

            if (position.mint) {
                const entry = position.entry_time ?? position.created_at ?? '';
                return `mint:${position.mint}:${entry}`;
            }

            return null;
        }

        function matchesStatusFilter(position, filterValue) {
            if (filterValue === 'all') return true;
            const isOpen = !position.transaction_exit_verified;
            if (filterValue === 'open') {
                return isOpen;
            }
            if (filterValue === 'closed') {
                return !isOpen;
            }
            return true;
        }

        function matchesSearchFilter(position, searchTerm) {
            if (!searchTerm) return true;
            const fields = [position.symbol, position.name, position.mint];
            return fields.some(field => field && field.toLowerCase().includes(searchTerm));
        }

        function shouldIncludeInCurrentFilter(position) {
            return matchesStatusFilter(position, getCurrentStatusFilter());
        }

        function renderPositionRow(pos) {
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
                <tr data-position-key="${getPositionKey(pos) || ''}">
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
        }

        function updatePositionCount(count) {
            const label = document.getElementById('positionCount');
            if (!label) return;
            const value = Number.isFinite(count) ? count : positionsStore.size;
            label.textContent = `${value} ${value === 1 ? 'position' : 'positions'}`;
        }

        function renderEmptyState(message, color = '#64748b') {
            const tbody = document.getElementById('positionsTableBody');
            if (!tbody) return;
            tbody.innerHTML = `
                <tr>
                    <td colspan="10" style="text-align: center; padding: 20px; color: ${color};">
                        ${message}
                    </td>
                </tr>
            `;
        }

        function startPositionsFallback(intervalMs = 3000) {
            if (positionsFallbackInterval) return;
            positionsFallbackInterval = setInterval(() => {
                loadPositions({ reason: 'fallback' });
            }, intervalMs);
        }

        function stopPositionsFallback() {
            if (!positionsFallbackInterval) return;
            clearInterval(positionsFallbackInterval);
            positionsFallbackInterval = null;
        }

        function renderPositionsTable() {
            const tbody = document.getElementById('positionsTableBody');
            if (!tbody) return;

            if (!positionsInitialized && positionsStore.size === 0) {
                renderEmptyState('Loading positions...');
                return;
            }

            const statusFilter = getCurrentStatusFilter();
            const searchTerm = getSearchTerm();

            const rows = Array.from(positionsStore.values())
                .filter(pos => matchesStatusFilter(pos, statusFilter))
                .filter(pos => matchesSearchFilter(pos, searchTerm))
                .sort((a, b) => {
                    const aTime = a.entry_time ?? 0;
                    const bTime = b.entry_time ?? 0;
                    return bTime - aTime;
                });

            if (rows.length === 0) {
                renderEmptyState('No positions found');
                updatePositionCount(0);
                return;
            }

            tbody.innerHTML = rows.map(renderPositionRow).join('');
            updatePositionCount(rows.length);
        }

        function syncPositionsStore(positions) {
            positionsStore.clear();
            positions.forEach(pos => {
                const normalized = normalizePositionFromApi(pos);
                const key = getPositionKey(normalized);
                if (key) {
                    positionsStore.set(key, normalized);
                }
            });
        }

        function handlePositionBroadcast(update) {
            if (!update || typeof update !== 'object') return;

            // Receiving live updates means WebSocket is active
            stopPositionsFallback();

            if (update.type === 'balance_changed') {
                return;
            }

            if (!update.position) {
                console.warn('[Positions] Update missing position payload:', update);
                return;
            }

            const normalized = normalizePositionFromApi(update.position);
            const key = getPositionKey(normalized);
            if (!key) {
                console.warn('[Positions] Unable to derive key for update:', update);
                return;
            }

            if (shouldIncludeInCurrentFilter(normalized)) {
                positionsStore.set(key, normalized);
            } else {
                positionsStore.delete(key);
            }

            positionsInitialized = true;
            renderPositionsTable();
        }

        function handlePositionsWarning(message) {
            if (!message || message.channel !== 'positions') return;
            console.warn('[Positions] WebSocket warning received:', message);
            startPositionsFallback();
            loadPositions({ reason: 'ws-warning', force: true });
        }

        function handlePositionsDisconnect() {
            startPositionsFallback();
        }

        function handlePositionsReconnect() {
            stopPositionsFallback();
            loadPositions({ reason: 'ws-reconnect', force: true });
        }

        async function loadPositions(options = {}) {
            const { reason = 'manual', force = false } = options;

            if (!force && isDropdownOpen()) {
                return;
            }

            if (positionsLoading && !force) {
                return;
            }

            const statusFilter = getCurrentStatusFilter();

            if (positionsStore.size === 0 && !positionsInitialized) {
                renderEmptyState('Loading positions...');
            }

            positionsLoading = true;

            if (positionsRequestController) {
                positionsRequestController.abort();
            }

            positionsRequestController = new AbortController();

            try {
                const response = await fetch(`/api/positions?status=${statusFilter}&limit=1000`, {
                    signal: positionsRequestController.signal
                });
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }

                const positions = await response.json();
                const array = Array.isArray(positions) ? positions : [];

                syncPositionsStore(array);
                positionsInitialized = true;
                renderPositionsTable();
            } catch (error) {
                if (error.name === 'AbortError') {
                    return;
                }
                console.error('Failed to load positions:', error);
                if (!positionsInitialized || positionsStore.size === 0) {
                    renderEmptyState('⚠️ Failed to load positions', '#ef4444');
                    updatePositionCount(0);
                }
            } finally {
                positionsRequestController = null;
                positionsLoading = false;
            }
        }

        document.addEventListener('DOMContentLoaded', () => {
            const searchInput = document.getElementById('searchInput');
            if (searchInput) {
                searchInput.addEventListener('input', () => {
                    renderPositionsTable();
                });
            }

            if (typeof WsHub !== 'undefined') {
                const wsDataHandler = (payload) => handlePositionBroadcast(payload);
                const wsWarningHandler = (message) => handlePositionsWarning(message);
                const wsDisconnectHandler = () => handlePositionsDisconnect();
                const wsReconnectHandler = () => handlePositionsReconnect();

                WsHub.subscribe('positions', wsDataHandler);
                WsHub.subscribe('_warning', wsWarningHandler);
                WsHub.subscribe('_disconnected', wsDisconnectHandler);
                WsHub.subscribe('_failed', wsDisconnectHandler);
                WsHub.subscribe('_connected', wsReconnectHandler);

                if (!WsHub.isConnected()) {
                    startPositionsFallback();
                }

                window.addEventListener('beforeunload', () => {
                    WsHub.unsubscribe('positions', wsDataHandler);
                    WsHub.unsubscribe('_warning', wsWarningHandler);
                    WsHub.unsubscribe('_disconnected', wsDisconnectHandler);
                    WsHub.unsubscribe('_failed', wsDisconnectHandler);
                    WsHub.unsubscribe('_connected', wsReconnectHandler);
                });
            } else {
                console.warn('[Positions] WsHub not available, enabling HTTP polling fallback.');
                startPositionsFallback();
            }

            loadPositions({ reason: 'initial', force: true });
        });
    </script>
    "#.to_string()
}

/// Tokens page content
pub fn tokens_content() -> String {
    r#"
    <script>
        const TOKENS_VIEWS = ['pool', 'all', 'passed', 'rejected', 'blacklisted', 'positions', 'secure', 'recent'];
        const VIEW_LABELS = {
            pool: 'with available prices',
            all: 'in database',
            passed: 'that passed filtering',
            rejected: 'rejected by filtering',
            blacklisted: 'blacklisted',
            positions: 'with open positions',
            secure: 'secure',
            recent: 'recently updated',
        };

        const tokensState = {
            view: 'pool',
            sortBy: 'symbol',
            sortDir: 'asc',
            searchTerm: '',
        };

    let allTokensData = [];
        let tokensRefreshInterval = null;
        let tokensRequestController = null;
        let tokensLoading = false;
        let searchDebounceHandle = null;

        document.addEventListener('DOMContentLoaded', () => {
            hydrateTokensState();
            initTokensSubTabs();
            initTokensToolbar();
            setupSortableHeaders();
            attachGlobalListeners();
            loadTokens({ reason: 'initial', force: true });
            startTokensRefresh();
        });

        function hydrateTokensState() {
            const savedView = window.sessionStorage.getItem('tokens.view');
            if (savedView && TOKENS_VIEWS.includes(savedView)) {
                tokensState.view = savedView;
            }

            const savedSearch = window.sessionStorage.getItem('tokens.search');
            if (typeof savedSearch === 'string') {
                tokensState.searchTerm = savedSearch;
            }
        }

        function attachGlobalListeners() {
            document.addEventListener('visibilitychange', handleVisibilityChange);
        }

        function handleVisibilityChange() {
            if (document.visibilityState === 'hidden') {
                stopTokensRefresh();
            } else {
                startTokensRefresh();
                loadTokens({ reason: 'visibility', force: true });
            }
        }

        function initTokensSubTabs() {
            const subTabsContainer = document.getElementById('subTabsContainer');
            if (!subTabsContainer) return;

            const views = [
                { id: 'pool', label: '💧 Pool Service' },
                { id: 'all', label: '📋 All Tokens' },
                { id: 'passed', label: '✅ Passed' },
                { id: 'rejected', label: '⛔ Rejected' },
                { id: 'blacklisted', label: '🚫 Blacklisted' },
            ];

            subTabsContainer.innerHTML = views
                .map(view => `
                    <button class="sub-tab ${view.id === tokensState.view ? 'active' : ''}" data-view="${view.id}">
                        ${view.label}
                    </button>
                `)
                .join('');

            subTabsContainer
                .querySelectorAll('.sub-tab')
                .forEach(button => {
                    button.addEventListener('click', () => switchTokensSubTab(button.dataset.view));
                });

            subTabsContainer.style.display = 'flex';
        }

        function switchTokensSubTab(view) {
            if (!view || tokensState.view === view) return;

            tokensState.view = view;
            window.sessionStorage.setItem('tokens.view', view);

            document
                .querySelectorAll('#subTabsContainer .sub-tab')
                .forEach(tab => {
                    tab.classList.toggle('active', tab.dataset.view === view);
                });

            loadTokens({ reason: 'view-change', force: true });
        }

        function initTokensToolbar() {
            const toolbarContainer = document.getElementById('toolbarContainer');
            if (!toolbarContainer) return;

            toolbarContainer.innerHTML = `
                <input
                    type="text"
                    id="searchInput"
                    placeholder="Search by symbol or mint..."
                    style="flex: 1; max-width: 300px; padding: 6px 10px; border: 1px solid var(--border-color); border-radius: 6px; font-size: 0.9em; background: var(--bg-primary); color: var(--text-primary);"
                >
                <div class="spacer"></div>
                <span id="tokenCount" style="color: var(--text-secondary); font-size: 0.9em; font-weight: 500;">Loading...</span>
                <button id="tokensRefreshBtn" class="btn btn-primary" style="padding: 6px 12px;">
                    🔄 Refresh
                </button>
                <button id="tokensExportBtn" class="btn btn-secondary" style="padding: 6px 12px;">
                    📥 Export
                </button>
            `;
            toolbarContainer.style.display = 'flex';

            const searchInput = toolbarContainer.querySelector('#searchInput');
            if (searchInput) {
                searchInput.value = tokensState.searchTerm;
                searchInput.addEventListener('input', handleSearchInput);
            }

            const refreshBtn = toolbarContainer.querySelector('#tokensRefreshBtn');
            if (refreshBtn) {
                refreshBtn.addEventListener('click', () => loadTokens({ reason: 'manual-refresh', force: true }));
            }

            const exportBtn = toolbarContainer.querySelector('#tokensExportBtn');
            if (exportBtn) {
                exportBtn.addEventListener('click', exportTokens);
            }
        }

        function handleSearchInput(event) {
            tokensState.searchTerm = event.target.value.trim();
            window.sessionStorage.setItem('tokens.search', tokensState.searchTerm);

            applySearchFilter();

            if (searchDebounceHandle) {
                clearTimeout(searchDebounceHandle);
            }
            searchDebounceHandle = setTimeout(() => {
                loadTokens({ reason: 'search', force: true });
            }, 350);
        }

        function exportTokens() {
            alert('Export functionality - to be implemented');
        }

        function normalizeTokenFromApi(rawToken) {
            if (!rawToken || typeof rawToken !== 'object') return rawToken;
            const token = { ...rawToken };

            if (token.price_updated_at) {
                const numericTimestamp = Number(token.price_updated_at);
                if (Number.isFinite(numericTimestamp)) {
                    token.price_updated_at = numericTimestamp;
                } else {
                    const parsed = Date.parse(token.price_updated_at);
                    token.price_updated_at = Number.isFinite(parsed) ? Math.floor(parsed / 1000) : null;
                }
            } else {
                token.price_updated_at = null;
            }

            return token;
        }

        function dedupeTokensByMint(tokens) {
            if (!Array.isArray(tokens) || tokens.length === 0) {
                return [];
            }

            const seen = new Set();
            const deduped = [];

            for (const token of tokens) {
                if (!token || typeof token !== 'object') continue;
                const mint = token.mint;
                if (!mint || seen.has(mint)) continue;
                seen.add(mint);
                deduped.push(token);
            }

            return deduped;
        }

        async function loadTokens(options = {}) {
            const { reason = 'manual', force = false } = options;
            const isAutoRefresh = reason === 'interval';
            const searchInput = document.getElementById('searchInput');

            if (isAutoRefresh && !force) {
                if (document.visibilityState === 'hidden') return;
                if (document.querySelector('.dropdown-menu.show')) return;
                if (searchInput && document.activeElement === searchInput) return;
            }

            if (tokensLoading && !force) {
                return;
            }

            if (tokensRequestController) {
                tokensRequestController.abort();
            }

            tokensRequestController = new AbortController();
            tokensLoading = true;

            const params = new URLSearchParams({
                view: tokensState.view,
                search: tokensState.searchTerm,
                sort_by: tokensState.sortBy,
                sort_dir: tokensState.sortDir,
                page: '1',
                page_size: '1000',
            });

            try {
                const res = await fetch(`/api/tokens/list?${params.toString()}`, {
                    signal: tokensRequestController.signal,
                });
                if (!res.ok) {
                    throw new Error(`HTTP ${res.status}`);
                }

                const data = await res.json();
                const items = Array.isArray(data.items) ? data.items : [];
                const normalized = items.map(normalizeTokenFromApi);
                allTokensData = dedupeTokensByMint(normalized);
                updateTokenCount(data.total ?? allTokensData.length);
                applySearchFilter();
            } catch (error) {
                if (error.name === 'AbortError') {
                    return;
                }
                console.error('Failed to load tokens:', error);
                showTokensError('Failed to load tokens');
            } finally {
                tokensLoading = false;
            }
        }

        function showTokensError(message) {
            const tbody = document.getElementById('tokensTableBody');
            if (!tbody) return;
            tbody.innerHTML = `<tr><td colspan="12" style="text-align: center; padding: 20px; color: #ef4444;">${message}</td></tr>`;
        }

        function updateTokenCount(total) {
            const label = document.getElementById('tokenCount');
            if (!label) return;

            const parsedTotal = Number(total);
            const totalValue = Number.isFinite(parsedTotal) ? parsedTotal : allTokensData.length;
            const viewLabel = VIEW_LABELS[tokensState.view] || '';
            label.textContent = viewLabel ? `${totalValue} tokens ${viewLabel}` : `${totalValue} tokens`;
        }

        function applySearchFilter() {
            const search = tokensState.searchTerm.toLowerCase();
            if (!search) {
                renderTokens(allTokensData);
                return;
            }

            const filtered = allTokensData.filter(token => {
                const symbol = (token.symbol || '').toString().toLowerCase();
                const mint = (token.mint || '').toString().toLowerCase();
                const name = (token.name || '').toString().toLowerCase();
                return symbol.includes(search) || mint.includes(search) || name.includes(search);
            });

            renderTokens(filtered);
        }

        function renderTokens(tokens) {
            const tbody = document.getElementById('tokensTableBody');
            if (!tbody) return;

            const openState = captureOpenDropdownState();

            if (!tokens || tokens.length === 0) {
                tbody.innerHTML = `<tr><td colspan="12" style="text-align: center; padding: 20px; color: #94a3b8;">No tokens found for view: ${tokensState.view}</td></tr>`;
                return;
            }

            tbody.innerHTML = tokens.map(createTokenRowHtml).join('');
            restoreDropdownState(openState);
        }

        function captureOpenDropdownState() {
            const menu = document.querySelector('.dropdown-menu.show');
            if (!menu) return null;
            const row = menu.closest('tr[data-mint]');
            if (!row) return null;
            return { mint: row.dataset.mint };
        }

        function restoreDropdownState(state) {
            if (!state || !state.mint) return;
            const container = document.querySelector(`.dropdown-container[data-mint="${state.mint}"]`);
            if (!container) return;

            const btn = container.querySelector('.dropdown-btn');
            const menu = container.querySelector('.dropdown-menu');
            if (!btn || !menu) return;

            openDropdownMenu(btn, menu);
        }

        function createTokenRowHtml(token) {
            const mint = token.mint;
            const symbol = escapeHtml(token.symbol || '');
            const name = token.name ? escapeHtml(token.name) : '';
            const timeAgo = token.price_updated_at ? formatTimeAgo(token.price_updated_at) : 'N/A';
            const priceDisplay = formatPriceSol(token.price_sol);
            const liquidity = formatCurrencyUSD(token.liquidity_usd);
            const vol24h = formatCurrencyUSD(token.volume_24h);
            const fdv = formatCurrencyUSD(token.fdv);
            const marketCap = formatCurrencyUSD(token.market_cap);
            const ch1h = formatPercent(token.price_change_h1);
            const ch24h = formatPercent(token.price_change_h24);
            const security = renderSecurity(token.security_score, token.rugged);
            const status = renderStatusBadges(token);
            const logo = safeLogoHtml(token.logo_url, token.symbol);

            return `
                <tr data-mint="${mint}" onclick="openTokenDetail('${mint}')" style="cursor:pointer;" title="Click to view details">
                    <td>
                        <div style="display:flex; align-items:center; gap:12px; min-width:0;">
                            ${logo}
                            <div style="display:flex; flex-direction:column; gap:1px; min-width:0;">
                                <div style="font-weight:600; color:var(--link-color); font-size:0.95em; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width: 260px;">${symbol}</div>
                                <div style="font-size:0.8em; color:#94a3b8; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width: 260px;">${name}</div>
                            </div>
                        </div>
                    </td>
                    <td class="num" style="font-weight:600;" title="${token.price_sol ?? ''}">${priceDisplay}</td>
                    <td class="num" title="${token.liquidity_usd ?? ''}">${liquidity}</td>
                    <td class="num" title="${token.volume_24h ?? ''}">${vol24h}</td>
                    <td class="num" title="${token.fdv ?? ''}">${fdv}</td>
                    <td class="num" title="${token.market_cap ?? ''}">${marketCap}</td>
                    <td class="num" title="${token.price_change_h1 ?? ''}">${ch1h}</td>
                    <td class="num" title="${token.price_change_h24 ?? ''}">${ch24h}</td>
                    <td class="num" title="${token.security_score ?? ''}">${security}</td>
                    <td>${status}</td>
                    <td style="font-size: 0.85em; color: #64748b;">${timeAgo}</td>
                    <td onclick="event.stopPropagation();">
                        <div class="dropdown-container" data-mint="${mint}">
                            <button class="dropdown-btn" type="button" onclick="toggleDropdown(event)" aria-label="Actions">⋮</button>
                            <div class="dropdown-menu">
                                <button onclick="copyMint('${mint}')" class="dropdown-item">📋 Copy Mint</button>
                                <button onclick="openGMGN('${mint}')" class="dropdown-item">🔗 Open GMGN</button>
                                <button onclick="openDexScreener('${mint}')" class="dropdown-item">📊 Open DexScreener</button>
                                <button onclick="openSolscan('${mint}')" class="dropdown-item">🔍 Open Solscan</button>
                                <button onclick="openTokenDetail('${mint}')" class="dropdown-item">🔎 View Details</button>
                            </div>
                        </div>
                    </td>
                </tr>
            `;
        }

        function openDropdownMenu(btn, menu) {
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

        function startTokensRefresh() {
            if (tokensRefreshInterval) clearInterval(tokensRefreshInterval);
            tokensRefreshInterval = setInterval(() => {
                loadTokens({ reason: 'interval', force: false });
            }, 2000);
        }

        function stopTokensRefresh() {
            if (tokensRefreshInterval) {
                clearInterval(tokensRefreshInterval);
                tokensRefreshInterval = null;
            }
        }

        function setupSortableHeaders() {
            const thead = document.querySelector('#tokensTable thead');
            if (!thead) return;
            thead.addEventListener('click', (e) => {
                const th = e.target.closest('th.sortable');
                if (!th) return;
                const key = th.getAttribute('data-sort-key');
                if (!key) return;
                if (tokensState.sortBy === key) {
                    tokensState.sortDir = (tokensState.sortDir === 'asc') ? 'desc' : 'asc';
                } else {
                    tokensState.sortBy = key;
                    tokensState.sortDir = (key === 'symbol') ? 'asc' : 'desc';
                }
                updateSortIndicators();
                loadTokens({ reason: 'sort', force: true });
            });
            updateSortIndicators();
        }

        function updateSortIndicators() {
            const indicators = document.querySelectorAll('#tokensTable .sort-indicator');
            indicators.forEach(el => { el.textContent = ''; });
            const id = `sort-indicator-${tokensState.sortBy}`;
            const el = document.getElementById(id);
            if (el) el.textContent = tokensState.sortDir === 'asc' ? '▲' : '▼';
        }

        function safeLogoHtml(url, symbol) {
            let fallback = '?';
            if (symbol && typeof symbol === 'string') {
                for (const ch of symbol) {
                    const u = ch.toUpperCase();
                    if (/^[A-Z0-9]$/.test(u)) { fallback = u; break; }
                }
            }
            if (!url) {
                return `<div style="width:32px;height:32px;border-radius:8px;background:linear-gradient(135deg, #1565c0 0%, #0f4c81 100%);color:white;display:flex;align-items:center;justify-content:center;font-weight:700;font-size:0.85em;flex-shrink:0;">${fallback}</div>`;
            }
            const esc = (s) => (s || '').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
            return `<img src="${esc(url)}" alt="${esc(symbol || '')}" width="32" height="32" style="border-radius:8px;object-fit:cover;flex-shrink:0;box-shadow:0 1px 3px rgba(0,0,0,0.1);" onerror="fallbackLogo(this, '${fallback}')">`;
        }

        function fallbackLogo(img, letter) {
            try {
                img.onerror = null;
                const div = document.createElement('div');
                div.style.width = '32px';
                div.style.height = '32px';
                div.style.borderRadius = '8px';
                div.style.background = 'linear-gradient(135deg, #1565c0 0%, #0f4c81 100%)';
                div.style.color = 'white';
                div.style.display = 'flex';
                div.style.alignItems = 'center';
                div.style.justifyContent = 'center';
                div.style.fontWeight = '700';
                div.style.fontSize = '0.85em';
                div.style.flexShrink = '0';
                div.textContent = (letter && /^[A-Z0-9]$/.test(letter)) ? letter : '?';
                img.replaceWith(div);
            } catch (_) { /* no-op */ }
        }

        function formatPriceSol(price) {
            if (price === null || price === undefined) return 'N/A';
            if (!Number.isFinite(price)) return 'N/A';
            if (price === 0) return '0';
            return price < 0.000001 ? price.toExponential(4) : price.toFixed(8);
        }

        function formatCurrencyUSD(value) {
            if (value === null || value === undefined || !Number.isFinite(value)) return '—';
            const abs = Math.abs(value);
            let v = value;
            let suffix = '';
            if (abs >= 1_000_000_000) { v = value / 1_000_000_000; suffix = 'B'; }
            else if (abs >= 1_000_000) { v = value / 1_000_000; suffix = 'M'; }
            else if (abs >= 1_000) { v = value / 1_000; suffix = 'K'; }
            return `$${v.toFixed(2)}${suffix}`;
        }

        function formatPercent(value) {
            if (value === null || value === undefined || !Number.isFinite(value)) return '<span>—</span>';
            const cls = value > 0 ? 'color: #16a34a;' : (value < 0 ? 'color:#ef4444;' : 'color:inherit;');
            const sign = value > 0 ? '+' : '';
            return `<span style="${cls}">${sign}${value.toFixed(2)}%</span>`;
        }

        function renderSecurity(score, rugged) {
            if (rugged === true) {
                return `<span class="badge" style="background:#fee2e2;color:#b91c1c;border:1px solid #fecaca;">RUG</span>`;
            }
            if (score === null || score === undefined) return '—';
            let color = '#64748b';
            if (score >= 700) color = '#16a34a';
            else if (score >= 500) color = '#22c55e';
            else if (score >= 300) color = '#f59e0b';
            else color = '#ef4444';
            return `<span style="font-weight:600;color:${color}">${score}</span>`;
        }

        function renderStatusBadges(token) {
            const badges = [];
            if (token.has_pool_price) badges.push('<span class="badge" style="background:#dbeafe;color:#1e40af;border:1px solid #bfdbfe;">POOL</span>');
            if (token.has_ohlcv) badges.push('<span class="badge" style="background:#dcfce7;color:#166534;border:1px solid #bbf7d0;">OHLCV</span>');
            if (token.has_open_position) badges.push('<span class="badge" style="background:#fde68a;color:#92400e;border:1px solid #fcd34d;">POS</span>');
            if (token.blacklisted) badges.push('<span class="badge" style="background:#fee2e2;color:#991b1b;border:1px solid #fecaca;">BL</span>');
            return badges.join(' ');
        }

        function formatTimeAgo(timestamp) {
            if (!timestamp || !Number.isFinite(timestamp)) return '-';
            const seconds = Math.max(0, Math.floor(Date.now() / 1000) - timestamp);
            
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

        function openTokenDetail(mint) {
            window.open(`/api/tokens/${mint}`, '_blank');
        }
    </script>
    
    <div class="page-section">
        <div class="table-scroll no-x-scroll">
            <table class="table" id="tokensTable">
                <thead>
                    <tr>
                        <th class="sortable" data-sort-key="symbol"><span class="sort-label">Token <span class="sort-indicator" id="sort-indicator-symbol"></span></span></th>
                        <th class="sortable" data-sort-key="price_sol"><span class="sort-label">Price (SOL) <span class="sort-indicator" id="sort-indicator-price_sol"></span></span></th>
                        <th class="sortable" data-sort-key="liquidity_usd"><span class="sort-label">Liquidity <span class="sort-indicator" id="sort-indicator-liquidity_usd"></span></span></th>
                        <th class="sortable" data-sort-key="volume_24h"><span class="sort-label">24h Vol <span class="sort-indicator" id="sort-indicator-volume_24h"></span></span></th>
                        <th class="sortable" data-sort-key="fdv"><span class="sort-label">FDV <span class="sort-indicator" id="sort-indicator-fdv"></span></span></th>
                        <th class="sortable" data-sort-key="market_cap"><span class="sort-label">Mkt Cap <span class="sort-indicator" id="sort-indicator-market_cap"></span></span></th>
                        <th class="sortable" data-sort-key="price_change_h1"><span class="sort-label">1h <span class="sort-indicator" id="sort-indicator-price_change_h1"></span></span></th>
                        <th class="sortable" data-sort-key="price_change_h24"><span class="sort-label">24h <span class="sort-indicator" id="sort-indicator-price_change_h24"></span></span></th>
                        <th class="sortable" data-sort-key="security_score"><span class="sort-label">Security <span class="sort-indicator" id="sort-indicator-security_score"></span></span></th>
                        <th>Status</th>
                        <th class="sortable" data-sort-key="updated_at"><span class="sort-label">Updated <span class="sort-indicator" id="sort-indicator-updated_at"></span></span></th>
                        <th>Actions</th>
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

    <!-- Token Details Modal -->
    <div class="token-modal-overlay" id="tokenModal" onclick="closeTokenModal(event)">
        <div class="token-modal" onclick="event.stopPropagation()">
            <!-- Modal Header -->
            <div class="modal-header">
                <div class="modal-header-left">
                    <img id="modalTokenLogo" class="modal-token-logo" src="" alt="">
                    <div class="modal-token-info">
                        <div class="modal-token-symbol" id="modalTokenSymbol">SONIC</div>
                        <div class="modal-token-name" id="modalTokenName">Sonic SVM</div>
                    </div>
                </div>
                <div class="modal-price-info">
                    <div class="modal-price" id="modalTokenPrice">0.00077622</div>
                    <div class="modal-price-change positive" id="modalTokenChange">+2.12%</div>
                </div>
                <button class="modal-close-btn" onclick="closeTokenModal()" title="Close (ESC)">×</button>
            </div>

            <!-- Modal Body -->
            <div class="modal-body token-detail-body">
                <!-- Main Content Area -->
                <div class="modal-main">
                    <!-- Tabs Navigation -->
                    <div class="modal-tabs">
                        <button class="modal-tab active" onclick="switchModalTab('overview')">📊 Overview</button>
                        <button class="modal-tab" onclick="switchModalTab('security')">🛡️ Security</button>
                        <button class="modal-tab" onclick="switchModalTab('chart')">📈 Chart</button>
                        <button class="modal-tab" onclick="switchModalTab('transactions')">💱 Transactions</button>
                    </div>

                    <!-- Tab Content -->
                    <div class="modal-content">
                        <!-- Overview Tab -->
                        <div class="tab-pane active" id="tab-overview">
                            <div class="metrics-grid">
                                <div class="metric-card">
                                    <div class="metric-card-header">🧾 Token Profile</div>
                                    <div class="metric-row" title="Full mint address">
                                        <span class="metric-label">Mint</span>
                                        <span class="metric-value" id="detail-mint" style="font-size:0.75em; letter-spacing:0.3px; overflow:hidden; text-overflow:ellipsis; max-width:160px;">—</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Decimals</span>
                                        <span class="metric-value" id="detail-decimals">—</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Verified</span>
                                        <span class="metric-value" id="detail-verified">—</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Blacklisted</span>
                                        <span class="metric-value" id="detail-blacklisted">—</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Launch</span>
                                        <span class="metric-value" id="detail-launch-date">—</span>
                                    </div>
                                </div>
                                <div class="metric-card">
                                    <div class="metric-card-header">💰 Market Data</div>
                                    <div class="metric-row">
                                        <span class="metric-label">Liquidity</span>
                                        <span class="metric-value" id="detail-liquidity">$628.66K</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Volume 24h</span>
                                        <span class="metric-value" id="detail-volume24h">$124.32K</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Volume 6h</span>
                                        <span class="metric-value" id="detail-volume6h">$45.67K</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Volume 1h</span>
                                        <span class="metric-value" id="detail-volume1h">$12.45K</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">FDV</span>
                                        <span class="metric-value" id="detail-fdv">$432.41M</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Market Cap</span>
                                        <span class="metric-value" id="detail-marketcap">$64.86M</span>
                                    </div>
                                </div>

                                <div class="metric-card">
                                    <div class="metric-card-header">🏊 Pool Metrics</div>
                                    <div class="metric-row">
                                        <span class="metric-label">DEX</span>
                                        <span class="metric-value" id="detail-dex">Raydium CPMM</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Pool Address</span>
                                        <span class="metric-value" id="detail-pool" style="font-size:0.75em;">—</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">SOL Reserves</span>
                                        <span class="metric-value" id="detail-sol-reserves">3,241.5</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Token Reserves</span>
                                        <span class="metric-value" id="detail-token-reserves">84.2M</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Confidence</span>
                                        <span class="metric-value" id="detail-confidence">0.95</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Pool Age</span>
                                        <span class="metric-value" id="detail-pool-age">2h 15m</span>
                                    </div>
                                </div>

                                <div class="metric-card">
                                    <div class="metric-card-header">📊 Price Changes</div>
                                    <div class="metric-row">
                                        <span class="metric-label">5 minutes</span>
                                        <span class="metric-value" id="detail-change5m">+2.5%</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">1 hour</span>
                                        <span class="metric-value" id="detail-change1h">-1.2%</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">6 hours</span>
                                        <span class="metric-value" id="detail-change6h">+5.8%</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">24 hours</span>
                                        <span class="metric-value" id="detail-change24h">+15.3%</span>
                                    </div>
                                </div>

                                <div class="metric-card">
                                    <div class="metric-card-header">💱 Transaction Flow</div>
                                    <div class="metric-row">
                                        <span class="metric-label">Buys 24h</span>
                                        <span class="metric-value" id="detail-buys24h">432</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Sells 24h</span>
                                        <span class="metric-value" id="detail-sells24h">389</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Buy/Sell Ratio</span>
                                        <span class="metric-value" id="detail-ratio">1.11</span>
                                    </div>
                                    <div class="metric-row">
                                        <span class="metric-label">Net Flow</span>
                                        <span class="metric-value" id="detail-netflow">+43 Buys</span>
                                    </div>
                                </div>
                            </div>
                        </div>

                        <!-- Security Tab -->
                        <div class="tab-pane" id="tab-security">
                            <div class="security-badge excellent" id="security-badge">
                                <span>🛡️</span>
                                <span>Security Score: <span id="security-score-value">35593</span> / 40000</span>
                            </div>

                            <div class="metric-card" style="margin-bottom: 20px;">
                                <div class="metric-card-header">Overall Security Analysis</div>
                                <div class="progress-bar">
                                    <div class="progress-fill" id="security-progress" style="width: 89%"></div>
                                </div>
                                <div style="margin-top: 12px; color: #94a3b8; font-size: 0.9em;" id="security-status">
                                    ✅ This token passes major security checks
                                </div>
                            </div>

                            <div class="metric-card" style="margin-bottom: 20px;">
                                <div class="metric-card-header">🔑 Token Authorities</div>
                                <div class="metric-row">
                                    <span class="metric-label">Mint Authority</span>
                                    <span class="metric-value" id="detail-mint-auth" style="font-size:0.75em;">None ✅</span>
                                </div>
                                <div class="metric-row">
                                    <span class="metric-label">Freeze Authority</span>
                                    <span class="metric-value" id="detail-freeze-auth" style="font-size:0.75em;">None ✅</span>
                                </div>
                            </div>

                            <div class="metric-card" style="margin-bottom: 20px;">
                                <div class="metric-card-header">👥 Holder Distribution</div>
                                <div class="metric-row">
                                    <span class="metric-label">Total Holders</span>
                                    <span class="metric-value" id="detail-holders">1,234</span>
                                </div>
                                <div class="metric-row">
                                    <span class="metric-label">Top 10 Concentration</span>
                                    <span class="metric-value" id="detail-top10">45.2%</span>
                                </div>
                            </div>

                            <div class="metric-card-header" style="margin-bottom: 12px;">⚠️ Risk Factors</div>
                            <div class="risk-list" id="risk-factors-list">
                                <!-- Risk items will be populated here -->
                            </div>
                        </div>

                        <!-- Chart Tab -->
                        <div class="tab-pane" id="tab-chart">
                            <div id="token-chart-container" style="height: 500px; width: 100%; position: relative; background: #1a1a1a; border-radius: 8px; overflow: hidden;">
                                <div id="token-chart" style="width: 100%; height: 100%;"></div>
                                <div id="chart-loading" class="chart-loading">
                                    <div class="spinner"></div>
                                    <p>Loading chart data...</p>
                                </div>
                                <div id="chart-error" class="chart-error" style="display: none;">
                                    <p>Failed to load chart data</p>
                                    <button onclick="retryChartLoad()">Retry</button>
                                </div>
                            </div>
                            
                            <!-- Chart Controls -->
                            <div class="chart-controls">
                                <div class="timeframe-selector">
                                    <button class="tf-btn active" data-tf="1m">1m</button>
                                    <button class="tf-btn" data-tf="5m">5m</button>
                                    <button class="tf-btn" data-tf="15m">15m</button>
                                    <button class="tf-btn" data-tf="1h">1h</button>
                                    <button class="tf-btn" data-tf="4h">4h</button>
                                    <button class="tf-btn" data-tf="1d">1D</button>
                                </div>
                                
                                <div class="indicator-toggles">
                                    <label><input type="checkbox" id="show-volume" checked> Volume</label>
                                    <label><input type="checkbox" id="show-ma20"> MA(20)</label>
                                    <label><input type="checkbox" id="show-ma50"> MA(50)</label>
                                </div>
                            </div>
                        </div>

                        <!-- Transactions Tab -->
                        <div class="tab-pane" id="tab-transactions">
                            <div class="metric-card">
                                <div class="metric-card-header">📜 Recent Transactions</div>
                                <div id="transactions-list" style="color: #94a3b8; text-align: center; padding: 40px;">
                                    Loading recent transactions...
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Sidebar -->
                <div class="modal-sidebar">
                    <div class="sidebar-section">
                        <div class="sidebar-section-title">⚡ Live Metrics</div>
                        <div class="sidebar-metric">
                            <span class="sidebar-metric-label">Price (SOL)</span>
                            <span class="sidebar-metric-value" id="sidebar-price">0.00077622</span>
                        </div>
                        <div class="sidebar-metric">
                            <span class="sidebar-metric-label">24h Volume</span>
                            <span class="sidebar-metric-value" id="sidebar-volume">$124.3K</span>
                        </div>
                        <div class="sidebar-metric">
                            <span class="sidebar-metric-label">Liquidity</span>
                            <span class="sidebar-metric-value" id="sidebar-liq">$628.7K</span>
                        </div>
                        <div class="sidebar-metric">
                            <span class="sidebar-metric-label">Market Cap</span>
                            <span class="sidebar-metric-value" id="sidebar-mcap">$64.86M</span>
                        </div>
                    </div>

                    <div class="sidebar-section">
                        <div class="sidebar-section-title">🔗 Quick Actions</div>
                        <div class="quick-actions">
                            <button class="quick-action-btn" id="action-copy-mint" onclick="copyMintFromModal()">
                                📋 Copy Mint Address
                            </button>
                            <button class="quick-action-btn" id="action-dexscreener" onclick="openDexScreenerFromModal()">
                                📊 Open DexScreener →
                            </button>
                            <button class="quick-action-btn" id="action-gmgn" onclick="openGMGNFromModal()">
                                🔗 Open GMGN →
                            </button>
                            <button class="quick-action-btn" id="action-solscan" onclick="openSolscanFromModal()">
                                🔍 Open Solscan →
                            </button>
                        </div>
                    </div>

                    <div class="sidebar-section">
                        <div class="sidebar-section-title">🏷️ Status Flags</div>
                        <div id="sidebar-badges" style="display: flex; flex-wrap: wrap; gap: 6px;">
                            <!-- Badges will be populated here -->
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
    
    <script>
        // ====================================================================
        // WebSocket real-time price updates
        // ====================================================================
        
        function handlePriceUpdate(message) {
            if (!message || typeof message !== 'object') return;
            const { mint, price_result: priceResult, timestamp } = message;
            if (!mint || !priceResult || typeof priceResult !== 'object') return;

            const tokenInList = allTokensData.find(t => t.mint === mint);
            const updatedPriceSol = Number(priceResult.price_sol);
            const updatedPriceUsd = Number(priceResult.price_usd);
            const updatedTimestamp = (() => {
                if (Number.isFinite(priceResult.timestamp)) return Number(priceResult.timestamp);
                if (typeof timestamp === 'string') {
                    const parsed = Date.parse(timestamp);
                    if (Number.isFinite(parsed)) return Math.floor(parsed / 1000);
                }
                return null;
            })();

            if (tokenInList) {
                if (Number.isFinite(updatedPriceSol)) {
                    tokenInList.price_sol = updatedPriceSol;
                }
                if (Number.isFinite(updatedPriceUsd)) {
                    tokenInList.price_usd = updatedPriceUsd;
                }
                if (updatedTimestamp !== null) {
                    tokenInList.price_updated_at = updatedTimestamp;
                }

                const row = document.querySelector(`tr[data-mint="${mint}"]`);
                if (row) {
                    const priceCell = row.cells?.[1];
                    const liquidityCell = row.cells?.[2];
                    const timeCell = row.cells?.[10];

                    if (priceCell && Number.isFinite(updatedPriceSol)) {
                        priceCell.innerHTML = formatPriceSol(updatedPriceSol);
                        priceCell.style.animation = 'highlight 0.6s ease-out';
                        setTimeout(() => { priceCell.style.animation = ''; }, 600);
                    }

                    if (liquidityCell && Number.isFinite(tokenInList.liquidity_usd)) {
                        liquidityCell.innerHTML = formatCurrencyUSD(tokenInList.liquidity_usd);
                    }

                    if (timeCell && updatedTimestamp !== null) {
                        timeCell.textContent = formatTimeAgo(updatedTimestamp);
                    }
                }
            }

            if (currentModalMint === mint) {
                const priceEl = document.getElementById('detail-price');
                const liquidityEl = document.getElementById('detail-liquidity');
                const sidebarPriceEl = document.getElementById('sidebar-price');
                const sidebarLiqEl = document.getElementById('sidebar-liq');

                if (priceEl && Number.isFinite(updatedPriceSol)) {
                    priceEl.textContent = formatPriceSol(updatedPriceSol);
                }
                if (sidebarPriceEl && Number.isFinite(updatedPriceSol)) {
                    sidebarPriceEl.textContent = formatPriceSol(updatedPriceSol);
                }

                if (Number.isFinite(tokenInList?.liquidity_usd)) {
                    const formatted = formatCurrencyUSD(tokenInList.liquidity_usd);
                    if (liquidityEl) liquidityEl.textContent = formatted;
                    if (sidebarLiqEl) sidebarLiqEl.textContent = formatted;
                }
            }
        }
        
        // Subscribe to WebSocket price updates
        if (typeof WsHub !== 'undefined') {
            WsHub.subscribe('prices', handlePriceUpdate);
            console.log('[Tokens] Subscribed to WebSocket price updates');
            
            // Cleanup on page unload
            window.addEventListener('beforeunload', () => {
                WsHub.unsubscribe('prices', handlePriceUpdate);
            });
        }
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
        <style>
            .event-ref-cell {
                display: inline-flex;
                align-items: center;
                gap: 6px;
                font-family: monospace;
            }

            .event-ref {
                color: var(--text-primary);
                max-width: 140px;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
            }

            .event-ref-copy {
                background: none;
                border: none;
                color: var(--link-color);
                cursor: pointer;
                font-size: 0.8em;
                padding: 2px 4px;
                border-radius: 4px;
                transition: color 0.2s ease;
            }

            .event-ref-copy:hover,
            .event-ref-copy:focus {
                color: var(--link-hover);
                outline: none;
            }

            .event-message-cell {
                display: flex;
                align-items: center;
                gap: 8px;
            }

            .event-message-text {
                flex: 1;
                min-width: 0;
                overflow: hidden;
                text-overflow: ellipsis;
                color: var(--text-primary);
            }

            .event-detail-btn {
                background: none;
                border: 1px solid var(--border-color);
                color: var(--text-secondary);
                cursor: pointer;
                font-size: 0.75em;
                padding: 2px 6px;
                border-radius: 4px;
                transition: all 0.2s ease;
            }

            .event-detail-btn:hover,
            .event-detail-btn:focus {
                color: var(--link-color);
                border-color: var(--link-color);
                outline: none;
            }

            .event-detail-modal {
                max-width: 680px;
                width: calc(100% - 48px);
            }

            .event-detail-meta {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
                gap: 12px;
                margin-bottom: 16px;
            }

            .event-detail-meta .meta-label {
                display: block;
                font-size: 0.75em;
                text-transform: uppercase;
                letter-spacing: 0.05em;
                color: var(--text-muted);
            }

            .event-detail-meta .meta-value {
                display: block;
                font-size: 0.9em;
                color: var(--text-primary);
                word-break: break-all;
                margin-top: 2px;
            }

            .event-detail-json-wrapper {
                background: var(--bg-card);
                border: 1px solid var(--border-color);
                border-radius: 8px;
                max-height: 320px;
                overflow: auto;
            }

            .event-detail-json {
                margin: 0;
                padding: 12px;
                font-size: 0.85em;
                line-height: 1.4;
                color: var(--text-primary);
            }
        </style>
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
        <div id="eventDetailModal" class="modal-overlay">
            <div class="modal-content event-detail-modal">
                <div class="modal-header">
                    <span class="modal-title" id="eventDetailTitle">Event Details</span>
                    <button id="eventDetailClose" class="modal-close" type="button" aria-label="Close">
                        ×
                    </button>
                </div>
                <div class="modal-body">
                    <div class="event-detail-meta">
                        <div>
                            <span class="meta-label">Category</span>
                            <span class="meta-value" id="eventDetailCategory">-</span>
                        </div>
                        <div>
                            <span class="meta-label">Severity</span>
                            <span class="meta-value" id="eventDetailSeverity">-</span>
                        </div>
                        <div>
                            <span class="meta-label">Time</span>
                            <span class="meta-value" id="eventDetailTime">-</span>
                        </div>
                        <div>
                            <span class="meta-label">Reference</span>
                            <span class="meta-value" id="eventDetailReference">-</span>
                        </div>
                        <div>
                            <span class="meta-label">Mint</span>
                            <span class="meta-value" id="eventDetailMint">-</span>
                        </div>
                    </div>
                    <div class="event-detail-json-wrapper">
                        <pre id="eventDetailPayload" class="event-detail-json">Loading...</pre>
                    </div>
                </div>
            </div>
        </div>
    </div>
    
    <script>
    let allEventsData = [];
    let eventsRefreshInterval = null;
    let maxEventId = 0;
    let connectionStatus = 'connecting'; // 'connecting', 'connected', 'disconnected', 'error'
    const eventsStore = new Map();
    const EVENT_MAX_RECORDS = 1000;
    let eventsLoadInFlight = false;
    let pendingEventsLoad = false;
    let searchDebounceHandle = null;
        
        // Update connection status indicator
        function updateConnectionStatus(status, message) {
            connectionStatus = status;
            const indicator = document.getElementById('eventsCountText');
            if (!indicator) return;
            const colors = {
                connecting: 'var(--text-secondary)',
                connected: 'var(--badge-online)',
                disconnected: 'var(--badge-loading)',
                error: 'var(--badge-error)'
            };
            const icons = {
                connecting: '⏳',
                connected: '✅',
                disconnected: '⚠️',
                error: '❌'
            };
            const defaults = {
                connecting: 'Connecting…',
                connected: 'Connected',
                disconnected: 'Disconnected',
                error: 'Error'
            };
            const icon = icons[status] || 'ℹ️';
            const text = message || defaults[status] || 'Status unavailable';
            indicator.style.color = colors[status] || 'var(--text-secondary)';
            indicator.textContent = `${icon} ${text}`;
            indicator.dataset.status = status;
            indicator.title = text;
        }
        
        function clearEventsCache() {
            eventsStore.clear();
            allEventsData = [];
        }

        function normalizeEvent(raw) {
            if (!raw || typeof raw !== 'object') return null;
            const severity = (raw.severity || '').toLowerCase();
            const message = raw.message ?? 'No message';
            return {
                ...raw,
                severity,
                message,
                payload: raw.payload ?? null,
            };
        }

        function hasEventChanged(existing, next) {
            if (!existing) return true;
            return (
                existing.message !== next.message ||
                existing.severity !== next.severity ||
                existing.category !== next.category ||
                existing.subtype !== next.subtype ||
                existing.reference_id !== next.reference_id ||
                existing.event_time !== next.event_time ||
                existing.mint !== next.mint ||
                JSON.stringify(existing.payload) !== JSON.stringify(next.payload)
            );
        }

        function matchesActiveFilters(event) {
            const category = document.getElementById('categoryFilter')?.value || '';
            const severity = document.getElementById('severityFilter')?.value || '';
            if (category && event.category !== category) return false;
            if (severity && event.severity !== severity) return false;
            return true;
        }

        function mergeEvents(events) {
            if (!Array.isArray(events) || events.length === 0) {
                return false;
            }

            let updated = false;
            for (const raw of events) {
                const event = normalizeEvent(raw);
                if (!event || typeof event.id !== 'number') continue;
                if (!matchesActiveFilters(event)) continue;
                const eventId = Number(event.id);
                const existing = eventsStore.get(eventId);
                if (hasEventChanged(existing, event)) {
                    eventsStore.set(eventId, event);
                    updated = true;
                }
            }

            if (!updated) {
                return false;
            }

            const orderedIds = Array.from(eventsStore.keys()).sort((a, b) => b - a);
            if (orderedIds.length > EVENT_MAX_RECORDS) {
                for (let i = EVENT_MAX_RECORDS; i < orderedIds.length; i++) {
                    eventsStore.delete(orderedIds[i]);
                }
                orderedIds.length = EVENT_MAX_RECORDS;
            }

            allEventsData = orderedIds.map(id => eventsStore.get(id));
            renderEvents();
            return true;
        }

        function stopEventsRefresh() {
            if (eventsRefreshInterval) {
                clearInterval(eventsRefreshInterval);
                eventsRefreshInterval = null;
            }
        }

        // Load events from API
        async function loadEvents(options = {}) {
            const { force = false } = options;
            if (eventsLoadInFlight) {
                if (force) {
                    pendingEventsLoad = true;
                }
                return;
            }

            const category = document.getElementById('categoryFilter').value;
            const severity = document.getElementById('severityFilter').value;
            const params = new URLSearchParams();
            params.set('limit', '200');
            if (category) params.set('category', category);
            if (severity) params.set('severity', severity);

            let url = '';
            const isInitialLoad = force || maxEventId === 0;
            try {
                eventsLoadInFlight = true;
                if (isInitialLoad) {
                    url = `/api/events/head?${params.toString()}`;
                    updateConnectionStatus('connecting', 'Loading events...');
                } else {
                    params.set('after_id', String(maxEventId));
                    url = `/api/events/since?${params.toString()}`;
                }

                const res = await fetch(url, { cache: 'no-cache' });
                if (!res.ok) {
                    throw new Error(`HTTP ${res.status}: ${res.statusText}`);
                }

                const data = await res.json();
                const payload = Array.isArray(data.events) ? data.events : [];

                if (isInitialLoad) {
                    clearEventsCache();
                }

                const added = mergeEvents(payload);
                if (added) {
                    const highestFromPayload = payload.reduce((max, evt) => (
                        evt && typeof evt.id === 'number' ? Math.max(max, evt.id) : max
                    ), 0);
                    maxEventId = Math.max(maxEventId, data.max_id || 0, highestFromPayload);
                } else if (isInitialLoad) {
                    renderEvents();
                    maxEventId = Math.max(maxEventId, data.max_id || 0);
                }

                const totalEvents = allEventsData.length;
                const countLabel = formatEventCountLabel(totalEvents);
                if (typeof WsHub !== 'undefined' && WsHub.isConnected()) {
                    updateConnectionStatus('connected', `Realtime • ${countLabel}`);
                    stopEventsRefresh();
                } else {
                    updateConnectionStatus('connected', `Polling • ${countLabel}`);
                }

                AppState.save('events_category', category);
                AppState.save('events_severity', severity);

            } catch (error) {
                console.error('Failed to load events:', error);

                let errorMsg = 'Failed to connect to server';
                if (error.message.includes('Failed to fetch')) {
                    errorMsg = 'Server not responding - check if bot is running';
                } else if (error.message.includes('HTTP')) {
                    errorMsg = `Server error: ${error.message}`;
                }

                document.getElementById('eventsTableBody').innerHTML = `
                    <tr>
                        <td colspan="6" style="text-align: center; padding: 40px;">
                            <div style="color: var(--badge-error); margin-bottom: 10px;">❌ ${errorMsg}</div>
                            <div style="color: var(--text-muted); font-size: 0.9em;">
                                Retrying automatically... or click Refresh to try again
                            </div>
                        </td>
                    </tr>
                `;
                updateConnectionStatus('error', errorMsg);
                startEventsRefresh(3000);
            } finally {
                eventsLoadInFlight = false;
                if (pendingEventsLoad) {
                    pendingEventsLoad = false;
                    loadEvents();
                }
            }
        }
        
        // Render events in table
        function getSearchTerm() {
            return (document.getElementById('eventSearch')?.value || '').toLowerCase();
        }

        function matchesSearch(event, searchTerm) {
            if (!searchTerm) return true;
            const haystacks = [
                event.category,
                event.subtype,
                event.message,
                event.reference_id,
                event.mint
            ];
            return haystacks.some(value =>
                typeof value === 'string' && value.toLowerCase().includes(searchTerm)
            );
        }

        function formatEventCountLabel(count) {
            const safe = Number.isFinite(count) ? count : 0;
            const plural = safe === 1 ? 'event' : 'events';
            return `${safe.toLocaleString()} ${plural}`;
        }

        function formatReferenceSnippet(value) {
            if (typeof value !== 'string') return '';
            if (value.length <= 12) return value;
            return `${value.slice(0, 6)}…${value.slice(-6)}`;
        }

        function renderEvents(forcedEvents) {
            const tbody = document.getElementById('eventsTableBody');
            const searchTerm = getSearchTerm();
            const events = Array.isArray(forcedEvents) ? forcedEvents : allEventsData;
            const filtered = events.filter(event => matchesSearch(event, searchTerm));

            if (!filtered || filtered.length === 0) {
                const message = connectionStatus === 'error' 
                    ? '⚠️ Error loading events. Please check server status.' 
                    : connectionStatus === 'connecting'
                    ? '🔄 Connecting to server...'
                    : '📋 No events found. Events will appear here as they occur.';
                    
                tbody.innerHTML = `
                    <tr>
                        <td colspan="6" style="text-align: center; padding: 40px; color: var(--text-muted);">
                            ${message}
                        </td>
                    </tr>
                `;
                return;
            }
            
            tbody.innerHTML = filtered.map(event => {
                const time = formatTimeAgo(new Date(event.event_time));
                const severityColor = getSeverityColor(event.severity);
                let referenceCell = '-';
                if (event.reference_id) {
                    const snippet = escapeHtml(formatReferenceSnippet(event.reference_id));
                    const full = escapeHtml(event.reference_id);
                    const encoded = encodeURIComponent(event.reference_id);
                    referenceCell = `
                        <span class="event-ref" title="${full}">${snippet}</span>
                        <button class="event-ref-copy" type="button" data-ref="${encoded}" title="Copy reference">📋</button>
                    `;
                }
                const messageText = escapeHtml(event.message);
                const detailButton = `<button class="event-detail-btn" type="button" data-event-id="${event.id}" title="View full event">Details</button>`;
                
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
                                ${event.severity.toUpperCase()}
                            </span>
                        </td>
                        <td style="padding: 10px; max-width: 420px;">
                            <div class="event-message-cell">
                                <span class="event-message-text" title="${messageText}">${messageText}</span>
                                ${detailButton}
                            </div>
                        </td>
                        <td style="padding: 10px; font-size: 0.85em;">
                            <span class="event-ref-cell">${referenceCell}</span>
                        </td>
                    </tr>
                `;
            }).join('');
        }
        
        // Get color for severity
        function getSeverityColor(severity) {
            const key = (severity || '').toLowerCase();
            const colors = {
                info: 'var(--badge-online)',
                warn: 'var(--badge-loading)',
                error: 'var(--badge-error)',
                debug: '#6b7280'
            };
            return colors[key] || '#6b7280';
        }
        
        // Search events
        document.getElementById('eventSearch').addEventListener('input', (e) => {
            const searchTerm = e.target.value.toLowerCase();
            AppState.save('events_search', searchTerm);
            if (searchDebounceHandle) {
                clearTimeout(searchDebounceHandle);
            }
            searchDebounceHandle = setTimeout(() => {
                renderEvents();
            }, 120);
        });
        
        // Filter by category
        document.getElementById('categoryFilter').addEventListener('change', () => {
            maxEventId = 0; // Reset to get fresh data with new filter
            allEventsData = [];
            clearEventsCache();
            loadEvents();
        });
        
        // Filter by severity
        document.getElementById('severityFilter').addEventListener('change', () => {
            maxEventId = 0; // Reset to get fresh data with new filter
            allEventsData = [];
            clearEventsCache();
            loadEvents();
        });
        
        // Refresh button
        document.getElementById('refreshEvents').addEventListener('click', loadEvents);

        // Reference copy handler
        document.getElementById('eventsTableBody').addEventListener('click', async (event) => {
            const copyBtn = event.target.closest('.event-ref-copy');
            if (copyBtn) {
                const encoded = copyBtn.getAttribute('data-ref');
                if (!encoded) return;
                const reference = decodeURIComponent(encoded);
                try {
                    await navigator.clipboard.writeText(reference);
                    showToast('✅ Reference copied to clipboard');
                } catch (err) {
                    console.error('Failed to copy reference:', err);
                    showToast('❌ Failed to copy reference', 'error');
                }
                return;
            }

            const detailBtn = event.target.closest('.event-detail-btn');
            if (detailBtn) {
                const idAttr = detailBtn.getAttribute('data-event-id');
                const eventId = Number(idAttr);
                if (Number.isFinite(eventId)) {
                    openEventDetailModal(eventId);
                }
            }
        });

        const eventDetailModal = document.getElementById('eventDetailModal');
        const eventDetailClose = document.getElementById('eventDetailClose');

        function openEventDetailModal(eventId) {
            if (!eventDetailModal) return;
            const eventData = eventsStore.get(eventId);
            if (!eventData) {
                showToast('❌ Event details unavailable', 'error');
                return;
            }

            document.getElementById('eventDetailTitle').textContent = `${(eventData.category || 'event').toUpperCase()} • #${eventData.id}`;
            document.getElementById('eventDetailCategory').textContent = (eventData.category || '-').toUpperCase();
            document.getElementById('eventDetailSeverity').textContent = (eventData.severity || '-').toUpperCase();
            document.getElementById('eventDetailTime').textContent = eventData.event_time ? new Date(eventData.event_time).toLocaleString() : '-';
            document.getElementById('eventDetailReference').textContent = eventData.reference_id || '—';
            document.getElementById('eventDetailMint').textContent = eventData.mint || '—';

            const payloadEl = document.getElementById('eventDetailPayload');
            if (eventData.payload !== null && eventData.payload !== undefined) {
                try {
                    payloadEl.textContent = JSON.stringify(eventData.payload, null, 2);
                } catch (err) {
                    console.error('Failed to stringify payload:', err);
                    payloadEl.textContent = 'Unable to render payload.';
                }
            } else {
                payloadEl.textContent = 'No payload data available.';
            }

            eventDetailModal.classList.add('show');
        }

        function closeEventDetailModal() {
            if (!eventDetailModal) return;
            eventDetailModal.classList.remove('show');
        }

        if (eventDetailClose) {
            eventDetailClose.addEventListener('click', closeEventDetailModal);
        }
        if (eventDetailModal) {
            eventDetailModal.addEventListener('click', (event) => {
                if (event.target === eventDetailModal) {
                    closeEventDetailModal();
                }
            });
        }

        document.addEventListener('keydown', (event) => {
            if (event.key === 'Escape' && eventDetailModal && eventDetailModal.classList.contains('show')) {
                closeEventDetailModal();
            }
        });
        
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
        function startEventsRefresh(intervalMs = 2000) {
            if (eventsRefreshInterval) return;
            eventsRefreshInterval = setInterval(() => {
                loadEvents();
            }, intervalMs);
        }

        // Handle incoming event from WebSocket
        function handleEventsWebSocket(event) {
            // Handle lag warning
            if (event.warning === 'lagged') {
                console.warn('[Events] WebSocket lagged, catching up via HTTP');
                updateConnectionStatus('disconnected', 'Connection lagged, catching up...');
                startEventsRefresh(1500);
                loadEvents();
                return;
            }
            
            // Add new event to list
            if (event && typeof event.id === 'number') {
                if (!matchesActiveFilters(normalizeEvent(event))) {
                    return;
                }
                maxEventId = Math.max(maxEventId, event.id);
                mergeEvents([event]);
                updateConnectionStatus('connected', `Realtime • ${formatEventCountLabel(allEventsData.length)}`);
            }
        }
        
        // Handle WebSocket warnings (lag detection)
        function handleWebSocketWarning(data) {
            if (data.channel === 'events') {
                console.warn('[Events] Channel warning:', data);
                updateConnectionStatus('disconnected', `Polling • ${formatEventCountLabel(allEventsData.length)}`);
                startEventsRefresh(2000);
                loadEvents();
            }
        }

        function handleEventsDisconnect() {
            console.warn('[Events] WebSocket disconnected, switching to polling');
            updateConnectionStatus('disconnected', `Polling • ${formatEventCountLabel(allEventsData.length)}`);
            startEventsRefresh(3000);
        }

        function handleEventsReconnect() {
            console.log('[Events] WebSocket reconnected');
            stopEventsRefresh();
            updateConnectionStatus('connected', `Realtime • ${formatEventCountLabel(allEventsData.length)}`);
            if (!allEventsData.length) {
                loadEvents({ force: true });
            }
        }
        
        // Subscribe to WebSocket events
        if (typeof WsHub !== 'undefined') {
            WsHub.subscribe('events', handleEventsWebSocket);
            WsHub.subscribe('_warning', handleWebSocketWarning);
            WsHub.subscribe('_disconnected', handleEventsDisconnect);
            WsHub.subscribe('_failed', handleEventsDisconnect);
            WsHub.subscribe('_connected', handleEventsReconnect);
            console.log('[Events] Subscribed to WebSocket events channel');
            
            // Update status based on WsHub connection state
            if (WsHub.isConnected()) {
                updateConnectionStatus('connected', `Realtime • ${formatEventCountLabel(allEventsData.length)}`);
                // Stop HTTP polling if running
                stopEventsRefresh();
            } else {
                updateConnectionStatus('connecting', 'Connecting to WebSocket...');
            }
            
            // Cleanup on page unload
            window.addEventListener('beforeunload', () => {
                WsHub.unsubscribe('events', handleEventsWebSocket);
                WsHub.unsubscribe('_warning', handleWebSocketWarning);
                WsHub.unsubscribe('_disconnected', handleEventsDisconnect);
                WsHub.unsubscribe('_failed', handleEventsDisconnect);
                WsHub.unsubscribe('_connected', handleEventsReconnect);
            });
        } else {
            console.warn('[Events] WsHub not available, falling back to HTTP polling');
            updateConnectionStatus('disconnected', 'WebSocket unavailable (polling)');
            startEventsRefresh();
        }
        
        // Restore saved filters
        const savedCategory = AppState.load('events_category', '');
        const savedSeverity = AppState.load('events_severity', '');
        const savedSearch = AppState.load('events_search', '');
        
        if (savedCategory) document.getElementById('categoryFilter').value = savedCategory;
        if (savedSeverity) document.getElementById('severityFilter').value = savedSeverity;
        if (savedSearch) document.getElementById('eventSearch').value = savedSearch;
        
    // Initial load
    loadEvents({ force: true });
    </script>
    "#.to_string()
}

/// Services management page content
pub fn services_content() -> String {
    r#"
    <div class="services-container">
        <div class="services-toolbar">
            <div class="toolbar-left">
                <input id="serviceSearch" class="input" type="text" placeholder="Search services..." />
                <select id="statusFilter" class="input">
                    <option value="all">All statuses</option>
                    <option value="healthy">Healthy</option>
                    <option value="degraded">Degraded</option>
                    <option value="starting">Starting</option>
                    <option value="unhealthy">Unhealthy</option>
                </select>
                <label class="toggle"><input type="checkbox" id="enabledOnly" /> Enabled only</label>
            </div>
            <div class="toolbar-right">
                <div class="chip">Total <span id="totalServices">-</span></div>
                <div class="chip chip-success">Healthy <span id="healthyServices">-</span></div>
                <div class="chip chip-warn">Starting <span id="startingServices">-</span></div>
                <div class="chip chip-error">Unhealthy <span id="unhealthyServices">-</span></div>
                <div class="chip" title="Process-wide CPU">CPU <span id="processCpu">-</span></div>
                <div class="chip" title="Process-wide Memory">Mem <span id="processMemory">-</span></div>
                <button class="btn btn-primary" onclick="refreshServices()">🔄 Refresh</button>
            </div>
        </div>

        <div class="table-scroll">
            <table class="table services-table" id="servicesTable">
                <thead>
                    <tr>
                        <th data-sort="name">Name</th>
                        <th data-sort="health">Health</th>
                        <th data-sort="priority">Priority</th>
                        <th data-sort="enabled">Enabled</th>
                        <th data-sort="uptime">Uptime</th>
                        <th data-sort="activity" title="Poll time ratio (busyness)">Activity</th>
                        <th data-sort="poll">Avg Poll</th>
                        <th data-sort="pps">Polls/s</th>
                        <th data-sort="tasks">Tasks</th>
                        <th data-sort="ops">Ops/s</th>
                        <th data-sort="errors">Errors</th>
                        <th>Dependencies</th>
                    </tr>
                </thead>
                <tbody id="servicesTableBody">
                    <tr>
                        <td colspan="12" style="text-align:center; padding: 20px; color: var(--text-muted);">
                            Loading services...
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>

    <style>
        .services-container { width: 100%; max-width: 1400px; margin: 0 auto; }
        .services-toolbar { display: flex; gap: 1rem; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
        .toolbar-left, .toolbar-right { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
        .input { background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-color); border-radius: 6px; padding: 0.4rem 0.6rem; }
        .toggle { color: var(--text-muted); font-size: 0.9rem; display: flex; align-items: center; gap: 0.4rem; }
        .chip { background: var(--bg-secondary); color: var(--text-primary); border: 1px solid var(--border-color); border-radius: 999px; padding: 0.25rem 0.6rem; font-size: 0.85rem; }
        .chip-success { background: #064e3b; color: #34d399; border-color: #065f46; }
        .chip-warn { background: #4a3000; color: #fbbf24; border-color: #92400e; }
        .chip-error { background: #4c0519; color: #fb7185; border-color: #7f1d1d; }

        /* Activity bar */
        .activity-cell { min-width: 140px; }
        .activity-track { background: #1f2937; border-radius: 6px; height: 8px; width: 100%; overflow: hidden; }
        .activity-fill { height: 100%; border-radius: 6px; background: #10b981; transition: width 0.25s ease; }
        .activity-meta { display: flex; justify-content: space-between; font-size: 0.75rem; color: var(--text-muted); margin-top: 4px; }

        /* Sortable headers */
        .services-table th[data-sort] { cursor: pointer; user-select: none; }
        .services-table th[data-sort].asc::after { content: ' \25B2'; color: var(--text-muted); }
        .services-table th[data-sort].desc::after { content: ' \25BC'; color: var(--text-muted); }

        /* Dependencies */
        .dependency-badge { font-size: 0.75rem; background: var(--bg-secondary); padding: 0.25rem 0.6rem; border-radius: 12px; display: inline-block; margin: 2px; }

        /* Health badges reuse existing theme */
        .badge.success { background: var(--badge-online); color: #fff; }
        .badge.warning { background: var(--badge-loading); color: #fff; }
        .badge.error { background: var(--badge-error); color: #fff; }
        .badge.secondary { background: var(--bg-secondary); color: var(--text-primary); }
    </style>

    <script>
        let servicesData = null;
        let sortKey = 'priority';
        let sortDir = 'asc';

        async function loadServices() {
            try {
                const response = await fetch('/api/services/overview');
                const data = await response.json();
                servicesData = data; // direct
                bindToolbarEventsOnce();
                renderServicesTable();
                bindSortHandlersOnce();
            } catch (error) {
                console.error('Failed to load services:', error);
                const tbody = document.getElementById('servicesTableBody');
                if (tbody) {
                    tbody.innerHTML = `
                        <tr>
                            <td colspan="12" style="text-align:center; padding: 20px; color: #ef4444;">Failed to load services</td>
                        </tr>
                    `;
                }
            }
        }

        function currentFilters() {
            const q = (document.getElementById('serviceSearch')?.value || '').toLowerCase();
            const status = document.getElementById('statusFilter')?.value || 'all';
            const enabledOnly = document.getElementById('enabledOnly')?.checked || false;
            return { q, status, enabledOnly };
        }

        function filteredAndSortedServices() {
            if (!servicesData) return [];
            const { q, status, enabledOnly } = currentFilters();
            let arr = servicesData.services.slice();

            // Filter
            arr = arr.filter(s => {
                const matchesText = !q || s.name.toLowerCase().includes(q);
                const matchesStatus = status === 'all' || (s.health?.status === status);
                const matchesEnabled = !enabledOnly || s.enabled;
                return matchesText && matchesStatus && matchesEnabled;
            });

            // Sort
            const getKey = (s) => {
                const m = s.metrics || {};
                switch (sortKey) {
                    case 'name': return s.name || '';
                    case 'health': return healthRank(s.health?.status);
                    case 'priority': return s.priority || 0;
                    case 'enabled': return s.enabled ? 1 : 0;
                    case 'uptime': return s.uptime_seconds || 0;
                    case 'activity': {
                        const total = (m.total_poll_duration_ns||0)+(m.total_idle_duration_ns||0);
                        return total>0 ? (m.total_poll_duration_ns||0)/total : 0;
                    }
                    case 'poll': return m.mean_poll_duration_ns || 0;
                    case 'pps': return (s.uptime_seconds>0) ? (m.total_polls||0)/s.uptime_seconds : 0;
                    case 'tasks': return m.task_count || 0;
                    case 'ops': return m.operations_per_second || 0;
                    case 'errors': return m.errors_total || 0;
                    default: return s.priority || 0;
                }
            };

            arr.sort((a,b) => {
                const ka = getKey(a);
                const kb = getKey(b);
                if (ka < kb) return sortDir === 'asc' ? -1 : 1;
                if (ka > kb) return sortDir === 'asc' ? 1 : -1;
                return 0;
            });

            return arr;
        }

        function renderServicesTable() {
            if (!servicesData) return;

            // Update compact summary chips
            document.getElementById('totalServices').textContent = servicesData.summary.total_services;
            document.getElementById('healthyServices').textContent = servicesData.summary.healthy_services;
            document.getElementById('startingServices').textContent = servicesData.summary.starting_services;
            document.getElementById('unhealthyServices').textContent = servicesData.summary.unhealthy_services;

            // Process-wide metrics
            if (servicesData.services && servicesData.services.length > 0) {
                const m0 = servicesData.services[0].metrics || {};
                document.getElementById('processCpu').textContent = `${(m0.process_cpu_percent||0).toFixed(1)}%`;
                document.getElementById('processMemory').textContent = formatBytes(m0.process_memory_bytes||0);
            }

            const tbody = document.getElementById('servicesTableBody');
            if (!tbody) return;

            const rows = filteredAndSortedServices().map(service => {
                const m = service.metrics || {};
                const deps = service.dependencies && service.dependencies.length
                    ? service.dependencies.map(dep => `<span class=\"dependency-badge\">${dep}</span>`).join(' ')
                    : '<span class="detail-value">None</span>';

                const total = (m.total_poll_duration_ns||0)+(m.total_idle_duration_ns||0);
                const activity = total>0 ? ((m.total_poll_duration_ns||0)/total*100) : 0;
                const activityColor = activity>80 ? '#10b981' : activity>50 ? '#3b82f6' : activity>20 ? '#f59e0b' : activity>5 ? '#6b7280' : '#9ca3af';
                const avgPoll = formatDuration(m.mean_poll_duration_ns||0);
                const pps = service.uptime_seconds>0 ? ((m.total_polls||0)/service.uptime_seconds).toFixed(2) : '0.00';

                const taskInfo = m.task_count>0 ? `${m.task_count} tasks\nPoll: ${formatDuration(m.mean_poll_duration_ns)}\nIdle: ${formatDuration(m.mean_idle_duration_ns)}\nTotal Polls: ${m.total_polls||0}` : 'No instrumented tasks';

                return `
                    <tr>
                        <td style="font-weight:600;">${service.name}</td>
                        <td><span class="badge ${getHealthBadgeClass(service.health)}">${getHealthStatus(service.health)}</span></td>
                        <td>${service.priority}</td>
                        <td>${service.enabled ? '✅' : '❌'}</td>
                        <td>${formatUptime(service.uptime_seconds)}</td>
                        <td class="activity-cell" title="${activity.toFixed(1)}% busy">
                            <div class="activity-track"><div class="activity-fill" style="width:${activity.toFixed(1)}%; background:${activityColor}"></div></div>
                            <div class="activity-meta"><span>${activity.toFixed(1)}%</span><span>${m.total_polls||0} polls</span></div>
                        </td>
                        <td title="Average duration per poll">${avgPoll}</td>
                        <td title="Polls per second">${pps}</td>
                        <td title="${taskInfo}">${m.task_count||0}</td>
                        <td title="Operations per second">${(m.operations_per_second||0).toFixed(2)}</td>
                        <td title="Total errors">${m.errors_total||0}</td>
                        <td>${deps}</td>
                    </tr>`;
            }).join('');

            tbody.innerHTML = rows || `
                <tr>
                    <td colspan="12" style="text-align:center; padding: 20px; color: var(--text-muted);">No services</td>
                </tr>`;
        }

        function bindToolbarEventsOnce() {
            if (bindToolbarEventsOnce._bound) return; bindToolbarEventsOnce._bound = true;
            document.getElementById('serviceSearch')?.addEventListener('input', renderServicesTable);
            document.getElementById('statusFilter')?.addEventListener('change', renderServicesTable);
            document.getElementById('enabledOnly')?.addEventListener('change', renderServicesTable);
        }

        function bindSortHandlersOnce() {
            if (bindSortHandlersOnce._bound) return; bindSortHandlersOnce._bound = true;
            document.querySelectorAll('#servicesTable thead th[data-sort]').forEach(th => {
                th.addEventListener('click', () => {
                    const key = th.getAttribute('data-sort');
                    if (!key) return;
                    if (sortKey === key) { sortDir = (sortDir === 'asc') ? 'desc' : 'asc'; } else { sortKey = key; sortDir = 'asc'; }
                    document.querySelectorAll('#servicesTable thead th[data-sort]').forEach(h => h.classList.remove('asc','desc'));
                    th.classList.add(sortDir);
                    renderServicesTable();
                });
            });
        }

        function healthRank(status) {
            switch (status) {
                case 'healthy': return 3;
                case 'degraded': return 2;
                case 'starting': return 1;
                case 'unhealthy': return 0;
                default: return -1;
            }
        }

        function getHealthStatus(health) {
            if (health?.status === 'healthy') return '✅ Healthy';
            if (health?.status === 'starting') return '⏳ Starting';
            if (health?.status === 'degraded') return '⚠️ Degraded';
            if (health?.status === 'unhealthy') return '❌ Unhealthy';
            return '⏸️ ' + (health?.status || 'unknown');
        }

        function getHealthBadgeClass(health) {
            if (health?.status === 'healthy') return 'success';
            if (health?.status === 'starting') return 'warning';
            if (health?.status === 'degraded') return 'warning';
            if (health?.status === 'unhealthy') return 'error';
            return 'secondary';
        }

        function formatUptime(seconds) {
            if (!seconds || seconds < 60) return `${seconds||0}s`;
            if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
            if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
            return `${Math.floor(seconds / 86400)}d ${Math.floor((seconds % 86400) / 3600)}h`;
        }

        function formatBytes(bytes) {
            if (!bytes || bytes < 1024) return `${bytes||0} B`;
            if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
            if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
            return `${(bytes / 1073741824).toFixed(2)} GB`;
        }

        function formatDuration(nanos) {
            if (!nanos || nanos < 1000) return `${nanos||0}ns`;
            if (nanos < 1000000) return `${(nanos / 1000).toFixed(1)}µs`;
            if (nanos < 1000000000) return `${(nanos / 1000000).toFixed(1)}ms`;
            return `${(nanos / 1000000000).toFixed(2)}s`;
        }

        function refreshServices() { loadServices(); }

        loadServices();
        setInterval(loadServices, 5000);
    </script>
    "#.to_string()
}

/// Configuration management page content
pub fn config_content() -> String {
    r#"
    <style>
        /* COMPACT PROFESSIONAL CONFIG UI */
        .config-toolbar {
            display: flex;
            gap: 8px;
            margin-bottom: 12px;
            flex-wrap: wrap;
            align-items: center;
        }
        
        .search-box {
            flex: 1;
            min-width: 200px;
            position: relative;
        }
        
        .search-box input {
            width: 100%;
            padding: 6px 12px 6px 32px;
            background: rgba(20, 20, 35, 0.8);
            border: 1px solid rgba(255, 255, 255, 0.15);
            border-radius: 6px;
            color: white;
            font-size: 13px;
        }
        
        .search-box::before {
            content: '🔍';
            position: absolute;
            left: 10px;
            top: 50%;
            transform: translateY(-50%);
            opacity: 0.6;
        }
        
        .toolbar-btn {
            padding: 6px 14px;
            background: rgba(30, 30, 50, 0.8);
            border: 1px solid rgba(255, 255, 255, 0.15);
            border-radius: 6px;
            color: white;
            font-size: 12px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.15s;
        }
        
        .toolbar-btn:hover {
            background: rgba(40, 40, 60, 0.9);
            border-color: rgba(255, 255, 255, 0.3);
        }
        
        .toolbar-btn.primary { background: linear-gradient(135deg, #1565c0 0%, #0f4c81 100%); border: none; }
        .toolbar-btn.success { background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%); border: none; }
        .toolbar-btn.danger { background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%); border: none; }
        
        .filter-chips {
            display: flex;
            gap: 6px;
        }
        
        .chip {
            padding: 4px 10px;
            background: rgba(20, 20, 35, 0.6);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 12px;
            font-size: 11px;
            cursor: pointer;
            transition: all 0.15s;
        }
        
        .chip.active {
            background: rgba(21, 101, 192, 0.3);
            border-color: #1565c0;
        }
        
        .config-card {
            background: rgba(30, 30, 50, 0.6);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 8px;
            margin-bottom: 12px;
            overflow: hidden;
        }
        
        .card-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 10px 14px;
            background: rgba(20, 20, 35, 0.4);
            border-bottom: 1px solid rgba(255, 255, 255, 0.08);
            cursor: pointer;
            user-select: none;
        }
        
        .card-header h3 {
            margin: 0;
            font-size: 15px;
            font-weight: 600;
            display: flex;
            align-items: center;
            gap: 8px;
        }
        
        .field-count {
            font-size: 11px;
            opacity: 0.6;
            font-weight: 400;
        }
        
        .expand-icon {
            font-size: 12px;
            transition: transform 0.2s;
        }
        
        .expand-icon.expanded {
            transform: rotate(180deg);
        }
        
        .card-body {
            display: none;
            padding: 8px;
        }
        
        .card-body.expanded {
            display: block;
        }
        
        .category {
            background: rgba(20, 20, 35, 0.3);
            border: 1px solid rgba(255, 255, 255, 0.05);
            border-radius: 6px;
            margin-bottom: 8px;
        }
        
        .category-header {
            padding: 8px 12px;
            font-size: 13px;
            font-weight: 600;
            cursor: pointer;
            user-select: none;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        .category-header:hover {
            background: rgba(255, 255, 255, 0.02);
        }
        
        .category-body {
            /* Vertical stacked layout for all fields */
            display: flex;
            flex-direction: column;
            gap: 8px;
            padding: 8px;
        }
        
        .category-body.collapsed {
            display: none;
        }
        
        .field {
            background: rgba(15, 15, 25, 0.6);
            border: 1px solid rgba(255, 255, 255, 0.08);
            border-radius: 5px;
            padding: 10px;
            transition: border-color 0.15s ease, background 0.15s ease;
        }

        .field:hover {
            background: rgba(20, 20, 35, 0.7);
            border-color: rgba(255, 255, 255, 0.14);
        }
        
        .field-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 5px;
        }
        
        .field-header label {
            font-size: 12px;
            font-weight: 500;
            color: #ddd;
        }
        
        .impact {
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 9px;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }
        
        .impact.critical { background: #f5576c; color: white; }
        .impact.high { background: #ffaa00; color: white; }
        .impact.medium { background: #4facfe; color: white; }
        .impact.low { background: #888; color: white; }
        
        .field input,
        .field select,
        .field textarea {
            width: 100%;
            padding: 5px 8px;
            background: rgba(10, 10, 20, 0.8);
            border: 1px solid rgba(255, 255, 255, 0.12);
            border-radius: 4px;
            color: white;
            font-size: 13px;
            font-family: monospace;
        }

        .field textarea {
            min-height: 64px;
        }
        
        .field input:focus,
        .field select:focus,
        .field textarea:focus {
            outline: none;
            border-color: #1565c0;
            background: rgba(15, 15, 25, 0.9);
        }
        
        .field input.valid {
            border-color: #38ef7d;
        }
        
        .field input.invalid {
            border-color: #ef4444;
            background: rgba(239, 68, 68, 0.08);
        }
        
        .field-hint {
            font-size: 10px;
            color: #888;
            margin-top: 3px;
            line-height: 1.3;
        }
        
        .field-error {
            display: none;
            font-size: 10px;
            color: #f5576c;
            margin-top: 3px;
            font-weight: 500;
        }
        
        .field-error.visible {
            display: block;
        }
        
        .card-actions {
            display: flex;
            gap: 8px;
            padding: 8px;
            border-top: 1px solid rgba(255, 255, 255, 0.08);
        }
        
        .save-btn {
            flex: 1;
            padding: 8px 16px;
            background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);
            border: none;
            border-radius: 5px;
            color: white;
            font-size: 12px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.15s;
        }
        
        .save-btn:hover:not(:disabled) {
            transform: translateY(-1px);
            box-shadow: 0 3px 10px rgba(56, 239, 125, 0.3);
        }
        
        .save-btn:disabled {
            opacity: 0.6;
            cursor: not-allowed;
        }
        
        .reset-btn {
            padding: 8px 16px;
            background: rgba(30, 30, 50, 0.8);
            border: 1px solid rgba(255, 255, 255, 0.15);
            border-radius: 5px;
            color: white;
            font-size: 12px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.15s;
        }
        
        .reset-btn:hover {
            background: rgba(40, 40, 60, 0.9);
        }
        
        .spinner {
            display: inline-block;
            width: 12px;
            height: 12px;
            border: 2px solid rgba(255,255,255,0.3);
            border-top-color: white;
            border-radius: 50%;
            animation: spin 0.6s linear infinite;
        }
        
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
        
        .status {
            padding: 6px 10px;
            border-radius: 5px;
            font-size: 11px;
            margin-top: 8px;
            display: none;
        }
        
        .status.success {
            background: rgba(56, 239, 125, 0.15);
            border: 1px solid rgba(56, 239, 125, 0.4);
            color: #38ef7d;
        }
        
        .status.error {
            background: rgba(245, 87, 108, 0.15);
            border: 1px solid rgba(245, 87, 108, 0.4);
            color: #f5576c;
        }
        
        .status.visible {
            display: block;
        }
        
        /* Modal styles */
        .modal {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(0, 0, 0, 0.85);
            align-items: center;
            justify-content: center;
            z-index: 10000;
        }
        
        .modal.visible {
            display: flex;
        }
        
        .modal-content {
            background: #1a1a2e;
            border-radius: 10px;
            max-width: 90%;
            max-height: 85%;
            overflow: hidden;
            display: flex;
            flex-direction: column;
            box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
        }
        
        .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 12px 16px;
            background: rgba(30, 30, 50, 0.8);
            border-bottom: 1px solid rgba(255, 255, 255, 0.1);
        }
        
        .modal-header h3 {
            margin: 0;
            font-size: 16px;
        }
        
        .modal-close {
            background: none;
            border: none;
            color: white;
            font-size: 24px;
            cursor: pointer;
            padding: 0;
            width: 30px;
            height: 30px;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        
        .modal-body {
            padding: 16px;
            overflow-y: auto;
        }
        
        .diff-table {
            width: 100%;
            border-collapse: collapse;
            font-size: 12px;
        }
        
        .diff-table th {
            background: rgba(255, 255, 255, 0.05);
            padding: 8px;
            text-align: left;
            font-weight: 600;
            font-size: 11px;
            border-bottom: 1px solid rgba(255, 255, 255, 0.1);
        }
        
        .diff-table td {
            padding: 6px 8px;
            border-bottom: 1px solid rgba(255, 255, 255, 0.05);
            font-family: monospace;
        }
        
        .diff-table .new-value {
            background: rgba(56, 239, 125, 0.1);
            color: #38ef7d;
        }
        
        .diff-table .old-value {
            background: rgba(245, 87, 108, 0.1);
            color: #f5576c;
        }
    </style>

    <!-- DYNAMIC CONFIG UI -->
    <div class="config-toolbar">
        <div class="search-box">
            <input type="text" id="searchInput" placeholder="Search configurations..." oninput="filterConfigs(this.value)">
        </div>
        <button class="toolbar-btn success" onclick="exportConfig()">📥 Export</button>
        <button class="toolbar-btn primary" onclick="document.getElementById('importFile').click()">📤 Import</button>
        <input type="file" id="importFile" style="display:none" accept=".json" onchange="importConfig(this.files[0])">
        <button class="toolbar-btn primary" onclick="reloadConfig()">🔄 Reload</button>
        <button class="toolbar-btn danger" onclick="resetConfig()">⚠️ Reset</button>
        <button class="toolbar-btn" onclick="viewDiff()">📋 Diff</button>
    </div>
    
    <div class="filter-chips">
        <div class="chip active" onclick="filterByImpact('all')">All</div>
        <div class="chip" onclick="filterByImpact('critical')">Critical</div>
        <div class="chip" onclick="filterByImpact('high')">High</div>
        <div class="chip" onclick="filterByImpact('medium')">Medium</div>
    </div>
    
    <div id="globalStatus" class="status"></div>
    <div id="configContainer"></div>
    
    <!-- Diff Modal -->
    <div id="diffModal" class="modal">
        <div class="modal-content">
            <div class="modal-header">
                <h3>Configuration Differences</h3>
                <button class="modal-close" onclick="closeDiffModal()">×</button>
            </div>
            <div class="modal-body" id="diffModalBody">
                <p style="text-align:center;opacity:0.6;">Loading...</p>
            </div>
        </div>
    </div>

    <script>
        // COMPLETE DYNAMIC CONFIG UI IMPLEMENTATION - ALL 155 FIELDS
        // =============================================================================
// CONFIGURATION METADATA - Single source of truth for all 155 fields
// =============================================================================
const CONFIG_METADATA = {
    trader: {
        max_open_positions: { type: 'number', label: 'Max Open Positions', hint: 'Max simultaneous positions (2-5 conservative)', min: 1, max: 100, unit: 'positions', impact: 'critical', category: 'Core Trading' },
        trade_size_sol: { type: 'number', label: 'Trade Size', hint: 'SOL per position (0.005-0.01 for testing)', min: 0.001, max: 10, step: 0.001, unit: 'SOL', impact: 'critical', category: 'Core Trading' },
        
        min_profit_threshold_enabled: { type: 'boolean', label: 'Enable Profit Threshold', hint: 'Require minimum profit before exit', impact: 'high', category: 'Profit Management' },
        min_profit_threshold_percent: { type: 'number', label: 'Min Profit %', hint: '2-5% typical for volatile tokens', min: 0, max: 100, step: 0.1, unit: '%', impact: 'high', category: 'Profit Management' },
        profit_extra_needed_sol: { type: 'number', label: 'Profit Extra Buffer', hint: 'Extra SOL for fees/slippage', min: 0, max: 0.01, step: 0.00001, unit: 'SOL', impact: 'medium', category: 'Profit Management' },
        
        time_override_duration_hours: { type: 'number', label: 'Time Override Duration', hint: 'Hours before forced exit (168=1 week)', min: 1, max: 720, step: 1, unit: 'hours', impact: 'medium', category: 'Time Overrides' },
        time_override_loss_threshold_percent: { type: 'number', label: 'Time Override Loss %', hint: 'Loss % to trigger time override (-40 = exit if down 40%)', min: -100, max: 0, step: 1, unit: '%', impact: 'medium', category: 'Time Overrides' },
        
        slippage_quote_default_pct: { type: 'number', label: 'Default Slippage', hint: '3% balanced, higher = more fills but worse price', min: 0.1, max: 25, step: 0.1, unit: '%', impact: 'high', category: 'Slippage' },
        slippage_exit_profit_shortfall_pct: { type: 'number', label: 'Profit Exit Slippage', hint: 'Extra slippage when exiting at profit', min: 0, max: 50, step: 1, unit: '%', impact: 'high', category: 'Slippage' },
        slippage_exit_loss_shortfall_pct: { type: 'number', label: 'Loss Exit Slippage', hint: 'Even higher to exit bad positions', min: 0, max: 50, step: 1, unit: '%', impact: 'high', category: 'Slippage' },
        slippage_exit_retry_steps_pct: { type: 'array', label: 'Exit Retry Steps', hint: 'Comma-separated slippage values for retries', unit: '%', impact: 'medium', category: 'Slippage' },
        
        debug_force_sell_mode: { type: 'boolean', label: 'Force Sell Mode', hint: '⚠️ DEBUG ONLY - Do not use in production!', impact: 'critical', category: 'Debug' },
        debug_force_sell_timeout_secs: { type: 'number', label: 'Force Sell Timeout', hint: 'Seconds before force sell (only if enabled)', min: 10, max: 300, step: 5, unit: 'seconds', impact: 'high', category: 'Debug' },
        debug_force_buy_mode: { type: 'boolean', label: 'Force Buy Mode', hint: '⚠️ DEBUG ONLY - Do not use in production!', impact: 'critical', category: 'Debug' },
        debug_force_buy_drop_threshold_percent: { type: 'number', label: 'Force Buy Drop %', hint: 'Price drop % to trigger buy (only if enabled)', min: 0, max: 50, step: 0.1, unit: '%', impact: 'high', category: 'Debug' },
        
        position_close_cooldown_minutes: { type: 'number', label: 'Close Cooldown', hint: 'Minutes before reopening same token', min: 0, max: 1440, step: 5, unit: 'minutes', impact: 'medium', category: 'Timing' },
        entry_monitor_interval_secs: { type: 'number', label: 'Entry Monitor Interval', hint: 'Seconds between entry scans (lower = more responsive)', min: 1, max: 60, step: 1, unit: 'seconds', impact: 'medium', category: 'Timing' },
        position_monitor_interval_secs: { type: 'number', label: 'Position Monitor Interval', hint: 'Seconds between position checks', min: 1, max: 60, step: 1, unit: 'seconds', impact: 'medium', category: 'Timing' },
        
        semaphore_acquire_timeout_secs: { type: 'number', label: 'Semaphore Timeout', hint: 'Prevents deadlocks in concurrent ops', min: 5, max: 300, step: 5, unit: 'seconds', impact: 'low', category: 'Timeouts' },
        token_check_task_timeout_secs: { type: 'number', label: 'Token Check Timeout', hint: 'Timeout for individual token validation', min: 5, max: 120, step: 5, unit: 'seconds', impact: 'low', category: 'Timeouts' },
        token_check_collection_timeout_secs: { type: 'number', label: 'Token Collection Timeout', hint: 'Collecting all token checks', min: 10, max: 180, step: 5, unit: 'seconds', impact: 'low', category: 'Timeouts' },
        token_check_handle_timeout_secs: { type: 'number', label: 'Token Handle Timeout', hint: 'Handle timeout for token tasks', min: 10, max: 150, step: 5, unit: 'seconds', impact: 'low', category: 'Timeouts' },
        sell_operations_collection_timeout_secs: { type: 'number', label: 'Sell Collection Timeout', hint: 'Overall limit for batch sells', min: 30, max: 600, step: 10, unit: 'seconds', impact: 'low', category: 'Timeouts' },
        sell_operation_smart_timeout_secs: { type: 'number', label: 'Smart Sell Timeout', hint: 'Intelligent timeout adapting to network', min: 60, max: 1200, step: 30, unit: 'seconds', impact: 'low', category: 'Timeouts' },
        sell_semaphore_acquire_timeout_secs: { type: 'number', label: 'Sell Semaphore Timeout', hint: 'Sell operation lock timeout', min: 10, max: 300, step: 5, unit: 'seconds', impact: 'low', category: 'Timeouts' },
        sell_task_handle_timeout_secs: { type: 'number', label: 'Sell Task Handle Timeout', hint: 'Handle timeout for sell tasks', min: 60, max: 600, step: 10, unit: 'seconds', impact: 'low', category: 'Timeouts' },
        entry_cycle_min_wait_ms: { type: 'number', label: 'Entry Cycle Min Wait', hint: 'Rate limiting for entry checks', min: 10, max: 5000, step: 10, unit: 'ms', impact: 'low', category: 'Timeouts' },
        token_processing_shutdown_check_ms: { type: 'number', label: 'Token Shutdown Check', hint: 'Milliseconds between shutdown checks', min: 10, max: 1000, step: 10, unit: 'ms', impact: 'low', category: 'Timeouts' },
        task_shutdown_check_ms: { type: 'number', label: 'Task Shutdown Check', hint: 'Milliseconds between task shutdown checks', min: 10, max: 1000, step: 10, unit: 'ms', impact: 'low', category: 'Timeouts' },
        sell_operation_shutdown_check_ms: { type: 'number', label: 'Sell Shutdown Check', hint: 'Milliseconds between sell shutdown checks', min: 10, max: 1000, step: 10, unit: 'ms', impact: 'low', category: 'Timeouts' },
        collection_shutdown_check_ms: { type: 'number', label: 'Collection Shutdown Check', hint: 'Milliseconds between collection shutdown checks', min: 10, max: 1000, step: 10, unit: 'ms', impact: 'low', category: 'Timeouts' },
        entry_check_concurrency: { type: 'number', label: 'Entry Check Concurrency', hint: 'Tokens to check concurrently (higher = faster but more CPU)', min: 1, max: 50, step: 1, unit: 'concurrent', impact: 'medium', category: 'Performance' }
    },
    
    positions: {
        position_open_cooldown_secs: { type: 'number', label: 'Open Cooldown', hint: 'Seconds between opening positions', min: 0, max: 300, step: 1, unit: 'seconds', impact: 'medium', category: 'Timing' },
        pending_open_ttl_secs: { type: 'number', label: 'Pending Open TTL', hint: 'Time to live for pending opens (consider failed after this)', min: 30, max: 600, step: 10, unit: 'seconds', impact: 'medium', category: 'Timing' },
        profit_extra_needed_sol: { type: 'number', label: 'Profit Extra Buffer', hint: 'Extra SOL needed for profit calculations (priority fees)', min: 0, max: 0.01, step: 0.0001, unit: 'SOL', impact: 'high', category: 'Profit' }
    },
    
    filtering: {
        filter_cache_ttl_secs: { type: 'number', label: 'Cache TTL', hint: 'How long to cache filter results (lower = more current)', min: 5, max: 300, step: 5, unit: 'seconds', impact: 'medium', category: 'Performance' },
        target_filtered_tokens: { type: 'number', label: 'Target Filtered Tokens', hint: 'Bot processes up to this many qualified tokens', min: 10, max: 10000, step: 100, unit: 'tokens', impact: 'medium', category: 'Performance' },
        max_tokens_to_process: { type: 'number', label: 'Max Tokens to Process', hint: 'Max tokens to evaluate before filtering', min: 100, max: 50000, step: 500, unit: 'tokens', impact: 'medium', category: 'Performance' },
        
        require_name_and_symbol: { type: 'boolean', label: 'Require Name & Symbol', hint: 'Recommended: true. Filters incomplete tokens', impact: 'high', category: 'Requirements' },
        require_logo_url: { type: 'boolean', label: 'Require Logo', hint: 'Optional. Logo may indicate legitimacy', impact: 'medium', category: 'Requirements' },
        require_website_url: { type: 'boolean', label: 'Require Website', hint: 'Optional. Website may indicate serious project', impact: 'medium', category: 'Requirements' },
        
        min_token_age_minutes: { type: 'number', label: 'Min Token Age', hint: '60min avoids brand new tokens, lower for sniping', min: 0, max: 10080, step: 10, unit: 'minutes', impact: 'high', category: 'Age' },
        
        min_transactions_5min: { type: 'number', label: 'Min TX (5min)', hint: 'Min transactions in last 5 minutes (1+ is minimal)', min: 0, max: 1000, step: 1, unit: 'txs', impact: 'medium', category: 'Activity' },
        min_transactions_1h: { type: 'number', label: 'Min TX (1h)', hint: 'Min transactions in last hour (sustained activity)', min: 0, max: 10000, step: 5, unit: 'txs', impact: 'medium', category: 'Activity' },
        
        min_liquidity_usd: { type: 'number', label: 'Min Liquidity', hint: '$1 very low, $1000+ for serious trading', min: 0, max: 10000000, step: 10, unit: 'USD', impact: 'critical', category: 'Liquidity' },
        max_liquidity_usd: { type: 'number', label: 'Max Liquidity', hint: 'High max to avoid filtering established tokens', min: 100, max: 1000000000, step: 100000, unit: 'USD', impact: 'medium', category: 'Liquidity' },
        
        min_market_cap_usd: { type: 'number', label: 'Min Market Cap', hint: '$1000 filters micro-cap tokens', min: 0, max: 10000000, step: 100, unit: 'USD', impact: 'high', category: 'Market Cap' },
        max_market_cap_usd: { type: 'number', label: 'Max Market Cap', hint: 'Filters out large-cap tokens', min: 1000, max: 1000000000, step: 100000, unit: 'USD', impact: 'high', category: 'Market Cap' },
        
        min_security_score: { type: 'number', label: 'Min Security Score', hint: '10+ decent, 50+ safer (rugcheck score)', min: 0, max: 100, step: 5, unit: 'score', impact: 'critical', category: 'Security' },
        max_top_holder_pct: { type: 'number', label: 'Max Top Holder %', hint: '15% means top holder can own max 15% supply', min: 0, max: 100, step: 1, unit: '%', impact: 'critical', category: 'Security' },
        max_top_3_holders_pct: { type: 'number', label: 'Max Top 3 Holders %', hint: 'Combined max for top 3 holders (lower = more distributed)', min: 0, max: 100, step: 1, unit: '%', impact: 'high', category: 'Security' },
        min_pumpfun_lp_lock_pct: { type: 'number', label: 'Min PumpFun LP Lock', hint: '50%+ reduces rug risk for PumpFun tokens', min: 0, max: 100, step: 5, unit: '%', impact: 'high', category: 'Security' },
        min_regular_lp_lock_pct: { type: 'number', label: 'Min Regular LP Lock', hint: '50%+ indicates locked liquidity for regular tokens', min: 0, max: 100, step: 5, unit: '%', impact: 'high', category: 'Security' },
        min_unique_holders: { type: 'number', label: 'Min Unique Holders', hint: '500+ indicates community adoption', min: 0, max: 1000000, step: 50, unit: 'holders', impact: 'medium', category: 'Community' }
    },
    
    swaps: {
        gmgn_enabled: { type: 'boolean', label: 'GMGN Router', hint: 'GMGN provides MEV protection', impact: 'high', category: 'Routers' },
        jupiter_enabled: { type: 'boolean', label: 'Jupiter Router', hint: 'Jupiter finds best routes across DEXes', impact: 'high', category: 'Routers' },
        raydium_enabled: { type: 'boolean', label: 'Raydium Direct', hint: 'Direct Raydium swaps (bypass aggregators)', impact: 'medium', category: 'Routers' },
        
        quote_timeout_secs: { type: 'number', label: 'Quote Timeout', hint: 'How long to wait for price quotes', min: 5, max: 60, step: 1, unit: 'seconds', impact: 'medium', category: 'Timeouts' },
        api_timeout_secs: { type: 'number', label: 'API Timeout', hint: 'Overall timeout for API calls', min: 10, max: 120, step: 5, unit: 'seconds', impact: 'medium', category: 'Timeouts' },
        retry_attempts: { type: 'number', label: 'Retry Attempts', hint: '3 is balanced, more can be excessive', min: 0, max: 10, step: 1, unit: 'attempts', impact: 'medium', category: 'Retry' },
        
        transaction_confirmation_timeout_secs: { type: 'number', label: 'TX Confirmation Timeout', hint: '300s = 5 min, congestion may need more', min: 60, max: 600, step: 30, unit: 'seconds', impact: 'high', category: 'Confirmation' },
        priority_confirmation_timeout_secs: { type: 'number', label: 'Priority Confirm Timeout', hint: 'Timeout for priority confirmation', min: 10, max: 300, step: 5, unit: 'seconds', impact: 'medium', category: 'Confirmation' },
        transaction_confirmation_max_attempts: { type: 'number', label: 'TX Confirm Max Attempts', hint: 'Max attempts to confirm transaction', min: 5, max: 100, step: 5, unit: 'attempts', impact: 'medium', category: 'Confirmation' },
        priority_confirmation_max_attempts: { type: 'number', label: 'Priority Confirm Attempts', hint: 'Max attempts for priority confirmation', min: 5, max: 50, step: 5, unit: 'attempts', impact: 'medium', category: 'Confirmation' },
        transaction_confirmation_retry_delay_ms: { type: 'number', label: 'TX Confirm Retry Delay', hint: 'Milliseconds between confirmation retries', min: 1000, max: 10000, step: 500, unit: 'ms', impact: 'low', category: 'Confirmation' },
        priority_confirmation_retry_delay_ms: { type: 'number', label: 'Priority Retry Delay', hint: 'Milliseconds between priority retries', min: 500, max: 5000, step: 500, unit: 'ms', impact: 'low', category: 'Confirmation' },
        fast_failure_threshold_attempts: { type: 'number', label: 'Fast Failure Threshold', hint: 'Attempts before fast failure', min: 1, max: 20, step: 1, unit: 'attempts', impact: 'low', category: 'Confirmation' },
        
        initial_confirmation_delay_ms: { type: 'number', label: 'Initial Confirm Delay', hint: 'Initial delay before first confirmation check', min: 1000, max: 10000, step: 500, unit: 'ms', impact: 'low', category: 'Delays' },
        max_confirmation_delay_secs: { type: 'number', label: 'Max Confirm Delay', hint: 'Maximum confirmation delay', min: 1, max: 60, step: 1, unit: 'seconds', impact: 'low', category: 'Delays' },
        confirmation_backoff_multiplier: { type: 'number', label: 'Confirm Backoff Multiplier', hint: 'Backoff multiplier for retries', min: 1.0, max: 5.0, step: 0.1, unit: 'x', impact: 'low', category: 'Delays' },
        confirmation_timeout_secs: { type: 'number', label: 'Confirmation Timeout', hint: 'Overall confirmation timeout', min: 10, max: 300, step: 10, unit: 'seconds', impact: 'medium', category: 'Delays' },
        priority_confirmation_timeout_secs_mod: { type: 'number', label: 'Priority Timeout Modifier', hint: 'Modifier for priority confirmation timeout', min: 1, max: 30, step: 1, unit: 'seconds', impact: 'low', category: 'Delays' },
        
        rate_limit_base_delay_secs: { type: 'number', label: 'Rate Limit Base Delay', hint: 'Base delay for rate limiting', min: 1, max: 60, step: 1, unit: 'seconds', impact: 'low', category: 'Rate Limit' },
        rate_limit_increment_secs: { type: 'number', label: 'Rate Limit Increment', hint: 'Increment for each rate limit hit', min: 1, max: 30, step: 1, unit: 'seconds', impact: 'low', category: 'Rate Limit' },
        
        early_attempt_delay_ms: { type: 'number', label: 'Early Attempt Delay', hint: 'Delay for early attempts', min: 500, max: 5000, step: 500, unit: 'ms', impact: 'low', category: 'Delays' },
        early_attempts_count: { type: 'number', label: 'Early Attempts Count', hint: 'Number of early attempts', min: 1, max: 10, step: 1, unit: 'attempts', impact: 'low', category: 'Delays' },
        
        gmgn_quote_api: { type: 'string', label: 'GMGN Quote API', hint: 'GMGN API endpoint for quotes', impact: 'low', category: 'GMGN' },
        gmgn_partner: { type: 'string', label: 'GMGN Partner', hint: 'Partner identifier for GMGN', impact: 'low', category: 'GMGN' },
        gmgn_anti_mev: { type: 'boolean', label: 'GMGN Anti-MEV', hint: 'Enable GMGN MEV protection', impact: 'medium', category: 'GMGN' },
        gmgn_fee_sol: { type: 'number', label: 'GMGN Fee', hint: 'Usually 0, check GMGN docs', min: 0, max: 0.1, step: 0.001, unit: 'SOL', impact: 'low', category: 'GMGN' },
        gmgn_default_swap_mode: { type: 'string', label: 'GMGN Swap Mode', hint: 'ExactIn or ExactOut', impact: 'low', category: 'GMGN' },
        
        jupiter_quote_api: { type: 'string', label: 'Jupiter Quote API', hint: 'Jupiter API endpoint for quotes', impact: 'low', category: 'Jupiter' },
        jupiter_swap_api: { type: 'string', label: 'Jupiter Swap API', hint: 'Jupiter API endpoint for swaps', impact: 'low', category: 'Jupiter' },
        jupiter_dynamic_compute_unit_limit: { type: 'boolean', label: 'Jupiter Dynamic CU Limit', hint: 'Let Jupiter calculate compute units', impact: 'medium', category: 'Jupiter' },
        jupiter_default_priority_fee: { type: 'number', label: 'Jupiter Priority Fee', hint: '1000 lamports = 0.000001 SOL, higher = faster', min: 0, max: 1000000, step: 100, unit: 'lamports', impact: 'medium', category: 'Jupiter' },
        jupiter_default_swap_mode: { type: 'string', label: 'Jupiter Swap Mode', hint: 'ExactIn or ExactOut', impact: 'low', category: 'Jupiter' },
        
        slippage_quote_default_pct: { type: 'number', label: 'Default Slippage', hint: '1% tight, 3-5% for volatile', min: 0.1, max: 25, step: 0.1, unit: '%', impact: 'high', category: 'Slippage' },
        slippage_exit_profit_shortfall_pct: { type: 'number', label: 'Profit Exit Slippage', hint: 'Higher ensures exits succeed', min: 0, max: 50, step: 1, unit: '%', impact: 'high', category: 'Slippage' },
        slippage_exit_loss_shortfall_pct: { type: 'number', label: 'Loss Exit Slippage', hint: 'Even higher to exit bad positions', min: 0, max: 50, step: 1, unit: '%', impact: 'high', category: 'Slippage' },
        slippage_exit_retry_steps_pct: { type: 'array', label: 'Exit Retry Steps', hint: 'Comma-separated slippage for retries', unit: '%', impact: 'medium', category: 'Slippage' }
    },
    
    tokens: {
        dexscreener_rate_limit_per_minute: { type: 'number', label: 'DexScreener Rate Limit', hint: 'API calls per minute', min: 10, max: 300, step: 10, unit: 'calls/min', impact: 'medium', category: 'API Limits' },
        dexscreener_discovery_rate_limit: { type: 'number', label: 'DexScreener Discovery Limit', hint: 'Discovery API calls per minute', min: 10, max: 300, step: 10, unit: 'calls/min', impact: 'medium', category: 'API Limits' },
        max_tokens_per_api_call: { type: 'number', label: 'Max Tokens Per Call', hint: 'Tokens per API request', min: 10, max: 100, step: 10, unit: 'tokens', impact: 'low', category: 'API Limits' },
        raydium_rate_limit_per_minute: { type: 'number', label: 'Raydium Rate Limit', hint: 'Raydium API calls per minute', min: 10, max: 300, step: 10, unit: 'calls/min', impact: 'medium', category: 'API Limits' },
        geckoterminal_rate_limit_per_minute: { type: 'number', label: 'GeckoTerminal Rate Limit', hint: 'GeckoTerminal API calls per minute', min: 10, max: 120, step: 10, unit: 'calls/min', impact: 'medium', category: 'API Limits' },
        max_tokens_per_batch: { type: 'number', label: 'Max Tokens Per Batch', hint: 'Tokens per batch operation', min: 10, max: 100, step: 10, unit: 'tokens', impact: 'low', category: 'API Limits' },
        
        max_price_deviation_percent: { type: 'number', label: 'Max Price Deviation', hint: 'Max allowed price deviation for validation', min: 1, max: 100, step: 1, unit: '%', impact: 'high', category: 'Validation' },
        
        max_accounts_per_call: { type: 'number', label: 'Max Accounts Per RPC Call', hint: 'Accounts per get_multiple_accounts (max 100)', min: 10, max: 100, step: 10, unit: 'accounts', impact: 'medium', category: 'RPC' },
        max_decimal_retry_attempts: { type: 'number', label: 'Max Decimal Retry', hint: 'Retries for fetching token decimals', min: 1, max: 10, step: 1, unit: 'attempts', impact: 'low', category: 'RPC' },
        
        low_liquidity_threshold: { type: 'number', label: 'Low Liquidity Threshold', hint: 'USD threshold for low liquidity blacklist', min: 10, max: 10000, step: 10, unit: 'USD', impact: 'high', category: 'Blacklist' },
        min_age_hours: { type: 'number', label: 'Min Age for Blacklist', hint: 'Hours before token can be blacklisted', min: 0, max: 168, step: 1, unit: 'hours', impact: 'medium', category: 'Blacklist' },
        max_low_liquidity_count: { type: 'number', label: 'Max Low Liq Count', hint: 'Times seen with low liquidity before blacklist', min: 1, max: 20, step: 1, unit: 'times', impact: 'medium', category: 'Blacklist' },
        max_no_route_failures: { type: 'number', label: 'Max No Route Failures', hint: 'Route failures before blacklist', min: 1, max: 20, step: 1, unit: 'failures', impact: 'medium', category: 'Blacklist' },
        cache_refresh_interval_minutes: { type: 'number', label: 'Cache Refresh Interval', hint: 'Minutes between cache refreshes', min: 1, max: 60, step: 5, unit: 'minutes', impact: 'low', category: 'Blacklist' },
        
        max_ohlcv_age_hours: { type: 'number', label: 'Max OHLCV Age', hint: 'Hours to keep OHLCV data', min: 24, max: 720, step: 24, unit: 'hours', impact: 'low', category: 'OHLCV' },
        max_memory_cache_entries: { type: 'number', label: 'Max Memory Cache', hint: 'OHLCV entries in memory cache', min: 100, max: 5000, step: 100, unit: 'entries', impact: 'medium', category: 'OHLCV' },
        max_ohlcv_limit: { type: 'number', label: 'Max OHLCV Limit', hint: 'Max OHLCV candles to fetch', min: 100, max: 5000, step: 100, unit: 'candles', impact: 'low', category: 'OHLCV' },
        default_ohlcv_limit: { type: 'number', label: 'Default OHLCV Limit', hint: 'Default candles to fetch', min: 10, max: 1000, step: 10, unit: 'candles', impact: 'low', category: 'OHLCV' },
        
        max_update_interval_hours: { type: 'number', label: 'Max Update Interval', hint: 'Hours between token updates', min: 1, max: 24, step: 1, unit: 'hours', impact: 'medium', category: 'Monitoring' },
        new_token_boost_max_age_minutes: { type: 'number', label: 'New Token Boost Age', hint: 'Minutes to boost new tokens', min: 10, max: 240, step: 10, unit: 'minutes', impact: 'low', category: 'Monitoring' },
        
        max_pattern_length: { type: 'number', label: 'Max Pattern Length', hint: 'Max length for pattern detection', min: 3, max: 20, step: 1, unit: 'chars', impact: 'low', category: 'Patterns' }
    },
    
    rpc: {
        urls: { type: 'array', label: 'RPC URLs', hint: 'Comma-separated RPC endpoints (round-robin)', impact: 'critical', category: 'Endpoints' }
    },
    
    sol_price: {
        price_refresh_interval_secs: { type: 'number', label: 'Price Refresh Interval', hint: 'Seconds between SOL price updates', min: 10, max: 300, step: 10, unit: 'seconds', impact: 'medium', category: 'Timing' }
    },
    
    summary: {
        summary_display_interval_secs: { type: 'number', label: 'Display Interval', hint: 'Seconds between summary display updates', min: 5, max: 300, step: 5, unit: 'seconds', impact: 'low', category: 'Display' },
        max_recent_closed_positions: { type: 'number', label: 'Max Recent Closed', hint: 'Number of recent closed positions to display', min: 5, max: 100, step: 5, unit: 'positions', impact: 'low', category: 'Display' }
    },
    
    events: {
        batch_timeout_ms: { type: 'number', label: 'Batch Timeout', hint: 'Milliseconds for event batch timeout', min: 10, max: 1000, step: 10, unit: 'ms', impact: 'low', category: 'Performance' }
    }
};

const SECTION_INFO = {
    trader: { icon: '🤖', title: 'Trader', fields: 33 },
    positions: { icon: '💰', title: 'Positions', fields: 3 },
    filtering: { icon: '🔍', title: 'Filtering', fields: 17 },
    swaps: { icon: '🔄', title: 'Swaps', fields: 36 },
    tokens: { icon: '🪙', title: 'Tokens', fields: 19 },
    rpc: { icon: '🌐', title: 'RPC', fields: 1 },
    sol_price: { icon: '💲', title: 'SOL Price', fields: 1 },
    summary: { icon: '📊', title: 'Summary', fields: 2 },
    events: { icon: '📝', title: 'Events', fields: 1 }
};

const INTEGER_FIELDS = {
    trader: new Set([
        'max_open_positions',
        'position_close_cooldown_minutes',
        'entry_monitor_interval_secs',
        'position_monitor_interval_secs',
        'semaphore_acquire_timeout_secs',
        'token_check_task_timeout_secs',
        'token_check_collection_timeout_secs',
        'token_check_handle_timeout_secs',
        'sell_operations_collection_timeout_secs',
        'sell_operation_smart_timeout_secs',
        'sell_semaphore_acquire_timeout_secs',
        'sell_task_handle_timeout_secs',
        'entry_cycle_min_wait_ms',
        'token_processing_shutdown_check_ms',
        'task_shutdown_check_ms',
        'sell_operation_shutdown_check_ms',
        'collection_shutdown_check_ms',
        'entry_check_concurrency'
    ]),
    positions: new Set([
        'position_open_cooldown_secs',
        'pending_open_ttl_secs'
    ]),
    filtering: new Set([
        'filter_cache_ttl_secs',
        'target_filtered_tokens',
        'max_tokens_to_process',
        'min_token_age_minutes',
        'min_transactions_5min',
        'min_transactions_1h',
        'min_security_score',
        'min_unique_holders'
    ]),
    swaps: new Set([
        'quote_timeout_secs',
        'api_timeout_secs',
        'retry_attempts',
        'transaction_confirmation_timeout_secs',
        'priority_confirmation_timeout_secs',
        'transaction_confirmation_max_attempts',
        'priority_confirmation_max_attempts',
        'transaction_confirmation_retry_delay_ms',
        'priority_confirmation_retry_delay_ms',
        'fast_failure_threshold_attempts',
        'initial_confirmation_delay_ms',
        'max_confirmation_delay_secs',
        'confirmation_timeout_secs',
        'priority_confirmation_timeout_secs_mod',
        'rate_limit_base_delay_secs',
        'rate_limit_increment_secs',
        'early_attempt_delay_ms',
        'early_attempts_count',
        'jupiter_default_priority_fee'
    ]),
    tokens: new Set([
        'dexscreener_rate_limit_per_minute',
        'dexscreener_discovery_rate_limit',
        'max_tokens_per_api_call',
        'raydium_rate_limit_per_minute',
        'geckoterminal_rate_limit_per_minute',
        'max_tokens_per_batch',
        'max_accounts_per_call',
        'max_decimal_retry_attempts',
        'min_age_hours',
        'max_low_liquidity_count',
        'max_no_route_failures',
        'cache_refresh_interval_minutes',
        'max_ohlcv_age_hours',
        'max_memory_cache_entries',
        'max_ohlcv_limit',
        'default_ohlcv_limit',
        'max_update_interval_hours',
        'new_token_boost_max_age_minutes',
        'max_pattern_length'
    ]),
    sol_price: new Set(['price_refresh_interval_secs']),
    summary: new Set(['summary_display_interval_secs', 'max_recent_closed_positions']),
    events: new Set(['batch_timeout_ms'])
};

function isIntegerField(sectionName, fieldKey) {
    return INTEGER_FIELDS[sectionName]?.has(fieldKey) || false;
}

// =============================================================================
// DYNAMIC RENDERING ENGINE
// =============================================================================

function renderAllSections() {
    const container = document.getElementById('configContainer');
    container.innerHTML = '';
    
    for (const [sectionName, sectionInfo] of Object.entries(SECTION_INFO)) {
        container.innerHTML += renderSection(sectionName, sectionInfo);
    }
}

function renderSection(sectionName, sectionInfo) {
    const metadata = CONFIG_METADATA[sectionName];
    if (!metadata) return '';
    
    // Group fields by category
    const categories = {};
    for (const [fieldKey, fieldMeta] of Object.entries(metadata)) {
        const cat = fieldMeta.category || 'General';
        if (!categories[cat]) categories[cat] = [];
        categories[cat].push({ key: fieldKey, ...fieldMeta });
    }
    
    let html = `
        <div class="config-card" data-section="${sectionName}">
            <div class="card-header" onclick="toggleSection('${sectionName}')">
                <h3>
                    <span>${sectionInfo.icon}</span>
                    <span>${sectionInfo.title}</span>
                    <span class="field-count">(${sectionInfo.fields} fields)</span>
                </h3>
                <span class="expand-icon" id="${sectionName}-icon">▼</span>
            </div>
            <div class="card-body" id="${sectionName}-body">
    `;
    
    // Render categories
    for (const [catName, fields] of Object.entries(categories)) {
        html += renderCategory(sectionName, catName, fields);
    }
    
    // Card actions
    html += `
                <div class="card-actions">
                    <button class="save-btn" onclick="saveSection('${sectionName}')">💾 Save</button>
                    <button class="reset-btn" onclick="resetSection('${sectionName}')">🔙 Reset</button>
                </div>
                <div class="status" id="${sectionName}-status"></div>
            </div>
        </div>
    `;
    
    return html;
}

function renderCategory(sectionName, catName, fields) {
    const isAdvanced = catName.includes('Timeout') || catName.includes('Delay') || catName.includes('Advanced');
    const collapsedClass = isAdvanced ? ' collapsed' : '';
    
    let html = `
        <div class="category">
            <div class="category-header" onclick="toggleCategory(this)">
                ${catName} (${fields.length})
                <span class="category-toggle">▼</span>
            </div>
            <div class="category-body${collapsedClass}">
    `;
    
    fields.forEach(field => {
        html += renderField(sectionName, field);
    });
    
    html += `
            </div>
        </div>
    `;
    
    return html;
}

function renderField(sectionName, field) {
    const fieldId = `${sectionName}_${field.key}`;
    const requiresInteger = isIntegerField(sectionName, field.key);
    let inputHTML = '';
    
    if (field.type === 'boolean') {
        inputHTML = `
            <select id="${fieldId}" onchange="validateField(this)">
                <option value="true">Enabled</option>
                <option value="false">Disabled</option>
            </select>
        `;
    } else if (field.type === 'array') {
        inputHTML = `
            <textarea id="${fieldId}" rows="2" placeholder="Comma-separated values" oninput="validateField(this)"></textarea>
        `;
    } else if (field.type === 'string') {
        inputHTML = `
            <input type="text" id="${fieldId}" oninput="validateField(this)">
        `;
    } else { // number
        const integerAttr = requiresInteger ? ' data-integer="true"' : '';
        inputHTML = `
            <input type="number" id="${fieldId}"
                ${field.min !== undefined ? `min="${field.min}"` : ''}
                ${field.max !== undefined ? `max="${field.max}"` : ''}
                ${field.step ? `step="${field.step}"` : ''}
                oninput="validateField(this)"${integerAttr}>
        `;
    }
    
    return `
        <div class="field" data-impact="${field.impact}" data-field-key="${field.key}">
            <div class="field-header">
                <label>${field.label}${field.unit ? ` (${field.unit})` : ''}</label>
                <span class="impact ${field.impact}">${field.impact}</span>
            </div>
            ${inputHTML}
            <div class="field-hint">${field.hint}</div>
            <div class="field-error"></div>
        </div>
    `;
}

// =============================================================================
// LOAD & SAVE FUNCTIONS
// =============================================================================

async function loadAllConfigs() {
    for (const sectionName of Object.keys(SECTION_INFO)) {
        await loadSection(sectionName);
    }
}

async function loadSection(sectionName) {
    try {
        const response = await fetch(`/api/config/${sectionName}`);
        const result = await response.json();
        
        if (result.data) {
            // Populate fields
            for (const [key, value] of Object.entries(result.data)) {
                const fieldId = `${sectionName}_${key}`;
                const input = document.getElementById(fieldId);
                if (input) {
                    if (input.tagName === 'SELECT') {
                        input.value = value.toString();
                    } else if (input.tagName === 'TEXTAREA') {
                        // Handle arrays
                        input.value = Array.isArray(value) ? value.join(', ') : value;
                    } else {
                        input.value = value;
                    }
                    validateField(input);
                }
            }
        }
    } catch (error) {
        console.error(`Failed to load ${sectionName} config:`, error);
    }
}

async function saveSection(sectionName) {
    const btn = event.target;
    const statusDiv = document.getElementById(`${sectionName}-status`);
    
    // Validate first
    const body = document.getElementById(`${sectionName}-body`);
    const inputs = body.querySelectorAll('input, select, textarea');
    let hasErrors = false;
    
    inputs.forEach(input => {
        if (!validateField(input)) hasErrors = true;
    });
    
    if (hasErrors) {
        showStatus(statusDiv, 'error', 'Fix validation errors first');
        return;
    }
    
    // Loading state
    btn.disabled = true;
    btn.innerHTML = '<span class="spinner"></span> Saving...';
    
    try {
        // Collect data
        const updates = {};
        inputs.forEach(input => {
            const key = input.id.replace(`${sectionName}_`, '');
            let value = input.value;
            const requiresInteger = input.dataset.integer === 'true';
            
            if (input.type === 'number') {
                value = requiresInteger ? parseInt(value, 10) : parseFloat(value);
            } else if (input.tagName === 'SELECT') {
                value = value === 'true';
            } else if (input.tagName === 'TEXTAREA') {
                // Handle arrays
                value = value.split(',').map(v => v.trim()).filter(v => v);
                // Try to convert to numbers if possible
                const numValue = value.map(v => parseFloat(v));
                if (numValue.every(v => !isNaN(v))) {
                    value = numValue;
                }
            }
            
            updates[key] = value;
        });
        
        // Save
        const response = await fetch(`/api/config/${sectionName}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(updates)
        });
        
        const result = await response.json();
        
        if (response.ok) {
            showStatus(statusDiv, 'success', result.message || 'Saved successfully');
            btn.innerHTML = '✅ Saved';
            setTimeout(() => { btn.innerHTML = '💾 Save'; }, 2000);
        } else {
            throw new Error(result.error?.message || 'Save failed');
        }
    } catch (error) {
        showStatus(statusDiv, 'error', error.message);
        btn.innerHTML = '❌ Failed';
        setTimeout(() => { btn.innerHTML = '💾 Save'; }, 2000);
    } finally {
        btn.disabled = false;
    }
}

async function resetSection(sectionName) {
    if (!confirm(`Reset ${SECTION_INFO[sectionName].title} to last saved values?`)) return;
    await loadSection(sectionName);
    showStatus(document.getElementById(`${sectionName}-status`), 'success', 'Reset to saved values');
}

// =============================================================================
// VALIDATION
// =============================================================================

function validateField(input) {
    const [sectionName, ...keyParts] = input.id.split('_');
    const fieldKey = keyParts.join('_');
    const metadata = CONFIG_METADATA[sectionName]?.[fieldKey];
    
    if (!metadata) return true;
    
    const errorEl = input.parentElement.querySelector('.field-error');
    const errors = [];
    const requiresInteger = input.dataset.integer === 'true';
    
    if (metadata.type === 'number') {
        const value = parseFloat(input.value);
        if (isNaN(value)) {
            errors.push('Must be a number');
        } else {
            if (metadata.min !== undefined && value < metadata.min) {
                errors.push(`Min: ${metadata.min}`);
            }
            if (metadata.max !== undefined && value > metadata.max) {
                errors.push(`Max: ${metadata.max}`);
            }
            if (requiresInteger && !Number.isInteger(value)) {
                errors.push('Must be an integer');
            }
        }
    }
    
    if (errors.length > 0) {
        input.classList.add('invalid');
        input.classList.remove('valid');
        errorEl.textContent = errors.join(', ');
        errorEl.classList.add('visible');
        return false;
    } else {
        input.classList.remove('invalid');
        input.classList.add('valid');
        errorEl.classList.remove('visible');
        return true;
    }
}

// =============================================================================
// SEARCH & FILTER
// =============================================================================

let currentImpactFilter = 'all';

function filterConfigs(query) {
    query = query.toLowerCase();
    
    document.querySelectorAll('.field').forEach(field => {
        const label = field.querySelector('label').textContent.toLowerCase();
        const hint = field.querySelector('.field-hint').textContent.toLowerCase();
        const impactMatch = currentImpactFilter === 'all' || field.dataset.impact === currentImpactFilter;
        const textMatch = !query || label.includes(query) || hint.includes(query);
        
        field.style.display = (impactMatch && textMatch) ? 'block' : 'none';
    });
    
    // Hide empty categories
    document.querySelectorAll('.category').forEach(cat => {
        const visibleFields = cat.querySelectorAll('.field[style*="block"]').length;
        cat.style.display = visibleFields > 0 ? 'block' : 'none';
    });
    
    // Hide empty sections
    document.querySelectorAll('.config-card').forEach(card => {
        const visibleCategories = card.querySelectorAll('.category[style*="block"]').length;
        card.style.display = visibleCategories > 0 ? 'block' : 'none';
    });
}

function filterByImpact(level) {
    currentImpactFilter = level;
    
    // Update chip states
    document.querySelectorAll('.chip').forEach(chip => {
        chip.classList.remove('active');
    });
    event.target.classList.add('active');
    
    // Reapply filter
    const searchQuery = document.getElementById('searchInput').value;
    filterConfigs(searchQuery);
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

async function exportConfig() {
    try {
        const response = await fetch('/api/config');
        const config = await response.json();
        
        const blob = new Blob([JSON.stringify(config, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `screenerbot-config-${Date.now()}.json`;
        a.click();
        URL.revokeObjectURL(url);
    } catch (error) {
        showStatus(document.getElementById('globalStatus'), 'error', 'Export failed');
    }
}

async function importConfig(file) {
    if (!file) return;
    
    try {
        const text = await file.text();
        const config = JSON.parse(text);
        
        if (!confirm(`Import configuration from ${file.name}? This will update ALL settings.`)) {
            return;
        }

        const skippedKeys = ['timestamp'];
        
        // Update each section
        for (const [section, data] of Object.entries(config)) {
            if (skippedKeys.includes(section) || typeof data !== 'object' || data === null) {
                continue;
            }

            try {
                await fetch(`/api/config/${section}`, {
                    method: 'PATCH',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(data)
                });
            } catch (sectionError) {
                console.error(`Failed to import section ${section}:`, sectionError);
            }
        }
        
        showStatus(document.getElementById('globalStatus'), 'success', 'Config imported');
        await loadAllConfigs();
    } catch (error) {
        showStatus(document.getElementById('globalStatus'), 'error', `Import failed: ${error.message}`);
    }
}

async function reloadConfig() {
    if (!confirm('Reload configuration from disk? Unsaved changes will be lost.')) return;
    
    try {
        const response = await fetch('/api/config/reload', { method: 'POST' });
        const result = await response.json();
        
        if (response.ok) {
            showStatus(document.getElementById('globalStatus'), 'success', 'Reloaded from disk');
            await loadAllConfigs();
        } else {
            throw new Error(result.error?.message || 'Reload failed');
        }
    } catch (error) {
        showStatus(document.getElementById('globalStatus'), 'error', error.message);
    }
}

async function resetConfig() {
    if (!confirm('⚠️ Reset ALL configuration to defaults? This CANNOT be undone!')) return;
    
    try {
        const response = await fetch('/api/config/reset', { method: 'POST' });
        const result = await response.json();
        
        if (response.ok) {
            showStatus(document.getElementById('globalStatus'), 'success', 'Reset to defaults');
            await loadAllConfigs();
        } else {
            throw new Error(result.error?.message || 'Reset failed');
        }
    } catch (error) {
        showStatus(document.getElementById('globalStatus'), 'error', error.message);
    }
}

async function viewDiff() {
    const modal = document.getElementById('diffModal');
    const body = document.getElementById('diffModalBody');
    
    modal.classList.add('visible');
    body.innerHTML = '<p style="text-align:center;opacity:0.6;">Loading...</p>';
    
    try {
        const response = await fetch('/api/config/diff');
        const result = await response.json();

        if (!response.ok) {
            throw new Error(result.error?.message || 'Failed to fetch diff');
        }

        if (!result.has_changes) {
            body.innerHTML = '<p style="text-align:center;">No changes detected - memory and disk configs match</p>';
            return;
        }

        if (!result.memory || !result.disk) {
            body.innerHTML = '<p style="color:#f5576c;text-align:center;">Diff data unavailable</p>';
            return;
        }

        // Build diff table
        let diffRows = '';

        for (const [section, memData] of Object.entries(result.memory)) {
            if (!memData || typeof memData !== 'object') continue;
            const diskData = result.disk[section] || {};
            const fieldKeys = new Set([
                ...Object.keys(memData),
                ...Object.keys(diskData)
            ]);

            fieldKeys.forEach(key => {
                const memValue = memData[key];
                const diskValue = diskData[key];
                if (JSON.stringify(memValue) !== JSON.stringify(diskValue)) {
                    diffRows += `
                        <tr>
                            <td>${section}</td>
                            <td>${key}</td>
                            <td class="new-value">${JSON.stringify(memValue)}</td>
                            <td class="old-value">${JSON.stringify(diskValue)}</td>
                        </tr>
                    `;
                }
            });
        }

        if (!diffRows) {
            body.innerHTML = '<p style="text-align:center;">Changes detected but could not be itemized.</p>';
            return;
        }

        body.innerHTML = `
            <table class="diff-table">
                <thead>
                    <tr>
                        <th>Section</th>
                        <th>Field</th>
                        <th>Memory</th>
                        <th>Disk</th>
                    </tr>
                </thead>
                <tbody>
                    ${diffRows}
                </tbody>
            </table>
        `;
    } catch (error) {
        body.innerHTML = `<p style="color:#f5576c;text-align:center;">Error: ${error.message}</p>`;
    }
}

function closeDiffModal() {
    document.getElementById('diffModal').classList.remove('visible');
}

function toggleSection(sectionName) {
    const body = document.getElementById(`${sectionName}-body`);
    const icon = document.getElementById(`${sectionName}-icon`);
    
    body.classList.toggle('expanded');
    icon.classList.toggle('expanded');
    
    // Load when expanding
    if (body.classList.contains('expanded')) {
        loadSection(sectionName);
    }
}

function toggleCategory(header) {
    const body = header.nextElementSibling;
    const toggle = header.querySelector('.category-toggle');
    
    body.classList.toggle('collapsed');
    toggle.textContent = body.classList.contains('collapsed') ? '▶' : '▼';
}

function showStatus(element, type, message) {
    element.className = `status ${type} visible`;
    element.textContent = message;
    
    setTimeout(() => {
        element.classList.remove('visible');
    }, 5000);
}

// =============================================================================
// INITIALIZATION
// =============================================================================

window.addEventListener('DOMContentLoaded', () => {
    renderAllSections();
    
    // Auto-expand first section
    const firstSection = Object.keys(SECTION_INFO)[0];
    toggleSection(firstSection);
});

// Close modal when clicking outside
document.addEventListener('click', (e) => {
    if (e.target.id === 'diffModal') {
        closeDiffModal();
    }
});
    </script>
    "#.to_string()
}
