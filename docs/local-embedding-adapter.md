# Local embedding adapter

Date: 2026-06-27  
Task: `20260626-095451-vault-layer-upgrade-pin-the-local-embedding-dependency-path-so-vaultlayer-can`

## Summary

VaultLayer now supports a real local embedding model path without adding a Rust `fastembed` dependency that breaks the current Cargo 1.75 toolchain.

The CLI keeps `deterministic-v0` as the default test/smoke provider and adds an explicit model switch:

```bash
vault-layer embed --db <db> --model fastembed-mini-lm
vault-layer vector-search "agent context" --db <db> --model fastembed-mini-lm --json
vault-layer hybrid-search "agent context" --db <db> --model fastembed-mini-lm --json
```

`fastembed-mini-lm` maps to:

```text
fastembed:sentence-transformers/all-MiniLM-L6-v2
```

Expected dimensions: `384`.

## Runtime model path

The Rust CLI stays dependency-light and calls `scripts/fastembed_embed.py` only when `--model fastembed-mini-lm` is requested. The helper uses Python `fastembed` + ONNX Runtime for local inference.

Default cache path:

```text
~/.local/share/vault-layer/models/fastembed/
```

Overrides:

```bash
VAULT_LAYER_FASTEMBED_CACHE_DIR=/path/outside/repo-and-vault
VAULT_LAYER_FASTEMBED_PYTHON=/path/to/python-with-fastembed
```

Install optional runtime dependency in the Python environment used by the CLI:

```bash
python3 -m pip install fastembed==0.7.3
```

No SaaS embedding API, hosted vector service, `TURSO_DATABASE_URL`, `TURSO_AUTH_TOKEN`, OpenAI key, or other token is required for normal local operation. First use may download the ONNX model into the cache directory; subsequent cached runs are local/offline.

## Storage contract

The `embeddings` table is keyed by `(chunk_id, model)` and records dimensions per row, so deterministic and real local model vectors can coexist for the same chunk:

```text
deterministic-v0 | 8
fastembed:sentence-transformers/all-MiniLM-L6-v2 | 384
```

The sqlite-vec materialization is refreshed for the selected model after `embed`.

## Test-vault evidence

Input:

```text
vault=/mnt/c/Users/viggo/github/obsidian-test-vault
notes_indexed=13
chunks=15
state=/tmp/vault-layer-fastembed-test-vault
```

Observed:

```text
deterministic-v0|8|15
fastembed:sentence-transformers/all-MiniLM-L6-v2|384|15
deterministic vector runtime=native-sqlite-vec
fastembed vector runtime=native-sqlite-vec
cache_dir=/home/viggo/.local/share/vault-layer/models/fastembed
```

## 5000-note bounded real-vault evidence

Input:

```text
vault=/mnt/c/Users/Viggo/Syncthing/vault
limit=5000
state=/tmp/vault-layer-fastembed-real-5000
```

Observed:

```text
notes=5000
sections=8304
fts_rows=8304
index_elapsed=1:52.73 index_maxrss=59264KB
embed_deterministic_elapsed=0:01.22 embed_deterministic_maxrss=59648KB
vector_deterministic_elapsed=0:00.37 vector_deterministic_maxrss=59648KB
embed_fastembed_elapsed=2:33.73 embed_fastembed_maxrss=1963516KB
vector_fastembed_elapsed=0:02.26 vector_fastembed_maxrss=221908KB
deterministic-v0|8|8304
fastembed:sentence-transformers/all-MiniLM-L6-v2|384|8304
```

The two models returned different top vector hits for `agent context`, which confirms the real model path is not just deterministic-v0 relabeled.

## Caveats

- The Python `fastembed` package is an optional runtime dependency, not vendored into the repo.
- First-run model download is expected unless the cache is already populated.
- Full-vault production runs still need progress/resume hardening before they become unattended release gates.
