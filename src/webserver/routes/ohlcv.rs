// OHLCV API routes

use crate::ohlcvs::priorities::ActivityType;
use crate::ohlcvs::types::{ OhlcvDataPoint, PoolMetadata, Priority, Timeframe };
use crate::ohlcvs::{
    add_token_monitoring,
    get_available_pools,
    get_data_gaps,
    get_metrics,
    get_ohlcv_data,
    record_activity,
    remove_token_monitoring,
    request_refresh,
};
use crate::webserver::utils::{ error_response, success_response };
use axum::{
    extract::{ Path, Query },
    http::StatusCode,
    response::Json,
    routing::{ get, post },
    Router,
};
use serde::{ Deserialize, Serialize };

// ==================== Response Types ====================

#[derive(Debug, Serialize)]
struct OhlcvDataResponse {
    mint: String,
    pool_address: Option<String>,
    timeframe: String,
    data: Vec<OhlcvDataPoint>,
    count: usize,
}

#[derive(Debug, Serialize)]
struct PoolsResponse {
    mint: String,
    pools: Vec<PoolMetadata>,
    default_pool: Option<String>,
}

#[derive(Debug, Serialize)]
struct GapsResponse {
    mint: String,
    timeframe: String,
    gaps: Vec<GapInfo>,
    total_gaps: usize,
}

#[derive(Debug, Serialize)]
struct GapInfo {
    start_timestamp: i64,
    end_timestamp: i64,
    duration_seconds: i64,
}

#[derive(Debug, Serialize)]
struct DataStatusResponse {
    mint: String,
    has_data: bool,
    timeframes_available: Vec<String>,
    latest_timestamp: Option<i64>,
    data_quality: String,
}

#[derive(Debug, Serialize)]
struct MetricsResponse {
    tokens_monitored: usize,
    pools_tracked: usize,
    api_calls_per_minute: f64,
    cache_hit_rate_percent: f64,
    average_fetch_latency_ms: f64,
    gaps_detected: usize,
    gaps_filled: usize,
    data_points_stored: usize,
    database_size_mb: f64,
}

// ==================== Query Parameters ====================

#[derive(Debug, Deserialize)]
struct OhlcvQuery {
    timeframe: Option<String>,
    pool: Option<String>,
    limit: Option<usize>,
    from: Option<i64>,
    to: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct GapsQuery {
    timeframe: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MonitorRequest {
    priority: Option<String>,
}

// ==================== Route Handlers ====================

async fn get_ohlcv_data_handler(
    Path(mint): Path<String>,
    Query(params): Query<OhlcvQuery>
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Parse timeframe
    let timeframe = params.timeframe
        .as_deref()
        .and_then(Timeframe::from_str)
        .unwrap_or(Timeframe::Minute1);

    let limit = params.limit.unwrap_or(100).min(1000); // Cap at 1000

    // Fetch data
    match
        get_ohlcv_data(
            &mint,
            timeframe,
            params.pool.as_deref(),
            limit,
            params.from,
            params.to
        ).await
    {
        Ok(data) => {
            let response = OhlcvDataResponse {
                mint: mint.clone(),
                pool_address: params.pool,
                timeframe: timeframe.as_str().to_string(),
                count: data.len(),
                data,
            };

            Ok(Json(success_response(response)))
        }
        Err(e) =>
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(&format!("Failed to fetch OHLCV data: {}", e))),
            )),
    }
}

async fn get_pools_handler(Path(mint): Path<String>) -> Result<
    Json<serde_json::Value>,
    (StatusCode, Json<serde_json::Value>)
> {
    match get_available_pools(&mint).await {
        Ok(pools) => {
            let default_pool = pools
                .iter()
                .find(|p| p.is_default)
                .map(|p| p.address.clone());

            let response = PoolsResponse {
                mint,
                pools,
                default_pool,
            };

            Ok(Json(success_response(response)))
        }
        Err(e) =>
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(&format!("Failed to fetch pools: {}", e))),
            )),
    }
}

async fn get_gaps_handler(
    Path(mint): Path<String>,
    Query(params): Query<GapsQuery>
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let timeframe = params.timeframe
        .as_deref()
        .and_then(Timeframe::from_str)
        .unwrap_or(Timeframe::Minute1);

    match get_data_gaps(&mint, timeframe).await {
        Ok(gap_tuples) => {
            let gaps: Vec<GapInfo> = gap_tuples
                .iter()
                .map(|(start, end)| GapInfo {
                    start_timestamp: *start,
                    end_timestamp: *end,
                    duration_seconds: end - start,
                })
                .collect();

            let response = GapsResponse {
                mint,
                timeframe: timeframe.as_str().to_string(),
                total_gaps: gaps.len(),
                gaps,
            };

            Ok(Json(success_response(response)))
        }
        Err(e) =>
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(&format!("Failed to fetch gaps: {}", e))),
            )),
    }
}

async fn get_status_handler(Path(mint): Path<String>) -> Result<
    Json<serde_json::Value>,
    (StatusCode, Json<serde_json::Value>)
