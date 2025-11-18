use thiserror::Error;

// Custom Result type alias for convenient use across the project
pub type Result<T> = std::result::Result<T, TrafficError>;

#[derive(Error, Debug)]
pub enum TrafficError {
    #[error("Kafka error: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] prost::DecodeError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),
}