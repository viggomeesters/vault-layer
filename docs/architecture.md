# VaultLayer Architecture

## Principle

The Markdown vault remains the source of truth. VaultLayer builds a rebuildable shadow database for fast retrieval, agent context, and viewer read models.

## Layers

```text
Markdown/Obsidian vault
  -> parser/chunker/link extractor
  -> local shadow database
  -> hybrid retrieval engine
  -> CLI / MCP / HTTP / viewer adapter
```

## Runtime data location

Index data is stored outside the Git repository. Default Linux/WSL location:

```text
~/.local/share/vault-layer/<vault-id>/
```

A future config may support custom paths, but the repo itself must stay free of private data and generated DB files.

## Retrieval model

Hybrid retrieval combines:

1. exact path/title lookup
2. FTS/BM25
3. WikiLink/backlink graph
4. metadata filters
5. vector similarity through SQLite/libSQL/Turso-compatible vector columns
6. reranking and citation packing

## Initial schemas

Planned core tables:

- `vaults`
- `notes`
- `sections`
- `chunks`
- `links`
- `frontmatter`
- `tags`
- `embeddings`
- `index_runs`
- `provenance`

## Safety

- Read-only vault scan by default.
- No writeback in MVP.
- Every result includes path, heading, chunk id, content hash, and excerpt.
- Private sample vault data is forbidden in the repo.

## CLI skeleton

The first CLI surface reserves `init`, `index`, `search`, `context`, and `serve`. `init` already reports the external runtime state directory and keeps writeback disabled. Later tasks fill the scanner, store, retrieval, vector, and MCP behavior behind this command surface.
