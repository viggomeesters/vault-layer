# VaultLayer MCP Interface

VaultLayer exposes an agent-facing MCP-compatible tool contract. The current MVP ships a CLI smoke surface that maps directly to the future stdio MCP server tools.

## Tools

| Tool | Purpose | Required input |
|---|---|---|
| `vault_search` | Search indexed chunks with citations | `query`, `db` |
| `vault_get_note` | Return one bounded note with provenance | `query`, `db` |
| `vault_related` | Return WikiLink/backlink related notes | `query`, `db` |

Every tool result includes bounded excerpts and provenance fields where applicable: `path`, `heading_path`, `chunk_id`, `content_hash`, `modified_unix`.

## Smoke commands

```bash
vault-layer serve --mcp --list-tools
vault-layer serve --mcp --call vault_search --query "agent" --db <db>
vault-layer serve --mcp --call vault_get_note --query "Projects/example.md" --db <db>
vault-layer serve --mcp --call vault_related --query "Projects/example.md" --db <db>
```

## Hermes setup sketch

Configure a future stdio MCP server to run:

```bash
vault-layer serve --mcp --db ~/.local/share/vault-layer/<vault-id>/vault-layer.db
```

The MVP keeps the mapping explicit and testable before adding a long-running JSON-RPC loop.
