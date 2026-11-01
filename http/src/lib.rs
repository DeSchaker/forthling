//! An HTTP client for forthling: the word list HTTP. A program asks for a
//! page or posts to an API and gets the status and the body back. It reaches
//! other machines, so it is guarded three ways: `Permission::Network` (which
//! `Forth::sandboxed` denies), a list of the hosts it may connect to (empty
//! until the host adds some with `allow`), and the limits of the sandbox: the
//! time left in the budget bounds a request, the body is capped and the bytes
//! are charged as work. Strings that a word returns are `ALLOCATE`d, for the
//! program to FREE.
//!
//! ```
//! use forthling::Forth;
//! use forthling_http::Http;
//!
//! let mut f = Forth::new();
//! f.install(&Http::new().allow("api.example.com"));
//! // The host is not in the list, so nothing is sent:
//! let e = f.interpret("ALSO HTTP  S\" http://other.example/\" HTTP-GET").unwrap_err();
//! assert_eq!(e.code, forthling::NOT_PERMITTED);
//! ```
//!
//! Redirects are followed (at most 5), and each hop must be to an allowed
//! host too; headers set for the request are dropped when a redirect goes to
//! another host. There is no proxy, and no decompression.

use forthling::{Cell, Extension, Forth, NOT_PERMITTED, Permission, Res, Throw};
use std::time::Duration;
use ureq::http::Uri;

/// The throw of a request that fails (not of a status such as 404, which
/// the program gets as a number): an address that is no http or https URL,
/// a connection that fails, a time that runs out, a response that is too
/// large, too many redirects. The message says which.
pub const HTTP_ERROR: Cell = -263;

/// The redirects a request follows.
const MAX_REDIRECTS: usize = 5;

/// The extension: `forth.install(&Http::new().allow("api.example.com"))`.
#[derive(Clone, Debug)]
pub struct Http {
    hosts: Vec<String>,
    timeout: Duration,
    max_body: usize,
}

impl Default for Http {
    fn default() -> Self {
        Http::new()
    }
}

impl Http {
    /// No host is allowed yet, a request may take 30 seconds when the
    /// sandbox sets no deadline, and a response may be 1 MiB.
    #[must_use]
    pub fn new() -> Self {
        Http { hosts: Vec::new(), timeout: Duration::from_secs(30), max_body: 1 << 20 }
    }

    /// Allows a host, by name without a port, in any case: `api.example.com`;
    /// `*.example.com` allows every host below it (not `example.com`
    /// itself), and `*` every host. An address is compared as text, so
    /// `127.0.0.1` allows that address, not `localhost`.
    #[must_use]
    pub fn allow(mut self, host: &str) -> Self {
        self.hosts.push(host.to_ascii_lowercase());
        self
    }

    /// Allows each of the hosts (see `allow`).
    #[must_use]
    pub fn allow_all<'a>(mut self, hosts: impl IntoIterator<Item = &'a str>) -> Self {
        for host in hosts {
            self = self.allow(host);
        }
        self
    }

    /// The longest a request may take when the evaluation has no time limit
    /// (with one, the time left is the longest).
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The largest response body in bytes; a larger one throws -263. It can
    /// never be larger than the Forth memory.
    #[must_use]
    pub fn max_body(mut self, bytes: usize) -> Self {
        self.max_body = bytes;
        self
    }

    fn allows(&self, host: &str) -> bool {
        let host = host.to_ascii_lowercase();
        self.hosts.iter().any(|h| {
            h == "*"
                || *h == host
                || h.strip_prefix("*.").is_some_and(|rest| {
                    host.len() > rest.len() + 1
                        && host.ends_with(rest)
                        && host.as_bytes()[host.len() - rest.len() - 1] == b'.'
                })
        })
    }
}

/// What the words keep between calls.
#[derive(Default)]
struct State {
    http: Http,
    headers: Vec<(String, String)>,  // for the next request
    response: Vec<(String, String)>, // of the latest response, names in lower case
}

fn failure(what: impl Into<String>) -> Throw {
    Throw::with(HTTP_ERROR, what)
}

