use super::page_links;
use reqwest::header::{ACCEPT, LINK};
use reqwest::{Client, RequestBuilder, Url};
use serde::de::DeserializeOwned;
use std::error::Error;

struct Response<T> {
    data: T,
    next_page_url: Option<Url>,
}

async fn call_api<T: DeserializeOwned>(
    request: RequestBuilder,
) -> Result<Response<T>, Box<dyn Error>> {
    let res = request.send().await?.error_for_status()?;
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
    let result = call_api::<T>(client.get(url)).await?;
    Ok(result.data)
}

pub async fn post_preview<T: DeserializeOwned>(
    client: &Client,
    url: Url,
) -> Result<T, Box<dyn Error>> {
    let result = call_api::<T>(
        client
            .post(url)
            .header(ACCEPT, "application/vnd.github.machine-man-preview+json"),
    )
    .await?;
    Ok(result.data)
}

pub async fn get_paged<T: DeserializeOwned>(
    client: &Client,
    url: Url,
) -> Result<Vec<T>, Box<dyn Error>> {
    let mut items = Vec::new();

    let mut next_page_url = Some(url);
    while let Some(url) = next_page_url {
        let result = call_api::<T>(client.get(url)).await?;
        items.push(result.data);
        next_page_url = result.next_page_url;
    }

    Ok(items)
}

pub async fn get_paged_preview<T: DeserializeOwned>(
    client: &Client,
    url: Url,
) -> Result<Vec<T>, Box<dyn Error>> {
    let mut items = Vec::new();

    let mut next_page_url = Some(url);
    while let Some(url) = next_page_url {
        let result = call_api::<T>(
            client
                .get(url)
                .header(ACCEPT, "application/vnd.github.machine-man-preview+json"),
        )
        .await?;
        items.push(result.data);
        next_page_url = result.next_page_url;
    }

    Ok(items)
}
