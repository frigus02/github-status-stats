fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(false)
        .compile(&["proto/store.proto", "proto/query.proto"], &["proto"])?;
    Ok(())
}
