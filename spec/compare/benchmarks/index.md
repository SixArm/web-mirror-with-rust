# Benchmarks

web-mirror against wget on a local site, so the network and the remote
server are out of the picture. Reproduce with `spec/compare/benchmarks/run.sh`
(needs `python3`, `hyperfine`, `wget`, and `cargo build --release`). It also
runs wget2, if installed.

## Setup

- Site: `server.py` (Python standard library, threaded): `/docs/` and 199
  more pages in a tree, 10 links per page, about 5 KiB each, every response
  delayed by 20 ms to stand in for network latency. No `robots.txt` (404).
- Machine: Apple M4 Max, 16 cores, macOS; November 2026.
- Tools: GNU Wget 1.25.0 (`wget -q -r -np -l inf`); web-mirror 0.1.0
  release build (`-v`, errors only). wget2 was not installed for this run.
- `hyperfine -N --runs 5`, output directory removed before each run. Every
  tool saved the same 200 files.

## Results

200 pages, 20 ms latency per response:

| Command | Mean | Min | Max | Pages per second | vs wget |
| :--- | ---: | ---: | ---: | ---: | ---: |
| `wget -r -np` | 4.95 s ± 0.09 | 4.82 s | 5.04 s | 40 | 1.0× |
| `web-mirror` (default, one at a time) | 5.17 s ± 0.20 | 4.98 s | 5.48 s | 39 | 0.96× |
| `web-mirror --max-connections 4 --per-host 4` | 1.33 s ± 0.07 | 1.25 s | 1.44 s | 150 | 3.7× |
| `web-mirror --max-connections 16 --per-host 16` | 0.40 s ± 0.03 | 0.38 s | 0.44 s | 500 | 12.3× |

Peak memory (maximum resident set size, `/usr/bin/time -l`):

| Command | Peak memory |
| :--- | ---: |
| `wget -r -np` | 7.8 MB |
| `web-mirror` | 13.5 MB |
| `web-mirror --max-connections 16 --per-host 16` | 14.4 MB |

## Reading the results

- **One at a time, web-mirror matches wget** (within 5%): both are bound by
  the 20 ms per request, as a polite sequential crawl should be.
- **`--max-connections 4 --per-host 4` is 3.9× faster** than the sequential
  default, close to the ideal 4×; 16 connections are 12.9× faster. The
  tree's shape (1, then 10, then 100 pages per level) limits how many URLs
  are known at once early in the crawl, so the ideal is not quite reached.
- **Memory stays small and flat**: about 6 MB more than wget, for the async
  runtime and TLS stack, and barely more with 16 connections, because each
  body is held only until it is saved.
- **On real sites, keep the default.** One request per host is what
  `robots.txt` etiquette and most site operators expect. Use
  `--max-connections` with several hosts (`-H -D`, `--input-file`), where
  each host still gets one request at a time, or with `--per-host` only on
  servers you run or have permission to load.

## Pitfall found while benchmarking

Python's `socketserver` accepts only 5 pending connections by default, so
at 16 connections the first runs stalled on refused connections (16
connections were slower than 4). `server.py` sets `request_queue_size = 256`.
