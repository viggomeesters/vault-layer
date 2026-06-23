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

Expected evidence:

- `db_path` is under `~/.local/share/vault-layer/` or `VAULT_LAYER_STATE_DIR`.
- `db_path` is not under the repository.
- `db_path` is not under the vault.
- `repo_db_files=0`.
- Vault writeback remains disabled.
