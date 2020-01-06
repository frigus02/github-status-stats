use super::page_links;
use serde::de::DeserializeOwned;

struct Response<T> {
    data: T,
    next_page_url: Option<reqwest::Url>,
}

async fn call_api<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: reqwest::Url,
) -> Result<Response<T>, Box<dyn std::error::Error>> {
    println!("Calling {:#?}", url);

    let res = client.get(url).send().await?.error_for_status()?;
    let next_page_url = match res.headers().get(reqwest::header::LINK) {
        Some(value) => {
            if let Some(url) = page_links::parse(value.to_str()?).next {
                Some(reqwest::Url::parse(url)?)
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

pub async fn call_api_paged<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: reqwest::Url,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let mut items = Vec::new();

    let mut next_page_url = Some(url);
    while let Some(url) = next_page_url {
        let mut result = call_api::<Vec<T>>(client, url).await?;
        items.append(&mut result.data);
        next_page_url = result.next_page_url;
    }

    Ok(items)
}
