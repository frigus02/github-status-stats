use opentelemetry::api::{Context, Key, TraceContextExt, TraceContextPropagator};
use std::error::Error;
use std::fmt;

type BoxError = Box<dyn Error + Send + Sync>;

pub fn init_tracer(
    service_name: &'static str,
    agent_endpoint: Option<&str>,
) -> Result<(), BoxError> {
    let provider = match agent_endpoint {
        Some(agent_endpoint) => {
            let exporter = opentelemetry_jaeger::Exporter::builder()
                .with_agent_endpoint(agent_endpoint.parse().unwrap())
                .with_process(opentelemetry_jaeger::Process {
                    service_name: service_name.into(),
                    tags: vec![],
                })
                .init()?;
            let batch = opentelemetry::sdk::BatchSpanProcessor::builder(
                exporter,
                tokio::spawn,
                tokio::time::interval,
            )
            .build();
            opentelemetry::sdk::Provider::builder()
                .with_batch_exporter(batch)
                .build()
        }
        None => {
            let exporter = opentelemetry::exporter::trace::stdout::Builder::default().init();
            opentelemetry::sdk::Provider::builder()
                .with_simple_exporter(exporter)
                .build()
        }
    };
    opentelemetry::global::set_provider(provider);

    let propagator = TraceContextPropagator::new();
    opentelemetry::global::set_http_text_propagator(propagator);

    Ok(())
}

pub fn log_event(message: String) {
    Context::current()
        .span()
        .add_event("log".into(), vec![Key::new("log.message").string(message)]);
}

#[derive(Debug)]
struct Exception<'a, 'b> {
    message: &'a str,
    inner: &'b dyn Error,
}

impl<'a, 'b> fmt::Display for Exception<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.message, self.inner)
    }
}

impl<'a, 'b> Error for Exception<'a, 'b> {}

pub fn error_event(message: &str, err: &dyn Error) {
    Context::current().span().record_exception(&Exception {
        message,
        inner: err,
    });
}
