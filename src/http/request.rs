#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    GET,
    HEAD,
    POST,
    UNKNOWN,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, path: String) -> Self {
        Self { method, path }
    }
}

pub fn parse_http_request(request: &[u8]) -> Option<HttpRequest> {
    let request_str = std::str::from_utf8(request).ok()?;
    let mut parts = request_str.split_whitespace();
    let method = match parts.next()? {
        "GET" => HttpMethod::GET,
        "HEAD" => HttpMethod::HEAD,
        "POST" => HttpMethod::POST,
        _ => HttpMethod::UNKNOWN,
    };
    let path = parts.next()?.to_string();
    Some(HttpRequest::new(method, path))
}
