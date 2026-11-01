# Web Mirror

Web Mirror is a command line tool, and a Rust library, to crawl a web URL and download pages. This is similar to `wget --mirror`.

```sh
web-mirror "https://www.nhs.uk/conditions/"
```

This creates `www.nhs.uk/conditions/...` in the current directory.

- Never goes above the start URL.
- Never downloads a file twice: existing files are read from disk to find their links.
- Follows URLs in HTML; does not follow URLs in JavaScript (unless `--follow-js`).
- Obeys `robots.txt` (including `Crawl-delay`); `--ignore-robots` to skip it.
- `--wait=SECONDS` and `--wait-jitter` for politeness.

Run `web-mirror --help` for all app. See `spec/index.md` for the specification.

As a library:

```rust
let app = web_mirror::App::new(web_mirror::parse_start_url("https://www.nhs.uk/conditions/")?);
let stats = web_mirror::run(&app).await?;
```
