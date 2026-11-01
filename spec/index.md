# Web Mirror

Rust command line tool to do website mirroring. Similar to command `wget --mirror`.

## Example

```sh
web-mirror "https://www.nhs.uk/conditions/"
```

Output is a directory tree rooted at the host name:

```text
www.nhs.uk/
  conditions/
    index.html
    acne/
      index.html
    ...
```

## Defaults

- **Never go above the start URL.** Every fetched URL must start with the start
  URL (same scheme family, host, port, and path prefix). Like `wget --no-parent`.
- **Never download twice.** If a URL's local file already exists, do not fetch
  it; read it from disk and parse its links so the crawl can continue below it.
  A file saved as `.html` or `.htm` is read as HTML; other files are sniffed.
  (Option `--force` re-downloads.)
- **Follow URLs found in HTML** (`a`, `area`, `link`, `img`, `script`, `iframe`,
  `frame`, `source`, `video`, `audio`, `track`, `embed`; `href`, `src`,
  `srcset`; `<base href>` is honoured).
- **Do not follow URLs found in JavaScript.** Option `--follow-js` opts in.
- Fragments are dropped; non-http(s) schemes (`mailto:`, `javascript:`) are ignored.
- Files are written atomically (`.part` then rename), so an interrupted run never
  leaves a truncated file that a later run would mistake for complete.

## URL to file mapping

- `https://h/a/b/` → `h/a/b/index.html`
- `https://h/a/b` (no extension) → `h/a/b/index.html`
- `https://h/a/b.pdf` → `h/a/b.pdf`
- query `?x=1` → added to the file stem: `index@x=1.html`
- Path segments are not percent-decoded; unsafe characters become `_`; the
  result can never leave the output directory.

## Options

Modeled on `wget`:

- `-v`, `--verbose`: repeat to set the log level: `-v` error, `-vv` warn, `-vvv`
  info, `-vvvv` debug, `-vvvvv` trace (default: `RUST_LOG`, else info)
- `--test`: print the parsed arguments, for debugging
- `-w`, `--wait=SECONDS`: wait SECONDS between retrievals (network only; reading
  from disk is not delayed)
- `--wait-jitter`: wait from 0.5*WAIT...1.5*WAIT seconds between retrievals
- `-P`, `--directory-prefix=DIR`: output directory (default `.`)
- `-l`, `--level=N`: maximum link depth (default 0 = unlimited)
- `-U`, `--user-agent=AGENT`
- `-t`, `--tries=N`: attempts per URL (default 3). Network errors, 5xx, and
  429 are retried after the server's `Retry-After` (seconds or an HTTP date,
  at most 5 minutes), or else after 1 s, 2 s, ...
- `-T`, `--timeout=SECONDS` (default 30)
- `--max-file-size=SIZE`: skip any one file larger than SIZE (k/m/g suffix;
  default 1g; 0 is unlimited). Downloads are held in memory before saving, so
  this bounds memory use.
- `--force`: re-download files that already exist
- `-n, --number-limit=NUMBER`: process at most NUMBER URLs (0 is unlimited)
- `--rate-limit=RATE`: limit download speed to RATE bytes/second, with optional k/m/g suffix, like `wget --limit-rate` (0 is unlimited)
- `--link-selector=CSS`: follow only links inside elements matching the CSS selector, e.g. `main` to skip site-wide header, navigation, and footer links
- `--follow-js`: also follow URL-looking strings in JavaScript

Input and reporting:

- `-i`, `--input-file=FILE`: also read start URLs from FILE, one per line (`-`
  is standard input; blank lines and `#` comments are skipped). The URL
  argument becomes optional. The scope is the union of every start URL's scope,
  and `robots.txt` is fetched once per origin.
- `--spider`: crawl and check links, but never save files (and never read
  them from disk). Failures are logged and give exit code 2, so this works as
  a link checker.
- `--rejected-log=FILE`: write each URL that was found but not fetched to FILE,
  tab-separated: `REASON`, `URL`, `PARENT_URL`. Reasons: `SCOPE`, `ROBOTS`,
  `REGEX`, `SUFFIX`, `DIRECTORY`. Each URL is logged at most once.

URL filtering (applies to discovered links; start URLs are always fetched):

- `-A`, `--accept=LIST`: save only files whose names end with a suffix in LIST,
  or match a wildcard pattern in LIST (`*`, `?`). Comma-separated; may be
  repeated. As with wget, pages (names ending `.html`/`.htm`, or with no
  extension) that are not accepted are still fetched to follow their links,
  but are not saved. Matching is case-sensitive, as with wget.
- `-R`, `--reject=LIST`: the opposite of `--accept`.
- `--accept-regex=REGEX`, `--reject-regex=REGEX`: match against the whole URL;
  a URL that fails is not fetched at all.
- `-I`, `--include-directories=LIST`, `-X`, `--exclude-directories=LIST`:
  directories such as `/conditions/acne`, matched on whole path segments
  (`/a` matches `/a` and `/a/b`, not `/ab`); wildcards allowed.

Output layout:

