use super::page_links;
use reqwest::header::{ACCEPT, LINK};
use reqwest::{Client, Request, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::error::Error;
use tracing::debug;

struct Response<T> {
    data: T,
    next_page_url: Option<Url>,
}

pub const MACHINE_MAN_PREVIEW: &str = "application/vnd.github.machine-man-preview+json";
pub const ANTIOPE_PREVIEW: &str = "application/vnd.github.antiope-preview+json";

async fn call_api<T: DeserializeOwned>(
    client: &Client,
    request: Request,
) -> Result<Response<T>, Box<dyn Error>> {
    debug!(request.method = %request.method(), request.url = %request.url(), request.body = ?request.body(), "github request");
    let res = client.execute(request).await?.error_for_status()?;
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

pub async fn get<T: DeserializeOwned>(client: &Client, url: Url) -> Result<T, Box<dyn Error>> {
    let result = call_api::<T>(client, client.get(url).build()?).await?;
    Ok(result.data)
}

pub async fn post<B: Serialize + ?Sized, T: DeserializeOwned>(
    client: &Client,
    url: Url,
    body: &B,
) -> Result<T, Box<dyn Error>> {
    let result = call_api::<T>(client, client.post(url).json(body).build()?).await?;
    Ok(result.data)
}

pub async fn post_preview<T: DeserializeOwned>(
    client: &Client,
    url: Url,
    preview: &str,
) -> Result<T, Box<dyn Error>> {
    let result = call_api::<T>(client, client.post(url).header(ACCEPT, preview).build()?).await?;
    Ok(result.data)
}

pub async fn get_paged<T: DeserializeOwned>(
    client: &Client,
    url: Url,
) -> Result<Vec<T>, Box<dyn Error>> {
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
) -> Result<Vec<T>, Box<dyn Error>> {
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
