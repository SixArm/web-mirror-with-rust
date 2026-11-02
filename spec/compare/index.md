# Compare: Unix command line tools for mirroring, fetching, spidering, crawling

Researched November 2026. This page surveys the tools people reach for, says
what each does best, and records what web-mirror should learn from it. The
resulting work is in `plan.md` and `tasks.md` at the repository root; the
proposed behavior is in `spec/proposals/index.md`. The option-by-option
mapping to wget is in `spec/compare/wget/index.md`.

## The landscape

| Tool | Language | Kind | Best at | Status (Nov 2026) |
| ---- | -------- | ---- | ------- | ----------------- |
| GNU Wget | C | mirror | The reference: `--mirror`, `-k` link conversion, `-p` page requisites, WARC, every option web-mirror models | Mature, maintained |
| GNU Wget2 | C | mirror | Wget's successor: HTTP/2, parallel downloads (`--max-threads`), sitemaps, HSTS, compression | 2.2.1, Jan 2026; fixed CVE-2025-69195 (file-name sanitization memory corruption with attacker-controlled URLs) and CVE-2025-69194 (Metalink path traversal) |
| HTTrack | C | offline browser | Browsable offline copies: rewrites links, fetches assets, updates mirrors in place | 3.50 in 2026, after nine years without a packaged release; adds HTTPS, files over 2 GB |
| curl | C | fetcher | One-shot transfers, every protocol, `--parallel`, URL globbing; no recursion | Mature, maintained |
| aria2 | C++ | fetcher | Segmented and parallel downloads of known URLs (HTTP, BitTorrent, Metalink); no crawling | Maintained |
| lftp `mirror` | C++ | mirror | Mirroring directory listings (FTP, HTTP index pages), parallel, reverse mirror | Maintained |
| rclone (`http` remote) | Go | sync | Copying HTTP directory listings like a file system, with checksums and resume | Maintained |
| wpull | Python | mirror | Wget-compatible crawler with WARC output, scriptable hooks | Used inside grab-site; Python version limits |
| grab-site | Python | archiver | Archivist's crawler: WARC output, crawl dashboard, ignore patterns changed live | Tied to old Python (3.7/3.8); dropped from nixpkgs after 23.05 |
| Heritrix | Java | archiver | Institutional web archiving at scale (Internet Archive) | Maintained; heavy |
| Browsertrix Crawler | Node (Docker) | archiver | High-fidelity archives through real browsers; WACZ output, sitemaps, screenshots | Maintained; needs Docker |
| ArchiveBox | Python | archiver | Archiving lists of URLs with many extractors (HTML, PDF, screenshot, WARC, media) | Maintained |
| monolith | Rust | page saver | One page as one self-contained HTML file, assets inlined | Maintained |
| SingleFile CLI | Node | page saver | One page as one file, through a browser | Maintained |
| katana | Go | spider | Fast URL discovery for security testing: headless mode, JS parsing, form filling, JSON output | Maintained (ProjectDiscovery) |
| gospider, hakrawler | Go | spider | Quick URL discovery, pipes well into other tools | Lightly maintained |
| gau, waybackurls | Go | URL lister | URLs from archives (Wayback Machine, Common Crawl), no crawling | Maintained |
| lychee | Rust | link checker | Fast, async link checking of files and sites; on-disk cache (`.lycheecache`, `--max-cache-age`); recursion long attempted | Maintained |
| muffet, LinkChecker, linkinator | Go, Python, Node | link checkers | Recursive broken-link reports with referrers | Maintained |
| Scrapy, Colly, spider (Rust) | Python, Go, Rust | frameworks | Programmable crawling and extraction | Maintained; libraries, not mirrors |

## Where web-mirror stands

Strengths today, compared with the field:

- **Safe defaults that no other mirror has together:** scope never above the
  start URL; never downloads twice; obeys `robots.txt` (RFC 9309) and
  `Crawl-delay`; sequential and polite; `Retry-After` honoured.
