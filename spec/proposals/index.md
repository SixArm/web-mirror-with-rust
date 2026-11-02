# Proposals

Proposed behavior for planned features. Nothing here is implemented yet.
When a proposal is built, move its text into `spec/index.md` (the single
source of truth), and delete it here. Ranking and status: `plan.md`; work
items: `tasks.md`; background: `spec/compare/index.md`.

Every proposal keeps the existing guarantees in "Security" in
`spec/index.md`: files only inside the output directory, no symbolic links,
no off-site redirects, bounded memory.

## P1. Browsable offline copies

Built: CSS URL discovery, `-E`, `-p`, `-k`, `-H`/`-D`; from Phase 2:
`--events`, `--sitemap`, URL normalization, `--accept-mime`/`--reject-mime`,
`--max-time`; and from Phase 3 (archival and refresh): the crawl state with
resume and `--fresh`, `ETag` refresh, `--warc-file`, `--load-cookies`,
`--save-cookies` (see `spec/index.md`).

## P2. Speed, politely

### `--max-connections=N`

Fetch up to N URLs at once (default 1, today's behavior), but never more
than one at a time per host unless `--per-host=N` says so, and always
honouring `--wait` and `Crawl-delay` per host. Output order and the "never
download twice" rule stay the same.

## P3. Smaller conveniences

- `--config=FILE`: load defaults from a TOML file (`app/config.rs` and
  `app/confy.rs` exist but are not loaded yet).
- `-nd`, `--no-directories`: save every file in one directory; name clashes
  get `-1`, `-2` suffixes, never overwriting.
- `--check-external`: with `--spider`, also check (HEAD, then GET) links that
  leave the scope, without crawling them, and report broken ones with the
  pages that link to them, like lychee and muffet.
- `--ignore-file=FILE`: reject patterns re-read during the crawl, so a crawl
  trap can be cut off without restarting (grab-site's dynamic ignores).