- `--no-host-directories`: do not create the host directory (wget's `-nH`;
  there is no `-nH` short form because `-n` is `--number-limit`).
- `--cut-dirs=N`: drop the first N directories of each URL path.
- `--default-page=NAME`: save directory URLs as NAME instead of `index.html`.
  NAME must be a plain file name.
- `-Q`, `--quota=SIZE`: stop starting new downloads once SIZE bytes have been
  downloaded in total (k/m/g suffix; 0 is unlimited). As with wget, the file
  that crosses the quota is finished.

HTTP:

- `-N`, `--timestamping`: for files that already exist, send `If-Modified-Since`
  with the file's time, and download only if the server's copy is newer.
  Conflicts with `--force`. Saved files always get the server's
  `Last-Modified` time, so this works on files from earlier runs.
- `--header=STRING`: send an extra request header such as
  `Accept-Language: en-GB`; may be repeated.
- `--referer=URL`: send `Referer: URL` with every request.
- `--max-redirect=N`: follow at most N redirects per request (default 20; 0
  follows none, and a redirect is then a failure).
- `--ignore-robots`: do not fetch or obey `robots.txt`
- `--log-file-create=FILE`: log messages to FILE instead of the console,
  replacing FILE if it exists (like `wget -o`)
- `--log-file-append=FILE`: log messages to FILE instead of the console,
  adding to FILE if it exists (like `wget -a`); the two log-file options
  conflict; file logs have timestamps

## Exit codes

- `0`: success.
- `1`: fatal error before or during the crawl, such as an invalid argument or
  an unreadable `robots.txt`.
- `2`: the crawl finished, but some URLs failed.

The final log line summarizes the run: downloaded (and bytes), read from disk,
unchanged (`--timestamping`), blocked by `robots.txt`, rejected by filters,
and failed.

## Tech stack

Rust crates:

- `clap`: command line argument parsing
- `confy`: configuration management
- `scraper`: web page HTML parser
- `reqwest`: HTTP client (rustls TLS with the system's certificate roots,
  HTTP/2, gzip/brotli/deflate/zstd decoding, system proxy settings)
- `thiserror`: error handling with specifics
- `tokio`: asynchronous runtime
- `url`: link parser/validator
- `regex`, `fastrand`: JS URL heuristic, `--accept-regex`/`--reject-regex`, and `--wait-jitter` randomness
- `httpdate`: HTTP dates for `--timestamping`

## Design notes

- The crawl loop (queue, visited set, scope check, disk cache) is ours, on top
  of the `reqwest` HTTP client. An earlier version used the `spider` crawler
  crate, but its `Website::crawl` fetches every page itself and cannot be told
  "use the file on disk instead", which is a core requirement, so only its
  re-exported `reqwest` client was used; depending on `reqwest` directly
  dropped about 200 unused crates (and the advisories that came with them).
- Fetching is sequential, as with `wget`, so `--wait` is meaningful.
- Non-goals for now: `--convert-links` link rewriting, cookies, parallel fetching.

## robots.txt

- On by default, as with `wget`. `--ignore-robots` turns it off.
- `/robots.txt` is fetched once per origin (scheme, host, port) of the start
  URLs, before any page. It is read up to 500 KiB.
- Semantics follow RFC 9309: groups by `User-agent` (a group naming our product
  token, such as `web-mirror`, beats `*`); longest matching pattern wins and
  `Allow` wins a tie; `*` and a trailing `$` work in patterns; query strings
  are matched.
- `Crawl-delay` is honoured as a floor under `--wait` (the largest across all
  sites, cut to one day).
- A missing `robots.txt` (4xx other than 429) means no restrictions. A network
  error, 5xx, or 429 Too Many Requests after retries stops the run with an error (RFC 9309 says to assume disallow-all)
  unless `--ignore-robots` is given.
- Disallowed URLs are never queued, fetched, or read from disk, and are counted
  as "blocked" in the final summary.

## Security

web-mirror downloads content from sites it does not control, so it treats
every byte from the network as hostile. Guarantees, and where they live:

- **Files are only written inside the output directory.** Every URL path
  segment is made safe (`url/from_str_into_safe_segment.rs`): path separators,
  control and Windows-reserved characters become `_`; `.`, `..`, and the empty
  name can never appear; trailing dots and spaces and Windows device names
  (`CON`, `nul.txt`) are neutralized. `--default-page` must be a plain file
  name.
- **Never writes through a symbolic link.** Before a write, every existing part
  of the path below the output directory is checked, and a symbolic link is
  refused (`disk/ensure_no_symlink_below_root.rs`). The `.part` file is created
  fresh with `create_new`, so it can never be a pre-placed link
  (`disk/write_atomically.rs`).
- **Never requests another site.** A redirect is followed only if it stays on
  the origin of a start URL, so a page cannot steer requests to other hosts,
  local network machines, or cloud metadata addresses
  (`http/from_app_into_redirect_policy.rs`). A redirect's final URL must
  also be in scope (`http/fetch.rs`).
- **Never sends or logs credentials from URLs.** Start URLs may not contain a
  user name or password (use `--header "Authorization: ..."`); links that do
  are out of scope; the rejected log redacts them.
