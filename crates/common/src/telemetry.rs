//! Telemetry and observability initialization.
//!
//! This module provides utilities for setting up structured logging and tracing
//! across all services in the traffic control system.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes the tracing subscriber for structured logging.
///
/// Sets up a global tracing subscriber with configurable log levels via the
/// `RUST_LOG` environment variable. The logger includes contextual information
/// such as targets, thread IDs, file names, and line numbers.
///
/// # Arguments
///
/// * `service_name` - Name of the service being initialized (used for logging)
///
/// # Log Format
///
/// By default, uses human-readable formatting. For production environments,
/// consider uncommenting the `.json()` option for structured JSON logs.
///
/// # Environment Variables
///
/// * `RUST_LOG` - Controls log level filtering (defaults to "info" if not set)
///   Examples: "debug", "trace", "my_crate=debug"
///
/// # Examples
///
/// ```no_run
/// use traffic_common::init_tracing;
///
/// init_tracing("traffic-api");
/// // Logs will now include service startup message
/// ```
///
/// # Panics
///
/// May panic if another global subscriber has already been set.
pub fn init_tracing(service_name: &str) {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
            // In production, you might want to use .json() instead of pretty print
            // .json()
        )
        .init();

    tracing::info!("Starting service: {}", service_name);
}