fn utf8(bytes: Vec<u8>, what: &str) -> Res<String> {
    String::from_utf8(bytes).map_err(|_| failure(format!("the {what} is not UTF-8")))
}

/// What a request is for.
struct Request {
    post: bool,
    url: String,
    body: Vec<u8>,
    content_type: String,
}

/// The host of a URL that is http or https; the URL parsed.
fn parse(url: &str) -> Res<(Uri, String)> {
    let parsed: Uri = url.parse().map_err(|_| failure(format!("{url} is not a URL")))?;
    match parsed.scheme_str() {
        Some("http" | "https") => {}
        _ => return Err(failure(format!("{url} is not an http or https URL"))),
    }
    let host = parsed.host().ok_or_else(|| failure(format!("{url} has no host")))?.to_string();
    Ok((parsed, host))
}

/// The address a redirect points to, from the one it came from.
fn redirect(from: &Uri, location: &str) -> Res<String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        return Ok(location.to_string());
    }
    if location.starts_with('/') && !location.starts_with("//") {
        let scheme = from.scheme_str().unwrap_or("http");
        let authority = from.authority().map_or("", |a| a.as_str());
        return Ok(format!("{scheme}://{authority}{location}"));
    }
    Err(failure(format!("cannot follow the redirect to {location}")))
}

/// Sends the request and gives the status and the body; the headers of the
/// response are kept for HTTP-RESPONSE-HEADER.
fn send(f: &mut Forth, mut req: Request) -> Res<(u16, Vec<u8>)> {
    if !f.permits(Permission::Network) {
        let what = format!("the network is not permitted (request to {})", req.url);
        return Err(Throw::with(NOT_PERMITTED, what));
    }
    f.charge(req.body.len())?;
    let room = f.memory().len();
    let state = f.state::<State>();
    let mut headers = std::mem::take(&mut state.headers);
    let http = state.http.clone();
    state.response.clear();
    let cap = http.max_body.min(room);
    let mut first_host: Option<String> = None;
    for _ in 0..=MAX_REDIRECTS {
        let (uri, host) = parse(&req.url)?;
        if !http.allows(&host) {
            return Err(Throw::with(NOT_PERMITTED, format!("the host {host} is not allowed")));
        }
        // What was set for a host does not go to another one.
        match &first_host {
            None => first_host = Some(host),
            Some(first) if !first.eq_ignore_ascii_case(&host) => headers.clear(),
            _ => {}
        }
        let time = match f.deadline() {
            Some(deadline) => {
                f.check_time()?;
                deadline.saturating_duration_since(std::time::Instant::now()).min(http.timeout)
            }
            None => http.timeout,
        };
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(time))
            .max_redirects(0)
            .max_redirects_will_error(false)
            .http_status_as_error(false)
            .proxy(None)
            .build()
            .into();
        let result = if req.post {
            let mut call = agent.post(&req.url);
            for (name, value) in &headers {
                call = call.header(name, value);
            }
            if !req.content_type.is_empty() {
                call = call.header("Content-Type", &req.content_type);
            }
            call.send(&req.body[..])
        } else {
            let mut call = agent.get(&req.url);
            for (name, value) in &headers {
                call = call.header(name, value);
            }
            call.call()
        };
        let mut response = match result {
            Ok(response) => response,
            Err(e) => {
                f.check_time()?; // the time was up, rather than the network down
                return Err(failure(format!("{}: {e}", req.url)));
            }
        };
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(std::string::ToString::to_string);
        if matches!(status, 301 | 302 | 303 | 307 | 308) {
            if let Some(location) = location {
                // 301, 302 and 303 turn a POST into a GET, as browsers do.
                if req.post && matches!(status, 301..=303) {
                    req.post = false;
                    req.body.clear();
                }
                req.url = redirect(&uri, &location)?;
                continue;
            }
        }
        let names = response.headers().iter();
        let kept: Vec<(String, String)> = names
            .filter_map(|(k, v)| {
                Some((k.as_str().to_ascii_lowercase(), v.to_str().ok()?.to_string()))
            })
            .collect();
        let body = response
            .body_mut()
            .with_config()
            .limit(cap as u64)
            .read_to_vec()
            .map_err(|e| failure(format!("{}: {e} (at most {cap} bytes)", req.url)))?;
        f.charge(body.len())?;
        f.state::<State>().response = kept;
        return Ok((status, body));
    }
    Err(failure(format!("more than {MAX_REDIRECTS} redirects")))
}

