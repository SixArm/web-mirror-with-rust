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
- **Follow URLs found in CSS:** `@import` and `url(...)` in stylesheets
  (`text/css`, or `.css` on disk), in `<style>` elements, and in `style`
  attributes, resolved against the stylesheet's URL. Parsing is a tolerant
  tokenizer: comments, quoted and unquoted URLs, and CSS escapes are handled;
  malformed CSS never stops the crawl.
- **Do not follow URLs found in JavaScript.** Option `--follow-js` opts in.
- Fragments are dropped; non-http(s) schemes (`mailto:`, `javascript:`) are ignored.
- Files are written atomically (`.part` then rename), so an interrupted run never
  leaves a truncated file that a later run would mistake for complete.
- **Resume an interrupted crawl.** A crawl that stops before its queue runs
  out (killed, or stopped by `-n`, `-Q`, or `--max-time`) carries on where it
  stopped the next time the same start URLs are mirrored into the same
  directory: see "Crawl state". `--fresh` starts over.

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
- `--test`: print the parsed arguments and log level to standard output, then
  run as usual, for debugging
- `-h`, `--help` and `-V`, `--version`: print help or the version and exit
- `-w`, `--wait=SECONDS`: wait SECONDS between retrievals (network only; reading
  from disk is not delayed)
- `--wait-jitter`: wait from 0.5*WAIT...1.5*WAIT seconds between retrievals
- `-P`, `--directory-prefix=DIR`: output directory (default `.`)
- `-l`, `--level=N`: maximum link depth (default 0 = unlimited)
- `-U`, `--user-agent=AGENT`
- `--max-connections=N`: fetch up to N URLs at once (1 to 64; default 1, one
  at a time, as with wget). Each host still gets one request at a time unless
  `--per-host` says more, and `--wait` (with `--wait-jitter`) and each host's
  `Crawl-delay` apply per host: a host rests that long after each of its
  requests, and its request starts are spaced that far apart. With 1, the
  wait is between any two requests, and the longest `Crawl-delay` of any site
  applies to all, as before. Parallelism changes only the order in which
  pages are handled: the same URLs are fetched, each once, and the same files
  are saved. Each request gets an equal share of `--rate-limit`. `-n` counts
  URLs as they are taken from the queue; with `-Q` and `--max-time`, requests
  in progress are finished. See `spec/compare/benchmarks/index.md`.
- `--per-host=N`: with `--max-connections`, allow up to N requests at once to
  the same host (1 to `--max-connections`; default 1). For servers you run or
  may load; one at a time is what most site operators expect.
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
- `-p`, `--page-requisites`: also fetch what saved pages need to display, like
  `wget -p`: stylesheets, icons, images (including `srcset`), scripts, media,
  embedded objects, and the URLs in stylesheets and inline CSS. Requisites are
  found in the whole page (not limited by `--link-selector`), are fetched even
  at the last `--level`, and may be anywhere on the same site (same scheme
  family, host, and port as a start URL), not only below the start URL. Their
  own links are not followed, except a stylesheet's. Filters and `robots.txt`
  still apply. Requisites on other hosts (such as a CDN) need `-H` and `-D`.
- `-H`, `--span-hosts` with `-D`, `--domains=LIST`: also fetch from other hosts,
  but only hosts on the listed domains, like `wget -H -D`: page requisites
  (with `-p`) and linked pages (bounded by `--level`), at any path on those
  hosts. Domains match on whole labels (`nhs.uk` matches `assets.nhs.uk`, not
  `evilnhs.uk`); entries must be plain domain names (IDNs in `xn--` form).
  The start URLs' own hosts never widen: "never above the start URL" still
  holds there. Each new host gets its own `robots.txt`, fetched the first time
  a link reaches it; if it cannot be read, that host is skipped and the crawl
  goes on. Its `Crawl-delay` raises the wait for the rest of the crawl.
  Redirects may also go to hosts on the listed domains. `-H` without `-D` is
  an error, so a crawl can never wander the web.
