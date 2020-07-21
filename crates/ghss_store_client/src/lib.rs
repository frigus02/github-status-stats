use ghss_github::{CheckRun, CheckRunConclusion};
use opentelemetry::api::{
    Carrier, Context, FutureExt, Key, SpanKind, StatusCode, TraceContextExt, Tracer,
};
pub use proto::*;
use std::convert::TryInto;
use tonic::{transport::channel::Channel, Request};
pub use tonic::{Code, Response, Status};

mod proto {
    tonic::include_proto!("ghss.store");
}

struct TonicMetadataMapCarrier<'a>(&'a mut tonic::metadata::MetadataMap);
impl<'a> Carrier for TonicMetadataMapCarrier<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|metadata| metadata.to_str().ok())
    }

    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_bytes(key.to_lowercase().as_bytes()) {
            if let Ok(val) = tonic::metadata::MetadataValue::from_str(&value) {
                self.0.insert(key, val);
            }
        }
    }
}

macro_rules! client_method {
    ($func:ident, $req_msg:ident, $reply_msg:ident, $service:literal, $method:literal) => {
        pub async fn $func(&mut self, message: $req_msg) -> Result<Response<$reply_msg>, Status> {
            let tracer = opentelemetry::global::tracer("store_client");
            let span = tracer
                .span_builder(concat!($service, "/", $method))
                .with_kind(SpanKind::Client)
                .with_attributes(vec![
                    Key::new("rpc.system").string("grpc"),
                    Key::new("rpc.service").string($service),
                    Key::new("rpc.method").string($method),
                ])
                .start(&tracer);
            let cx = Context::current_with_span(span);
            let mut request = Request::new(message);
            opentelemetry::global::get_http_text_propagator(|propagator| {
                propagator
                    .inject_context(&cx, &mut TonicMetadataMapCarrier(request.metadata_mut()));
            });
            let res = self.inner.$func(request).with_context(cx.clone()).await;
            if let Err(err) = res.as_ref() {
                let span = cx.span();
                span.set_status(tonic_to_otel_status(&err), err.to_string());
                span.set_attribute(Key::new("error").string(err.to_string()));
            }
            res
        }
    };
}

#[derive(Clone)]
pub struct StoreClient {
    inner: store_client::StoreClient<Channel>,
}

impl StoreClient {
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<tonic::codegen::StdError>,
    {
        let inner = store_client::StoreClient::connect(dst).await?;
        Ok(Self { inner })
    }

    client_method!(
        import,
        ImportRequest,
        ImportReply,
        "ghss.store.Store",
        "Import"
    );

    client_method!(
        record_hook,
        RecordHookRequest,
        RecordHookReply,
        "ghss.store.Store",
        "RecordHook"
    );

    client_method!(
        get_hooked_commits_since_last_import,
        HookedCommitsRequest,
        HookedCommitsReply,
        "ghss.store.Store",
        "GetHookedCommitsSinceLastImport"
    );
}

#[derive(Clone)]
pub struct QueryClient {
    inner: query_client::QueryClient<Channel>,
}

impl QueryClient {
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<tonic::codegen::StdError>,
    {
        let inner = query_client::QueryClient::connect(dst).await?;
        Ok(Self { inner })
    }

    client_method!(
        get_total_aggregates,
        TotalAggregatesRequest,
        TotalAggregatesReply,
        "ghss.store.Query",
        "GetTotalAggregates"
    );

    client_method!(
        get_interval_aggregates,
        IntervalAggregatesRequest,
        IntervalAggregatesReply,
        "ghss.store.Query",
        "GetIntervalAggregates"
    );
}

fn tonic_to_otel_status(status: &Status) -> StatusCode {
    use Code::*;
    match status.code() {
        Ok => StatusCode::OK,
        Cancelled => StatusCode::Canceled,
        Unknown => StatusCode::Unknown,
        InvalidArgument => StatusCode::InvalidArgument,
        DeadlineExceeded => StatusCode::DeadlineExceeded,
        NotFound => StatusCode::NotFound,
        AlreadyExists => StatusCode::AlreadyExists,
        PermissionDenied => StatusCode::PermissionDenied,
        ResourceExhausted => StatusCode::ResourceExhausted,
        FailedPrecondition => StatusCode::FailedPrecondition,
        Aborted => StatusCode::Aborted,
        OutOfRange => StatusCode::OutOfRange,
        Unimplemented => StatusCode::Unimplemented,
        Internal => StatusCode::Internal,
        Unavailable => StatusCode::Unavailable,
        DataLoss => StatusCode::DataLoss,
        Unauthenticated => StatusCode::Unauthenticated,
        _ => StatusCode::Unknown,
    }
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
