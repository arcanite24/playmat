//! Fetches deck pages and APIs for the sources, and nothing else (Java's `HttpFetcher`):
//! https on the default port to allowlisted hosts only, redirects followed by hand and only to allowlisted hosts,
//! GET only with no cookies and an honest User-Agent, an 8 second deadline and a 2 MB response cap.

use super::{Failure, Reason};
use async_trait::async_trait;
use regex::Regex;
use std::collections::BTreeSet;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::time::Instant;
use url::Url;

pub const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_REDIRECTS: usize = 3;
const TIMEOUT: Duration = Duration::from_secs(8);
/// Java's `MageVersion`
const USER_AGENT: &str = "Playmat/1.4.62 (XMage web client; deck import on request)";

static CHARSET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)charset=([A-Za-z0-9_.-]+)").unwrap());

/// Reads one page or API response from a deck website: tests serve recorded responses.
#[async_trait]
pub trait Fetcher: Send + Sync {
    /// the response body, decoded with the response's charset
    async fn get(&self, url: &str, accept: &str) -> Result<String, Failure>;
}

pub struct HttpFetcher {
    allowed: BTreeSet<String>,
    client: reqwest::Client,
}

impl HttpFetcher {
    pub fn new(allowed: BTreeSet<String>) -> HttpFetcher {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .user_agent(USER_AGENT)
            .no_proxy()
            .build()
            .expect("the deck import HTTP client");
        HttpFetcher { allowed, client }
    }
}

/// Whether the server may fetch this address at all.
pub fn allowed(url: &Url, hosts: &BTreeSet<String>) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none_or(|port| port == 443)
        && url.host_str().is_some_and(|host| hosts.contains(&host.to_ascii_lowercase()))
}

fn host(url: &Url) -> &str {
    url.host_str().unwrap_or("")
}

/// The failure a non-200 answer means (`HttpFetcher.failure`).
pub fn failure(status: u16, body: &str, host: &str) -> Failure {
    if body.contains("Just a moment...") || body.contains("cf-chl") || body.contains("challenge-platform") {
        return Failure::new(Reason::Blocked, format!("{host} asked for a browser check"));
    }
    match status {
        401 | 403 => Failure::new(Reason::Private, format!("{host} won't share this deck")),
        404 | 410 => Failure::new(Reason::NotFound, format!("{host} has no such deck")),
        429 => Failure::new(Reason::RateLimited, format!("{host} asked us to slow down")),
        _ if status >= 500 => Failure::new(Reason::Unavailable, format!("{host} answered {status}")),
        _ => Failure::new(Reason::SiteChanged, format!("{host} answered {status}")),
    }
}

fn decode(bytes: &[u8], content_type: Option<&str>) -> String {
    let encoding = content_type
        .and_then(|t| CHARSET.captures(t))
        .and_then(|c| encoding_rs::Encoding::for_label(c[1].as_bytes()))
        .unwrap_or(encoding_rs::UTF_8);
    encoding.decode_without_bom_handling(bytes).0.into_owned()
}

#[async_trait]
impl Fetcher for HttpFetcher {
    async fn get(&self, url: &str, accept: &str) -> Result<String, Failure> {
        let deadline = Instant::now() + TIMEOUT;
        let original = Url::parse(url).map_err(|_| Failure::Shape)?;
        let mut current = original.clone();
        for _ in 0..=MAX_REDIRECTS {
            if !allowed(&current, &self.allowed) {
                return Err(Failure::new(
                    Reason::Unsupported,
                    format!("Not a deck site the server reads: {}", host(&current)),
                ));
            }
            let request = self.client.get(current.clone()).header(reqwest::header::ACCEPT, accept).send();
            let mut response = match tokio::time::timeout_at(deadline, request).await {
                Err(_) => {
                    return Err(Failure::new(Reason::Timeout, format!("{} didn't answer in time", host(&current))))
                }
                Ok(Err(e)) if e.is_timeout() => {
                    return Err(Failure::new(Reason::Timeout, format!("{} didn't answer in time", host(&current))));
                }
                Ok(Err(_)) => {
                    return Err(Failure::new(Reason::Unavailable, format!("{} couldn't be reached", host(&current))));
                }
                Ok(Ok(response)) => response,
            };
            let status = response.status().as_u16();
            if (300..400).contains(&status) {
                let Some(location) = response.headers().get(reqwest::header::LOCATION) else {
                    return Err(Failure::new(Reason::SiteChanged, format!("{} redirected nowhere", host(&current))));
                };
                // URI.resolve throws on a location it can't read
                let location = location.to_str().map_err(|_| Failure::Shape)?;
                current = current.join(location).map_err(|_| Failure::Shape)?;
                continue;
            }
            let content_type =
                response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(str::to_string);
            let mut body = Vec::new();
            loop {
                let chunk = match tokio::time::timeout_at(deadline, response.chunk()).await {
                    Err(_) => {
                        return Err(Failure::new(
                            Reason::Timeout,
                            format!("{} didn't finish answering in time", host(&current)),
                        ));
                    }
                    Ok(Err(_)) => {
                        return Err(Failure::new(
                            Reason::Unavailable,
                            format!("{} couldn't be reached", host(&current)),
                        ));
                    }
                    Ok(Ok(None)) => break,
                    Ok(Ok(Some(chunk))) => chunk,
                };
                if body.len() + chunk.len() > MAX_BYTES {
                    return Err(Failure::new(
                        Reason::TooLarge,
                        format!("{} sent more than a deck's worth of data", host(&current)),
                    ));
                }
                body.extend_from_slice(&chunk);
            }
            let text = decode(&body, content_type.as_deref());
            if status == 200 {
                return Ok(text);
            }
            return Err(failure(status, &text, host(&current)));
        }
        Err(Failure::new(Reason::SiteChanged, format!("{} redirected too many times", host(&original))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist() {
        let hosts: BTreeSet<String> = ["archidekt.com".to_string()].into();
        let ok = |url: &str| allowed(&Url::parse(url).unwrap(), &hosts);
        assert!(ok("https://archidekt.com/api/decks/1/"));
        assert!(ok("https://ARCHIDEKT.com:443/x"));
        assert!(!ok("http://archidekt.com/x"));
        assert!(!ok("https://archidekt.com:8443/x"));
        assert!(!ok("https://me@archidekt.com/x"));
        assert!(!ok("https://127.0.0.1/x"));
        assert!(!ok("https://evil.com/x"));
    }

    #[test]
    fn failures() {
        assert_eq!(
            failure(503, "<title>Just a moment...</title>", "a"),
            Failure::new(Reason::Blocked, "a asked for a browser check")
        );
        assert_eq!(failure(403, "", "a"), Failure::new(Reason::Private, "a won't share this deck"));
        assert_eq!(failure(410, "", "a"), Failure::new(Reason::NotFound, "a has no such deck"));
        assert_eq!(failure(502, "", "a"), Failure::new(Reason::Unavailable, "a answered 502"));
        assert_eq!(failure(418, "", "a"), Failure::new(Reason::SiteChanged, "a answered 418"));
        assert_eq!(decode(&[0x63, 0x61, 0x66, 0xE9], Some("text/html; Charset=ISO-8859-1")), "café");
        assert_eq!(decode("café".as_bytes(), None), "café");
    }
}
