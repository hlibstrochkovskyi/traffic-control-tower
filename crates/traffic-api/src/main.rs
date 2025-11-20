//! Traffic API service - WebSocket and REST API server.
//!
//! This service provides:
//! - REST endpoints for health checks and map data
//! - WebSocket connections for real-time vehicle updates
//! - Redis pub/sub integration for broadcasting vehicle telemetry

use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, error, warn};
use common::{telemetry, Config};
use common::map::RoadGraph;
use tower_http::cors::CorsLayer;
use serde::Serialize;
use futures_util::StreamExt;

/// Simplified road representation for frontend consumption.
#[derive(Serialize, Clone)]
struct Road {
    /// Road identifier
    id: u64,
    /// Sequence of [longitude, latitude] coordinates defining the road geometry
    geometry: Vec<[f64; 2]>,
}

/// Shared application state across all handlers.
struct AppState {
    /// Broadcast channel for sending vehicle updates to WebSocket clients
    tx: broadcast::Sender<String>,
    /// Pre-filtered road segments for the frontend
    map_points: Vec<Road>,
    /// Total number of roads loaded from the map
    total_roads: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_tracing("traffic-api");

    // Load configuration from environment
    let config = Config::from_env().unwrap_or_else(|e| {
        warn!("Failed to load config: {}. Using defaults.", e);
        Config {
            kafka_brokers: "localhost:19092".to_string(),
            postgres_url: "".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            log_level: "info".to_string(),
        }
    });

    info!("🗺️ Loading map for API...");

    // Load road network from OpenStreetMap data
    let road_graph = match RoadGraph::load_from_pbf("crates/traffic-sim/assets/berlin.osm.pbf") {
        Ok(graph) => {
            info!("✅ API Map loaded: {} roads", graph.edges.len());
            graph
        },
        Err(e) => {
            error!("❌ Failed to load map: {}", e);
            RoadGraph::default()
        }
    };

    let total_roads = road_graph.edges.len();

    // Filter and transform roads for frontend rendering
    let map_points: Vec<Road> = road_graph.edges
        .iter()
        .filter(|road| {
            matches!(
                road.highway_type.as_str(),
                "motorway" | "trunk" | "primary" | "secondary" | "tertiary" |
                "residential" | "service" | "living_street"
            )
        })
        .map(|road| Road {
            id: road.id as u64,
            geometry: road.geometry
                .iter()
                .map(|point| [point.x, point.y])
                .collect(),
        })
        .collect();

    info!("📊 Prepared {} road segments for frontend", map_points.len());

    let (tx, _rx) = broadcast::channel(1000);

    let shared_state = Arc::new(AppState {
        tx: tx.clone(),
        map_points,
        total_roads,
    });

    // Start Redis pub/sub listener in background
    let state_clone = shared_state.clone();
    let redis_url = config.redis_url.clone();
    tokio::spawn(async move {
        subscribe_redis(state_clone, redis_url).await;
    });

    // Build and configure the HTTP router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/map", get(get_map))
        .route("/ws", get(ws_handler))
        .with_state(shared_state)
        .layer(CorsLayer::permissive());

    info!("🚀 API listening on 0.0.0.0:3000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// --- HANDLERS ---

/// Health check response payload.
#[derive(Serialize)]
struct HealthStatus {
    status: String,
    map_loaded: bool,
    total_roads: usize,
    visible_roads: usize,
}

/// Health check endpoint handler.
///
/// Returns the service status and map loading statistics.
async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthStatus> {
    Json(HealthStatus {
        status: "OK".to_string(),
        map_loaded: state.total_roads > 0,
        total_roads: state.total_roads,
        visible_roads: state.map_points.len(),
    })
}

/// Map data endpoint handler.
///
/// Returns all pre-filtered road segments for rendering on the frontend.
async fn get_map(State(state): State<Arc<AppState>>) -> Json<Vec<Road>> {
    info!("📍 Map requested, sending {} road segments", state.map_points.len());
    Json(state.map_points.clone())
}

/// WebSocket upgrade handler.
///
/// Upgrades the HTTP connection to a WebSocket for real-time updates.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handles an individual WebSocket connection.
///
/// Subscribes to the broadcast channel and forwards vehicle updates
/// to the connected client until disconnection.
///
/// # Arguments
///
/// * `socket` - The WebSocket connection
/// * `state` - Shared application state containing the broadcast channel
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    info!("🔌 New WebSocket client connected");

    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg)).await.is_err() {
            break;
        }
    }
}

/// Subscribes to Redis pub/sub and broadcasts messages to WebSocket clients.
///
/// Listens to the "vehicles:update" channel and forwards all received
/// messages to connected WebSocket clients via the broadcast channel.
///
/// # Arguments
///
/// * `state` - Shared application state with the broadcast sender
/// * `redis_url` - Redis connection URL
///
/// # Behavior
///
/// Runs indefinitely until the Redis connection is lost. Errors are logged
/// but the function does not panic, allowing graceful degradation.
async fn subscribe_redis(state: Arc<AppState>, redis_url: String) {
    info!("🔌 Connecting to Redis at: {}", redis_url);

    let client = match redis::Client::open(redis_url.as_str()) {
        Ok(c) => c,
        Err(e) => {
            error!("❌ Failed to create Redis client: {}", e);
            return;
        }
    };

    let con = match client.get_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            error!("❌ Failed to connect to Redis: {}", e);
            return;
        }
    };

    let mut pubsub = con.into_pubsub();
    if let Err(e) = pubsub.subscribe("vehicles:update").await {
        error!("❌ Failed to subscribe to channel: {}", e);
        return;
    }

    info!("✅ Successfully subscribed to 'vehicles:update'. Waiting for messages...");

    while let Some(msg) = pubsub.on_message().next().await {
        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(e) => {
                error!("Error getting payload: {}", e);
                continue;
            }
        };

        // Broadcast to WebSocket clients (ignore error if no subscribers)
        let _ = state.tx.send(payload);
    }

    error!("❌ Redis connection lost!");
}