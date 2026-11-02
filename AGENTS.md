# AGENTS.md

Guide for AI agents and contributors working on web-mirror.

## What this is

A Rust command line tool and library that mirrors a section of a website to
disk, like `wget --mirror`. Entry point: `src/main.rs` → `app::run::run`.

## Source of truth

- `spec/index.md` is the specification. Change the spec first, then the code,
  then the tests. Keep README.md, `--help` text, and the spec in agreement.
- `spec/compare/wget/index.md` maps wget options to ours;
  `spec/compare/index.md` surveys the other tools.
- `plan.md` (phases and why), `tasks.md` (work items with acceptance
  criteria), and `spec/proposals/index.md` (proposed behavior). When a task
  is built, move its proposal into `spec/index.md` and tick it in `tasks.md`.
- `spec/tests/` holds test rules and expected outputs:
  - `integration-network-tests-must-use-ignore/index.md`: every test that
    touches the network gets `#[ignore = "needs network access; ..."]`.
  - `www.nhs.uk/<section>/output.txt`: the exact files a live run must save.

## Commands

```sh
cargo build
cargo test                              # offline unit tests; must pass
cargo test -- --ignored                 # live-site tests (network, ~1 minute)
cargo clippy --all-targets              # must have no warnings
cargo doc --no-deps                     # must have no warnings
cargo audit && cargo deny check advisories
spec/compare/benchmarks/run.sh          # benchmarks vs wget (release build)
```

CI (`.github/workflows/ci.yml`) runs the offline checks above on Linux,
macOS, and Windows. Paths written into web-mirror's own files (events,
crawl state, `-k` record) use `url/from_path_into_slash_string.rs`, never
`Path::display`, so they are the same on Windows; tests compare saved names
with `testing::file_names_under`.

The build directory may be set by `CARGO_TARGET_DIR`; find the binary with
`cargo metadata --format-version 1 --no-deps` (`target_directory`).

## Code conventions

- One function per file for converters: `from_<input>_into_<output>.rs` holds
  `fn from_<input>_into_<output>`, with its unit tests in the same file.
- Deep directories by concern (see "Source directories" in `spec/index.md`).
  Security-relevant code lives where its directory name says: `scope/`
  (what may be fetched), `robots/`, `url/` (URL → path mapping), `disk/`
  (every file write), `http/` (every network request), `cookies/` (which
  cookies go where). `warc/`, `sitemap/`, and `convert/` are pure (no I/O).
- A directory's main type lives in a same-named file (`app/app.rs`), so
  `clippy::module_inception` is allowed in `src/lib.rs`.
- New command line option: add it in `app/clap.rs` (with help text and a parse
  test), `app/args.rs` (field, default, validation in `into_app`, test),
  `app/app.rs` (validated field and default), use it in the code, and document
  it in `spec/index.md`, `README.md`, and `spec/compare/wget/index.md`.
- Every public item has a doc comment that says what and why.
- Logs: `log::info!` for progress, `log::warn!` for per-URL failures (the
  crawl continues), `Error` for fatal problems.

## Security rules (do not weaken)

- Every path written comes from `url/from_mirror_layout_and_url_into_path_buf.rs`,
  which makes each segment safe with `url/from_str_into_safe_segment.rs`.
- Every write goes through `disk/` and checks
  `disk/ensure_no_symlink_below_root.rs` first.
- Every fetched URL passes `Scope::contains`; redirects stay on start-URL
  origins (`http/from_app_into_redirect_policy.rs`).
- Every response body is read with `http/read_body_with_limit.rs`.
- Never log or send credentials from URLs; redact with
  `url/from_url_into_redacted_url.rs`.
- Cookies go only to hosts in scope (`cookies/cookie_jar.rs`), are never
  logged (keep the `cookie_store` log filter in `app/run.rs`), and are saved
  only with `disk/write_private_file_atomically.rs` (mode 0600). WARC request
  records redact secret headers (`warc/from_exchange_into_http_message_bytes.rs`).
- `--check-external` is the only feature that requests URLs outside the
  scope. It must use `http/from_app_into_external_client.rs` (public
  addresses only, no secrets sent), never the crawl's client.
- web-mirror's own files in `.web-mirror/` (crawl state, `-k` record) are
  read as untrusted input: validate every line, and send every URL read back
  through the normal scope, filter, and robots.txt checks.
- With `--max-connections`, a request task does only `http::fetch::fetch`
  (and its rate-limit sleep); saving, the crawl state, events, and the
  queue stay in the crawl loop in `crawl/run.rs`, so no two tasks ever
  write the same file. Keep it that way.
- Durations from outside input go through `crawl/pause.rs`
  `from_seconds_into_duration`, which cannot panic.
- `cargo audit` and `cargo deny check advisories` pass with no exceptions;
  any future exception needs a written reason in `.cargo/audit.toml` and
  `deny.toml`.

## Offline end-to-end tests

Prefer these to live-site tests. In `src/crawl/run.rs` tests, start a
`TestServer` (in `src/app/testing.rs`) with a closure from `TestRequest` to
`TestResponse`, build an `App` for `server.url("/docs/")`, `run(&app).await`,
then check `Stats`, the saved files (`files_under`, which leaves out
`.web-mirror/`), and `server.paths()` (what was, and was not, requested).
Every run keeps a crawl state, so a second run on the same directory resumes
if the first stopped early (`-n`, `-Q`, `--max-time`); set `app.fresh = true`
to walk again from the start.

## Live-site tests

`src/app/clap.rs` has `assert_mirror_matches_expected_file`. A new live-site
test: put the expected file list in `spec/tests/<host>/<path>/output.txt`
(one path per line, sorted), add an `#[ignore]` test that calls the helper (which takes `live_site_lock`,
so live-site tests run one at a time)
with the start URL and options, and list it under "Tests" in `spec/index.md`.
If a live site changes, update its `output.txt`.
