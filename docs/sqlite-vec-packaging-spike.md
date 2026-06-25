# sqlite-vec packaging spike

Date: 2026-06-25  
Task: `20260625-112922-sqlite-vec-packaging-spike`

## Result

Native sqlite-vec is feasible on the current WSL/Rust path, but it needs an explicit Rust-owned SQLite connection path rather than the existing system `sqlite3` CLI writer.

## Proof command

A temporary proof crate was built outside the repo at `/tmp/vault-layer-sqlite-vec-proof` with:

```toml
rusqlite = { version = "0.31", features = ["bundled"] }
sqlite-vec = "0.1.9"
zerocopy = "0.7"
```

The proof registered sqlite-vec through `rusqlite::ffi::sqlite3_auto_extension`, opened an in-memory SQLite DB, created a `vec0` virtual table, inserted a vector, and queried nearest neighbor distance.

Observed output:

```text
vec_version=v0.1.9 embedding=[0.100000,0.200000,0.300000]
distance=0
```

## Integration decision

Implement sqlite-vec through a separate Rust/rusqlite path for SQLite vector tables. Do not try to force native sqlite-vec through the current system `sqlite3` CLI SQL import path because the sqlite-vec Rust crate statically links the extension into the Rust binary and exposes `sqlite3_vec_init`; it is not a standalone `.so` that the CLI can `.load` without extra distribution work.

The current workspace has `unsafe_code = "forbid"`, while sqlite-vec registration requires the standard unsafe call:

```rust
sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())))
```

So the clean implementation path is:

1. Add a small, tightly scoped adapter module/crate for sqlite-vec registration.
2. Keep the unsafe boundary tiny and documented.
3. Use rusqlite with bundled SQLite for vector table creation/query.
4. Keep deterministic JSON cosine as fallback until native path is fully green.

## Product boundary

sqlite-vec is a native local vector backend candidate, not a new product identity. SQLite + FTS5 remains primary retrieval; sqlite-vec should add vector acceleration inside the same local-first story.

## Safety

- Proof used synthetic vectors only.
- No private vault content was copied into the repo.
- No generated DB/vector artifacts were committed.
