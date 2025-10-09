/// WebSocket connection handler
///
/// Manages individual WebSocket connections with:
/// - Upgrade handshake
/// - Control message handling (subscribe/unsubscribe/ping/filters)
/// - Message forwarding from hub to client
/// - Health monitoring and heartbeat
/// - Graceful shutdown
use axum::extract::ws::{Message, WebSocket};
use axum::extract::Query;
use futures::{SinkExt, StreamExt};
use serde_json::{Map, Value};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::{
    arguments::is_debug_webserver_enabled,
    config,
    events::{self, Event},
    filtering,
    logger::{log, LogTag},
    pools,
    tokens::{summary_cache, TokenSummary},
    webserver::routes::tokens::{self as tokens_routes, TokenListQuery, TokenListResponse},
};

use super::{
    health::{ConnectionHealth, HealthConfig},
    hub::{ConnectionId, WsHub},
    message::{ClientMessage, MessageMetadata, ServerMessage, Topic, WsEnvelope},
    metrics::ConnectionMetrics,
    topics,
};

const EVENTS_SNAPSHOT_LIMIT: usize = 100;
const EVENTS_SNAPSHOT_FETCH_LIMIT: usize = EVENTS_SNAPSHOT_LIMIT * 3;
const TOKENS_SNAPSHOT_DEFAULT_LIMIT: usize = 200;

#[derive(Default)]
struct ConnectionState {
    filters: HashMap<String, TopicFilter>,
    paused_topics: HashSet<String>,
    pause_all: bool,
}

struct FilterUpdateResult {
    snapshot_requested: bool,
    topic: String,
}

#[derive(Clone, Debug)]
enum TopicFilter {
    Events(EventsRealtimeFilter),
    Tokens(TokensRealtimeFilter),
    Passthrough,
}

#[derive(Clone, Debug, Default)]
struct EventsRealtimeFilter {
    categories: Vec<String>,
    severity: Option<String>,
    search: Option<String>,
    mint: Option<String>,
    reference: Option<String>,
    since_id: Option<i64>,
}

#[derive(Clone, Debug)]
struct TokensRealtimeFilter {
    view: String,
    search: Option<String>,
    sort_by: String,
    sort_dir: String,
    limit: usize,
}

impl Default for TokensRealtimeFilter {
    fn default() -> Self {
        let max_limit = config::with_config(|cfg| cfg.webserver.tokens_tab.max_page_size);
        Self {
            view: "pool".to_string(),
            search: None,
            sort_by: "liquidity_usd".to_string(),
            sort_dir: "desc".to_string(),
            limit: max_limit.min(TOKENS_SNAPSHOT_DEFAULT_LIMIT).max(1),
        }
    }
}

impl TokensRealtimeFilter {
    fn from_value(raw: &Value) -> Self {
        let mut filter = TokensRealtimeFilter::default();

        if let Some(map) = raw.as_object() {
            if let Some(view) = map.get("view").and_then(|v| v.as_str()) {
                filter.view = view.to_string();
            }
            if let Some(search) = map.get("search").and_then(|v| v.as_str()) {
                let trimmed = search.trim();
                if trimmed.is_empty() {
                    filter.search = None;
                } else {
                    filter.search = Some(trimmed.to_string());
                }
            }
            if let Some(sort_by) = map.get("sort_by").and_then(|v| v.as_str()) {
                filter.sort_by = sort_by.to_string();
            }
            if let Some(sort_dir) = map.get("sort_dir").and_then(|v| v.as_str()) {
                let normalized = match sort_dir.to_lowercase().as_str() {
                    "asc" => "asc",
                    _ => "desc",
                };
                filter.sort_dir = normalized.to_string();
            }
            if let Some(limit) = map
                .get("limit")
                .or_else(|| map.get("page_size"))
                .and_then(|v| v.as_u64())
            {
                filter.limit = limit as usize;
            }
        }

        let max_limit = config::with_config(|cfg| cfg.webserver.tokens_tab.max_page_size);
        filter.limit = filter.limit.min(max_limit).max(1);
        filter
    }

    fn normalized_limit(&self) -> usize {
        let max_limit = config::with_config(|cfg| cfg.webserver.tokens_tab.max_page_size);
        self.limit.min(max_limit).max(1)
    }

