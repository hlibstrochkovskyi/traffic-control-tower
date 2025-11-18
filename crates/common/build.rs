fn main() {
    // Setup the generator
    let mut config = prost_build::Config::new();

    // Compile the protos
    // We removed .out_dir() to let it use the default Cargo OUT_DIR
    config.compile_protos(
        &["../../proto/telemetry.proto"], // Path to .proto file
        &["../../proto/"]                 // Include path
    )
        .expect("Failed to compile protos");
}