//! Build script for compiling Protocol Buffers definitions.
//!
//! This script runs at compile time to generate Rust code from
//! the telemetry.proto file using prost-build.

fn main() {
    setup_proto_compilation();
}

/// Sets up and executes Protocol Buffers compilation.
///
/// Configures prost-build and compiles the telemetry.proto file,
/// generating Rust type definitions that will be available at compile time.
///
/// # Panics
///
/// Panics if the proto files cannot be compiled, which typically occurs when:
/// - The .proto file path is incorrect
/// - The proto file contains syntax errors
/// - Include paths are misconfigured
fn setup_proto_compilation() {
    let mut config = prost_build::Config::new();

    config
        .compile_protos(
            &["../../proto/telemetry.proto"],
            &["../../proto/"],
        )
        .expect("Failed to compile protos");
}