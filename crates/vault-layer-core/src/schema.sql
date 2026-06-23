-- VaultLayer local SQLite/libSQL-compatible schema.
-- This file is public and contains no private vault data.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS vaults (
  id TEXT PRIMARY KEY,
  root_path TEXT NOT NULL,
  indexed_at_unix INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS notes (
  id TEXT PRIMARY KEY,
  vault_id TEXT NOT NULL REFERENCES vaults(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  title TEXT NOT NULL,
  modified_unix INTEGER NOT NULL,
  content_hash TEXT NOT NULL,
  UNIQUE(vault_id, path)
);

CREATE TABLE IF NOT EXISTS sections (
  id TEXT PRIMARY KEY,
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  heading_path TEXT NOT NULL,
  level INTEGER NOT NULL,
  text TEXT NOT NULL,
  content_hash TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS links (
  source_note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  target TEXT NOT NULL,
  raw TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS frontmatter (
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  tag TEXT NOT NULL,
  UNIQUE(note_id, tag)
);

CREATE TABLE IF NOT EXISTS index_runs (
  id TEXT PRIMARY KEY,
  vault_id TEXT NOT NULL REFERENCES vaults(id) ON DELETE CASCADE,
  started_at_unix INTEGER NOT NULL,
  notes_indexed INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS provenance (
  chunk_id TEXT PRIMARY KEY REFERENCES sections(id) ON DELETE CASCADE,
  note_path TEXT NOT NULL,
  heading_path TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  modified_unix INTEGER NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS sections_fts USING fts5(
  id UNINDEXED,
  note_id UNINDEXED,
  path UNINDEXED,
  heading_path,
  text
);

CREATE TABLE IF NOT EXISTS embeddings (
  chunk_id TEXT PRIMARY KEY REFERENCES sections(id) ON DELETE CASCADE,
  model TEXT NOT NULL,
  dimensions INTEGER NOT NULL,
  embedding_json TEXT NOT NULL,
  -- Future libSQL/Turso native shape: embedding F32_BLOB(dimensions) with libsql_vector_idx(embedding).
  embedded_at_unix INTEGER NOT NULL
);
