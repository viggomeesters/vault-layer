# Roadmap

## v0.1.x — public-safe local core

- Harden CLI UX and config files.
- Replace shell-out SQLite writer with a Rust SQLite/libSQL abstraction.
- Add real integration tests around synthetic fixture vaults.
- Add stdio JSON-RPC MCP server loop.

## v0.2 — real retrieval quality

- Hybrid scoring across exact path, FTS, links, tags, recency, and vectors.
- Pluggable embedding providers.
- Native libSQL/Turso vector backend when available.
- Incremental indexing and stale-embedding detection.

## v0.3 — huge-vault operations

- Performance profiles for 10k/100k note vaults.
- Watch mode and resumable indexing.
- Corruption recovery and rebuild UX.
- Viewer read-model stability for Mega Vault Viewer.

## v0.4 — multi-agent integration

- First-class Hermes/Codex MCP setup docs.
- Safer writeback design, still disabled by default.
- Optional remote Turso sync for multi-device shadow indexes.