> {
    // Check if we have data for this token
    let has_data = get_ohlcv_data(&mint, Timeframe::Minute1, None, 1, None, None).await
        .map(|d| !d.is_empty())
        .unwrap_or(false);

    // Check which timeframes have data
    let mut timeframes_available = Vec::new();
    for tf in Timeframe::all() {
        if let Ok(data) = get_ohlcv_data(&mint, tf, None, 1, None, None).await {
            if !data.is_empty() {
                timeframes_available.push(tf.as_str().to_string());
            }
        }
    }

    // Get latest timestamp
    let latest_timestamp = get_ohlcv_data(&mint, Timeframe::Minute1, None, 1, None, None).await
        .ok()
        .and_then(|d| d.first().map(|p| p.timestamp));

    // Simple quality assessment
    let data_quality = if has_data {
        if timeframes_available.len() >= 5 {
            "excellent"
        } else if timeframes_available.len() >= 3 {
            "good"
        } else {
            "partial"
        }
    } else {
        "no_data"
    };

    let response = DataStatusResponse {
        mint,
        has_data,
        timeframes_available,
        latest_timestamp,
        data_quality: data_quality.to_string(),
    };

    Ok(Json(success_response(response)))
}

async fn refresh_handler(Path(mint): Path<String>) -> Result<
    Json<serde_json::Value>,
    (StatusCode, Json<serde_json::Value>)
> {
    match request_refresh(&mint).await {
        Ok(_) =>
            Ok(
                Json(
                    success_response(
                        serde_json::json!({
            "message": "Refresh requested",
            "mint": mint
        })
                    )
                )
            ),
        Err(e) =>
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(&format!("Failed to refresh: {}", e))),
            )),
    }
}

async fn get_metrics_handler() -> Result<
    Json<serde_json::Value>,
    (StatusCode, Json<serde_json::Value>)
> {
    let metrics = get_metrics().await;

    let response = MetricsResponse {
        tokens_monitored: metrics.tokens_monitored,
        pools_tracked: metrics.pools_tracked,
        api_calls_per_minute: metrics.api_calls_per_minute,
        cache_hit_rate_percent: metrics.cache_hit_rate * 100.0,
        average_fetch_latency_ms: metrics.average_fetch_latency_ms,
        gaps_detected: metrics.gaps_detected,
        gaps_filled: metrics.gaps_filled,
        data_points_stored: metrics.data_points_stored,
        database_size_mb: metrics.database_size_mb,
    };

    Ok(Json(success_response(response)))
}

async fn add_monitoring_handler(
    Path(mint): Path<String>,
    Json(body): Json<MonitorRequest>
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let priority = body.priority
        .as_deref()
        .and_then(Priority::from_str)
        .unwrap_or(Priority::Medium);

    match add_token_monitoring(&mint, priority).await {
        Ok(_) =>
            Ok(
                Json(
                    success_response(
                        serde_json::json!({
            "message": "Monitoring started",
            "mint": mint,
            "priority": priority.as_str()
        })
                    )
                )
            ),
        Err(e) =>
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(&format!("Failed to start monitoring: {}", e))),
            )),
    }
}

async fn remove_monitoring_handler(Path(mint): Path<String>) -> Result<
    Json<serde_json::Value>,
    (StatusCode, Json<serde_json::Value>)
> {
    match remove_token_monitoring(&mint).await {
        Ok(_) =>
            Ok(
                Json(
                    success_response(
                        serde_json::json!({
            "message": "Monitoring stopped",
            "mint": mint
        })
                    )
                )
            ),
        Err(e) =>
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(&format!("Failed to stop monitoring: {}", e))),
            )),
    }
}

async fn record_view_handler(Path(mint): Path<String>) -> Result<
    Json<serde_json::Value>,
    (StatusCode, Json<serde_json::Value>)
> {
    match record_activity(&mint, ActivityType::ChartViewed).await {
        Ok(_) =>
            Ok(
                Json(
                    success_response(
                        serde_json::json!({
            "message": "Activity recorded",
            "mint": mint
        })
                    )
                )
            ),
        Err(e) =>
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(&format!("Failed to record activity: {}", e))),
            )),
    }
}

// ==================== Router ====================

pub fn ohlcv_routes() -> Router {
    Router::new()
        // Data endpoints
        .route("/api/ohlcv/:mint", get(get_ohlcv_data_handler))
        .route("/api/ohlcv/:mint/pools", get(get_pools_handler))
        .route("/api/ohlcv/:mint/gaps", get(get_gaps_handler))
        .route("/api/ohlcv/:mint/status", get(get_status_handler))
        // Control endpoints
        .route("/api/ohlcv/:mint/refresh", post(refresh_handler))
        .route("/api/ohlcv/:mint/monitor", post(add_monitoring_handler))
        .route("/api/ohlcv/:mint/monitor", axum::routing::delete(remove_monitoring_handler))
        .route("/api/ohlcv/:mint/view", post(record_view_handler))
        // System endpoints
        .route("/api/ohlcv/metrics", get(get_metrics_handler))
}
