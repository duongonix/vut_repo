//! HTTP request specification: validate input, encode query, execute.
//!
//! This is the pure request side of the native backend: it turns the flat ABI
//! arguments into a concrete `reqwest` request and runs it, producing a
//! transport-neutral [`Outcome`] that the response encoder serializes.
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use super::{error, response::Outcome};

pub(super) struct RequestSpec {
    pub(super) method: reqwest::Method,
    pub(super) url: reqwest::Url,
    pub(super) headers: HeaderMap,
    pub(super) body: Vec<u8>,
    pub(super) timeout: Option<Duration>,
    pub(super) redirects: usize,
}

/// Parses the flat `Name: value` header block, preserving every value.
pub(super) fn parse_headers(raw: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for line in raw.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let Ok(name) = HeaderName::from_bytes(name.trim().as_bytes()) else {
            continue;
        };
        let Ok(value) = HeaderValue::from_str(value.trim()) else {
            continue;
        };
        headers.append(name, value);
    }
    headers
}

/// Applies raw `key=value` pairs, percent-encoding them correctly.
fn apply_query(url: &mut reqwest::Url, raw: &str) {
    let mut pairs = url.query_pairs_mut();
    for line in raw.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once('=').unwrap_or((line, ""));
        pairs.append_pair(key, value);
    }
}

pub(super) fn build_spec(
    method: &str,
    url: &str,
    headers: &str,
    query: &str,
    body: &[u8],
    timeout_nanos: usize,
    redirects: usize,
) -> Result<RequestSpec, (i32, String)> {
    let Ok(method) = reqwest::Method::from_bytes(method.as_bytes()) else {
        return Err((error::UNSUPPORTED, "unsupported HTTP method".into()));
    };
    let Ok(mut url) = reqwest::Url::parse(url) else {
        return Err((error::INVALID_URL, "invalid URL".into()));
    };
    if !query.is_empty() {
        apply_query(&mut url, query);
    }
    let timeout = (timeout_nanos > 0)
        .then(|| Duration::from_nanos(u64::try_from(timeout_nanos).unwrap_or(u64::MAX)));
    Ok(RequestSpec {
        method,
        url,
        headers: parse_headers(headers),
        body: body.to_vec(),
        timeout,
        redirects,
    })
}

pub(super) async fn execute(spec: RequestSpec, client: reqwest::Client) -> Outcome {
    let mut request = client
        .request(spec.method, spec.url)
        .headers(spec.headers)
        .body(spec.body);
    if let Some(timeout) = spec.timeout {
        request = request.timeout(timeout);
    }
    match request.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let headers = response.headers().clone();
            match response.bytes().await {
                Ok(body) => Outcome::Response {
                    status,
                    headers,
                    body: body.to_vec(),
                },
                Err(error) => Outcome::Failure {
                    kind: error::BODY,
                    message: error.to_string(),
                },
            }
        }
        Err(error) => Outcome::Failure {
            kind: error::kind(&error),
            message: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_pairs_are_percent_encoded() {
        let mut url = reqwest::Url::parse("http://example.com/").expect("url");
        apply_query(&mut url, "q=a b&c");
        assert_eq!(url.query(), Some("q=a+b%26c"));
    }

    #[test]
    fn invalid_url_and_method_map_to_native_codes() {
        assert!(matches!(
            build_spec("GET", "not a url", "", "", &[], 0, 0),
            Err((error::INVALID_URL, _))
        ));
        assert!(matches!(
            build_spec("bad method", "http://example.com/", "", "", &[], 0, 0),
            Err((error::UNSUPPORTED, _))
        ));
    }

    #[test]
    fn headers_round_trip_preserving_multi_values() {
        let map = parse_headers("X-Test: one\nX-Test: two\n\nY: 3");
        assert_eq!(map.get_all("x-test").iter().count(), 2);
        let rendered = super::super::response::render_headers(&map);
        assert!(rendered.contains("x-test: one"));
        assert!(rendered.contains("x-test: two"));
    }
}
