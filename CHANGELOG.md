# Changelog

## Unreleased

- Add `scripts/make_messy_vault.py` and `docs/synthetic-messy-vault-preflight.md`; package smoke and benchmark now have a deterministic fake messy-vault preflight before Niels private-vault testing.
- Add `scripts/package_smoke.sh` and include the claim gate in the pilot package so the tarball is verified from outside the repo with explicit FastEmbed helper paths and no bundled runtime/private artifacts.
- Add `docs/claim-evidence-gate.md` to make self-contained/performance-win language falsifiable and disallow unscoped guarantees until target-vault evidence passes.
- Align public product claims around “pilot-ready local MVP, not fully self-contained production product” and add `docs/niels-pilot-runbook.md` with safe install, benchmark, success metrics, cleanup, and stop criteria.
- Add `scripts/package_pilot.sh` and expand `docs/PACKAGE.md` for a local pilot artifact with release binary, helper scripts, docs, SHA256, FastEmbed runtime expectations, and cleanup commands.
- Add progress output for long index runs and a conservative existing-DB same-note-count write skip; document the current resume boundary in `docs/full-vault-progress-resume.md`.
- Add `scripts/benchmark_vault.sh` and `docs/niels-pilot-benchmark.md` to measure baseline filesystem search, VaultLayer index/search/embed/vector timings, DB size, and provenance sample paths for a target-vault pilot.
- Add `vault-layer doctor` plus `scripts/pilot_doctor.sh` and `docs/niels-pilot-install.md` so a Niels-style local pilot can verify read-only vault access, local state/cache placement, sqlite-vec, Python FastEmbed, and disabled remote sync before indexing.
- Add `--model fastembed-mini-lm` for real local Python FastEmbed/ONNX embeddings across `embed`, `vector-search`, and `hybrid-search`, with model cache outside repo/vault and 5000-note deterministic-vs-real evidence in docs.

- Record the Rust-crate local embedding adapter blocker for `fastembed`/local ONNX under Cargo 1.75 and harden `embeddings` to keep `(chunk_id, model)` rows with explicit dimensions for deterministic-v0 vs real model comparison.

- Add native sqlite-vec table search and `hybrid-search` reranking that combines FTS, vector score, human relevance, and text quality while preserving provenance.

- Add retrieval quality first pass for vector fallback: score now includes `text_quality_score` and `cosine_score`, demoting status-only and boilerplate chunks.

- Add test-vault retrieval benchmark evidence before full-vault scale validation.

- Add bounded real-vault retrieval benchmark evidence and document the full-vault WSL progress/resume blocker.

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
