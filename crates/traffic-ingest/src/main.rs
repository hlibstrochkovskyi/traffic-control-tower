use traffic_common::{Config, VehiclePosition, init_tracing};
use rdkafka::consumer::{Consumer, StreamConsumer, CommitMode};
use rdkafka::config::ClientConfig;
use rdkafka::Message;
use futures::StreamExt;
use anyhow::{Context, Result};
use prost::Message as ProstMessage;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing("traffic-ingest");
    let config = Config::from_env()?;

    // 1. Создаем Consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("group.id", "ingest-group-2") // Важно: ID группы потребителей
        .set("auto.offset.reset", "earliest") // Читать с начала, если нет истории
        .set("enable.auto.commit", "false")   // Мы будем подтверждать вручную
        .create()
        .context("Failed to create Kafka consumer")?;

    // 2. Подписываемся на топик
    consumer.subscribe(&["raw-telemetry"])?;
    tracing::info!("Subscribed to 'raw-telemetry', waiting for messages...");

    // 3. Настройка Graceful Shutdown
    let shutdown = async {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        tracing::warn!("Received shutdown signal");
    };

    // 4. Главный цикл обработки
    tokio::select! {
        _ = consume_messages(&consumer) => {},
        _ = shutdown => {
            tracing::info!("Shutting down...");
        }
    }

    Ok(())
}

async fn consume_messages(consumer: &StreamConsumer) {
    let mut stream = consumer.stream();

    while let Some(message_result) = stream.next().await {
        match message_result {
            Ok(msg) => {
                // Пытаемся достать данные (payload)
                if let Some(payload) = msg.payload() {
                    // Десериализуем из Protobuf
                    match VehiclePosition::decode(payload) {
                        Ok(pos) => {
                            tracing::info!(
                                "Received: Car {} at ({:.4}, {:.4}) speed {:.1}", 
                                pos.vehicle_id, pos.latitude, pos.longitude, pos.speed
                            );
                            // Подтверждаем, что обработали
                            let _ = consumer.commit_message(&msg, CommitMode::Async);
                        },
                        Err(e) => tracing::error!("Failed to decode protobuf: {}", e),
                    }
                }
            },
            Err(e) => tracing::error!("Kafka error: {}", e),
        }
    }
}