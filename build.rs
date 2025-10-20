use std::env;

fn main() {
    let is_proto_build = env::var("CARGO_FEATURE_PROTO_BUILD").is_ok();
    if  is_proto_build {
        // cargo test --features proto_build
        println!("cargo:warning=Build script logic IS RUNNING (feature active or test build).");

        prost_build::compile_protos(
            &["src/proto/common.proto"],             // proto file path
            &["src/proto"],                          // proto dir
        ).unwrap();
    } else {
        // default (cargo build --release)
        println!("cargo:warning=Build script logic IS SKIPPED (default build).");
    }
}