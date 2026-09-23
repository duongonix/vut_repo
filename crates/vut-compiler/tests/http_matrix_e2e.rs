use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, SystemTime},
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

/// The routed response: status line suffix, extra headers, and raw payload.
type Routed = (String, String, Vec<u8>);

fn route(method: &str, path: &str, query: &str, headers: &str, body: &[u8]) -> Routed {
    if method == "OPTIONS" {
        return (
            "204 No Content".to_owned(),
            "Allow: GET,POST,OPTIONS\r\n".to_owned(),
            Vec::new(),
        );
    }
    if let Some(code) = path.strip_prefix("/status/") {
        let code: u16 = code.parse().unwrap_or(200);
        return (format!("{code} Status"), String::new(), Vec::new());
    }
    match path {
        "/binary" => ("200 OK".to_owned(), String::new(), vec![0xff, 0xfe, 0xfd]),
        "/query" => (
            "200 OK".to_owned(),
            String::new(),
            query.as_bytes().to_vec(),
        ),
        "/echo" => ("200 OK".to_owned(), String::new(), body.to_vec()),
        "/headers" => (
            "200 OK".to_owned(),
            String::new(),
            headers.as_bytes().to_vec(),
        ),
        "/multi" => (
            "200 OK".to_owned(),
            "Set-Cookie: a=1\r\nSet-Cookie: b=2\r\n".to_owned(),
            b"ok".to_vec(),
        ),
        "/loop" => (
            "302 Found".to_owned(),
            "Location: /loop\r\n".to_owned(),
            Vec::new(),
        ),
        "/redirect-once" => (
            "302 Found".to_owned(),
            "Location: /\r\n".to_owned(),
            Vec::new(),
        ),
        "/slow" => {
            let ms: u64 = query
                .split('&')
                .find_map(|pair| pair.strip_prefix("ms="))
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            thread::sleep(Duration::from_millis(ms));
            ("200 OK".to_owned(), String::new(), b"slow".to_vec())
        }
        _ => ("200 OK".to_owned(), String::new(), b"hello".to_vec()),
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(read) => read,
        };
        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
        let request_line = request.lines().next().unwrap_or("");
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("GET").to_owned();
        let target = parts.next().unwrap_or("/").to_owned();
        let (path, query) = target
            .split_once('?')
            .map_or((target.as_str(), ""), |(path, query)| (path, query));
        let close = request.to_ascii_lowercase().contains("connection: close");
        let (headers, body) = request
            .split_once("\r\n\r\n")
            .map_or(("", ""), |(headers, body)| (headers, body));

        let (status, extra, payload) = route(&method, path, query, headers, body.as_bytes());
        let connection = if close { "Connection: close\r\n" } else { "" };
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\n{extra}Content-Length: {}\r\n{connection}\r\n",
            payload.len()
        );
        if stream.write_all(head.as_bytes()).is_err() || stream.write_all(&payload).is_err() {
            return;
        }
        let _ = stream.flush();
        if close {
            return;
        }
    }
}

/// Starts a keep-alive HTTP server and returns its base URL plus the number of
/// accepted connections (one per TCP connection, reused across requests).
fn start_server() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    let connections = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&connections);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            counter.fetch_add(1, Ordering::SeqCst);
            thread::spawn(move || handle_connection(stream));
        }
    });
    (format!("http://{address}"), connections)
}

