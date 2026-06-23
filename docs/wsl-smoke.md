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

For WSL-mounted Windows vaults, use bounded smoke runs before full indexing.
Large `/mnt/c` trees can be slow to enumerate from WSL. VaultLayer skips hidden
runtime folders such as `.obsidian`, `.stversions`, `.hermes`, and `.git` by
default, but full-vault indexing should become resumable/incremental before it is
treated as unattended production indexing.

Expected evidence:

- `db_path` is under `~/.local/share/vault-layer/` or `VAULT_LAYER_STATE_DIR`.
- `db_path` is not under the repository.
- `db_path` is not under the vault.
- `repo_db_files=0`.
- Vault writeback remains disabled.
