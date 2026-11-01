//! The HTTP words, against a small server on this machine.

use forthling::{Forth, LIMIT_EXCEEDED, NOT_PERMITTED, Permission};
use forthling_http::{HTTP_ERROR, Http};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// The requests the server saw, as text (the head and the body).
type Seen = Arc<Mutex<Vec<String>>>;

fn read_request(stream: &mut TcpStream) -> String {
    let mut data = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let n = stream.read(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        let text = String::from_utf8_lossy(&data).to_string();
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase().strip_prefix("content-length:")?.trim().parse().ok()
                })
                .unwrap_or(0usize);
            if data.len() >= end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&data).to_string()
}

fn respond(stream: &mut TcpStream, status: &str, headers: &[&str], body: &[u8]) {
    let mut head = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
    for h in headers {
        head += &format!("{h}\r\n");
    }
    head += &format!("Content-Length: {}\r\n\r\n", body.len());
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

/// Serves until the process ends; returns the port and what it saw.
fn serve() -> (u16, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen: Seen = Arc::default();
    let log = seen.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let log = log.clone();
            thread::spawn(move || {
                let request = read_request(&mut stream);
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
                log.lock().unwrap().push(request.clone());
                let body = request.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
                match path.as_str() {
                    "/hello" => respond(
                        &mut stream,
                        "200 OK",
                        &["X-Greeting: Hi", "Content-Type: text/plain"],
                        b"hello",
                    ),
                    "/empty" => respond(&mut stream, "204 No Content", &[], b""),
                    "/missing" => respond(&mut stream, "404 Not Found", &[], b"nope"),
                    "/echo" => respond(&mut stream, "200 OK", &[], body.as_bytes()),
                    "/redirect" => respond(&mut stream, "302 Found", &["Location: /hello"], b""),
                    "/loop" => respond(&mut stream, "302 Found", &["Location: /loop"], b""),
                    "/offsite" => {
                        let to = format!("Location: http://localhost:{}/hello", port);
                        respond(&mut stream, "302 Found", &[&to], b"");
                    }
                    "/big" => respond(&mut stream, "200 OK", &[], &vec![b'x'; 2 << 20]),
                    "/slow" => {
                        thread::sleep(Duration::from_secs(3));
                        respond(&mut stream, "200 OK", &[], b"late");
                    }
                    _ => respond(&mut stream, "404 Not Found", &[], b""),
                }
            });
        }
    });
    (port, seen)
}

fn forth(http: Http) -> Forth {
    let mut f = Forth::new();
    f.install(&http);
    f.interpret("ALSO HTTP").unwrap();
    f
}

fn run(f: &mut Forth, code: &str) -> String {
    f.interpret(code).unwrap_or_else(|e| panic!("{code}: {e}"));
    String::from_utf8(f.take_output()).unwrap()
}

fn get(port: u16, path: &str) -> String {
    format!("S\" http://127.0.0.1:{port}{path}\" HTTP-GET")
}

#[test]
fn get_gives_body_and_status() {
    let (port, _) = serve();
    let mut f = forth(Http::new().allow("127.0.0.1"));
    let code = format!("{} . TYPE", get(port, "/hello"));
    assert_eq!(run(&mut f, &code), "200 hello");
    assert_eq!(run(&mut f, &format!("{} . TYPE", get(port, "/missing"))), "404 nope");
    assert_eq!(run(&mut f, &format!("{} . NIP", get(port, "/empty"))), "204 ");
}

#[test]
fn post_sends_body_and_content_type() {
    let (port, seen) = serve();
    let mut f = forth(Http::new().allow("127.0.0.1"));
    // In a definition, since the strings of S" in the interpreter share a buffer.
    let code = format!(
        ": POST  S\" http://127.0.0.1:{port}/echo\" S\" a=1\" S\" application/json\" HTTP-POST ; POST . TYPE"
    );
    assert_eq!(run(&mut f, &code), "200 a=1");
    let request = seen.lock().unwrap().last().unwrap().to_ascii_lowercase();
    assert!(request.starts_with("post /echo "));
    assert!(request.contains("content-type: application/json"));
}

#[test]
fn headers_apply_to_the_next_request_only() {
    let (port, seen) = serve();
    let mut f = forth(Http::new().allow("127.0.0.1"));
    run(&mut f, "S\" Authorization\" S\" Bearer abc\" HTTP-HEADER");
    run(&mut f, &format!("{} 2DROP DROP", get(port, "/hello")));
    run(&mut f, &format!("{} 2DROP DROP", get(port, "/hello")));
    let seen = seen.lock().unwrap();
    assert!(seen[0].to_ascii_lowercase().contains("authorization: bearer abc"));
    assert!(!seen[1].to_ascii_lowercase().contains("authorization"));
}

#[test]
fn response_headers() {
    let (port, _) = serve();
    let mut f = forth(Http::new().allow("127.0.0.1"));
    run(&mut f, &format!("{} 2DROP DROP", get(port, "/hello")));
    assert_eq!(run(&mut f, "S\" x-greeting\" HTTP-RESPONSE-HEADER . TYPE"), "-1 Hi");
    assert_eq!(run(&mut f, "S\" X-GREETING\" HTTP-RESPONSE-HEADER . TYPE"), "-1 Hi");
    assert_eq!(run(&mut f, "S\" x-none\" HTTP-RESPONSE-HEADER ."), "0 ");
}

