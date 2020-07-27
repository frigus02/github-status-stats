use opentelemetry::api::{Context, FutureExt, Key, SpanKind, TraceContextExt, Tracer};
use tide::{http::headers, utils::async_trait, Middleware, Next, Request, Result};

pub struct TelemetryMiddleware {}

#[async_trait]
impl<State> Middleware<State> for TelemetryMiddleware
where
    State: Clone + Send + Sync + 'static,
{
    async fn handle(&self, req: Request<State>, next: Next<'_, State>) -> Result {
        let tracer = opentelemetry::global::tracer("website");
        let span = tracer
            .span_builder(&format!("HTTP {}", req.method().as_ref()))
            .with_kind(SpanKind::Server)
            .with_attributes(vec![
                Key::new("http.method").string(req.method().as_ref()),
                Key::new("http.target").string(req.url().as_ref()),
                Key::new("http.user_agent").string(
                    req.header(headers::USER_AGENT)
                        .map(|v| v.as_str())
                        .unwrap_or(""),
                ),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);

        let res = next.run(req).with_context(cx.clone()).await;

        let http_status_code = u16::from(res.status());
        let span = cx.span();
        span.set_attribute(Key::new("http.status_code").u64(http_status_code.into()));
        use opentelemetry::api::StatusCode;
        let telemetry_status = match http_status_code {
            200..=399 => StatusCode::OK,
            401 => StatusCode::Unauthenticated,
            403 => StatusCode::PermissionDenied,
            404 => StatusCode::NotFound,
            400 | 402 | 405..=499 => StatusCode::InvalidArgument,
            500..=599 => StatusCode::Internal,
            _ => StatusCode::Unknown,
        };
        span.set_status(
            telemetry_status,
            res.error()
                .map(|err| err.to_string())
                .unwrap_or_else(|| "".to_owned()),
        );

        Ok(res)
    }
}