- **Hardening against hostile sites:** safe file names on every OS (the class
  of bug Wget2 fixed in CVE-2025-69195), no writes through symbolic links,
  no redirects off the start sites, bounded bodies and URLs, redacted
  credentials. See "Security" in `spec/index.md`.
- **Precise selection:** `--link-selector` (CSS) picks which part of a page to
  follow, something wget, wget2, and HTTrack cannot do; plus wget's accept,
  reject, regex, and directory filters.
- **A library as well as a tool**, with offline end-to-end tests.

Gaps, compared with the field (ranked in `plan.md`). Gaps 1–5 and 7 are
closed (Phases 1–3); each says how:

1. **Browsable offline copies.** wget `-k`/`-p`, Wget2, and HTTrack rewrite
   links and fetch page requisites (CSS, images, scripts, and the URLs inside
   CSS). Closed: `-k`, `-p`, `-E`, `-H`/`-D`.
2. **Complete discovery.** Wget2 and Browsertrix read sitemaps; web-mirror
   only followed links. Closed: `--sitemap`.
3. **Archival output.** wget, wpull, grab-site, Heritrix, and Browsertrix write
   WARC/WACZ, the standard archive formats. Closed: `--warc-file` (WARC 1.1
   and CDX; not WACZ).
4. **Machine-readable output.** katana and lychee emit JSON so other tools can
   consume results. Closed: `--events` (JSON Lines).
5. **Smarter refreshes.** `-N` uses `Last-Modified` only; `ETag`
   (`If-None-Match`) is more reliable on modern sites. Closed: `-N` sends
   both; interrupted crawls also resume from the crawl state.
6. **Speed on big sites.** Wget2 and aria2 fetch in parallel; web-mirror is
   deliberately sequential (polite), which is slow for large mirrors.
7. **Logged-in sites.** wget, curl, and HTTrack load cookies. Closed:
   `--load-cookies`, `--save-cookies`.

## Deliberate non-goals

- **Running JavaScript** (headless browsers, as katana, Browsertrix, and
  SingleFile do). It needs a browser, which brings a large attack surface and
  install burden. `--follow-js` covers the common case; for script-built
  sites, pair web-mirror with a browser-based tool, or feed it URLs with
  `--input-file`.
- **Segmented downloads of single files** (aria2, Wget2 `--chunk-size`).
  Mirrors are many small files.
- **Non-HTTP protocols** (FTP, BitTorrent, Metalink). Metalink in particular
  was the source of Wget2's CVE-2025-69194 path traversal.
- **Form filling and crawling as an attacker** (katana). That is security
  testing, not mirroring.

## Sources

- [GNU Wget2 2.2.1 release, oss-sec, January 2026](https://seclists.org/oss-sec/2026/q1/30)
- [LWN: Wget2 2.2.1 security updates](https://lwn.net/Articles/1053245)
- [HTTrack Website Copier](https://httrack.com/)
- [HTTrack updated after 9 years (Eleven Forum)](https://www.elevenforum.com/t/httrack-website-copier-updated-after-9-years.49392/)
- [katana (ProjectDiscovery)](https://github.com/projectdiscovery/katana)
- [grab-site (ArchiveTeam)](https://fr.github.com/ArchiveTeam/grab-site)
- [The WARC Ecosystem (ArchiveTeam wiki)](https://wiki.archiveteam.org/index.php/The_WARC_Ecosystem)
- [lychee on lib.rs](https://lib.rs/crates/lychee)
- [Five years of trying to add recursion to lychee (endler.dev)](https://endler.dev/json/)
- [monolith](https://gittrend.io/repo/Y2Z/monolith)
- [Browsertrix Crawler options](https://www.github.com/webrecorder/browsertrix-crawler/blob/2497189dcbbfdf301031bbf8510d4f057672a9b9/docs/docs/user-guide/common-options.md)
