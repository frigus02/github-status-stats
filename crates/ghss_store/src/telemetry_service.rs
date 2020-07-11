use hyper::{Body, Request, Response};
use opentelemetry::api::{
    trace::futures::WithContext, Context as OtelContext, FutureExt, Key, SpanKind, TraceContextExt,
    Tracer,
};
use std::task::{Context, Poll};
use tonic::{body::BoxBody, transport::NamedService};
use tower::Service;

#[derive(Debug, Clone)]
pub struct WithTelemetry<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for WithTelemetry<S>
where
    S: Service<Request<Body>, Response = Response<BoxBody>> + NamedService + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = WithContext<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let remote_cx = opentelemetry::global::get_http_text_propagator(|propagator| {
            propagator.extract(req.headers())
        });
        let tracer = opentelemetry::global::tracer("store");
        let span = tracer.build_with_context(
            tracer
                .span_builder(req.uri().path().get(1..).unwrap_or("INVALID PATH"))
                .with_kind(SpanKind::Server)
                .with_attributes(vec![
                    Key::new("rpc.system").string("grpc"),
                    Key::new("rpc.service").string(Self::NAME),
                    Key::new("rpc.method")
                        .string(req.uri().path().get(Self::NAME.len() + 2..).unwrap_or("")),
                ]),
            &remote_cx,
        );
        self.inner
            .call(req)
            .with_context(OtelContext::current_with_span(span))
    }
}

impl<S: NamedService> NamedService for WithTelemetry<S> {
    const NAME: &'static str = S::NAME;
}

pub trait TelemetryServiceExt: Sized {
    fn with_telemetry(self) -> WithTelemetry<Self> {
        WithTelemetry { inner: self }
    }
}

impl<T: Sized> TelemetryServiceExt for T {}
