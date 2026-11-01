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

- `-w`, `--wait=SECONDS`: wait SECONDS between retrievals (network only; reading
  from disk is not delayed)
- `--wait-jitter`: wait from 0.5*WAIT...1.5*WAIT seconds between retrievals
- `-P`, `--directory-prefix=DIR`: output directory (default `.`)
- `-l`, `--level=N`: maximum link depth (default 0 = unlimited)
- `-U`, `--user-agent=AGENT`
- `-t`, `--tries=N`: attempts per URL (default 3)
- `-T`, `--timeout=SECONDS` (default 30)
- `--force`: re-download files that already exist
- `-n, --number-limit=NUMBER`: process at most NUMBER URLs (0 is unlimited)
- `--rate-limit=RATE`: limit download speed to RATE bytes/second, with optional k/m/g suffix, like `wget --limit-rate` (0 is unlimited)
- `--link-selector=CSS`: follow only links inside elements matching the CSS selector, e.g. `main` to skip site-wide header, navigation, and footer links
- `--follow-js`: also follow URL-looking strings in JavaScript
- `--ignore-robots`: do not fetch or obey `robots.txt`
- `--log-file-create=FILE`: log messages to FILE instead of the console,
  replacing FILE if it exists (like `wget -o`)
- `--log-file-append=FILE`: log messages to FILE instead of the console,
  adding to FILE if it exists (like `wget -a`); the two log-file options
  conflict; file logs have timestamps

## Tech stack

Rust crates:

- `clap`: command line argument parsing
- `confy`: configuration management
- `scraper`: web page HTML parser
- `spider`: web crawler for recursive fetching (we use its re-exported `reqwest`
  HTTP client; see Design notes)
- `thiserror`: error handling with specifics
- `tokio`: asynchronous runtime
- `url`: link parser/validator
- `regex`, `fastrand`: JS URL heuristic and `--wait-jitter` randomness

## Design notes

- `spider`'s `Website::crawl` fetches every page itself and cannot be told "use
  the file on disk instead", which is a core requirement. So the crawl loop
  (queue, visited set, scope check, disk cache) is ours, and `spider` supplies
  the HTTP client.
- Fetching is sequential, as with `wget`, so `--wait` is meaningful.
- Non-goals for now: `--convert-links` link rewriting, cookies, parallel fetching.

## robots.txt

- On by default, as with `wget`. `--ignore-robots` turns it off.
- `/robots.txt` is fetched once, from the start URL's origin, before any page.
- Semantics follow RFC 9309: groups by `User-agent` (a group naming our product
  token, such as `web-mirror`, beats `*`); longest matching pattern wins and
  `Allow` wins a tie; `*` and a trailing `$` work in patterns; query strings
  are matched.
- `Crawl-delay` is honoured as a floor under `--wait`.
- A missing `robots.txt` (4xx) means no restrictions. A network error or 5xx
  after retries stops the run with an error (RFC 9309 says to assume disallow-all)
  unless `--ignore-robots` is given.
- Disallowed URLs are never queued, fetched, or read from disk, and are counted
  as "blocked" in the final summary.

## Tests

Unit tests live beside the code they test. `cargo test` runs them. Library
doctests are off (`doctest = false`) because `src/types/` has illustrative,
non-runnable examples.

## Project file guidance

TODO add more.

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

Application files:

- app/args.rs : arguments
- app/confy.rs : rust crate "confy" helpers
- app/config.rs : configuration
- app/clap.rs : command line argument parsing
- app/run.rs : run the application (the main function calls this)
- app/testing.rs : testing helpers

Directories that contain helpers for specific crates:

- scraper/
- spider/
- url/
