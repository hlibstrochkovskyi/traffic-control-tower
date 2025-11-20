use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, error};
use common::telemetry;
use common::map::RoadGraph;
use tower_http::cors::CorsLayer;
use serde::Serialize;
use futures_util::StreamExt;

// Структура дороги для фронтенда
#[derive(Serialize, Clone)]
struct Road {
    id: u64,
    geometry: Vec<[f64; 2]>, // [lon, lat]
}

struct AppState {
    tx: broadcast::Sender<String>,
    map_points: Vec<Road>,
    total_roads: usize,  // Store total roads count
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_tracing("traffic-api");
    info!("🗺️ Loading map for API...");

    // Загрузка карты через правильную функцию
    let road_graph = match RoadGraph::load_from_pbf("crates/traffic-sim/assets/berlin.osm.pbf") {
        Ok(graph) => {
            info!("✅ API Map loaded: {} roads", graph.edges.len());
            graph
        },
        Err(e) => {
            error!("❌ Failed to load map: {}", e);
            RoadGraph::default() // Пустая карта, если не загрузилась
        }
    };

    let total_roads = road_graph.edges.len();

    // Конвертируем дороги в формат для фронтенда
    // Filter to only major roads for better performance
    let map_points: Vec<Road> = road_graph.edges
        .iter()
        .filter(|road| {
            // Filter to show only major roads for better performance and clarity
            matches!(
                road.highway_type.as_str(),
                "motorway" | "trunk" | "primary" | "secondary" | "tertiary"
            )
        })
        .take(10000) // Limit to 10000 major road segments
        .enumerate()
        .map(|(_idx, road)| Road {
            id: road.id as u64,
            geometry: road.geometry
                .iter()
                .map(|point| [point.x, point.y]) // DVec2 -> [lon, lat]
                .collect(),
        })
        .collect();

    info!("📊 Prepared {} road segments for frontend (from {} total)", 
          map_points.len(), road_graph.edges.len());

    let (tx, _rx) = broadcast::channel(100);

    let shared_state = Arc::new(AppState {
        tx: tx.clone(),
        map_points,
        total_roads,
    });

    // Redis Listener
    let state_clone = shared_state.clone();
    tokio::spawn(async move {
        subscribe_redis(state_clone).await;
    });

    // Роутер
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

// --- ХЕНДЛЕРЫ ---

#[derive(Serialize)]
struct HealthStatus {
    status: String,
    map_loaded: bool,
    total_roads: usize,
    visible_roads: usize,
}

async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthStatus> {
    Json(HealthStatus {
        status: "OK".to_string(),
        map_loaded: state.total_roads > 0,
        total_roads: state.total_roads,
        visible_roads: state.map_points.len(),
    })
}

async fn get_map(State(state): State<Arc<AppState>>) -> Json<Vec<Road>> {
    info!("📍 Map requested, sending {} road segments", state.map_points.len());
    Json(state.map_points.clone())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg)).await.is_err() {
            break;
        }
    }
}

async fn subscribe_redis(state: Arc<AppState>) {
    let client = match redis::Client::open("redis://127.0.0.1:6379/") {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create Redis client: {}", e);
            return;
        }
    };

    let con = match client.get_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to connect to Redis: {}", e);
            return;
        }
    };

    let mut pubsub = con.into_pubsub();
    if let Err(e) = pubsub.subscribe("vehicles:update").await {
        error!("Failed to subscribe to channel: {}", e);
        return;
    }

    while let Some(msg) = pubsub.on_message().next().await {
        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let _ = state.tx.send(payload);
    }
}