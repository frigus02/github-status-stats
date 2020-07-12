use super::page_links;
use opentelemetry::api::{Context, FutureExt, Key, SpanKind, StatusCode, TraceContextExt, Tracer};
use reqwest::header::{ACCEPT, LINK};
use reqwest::{Client, Request, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

struct Response<T> {
    data: T,
    next_page_url: Option<Url>,
}

pub const MACHINE_MAN_PREVIEW: &str = "application/vnd.github.machine-man-preview+json";
pub const ANTIOPE_PREVIEW: &str = "application/vnd.github.antiope-preview+json";

async fn call_api<T: DeserializeOwned>(
    client: &Client,
    request: Request,
) -> Result<Response<T>, BoxError> {
    let tracer = opentelemetry::global::tracer("github");
    let span = tracer
        .span_builder("github request")
        .with_kind(SpanKind::Client)
        .with_attributes(vec![
            Key::new("http.method").string(request.method().as_str()),
            Key::new("http.url").string(request.url().as_str()),
        ])
        .start(&tracer);
    let cx = Context::current_with_span(span);

    let res = client.execute(request).with_context(cx.clone()).await;

    let span = cx.span();
    match res.as_ref() {
        Ok(res) => {
            span.set_attribute(Key::new("http.status_code").u64(res.status().as_u16().into()));
            let code = match res.status().as_u16() {
                200..=399 => StatusCode::OK,
                401 => StatusCode::Unauthenticated,
                403 => StatusCode::PermissionDenied,
                404 => StatusCode::NotFound,
                400 | 402 | 405..=499 => StatusCode::InvalidArgument,
                500..=599 => StatusCode::Internal,
                _ => StatusCode::Unknown,
            };
            span.set_status(code, "".into());
        }
        Err(err) => {
            span.set_status(StatusCode::Internal, err.to_string());
            span.set_attribute(Key::new("error").string(err.to_string()));
        }
    };

    let res = res?.error_for_status()?;
    let next_page_url = match res.headers().get(LINK) {
        Some(value) => {
            if let Some(url) = page_links::parse(value.to_str()?).next {
                Some(Url::parse(url)?)
            } else {
                None
            }
        }
        None => None,
    };
    let data: T = res.json().await?;

    Ok(Response {
        data,
        next_page_url,
    })
}

pub async fn get<T: DeserializeOwned>(client: &Client, url: Url) -> Result<T, BoxError> {
    let result = call_api::<T>(client, client.get(url).build()?).await?;
    Ok(result.data)
}

pub async fn post<B: Serialize + ?Sized, T: DeserializeOwned>(
    client: &Client,
    url: Url,
    body: &B,
) -> Result<T, BoxError> {
    let result = call_api::<T>(client, client.post(url).json(body).build()?).await?;
    Ok(result.data)
}

pub async fn post_preview<T: DeserializeOwned>(
    client: &Client,
    url: Url,
    preview: &str,
) -> Result<T, BoxError> {
    let result = call_api::<T>(client, client.post(url).header(ACCEPT, preview).build()?).await?;
    Ok(result.data)
}

pub async fn get_paged<T: DeserializeOwned>(client: &Client, url: Url) -> Result<Vec<T>, BoxError> {
    let mut items = Vec::new();

    let mut next_page_url = Some(url);
    while let Some(url) = next_page_url {
        let result = call_api::<T>(client, client.get(url).build()?).await?;
        items.push(result.data);
        next_page_url = result.next_page_url;
    }

    Ok(items)
}

pub async fn get_paged_preview<T: DeserializeOwned>(
    client: &Client,
    url: Url,
    preview: &str,
) -> Result<Vec<T>, BoxError> {
    let mut items = Vec::new();

    let mut next_page_url = Some(url);
    while let Some(url) = next_page_url {
        let result =
            call_api::<T>(client, client.get(url).header(ACCEPT, preview).build()?).await?;
        items.push(result.data);
        next_page_url = result.next_page_url;
    }

    Ok(items)
}
