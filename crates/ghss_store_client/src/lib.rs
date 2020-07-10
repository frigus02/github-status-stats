use ghss_github::{CheckRun, CheckRunConclusion};
use opentelemetry::api::{Context, Key, SpanKind, TraceContextExt, Tracer};
use std::convert::TryInto;
pub use tonic::{transport::channel::Channel, Code, Request, Response, Status};

tonic::include_proto!("ghss.store");

pub fn request_context(span_name: &str) -> Context {
    let tracer = opentelemetry::global::tracer("store_client");
    let span = tracer
        .span_builder(span_name)
        .with_kind(SpanKind::Client)
        .with_attributes(vec![Key::new("rpc.system").string("grpc")])
        .start(&tracer);
    Context::current_with_span(span)
}

pub fn request<T>(message: T, cx: &Context) -> Request<T> {
    let mut request = Request::new(message);
    opentelemetry::global::get_http_text_propagator(|propagator| {
        propagator.inject_context(cx, request.metadata_mut())
    });
    request
}

impl From<CheckRun> for Build {
    fn from(check_run: CheckRun) -> Self {
        Self {
            name: check_run.name,
            source: BuildSource::CheckRun as i32,
            commit: check_run.head_sha,
            successful: match &check_run.conclusion {
                Some(conclusion) => conclusion == &CheckRunConclusion::Success,
                None => false,
            },
            failed: match &check_run.conclusion {
                Some(conclusion) => {
                    conclusion == &CheckRunConclusion::Failure
                        || conclusion == &CheckRunConclusion::TimedOut
                }
                None => false,
            },
            duration_ms: match check_run.completed_at {
                Some(completed_at) => (completed_at.timestamp_millis()
                    - check_run.started_at.timestamp_millis())
                .try_into()
                .expect("duration should fit into u32"),
                None => 0,
            },
            timestamp: check_run.started_at.timestamp_millis(),
        }
    }
}
