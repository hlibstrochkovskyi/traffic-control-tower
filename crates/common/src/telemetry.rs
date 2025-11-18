use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

struct EnvFilter();

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