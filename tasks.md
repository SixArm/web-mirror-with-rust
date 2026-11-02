# Tasks

Work items for `plan.md`, in order. Each task: move its proposal from
`spec/proposals/index.md` into `spec/index.md`, implement, test, and update
README.md, `spec/compare/wget/index.md`, `AGENTS.md`, and `llms.json` where
they list options. "Offline test" means a `TestServer` test in
`src/crawl/run.rs` (see `AGENTS.md`).

## Continuous

- [ ] **CI workflow** `.github/workflows/ci.yml`: build, test, clippy (deny
  warnings), doc, `cargo audit`, `cargo deny check advisories`; matrix of
  ubuntu, macos, windows. Accept: green on all three; live tests not run.
- [ ] **Property tests for the path mapping** (`proptest` dev-dependency):
  for arbitrary URLs and layouts, the path starts with the root, has no
  `.`/`..`/empty segment, no segment over 200 bytes, no Windows device name.
  Accept: 10,000 cases per run pass.
- [ ] **Fuzz targets** (`fuzz/`, `cargo fuzz`): path mapping, safe segment,
  robots.txt parser, HTML link discovery. Accept: one hour each, no panic.
- [ ] **Re-check `spec/compare/index.md`** (next: November 2027).

## Phase 1: browsable offline copies

- [x] **CSS URL discovery.** New `scraper/from_css_str_and_url_into_url_list.rs`
  (`@import`, `url()`; quoted and unquoted; comments; escapes) and use it in
  `crawl/from_bytes_and_url_into_url_list.rs` for `text/css` and `.css`, and
  for `<style>` and `style=""` in HTML. Accept: unit tests for each syntax;
  offline test mirrors an image referenced only from CSS.
- [x] **`-E`, `--adjust-extension`.** Mapping takes the content type;
  `text/html` gets `.html`, `text/css` gets `.css`. Disk check finds either
  name. Accept: `page.php?x=1` served as HTML saves `page.php@x=1.html`; a
  re-run reads it from disk.
- [x] **Run record of URL → saved path** (in memory, plus the start of
  `.web-mirror/state.jsonl`). Accept: available to the post-crawl step.
- [x] **`-p`, `--page-requisites`.** Requisite discovery (`link[rel=stylesheet]`,
  `img`, `script`, `source`, `video`, `audio`, `track`, icons, CSS urls) beyond
  `--level` and `--link-selector`; same host allowed outside the start
  directory. Accept: offline test with `-l 1 --link-selector main -p` saves
  the pages' CSS and images, and does not follow links found in them.
- [x] **`-k`, `--convert-links`.** New `disk/` step after the crawl that
  rewrites attribute values in place (no re-serialization), atomically.
  Accept: offline test where every `href`/`src` in every saved page resolves
  to a saved file or is absolute; fragments kept; `<base>` removed; a re-run
  reads converted pages back correctly.
- [x] **`-H`, `--span-hosts` with `-D`, `--domains`.** Scope gains an allowed
  host list (whole-label suffix match); per-host robots.txt; redirect policy
  uses the same list; `-H` without `-D` is an error. Accept: offline tests
  with two `TestServer`s; `evilnhs.uk`-style lookalikes refused.
- [x] **Live test:** NHS conditions with `-k -p -l 1 --link-selector main`
  opens offline with styles. Done November 2026 as a manual check, with
  `-k -p -E -H -D nhs.uk --reject-regex 'v=[0-9a-f-]{36}'`: 3,233 files
  (238 MB), 0 failures; all 11,480 local links in 642 converted files
  resolve; in a browser with the network blocked, the pages render with the
  NHS stylesheet and Frutiger fonts, and links open local pages. Not an
  `#[ignore]` test with an `output.txt`: the image set (most of the files)
  changes too often for an exact file list to stay stable.

## Phase 2: discovery and machine-readable output

- [x] **`--events=FILE`** (JSON Lines, `-` for stdout) in a new
  `disk/events_log.rs`; `serde_json` dependency. Accept: one event per URL;
  `summary` last; credentials redacted; `jq` parses it.
- [x] **`--sitemap`.** Read `Sitemap:` from robots.txt and `/sitemap.xml`;
  `urlset` and `sitemapindex` (3 levels); `.xml.gz`; protocol limits;
  `<lastmod>` with `-N`. Accept: offline test finds a page linked from
  nowhere but the sitemap.
- [x] **URL normalization** and `--strip-query-params`, `--sort-query`,
  `--ignore-query`, in `url/`. Accept: `?utm_source=x` and no query fetch
  once. Real case found with `-p` on www.nhs.uk: every page includes a
  bot-detection script at `/dKjt/.../?v=<random uuid>`; with
  `--strip-query-params v`, about 30 copies became 2 (one per script path).
- [x] **`--accept-mime`, `--reject-mime`** (wildcards), decided before the body
  is read. Accept: a rejected `video/mp4` body is never downloaded (server
  sees the connection closed early).
- [x] **`--max-time=DURATION`.** Accept: offline test with a slow server stops
  on time.

## Phase 3: archival and refresh

- [x] **Crawl state** `.web-mirror/state.jsonl` with resume and `--fresh`.
  Accept: a run killed midway resumes without re-reading saved files.
  (`an_interrupted_crawl_resumes_from_its_queue`.)
- [x] **ETag refresh** (`If-None-Match`). Accept: offline test, 304 by ETag
  with no `Last-Modified`. (`timestamping_uses_the_etag`.)
- [x] **`--warc-file`** (WARC 1.1, gzip per record, CDX). Accept: output
  validated by an independent WARC tool (for example `warcio check`).
  (Records, digests, and CDX offsets checked by `warc_writer` tests and a
  standard-library reader; still to do: run `warcio check` on a real crawl.)
- [x] **`--load-cookies`, `--save-cookies`** (Netscape format). Accept:
  cookies only to in-scope hosts; never in logs.
  (`cookies_load_send_and_save`, `cookie_jar` tests.)

## Phase 4: speed, politely

- [ ] **`--max-connections`, `--per-host`.** Accept: with N=4 on a local
  server, 4× faster; per-host waits still honoured; same files as N=1.
- [ ] **Benchmarks** against wget and wget2 on a local server, written to
  `spec/compare/benchmarks.md`.

## Phase 5: conveniences

- [ ] **`--config=FILE`** (TOML via `confy`); command line overrides it.
- [ ] **`-nd`, `--no-directories`** with `-1`, `-2` clash suffixes.
- [ ] **`--check-external`** for `--spider`, with referrers in the report.
- [ ] **`--ignore-file=FILE`**, re-read every N seconds during a crawl.

## Done

- [x] wget-style filters, input and reporting, HTTP, and layout options.
- [x] Security hardening (see "Security" in `spec/index.md`).
- [x] Replace `spider` with `reqwest`.
- [x] Retry-After; robots.txt 429 handling.
- [x] Offline end-to-end tests; live tests take turns.
- [x] License: MIT OR Apache-2.0 OR GPL-2.0-only OR GPL-3.0-only OR BSD-3-Clause.
