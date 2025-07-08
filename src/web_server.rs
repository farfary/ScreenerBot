#![allow(warnings)]
use crate::prelude::*;
use crate::persistence::{
    OPEN_POSITIONS,
    RECENT_CLOSED_POSITIONS,
    ALL_CLOSED_POSITIONS,
    TRADING_HISTORY,
    Position,
    TradingSnapshot,
};
use crate::dexscreener::{ TOKENS, Token };
use crate::pool_price::POOL_CACHE;

use axum::{
    extract::{ ws::WebSocket, WebSocketUpgrade, Path, Query },
    response::{ Html, Json, Response },
    routing::{ get, post },
    Router,
};
use futures::{ sink::SinkExt, stream::StreamExt };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionWithId {
    pub id: String,
    pub position: Position,
    pub current_price: Option<f64>,
    pub pnl: Option<f64>,
    pub pnl_percentage: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenWithAnalysis {
    pub token: Token,
    pub analysis: TokenAnalysis,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenAnalysis {
    pub score: f64,
    pub volume_trend: String,
    pub price_trend: String,
    pub liquidity_health: String,
    pub risk_level: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardData {
    pub open_positions: Vec<PositionWithId>,
    pub closed_positions: Vec<PositionWithId>,
    pub watched_tokens: Vec<TokenWithAnalysis>,
    pub total_pnl: f64,
    pub total_invested: f64,
    pub active_trades: usize,
    pub win_rate: f64,
}

// Broadcast channel for real-time updates
pub static DASHBOARD_UPDATES: tokio::sync::OnceCell<broadcast::Sender<DashboardData>> = tokio::sync::OnceCell::const_new();

pub async fn start_web_server() -> Result<()> {
    println!("🌐 Starting web server...");

    // Initialize broadcast channel
    let (tx, _rx) = broadcast::channel(100);
    let _ = DASHBOARD_UPDATES.set(tx.clone());

    // Create static file directory
    tokio::fs::create_dir_all("web/static").await?;
    create_html_files().await?;

    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/api/dashboard", get(get_dashboard_data))
        .route("/api/positions/open", get(get_open_positions))
        .route("/api/positions/closed", get(get_closed_positions))
        .route("/api/positions/all-closed", get(get_all_closed_positions))
        .route("/api/tokens", get(get_watched_tokens))
        .route("/api/trading-history", get(get_trading_history))
        .route("/api/ws", get(websocket_handler))
        .nest_service("/static", ServeDir::new("web/static"))
        .layer(CorsLayer::permissive());

    // Start background task to periodically update dashboard
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Ok(data) = collect_dashboard_data().await {
                let _ = tx_clone.send(data);
            }
        }
    });

    println!("🚀 Web dashboard available at: http://localhost:3000");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_dashboard() -> Html<String> {
    let html = tokio::fs
        ::read_to_string("web/static/index.html").await
        .unwrap_or_else(|_| create_default_html());
    Html(html)
}

async fn get_dashboard_data() -> Json<DashboardData> {
    let data = collect_dashboard_data().await.unwrap_or_else(|_| DashboardData {
        open_positions: vec![],
        closed_positions: vec![],
        watched_tokens: vec![],
        total_pnl: 0.0,
        total_invested: 0.0,
        active_trades: 0,
        win_rate: 0.0,
    });
    Json(data)
}

async fn get_open_positions() -> Json<Vec<PositionWithId>> {
    let positions = OPEN_POSITIONS.read().await;
    let mut result = Vec::new();

    for (id, position) in positions.iter() {
        let current_price = get_current_token_price(id).await;
        let (pnl, pnl_percentage) = calculate_pnl(position, current_price);

        result.push(PositionWithId {
            id: id.clone(),
            position: position.clone(),
            current_price,
            pnl,
            pnl_percentage,
        });
    }

    Json(result)
}

async fn get_closed_positions() -> Json<Vec<PositionWithId>> {
    let positions = RECENT_CLOSED_POSITIONS.read().await;
    let mut result = Vec::new();

    for (id, position) in positions.iter() {
        let (pnl, pnl_percentage) = calculate_pnl(position, None);

        result.push(PositionWithId {
            id: id.clone(),
            position: position.clone(),
            current_price: None,
            pnl,
            pnl_percentage,
        });
    }

    Json(result)
}

async fn get_all_closed_positions() -> Json<Vec<PositionWithId>> {
    let positions = ALL_CLOSED_POSITIONS.read().await;
    let mut result = Vec::new();

    for (id, position) in positions.iter() {
        let (pnl, pnl_percentage) = calculate_pnl(position, None);

        result.push(PositionWithId {
            id: id.clone(),
            position: position.clone(),
            current_price: None,
            pnl,
            pnl_percentage,
        });
    }

    // Sort by close time (most recent first)
    result.sort_by(|a, b| { b.position.close_time.cmp(&a.position.close_time) });

    Json(result)
}

async fn get_watched_tokens() -> Json<Vec<TokenWithAnalysis>> {
    let tokens = TOKENS.read().await;
    let mut result = Vec::new();

    for token in tokens.iter() {
        let analysis = analyze_token(token);
        result.push(TokenWithAnalysis {
            token: token.clone(),
            analysis,
        });
    }

    Json(result)
}

async fn get_trading_history() -> Json<Vec<TradingSnapshot>> {
    let history = TRADING_HISTORY.read().await;
    Json(history.clone())
}

async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_websocket)
}

