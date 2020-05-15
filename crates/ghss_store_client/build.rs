fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(false).compile(
        &["../ghss_store/proto/store.proto", "../ghss_store/proto/query.proto"],
        &["../ghss_store/proto"],
    )?;
    Ok(())
}