    fn to_query(&self) -> TokenListQuery {
        TokenListQuery {
            view: self.view.clone(),
            search: self.search.clone().unwrap_or_default(),
            sort_by: self.sort_by.clone(),
            sort_dir: self.sort_dir.clone(),
            page: 1,
            page_size: self.normalized_limit(),
        }
    }
}

impl ConnectionState {
    fn new() -> Self {
        Self::default()
    }

    fn update_filter(&mut self, topic: &str, raw: &Value) -> FilterUpdateResult {
        let normalized_topic = Self::normalize_topic(topic);

        let snapshot_requested = raw
            .as_object()
            .and_then(|map| map.get("snapshot"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match Topic::from_code(&normalized_topic) {
            Some(Topic::EventsNew) => {
                let filter = EventsRealtimeFilter::from_value(raw);
                self.filters
                    .insert(normalized_topic.clone(), TopicFilter::Events(filter));
            }
            Some(Topic::TokensUpdate) => {
                let filter = TokensRealtimeFilter::from_value(raw);
                self.filters
                    .insert(normalized_topic.clone(), TopicFilter::Tokens(filter));
            }
            _ => {
                self.filters
                    .insert(normalized_topic.clone(), TopicFilter::Passthrough);
            }
        }

        FilterUpdateResult {
            snapshot_requested,
            topic: normalized_topic,
        }
    }

    fn events_filter(&self) -> Option<&EventsRealtimeFilter> {
        match self.filters.get(Topic::EventsNew.code()) {
            Some(TopicFilter::Events(filter)) => Some(filter),
            _ => None,
        }
    }

    fn set_events_filter(&mut self, filter: EventsRealtimeFilter) {
        self.filters.insert(
            Topic::EventsNew.code().to_string(),
            TopicFilter::Events(filter),
        );
    }

    fn tokens_filter(&self) -> Option<&TokensRealtimeFilter> {
        match self.filters.get(Topic::TokensUpdate.code()) {
            Some(TopicFilter::Tokens(filter)) => Some(filter),
            _ => None,
        }
    }

    fn set_tokens_filter(&mut self, filter: TokensRealtimeFilter) {
        self.filters.insert(
            Topic::TokensUpdate.code().to_string(),
            TopicFilter::Tokens(filter),
        );
    }

    fn prune_filters(&mut self, allowed: &HashSet<String>) {
        self.filters.retain(|topic, _| allowed.contains(topic));
        self.paused_topics.retain(|topic| allowed.contains(topic));
    }

    fn active_topics(&self) -> HashSet<String> {
        self.filters.keys().cloned().collect()
    }

    fn update_events_since(&mut self, since_id: i64) {
        if let Some(TopicFilter::Events(filter)) = self.filters.get_mut(Topic::EventsNew.code()) {
            filter.since_id = Some(since_id);
        }
    }

    fn pause_topics(&mut self, topics: &[String]) {
        if topics.is_empty() {
            self.pause_all = true;
            self.paused_topics.clear();
            return;
        }

        for topic in topics {
            self.paused_topics.insert(Self::normalize_topic(topic));
        }
    }

    fn resume_topics(&mut self, topics: &[String]) {
        if topics.is_empty() {
            self.pause_all = false;
            self.paused_topics.clear();
            return;
        }

        for topic in topics {
            let normalized = Self::normalize_topic(topic);
            self.paused_topics.remove(&normalized);
        }
    }

    fn should_forward(&self, envelope: &WsEnvelope) -> bool {
        if self.pause_all {
            return false;
        }

        let topic = Self::normalize_topic(envelope.t.as_str());
        if self.paused_topics.contains(&topic) {
            return false;
        }

        match self.filters.get(&topic) {
            Some(filter) => filter.allows(envelope),
            None => true,
        }
    }

    fn normalize_topic(topic: &str) -> String {
        Topic::from_code(topic)
            .map(|t| t.code().to_string())
            .unwrap_or_else(|| topic.to_string())
    }
}

impl TopicFilter {
    fn allows(&self, envelope: &WsEnvelope) -> bool {
        match self {
            TopicFilter::Events(filter) => filter.matches_value(&envelope.data),
            TopicFilter::Tokens(_) => true,
            TopicFilter::Passthrough => true,
        }
    }
}

impl EventsRealtimeFilter {
    fn from_value(raw: &Value) -> Self {
        let mut filter = EventsRealtimeFilter::default();

        if let Some(map) = raw.as_object() {
            if let Some(category) = map.get("category").and_then(|v| v.as_str()) {
                filter.categories.push(category.to_lowercase());
            }
            if let Some(categories) = map.get("categories").and_then(|v| v.as_array()) {
                for value in categories.iter().filter_map(|v| v.as_str()) {
                    filter.categories.push(value.to_lowercase());
                }
            }
            if let Some(severity) = map
                .get("severity")
                .or_else(|| map.get("min_level"))
                .and_then(|v| v.as_str())
            {
                filter.severity = Some(severity.to_lowercase());
            }
            if let Some(search) = map.get("search").and_then(|v| v.as_str()) {
                let trimmed = search.trim();
                if !trimmed.is_empty() {
                    filter.search = Some(trimmed.to_lowercase());
                }
            }
            if let Some(mint) = map.get("mint").and_then(|v| v.as_str()) {
                filter.mint = Some(mint.to_string());
            }
            if let Some(reference) = map.get("reference").and_then(|v| v.as_str()) {
                filter.reference = Some(reference.to_string());
            }
            if let Some(since_id) = map.get("since_id").and_then(|v| v.as_i64()) {
                filter.since_id = Some(since_id);
            }
        }

        if filter.categories.len() > 1 {
            filter.categories.sort();
            filter.categories.dedup();
        }

        filter
    }

    fn with_since_id(&self, since_id: Option<i64>) -> Self {
        let mut clone = self.clone();
        clone.since_id = since_id.or(self.since_id);
        clone
    }

    fn set_since_id(&mut self, since_id: Option<i64>) {
        self.since_id = since_id;
    }

    fn matches_value(&self, data: &Value) -> bool {
        if let Some(since) = self.since_id {
            if let Some(id) = data.get("id").and_then(|v| v.as_i64()) {
                if id <= since {
                    return false;
                }
            }
        }

        if !self.categories.is_empty() {
            let category = data
                .get("category")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase());
            if category
                .as_ref()
                .map(|c| !self.categories.contains(c))
                .unwrap_or(false)
            {
                return false;
            }
        }

        if let Some(ref severity_filter) = self.severity {
            let severity = data
                .get("severity")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase());
            if severity
                .as_ref()
                .map(|s| !severity_matches(severity_filter, s))
                .unwrap_or(true)
            {
                return false;
            }
        }

        if let Some(ref mint_filter) = self.mint {
            let mint = data.get("mint").and_then(|v| v.as_str()).unwrap_or("");
            if mint != mint_filter {
                return false;
            }
        }

        if let Some(ref reference_filter) = self.reference {
            let reference = data
                .get("reference_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if reference != reference_filter {
                return false;
            }
        }

        if let Some(ref search) = self.search {
            let mut haystacks = Vec::with_capacity(4);
            if let Some(value) = data.get("message").and_then(|v| v.as_str()) {
                haystacks.push(value.to_lowercase());
            }
            if let Some(value) = data.get("reference_id").and_then(|v| v.as_str()) {
                haystacks.push(value.to_lowercase());
            }
            if let Some(value) = data.get("mint").and_then(|v| v.as_str()) {
                haystacks.push(value.to_lowercase());
            }
            if let Some(value) = data.get("subtype").and_then(|v| v.as_str()) {
                haystacks.push(value.to_lowercase());
            }

            if !haystacks.iter().any(|hay| hay.contains(search)) {
                return false;
            }
        }

        true
    }

