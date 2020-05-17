pub mod tonic;

#[cfg(not(debug_assertions))]
use std::str::FromStr;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;

pub struct Config {
    pub honeycomb_api_key: String,
    pub honeycomb_dataset: String,
    pub service_name: &'static str,
}

#[allow(unused_variables)]
pub fn setup(config: Config) {
    let registry = tracing_subscriber::registry().with(LevelFilter::INFO);

    #[cfg(debug_assertions)]
    let registry = registry.with(tracing_subscriber::fmt::layer());

    #[cfg(not(debug_assertions))]
    let registry = registry.with({
        let honeycomb_config = libhoney::Config {
            options: libhoney::client::Options {
                api_key: config.honeycomb_api_key,
                dataset: config.honeycomb_dataset,
                ..libhoney::client::Options::default()
            },
            transmission_options: libhoney::transmission::Options::default(),
        };
        tracing_honeycomb::new_honeycomb_telemetry_layer(config.service_name, honeycomb_config)
    });

    registry.init();
}

#[cfg(debug_assertions)]
pub fn register_new_tracing_root() {}

#[cfg(debug_assertions)]
pub fn register_tracing_root(_trace_id: &str, _parent_span_id: &str) {}

#[cfg(not(debug_assertions))]
pub fn register_new_tracing_root() {
    tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::generate(), None)
        .expect("register tracing root");
}

#[cfg(not(debug_assertions))]
pub fn register_tracing_root(trace_id: &str, parent_span_id: &str) {
    tracing_honeycomb::register_dist_tracing_root(
        tracing_honeycomb::TraceId::from_str(trace_id).expect("parse traceid"),
        Some(tracing_honeycomb::SpanId::from_str(parent_span_id).expect("parse spanid")),
    )
    .expect("register tracing root");
}

#[cfg(debug_assertions)]
pub async fn flush() {}

#[cfg(not(debug_assertions))]
pub async fn flush() {
    // libhoney-rust batches events and has a default batch timeout of 100ms
    //   https://github.com/nlopes/libhoney-rust/blob/3acdc4021d08a9b78653c77bb4ff3dab3e2b9556/src/transmission.rs#L33
    // It provides Client::flush() but this is not exposed by tracing-honeycomb.
    tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
}

pub const HEADER_TRACE_ID: &str = "x-b3-traceid";
pub const HEADER_PARENT_SPAN_ID: &str = "x-b3-parentspanid";
