//! Generates the Meshtastic protobuf types from the definitions in `proto/`.
//!
//! The files are compiled with protox, a protobuf compiler written in Rust, so
//! building meshurl does not require `protoc` to be installed.

use std::path::PathBuf;

/// The definitions meshurl needs; protox pulls in whatever they import.
const PROTO_FILES: &[&str] = &[
    "meshtastic/apponly.proto",
    "meshtastic/channel.proto",
    "meshtastic/config.proto",
    "meshtastic/mesh.proto",
];

/// Root the imports resolve against, so `import "meshtastic/x.proto"` works.
const PROTO_ROOT: &str = "proto";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed={}", PROTO_ROOT);

    let descriptors = protox::compile(PROTO_FILES, [PROTO_ROOT])?;

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_fds(descriptors)?;

    Ok(())
}
