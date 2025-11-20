//! Traffic Ingest Service - Kafka consumer for vehicle telemetry.
//!
//! This service consumes vehicle position messages from Kafka and implements
//! a dual-path architecture:
//! - **Cold Path**: Batches data to TimescaleDB for historical analysis
//! - **Hot Path**: Updates Redis with real-time vehicle locations and publishes
//!   updates to connected clients via pub/sub

mod batch;

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
use redis::AsyncCommands;

/// Main ingestion service handling both database writes and Redis updates.
struct IngestService {
    /// Batched writer for efficient TimescaleDB inserts
    batch_writer: BatchWriter,
    /// Redis connection for real-time geospatial indexing and pub/sub
    redis: redis::aio::ConnectionManager,
}

impl IngestService {
    /// Creates a new IngestService with database and Redis connections.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration with connection URLs
    ///
    /// # Returns
    ///
    /// An initialized `IngestService` ready to process vehicle positions.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - PostgreSQL connection fails
    /// - Redis connection cannot be established
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

    /// Processes a single vehicle position through both cold and hot paths.
    ///
    /// # Cold Path (Historical Storage)
    /// - Adds position to batch buffer for TimescaleDB
    /// - Data is flushed periodically for efficient bulk inserts
    ///
    /// # Hot Path (Real-Time Updates)
    /// - Updates Redis geospatial index for proximity queries
    /// - Stores vehicle metadata (speed, timestamp) with TTL
    /// - Publishes update to "vehicles:update" channel for WebSocket clients
    ///
    /// # Arguments
    ///
    /// * `position` - Vehicle position telemetry data
    ///
    /// # Errors
    ///
    /// Returns an error if database or Redis operations fail.
    async fn process(&mut self, position: VehiclePosition) -> Result<()> {
        // 1. Cold Path: Accumulate batch for TimescaleDB
        self.batch_writer.add(position.clone()).await?;

        // 2. Hot Path: Update Redis Geo Index for proximity searches
        let _: () = self.redis.geo_add(
            "vehicles:current",
            (position.longitude, position.latitude, &position.vehicle_id)
        ).await?;

        // 3. Store metadata (speed) with 60-second TTL
        let metadata = serde_json::json!({
            "speed": position.speed,
            "timestamp": position.timestamp
        });

        let _: () = self.redis.set_ex(
            format!("vehicle:{}:meta", position.vehicle_id),
            metadata.to_string(),
            60
        ).await?;

        // 4. Publish update to WebSocket clients via Redis pub/sub
        let payload = serde_json::json!({
            "id": position.vehicle_id,
            "lat": position.latitude,
            "lon": position.longitude,
            "speed": position.speed
        }).to_string();

        let _: () = self.redis.publish("vehicles:update", payload).await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing("traffic-ingest");
    let config = Config::from_env()?;

    let mut service = IngestService::new(&config).await?;

    // Configure Kafka consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("group.id", "ingest-group-final")
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()
        .context("Failed to create Kafka consumer")?;

    consumer.subscribe(&["raw-telemetry"])?;
    tracing::info!("Ingest Service Started: Writing to DB (Batch=100) & Redis");

    let mut stream = consumer.stream();
    let shutdown = signal::ctrl_c();

    // Main processing loop with graceful shutdown
    tokio::select! {
        _ = async {
            while let Some(msg_result) = stream.next().await {
                if let Ok(msg) = msg_result {
                    if let Some(payload) = msg.payload() {
                        if let Ok(pos) = VehiclePosition::decode(payload) {
                            // Process vehicle position
                            if let Err(e) = service.process(pos).await {
                                tracing::error!("Processing error: {}", e);
                            }
                            // Acknowledge message processing
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