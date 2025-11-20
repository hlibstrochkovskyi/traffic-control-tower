# --- Stage 1: Builder (Compilation) ---
# Using latest to support newer libraries (edition 2024)
FROM rust:latest as builder

WORKDIR /app

# Install system dependencies
# [FIX] Added protobuf-compiler, which is required for compiling .proto files
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    g++ \
    zlib1g-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests and source code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY proto ./proto

# Build all binaries in release mode
# This is the longest step
RUN cargo build --release --workspace

# --- Stage 2: Runtime (Execution) ---
# Using lightweight image for the final container
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (SSL certificates and libraries)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binaries from builder stage
COPY --from=builder /app/target/release/traffic-sim /usr/local/bin/
COPY --from=builder /app/target/release/traffic-ingest /usr/local/bin/
COPY --from=builder /app/target/release/traffic-api /usr/local/bin/

# Copy map data (required, as it's needed by the simulator)
# Create folder structure to match the path referenced in code
COPY crates/traffic-sim/assets /app/crates/traffic-sim/assets

# Set default logging level
ENV RUST_LOG=info

# CMD not specified, as it will be overridden in docker-compose.yml for each service