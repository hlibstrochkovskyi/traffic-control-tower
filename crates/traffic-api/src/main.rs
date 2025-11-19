use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State, Query},
    response::IntoResponse,
    routing::get,
    Router,
};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use traffic_common::{Config, init_tracing};
use anyhow::Result;
use redis::AsyncCommands;

// Application state (available to all handlers)
#[derive(Clone)]
struct AppState {
    redis: ConnectionManager,
}

// Query parameters from the frontend (what the user is viewing)
#[derive(Deserialize)]
struct ViewportParams {
    lat: f64,
    lon: f64,
    radius_km: f64,
}

// Data to send to the frontend
#[derive(Serialize)]
struct VehicleData {
    id: String,
    lat: f64,
    lon: f64,
    speed: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing("traffic-api");
    let config = Config::from_env()?;

    // Connect to Redis
    let client = redis::Client::open(config.redis_url.as_str())?;
    let redis = client.get_tokio_connection_manager().await?;

    let state = AppState { redis };

    // Router
    let app = Router::new()
        .route("/ws", get(ws_handler)) // WebSocket endpoint
        .route("/health", get(|| async { "OK" })) // Liveness check
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any) // Allow requests from any origin (e.g., localhost:5173 for Vite)
                .allow_methods(tower_http::cors::Any),
        )
        .with_state(Arc::new(state));

    let addr = "0.0.0.0:3000";
    tracing::info!("API listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// WebSocket connection handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<ViewportParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!("New client connected: view ({}, {}) r={}km", params.lat, params.lon, params.radius_km);
    ws.on_upgrade(move |socket| handle_socket(socket, state, params))
}

// Logic for sending data to the client
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, viewport: ViewportParams) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100)); // 10 FPS
    let mut redis = state.redis.clone();

    loop {
        interval.tick().await;

        // Look up vehicles within the radius
        match fetch_vehicles_in_viewport(&mut redis, &viewport).await {
            Ok(vehicles) => {
                let json = serde_json::to_string(&vehicles).unwrap_or_default();
                if socket.send(Message::Text(json)).await.is_err() {
                    tracing::info!("Client disconnected");
                    break;
                }
            }
            Err(e) => {
                tracing::error!("Redis error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}

async fn fetch_vehicles_in_viewport(
    redis: &mut ConnectionManager,
    viewport: &ViewportParams,
) -> Result<Vec<VehicleData>> {
    // GEORADIUS returns a list of vehicles within the circle
    // Use WITHCOORD option to get coordinates immediately
    let results: Vec<(String, f64, f64)> = redis.geo_radius(
        "vehicles:current",
        viewport.lon,
        viewport.lat,
        viewport.radius_km,
        redis::geo::Unit::Kilometers,
        redis::geo::RadiusOptions::default().with_coord(),
    ).await?;

    let mut vehicles = Vec::with_capacity(results.len());

    for (vehicle_id, lon, lat) in results {
        // For speed we use a placeholder 0.0 for now to avoid MGET (optimization for the future)
        // Or add a GET request if desired
        let speed = 0.0;

        vehicles.push(VehicleData {
            id: vehicle_id,
            lat,
            lon,
            speed,
        });
    }

    Ok(vehicles)
}