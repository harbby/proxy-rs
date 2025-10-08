fn main() {
    prost_build::compile_protos(
        &["src/proto/common.proto"],             // proto file path
        &["src/proto"],                          // proto dir
    ).unwrap();
}