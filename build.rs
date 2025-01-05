use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .build_server(false)
        .out_dir(out_dir)
        .compile(
            &[
                "proto/jito-protos/block_engine.proto",
                "proto/jito-protos/bundle.proto",
                "proto/jito-protos/packet.proto",
                "proto/jito-protos/shared.proto",
                "proto/jito-protos/searcher.proto",
            ],
            &["proto/jito-protos"],
        )?;
    Ok(())
} 