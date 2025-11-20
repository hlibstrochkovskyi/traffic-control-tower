//! Configuration management for the traffic control system.
//!
//! This module provides configuration loading from environment variables
//! with sensible defaults for development environments.

use anyhow::{Context, Result};
use serde::Deserialize;

/// Main configuration structure for the traffic control system.
///
/// All fields can be overridden via environment variables. If not provided,
/// default values suitable for local development will be used.
///
/// # Environment Variables
///
/// - `KAFKA_BROKERS`: Kafka broker addresses (default: "localhost:19092")
/// - `POSTGRES_URL`: PostgreSQL connection URL (default: local instance)
/// - `REDIS_URL`: Redis connection URL (default: "redis://localhost:6379")
/// - `LOG_LEVEL`: Logging verbosity level (default: "info")
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_kafka_brokers")]
    pub kafka_brokers: String,

    #[serde(default = "default_postgres_url")]
    pub postgres_url: String,

    #[serde(default = "default_redis_url")]
    pub redis_url: String,

    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// Returns the default Kafka brokers address for local development.
fn default_kafka_brokers() -> String {
    "localhost:19092".to_string()
}

/// Returns the default PostgreSQL connection URL for local development.
fn default_postgres_url() -> String {
    "postgres://postgres:password@localhost:5432/traffic".to_string()
}

/// Returns the default Redis connection URL for local development.
fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

/// Returns the default log level.
fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    /// Loads configuration from environment variables.
    ///
    /// Attempts to load a `.env` file if present, then parses environment
    /// variables into the Config structure. Missing variables will use
    /// their default values.
    ///
    /// # Returns
    ///
    /// A `Config` instance populated from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if environment variables are malformed or cannot
    /// be parsed into the expected types.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use traffic_common::Config;
    ///
    /// let config = Config::from_env().expect("Failed to load config");
    /// println!("Kafka brokers: {}", config.kafka_brokers);
    /// ```
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        envy::from_env().context("Failed to load config from environment")
    }
}