    fn matches_event(&self, event: &Event) -> bool {
        if let Some(since) = self.since_id {
            if let Some(id) = event.id {
                if id <= since {
                    return false;
                }
            }
        }

        if !self.categories.is_empty() {
            let category = event.category.to_string().to_lowercase();
            if !self.categories.contains(&category) {
                return false;
            }
        }

        if let Some(ref severity_filter) = self.severity {
            let severity = event.severity.to_string().to_lowercase();
            if !severity_matches(severity_filter, &severity) {
                return false;
            }
        }

        if let Some(ref mint_filter) = self.mint {
            if event.mint.as_deref() != Some(mint_filter.as_str()) {
                return false;
            }
        }

        if let Some(ref reference_filter) = self.reference {
            if event.reference_id.as_deref() != Some(reference_filter.as_str()) {
                return false;
            }
        }

        if let Some(ref search) = self.search {
            let mut haystacks = Vec::with_capacity(4);
            if let Some(value) = event
                .payload
                .get("message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase())
            {
                haystacks.push(value);
            }
            if let Some(value) = event.reference_id.as_ref().map(|s| s.to_lowercase()) {
                haystacks.push(value);
            }
            if let Some(value) = event.mint.as_ref().map(|s| s.to_lowercase()) {
                haystacks.push(value);
            }
            if let Some(value) = event.subtype.as_ref().map(|s| s.to_lowercase()) {
                haystacks.push(value);
            }

            if !haystacks.iter().any(|hay| hay.contains(search)) {
                return false;
            }
        }

        true
    }
}

fn severity_matches(filter: &str, actual: &str) -> bool {
    severity_rank(actual) >= severity_rank(filter)
}

fn severity_rank(value: &str) -> u8 {
    match value.to_lowercase().as_str() {
        "debug" => 0,
        "info" => 1,
        "warn" | "warning" => 2,
        "error" => 3,
        _ => 1,
    }
}

/// Handle a WebSocket connection
pub async fn handle_connection(socket: WebSocket, hub: Arc<WsHub>) {
    // Register connection with hub
    let (conn_id, mut hub_rx) = hub.register_connection().await;

    // Split socket
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Initialize health tracker
    let health_config = config::with_config(|cfg| {
        HealthConfig::from_config(
            cfg.webserver.websocket.heartbeat_secs,
            cfg.webserver.websocket.client_idle_timeout_secs,
        )
    });
    let mut health = ConnectionHealth::new(health_config);

    // Initialize metrics
    let metrics = ConnectionMetrics::new();
    let mut state = ConnectionState::new();

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!("Connection {} started", conn_id),
        );
    }

    // Main message loop
    loop {
        tokio::select! {
            biased;

            // Messages from hub (broadcast to client)
            Some(envelope) = hub_rx.recv() => {
        if let Err(e) = forward_to_client(&mut ws_tx, envelope, &metrics, &state).await {
                    log(
                        LogTag::Webserver,
                        "WARN",
                        &format!("Connection {}: failed to send message: {}", conn_id, e),
                    );
                    break;
                }
            }

            // Messages from client (control commands)
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        health.record_activity();

                        if let Err(e) = handle_client_message(&text, &mut ws_tx, &hub, conn_id, &mut state, &metrics).await {
                            log(
                                LogTag::Webserver,
                                "WARN",
                                &format!("Connection {}: error handling client message: {}", conn_id, e),
                            );
                        }
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {
                        health.record_activity();
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        if is_debug_webserver_enabled() {
                            log(
                                LogTag::Webserver,
                                "DEBUG",
                                &format!("Connection {}: client closed", conn_id),
                            );
                        }
                        break;
                    }
                    Some(Err(e)) => {
                        log(
                            LogTag::Webserver,
                            "WARN",
                            &format!("Connection {}: websocket error: {}", conn_id, e),
                        );
                        break;
                    }
                    _ => {}
                }
            }

            // Health checks
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                // Check if client is idle
                if health.is_idle() {
                    log(
                        LogTag::Webserver,
                        "WARN",
                        &format!(
                            "Connection {}: idle timeout ({}s)",
                            conn_id,
                            health.seconds_since_activity()
                        ),
                    );
                    break;
                }

                // Check if pong is overdue
                if health.is_pong_overdue() {
                    log(
                        LogTag::Webserver,
                        "WARN",
                        &format!("Connection {}: pong timeout", conn_id),
                    );
                    break;
                }

                // Send ping if needed
                if health.needs_ping() {
                    if is_debug_webserver_enabled() {
                        log(
                            LogTag::Webserver,
                            "DEBUG",
                            &format!("Connection {}: sending ping", conn_id),
                        );
                    }
                    if let Err(_) = ws_tx.send(Message::Ping(vec![])).await {
                        break;
                    }
                    health.record_ping();
                }
            }
        }
    }

    // Cleanup
    hub.unregister_connection(conn_id).await;

    if is_debug_webserver_enabled() {
        let snapshot = metrics.snapshot();
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "Connection {} closed (sent={}, dropped={}, lag_events={})",
                conn_id, snapshot.messages_sent, snapshot.messages_dropped, snapshot.lag_events
            ),
        );
    }
}

