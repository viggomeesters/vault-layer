# Changelog

## Unreleased

- Add `vault-layer-sqlite-vec` native smoke adapter and `vault-layer sqlite-vec-info`; `backend-info` now reports sqlite-vec availability while production vector search remains explicit JSON cosine fallback.

- Add sqlite-vec packaging spike evidence: native sqlite-vec builds on WSL via rusqlite bundled SQLite, but needs a scoped Rust adapter/unsafe boundary instead of the system sqlite3 CLI writer.

- Add ADR 0001 documenting SQLite + FTS5 as primary retrieval, DuckDB as analytics sidecar, and Turso/libSQL as explicit remote/cloud sync.

- Restore SQLite + FTS5 as the recommended/default primary retrieval backend (`vault-layer.db`) and use BM25-ranked FTS search for SQLite queries.
- Reposition DuckDB as an explicit analytics/export sidecar (`VAULT_LAYER_BACKEND=duckdb`).
- Implement local open-source libSQL/Turso-compatible indexing via `VAULT_LAYER_BACKEND=libsql-local` with no URL/token.
- Implement explicit Turso/libSQL remote sync via HTTPS pipeline (`sync-turso` and `index --remote-sync`).
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
