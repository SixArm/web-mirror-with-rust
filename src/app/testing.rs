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
        &stdout,
        inner,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Every file under `dir`, as sorted paths relative to `dir`.
#[allow(dead_code)]
pub fn files_under(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap() {
            let path = entry.unwrap().path();
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