/// Forward envelope to client
async fn forward_to_client(
    ws_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
    envelope: WsEnvelope,
    metrics: &Arc<ConnectionMetrics>,
    state: &ConnectionState,
) -> Result<(), axum::Error> {
    if !state.should_forward(&envelope) {
        return Ok(());
    }

    let msg = ServerMessage::Data(envelope);

    match msg.to_json() {
        Ok(json) => {
            ws_tx.send(Message::Text(json)).await?;
            metrics.inc_sent();
            Ok(())
        }
        Err(e) => {
            log(
                LogTag::Webserver,
                "ERROR",
                &format!("Failed to serialize message: {}", e),
            );
            Ok(()) // Don't break connection on serialization error
        }
    }
}

/// Handle client control message
async fn handle_client_message(
    text: &str,
    ws_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
    hub: &Arc<WsHub>,
    conn_id: ConnectionId,
    state: &mut ConnectionState,
    metrics: &Arc<ConnectionMetrics>,
) -> Result<(), String> {
    let client_msg: ClientMessage =
        serde_json::from_str(text).map_err(|e| format!("Invalid client message: {}", e))?;

    match client_msg {
        ClientMessage::Hello {
            client_id,
            app_version,
            pages_supported,
        } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!(
                        "Connection {}: hello (client_id={:?}, version={:?}, pages={:?})",
                        conn_id, client_id, app_version, pages_supported
                    ),
                );
            }

            let response = ServerMessage::Ack {
                message: "Hello acknowledged".to_string(),
                context: Some(serde_json::json!({
                    "connection_id": conn_id,
                    "protocol_version": super::message::PROTOCOL_VERSION,
                })),
            };

            send_control_message(ws_tx, response).await?;
        }

        ClientMessage::SetFilters { topics } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!(
                        "Connection {}: set_filters ({} topics)",
                        conn_id,
                        topics.len()
                    ),
                );
            }

            let mut snapshot_topics = Vec::new();
            let mut desired_topics = HashSet::new();
            for (topic, value) in topics.iter() {
                let result = state.update_filter(topic, value);
                desired_topics.insert(result.topic.clone());
                if result.snapshot_requested {
                    snapshot_topics.push(result.topic.clone());
                }
            }

            state.prune_filters(&desired_topics);
            hub.update_connection_topics(conn_id, state.active_topics())
                .await;

            let response = ServerMessage::Ack {
                message: format!("Filters updated for {} topics", topics.len()),
                context: Some(serde_json::json!({
                    "topics": topics.keys().collect::<Vec<_>>(),
                })),
            };

            send_control_message(ws_tx, response).await?;

            for topic in snapshot_topics {
                match Topic::from_code(&topic) {
                    Some(Topic::EventsNew) => {
                        let filter = state.events_filter().cloned().unwrap_or_default();
                        if let Some(last_id) =
                            send_events_snapshot(ws_tx, hub, metrics, filter).await?
                        {
                            state.update_events_since(last_id);
                        }
                    }
                    Some(Topic::TokensUpdate) => {
                        let filter = state.tokens_filter().cloned().unwrap_or_default();
                        send_tokens_snapshot(ws_tx, hub, metrics, filter).await?;
                    }
                    _ => {}
                }
            }
        }

        ClientMessage::Pause { topics } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Connection {}: pause ({:?})", conn_id, topics),
                );
            }

            state.pause_topics(&topics);
            let response = ServerMessage::Ack {
                message: "Pause acknowledged".to_string(),
                context: Some(serde_json::json!({"topics": topics})),
            };

            send_control_message(ws_tx, response).await?;
        }

        ClientMessage::Resume { topics } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Connection {}: resume ({:?})", conn_id, topics),
                );
            }

            state.resume_topics(&topics);
            let response = ServerMessage::Ack {
                message: "Resume acknowledged".to_string(),
                context: Some(serde_json::json!({"topics": topics})),
            };

            send_control_message(ws_tx, response).await?;
        }

        ClientMessage::Resync { topics } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Connection {}: resync ({} topics)", conn_id, topics.len()),
                );
            }

            let response = ServerMessage::Ack {
                message: "Resync acknowledged".to_string(),
                context: Some(serde_json::json!({
                    "topics": topics.keys().collect::<Vec<_>>(),
                })),
            };

            send_control_message(ws_tx, response).await?;

            for (topic, value) in topics.iter() {
                match Topic::from_code(topic) {
                    Some(Topic::EventsNew) => {
                        let mut filter = state.events_filter().cloned().unwrap_or_default();
                        let since_override = value
                            .as_object()
                            .and_then(|map| map.get("since_id"))
                            .and_then(|v| v.as_i64());
                        filter.set_since_id(since_override);
                        state.set_events_filter(filter.clone());
                        if let Some(last_id) =
                            send_events_snapshot(ws_tx, hub, metrics, filter).await?
                        {
                            state.update_events_since(last_id);
                        }
                    }
                    Some(Topic::TokensUpdate) => {
                        let filter = TokensRealtimeFilter::from_value(value);
                        state.set_tokens_filter(filter.clone());
                        send_tokens_snapshot(ws_tx, hub, metrics, filter).await?;
                    }
                    _ => {}
                }
            }
        }

        ClientMessage::Ping { id } => {
            let response = ServerMessage::Pong { id };
            send_control_message(ws_tx, response).await?;
        }
    }

    Ok(())
}

