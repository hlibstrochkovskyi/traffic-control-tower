use axum::{
    extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
    routing::get,
    Router,
    Json,
};
// Убрали лишние импорты, чтобы не было варнингов
use futures::{sink::SinkExt, stream::StreamExt};
use std::{sync::Arc, net::SocketAddr};
use tokio::sync::broadcast;
use tokio::net::TcpListener; // <--- НУЖНО ДЛЯ AXUM 0.7
use traffic_common::{Config, init_tracing};
use traffic_common::map::{RoadGraph, Road};
use anyhow::Result;

// Состояние приложения
struct AppState {
    redis_client: redis::Client,
    tx: broadcast::Sender<String>,
    map: Arc<RoadGraph>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing("traffic-api");
    let config = Config::from_env()?;

    // 1. Подключение к Redis
    let client = redis::Client::open(config.redis_url.as_str())?;

    // 2. Загрузка Карты
    let map_path = "crates/traffic-sim/assets/berlin.osm.pbf";
    tracing::info!("🗺️ Loading map for API...");
    let graph = RoadGraph::load_from_pbf(map_path)?;
    tracing::info!("✅ API Map loaded: {} roads", graph.edges.len());

    // 3. Канал для WebSocket
    let (tx, _rx) = broadcast::channel(100);

    // 4. Состояние
    let app_state = Arc::new(AppState {
        redis_client: client,
        tx: tx.clone(),
        map: Arc::new(graph),
    });

    // 5. Роутер
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler))
        .route("/map", get(get_map_geometry))
        .with_state(app_state.clone()); // Клонируем Arc для передачи

    // 6. Запуск сервера (СИНТАКСИС AXUM 0.7)
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("🚀 API listening on {}", addr);

    // Запускаем чтение Redis в фоне
    let redis_clone = app_state.redis_client.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        listen_redis_updates(redis_clone, tx_clone).await;
    });

    // В версии 0.7 используем TcpListener и axum::serve
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// --- Handlers ---

async fn health_check() -> &'static str {
    "OK"
}

// Ручка для получения карты
async fn get_map_geometry(State(state): State<Arc<AppState>>) -> Json<Vec<Road>> {
    Json(state.map.edges.clone())
}

// WebSocket
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    let (mut sender, _receiver) = socket.split();

    while let Ok(msg) = rx.recv().await {
        if sender.send(Message::Text(msg)).await.is_err() {
            break;
        }
    }
}

// Redis Listener
async fn listen_redis_updates(client: redis::Client, tx: broadcast::Sender<String>) {
    // Используем get_connection_manager, так как get_async_connection иногда отваливается при разрывах
    // Но для простоты оставим пока get_multiplexed_async_connection или просто создадим соединение
    let mut con = client.get_async_connection().await.expect("Redis connect failed");
    let mut pubsub = con.into_pubsub();
    pubsub.subscribe("traffic_updates").await.expect("Subscribe failed");

    while let Some(msg) = pubsub.on_message().next().await {
        if let Ok(payload) = msg.get_payload::<String>() {
            let _ = tx.send(payload);
        }
    }
}