- **Bounded memory per response.** Bodies are streamed and cut off at
  `--max-file-size` (default 1 GiB), whatever `Content-Length` claims, and
  `robots.txt` is read up to 500 KiB (`http/read_body_with_limit.rs`).
- **Bounded URLs.** URLs longer than 4096 bytes are out of scope, which stops
  crawler traps that grow URLs without end.
- **No crashes from hostile values.** A huge or infinite `Crawl-delay` or
  `--wait` is cut to one day instead of overflowing (`crawl/pause.rs`).
- **No slow paths on hostile input.** Regexes use the linear-time `regex`
  crate; wildcard matching has no recursion.
- **Logs cannot be forged by URLs.** The `url` crate percent-encodes control
  characters, tabs, and newlines, so a URL cannot add lines to the log or the
  rejected log.

Limits, by design:

- Crawls without `-l`, `-n`, or `-Q` are unbounded in time and disk; a site
  with endless distinct URLs needs one of them.
- The symbolic link check cannot stop another local user who can write to the
  output directory at the same moment; mirror into a directory only you can
  write to.
- Fetching is plain HTTP(S) with certificate checks on; there is no option to
  turn them off.

## Tests

Unit tests live beside the code they test. `cargo test` runs them.

End-to-end crawl tests in `src/crawl/run.rs` run the real crawl against a
local test server (`TestServer` in `src/app/testing.rs`), so they need no
network and run with every `cargo test`. They cover scope, `robots.txt`
(including 429), never downloading twice, `--link-selector`, `--accept`,
`--reject`, `--spider`, `--rejected-log`, `--quota`, `--number-limit`,
`--timestamping`, refused off-site redirects, `--max-file-size`,
`Retry-After`, and refusing to write through a symbolic link.

Integration tests that use the network are marked `#[ignore]` (see
`spec/tests/integration-network-tests-must-use-ignore/index.md`); run them
with `cargo test -- --ignored`. They take turns (`live_site_lock` in
`src/app/testing.rs`), so a site never sees several crawls at once. Each
live-site test compares the files a run
saves with an expected list:

- `spec/tests/www.nhs.uk/conditions/output.txt`:
  `web-mirror "https://www.nhs.uk/conditions/" --level 1 --link-selector main`
- `spec/tests/www.nhs.uk/medicines/output.txt`:
  `web-mirror "https://www.nhs.uk/medicines/" --level 1 --link-selector main`
- `spec/tests/www.nhs.uk/mental-health/conditions/output.txt`:
  `web-mirror "https://www.nhs.uk/mental-health/conditions/" --level 1 --link-selector "main p"` Library
doctests are off (`doctest = false`) because `src/types/` has illustrative,
non-runnable examples.

## Project file guidance

`AGENTS.md` is the short guide for contributors and AI agents; this file is
the specification, and the single source of truth for behavior. Change this
file first, then the code, then the tests.

Converter functions: use one file per function. The file name convention is
`from_*_into_*.rs` and the function name convention is `from_*_into_*`, and the
file name equals the function name (for example
`url/from_str_into_start_url.rs` holds `fn from_str_into_start_url`). Each
converter's unit tests live in the same file.

Organization: favor deep organization of files, not a flat layout of files.
The purpose is auditability for security reviewers: a reviewer should be able
to read one small file and know what one function does, and find the
security-relevant code (scope checks, URL-to-path mapping, `robots.txt`, file
writes, network fetches) by directory name alone, then review each in
isolation. Group files into directories by concern or by the crate they help, and prefer
small files that each hold one function or type (for example
`url/from_str_into_start_url.rs`), with a `mod.rs` that re-exports them. Do not
put many unrelated items in one big file, and do not put source files directly
under `src/` beyond `main.rs` and `lib.rs`.

Source directories (one concern each; see also `src/lib.rs`):

- `app/`: `App` (validated settings), `Args` (raw arguments), `clap.rs`
  (command line), `run.rs` (program entry: logging, run, exit code),
  `config.rs`/`confy.rs` (config file, not loaded yet), `testing.rs` (test
  helpers), and converters for `--input-file`, sizes, and log files.
- `crawl/`: the crawl loop (`run.rs`), the polite pause, `--rate-limit`
  throttling, and the summary `Stats`.
- `disk/`: every file system write: atomic writes, the symbolic link check,
  file times, and the rejected log.
- `error/`: the crate's `Error`.
- `filter/`: `--accept`, `--reject`, the regexes, and the directory lists.
- `robots/`: `robots.txt` parsing and matching.
- `scope/`: `Scope`, what may be fetched at all.
- `scraper/`: HTML parsing and link discovery.
- `http/`: every network request: the client, redirect policy, fetches,
  and the body size limit.
- `types/`: semantic type aliases and macros (`List`, `Set`, `UrlString`, ...).
- `url/`: URL parsing, URL → file path mapping, safe names, redaction.

Dependency checks: `cargo audit` and `cargo deny check advisories` must pass
with no exceptions. If one is ever needed, record it with a written reason in
`.cargo/audit.toml` and `deny.toml`.
