use std::convert::Infallible;
use warp::{http::Request, hyper::Body, Filter};

pub fn raw_query_option() -> impl Filter<Extract = (Option<String>,), Error = Infallible> + Clone {
    warp::query::raw()
        .map(Some)
        .or(warp::any().map(|| None))
        .unify()
}

pub fn raw_request() -> impl Filter<Extract = (Request<Body>,), Error = warp::Rejection> + Clone {
    warp::method()
        .and(warp::path::full())
        .and(raw_query_option())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .map(
            |method, path: warp::filters::path::FullPath, query: Option<String>, headers, body| {
                let mut req = Request::builder()
                    .method(method)
                    .uri(format!(
                        "{}{}",
                        path.as_str(),
                        query.map_or("".to_owned(), |q| format!("?{}", q))
                    ))
                    .body(warp::hyper::body::Body::from(body))
                    .expect("request builder");
                *req.headers_mut() = headers;
                req
            },
        )
}
