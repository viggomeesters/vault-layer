
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/hero.svg">
  <img alt="VaultLayer — files stay yours, agents get a database" src="assets/hero.svg">
</picture>

# VaultLayer

Project page: <https://viggomeesters.com/vault-layer/>

[![status: early](https://img.shields.io/badge/status-early-orange)](#status)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![gate: make check](https://img.shields.io/badge/gate-make%20check-0f766e)](#verify)

**Stop making agents crawl your vault. Give them a local read model instead.**

VaultLayer turns a Markdown/Obsidian vault into a rebuildable local database with full-text search, vectors, WikiLinks, metadata, provenance, and CLI/MCP access. Your vault stays plain files. The generated index, embeddings, benchmark reports, and caches live outside the vault and outside the repo.

## Why download this?

Download VaultLayer if you have a serious Markdown/Obsidian vault and want to:

- query it without repeatedly scanning folders and parsing Markdown;
- give agents bounded, cited context instead of dumping broad filesystem reads into prompts;
- keep the vault as source of truth while generating disposable search/vector state elsewhere;
- preserve provenance for every result: path, heading/chunk id, content hash, modified time, and excerpt;
- compare raw filesystem search vs indexed retrieval with a repeatable benchmark;
- prototype local-first vault retrieval before committing to a viewer, MCP server, or cloud sync story.

## What you get

```text
Markdown/Obsidian vault -> external VaultLayer DB -> CLI / MCP / viewer / benchmark
```

Current pilot includes:

- read-only indexing of Markdown/Obsidian-style vaults;
- SQLite + FTS5 local search by default;
- sqlite-vec + real local FastEmbed MiniLM vector retrieval;
- deterministic synthetic messy-vault preflight;
- package, doctor, and benchmark scripts;
- safety guards against committing private vault content or generated DB/cache artifacts.

## What this is not yet

VaultLayer is a **pilot-ready local MVP**, not a production product or a guaranteed speedup for every vault. Performance depends on vault size, filesystem, machine, query shape, and first-run model/cache setup. The repo gives you a safe way to test and measure that locally.

## Quick start

```bash
git clone https://github.com/viggomeesters/vault-layer.git
cd vault-layer
make check
cargo run -p vault-layer -- --help
```

Prove the flow on a generated messy fake vault, without touching a real vault:

```bash
python3 scripts/make_messy_vault.py /tmp/vault-layer-messy --force
scripts/package_smoke.sh /tmp/vault-layer-messy --work-dir /tmp/vault-layer-package-smoke
scripts/benchmark_vault.sh /tmp/vault-layer-messy \
  --state-dir /tmp/vault-layer-benchmark \
  --query "performance baseline vector provenance" \
  --limit 500
```

Expected proof points: `package_smoke=ok`, `doctor_status=ok`, `runtime_outside_vault=true`, indexed notes/sections, local embeddings, and benchmark timings.

Index your own local vault when you are ready:

```bash
cargo run -p vault-layer -- index /path/to/vault --state-dir ~/.local/share/vault-layer --limit 20
```

Inspect the configured storage backend:

```bash
cargo run -p vault-layer -- backend-info

# Recommended local SQLite + FTS5 retrieval projection, no credentials/network
cargo run -p vault-layer -- index /path/to/vault

# Optional DuckDB analytics/export sidecar
VAULT_LAYER_BACKEND=duckdb cargo run -p vault-layer -- index /path/to/vault

# Explicit remote sync to hosted Turso/libSQL (requires real credentials)
TURSO_DATABASE_URL=libsql://your-database.turso.io \
TURSO_AUTH_TOKEN=*** \
cargo run -p vault-layer -- sync-turso /path/to/vault --limit 100
```

Local SQLite + FTS5 is the implemented primary retrieval default. `TURSO_DATABASE_URL` / `TURSO_AUTH_TOKEN` can be configured for the Turso/libSQL target, but remote sync only runs through explicit `sync-turso` / `index --remote-sync`; VaultLayer will not upload private vault text by accident.

Search with citations:

```bash
vault-layer search "agent context" --db ~/.local/share/vault-layer/<vault-id>/vault-layer.db --json
vault-layer get-note "Projects/example.md" --db <db> --json
vault-layer related "Projects/example.md" --db <db> --json
```

Generate local embeddings and run vector retrieval:

```bash
# deterministic smoke/test provider
vault-layer embed --db <db> --model deterministic-v0

# real local ONNX model via Python fastembed; cache stays outside repo/vault
python3 -m pip install fastembed==0.7.3
vault-layer embed --db <db> --model fastembed-mini-lm
vault-layer vector-search "agent context" --db <db> --model fastembed-mini-lm --json
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
- SQLite + FTS5 is the recommended/default local retrieval backend over `.md` while the vault remains source of truth.
- sqlite-vec is the intended native local vector path; `fastembed-mini-lm` is the working real local embedding model path, while deterministic JSON cosine remains a smoke-test fallback.
- DuckDB is an optional analytics/export sidecar: set `VAULT_LAYER_BACKEND=duckdb`.
- Hosted Turso/libSQL is treated as cloud/sync/export target, not the local core.

## Product split

- **VaultLayer core** — parser, stable IDs, shadow DB, search, vectors, provenance, human relevance scores.
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
- [`docs/claim-evidence-gate.md`](docs/claim-evidence-gate.md)
- [`docs/full-vault-progress-resume.md`](docs/full-vault-progress-resume.md)
- [`docs/local-embedding-adapter.md`](docs/local-embedding-adapter.md)
- [`docs/niels-pilot-install.md`](docs/niels-pilot-install.md)
- [`docs/niels-pilot-benchmark.md`](docs/niels-pilot-benchmark.md)
- [`docs/niels-pilot-runbook.md`](docs/niels-pilot-runbook.md)
- [`docs/synthetic-messy-vault-preflight.md`](docs/synthetic-messy-vault-preflight.md)
- [`docs/local-embedding-adapter-blocker.md`](docs/local-embedding-adapter-blocker.md)

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


## Backend decision

The accepted backend decision is documented in `docs/ADR-0001-primary-retrieval-backend.md`; benchmark evidence lives in `docs/backend-decision-benchmark.md`.


## sqlite-vec status

Native sqlite-vec is feasible and has a scoped Rust/rusqlite smoke adapter exposed via `vault-layer sqlite-vec-info`; see `docs/sqlite-vec-packaging-spike.md`. `vault-layer embed` refreshes sqlite-vec rows for the selected model, including the working real local `fastembed-mini-lm` path.


## Retrieval benchmark evidence

Current bounded real-vault retrieval benchmark evidence lives in `docs/full-vault-retrieval-benchmark.md`. Long index runs now emit progress and can skip rewriting an existing same-count SQLite DB; see `docs/full-vault-progress-resume.md`. Target-vault performance still must be proven per pilot with `scripts/benchmark_vault.sh`.


## Retrieval quality

Vector fallback results now expose `cosine_score` and `text_quality_score` so low-information chunks can be demoted while native sqlite-vec and real embeddings mature. See `docs/retrieval-quality-first-pass.md`.


## Hybrid retrieval

`vault-layer embed` refreshes native sqlite-vec rows when available, `vector-search` prefers native sqlite-vec KNN, and `hybrid-search` reranks FTS candidates with vector, human relevance, and text-quality signals. Use `--model fastembed-mini-lm` for the working real local model path, or `--model deterministic-v0` for smoke tests. See `docs/sqlite-vec-hybrid-retrieval.md` and `docs/local-embedding-adapter.md`.
