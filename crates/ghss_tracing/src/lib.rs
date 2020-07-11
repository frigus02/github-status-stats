use opentelemetry::api::TraceContextPropagator;

pub fn init_tracer(
    service_name: &'static str,
    agent_endpoint: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
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