/// Send control message to client
async fn send_control_message(
    ws_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
    msg: ServerMessage,
) -> Result<(), String> {
    let json = msg
        .to_json()
        .map_err(|e| format!("Serialization error: {}", e))?;
    ws_tx
        .send(Message::Text(json))
        .await
        .map_err(|e| format!("Send error: {}", e))?;
    Ok(())
}

async fn send_events_snapshot(
    ws_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
    hub: &Arc<WsHub>,
    metrics: &Arc<ConnectionMetrics>,
    filter: EventsRealtimeFilter,
) -> Result<Option<i64>, String> {
    let mut snapshot_events = Vec::with_capacity(EVENTS_SNAPSHOT_LIMIT);
    let mut cached = events::cached_events_head(EVENTS_SNAPSHOT_FETCH_LIMIT).await;

    // cached_events_head returns newest first; reverse iterate to send oldest-to-newest snapshot
    cached.reverse();

    for event in cached.into_iter() {
        if filter.matches_event(&event) {
            snapshot_events.push(event);
        }
        if snapshot_events.len() >= EVENTS_SNAPSHOT_LIMIT {
            break;
        }
    }

    let total = snapshot_events.len();

    send_control_message(
        ws_tx,
        ServerMessage::SnapshotBegin {
            topic: Topic::EventsNew.code().to_string(),
            total,
        },
    )
    .await?;

    let mut last_id = None;

    for event in snapshot_events.into_iter() {
        if let Some(id) = event.id {
            last_id = Some(id);
        }
        let seq = hub.next_seq(Topic::EventsNew.code()).await;
        let envelope = topics::events::event_to_envelope(&event, seq).as_snapshot();
        let msg = ServerMessage::Data(envelope);
        let json = msg
            .to_json()
            .map_err(|e| format!("Serialization error: {}", e))?;
        ws_tx
            .send(Message::Text(json))
            .await
            .map_err(|e| format!("Send error: {}", e))?;
        metrics.inc_sent();
    }

    send_control_message(
        ws_tx,
        ServerMessage::SnapshotEnd {
            topic: Topic::EventsNew.code().to_string(),
            sent: total,
        },
    )
    .await?;

    Ok(last_id)
}

