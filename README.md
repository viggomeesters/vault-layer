
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/hero.svg">
  <img alt="VaultLayer — files stay yours, agents get a database" src="assets/hero.svg">
</picture>

# VaultLayer

[![status: early](https://img.shields.io/badge/status-early-orange)](#status)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![gate: make check](https://img.shields.io/badge/gate-make%20check-0f766e)](#verify)

**VaultLayer is a local-first database and retrieval layer for huge Markdown/Obsidian vaults.**

Your vault stays plain files. VaultLayer builds a rebuildable shadow database with metadata, WikiLinks, FTS, vectors, citations, CLI, and MCP tools so humans and agents do not have to crawl 100k notes every time.

## Status

Early public MVP. Useful for experiments and architecture work; not production-stable yet.

## Why

Obsidian is excellent as a human writing surface, but very large vaults need a fast read model. Agents also need bounded, cited, queryable context instead of broad filesystem scans.

VaultLayer provides the shared engine:

```text
Markdown/Obsidian vault -> VaultLayer index DB -> CLI/MCP/Viewer
```

## Quick start

```bash
git clone https://github.com/viggomeesters/vault-layer.git
cd vault-layer
make check
cargo run -p vault-layer -- --help
```

Index a small synthetic or local test vault:

```bash
cargo run -p vault-layer -- index /path/to/vault --state-dir ~/.local/share/vault-layer --limit 20
```

Search with citations:

```bash
vault-layer search "agent context" --db ~/.local/share/vault-layer/<vault-id>/vault-layer.db --json
vault-layer get-note "Projects/example.md" --db <db> --json
vault-layer related "Projects/example.md" --db <db> --json
```

MCP smoke interface:

```bash
vault-layer serve --mcp --list-tools
vault-layer serve --mcp --call vault_search --query "agent" --db <db>
```

## Safety boundary

VaultLayer treats the source vault as read-only by default.

- Do **not** commit private vault content.
- Do **not** commit generated DB/index/embedding files.
- Runtime state belongs outside both the repo and the vault, e.g. `~/.local/share/vault-layer/`.
- Examples and tests must use synthetic fixtures.
- Writeback is disabled in the MVP.

## Product split

- **VaultLayer core** — parser, stable IDs, shadow DB, search, vectors, provenance.
- **VaultLayer CLI/MCP** — agent and automation surface.
- **Mega Vault Viewer** — human UI consumer of VaultLayer read models.

See [`docs/viewer-adapter.md`](docs/viewer-adapter.md).

## Docs

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- [`docs/api.md`](docs/api.md)
- [`docs/embeddings.md`](docs/embeddings.md)
- [`docs/mcp.md`](docs/mcp.md)
- [`docs/wsl-smoke.md`](docs/wsl-smoke.md)
- [`docs/ROADMAP.md`](docs/ROADMAP.md)
- [`docs/REPO_COMPLETE.md`](docs/REPO_COMPLETE.md)
- [`docs/FILL_LOOP.md`](docs/FILL_LOOP.md)

## Verify

```bash
make check
```

The gate runs Rust checks/tests, repository safety guard, Python guard tests, `git diff --check`, and a generated-artifact tracking check.

## Package

```bash
cargo build --release -p vault-layer
./target/release/vault-layer --help
```

See [`docs/PACKAGE.md`](docs/PACKAGE.md).

## Contributing

Read [`CONTRIBUTORS.md`](CONTRIBUTORS.md), [`SUPPORT.md`](SUPPORT.md), [`SECURITY.md`](SECURITY.md), and [`AGENTS.md`](AGENTS.md). Keep all fixtures synthetic and all runtime artifacts outside Git.

## License

MIT. See [`LICENSE`](LICENSE).
