# Roadmap

## v0.1.x — pilot-ready local core

- Keep the public-safe local core green: read-only vault input, runtime state outside repo/vault, repository guard, and `make check`.
- Harden CLI UX and config files.
- Keep `vault-layer doctor`, `scripts/pilot_doctor.sh`, `scripts/benchmark_vault.sh`, and `scripts/package_pilot.sh` usable for read-only pilots.
- Replace shell-out SQLite writer with a Rust SQLite/libSQL abstraction.
- Add real integration tests around synthetic fixture vaults.
- Add stdio JSON-RPC MCP server loop.

## v0.2 — real retrieval quality

- Hybrid scoring across exact path, FTS, links, tags, recency, and vectors.
- Maintain pluggable embedding providers: `deterministic-v0` for smoke tests and `fastembed-mini-lm` as the first real local model path.
- Native libSQL/Turso vector backend when available.
- Content-hash-level incremental indexing and stale-embedding detection.

## v0.3 — huge-vault operations

- Performance profiles for 10k/100k note vaults.
- Watch mode and resumable indexing.
- Corruption recovery and rebuild UX.
- Viewer read-model stability for Mega Vault Viewer.

## v0.4 — multi-agent integration

- First-class Hermes/Codex MCP setup docs.
- Safer writeback design, still disabled by default.
- Harden optional remote Turso sync for multi-device shadow indexes with resumable batches and native vector columns.
