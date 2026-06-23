# Mega Vault Viewer Adapter Contract

Mega Vault Viewer is a human UI consumer of VaultLayer. VaultLayer owns indexing, retrieval, provenance, and local shadow DB storage.

## Boundary

```text
Markdown/Obsidian vault -> VaultLayer index DB -> Viewer read models -> Mega Vault Viewer UI
```

Mega Vault Viewer must not create its own incompatible vault index. It can cache UI state, but durable search/link/chunk/entity data should come from VaultLayer.

## Viewer needs

| Need | VaultLayer source |
|---|---|
| Fast note list | `notes` table / future `viewer_notes` view |
| Open note quickly | `vault_get_note` / `notes` + `sections` |
| Backlinks/outlinks | `links` table + `vault_related` |
| Graph slices | `links`, `tags`, future entity edges |
| Search | `vault_search`, FTS, vector search |
| Task/project/person views | metadata/frontmatter/tags + future typed extractors |
| Agent context panel | `context`, `vector-search`, MCP tools |

## Initial read models

The current DB schema can already support:

```sql
CREATE VIEW IF NOT EXISTS viewer_notes AS
SELECT id, path, title, modified_unix, content_hash
FROM notes
ORDER BY modified_unix DESC;

CREATE VIEW IF NOT EXISTS viewer_links AS
SELECT n.path AS source_path, l.target, l.raw
FROM links l
JOIN notes n ON n.id = l.source_note_id;
```

These views are documented as adapter contract, not yet required in the schema until the viewer consumes them.

## API shape

Viewer-facing responses should be stable and small:

```json
{
  "path": "Projects/example.md",
  "title": "Example",
  "modified_unix": 1234567890,
  "content_hash": "...",
  "preview": "bounded excerpt"
}
```

Agent-facing responses include more provenance:

```json
{
  "chunk_id": "chunk_...",
  "path": "Projects/example.md",
  "heading_path": "Decision",
  "excerpt": "bounded excerpt",
  "content_hash": "...",
  "modified_unix": 1234567890
}
```

## Tauri/Rust integration notes

- The viewer can link to `vault-layer-core` directly for embedded desktop indexing, or call `vault-layer` CLI/MCP as a sidecar.
- For huge vaults, prefer sidecar/worker indexing so UI startup stays fast.
- DB files remain under user data directories, not inside the viewer repo and not inside the vault.
- UI should treat the DB as rebuildable cache; markdown files remain canonical.

## Decision

VaultLayer is the shared engine. Mega Vault Viewer is a cockpit. This keeps agent tooling, CLI, MCP, and UI from drifting into separate indexing implementations.
