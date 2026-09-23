use std::{
    fmt::Write as _,
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

use vut_compiler::{CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn scratch(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("{prefix}-{nonce}-{sequence}"));
    fs::create_dir_all(&root).expect("create scratch");
    root
}

/// Starts a minimal HTTP/1.1 server that returns canned responses.
fn start_server() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            counter.fetch_add(1, Ordering::Relaxed);
            let mut buffer = [0_u8; 8192];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let mut lines = request.lines();
            let request_line = lines.next().unwrap_or("");
            let mut parts = request_line.split_whitespace();
            let method = parts.next().unwrap_or("GET");
            let target = parts.next().unwrap_or("/");
            let path = target.split('?').next().unwrap_or("/");
            let range = request.lines().find_map(|line| {
                let lower = line.to_ascii_lowercase();
                lower
                    .starts_with("range:")
                    .then(|| {
                        line.split_once(':')
                            .map(|(_, value)| value.trim().to_owned())
                    })
                    .flatten()
            });
            let body = request.split_once("\r\n\r\n").map_or("", |(_, body)| body);

            if method == "GET" && path == "/chunked" {
                let mut response = String::from(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                );
                for chunk in ["aaaaa", "bbbbb", "ccccc"] {
                    write!(response, "{:x}\r\n{chunk}\r\n", chunk.len())
                        .expect("writing to a String cannot fail");
                }
                response.push_str("0\r\n\r\n");
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
                continue;
            }

            let (status, extra, payload) = match (method, path) {
                ("GET", "/missing") => ("404 Not Found", "", "nope".to_string()),
                ("GET", "/redirect") => ("302 Found", "Location: /\r\n", String::new()),
                ("POST", "/echo") => ("200 OK", "", body.to_string()),
                ("PUT", _) => ("200 OK", "", format!("put:{body}")),
                ("PATCH", _) => ("200 OK", "", format!("patch:{body}")),
                ("DELETE", _) => ("200 OK", "", "deleted".to_string()),
                ("HEAD", _) => ("200 OK", "", String::new()),
                ("OPTIONS", _) => ("204 No Content", "Allow: GET,POST\r\n", String::new()),
                ("GET", "/large") => ("200 OK", "", "x".repeat(100_000)),
                ("GET", "/range") => {
                    let full = "0123456789";
                    if let Some(spec) = range.as_deref() {
                        let spec = spec.trim_start_matches("bytes=");
                        let (start, end) = spec.split_once('-').unwrap_or(("0", "9"));
                        let start: usize = start.parse().unwrap_or(0);
                        let end: usize = end.parse().unwrap_or(9);
                        let slice = &full[start..=end.min(full.len() - 1)];
                        (
                            "206 Partial Content",
                            "Accept-Ranges: bytes\r\n",
                            slice.to_string(),
                        )
                    } else {
                        ("200 OK", "", full.to_string())
                    }
                }
                _ => ("200 OK", "", "hello".to_string()),
            };

            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (format!("http://{address}"), hits)
}

/// Starts an HTTP server whose `/slow` endpoint responds only after two
/// requests are in flight at once, so a serialized client cannot pass. Returns
/// the base URL and the maximum observed concurrent request count.
fn start_overlap_server() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    let max = Arc::new(AtomicUsize::new(0));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_out = Arc::clone(&max);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let max = Arc::clone(&max_out);
            let in_flight = Arc::clone(&in_flight);
            thread::spawn(move || {
                let mut buffer = [0_u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_owned();
                let body = if path.starts_with("/slow") {
                    let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    max.fetch_max(now, Ordering::SeqCst);
                    let deadline = Instant::now() + Duration::from_millis(2000);
                    while in_flight.load(Ordering::SeqCst) < 2 && Instant::now() < deadline {
                        thread::sleep(Duration::from_millis(1));
                    }
                    let result = "ok";
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    result
                } else {
                    "hello"
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            });
        }
    });
    (format!("http://{address}"), max)
}

fn run(source: &str) -> (Option<i32>, String) {
    let root = scratch("vut-http-e2e");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(runtime_library()),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .expect("emit executable");
    let output = Command::new(&executable).output().expect("run executable");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    fs::remove_dir_all(&root).ok();
    (output.status.code(), stdout)
}

/// Reports status, success, and body of a response through the public methods.
const REPORT: &str = "fn report(response: http.Response):\n  out(\"status=$(response.status.code)\")\n  out(\"success=$(response.is_success())\")\n  match response.text():\n    ok(text): out(\"body=$text\")\n    err(error): out(\"body-error\")\n";

