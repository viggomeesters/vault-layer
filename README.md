# VaultLayer

Local-first database and retrieval layer for huge Markdown/Obsidian vaults.

**Goal:** keep the vault as plain files, while humans and agents get a fast query layer: metadata, WikiLinks, FTS, vectors, citations, and MCP.

## Status

Planning / early scaffold. Public repo, but not production-ready yet.

## Product split

- **VaultLayer core** indexes Markdown vaults into a rebuildable local shadow database.
- **VaultLayer CLI/MCP** exposes fast search/context tools for agents such as Hermes, Codex, and local assistants.
- **Mega Vault Viewer** can consume the same index as a fast human UI above huge vaults.

## Non-goals

- The vault is not converted into a database.
- The repository must never contain a user's private vault index.
- Vault writeback is disabled by default and out of scope for the first MVP.

## Storage contract

Runtime indexes live outside the repo, for example:

```text
~/.local/share/vault-layer/
```

For Viggo's WSL machine, the initial target vault path is external to this repo:

```text
/mnt/c/Users/Viggo/Syncthing/vault
```

The local shadow database belongs under user data/state directories, not under the Git checkout.

## Planned MVP

```bash
vault-layer init /path/to/vault
vault-layer index --limit 1000
vault-layer search "open loops rond Supabase" --json
vault-layer context "wat weten we over Turso en vault indexering?" --json
vault-layer serve --mcp
```

## License

MIT.