#[test]
fn redirects_stay_among_the_allowed_hosts() {
    let (port, _) = serve();
    let mut f = forth(Http::new().allow("127.0.0.1"));
    assert_eq!(run(&mut f, &format!("{} . TYPE", get(port, "/redirect"))), "200 hello");
    // localhost is not in the list: the second hop is refused.
    let e = f.interpret(&get(port, "/offsite")).unwrap_err();
    assert_eq!(e.code, NOT_PERMITTED);
    assert!(e.to_string().contains("localhost"), "{e}");
    let e = f.interpret(&get(port, "/loop")).unwrap_err();
    assert_eq!(e.code, HTTP_ERROR);
    assert!(e.to_string().contains("redirects"), "{e}");
}

#[test]
fn no_host_no_request() {
    let (port, seen) = serve();
    let mut f = forth(Http::new());
    assert_eq!(f.interpret(&get(port, "/hello")).unwrap_err().code, NOT_PERMITTED);
    let mut f = forth(Http::new().allow("example.org"));
    assert_eq!(f.interpret(&get(port, "/hello")).unwrap_err().code, NOT_PERMITTED);
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn the_network_can_be_denied() {
    let (port, seen) = serve();
    let mut f = forth(Http::new().allow("*"));
    f.permit(Permission::Network, false);
    assert_eq!(f.interpret(&get(port, "/hello")).unwrap_err().code, NOT_PERMITTED);
    assert!(seen.lock().unwrap().is_empty());
    // The sandbox denies it by itself.
    let mut f = Forth::sandboxed();
    f.install(&Http::new().allow("*"));
    f.interpret("ALSO HTTP").unwrap();
    assert!(!f.permits(Permission::Network));
    assert_eq!(f.interpret(&get(port, "/hello")).unwrap_err().code, NOT_PERMITTED);
    f.permit(Permission::Network, true);
    assert_eq!(run(&mut f, &format!("{} . TYPE", get(port, "/hello"))), "200 hello");
}

#[test]
fn hosts_outside_the_list_are_refused() {
    let http = Http::new().allow("API.Example.com").allow("*.cdn.org");
    for host in ["example.com", "cdn.org", "evilcdn.org", "api.example.com.evil.net"] {
        let mut forth = forth(http.clone());
        let e = forth.interpret(&format!("S\" http://{host}:1/\" HTTP-GET")).unwrap_err();
        assert_eq!(e.code, NOT_PERMITTED, "{host}");
    }
}

#[test]
fn bad_input_throws() {
    let mut f = forth(Http::new().allow("*"));
    for url in ["not a url", "ftp://example.org/", "file:///etc/passwd", "/relative"] {
        let e = f.interpret(&format!("S\" {url}\" HTTP-GET")).unwrap_err();
        assert_eq!(e.code, HTTP_ERROR, "{url}");
    }
    let e = f.interpret("S\" A:B\" S\" x\" HTTP-HEADER").unwrap_err();
    assert_eq!(e.code, HTTP_ERROR);
    let e = f.interpret("S\" X\" S\\\" a\\nb\" HTTP-HEADER").unwrap_err();
    assert_eq!(e.code, HTTP_ERROR);
}

#[test]
fn a_large_response_is_refused() {
    let (port, _) = serve();
    let mut f = forth(Http::new().allow("127.0.0.1"));
    let e = f.interpret(&get(port, "/big")).unwrap_err();
    assert_eq!(e.code, HTTP_ERROR);
    assert!(e.to_string().contains("1048576"), "{e}");
    let mut f = Forth::with_memory(8 << 20);
    f.install(&Http::new().allow("127.0.0.1").max_body(4 << 20));
    f.interpret("ALSO HTTP").unwrap();
    assert_eq!(run(&mut f, &format!("{} . NIP .", get(port, "/big"))), "200 2097152 ");
}

#[test]
fn the_time_left_bounds_the_request() {
    let (port, _) = serve();
    let mut f = Forth::new();
    f.set_limits(forthling::Limits {
        time: Some(Duration::from_millis(400)),
        ..Default::default()
    });
    f.install(&Http::new().allow("127.0.0.1"));
    f.interpret("ALSO HTTP").unwrap();
    let start = std::time::Instant::now();
    let e = f.interpret(&get(port, "/slow")).unwrap_err();
    assert_eq!(e.code, LIMIT_EXCEEDED);
    assert!(start.elapsed() < Duration::from_secs(2));
    // Without a time limit the request waits for its own timeout.
    let mut f = forth(Http::new().allow("127.0.0.1").timeout(Duration::from_millis(300)));
    let e = f.interpret(&get(port, "/slow")).unwrap_err();
    assert_eq!(e.code, HTTP_ERROR);
}

#[test]
fn a_refused_connection_can_be_caught() {
    // Nothing listens on the port a closed listener had.
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let mut f = forth(Http::new().allow("127.0.0.1"));
    let code = format!(": TRY  {} 2DROP . ;  : SAFE  ['] TRY CATCH . ;  SAFE", get(port, "/"));
    assert_eq!(run(&mut f, &code), "-263 ");
}
