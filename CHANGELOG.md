# Changelog

## Unreleased

- Add `human_relevance_score` to notes, sections, provenance, and cited retrieval outputs.
- Add explicit `backend-info` command for SQLite vs Turso/libSQL capability reporting.
- Add Turso/libSQL environment contract without enabling accidental remote index writes.
- Skip hidden runtime folders during vault scans and fix deterministic embedding writes for multiline chunks.

## v0.1.0 - 2026-06-23

Initial public-ready release.

- Rust workspace and `vault-layer` CLI scaffold.
- Read-only Markdown vault scanner with stable note/section/chunk IDs.
- SQLite/libSQL-compatible shadow database schema.
- Cited search, note lookup, related-links, context, deterministic embeddings, vector-search spike.
- MCP smoke interface and WSL bounded smoke script.
- Public repo-complete docs, guardrails, and governance.
