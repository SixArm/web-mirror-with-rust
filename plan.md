# Plan

Where web-mirror is going, and why. Updated November 2026.

- Background research: `spec/compare/index.md` (the tool landscape) and
  `spec/compare/wget/index.md` (option by option).
- Proposed behavior: `spec/proposals/index.md`.
- Work items with acceptance criteria: `tasks.md`.

## Assessment

web-mirror already beats the field on **safety** (scope, robots.txt,
hardening against hostile sites) and **precision** (`--link-selector`, wget's
filters). It is **behind on output**: the mirror is a pile of files whose
links still point at the live site, with no archive format and no
machine-readable record. People choose wget `-k -p` or HTTrack because the
result opens in a browser; they choose grab-site or Browsertrix for WARC; they
choose katana or lychee for JSON. Closing those gaps, without giving up the
safe defaults, is the plan.

Principles for every step:

1. **Spec first.** Move the proposal into `spec/index.md`, then code, then
   tests (offline `TestServer` tests first; live tests only when needed).
2. **Safe by default.** New capabilities that widen the crawl (other hosts,
   parallelism) are off by default and bounded when on.
3. **Every write in `disk/`, every request in `http/`.** No exceptions for
   new features such as link conversion or WARC.
4. **No browser.** JavaScript execution stays a non-goal; see
   `spec/compare/index.md`.

## Phases

### Phase 1: browsable offline copies (done, November 2026)

Goal: `web-mirror URL -k -p` produces a copy you can open from disk and
browse, as with `wget -k -p` or HTTrack.

1. CSS URL discovery (`@import`, `url()`): small, standalone, and needed by
   both `-k` and `-p`.
2. `-E`, `--adjust-extension`: so saved pages open as HTML.
3. Crawl state, minimal: the URL → saved path record for the run (needed by
   `-k`). The full resumable state comes in Phase 3.
4. `-p`, `--page-requisites`.
5. `-k`, `--convert-links`.
6. `-H`/`-D` (`--span-hosts`, `--domains`): for requisites on CDNs.

Done when: an offline test mirrors a `TestServer` site with CSS, images, and
nested pages, and every link in every saved page resolves to a saved file
or an absolute URL; and the NHS conditions page, mirrored with `-k -p -l 1
--link-selector main`, renders offline with styles.

### Phase 2: discovery and machine-readable output (done, November 2026)

1. `--events=FILE` (JSON Lines). Cheap, and makes every later feature easier
   to test and to use from scripts and AI agents.
2. `--sitemap`.
3. URL normalization, `--strip-query-params`, `--sort-query`, `--ignore-query`.
4. `--accept-mime`, `--reject-mime`, `--max-time`.

Done when: a sitemap-only page is found and saved in an offline test; the
events file round-trips through `jq`; `utm_*` duplicates are fetched once.

### Phase 3: archival and refresh (done, November 2026)

1. Full crawl state (`.web-mirror/state.jsonl`): resume from the queue.
2. ETag refresh (`If-None-Match`) for `--timestamping`.
3. `--warc-file` (WARC 1.1 + CDX), validated with an independent WARC reader.
4. `--load-cookies`, `--save-cookies`.

Done when: a stopped crawl resumes without fetching or reading what it
already handled; `-N` gets a 304 by `ETag` alone; the WARC passes an
independent reader; cookies reach only in-scope hosts.

### Phase 4: speed, politely (done, November 2026)

1. `--max-connections` with per-host limits and per-host waits.
2. Benchmarks against wget and wget2 on a local server (pages per second,
   memory), recorded in `spec/compare/benchmarks/`.

Result: one at a time, web-mirror matches wget; 4 connections are 3.9×
faster and 16 are 12.9× faster on a local site with 20 ms latency, in
14 MB of memory.

### Phase 5: conveniences (done, November 2026)

`--config`, `--no-directories`, `--check-external`, `--ignore-file`.

## Continuous work (every phase)

- **Fuzz the path mapping and the parsers.** Wget2's CVE-2025-69195 was
  memory corruption in file-name sanitization driven by attacker URLs. Rust
  rules out the memory corruption, but a logic bug could still escape the
  output directory. Property tests (`proptest`, 10,000 cases) for
  `from_mirror_layout_and_url_into_path_buf` and `from_str_into_safe_segment`
  are done; they found two safe-name gaps (trailing dots from `.`/`..`, and
  truncation after a device-name prefix), now fixed. Still to do: `cargo fuzz`
  targets for those two, the robots.txt parser, and link discovery: every
  output must stay inside the root, contain no `.`/`..`/empty segment, and
  never panic.
- **CI.** Done: `.github/workflows/ci.yml` runs build, `cargo test`, clippy,
  `cargo doc`, an MSRV check, `cargo audit`, and `cargo deny check
  advisories` on Linux, macOS, and Windows (Windows matters for the safe-name
  rules). Live-site tests stay manual.
- **Keep the comparison current.** Re-check `spec/compare/index.md` yearly.

## Risks

- **`-k` on sites with odd HTML.** Mitigation: rewrite attribute values in
  place, never re-serialize the document; test with malformed HTML.
- **`-p` widening scope.** Mitigation: requisites never recurse, other hosts
  need `--domains`, and the redirect policy follows the same list.
- **Parallelism and politeness.** Mitigation: default stays 1; per-host
  limits; `Crawl-delay` and `Retry-After` per host.
- **Crawl state corruption.** Mitigation: append-only JSON Lines, written
  through `disk/`, tolerant of a truncated last line.
