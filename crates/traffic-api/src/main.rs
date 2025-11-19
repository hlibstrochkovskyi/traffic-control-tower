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
use common::map::load_map; // Импортируем ТОЛЬКО функцию загрузки
use tower_http::cors::CorsLayer;
use serde::Serialize; // Нужен для сериализации Road

// --- СТРУКТУРЫ ДАННЫХ ---

// Описываем, как выглядит дорога для Фронтенда
#[derive(Serialize, Clone)]
struct Road {
    id: u64,
    // glam::DVec2 сериализуется как [x, y], что и нужно нашему исправленному фронту
    geometry: Vec<glam::DVec2>,
}

struct AppState {
    tx: broadcast::Sender<String>,
    map_points: Vec<Road>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_tracing("traffic-api");
    info!("🗺️ Loading map for API...");

    // Загрузка карты
    let map_points = match load_map("crates/traffic-sim/assets/berlin.osm.pbf") {
        Ok(map) => {
            info!("✅ API Map loaded: {} roads", map.graph.edge_count());
            // Конвертируем граф в простой список дорог для JSON
            map.graph.edge_references().map(|e| {
                Road {
                    id: e.id().index() as u64,
                    geometry: e.weight().geometry.clone(),
                }
            }).collect()
        },
        Err(e) => {
            error!("❌ Failed to load map: {}", e);
            vec![]
        }
    };

    let (tx, _rx) = broadcast::channel(100);

    let shared_state = Arc::new(AppState {
        tx: tx.clone(),
        map_points,
    });

    // Redis Listener
    let state_clone = shared_state.clone();
    tokio::spawn(async move {
        subscribe_redis(state_clone).await;
    });

    // Роутер
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
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

async fn get_map(State(state): State<Arc<AppState>>) -> Json<Vec<Road>> {
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

    let mut con = match client.get_async_connection().await {
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

    use futures_util::StreamExt;
    while let Some(msg) = pubsub.on_message().next().await {
        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let _ = state.tx.send(payload);
    }
}