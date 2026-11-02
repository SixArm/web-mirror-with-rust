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
`--save-cookies`; from Phase 4: `--max-connections`, `--per-host`; and from
Phase 5: `--config`, `--no-directories`, `--check-external`, `--ignore-file`
(see `spec/index.md`).

Every proposal so far is built. New proposals go here.
