# Web Mirror

Web Mirror is a command line tool, and a Rust library, that mirrors a section
of a website to disk, like `wget --mirror`, with safe defaults.

```sh
web-mirror "https://www.nhs.uk/conditions/"
```

This creates `www.nhs.uk/conditions/...` in the current directory:

```text
www.nhs.uk/
  conditions/
    index.html
    acne/
      index.html
    ...
```

## Why

- **Never goes above the start URL**, like `wget --no-parent`, always.
- **Never downloads a file twice.** Existing files are read from disk to find
  their links, and an interrupted crawl resumes from its queue the next time
  (`--fresh` starts over). Use `--force` to re-download, or `-N` to
  re-download only what changed on the server (by date and `ETag`).
- **Polite by default.** Obeys `robots.txt` and its `Crawl-delay`; one
  request at a time; `--wait`, `--wait-jitter`, and `--rate-limit` for more.
  `--max-connections` fetches in parallel, still one request per host unless
  `--per-host` says more (12× faster on a local test site; see
  [Benchmarks](spec/compare/benchmarks/index.md)).
- **Safe with hostile sites.** Never writes outside the output directory,
  never follows a redirect to another site, never writes through a symbolic
  link, and bounds the memory any one response can use. See
  [Security](spec/index.md#security).

## Install

```sh
git clone https://github.com/SixArm/web-mirror-with-rust
cd web-mirror-with-rust
cargo install --path .
```

Needs Rust 1.99 or newer (`rust-version` in `Cargo.toml`).

## Examples

Mirror one level: an index page and every page it links to from its main
content, skipping the site header, navigation, and footer:

```sh
web-mirror "https://www.nhs.uk/conditions/" --level 1 --link-selector main
```

A copy you can browse offline: pages, their stylesheets, images, and
scripts, with links rewritten to the local files:

```sh
web-mirror "https://www.nhs.uk/mental-health/conditions/" --level 1 --link-selector "main p" -k -p -E
```

Add `-H -D nhs.uk` to include fonts and scripts from other NHS hosts, such as
`assets.nhs.uk`, and `--strip-query-params v` to fetch the site's
bot-detection script once instead of once per page.

Only the links inside paragraphs (for example, to skip promotional cards):

```sh
web-mirror "https://www.nhs.uk/mental-health/conditions/" --level 1 --link-selector "main p"
```

Download only the PDFs below a page (pages are still crawled for links, but
not saved):

```sh
web-mirror "https://example.com/reports/" --accept pdf
```

Be gentle: one request every 2–6 seconds, at most 200 KiB/s, stop after 100 MiB:

```sh
web-mirror "https://example.com/docs/" --wait 4 --wait-jitter --rate-limit 200k --quota 100m
```

Refresh an existing mirror, downloading only pages that changed:

```sh
web-mirror "https://example.com/docs/" --timestamping
```

Archive a crawl for replay tools such as pywb (`crawl.warc.gz` and
`crawl.cdx`), as well as saving the files:

```sh
web-mirror "https://example.com/docs/" -p --warc-file crawl
```

Mirror pages that need a login, with cookies exported from a browser:

```sh
web-mirror "https://example.com/members/" --load-cookies cookies.txt
```

Check links without saving anything, and log why URLs were skipped:

```sh
web-mirror "https://example.com/docs/" --spider --rejected-log rejected.tsv
```

Check links to other sites too, and list the pages with broken ones:

```sh
web-mirror "https://example.com/docs/" --spider --check-external
```

Keep your usual options in a file (TOML; the command line overrides it):

```sh
web-mirror "https://example.com/docs/" --config ~/.config/web-mirror.toml
```

Cut off a crawler trap without stopping the crawl: add a pattern such as
`/calendar/` to the file while it runs:

```sh
web-mirror "https://example.com/docs/" --ignore-file traps.txt
```

Watch a crawl as JSON, one line per URL (logs stay on standard error):

```sh
web-mirror "https://example.com/docs/" --events - | jq -c '{event, url, status}'
```

Mirror several sections, listed in a file (one URL per line, `#` comments):

```sh
web-mirror --input-file urls.txt
```

Try a crawl with a small limit first:

```sh
web-mirror "https://example.com/docs/" --number-limit 10
```

## Options

Run `web-mirror --help` for the full list. In brief:

| Area | Options |
| ---- | ------- |
| Scope and depth | `-l, --level`, `-n, --number-limit`, `--link-selector`, `-p, --page-requisites`, `--sitemap`, `--follow-js` |
| URL normalization | `--strip-query-params`, `--sort-query`, `--ignore-query` |
| Filters | `-A, --accept`, `-R, --reject`, `--ignore-file`, `--accept-mime`, `--reject-mime`, `--accept-regex`, `--reject-regex`, `-I, --include-directories`, `-X, --exclude-directories` |
| Input and reporting | `--config`, `-i, --input-file`, `--spider`, `--check-external`, `--rejected-log`, `--events`, `-v, --verbose`, `--log-file-create`, `--log-file-append`, `--test` |
| Politeness and limits | `-w, --wait`, `--wait-jitter`, `--rate-limit`, `--max-connections`, `--per-host`, `-Q, --quota`, `--max-time`, `--max-file-size`, `--ignore-robots` |
| Re-downloading and resuming | `--force`, `-N, --timestamping`, `--fresh` |
| Archiving | `--warc-file` |
| Offline browsing | `-k, --convert-links`, `-p, --page-requisites`, `-E, --adjust-extension`, `-H, --span-hosts` with `-D, --domains` |
| Output layout | `-P, --directory-prefix`, `-E, --adjust-extension`, `--no-directories`, `--no-host-directories`, `--cut-dirs`, `--default-page` |
| HTTP | `-U, --user-agent`, `--header`, `--referer`, `--max-redirect`, `-t, --tries`, `-T, --timeout`, `--load-cookies`, `--save-cookies`, `--keep-session-cookies` |

Sizes take `k`, `m`, or `g` suffixes (powers of 1024). For each option's
exact behavior, see [the specification](spec/index.md#options); for the
mapping to wget's options, see [Compare wget](spec/compare/wget/index.md).

Exit codes: `0` success, `1` fatal error, `2` finished but some URLs failed.

## As a library

```rust
use web_mirror::{App, Error, parse_start_url, run};

async fn mirror() -> Result<(), Error> {
    let mut app = App::new(parse_start_url("https://www.nhs.uk/conditions/")?);
    app.max_depth = Some(1);
    let stats = run(&app).await?;
    println!("{} downloaded, {} from disk", stats.downloaded, stats.from_disk);
    Ok(())
}
```

## Development

```sh
cargo test                     # offline unit, end-to-end, and property tests
cargo test -- --ignored        # live-site integration tests (network)
cargo fmt                      # format (rustfmt.toml)
cargo clippy --all-targets     # lints
cargo doc --no-deps            # API docs
cargo audit && cargo deny check advisories   # security advisories
spec/compare/benchmarks/run.sh # benchmarks against wget
```

Fuzz targets for the parsers and the URL → path mapping are in
[fuzz/](fuzz/README.md) (nightly Rust and `cargo-fuzz`).

CI ([.github/workflows/ci.yml](.github/workflows/ci.yml)) runs the offline
checks on Linux, macOS, and Windows on every push and pull request, and checks
the build with the minimum Rust version.

The specification, [spec/index.md](spec/index.md), is the single source of
truth. The roadmap is in [plan.md](plan.md) and [tasks.md](tasks.md), based on
a [survey of similar tools](spec/compare/index.md). Contributors and AI agents: read [AGENTS.md](AGENTS.md).

## License

`MIT OR Apache-2.0 OR GPL-2.0-only OR GPL-3.0-only OR BSD-3-Clause`, at your
option, or contact us for other terms. See [LICENSE.md](LICENSE.md).