async fn handle_websocket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = DASHBOARD_UPDATES.get().unwrap().subscribe();

    tokio::spawn(async move {
        while let Ok(data) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&data) {
                if sender.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages (if needed for interactive features)
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                axum::extract::ws::Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    }
}

async fn collect_dashboard_data() -> Result<DashboardData> {
    let open_positions = get_open_positions().await.0;
    let closed_positions = get_closed_positions().await.0;
    let watched_tokens = get_watched_tokens().await.0;

    // Calculate total PnL from ALL closed positions, not just recent ones
    let all_closed = ALL_CLOSED_POSITIONS.read().await;
    let closed_pnl: f64 = all_closed
        .values()
        .map(|p| p.sol_received - p.sol_spent)
        .sum::<f64>();

    let open_pnl: f64 = open_positions
        .iter()
        .filter_map(|p| p.pnl)
        .sum::<f64>();

    let total_pnl = closed_pnl + open_pnl;

    let total_invested: f64 =
        open_positions
            .iter()
            .map(|p| p.position.sol_spent)
            .sum::<f64>() +
        all_closed
            .values()
            .map(|p| p.sol_spent)
            .sum::<f64>();

    let active_trades = open_positions.len();

    let profitable_trades = all_closed
        .values()
        .filter(|p| p.sol_received > p.sol_spent)
        .count();

    let win_rate = if !all_closed.is_empty() {
        ((profitable_trades as f64) / (all_closed.len() as f64)) * 100.0
    } else {
        0.0
    };

    Ok(DashboardData {
        open_positions,
        closed_positions,
        watched_tokens,
        total_pnl,
        total_invested,
        active_trades,
        win_rate,
    })
}

async fn get_current_token_price(token_id: &str) -> Option<f64> {
    // Try to get price from existing token data
    let tokens = TOKENS.read().await;
    if let Some(token) = tokens.iter().find(|t| t.mint == token_id) {
        if let Ok(price) = token.price_usd.parse::<f64>() {
            return Some(price);
        }
    }

    // Fallback to pool price calculation
    match crate::pool_price::price_from_biggest_pool(&*crate::configs::RPC, token_id) {
        Ok(price) => Some(price),
        Err(_) => None,
    }
}

fn calculate_pnl(position: &Position, current_price: Option<f64>) -> (Option<f64>, Option<f64>) {
    if let Some(close_time) = position.close_time {
        // Closed position - use actual received amount
        let pnl = position.sol_received - position.sol_spent;
        let pnl_percentage = (pnl / position.sol_spent) * 100.0;
        (Some(pnl), Some(pnl_percentage))
    } else if let Some(price) = current_price {
        // Open position - calculate unrealized PnL
        let current_value = position.token_amount * price;
        let pnl = current_value - position.sol_spent;
        let pnl_percentage = (pnl / position.sol_spent) * 100.0;
        (Some(pnl), Some(pnl_percentage))
    } else {
        (None, None)
    }
}

fn analyze_token(token: &Token) -> TokenAnalysis {
    let volume_trend = if token.volume.h1 > token.volume.h6 / 6.0 {
        "Increasing".to_string()
    } else if token.volume.h1 < token.volume.h6 / 12.0 {
        "Decreasing".to_string()
    } else {
        "Stable".to_string()
    };

    let price_trend = if token.price_change.h1 > 5.0 {
        "Bullish".to_string()
    } else if token.price_change.h1 < -5.0 {
        "Bearish".to_string()
    } else {
        "Sideways".to_string()
    };

    let liquidity_health = if token.liquidity.usd > 50000.0 {
        "Good".to_string()
    } else if token.liquidity.usd > 10000.0 {
        "Moderate".to_string()
    } else {
        "Low".to_string()
    };

    let risk_level = calculate_risk_score(token);

    // Calculate overall score based on multiple factors
    let mut score: f64 = 50.0; // Base score

    // Volume factor
    if token.volume.h1 > 10000.0 {
        score += 10.0;
    }
    if token.volume.h24 > 100000.0 {
        score += 10.0;
    }

    // Price change factor
    if token.price_change.h1 > 0.0 && token.price_change.h1 < 50.0 {
        score += 15.0;
    }
    if token.price_change.h24 > 0.0 && token.price_change.h24 < 100.0 {
        score += 10.0;
    }

    // Liquidity factor
    if token.liquidity.usd > 50000.0 {
        score += 15.0;
    } else if token.liquidity.usd > 10000.0 {
        score += 5.0;
    }

    // Transaction activity
    if token.txns.h1.buys > token.txns.h1.sells {
        score += 10.0;
    }
    if token.txns.h1.buys + token.txns.h1.sells > 50 {
        score += 5.0;
    }

    score = score.min(100.0).max(0.0);

    TokenAnalysis {
        score,
        volume_trend,
        price_trend,
        liquidity_health,
        risk_level,
    }
}

