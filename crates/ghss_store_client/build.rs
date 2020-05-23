fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store_proto = "../ghss_store/proto/store.proto";
    let query_proto = "../ghss_store/proto/query.proto";
    tonic_build::configure()
        .build_server(false)
        .compile(&[store_proto, query_proto], &["../ghss_store/proto"])?;
    println!("cargo:rerun-if-changed={}", store_proto);
    println!("cargo:rerun-if-changed={}", query_proto);
    Ok(())
}