fn build_cached_tokens_snapshot(filter: &TokensRealtimeFilter) -> Option<TokenListResponse> {
    match filter.view.as_str() {
        "pool" => build_cached_pool_snapshot(filter),
        "all" => build_cached_all_snapshot(filter),
        "positions" => build_cached_positions_snapshot(filter),
        "blacklisted" => build_cached_blacklisted_snapshot(filter),
        "secure" => build_cached_secure_snapshot(filter),
        "passed" => build_cached_passed_snapshot(filter),
        "rejected" => build_cached_rejected_snapshot(filter),
        "recent" => build_cached_recent_snapshot(filter),
        _ => None,
    }
}

fn build_cached_pool_snapshot(filter: &TokensRealtimeFilter) -> Option<TokenListResponse> {
    let mints = pools::get_available_tokens();
    if mints.is_empty() {
        return Some(finalize_token_snapshot(Vec::new(), filter));
    }

    let (summaries, _missing) = summary_cache::get_for_mints(&mints);

    // Cache is pre-warmed at startup, so use whatever we have
    Some(finalize_token_snapshot(summaries, filter))
}

fn build_cached_all_snapshot(filter: &TokensRealtimeFilter) -> Option<TokenListResponse> {
    let summaries = summary_cache::all();
    if summaries.is_empty() {
        return None;
    }

    Some(finalize_token_snapshot(summaries, filter))
}

fn build_cached_positions_snapshot(filter: &TokensRealtimeFilter) -> Option<TokenListResponse> {
    let summaries: Vec<TokenSummary> = summary_cache::all()
        .into_iter()
        .filter(|summary| summary.has_open_position)
        .collect();

    Some(finalize_token_snapshot(summaries, filter))
}

