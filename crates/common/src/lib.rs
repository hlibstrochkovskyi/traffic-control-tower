//! Common library for traffic control tower system.
//!
//! This crate provides shared functionality across all traffic-control-tower services,
//! including Protocol Buffers definitions, configuration management, error handling,
//! telemetry utilities, and map-related operations.

// Protocol Buffers module containing generated types from telemetry.proto
pub mod proto;

// Re-export all Protocol Buffers types for convenient access
pub use proto::*;

// Configuration management
pub mod config;
pub use config::Config;

// Error handling types
pub mod error;
pub use error::{Result, TrafficError};

// Telemetry and observability
pub mod telemetry;

// Map and geographic data operations
pub mod map;

pub use telemetry::init_tracing;