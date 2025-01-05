use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .build_server(false)
        .out_dir(out_dir)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile(
            &[
                "proto/jito-protos/proto/block_engine.proto",
                "proto/jito-protos/proto/bundle.proto",
                "proto/jito-protos/proto/packet.proto",
                "proto/jito-protos/proto/shared.proto",
            ],
            &["proto/jito-protos/proto"],
        )?;
    Ok(())
} 