- `-k`, `--convert-links`: after the crawl, rewrite links in the HTML pages and
  stylesheets saved or found in this run, so the mirror can be browsed
  offline, like `wget -k`. A link to a saved file becomes a relative path
  (`../acne/index.html`, keeping any `#fragment`); a link to anything else
  becomes absolute, so it still works online; fragment-only and non-http(s)
  links stay. Rewritten: `href`, `src`, `poster`, `data`, `background`,
  `srcset`, `style` attributes, `<style>` elements, and `url(...)`/`@import`
  in stylesheets. Only those values change; every other byte of a file stays
  as it was. `<base>` is removed. A `%` in a saved name is written `%25` in the
  link, because browsers decode links. Files that are not UTF-8 are left
  alone. Converted files keep their modification time (for `-N`).
  - The record `DIR/.web-mirror/saved.tsv` (`PATH<TAB>URL` per line) says which
    file holds which URL. When a later run reads a converted page from disk,
    local links are turned back into the original URLs, so the crawl (and
    `-N`) asks the server for the real URLs. The record is read defensively:
    lines with unsafe paths or non-http(s) URLs are ignored.
- `--sitemap`: also crawl the URLs that the sites' sitemaps list, as if the
  start page linked to them (depth 1, same scope, filters, and `robots.txt`
  checks as links). Sitemaps are the `Sitemap:` lines in each start site's
  `robots.txt`, plus `/sitemap.xml`. Both `<urlset>` and `<sitemapindex>` are
  read, indexes at most three levels deep; `.xml.gz` files are un-gzipped.
  Limits (the sitemaps protocol's): 50,000 entries and 50 MiB per file,
  compressed or not, and at most 1,000,000 URLs in total. A sitemap file
  must be on a start site (or a `-D` domain with `-H`) and allowed by
  `robots.txt`. The XML is read by a small scanner that never expands DTDs or
  external entities. With `-N`, a page whose `<lastmod>` is no newer than its
  local file is not requested at all. Example: www.nhs.uk's sitemap index
  lists 8 sitemaps and 56,554 URLs.
- `--link-selector=CSS`: follow only links inside elements matching the CSS selector, e.g. `main` to skip site-wide header, navigation, and footer links
- `--follow-js`: also follow URL-looking strings in JavaScript

Input and reporting:

- `--config=FILE`: read default options from FILE, a TOML file whose keys are
  long option names (`max-connections` or `max_connections`) and whose values
  are what the option takes:

  ```toml
  version = 1          # optional; the only version is 1
  wait = 2
  wait-jitter = true   # a flag: true sets it, false leaves it unset
  verbose = 3          # a count: -vvv
  reject = ["*.mp4", "*.zip"]
  header = ["Accept-Language: en-GB"]
  ```

  Options on the command line override the file: a single-valued option
  replaces the file's value, and a repeatable one (`--header`, `--accept`,
  ...) adds to the file's values. A flag set in the file cannot be unset on
  the command line, and options that conflict (such as `force` in the file
  and `-N` on the command line) are an error. The file must exist (it is
  never created) and be at most 1 MiB. Unknown keys, the wrong kind of value,
  nested tables, and `config`, `help`, `version`, and `test` are errors, so
  a typo is never silently ignored. The start URL cannot be in the file (use
  `input-file`). There is no default config file.
- `-i`, `--input-file=FILE`: also read start URLs from FILE, one per line (`-`
  is standard input; blank lines and `#` comments are skipped). The URL
  argument becomes optional. The scope is the union of every start URL's scope,
  and `robots.txt` is fetched once per origin.
- `--spider`: crawl and check links, but never save files (and never read
  them from disk). Failures are logged and give exit code 2, so this works as
  a link checker.
