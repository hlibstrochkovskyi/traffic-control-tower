mod batch; // Include the created file

use traffic_common::{Config, VehiclePosition, init_tracing};
use rdkafka::consumer::{Consumer, StreamConsumer, CommitMode};
use rdkafka::config::ClientConfig;
use rdkafka::Message;
use futures::StreamExt;
use anyhow::{Context, Result};
use prost::Message as ProstMessage;
use tokio::signal;
use sqlx::PgPool;
use crate::batch::BatchWriter;
use redis::AsyncCommands; // For working with Redis

struct IngestService {
    batch_writer: BatchWriter,
    redis: redis::aio::ConnectionManager,
}

impl IngestService {
    async fn new(config: &Config) -> Result<Self> {
        // Connect to Postgres
        let pool = PgPool::connect(&config.postgres_url).await
            .context("Failed to connect to Postgres")?;
        // Batch size 100 for testing (to see logs quicker); in production use 1000+
        let batch_writer = BatchWriter::new(pool, 100);

        // Connect to Redis
        let client = redis::Client::open(config.redis_url.as_str())
            .context("Invalid Redis URL")?;
        let redis = client.get_tokio_connection_manager().await
            .context("Failed to connect to Redis")?;

        Ok(Self { batch_writer, redis })
    }

    async fn process(&mut self, position: VehiclePosition) -> Result<()> {
        // 1. Cold Path: Accumulate a batch for the DB
        self.batch_writer.add(position.clone()).await?;

        // 2. Hot Path: Instantly update Redis
        // Geo-index for the map
        let _: () = self.redis.geo_add(
            "vehicles:current",
            (position.longitude, position.latitude, &position.vehicle_id)
        ).await?;

        // Metadata (speed) with TTL 60 seconds
        let metadata = serde_json::json!({
            "speed": position.speed,
            "timestamp": position.timestamp
        });
        let _: () = self.redis.set_ex(
            format!("vehicle:{}:meta", position.vehicle_id),
            metadata.to_string(),
            60
        ).await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing("traffic-ingest");
    let config = Config::from_env()?;

    let mut service = IngestService::new(&config).await?;

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("group.id", "ingest-group-final") // New consumer group to read messages from the start again
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()
        .context("Failed to create Kafka consumer")?;

    consumer.subscribe(&["raw-telemetry"])?;
    tracing::info!("Ingest Service Started: Writing to DB (Batch=100) & Redis");

    let mut stream = consumer.stream();
    let shutdown = signal::ctrl_c();

    tokio::select! {
        _ = async {
            while let Some(msg_result) = stream.next().await {
                if let Ok(msg) = msg_result {
                    if let Some(payload) = msg.payload() {
                        if let Ok(pos) = VehiclePosition::decode(payload) {
                            // Processing
                            if let Err(e) = service.process(pos).await {
                                tracing::error!("Processing error: {}", e);
                            }
                            // Acknowledge processing
                            let _ = consumer.commit_message(&msg, CommitMode::Async);
                        }
                    }
                }
            }
        } => {},
        _ = shutdown => {
            tracing::info!("Shutdown signal received. Flushing DB buffer...");
            if let Err(e) = service.batch_writer.flush().await {
                tracing::error!("Flush error: {}", e);
            }
            tracing::info!("Shutdown complete.");
        }
    }

    Ok(())
}