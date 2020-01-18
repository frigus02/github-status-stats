use actix_web::HttpRequest;

pub fn header_as_string<'a>(req: &'a HttpRequest, header_name: &str) -> Result<&'a str, String> {
    req.headers()
        .get(header_name)
        .ok_or_else(|| format!("Header {} missing", header_name))
        .and_then(|header| {
            header
                .to_str()
                .map_err(|err| format!("Header {} not readable: {}", header_name, err))
        })
}