- `--check-external` (needs `--spider`): also check the links that leave the
  mirror's scope (not page requisites), each once, after the crawl, without
  crawling them, like lychee and muffet. Each is requested with `HEAD`, then,
  if that does not succeed, `GET` (whose body is not read). A link is broken
  on a 4xx or 5xx status, an unfollowed redirect, or no response; 429 Too
  Many Requests counts as not checked, not broken. Broken links are logged
  with the pages that link to them (`broken URL (HTTP 404), linked from A, B
  and 3 more`), count as failures (exit code 2), and are `broken` events with
  `referrers` (up to 100). Links on one host are checked one at a time with
  `--wait` between them; up to `--max-connections` hosts at once. At most
  100,000 external links. External links are not written to the rejected
  log. Safety:
  - They are checked with a separate HTTP client that sends only
    `User-Agent`: never `--header` values, `--referer`, or cookies.
  - Only public addresses are requested: links to IP addresses that are
    loopback, private, link-local (including cloud metadata), carrier-grade
    NAT, or reserved, and to `localhost` names, are skipped; host names are
    resolved by a resolver that drops non-public addresses, so a name pointed
    at the local network fails, and redirects must pass the same check
    (`http/from_app_into_external_client.rs`, `http/public_only_resolver.rs`).
    No proxy is used for these checks, so the address check is the real one.
- `--rejected-log=FILE`: write each URL that was found but not fetched to FILE,
  tab-separated: `REASON`, `URL`, `PARENT_URL`. Reasons: `SCOPE`, `ROBOTS`,
  `REGEX`, `SUFFIX`, `DIRECTORY`, `MIME`, `IGNORE`. Each URL is logged at most
  once.

- `--events=FILE`: write one JSON object per line (JSON Lines) for every URL
  the crawl handles, then a summary, for scripts, dashboards, and AI agents;
  `-` writes to standard output (the human log goes to standard error).
  Example:

  ```json
  {"time":"2026-11-01T17:45:05Z","event":"saved","url":"https://www.nhs.uk/conditions/acne/","parent":"https://www.nhs.uk/conditions/","depth":1,"status":200,"bytes":51234,"path":"www.nhs.uk/conditions/acne/index.html"}
  ```

  `event` is `saved`, `fetched` (fetched for its links, not saved: `--accept`
  or `--reject`), `checked` (`--spider`), `exists` (read from disk),
  `unchanged` (304, `-N`), `failed` (with `error`, and `status` for an HTTP
  error), `rejected` (with `reason`, as in `--rejected-log`), `blocked`
  (`robots.txt`), `external` (a working, or not checked, external link) and
  `broken` (with `referrers`), for `--check-external`, or `summary` (last
  line, with `stats`). Other fields:
  `time` (RFC 3339, UTC), `url`, `parent` (the linking page; absent for start
  URLs), `depth`, `requisite` (true for `-p` requisites), `status`, `bytes`,
  `path` (relative to the output directory). Fields that do not apply are
  left out. Credentials are redacted.

URL normalization (applies to start URLs and every discovered link, before
URLs are compared, so one page is not fetched twice under two spellings):

- Always: an empty query (`page?`) and empty query pieces (`a=1&&b=2`) are
  removed. The `url` crate already lower-cases the scheme and host, removes
  the default port, and resolves `.` and `..`.
- `--strip-query-params=LIST`: remove these query parameters (matched on the
  percent-decoded name; `*` and `?` wildcards), such as `utm_*,fbclid,gclid`.
  Comma-separated; may be repeated.
- `--sort-query`: sort query parameters by name, so `?b=2&a=1` equals `?a=1&b=2`.
- `--ignore-query`: drop query strings entirely.
- Kept parameters are byte for byte as the page wrote them (never decoded and
  re-encoded).
- Example: every page on www.nhs.uk includes a bot-detection script with a
  random `?v=`, so `-p` fetches one copy per page; `--strip-query-params v`
  fetches one.

URL filtering (applies to discovered links; start URLs are always fetched):

- `-A`, `--accept=LIST`: save only files whose names end with a suffix in LIST,
  or match a wildcard pattern in LIST (`*`, `?`). Comma-separated; may be
  repeated. As with wget, pages (names ending `.html`/`.htm`, or with no
  extension) that are not accepted are still fetched to follow their links,
  but are not saved. Matching is case-sensitive, as with wget.