fn status(f: &mut Forth, (code, body): (u16, Vec<u8>)) -> Res {
    f.push_bytes(&body)?;
    f.push(Cell::from(code));
    Ok(())
}

/// A word of the HTTP word list; the word list exists once installed.
fn word(f: &mut Forth, name: &str, word: impl Fn(&mut Forth) -> Res + 'static) {
    let wid = f.wordlist("HTTP");
    f.add_word(wid, name, word).expect("HTTP is a word list and names are not empty");
}

impl Extension for Http {
    fn name(&self) -> &'static str {
        "FORTHLING-HTTP"
    }

    fn install(&self, f: &mut Forth) {
        f.state::<State>().http = self.clone();
        // HTTP-GET ( url-addr url-u -- body-addr body-u status ): asks for a
        // page. The body is ALLOCATEd; a status such as 404 is no error.
        word(f, "HTTP-GET", |f| {
            let url = utf8(f.pop_bytes()?, "URL")?;
            let response = send(
                f,
                Request { post: false, url, body: Vec::new(), content_type: String::new() },
            )?;
            status(f, response)
        });
        // HTTP-POST ( url-addr url-u body-addr body-u type-addr type-u --
        // body-addr' body-u' status ): sends the body with the content type,
        // such as S" application/json".
        word(f, "HTTP-POST", |f| {
            let content_type = utf8(f.pop_bytes()?, "content type")?;
            let body = f.pop_bytes()?;
            let url = utf8(f.pop_bytes()?, "URL")?;
            let response = send(f, Request { post: true, url, body, content_type })?;
            status(f, response)
        });
        // HTTP-HEADER ( name-addr name-u value-addr value-u -- ): adds a
        // header to the next request, and only that one.
        word(f, "HTTP-HEADER", |f| {
            let value = utf8(f.pop_bytes()?, "header value")?;
            let name = utf8(f.pop_bytes()?, "header name")?;
            if name.is_empty() || name.contains(|c: char| c.is_whitespace() || c == ':') {
                return Err(failure(format!("{name:?} is not a header name")));
            }
            if value.contains(['\r', '\n']) {
                return Err(failure("a header value cannot hold a line break"));
            }
            f.state::<State>().headers.push((name, value));
            Ok(())
        });
        // HTTP-RESPONSE-HEADER ( name-addr name-u -- value-addr value-u true
        // | false ): a header of the latest response, whatever the case of
        // the name; the value is ALLOCATEd.
        word(f, "HTTP-RESPONSE-HEADER", |f| {
            let name = utf8(f.pop_bytes()?, "header name")?.to_ascii_lowercase();
            let value = f
                .state::<State>()
                .response
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.clone());
            match value {
                Some(value) => {
                    f.push_bytes(value.as_bytes())?;
                    f.push(-1);
                }
                None => f.push(0),
            }
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_patterns() {
        let http = Http::new().allow("API.Example.com").allow("*.cdn.org");
        for host in ["api.example.com", "Api.Example.COM", "a.cdn.org", "x.y.cdn.org"] {
            assert!(http.allows(host), "{host}");
        }
        for host in ["example.com", "cdn.org", "evilcdn.org", "api.example.com.evil.net", ""] {
            assert!(!http.allows(host), "{host}");
        }
        assert!(Http::new().allow("*").allows("anything.at.all"));
        assert!(!Http::new().allows("localhost"));
        assert!(Http::new().allow_all(["a.org", "b.org"]).allows("b.org"));
    }

    #[test]
    fn redirects_resolve() {
        let from: Uri = "https://a.org:8080/x/y?q=1".parse().unwrap();
        assert_eq!(redirect(&from, "/z").unwrap(), "https://a.org:8080/z");
        assert_eq!(redirect(&from, "http://b.org/").unwrap(), "http://b.org/");
        assert!(redirect(&from, "//b.org/").is_err());
        assert!(redirect(&from, "z").is_err());
    }
}
