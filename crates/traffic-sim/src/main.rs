use traffic_common::{Config, VehiclePosition, init_tracing};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::config::ClientConfig;
use prost::Message;
use anyhow::{Context, Result};
use tokio::signal;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Инициализация логов
    init_tracing("traffic-sim");

    // 2. Загрузка конфига
    let config = Config::from_env()?;

    // 3. Создание Kafka продюсера
    let producer = create_producer(&config.kafka_brokers)?;

    // 4. Настройка Graceful Shutdown
    let shutdown = async {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        tracing::warn!("Received shutdown signal");
    };

    // 5. Запуск цикла
    tracing::info!("Starting simulation loop...");
    tokio::select! {
        result = run_simulation(&producer) => {
            if let Err(e) = result {
                tracing::error!("Simulation error: {}", e);
            }
        }
        _ = shutdown => {
            tracing::info!("Shutting down gracefully...");
        }
    }

    Ok(())
}

fn create_producer(brokers: &str) -> Result<FutureProducer> {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create Kafka producer")
}

async fn run_simulation(producer: &FutureProducer) -> Result<()> {
    // Отправляем данные каждые 100мс
    let mut interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        interval.tick().await;

        // Генерируем случайную машину (ID от 0 до 99)
        let position = VehiclePosition {
            vehicle_id: format!("car_{}", rand::random::<u32>() % 100),
            latitude: 52.52 + rand::random::<f64>() * 0.01, // Где-то в Берлине
            longitude: 13.40 + rand::random::<f64>() * 0.01,
            speed: 30.0 + rand::random::<f64>() * 10.0,
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Сериализуем в Protobuf
        let mut buf = Vec::new();
        position.encode(&mut buf)?;

        // Отправляем в Kafka
        let record = FutureRecord::to("raw-telemetry")
            .payload(&buf)
            .key(&position.vehicle_id);

        match producer.send(record, Duration::from_secs(0)).await {
            Ok(_) => tracing::info!("Sent position for {}", position.vehicle_id),
            Err((e, _)) => tracing::error!("Failed to send message: {}", e),
        }
    }
}