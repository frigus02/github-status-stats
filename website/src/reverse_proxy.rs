use log::{debug, error};
use warp::http::uri::{Authority, Scheme};
use warp::hyper::header::{HeaderMap, HeaderName, HeaderValue};
use warp::hyper::{client::HttpConnector, Body, Client, Request, Response, StatusCode, Uri};

type BoxError = Box<dyn std::error::Error>;

lazy_static! {
    static ref WEBAUTH_USER_HEADER: HeaderName = HeaderName::from_static("x-webauth-user");
    static ref WEBAUTH_NAME_HEADER: HeaderName = HeaderName::from_static("x-webauth-name");
    static ref WEBAUTH_EMAIL_HEADER: HeaderName = HeaderName::from_static("x-webauth-email");
    static ref HEADER_BLACKLIST: Vec<HeaderName> = vec![
        // Hop-by-hop
        HeaderName::from_static("connection"),
        HeaderName::from_static("keep-alive"),
        HeaderName::from_static("proxy-authenticate"),
        HeaderName::from_static("proxy-authorization"),
        HeaderName::from_static("te"),
        HeaderName::from_static("trailers"),
        HeaderName::from_static("transfer-encoding"),
        HeaderName::from_static("upgrade"),
        // Auth proxy
        WEBAUTH_USER_HEADER.clone(),
        WEBAUTH_NAME_HEADER.clone(),
        WEBAUTH_EMAIL_HEADER.clone(),
        // Others
        HeaderName::from_static("host"),
    ];
}

fn filter_headers_by_blacklist(headers: &HeaderMap<HeaderValue>) -> HeaderMap<HeaderValue> {
    let mut result = HeaderMap::new();
    for (k, v) in headers.iter() {
        if !HEADER_BLACKLIST.iter().any(|h| h == k) {
            result.insert(k.clone(), v.clone());
        }
    }

    result
}

fn create_proxied_response(mut response: Response<Body>) -> Response<Body> {
    *response.headers_mut() = filter_headers_by_blacklist(response.headers());
    response
}

fn create_proxied_request(
    mut request: Request<Body>,
    new_scheme: &Scheme,
    new_authority: &Authority,
) -> Request<Body> {
    *request.headers_mut() = filter_headers_by_blacklist(request.headers());
    *request.uri_mut() = Uri::builder()
        .scheme(new_scheme.as_str())
        .authority(new_authority.as_str())
        .path_and_query(request.uri().path_and_query().map_or("", |p| p.as_str()))
        .build()
        .unwrap();
    request
}

fn add_request_authentication(request: &mut Request<Body>, auth: Auth) -> Result<(), BoxError> {
    let headers = request.headers_mut();
    headers.insert(&*WEBAUTH_USER_HEADER, auth.login.parse()?);
    headers.insert(&*WEBAUTH_NAME_HEADER, auth.name.parse()?);
    headers.insert(&*WEBAUTH_EMAIL_HEADER, auth.email.parse()?);
    Ok(())
}

pub struct ReverseProxy {
    client: Client<HttpConnector, Body>,
    scheme: Scheme,
    authority: Authority,
}

pub struct Auth {
    pub login: String,
    pub name: String,
    pub email: String,
}

impl ReverseProxy {
    pub fn new(base_url: &str) -> Result<ReverseProxy, BoxError> {
        let parts = base_url.parse::<Uri>()?.into_parts();
        Ok(ReverseProxy {
            client: Client::new(),
            scheme: parts.scheme.ok_or("base_url doesn't have scheme")?,
            authority: parts.authority.ok_or("base_url doesn't have authority")?,
        })
    }

    pub async fn call(
        &self,
        request: Request<Body>,
        auth: Option<Auth>,
    ) -> Result<Response<Body>, BoxError> {
        let mut proxied_request = create_proxied_request(request, &self.scheme, &self.authority);
        if let Some(auth) = auth {
            add_request_authentication(&mut proxied_request, auth)?;
        }

        debug!("reverse proxy request: {:?}", proxied_request);
        let response = self.client.request(proxied_request).await;
        match response {
            Ok(response) => Ok(create_proxied_response(response)),
            Err(error) => {
                error!("reverse proxy error: {}", error);
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())?)
            }
        }
    }
}