fn build_cached_blacklisted_snapshot(filter: &TokensRealtimeFilter) -> Option<TokenListResponse> {
    let summaries: Vec<TokenSummary> = summary_cache::all()
        .into_iter()
        .filter(|summary| summary.blacklisted)
        .collect();

    Some(finalize_token_snapshot(summaries, filter))
}

fn build_cached_secure_snapshot(filter: &TokensRealtimeFilter) -> Option<TokenListResponse> {
    let threshold =
        config::with_config(|cfg| cfg.webserver.tokens_tab.secure_token_score_threshold);

    let summaries: Vec<TokenSummary> = summary_cache::all()
        .into_iter()
        .filter(|summary| {
            summary
                .security_score
                .map(|score| score > threshold)
                .unwrap_or(false)
                && !summary.rugged.unwrap_or(false)
        })
        .collect();

    Some(finalize_token_snapshot(summaries, filter))
}

fn build_cached_passed_snapshot(filter: &TokensRealtimeFilter) -> Option<TokenListResponse> {
    let passed = filtering::get_passed_tokens();
    if passed.is_empty() {
        return Some(finalize_token_snapshot(Vec::new(), filter));
    }

    // Deduplicate by mint while preserving most recent pass result
    let mut seen = HashSet::new();
    let mut passed_mints: Vec<String> = Vec::new();
    for entry in passed.iter().rev() {
        let mint = &entry.mint;
        if !seen.contains(mint) {
            seen.insert(mint.clone());
            passed_mints.push(mint.clone());
        }
    }

    let (summaries, _missing) = summary_cache::get_for_mints(&passed_mints);

    // Cache is pre-warmed at startup, so use whatever we have
    Some(finalize_token_snapshot(summaries, filter))
}

fn build_cached_rejected_snapshot(filter: &TokensRealtimeFilter) -> Option<TokenListResponse> {
    let rejected = filtering::get_rejected_tokens();
    if rejected.is_empty() {
        return Some(finalize_token_snapshot(Vec::new(), filter));
    }

    // Deduplicate by mint while preserving most recent rejection
    let mut seen = HashSet::new();
    let mut rejected_mints: Vec<String> = Vec::new();
    for entry in rejected.iter().rev() {
        let mint = &entry.mint;
        if !seen.contains(mint) {
            seen.insert(mint.clone());
            rejected_mints.push(mint.clone());
        }
    }

    let (summaries, _missing) = summary_cache::get_for_mints(&rejected_mints);

    // Cache is pre-warmed at startup, so use whatever we have
    Some(finalize_token_snapshot(summaries, filter))
}

fn build_cached_recent_snapshot(filter: &TokensRealtimeFilter) -> Option<TokenListResponse> {
    let hours = config::with_config(|cfg| cfg.webserver.tokens_tab.recent_token_hours);
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);

    let summaries: Vec<TokenSummary> = summary_cache::all()
        .into_iter()
        .filter(|summary| {
            // Filter by price_updated_at as a proxy for recent activity
            // (we don't have pair_created_at in TokenSummary)
            summary
                .price_updated_at
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                .map(|dt| dt > cutoff)
                .unwrap_or(false)
        })
        .collect();

    Some(finalize_token_snapshot(summaries, filter))
}

