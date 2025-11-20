use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, error, warn}; // Добавили warn
use common::{telemetry, Config}; // Добавили Config
use common::map::RoadGraph;
use tower_http::cors::CorsLayer;
use serde::Serialize;
use futures_util::StreamExt;

#[derive(Serialize, Clone)]
struct Road {
    id: u64,
    geometry: Vec<[f64; 2]>,
}

struct AppState {
    tx: broadcast::Sender<String>,
    map_points: Vec<Road>,
    total_roads: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_tracing("traffic-api");

    // 1. Загружаем конфиг (чтобы брать правильный URL Redis)
    let config = Config::from_env().unwrap_or_else(|e| {
        warn!("Failed to load config: {}. Using defaults.", e);
        Config {
            kafka_brokers: "localhost:19092".to_string(),
            postgres_url: "".to_string(),
            redis_url: "redis://localhost:6379".to_string(), // Используем localhost как в Ingest
            log_level: "info".to_string(),
        }
    });

    info!("🗺️ Loading map for API...");

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

    // Без лимита .take(10000), грузим всё!
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

    let (tx, _rx) = broadcast::channel(1000); // Увеличим буфер на всякий случай

    let shared_state = Arc::new(AppState {
        tx: tx.clone(),
        map_points,
        total_roads,
    });

    // Запускаем Redis Listener с конфигом
    let state_clone = shared_state.clone();
    let redis_url = config.redis_url.clone();
    tokio::spawn(async move {
        subscribe_redis(state_clone, redis_url).await;
    });

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
    info!("🔌 New WebSocket client connected");

    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg)).await.is_err() {
            // Client disconnected
            break;
        }
    }
}

// Исправленная функция подписки
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

        // Отправляем в сокеты
        // Если нет подписчиков, send вернет ошибку, это нормально, игнорируем
        let _ = state.tx.send(payload);
    }

    error!("❌ Redis connection lost!");
}