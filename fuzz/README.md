# Fuzz targets

[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets for the code
that reads untrusted input from websites, or turns it into file paths. Each
target checks a property, not just "no panic".

| Target | Code | Property |
| ------ | ---- | -------- |
| `path_mapping` | `url/from_mirror_layout_and_url_into_path_buf.rs` | for any URL and layout, the path stays under the root, every name passes `from_str_into_is_safe_segment`, and nothing maps into `.web-mirror/` |
| `safe_segment` | `url/from_str_into_safe_segment.rs` | any string becomes a safe name; a safe name is unchanged |
| `robots` | `robots/` | any robots.txt, user agent, and path parse and match |
| `link_discovery` | `crawl/from_bytes_and_url_into_url_list.rs`, `..._requisite_url_list.rs` | any HTML, CSS, or JavaScript bytes yield only http(s) links |
| `convert_links` | `convert/` (`-k`) | the HTML and CSS scanners never panic, and with nothing to rewrite every byte stays as it was |
| `sitemap` | `sitemap/` | any XML, date, or gzip parses; decompression never exceeds its limit |

## Run

Needs nightly Rust and `cargo install cargo-fuzz`. From this directory:

```sh
cargo +nightly fuzz list
cargo +nightly fuzz run path_mapping -- -max_total_time=3600 -timeout=10
```

A failing input is saved in `artifacts/<target>/`; replay it with
`cargo +nightly fuzz run <target> artifacts/<target>/<file>`, then add a
unit test for it beside the code it breaks. `corpus/` and `artifacts/` are not
committed.

CI (`.github/workflows/ci.yml`, job `fuzz`) builds every target and runs
each for 30 seconds on every push. Longer runs are manual: one hour per target
before a release.

This is a separate Cargo workspace, so it does not change the main crate's
build or lockfile.