fn calculate_risk_score(token: &Token) -> String {
    let mut risk_points = 0;

    // High price volatility
    if token.price_change.h1.abs() > 30.0 {
        risk_points += 2;
    }
    if token.price_change.h24.abs() > 100.0 {
        risk_points += 2;
    }

    // Low liquidity
    if token.liquidity.usd < 10000.0 {
        risk_points += 3;
    }

    // Low volume
    if token.volume.h24 < 10000.0 {
        risk_points += 2;
    }

    // New token (less than 24h old)
    let now = chrono::Utc::now().timestamp() as u64;
    if
        token.pair_created_at > 0 &&
        now > token.pair_created_at &&
        now - token.pair_created_at < 86400
    {
        risk_points += 2;
    }

    match risk_points {
        0..=2 => "Low".to_string(),
        3..=5 => "Medium".to_string(),
        6..=8 => "High".to_string(),
        _ => "Very High".to_string(),
    }
}

async fn create_html_files() -> Result<()> {
    let html = create_default_html();
    tokio::fs::write("web/static/index.html", html).await?;

    // CSS will be served from file system - no need to embed
    // JS will be served from file system - no need to embed

    Ok(())
}

fn create_default_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ScreenerBot Dashboard</title>
    <link rel="stylesheet" href="/static/style.css">
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
</head>
<body>
    <div class="container">
        <header>
            <h1>🤖 ScreenerBot Dashboard</h1>
            <div class="status" id="status">
                <span class="status-dot"></span>
                <span>Connected</span>
            </div>
        </header>
        
        <div class="stats-grid">
            <div class="stat-card">
                <h3>Total PnL</h3>
                <div class="stat-value" id="totalPnl">$0.00</div>
            </div>
            <div class="stat-card">
                <h3>Total Invested</h3>
                <div class="stat-value" id="totalInvested">$0.00</div>
            </div>
            <div class="stat-card">
                <h3>Active Trades</h3>
                <div class="stat-value" id="activeTrades">0</div>
            </div>
            <div class="stat-card">
                <h3>Win Rate</h3>
                <div class="stat-value" id="winRate">0%</div>
            </div>
        </div>
        
        <div class="tabs">
            <button class="tab-button active" onclick="showTab('positions')">Positions</button>
            <button class="tab-button" onclick="showTab('tokens')">Watched Tokens</button>
            <button class="tab-button" onclick="showTab('analytics')">Analytics</button>
        </div>
        
        <div id="positions" class="tab-content active">
            <div class="section">
                <h2>Open Positions</h2>
                <div class="table-container">
                    <table id="openPositionsTable">
                        <thead>
                            <tr>
                                <th>Token</th>
                                <th>Entry Price</th>
                                <th>Current Price</th>
                                <th>Amount</th>
                                <th>Invested</th>
                                <th>PnL</th>
                                <th>PnL %</th>
                                <th>Duration</th>
                            </tr>
                        </thead>
                        <tbody></tbody>
                    </table>
                </div>
            </div>
            
            <div class="section">
                <h2>Recent Closed Positions</h2>
                <div class="section-actions">
                    <button onclick="toggleAllClosedPositions()" class="action-button">
                        View All Closed Positions
                    </button>
                </div>
                <div class="table-container">
                    <table id="closedPositionsTable">
                        <thead>
                            <tr>
                                <th>Token</th>
                                <th>Entry Price</th>
                                <th>Exit Price</th>
                                <th>Amount</th>
                                <th>Invested</th>
                                <th>Received</th>
                                <th>PnL</th>
                                <th>PnL %</th>
                                <th>Duration</th>
                            </tr>
                        </thead>
                        <tbody></tbody>
                    </table>
                </div>
            </div>
        </div>
        
        <div id="tokens" class="tab-content">
            <div class="section">
                <h2>Watched Tokens</h2>
                <div class="tokens-grid" id="tokensGrid"></div>
            </div>
        </div>
        
        <div id="analytics" class="tab-content">
            <div class="section">
                <h2>Performance Analytics</h2>
                <div class="chart-container">
                    <canvas id="pnlChart"></canvas>
                </div>
            </div>
        </div>
    </div>
    
    <script src="/static/script.js"></script>
</body>
</html>"#.to_string()
}
