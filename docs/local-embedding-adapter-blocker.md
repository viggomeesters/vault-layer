# Local embedding adapter blocker history

Date: 2026-06-26  
Task: `20260625-204500-vault-layer-real-local-embedding-adapter`

## Resolution

The original Rust-crate path remains blocked under Cargo 1.75, but the follow-up task implemented a compatible local adapter through Python `fastembed` + ONNX Runtime. See [`local-embedding-adapter.md`](local-embedding-adapter.md) for the current working path and 5000-note evidence.

## Original decision

VaultLayer should use a real local/open-source embedding backend before claiming semantic retrieval quality. The first candidate is `fastembed` with `sentence-transformers/all-MiniLM-L6-v2` because it runs local ONNX inference, needs no SaaS API token, and produces 384-dimensional retrieval embeddings.

The adapter was not merged in this task because the current repository toolchain is pinned to Rust/Cargo 1.75.0 and the tested `fastembed` dependency chain requires crates that use the unstable `edition2024` manifest feature.

## Measured blocker evidence

Attempted command:

```bash
cargo add fastembed@5.17.2 -p vault-layer
cargo check -p vault-layer
```

Observed blocker:

```text
cargo 1.75.0
rustc 1.75.0 (82e1608df 2023-12-21)
failed to parse manifest ... hashbrown-0.17.1/Cargo.toml
feature `edition2024` is required
```

A second attempt pinned `fastembed` to `=4.9.0` and downgraded `indexmap` away from `hashbrown 0.17.1`; the dependency graph then failed on another transitive crate with the same toolchain class:

```text
failed to parse manifest ... idna_adapter-1.2.2/Cargo.toml
feature `edition2024` is required
```

This is a toolchain/dependency compatibility blocker, not a semantic-quality pass.

## Schema hardening completed

The `embeddings` table now uses a composite primary key:

```sql
PRIMARY KEY(chunk_id, model)
```

That lets deterministic test embeddings and future real local model embeddings coexist for the same chunk while keeping explicit `model` and `dimensions` fields per row.

## Runtime/cache policy for the future adapter

When the real adapter lands:

- model identity must be explicit, for example `fastembed:sentence-transformers/all-MiniLM-L6-v2`;
- dimensions must be recorded from generated vectors, expected `384` for all-MiniLM-L6-v2;
- model/cache files must live outside the repo and vault, for example `~/.local/share/vault-layer/models/fastembed/`;
- normal retrieval must not require `TURSO_DATABASE_URL`, `TURSO_AUTH_TOKEN`, OpenAI keys, or any SaaS embedding endpoint;
- first-run download must be explicit in docs; cached/offline runs must be supported after model materialization;
- `deterministic-v0` remains the test/smoke fallback and must not be used as semantic-quality evidence.

## Required follow-up before semantic quality claims

1. Upgrade or override the Rust toolchain for this repo to a version that accepts the selected embedding dependency graph, or vendor/pin a compatible local ONNX embedding stack.
2. Add an explicit CLI/config switch such as `vault-layer embed --model fastembed-mini-lm` and matching `vector-search` / `hybrid-search --model fastembed-mini-lm`.
3. Re-run the existing gates on both:
   - the synthetic/test vault;
   - the bounded 5000-note real-vault input.
4. Compare deterministic-v0 vs real-model output quality with query examples and store the evidence outside the repo/vault, with only summarized public-safe docs committed.