fn finalize_token_snapshot(
    mut items: Vec<TokenSummary>,
    filter: &TokensRealtimeFilter,
) -> TokenListResponse {
    if let Some(search) = filter.search.as_ref() {
        let search_lower = search.trim().to_lowercase();
        if !search_lower.is_empty() {
            items = items
                .into_iter()
                .filter(|summary| token_matches_search(summary, &search_lower))
                .collect();
        }
    }

    sort_token_summaries(&mut items, &filter.sort_by, &filter.sort_dir);

    let total = items.len();
    let page_size = filter.normalized_limit();
    if items.len() > page_size {
        items.truncate(page_size);
    }
    let total_pages = if total == 0 {
        0
    } else {
        (total + page_size - 1) / page_size
    };

    TokenListResponse {
        items,
        page: 1,
        page_size,
        total,
        total_pages,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

fn token_matches_search(summary: &TokenSummary, search_lower: &str) -> bool {
    summary.symbol.to_lowercase().contains(search_lower)
        || summary.mint.to_lowercase().contains(search_lower)
        || summary
            .name
            .as_ref()
            .map(|name| name.to_lowercase().contains(search_lower))
            .unwrap_or(false)
}

fn sort_token_summaries(tokens: &mut [TokenSummary], sort_by: &str, sort_dir: &str) {
    let ascending = sort_dir.eq_ignore_ascii_case("asc");

    tokens.sort_by(|a, b| {
        let cmp = match sort_by {
            "symbol" => a.symbol.cmp(&b.symbol),
            "liquidity_usd" => cmp_f64(a.liquidity_usd, b.liquidity_usd),
            "volume_24h" => cmp_f64(a.volume_24h, b.volume_24h),
            "price_sol" => cmp_f64(a.price_sol, b.price_sol),
            "market_cap" => cmp_f64(a.market_cap, b.market_cap),
            "fdv" => cmp_f64(a.fdv, b.fdv),
            "security_score" => a
                .security_score
                .unwrap_or(0)
                .cmp(&b.security_score.unwrap_or(0)),
            "price_change_h1" => cmp_f64(a.price_change_h1, b.price_change_h1),
            "price_change_h24" => cmp_f64(a.price_change_h24, b.price_change_h24),
            "updated_at" => a
                .price_updated_at
                .unwrap_or(0)
                .cmp(&b.price_updated_at.unwrap_or(0)),
            _ => a.mint.cmp(&b.mint),
        };

        if ascending {
            cmp
        } else {
            cmp.reverse()
        }
    });
}

fn cmp_f64(left: Option<f64>, right: Option<f64>) -> Ordering {
    left.unwrap_or(0.0)
        .partial_cmp(&right.unwrap_or(0.0))
        .unwrap_or(Ordering::Equal)
}

async fn send_tokens_snapshot(
    ws_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
    hub: &Arc<WsHub>,
    metrics: &Arc<ConnectionMetrics>,
    filter: TokensRealtimeFilter,
) -> Result<(), String> {
    let query = filter.to_query();
    let response = match build_cached_tokens_snapshot(&filter) {
        Some(snapshot) => snapshot,
        None => {
            // Fallback only for unsupported views (shouldn't happen with current implementation)
            let axum::Json(legacy) = tokens_routes::get_tokens_list(Query(query)).await;
            legacy
        }
    };

    let total_sent = response.items.len();
    let total_available = response.total;

    send_control_message(
        ws_tx,
        ServerMessage::SnapshotBegin {
            topic: Topic::TokensUpdate.code().to_string(),
            total: total_sent,
        },
    )
    .await?;

    for (idx, summary) in response.items.into_iter().enumerate() {
        let seq = hub.next_seq(Topic::TokensUpdate.code()).await;
        let data = serde_json::to_value(&summary).unwrap_or_else(|err| {
            log(
                LogTag::Webserver,
                "ERROR",
                &format!(
                    "Failed to serialize token summary for {}: {}",
                    summary.mint, err
                ),
            );
            serde_json::json!({ "mint": summary.mint })
        });

        let mut envelope = topics::tokens::token_to_envelope(&summary.mint, data, seq);

        if idx == 0 {
            let mut extra = Map::new();
            extra.insert("total".to_string(), serde_json::json!(total_available));
            extra.insert("view".to_string(), serde_json::json!(filter.view.clone()));
            extra.insert(
                "sort_by".to_string(),
                serde_json::json!(filter.sort_by.clone()),
            );
            extra.insert(
                "sort_dir".to_string(),
                serde_json::json!(filter.sort_dir.clone()),
            );
            if let Some(search) = filter.search.clone() {
                extra.insert("search".to_string(), serde_json::json!(search));
            }
            let meta = MessageMetadata {
                snapshot: None,
                dropped: None,
                extra: Some(extra),
            };
            envelope = envelope.with_meta(meta);
        }

        envelope = envelope.as_snapshot();
        let msg = ServerMessage::Data(envelope);
        let json = msg
            .to_json()
            .map_err(|e| format!("Serialization error: {}", e))?;
        ws_tx
            .send(Message::Text(json))
            .await
            .map_err(|e| format!("Send error: {}", e))?;
        metrics.inc_sent();
    }

    send_control_message(
        ws_tx,
        ServerMessage::SnapshotEnd {
            topic: Topic::TokensUpdate.code().to_string(),
            sent: total_sent,
        },
    )
    .await?;

    Ok(())
}
