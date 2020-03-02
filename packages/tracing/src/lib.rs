use tracing::Subscriber;
use tracing_log::LogTracer;
use uuid::Uuid;

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
fn create_subscriber(_config: Config) -> impl Subscriber {
    tracing_subscriber::FmtSubscriber::new()
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

pub fn uuid() -> String {
    let mut buf = vec![0; uuid::adapter::Simple::LENGTH];
    Uuid::new_v4().to_simple().encode_lower(&mut buf);
    String::from_utf8(buf).expect("uuid produced values outside  UTF-8 range")
}
