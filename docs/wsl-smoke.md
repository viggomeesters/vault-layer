# WSL Smoke Run

VaultLayer can smoke-index Viggo's Windows/Syncthing vault from WSL as read-only input. Runtime state is written outside both the repo and the vault.

Default target:

```text
/mnt/c/Users/Viggo/Syncthing/vault
```

Default state:

```text
~/.local/share/vault-layer/
```

Run a bounded smoke first:

```bash
bash scripts/wsl-smoke.sh /mnt/c/Users/Viggo/Syncthing/vault 20
```

The smoke defaults to `VAULT_LAYER_BACKEND=sqlite` unless the caller sets another backend. It reports:

- `elapsed` and `maxrss` from `/usr/bin/time`;
- `db_path` and `db_size_bytes`;
- `notes_indexed`, `sections_indexed`, `embeddings_before`, `embeddings_after` when the backend is SQLite;
- `sample_search`, `sample_embed`, and `sample_vector` JSON;
- `repo_db_files`, which must remain `0`.

For WSL-mounted Windows vaults, use bounded smoke runs before full indexing. Large `/mnt/c` trees can be slow to enumerate from WSL. VaultLayer skips hidden runtime folders such as `.obsidian`, `.stversions`, `.hermes`, and `.git` by default.

Expected evidence:

- `db_path` is under `~/.local/share/vault-layer/` or `VAULT_LAYER_STATE_DIR`.
- `db_path` is not under the repository.
- `db_path` is not under the vault.
- `repo_db_files=0`.
- Search/vector output includes provenance fields such as `chunk_id`, `path`, `heading_path`, `content_hash`, `modified_unix`, and `human_relevance_score`.
- Vault writeback remains disabled.