fn run(source: &str) -> (Option<i32>, String) {
    let root = scratch("vut-http-matrix");
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

/// Maps every `http.ErrorKind` variant to a stable string for assertions.
const KIND_NAME: &str = "fn kind_name(kind: http.ErrorKind) -> str:\n  name: str = match kind:\n    invalid_url: \"invalid_url\"\n    dns: \"dns\"\n    connect: \"connect\"\n    tls: \"tls\"\n    timeout: \"timeout\"\n    redirect: \"redirect\"\n    request: \"request\"\n    response: \"response\"\n    body: \"body\"\n    cancelled: \"cancelled\"\n    unsupported: \"unsupported\"\n    other: \"other\"\n  name\n";

/// Prints the transport error kind of an outcome.
const SHOW_ERROR: &str =
    "fn show_error(error: http.Error):\n  name = kind_name(error.kind)\n  out(\"kind=$name\")\n";

/// Extracts the response text, or the string `\"text-error\"` on failure.
const TEXT_OR: &str = "fn text_or(response: http.Response) -> str:\n  text: str = match response.text():\n    ok(body): body\n    err(error): \"text-error\"\n  text\n";

#[test]
fn options_reaches_the_server() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\nasync fn main():\n  match await http.options(\"{base}/\"):\n    ok(response): out(\"status=$(response.status.code)\")\n    err(error): out(\"error\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "status=204\n");
}

#[test]
fn status_classification_helpers_are_correct() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\nfn flag_text(response: http.Response) -> str:\n  inf = response.status.is_informational()\n  success = response.status.is_success()\n  redirect = response.status.is_redirect()\n  client = response.status.is_client_error()\n  server = response.status.is_server_error()\n  \"$inf,$success,$redirect,$client,$server\"\n\nasync fn flags(url: str) -> str:\n  value: str = match await http.get(url):\n    ok(response): flag_text(response)\n    err(error): \"err\"\n  value\n\nasync fn main():\n  a = await flags(\"{base}/status/200\")\n  b = await flags(\"{base}/status/301\")\n  c = await flags(\"{base}/status/404\")\n  d = await flags(\"{base}/status/500\")\n  out(\"200=$a\")\n  out(\"301=$b\")\n  out(\"404=$c\")\n  out(\"500=$d\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "200=0,1,0,0,0\n301=0,0,1,0,0\n404=0,0,0,1,0\n500=0,0,0,0,1\n"
    );
}

#[test]
fn client_and_server_errors_are_valid_responses() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\nasync fn main():\n  match await http.get(\"{base}/status/503\"):\n    ok(response): out(\"ok success=$(response.is_success())\")\n    err(error): out(\"transport\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "ok success=0\n");
}

#[test]
fn multi_value_response_headers_are_preserved() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\nfn show_multi(response: http.Response):\n  first = response.header(\"Set-Cookie\")\n  has = response.headers.raw().contains(\"b=2\")\n  out(\"first=$first\")\n  out(\"second=$has\")\n\nasync fn main():\n  match await http.get(\"{base}/multi\"):\n    ok(response): show_multi(response)\n    err(error): out(\"error\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "first=a=1\nsecond=1\n");
}

#[test]
fn multi_value_request_headers_reach_the_server() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\nfn show_headers(response: http.Response):\n  body = text_or(response)\n  a = body.contains(\"x-multi: a\")\n  b = body.contains(\"x-multi: b\")\n  out(\"a=$a b=$b\")\n\n{TEXT_OR}\nasync fn main():\n  request = http.Request(method: http.Method.get, url: \"{base}/headers\")\n  request.header(\"X-Multi\", \"a\")\n  request.header(\"X-Multi\", \"b\")\n  match await request.send():\n    ok(response): show_headers(response)\n    err(error): out(\"error\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a=1 b=1\n");
}

#[test]
fn query_parameters_are_percent_encoded() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\n{TEXT_OR}\nfn show_query(response: http.Response):\n  body = text_or(response)\n  out(\"query=$body\")\n\nasync fn main():\n  request = http.Request(method: http.Method.get, url: \"{base}/query\")\n  request.query(\"q\", \"a b&c\")\n  match await request.send():\n    ok(response): show_query(response)\n    err(error): out(\"error\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "query=q=a+b%26c\n");
}

#[test]
fn bytes_body_round_trips() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\n{TEXT_OR}\nfn show_body(response: http.Response):\n  body = text_or(response)\n  out(\"echo=$body\")\n\nasync fn main():\n  match await http.post_bytes(\"{base}/echo\", \"bytes-body\".to_bytes()):\n    ok(response): show_body(response)\n    err(error): out(\"error\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "echo=bytes-body\n");
}

#[test]
fn invalid_utf8_text_is_a_typed_body_error() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\n{KIND_NAME}{SHOW_ERROR}fn show_text_kind(response: http.Response):\n  match response.text():\n    ok(text): out(\"text\")\n    err(error): show_error(error)\n\nasync fn main():\n  match await http.get(\"{base}/binary\"):\n    ok(response): show_text_kind(response)\n    err(error): out(\"transport\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "kind=body\n");
}

#[test]
fn timeout_is_a_typed_timeout_error() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\nimport time\n\n{KIND_NAME}{SHOW_ERROR}async fn main():\n  config = http.client(time.milliseconds(200), 2)\n  match await config.get(\"{base}/slow?ms=2000\"):\n    ok(response): out(\"ok\")\n    err(error): show_error(error)\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "kind=timeout\n");
}