#[test]
fn http_get_returns_status_and_body() {
    let (base, _hits) = start_server();
    let source = format!(
        "import http\n\n{REPORT}\nasync fn main():\n  match await http.get(\"{base}/\"):\n    ok(response): report(response)\n    err(error): out(\"failed\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "status=200\nsuccess=1\nbody=hello\n");
}

#[test]
fn http_not_found_is_a_valid_response() {
    let (base, _hits) = start_server();
    let source = format!(
        "import http\n\n{REPORT}\nasync fn main():\n  match await http.get(\"{base}/missing\"):\n    ok(response): report(response)\n    err(error): out(\"failed\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "status=404\nsuccess=0\nbody=nope\n");
}

#[test]
fn http_post_echoes_body() {
    let (base, _hits) = start_server();
    let source = format!(
        "import http\n\n{REPORT}\nasync fn main():\n  match await http.post(\"{base}/echo\", \"payload\"):\n    ok(response): report(response)\n    err(error): out(\"failed\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "status=200\nsuccess=1\nbody=payload\n");
}

#[test]
fn http_follows_redirects_by_default() {
    let (base, _hits) = start_server();
    let source = format!(
        "import http\n\n{REPORT}\nasync fn main():\n  match await http.get(\"{base}/redirect\"):\n    ok(response): report(response)\n    err(error): out(\"failed\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "status=200\nsuccess=1\nbody=hello\n");
}

#[test]
fn http_invalid_url_is_a_transport_error() {
    let source = "import http\n\nasync fn main():\n  match await http.get(\"not a url\"):\n    ok(response): out(\"unexpected\")\n    err(error): out(\"error\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "error\n");
}

#[test]
fn http_range_request_reports_partial_status() {
    let (base, _hits) = start_server();
    let source = format!(
        "import http\n\n{REPORT}\nasync fn main():\n  match await http.get_range(\"{base}/range\", 2, 5):\n    ok(response): report(response)\n    err(error): out(\"failed\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "status=206\nsuccess=1\nbody=2345\n");
}

#[test]
fn http_streaming_reads_the_body_in_chunks() {
    let (base, _hits) = start_server();
    let source = format!(
        "import http\n\nfn chunk_len(outcome: result[bytes, http.Error]) -> int:\n  value: int = match outcome:\n    ok(chunk): chunk.len()\n    err(error): 0\n  value\n\nasync fn drain(stream: http.Stream) -> int:\n  total = 0\n  for:\n    next = await stream.read()\n    length = chunk_len(next)\n    total = total + length\n    if length == 0:\n      break\n  total\n\nasync fn main():\n  stream = http.get_stream(\"{base}/large\")\n  total = await drain(stream)\n  out(\"total=$total\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "total=100000\n");
}

#[test]
fn http_stream_reads_multiple_chunks() {
    let (base, _hits) = start_server();
    let source = format!(
        "import http\n\nfn chunk_len(outcome: result[bytes, http.Error]) -> int:\n  value: int = match outcome:\n    ok(chunk): chunk.len()\n    err(error): 0\n  value\n\nasync fn drain(stream: http.Stream):\n  total = 0\n  chunks = 0\n  for:\n    next = await stream.read()\n    length = chunk_len(next)\n    if length == 0:\n      break\n    total = total + length\n    chunks = chunks + 1\n  out(\"total=$total chunks=$chunks\")\n\nasync fn main():\n  stream = http.get_stream(\"{base}/chunked\")\n  await drain(stream)\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "total=15 chunks=3\n");
}

#[test]
fn http_method_matrix_reaches_the_server() {
    let (base, _hits) = start_server();
    let source = format!(
        "import http\n\nasync fn main():\n  match await http.put(\"{base}/m\", \"p\"):\n    ok(response): out(\"put=$(response.status.code)\")\n    err(error): out(\"put-err\")\n  match await http.patch(\"{base}/m\", \"q\"):\n    ok(response): out(\"patch=$(response.status.code)\")\n    err(error): out(\"patch-err\")\n  match await http.delete(\"{base}/m\"):\n    ok(response): out(\"delete=$(response.status.code)\")\n    err(error): out(\"delete-err\")\n  match await http.head(\"{base}/m\"):\n    ok(response): out(\"head=$(response.status.code)\")\n    err(error): out(\"head-err\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "put=200\npatch=200\ndelete=200\nhead=200\n");
}

#[test]
fn http_request_builder_and_client_methods() {
    let (base, _hits) = start_server();
    let source = format!(
        "import http\nimport time\n\n{REPORT}\nasync fn main():\n  request = http.Request(method: http.Method.post, url: \"{base}/echo\")\n  request.header(\"X-Test\", \"abc\")\n  request.query(\"page\", \"2\")\n  request.body_text(\"built\")\n  request.timeout(time.seconds(5))\n  match await request.send():\n    ok(response): report(response)\n    err(error): out(\"failed\")\n  client = http.client(time.seconds(5), 5)\n  match await client.get(\"{base}/\"):\n    ok(response): out(\"client=$(response.status.code)\")\n    err(error): out(\"client-failed\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "status=200\nsuccess=1\nbody=built\nclient=200\n");
}

#[test]
fn http_requests_in_two_vutcons_overlap() {
    let (base, max_in_flight) = start_overlap_server();
    let source = format!(
        "import http\n\nasync fn main():\n  a = vut(async fn():\n    return await http.get(\"{base}/slow\")\n  )\n  b = vut(async fn():\n    return await http.get(\"{base}/slow\")\n  )\n  first = await a\n  second = await b\n  match first:\n    ok(response): out(\"first=$(response.status.code)\")\n    err(error): out(\"first-err\")\n  match second:\n    ok(response): out(\"second=$(response.status.code)\")\n    err(error): out(\"second-err\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "first=200\nsecond=200\n");
    assert_eq!(
        max_in_flight.load(Ordering::SeqCst),
        2,
        "the two HTTP requests should be in flight at once"
    );
}
