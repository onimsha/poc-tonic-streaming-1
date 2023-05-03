fn main() {
    tonic_build::configure()
        .compile(&["proto/streaming.proto"], &["proto"])
        .unwrap();
}