#[test]
fn redirect_limit_is_a_typed_redirect_error() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\nimport time\n\n{KIND_NAME}{SHOW_ERROR}async fn main():\n  config = http.client(time.nanoseconds(0), 2)\n  match await config.get(\"{base}/loop\"):\n    ok(response): out(\"ok=$(response.status.code)\")\n    err(error): show_error(error)\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "kind=redirect\n");
}

#[test]
fn a_client_with_only_a_timeout_still_follows_redirects() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\nimport time\n\nasync fn main():\n  config = http.client(time.seconds(5), 0)\n  match await config.get(\"{base}/redirect-once\"):\n    ok(response): out(\"status=$(response.status.code)\")\n    err(error): out(\"error\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "status=200\n");
}

#[test]
fn response_stream_reads_a_buffered_body() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\nfn chunk_len(outcome: result[bytes, http.Error]) -> int:\n  value: int = match outcome:\n    ok(chunk): chunk.len()\n    err(error): 0\n  value\n\nasync fn drain(stream: http.Stream) -> int:\n  total = 0\n  for:\n    next = await stream.read()\n    length = chunk_len(next)\n    if length == 0:\n      break\n    total = total + length\n  total\n\nasync fn main():\n  match await http.get(\"{base}/\"):\n    ok(response): out(\"total=$(await drain(response.stream()))\")\n    err(error): out(\"error\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "total=5\n");
}

#[test]
fn request_send_stream_streams_any_method() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\n\nfn chunk_len(outcome: result[bytes, http.Error]) -> int:\n  value: int = match outcome:\n    ok(chunk): chunk.len()\n    err(error): 0\n  value\n\nasync fn main():\n  request = http.Request(method: http.Method.post, url: \"{base}/echo\")\n  request.body_text(\"streamed\")\n  stream = request.send_stream()\n  total = 0\n  for:\n    next = await stream.read()\n    length = chunk_len(next)\n    if length == 0:\n      break\n    total = total + length\n  out(\"total=$total\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "total=8\n");
}

#[test]
fn sequential_requests_reuse_one_connection() {
    let (base, connections) = start_server();
    let source = format!(
        "import http\n\nasync fn main():\n  match await http.get(\"{base}/\"):\n    ok(response): out(\"first=$(response.status.code)\")\n    err(error): out(\"error\")\n  match await http.get(\"{base}/\"):\n    ok(response): out(\"second=$(response.status.code)\")\n    err(error): out(\"error\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "first=200\nsecond=200\n");
    assert_eq!(
        connections.load(Ordering::SeqCst),
        1,
        "sequential requests should reuse the pooled connection"
    );
}

#[test]
fn headers_type_has_multi_value_and_case_insensitive_semantics() {
    let source = "import http\n\nfn main():\n  headers = http.Headers()\n  headers.append(\"X-Test\", \"1\")\n  headers.append(\"x-test\", \"2\")\n  headers.set(\"Accept\", \"text/plain\")\n  first = headers.get(\"X-TEST\")\n  has = headers.contains(\"accept\")\n  headers.remove(\"Accept\")\n  after = headers.contains(\"accept\")\n  out(\"first=$first\")\n  out(\"has=$has\")\n  out(\"after=$after\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "first=1\nhas=1\nafter=0\n");
}

/// An `https://` request against a plaintext listener must fail the TLS
/// handshake with a typed transport error (never a successful response), which
/// exercises the HTTPS code path deterministically without trusting a test CA.
#[test]
fn https_to_a_plain_listener_is_a_transport_error() {
    let (base, _connections) = start_server();
    let https = base.replace("http://", "https://");
    let source = format!(
        "import http\nimport time\n\n{KIND_NAME}async fn main():\n  config = http.client(time.seconds(5), 2)\n  match await config.get(\"{https}/\"):\n    ok(response): out(\"ok\")\n    err(error): out(\"kind=$(kind_name(error.kind))\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        stdout.starts_with("kind="),
        "expected a transport error, got: {stdout}"
    );
}

#[test]
fn client_request_response_and_stream_drop_cleanly() {
    let (base, _connections) = start_server();
    let source = format!(
        "import http\nimport time\n\nasync fn main():\n  config = http.client(time.seconds(5), 2)\n  request = config.request(http.Method.get, \"{base}/\")\n  match await config.execute(request):\n    ok(response): out(\"status=$(response.status.code)\")\n    err(error): out(\"error\")\n  stream = config.get_stream(\"{base}/\")\n  match await stream.read():\n    ok(chunk): out(\"chunk=$(chunk.len())\")\n    err(error): out(\"error\")\n  out(\"done\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "status=200\nchunk=5\ndone\n");
}
