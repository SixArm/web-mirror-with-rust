//! Testing helpers, such as directories and files.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The path to the freshly built `web-mirror` command line program.
///
/// Builds it once first, because `cargo test` does not necessarily rebuild the
/// binary, and the binary sits next to the `deps` directory that holds the
/// running test executable (this also works when `CARGO_TARGET_DIR` is set).
#[allow(dead_code)]
pub static COMMAND_OS: LazyLock<OsString> = LazyLock::new(|| {
    let status = std::process::Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--bin", env!("CARGO_PKG_NAME")])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("run cargo build");
    assert!(status.success(), "cargo build failed");
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("target profile directory")
        .join(env!("CARGO_PKG_NAME"))
        .into_os_string()
});

/// Assert that the command's stdout contains `inner`, whatever its exit status.
#[allow(dead_code)]
pub fn assert_command_stdout_contains(mut command: std::process::Command, inner: &str) {
    let output = command.output().expect("run command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(inner),
        "stdout: {:?}\n inner: {}\n stderr: {:?}\n",
        stdout,
        inner,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Every file under `dir`, as sorted paths relative to `dir`, leaving out
/// web-mirror's own `dir/.web-mirror/` (the crawl state is not part of the mirror).
#[allow(dead_code)]
pub fn files_under(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap() {
            let path = entry.unwrap().path();
            // web-mirror's own state directory is not part of the mirror.
            if path.is_dir() && path.file_name().is_some_and(|n| n == crate::disk::saved_record::STATE_DIR) && d == dir {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path.strip_prefix(dir).unwrap().to_path_buf());
            }
        }
    }
    files.sort();
    files
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A new, empty, unique directory under the system temp directory.
/// The caller removes it when done.
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "web-mirror-test-{}-{}-{}",
        name,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Held by every live-site test, so they run one at a time even though
/// `cargo test` runs tests in parallel. Several crawls of one site at once
/// look like a flood to it, and it may start answering 429 Too Many Requests.
#[allow(dead_code)]
pub static LIVE_SITE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Take the [`LIVE_SITE`] lock. A test that panicked while holding it leaves
/// it "poisoned"; that is harmless here, so carry on.
#[allow(dead_code)]
pub fn live_site_lock() -> std::sync::MutexGuard<'static, ()> {
    LIVE_SITE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One HTTP request, as the [`TestServer`] saw it.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestRequest {
    /// The path and query, such as `/docs/?x=1`.
    pub path: String,
    /// Header names, lower-cased, and their values.
    pub headers: Vec<(String, String)>,
}

impl TestRequest {
    /// The value of the header `name` (lower-case), if sent.
    #[allow(dead_code)]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

/// One HTTP response for the [`TestServer`] to send.
#[allow(dead_code)]
pub struct TestResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[allow(dead_code)]
impl TestResponse {
    /// `200 OK` with an HTML body.
    pub fn html(body: &str) -> Self {
        Self::new(200, body.as_bytes()).header("Content-Type", "text/html")
    }

    /// Any status and body.
    pub fn new(status: u16, body: &[u8]) -> Self {
        Self { status, headers: Vec::new(), body: body.to_vec() }
    }

    /// `404 Not Found`.
    pub fn not_found() -> Self {
        Self::new(404, b"")
    }

    /// Add a header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

/// The function a [`TestServer`] calls for each request.
type Handler = dyn Fn(&TestRequest) -> TestResponse + Send + Sync + 'static;

/// A tiny HTTP/1.1 server on `127.0.0.1` with a random free port, for
/// end-to-end crawl tests without the network. Each connection serves one
/// request and closes. Every request is recorded, so tests can check what
/// was (and was not) asked for. The server stops when the test process ends.
#[allow(dead_code)]
pub struct TestServer {
    /// Such as `http://127.0.0.1:54321`, with no trailing slash.
    pub base: String,
    requests: std::sync::Arc<std::sync::Mutex<Vec<TestRequest>>>,
}

#[allow(dead_code)]
impl TestServer {
    /// Start a server that answers with `handler`.
    pub fn start(handler: impl Fn(&TestRequest) -> TestResponse + Send + Sync + 'static) -> Self {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let handler: std::sync::Arc<Handler> = std::sync::Arc::new(handler);
        let recorded = requests.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let handler = handler.clone();
                let recorded = recorded.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_err() {
                        return;
                    }
                    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let mut headers = Vec::new();
                    loop {
                        let mut h = String::new();
                        if reader.read_line(&mut h).unwrap_or(0) == 0 || h.trim().is_empty() {
                            break;
                        }
                        if let Some((n, v)) = h.split_once(':') {
                            headers.push((n.trim().to_ascii_lowercase(), v.trim().to_string()));
                        }
                    }
                    let request = TestRequest { path, headers };
                    recorded.lock().unwrap().push(request.clone());
                    let response = handler(&request);
                    let mut out = format!("HTTP/1.1 {} Test\r\n", response.status);
                    for (n, v) in &response.headers {
                        out.push_str(&format!("{n}: {v}\r\n"));
                    }
                    out.push_str(&format!(
                        "Content-Length: {}\r\nConnection: close\r\n\r\n",
                        response.body.len()
                    ));
                    let mut stream = stream;
                    let _ = stream.write_all(out.as_bytes());
                    let _ = stream.write_all(&response.body);
                });
            }
        });
        Self { base, requests }
    }

    /// The full URL for `path` on this server, such as `url("/docs/")`.
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// Every request so far, in arrival order.
    pub fn requests(&self) -> Vec<TestRequest> {
        self.requests.lock().unwrap().clone()
    }

    /// Every requested path so far, in arrival order.
    pub fn paths(&self) -> Vec<String> {
        self.requests().into_iter().map(|r| r.path).collect()
    }
}