- `-R`, `--reject=LIST`: the opposite of `--accept`.
- `--accept-regex=REGEX`, `--reject-regex=REGEX`: match against the whole URL;
  a URL that fails is not fetched at all.
- `--ignore-file=FILE`: also reject URLs that match any regular expression in
  FILE (one per line, matched anywhere in the URL like `--reject-regex`;
  blank lines and `#` comments skipped; at most 1 MiB). FILE is checked every
  5 seconds during the crawl and read again when its modification time
  changes, and its patterns also drop URLs already queued, so a crawler trap
  (an endless calendar, search pages) can be cut off without restarting, as
  with grab-site's ignores. FILE must be readable at the start; a line that is
  not a valid pattern is skipped with a warning; if FILE cannot be read later,
  the patterns in use are kept. Start URLs are always fetched. Rejections are
  logged as `IGNORE`.
- `--accept-mime=LIST`, `--reject-mime=LIST`: download only responses whose
  `Content-Type` matches (or does not match) LIST, such as `text/*` or
  `video/*,audio/*`: type and subtype, ignoring case and parameters, with `*`
  and `?` wildcards; no `Content-Type` counts as `application/octet-stream`.
  Decided when the response headers arrive, before any of the body is read,
  so an unwanted video is never downloaded. As with `--accept`, HTML pages
  are still read for their links, just not saved. Rejections are counted,
  logged as `MIME` in `--rejected-log`, and reported as `rejected` events.
- `-I`, `--include-directories=LIST`, `-X`, `--exclude-directories=LIST`:
  directories such as `/conditions/acne`, matched on whole path segments
  (`/a` matches `/a` and `/a/b`, not `/ab`); wildcards allowed.

Output layout:

- `--no-host-directories`: do not create the host directory (wget's `-nH`;
  there is no `-nH` short form because `-n` is `--number-limit`).
- `--cut-dirs=N`: drop the first N directories of each URL path.
- `-E`, `--adjust-extension`: save a document served as HTML (`text/html`,
  `application/xhtml+xml`) with `.html` added when its name does not end in
  `.html` or `.htm`, and CSS (`text/css`) with `.css` added, so a browser
  opens the offline copy correctly, like `wget -E`. The query stays before
  the extension: `page.php?x=1` → `page@x=1.php.html`. The "never download
  twice" check finds a file under either name.
- `--no-directories`: save every file directly in the output directory, with
  no host or path directories (wget's `-nd`; there is no `-nd` short form,
  because `-n` is `--number-limit`). When two URLs map to the same name, the
  later one gets a number before the extension: `index.html`, `index-1.html`,
  `index-2.html`. No file is ever overwritten: a name already used by another
  URL, or held by a file web-mirror did not save, is skipped. The record
  `DIR/.web-mirror/saved.tsv` is kept (as with `-k`), so each URL gets the
  same name in later runs and "never download twice" still works. With `-E`,
  the adjusted name is checked for clashes too. `--no-host-directories` and
  `--cut-dirs` have no effect.
- `--default-page=NAME`: save directory URLs as NAME instead of `index.html`.
  NAME must be a plain file name.
- `--max-time=DURATION`: stop starting new requests after DURATION: seconds
  (`90`), or with units `s`, `m`, `h`, `d`, combined (`1h30m`); 0 is
  unlimited. The request in progress is finished.
- `-Q`, `--quota=SIZE`: stop starting new downloads once SIZE bytes have been
  downloaded in total (k/m/g suffix; 0 is unlimited). As with wget, the file
  that crosses the quota is finished.

HTTP:

- `-N`, `--timestamping`: for files that already exist, send `If-Modified-Since`
  with the file's time, and `If-None-Match` with the `ETag` the server gave
  last time (kept in the crawl state), and download only if the server's copy
  has changed. Conflicts with `--force`. Saved files always get the server's
  `Last-Modified` time, so this works on files from earlier runs; the `ETag`
  makes it work on servers that send no `Last-Modified`.
