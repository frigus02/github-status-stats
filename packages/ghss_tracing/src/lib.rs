use tracing::Subscriber;
use tracing_log::LogTracer;

pub use tracing::{
    debug, debug_span, error, error_span, field, field::Empty as EmptyField, info, info_span,
    instrument, span::Span, trace, trace_span, warn, warn_span,
};
pub use tracing_futures::Instrument;

pub struct Config {
    pub honeycomb_api_key: String,
    pub honeycomb_dataset: String,
    pub service_name: String,
}

pub fn setup(config: Config) {
    LogTracer::builder()
        .with_max_level(log::LevelFilter::Info)
        .init()
        .expect("initializing log tracer failed");

    let subscriber = create_subscriber(config);
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting global default tracing subscriber failed");
}

#[cfg(debug_assertions)]
pub async fn flush() {}

#[cfg(not(debug_assertions))]
pub async fn flush() {
    // libhoney-rust batches events and has a default batch timeout of 100ms
    //   https://github.com/nlopes/libhoney-rust/blob/3acdc4021d08a9b78653c77bb4ff3dab3e2b9556/src/transmission.rs#L33
    // It provides Client::flush() but this is not exposed by honeycomb-tracing.
    tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
}

#[cfg(debug_assertions)]
fn create_subscriber(_config: Config) -> impl Subscriber {
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing_subscriber::filter::LevelFilter::DEBUG)
        .finish()
}

#[cfg(not(debug_assertions))]
fn create_subscriber(config: Config) -> impl Subscriber {
    let honeycomb_config = libhoney::Config {
        options: libhoney::client::Options {
            api_key: config.honeycomb_api_key,
            dataset: config.honeycomb_dataset,
            ..libhoney::client::Options::default()
        },
        transmission_options: libhoney::transmission::Options::default(),
    };
    honeycomb_tracing::TelemetrySubscriber::new(config.service_name, honeycomb_config)
}
