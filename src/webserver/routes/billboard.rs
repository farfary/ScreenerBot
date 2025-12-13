//! Billboard API routes
//!
//! Fetches featured tokens from the website's billboard API

use axum::{
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::webserver::{state::AppState, utils::success_response};

/// Billboard token from website API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillboardToken {
    pub id: String,
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub description: Option<String>,
    pub logo: Option<String>,
    pub banner: Option<String>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
    pub featured: bool,
}

/// Cached billboard data
struct BillboardCache {
    tokens: Vec<BillboardToken>,
    fetched_at: Instant,
}

static BILLBOARD_CACHE: LazyLock<Arc<RwLock<Option<BillboardCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes
const WEBSITE_BILLBOARD_URL: &str = "https://screenerbot.io/api/billboard";

/// Fetch billboard tokens from website
async fn fetch_from_website() -> Result<Vec<BillboardToken>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(WEBSITE_BILLBOARD_URL)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch billboard: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Billboard API returned status: {}",
            response.status()
        ));
    }

    let tokens: Vec<BillboardToken> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse billboard: {}", e))?;

    Ok(tokens)
}

/// Get billboard tokens (with caching)
async fn get_billboard() -> Result<Vec<BillboardToken>, String> {
    // Check cache
    {
        let cache = BILLBOARD_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < CACHE_TTL {
                return Ok(cached.tokens.clone());
            }
        }
    }

    // Fetch fresh data
    let tokens = fetch_from_website().await?;

    // Update cache
    {
        let mut cache = BILLBOARD_CACHE.write().await;
        *cache = Some(BillboardCache {
            tokens: tokens.clone(),
            fetched_at: Instant::now(),
        });
    }

    Ok(tokens)
}

/// GET /api/billboard - Get featured tokens
async fn get_billboard_handler() -> Response {
    match get_billboard().await {
        Ok(tokens) => {
            let count = tokens.len();
            success_response(serde_json::json!({
                "tokens": tokens,
                "count": count
            }))
        }
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e,
            "tokens": [],
            "count": 0
        }))
        .into_response(),
    }
}

/// Billboard routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/billboard", get(get_billboard_handler))
}