- `--fresh`: ignore the crawl state: do not resume an interrupted crawl, and
  forget remembered `ETag`s.
- `--load-cookies=FILE`: load cookies from FILE, in the Netscape `cookies.txt`
  format that browsers (via extensions), wget, and curl write: seven
  tab-separated fields per line, `#HttpOnly_` prefixes understood, `#`
  comments skipped, expired cookies dropped; at most 10,000 cookies and
  16 MiB. Cookie rules (domain, path, `Secure`, expiry) are RFC 6265.
- `--save-cookies=FILE`: after the crawl, save the cookies (loaded, plus any
  the servers set) to FILE in the same format, readable only by you (mode
  0600). Session cookies (no expiry) are left out, as with wget, unless
  `--keep-session-cookies`.
- With either cookie option, cookies the servers set during the crawl are
  kept and sent back for the rest of it, redirects included. Without them, no
  cookies are sent or kept. Cookies go only to hosts in the mirror's scope
  (the start URLs' sites, and `-D` hosts with `-H`), even if the file holds
  cookies for other sites, and are never logged.
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

Archiving:

- `--warc-file=NAME`: also write every page fetched (pages and requisites)
  to `NAME.warc.gz` in WARC 1.1, the Internet Archive's format, also written by
  wget, wpull, and grab-site, plus an index `NAME.cdx`. Both are replaced if
  they exist, and are written as the crawl goes (a killed run leaves every
  record but the last readable).
  - The archive starts with a `warcinfo` record (software, format, user agent,
    robots policy), then for each fetch a `response` record and a `request`
    record (`WARC-Concurrent-To` links them). Each record is its own gzip
    member, so a reader can seek to any record. Records carry
    `WARC-Block-Digest` and, for responses, `WARC-Payload-Digest` (SHA-1, base
    32).
  - The HTTP client decodes compressed bodies and does not reveal the exact
    bytes on the wire, so the records are rebuilt: the response is its status
    line, its headers as received (less `Content-Encoding` and
    `Transfer-Encoding`), the real `Content-Length`, and the decoded body;
    the request is the request line, `Host`, and the headers web-mirror asked
    for, with `Authorization`, `Proxy-Authorization`, `Cookie`, and
    `X-Api-Key` values written as `[redacted]`.
  - Archived: successful (2xx) responses for pages and requisites. Not
    archived: `robots.txt`, sitemaps, failed requests, 304s, and the hops of
    a redirect (the response is archived under its final URL).
  - The index is the 11-field CDX format (` CDX N b a m s k r M S V g`): SURT
    key, 14-digit time, URL, media type, status, payload digest, `-`, `-`,
    compressed record length, offset, and the archive's file name.

## Crawl state

web-mirror keeps `DIR/.web-mirror/state.jsonl` (not with `--spider`, which
writes nothing): one JSON object per line, appended as the crawl goes.

- `{"t":"run","starts":[...]}`: the first line, naming the start URLs.
- `{"t":"queued","url":...,"depth":1,"requisite":true,"parent":...}`: a URL
  that passed the scope, filter, and `robots.txt` checks and joined the queue.
- `{"t":"done","url":...,"path":...,"final_url":...,"etag":...}`: a queued URL
  that was handled: saved, read from disk, unchanged (304), fetched for links,
  or rejected by its media type. Failed URLs are not "done", so a resumed run
  tries them again.
- `{"t":"etag","url":...,"etag":...}`: an `ETag` kept from a finished crawl.
- `{"t":"finished"}`: the queue ran out.

At the start of each run the file is compacted (rewritten through `.part` and
a rename):

- If the last crawl of the same start URLs did not finish, the run resumes:
  URLs already done are neither fetched nor read from disk again (files they
  saved still count for `-k`), and the URLs still queued are checked again
  (scope, filters, and `robots.txt`, as given this time) and queued first.
  Sitemaps are not read again. A resumed crawl that runs out of queue is
  finished.
- Otherwise the crawl starts from the start URLs, keeping only the `ETag`s.
- `--fresh` starts over and forgets the `ETag`s.

The file is read defensively: a truncated or garbled line (such as the last
line of a killed run) is skipped; only http(s) URLs, plain relative paths, and
`ETag`s of at most 1024 bytes of visible ASCII count; a file over 512 MiB is
ignored. URLs are written with credentials redacted.

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
- `scraper`: web page HTML parser
- `reqwest`: HTTP client (rustls TLS with the system's certificate roots,
  HTTP/2, gzip/brotli/deflate/zstd decoding, system proxy settings)
- `thiserror`: error handling with specifics
- `tokio`: asynchronous runtime
- `url`: link parser/validator
- `regex`, `fastrand`: JS URL heuristic, `--accept-regex`/`--reject-regex`, and `--wait-jitter` randomness
- `httpdate`: HTTP dates for `--timestamping`
- `serde_json`: `--events` and the crawl state (JSON Lines)
- `flate2`: gzipped sitemaps and the `--warc-file` archive
- `cookie_store`, `cookie`: cookie rules (RFC 6265) for `--load-cookies`
- `confy`: reads the `--config` TOML file
- `sha1`: WARC digests

## Design notes

- The crawl loop (queue, visited set, scope check, disk cache) is ours, on top
  of the `reqwest` HTTP client. An earlier version used the `spider` crawler
  crate, but its `Website::crawl` fetches every page itself and cannot be told
  "use the file on disk instead", which is a core requirement, so only its
  re-exported `reqwest` client was used; depending on `reqwest` directly
  dropped about 200 unused crates (and the advisories that came with them).
- Fetching is sequential by default, as with `wget`, so `--wait` is
  meaningful. With `--max-connections`, requests run as tasks on the async
  runtime, but everything else (saving, the crawl state, events, link
  discovery, the queue) stays in the one crawl loop, so nothing on disk is
  ever written by two tasks at once.
- Every phase in `plan.md` is built. Continuous work (fuzzing, CI) is in
  `plan.md` and `tasks.md`. Running JavaScript stays a non-goal (see
  `spec/compare/index.md`).

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

- **web-mirror's own files never clash with mirrored ones.** They live in
  `DIR/.web-mirror/`; with host directories no URL can map there, and with
  `--no-host-directories` a site's own `/.web-mirror/` is saved as
  `_.web-mirror/`.
- **Files are only written inside the output directory.** Every URL path
  segment is made safe (`url/from_str_into_safe_segment.rs`): path separators,
  control and Windows-reserved characters become `_`; `.`, `..`, and the empty
  name can never appear (`.` becomes `_`, `..` becomes `._`); trailing dots
  and spaces become `_`; Windows device names (`CON`, `nul.txt`, `CON .txt`,
  `COM0`–`COM9`, `COM¹`–`COM³`, the same for `LPT`, `CONIN$`, `CONOUT$`) get
  a leading `_`; and no name is longer than 200 bytes. `--default-page` must
  be a plain file name. Property tests check these rules for 10,000
  arbitrary URLs and layouts on every `cargo test`.
- **Paths in web-mirror's own files use `/`.** The events log, the crawl
  state, and the `-k` record write paths relative to the output directory
  with `/` between names on every platform, so they read the same on
  Windows.
- **Never writes through a symbolic link.** Before a write, every existing part
  of the path below the output directory is checked, and a symbolic link is
  refused (`disk/ensure_no_symlink_below_root.rs`). The `.part` file is created
  fresh with `create_new`, so it can never be a pre-placed link
  (`disk/write_atomically.rs`).
- **Never requests another site** (unless `-H -D` lists it). A redirect is
  followed only if it stays on the origin of a start URL (or goes to a host on
  a `-D` domain, with `-H`), so a page cannot steer requests to other hosts,
  local network machines, or cloud metadata addresses
  (`http/from_app_into_redirect_policy.rs`). A redirect's final URL must
  also be in scope (`http/fetch.rs`).
- **Link checking never reaches the local network.** `--check-external`,
  the one feature that requests URLs outside the scope, connects only to
  public addresses (checked on the address actually connected to, so DNS
  rebinding cannot get past it), and sends no secrets (see `--check-external`).
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
- **Cookies are secrets.** They are sent to, and accepted from, only hosts in
  scope (`cookies/cookie_jar.rs`), never logged (the cookie crate's own debug
  logging is turned off), and saved with mode 0600 through a fresh `.part`
  file (`disk/write_private_file_atomically.rs`). WARC request records redact
  secret header values.
- **web-mirror's own state is untrusted on read.** The crawl state and the
  `-k` record are re-checked line by line, and every URL read back passes the
  scope, filter, and `robots.txt` checks before it is fetched.
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
`--timestamping` (by date and by `ETag`), resuming an interrupted crawl,
cookies, `--warc-file`, refused off-site redirects, `--max-file-size`,
`Retry-After`, and refusing to write through a symbolic link.

End-to-end tests also cover `--no-directories` (numbered clashes, stable
across runs), `--ignore-file` (a pattern added mid-crawl drops queued URLs),
and `--check-external` (broken links with referrers, HEAD then GET, private
addresses never requested; a second test server stands in for the web).
`--config` is tested through the argument parser.

End-to-end tests also cover `--max-connections` and `--per-host`: peak
concurrency (measured by the test server), per-host limits across two hosts,
per-host waits, `-n` and resume under parallelism, and the same files as one
at a time. Benchmarks against wget: `spec/compare/benchmarks/index.md`.

Property tests (`proptest`, 10,000 cases each) check that
`from_mirror_layout_and_url_into_path_buf` keeps every path under the output
directory with only safe names, for arbitrary URLs (dot segments, escaped
dots and slashes, device names, long and non-ASCII names) and layouts, and
that `from_str_into_safe_segment` makes any string a safe name.

Continuous integration (`.github/workflows/ci.yml`) runs the build, the
offline tests, clippy (warnings denied), `cargo doc` (warnings denied),
`cargo audit`, and `cargo deny check advisories` on Linux, macOS, and
Windows, and checks the build with the `rust-version` in `Cargo.toml`.
Live-site tests are not run in CI.

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
  `config.rs`/`confy.rs` and converters for `--config`, `testing.rs` (test
  helpers, `TestServer`), and converters for `--input-file`, sizes,
  durations, and log files.
- `convert/`: `-k` text rewriting of HTML and CSS (pure, no I/O).
- `cookies/`: the cookie jar (only hosts in scope; Netscape format).
- `crawl/`: the crawl loop (`run.rs`), sitemaps, `-k` conversion, external
  link checks, the polite pause, `--rate-limit` throttling, and `Stats`.
- `disk/`: every file system write: atomic and private writes, the symbolic
  link check, file times, the crawl state, the `-k` record, the events,
  rejected, and WARC files.
- `error/`: the crate's `Error`.
- `filter/`: `--accept`, `--reject`, the regexes, the directory lists, the
  MIME filter, and `--ignore-file`.
- `http/`: every network request: the clients, redirect policies, fetches,
  `robots.txt`, external link checks, the public-only resolver, and the body
  size limit.
- `robots/`: `robots.txt` parsing and matching.
- `scope/`: `Scope`, what may be fetched at all.
- `scraper/`: HTML and CSS parsing and link discovery.
- `sitemap/`: sitemap XML parsing and decompression (pure).
- `time/`: timestamps.
- `types/`: semantic type aliases and macros (`List`, `Set`, `UrlString`, ...).
- `url/`: URL parsing, URL → file path mapping, safe names, normalization,
  redaction.
- `warc/`: WARC records and CDX lines (pure).

Dependency checks: `cargo audit` and `cargo deny check advisories` must pass
with no exceptions. If one is ever needed, record it with a written reason in
`.cargo/audit.toml` and `deny.toml`.
