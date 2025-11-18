use anyhow::{Context, Result};
use serde::Deserialize;

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

fn default_kafka_brokers() -> String {
    "localhost:19092".to_string()
}

fn default_postgres_url() -> String {
    "postgres://postgres:password@localhost:5432/traffic".to_string()
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();
        // Parse environment variables into the Config struct
        envy::from_env().context("Failed to load config from environment")
    }
}