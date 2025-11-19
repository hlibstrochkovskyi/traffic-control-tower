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

#[derive(Clone)]
struct AppState {
    redis: ConnectionManager,
}

#[derive(Deserialize, Debug)] // Добавил Debug для логирования
struct ViewportParams {
    lat: f64,
    lon: f64,
    radius_km: f64,
}

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

    let client = redis::Client::open(config.redis_url.as_str())?;
    let redis = client.get_tokio_connection_manager().await?;

    let state = AppState { redis };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(|| async { "OK" }))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any),
        )
        .with_state(Arc::new(state));

    let addr = "0.0.0.0:3000";
    tracing::info!("API listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<ViewportParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!("🔌 New client connected: {:?}", params);
    ws.on_upgrade(move |socket| handle_socket(socket, state, params))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, viewport: ViewportParams) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
    let mut redis = state.redis.clone();

    loop {
        interval.tick().await;

        match fetch_vehicles_in_viewport(&mut redis, &viewport).await {
            Ok(vehicles) => {
                // Логируем только если нашли машины, чтобы не спамить
                if !vehicles.is_empty() {
                    tracing::info!("📨 Sending {} vehicles to client", vehicles.len());
                }
                // Если 0 машин, логируем раз в 5 секунд (примерно), иначе консоль взорвется
                // (но для теста пока оставим как есть или можно смотреть на "Found 0" ниже)

                let json = serde_json::to_string(&vehicles).unwrap_or_default();
                if socket.send(Message::Text(json)).await.is_err() {
                    tracing::warn!("❌ Client disconnected");
                    break;
                }
            }
            Err(e) => {
                tracing::error!("❌ Redis error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}

async fn fetch_vehicles_in_viewport(
    redis: &mut ConnectionManager,
    viewport: &ViewportParams,
) -> Result<Vec<VehicleData>> {
    tracing::debug!(
        "🔍 GEORADIUS key='vehicles:current' lon={} lat={} rad={}km",
        viewport.lon,
        viewport.lat,
        viewport.radius_km
    );

    // Используем сырой запрос redis::cmd, чтобы точно контролировать ответ
    // GEORADIUS возвращает сложную структуру: [ [name, [lon, lat]], ... ]
    // Библиотека redis-rs иногда путается в типах, поэтому парсим вручную.

    let raw_results: Vec<redis::Value> = redis::cmd("GEORADIUS")
        .arg("vehicles:current")
        .arg(viewport.lon)
        .arg(viewport.lat)
        .arg(viewport.radius_km)
        .arg("km")
        .arg("WITHCOORD")
        .query_async(redis)
        .await?;

    let mut vehicles = Vec::with_capacity(raw_results.len());

    for item in raw_results {
        // Парсим каждый элемент ответа [name, [lon, lat]]
        if let redis::Value::Bulk(items) = item {
            if items.len() >= 2 {
                // 1. Получаем ID
                let id_val = &items[0];
                let id: String = redis::from_redis_value(id_val)?;

                // 2. Получаем Координаты (это вложенный Bulk)
                let coords_val = &items[1];
                if let redis::Value::Bulk(coords) = coords_val {
                    if coords.len() >= 2 {
                        let lon: f64 = redis::from_redis_value(&coords[0])?;
                        let lat: f64 = redis::from_redis_value(&coords[1])?;

                        vehicles.push(VehicleData {
                            id,
                            lat,
                            lon,
                            speed: 15.0, // Заглушка
                        });
                    }
                }
            }
        }
    }

    if vehicles.is_empty() {
        tracing::warn!("⚠️ Found 0 vehicles (parsed).");
    } else {
        tracing::info!("✅ Successfully parsed {} vehicles", vehicles.len());
    }

    Ok(vehicles)
}