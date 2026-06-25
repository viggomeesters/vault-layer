#!/usr/bin/env bash
set -euo pipefail

VAULT_PATH="${1:-/mnt/c/Users/Viggo/Syncthing/vault}"
LIMIT="${2:-20}"
STATE_DIR="${VAULT_LAYER_STATE_DIR:-$HOME/.local/share/vault-layer}"
BACKEND="${VAULT_LAYER_BACKEND:-sqlite}"
QUERY="${VAULT_LAYER_SMOKE_QUERY:-Context}"

if [[ ! -d "$VAULT_PATH" ]]; then
  echo "vault_missing=$VAULT_PATH" >&2
  exit 2
fi

mkdir -p "$STATE_DIR"
LOG=$(mktemp)
TIME=$(mktemp)
trap 'rm -f "$LOG" "$TIME"' EXIT

/usr/bin/time -f 'elapsed=%E maxrss=%MKB' -o "$TIME" \
  env VAULT_LAYER_BACKEND="$BACKEND" cargo run -p vault-layer -- index "$VAULT_PATH" --state-dir "$STATE_DIR" --limit "$LIMIT" >"$LOG"
cat "$LOG"
cat "$TIME"

case "$BACKEND" in
  duckdb|duckdb-local)
    DB_GLOB='vault-layer.duckdb'
    ;;
  libsql|libsql-local|turso-local)
    DB_GLOB='vault-layer.libsql'
    ;;
  *)
    DB_GLOB='vault-layer.db'
    ;;
esac

DB_PATH=$(find "$STATE_DIR" -maxdepth 2 -name "$DB_GLOB" -type f -printf '%T@ %p\n' | sort -nr | head -1 | cut -d' ' -f2-)

if [[ -z "$DB_PATH" ]]; then
  echo "db_missing_under=$STATE_DIR pattern=$DB_GLOB" >&2
  exit 3
fi

case "$DB_PATH" in
  "$PWD"/*|"$VAULT_PATH"/*)
    echo "unsafe_db_path=$DB_PATH" >&2
    exit 4
    ;;
esac

if [[ "$DB_PATH" == *.db ]]; then
  NOTES=$(sqlite3 "$DB_PATH" 'select count(*) from notes;')
  SECTIONS=$(sqlite3 "$DB_PATH" 'select count(*) from sections;')
  EMBEDDINGS_BEFORE=$(sqlite3 "$DB_PATH" 'select count(*) from embeddings;')
else
  NOTES="n/a"
  SECTIONS="n/a"
  EMBEDDINGS_BEFORE="n/a"
fi

SEARCH_JSON=$(cargo run -q -p vault-layer -- search "$QUERY" --db "$DB_PATH" --limit 1)
if [[ "$DB_PATH" == *.db ]]; then
  EMBED_JSON=$(cargo run -q -p vault-layer -- embed --db "$DB_PATH")
  VECTOR_JSON=$(cargo run -q -p vault-layer -- vector-search "$QUERY" --db "$DB_PATH" --limit 1)
  EMBEDDINGS_AFTER=$(sqlite3 "$DB_PATH" 'select count(*) from embeddings;')
else
  EMBED_JSON='{"skipped":"non-sqlite backend"}'
  VECTOR_JSON='[]'
  EMBEDDINGS_AFTER="$EMBEDDINGS_BEFORE"
fi

DB_SIZE_BYTES=$(stat -c '%s' "$DB_PATH")
REPO_DB_FILES=$({ git ls-files | grep -E '\.(db|sqlite|sqlite3|libsql|duckdb|turso|parquet|arrow)$' || true; } | wc -l)

echo "vault_path=$VAULT_PATH"
echo "state_dir=$STATE_DIR"
echo "backend=$BACKEND"
echo "db_path=$DB_PATH"
echo "db_size_bytes=$DB_SIZE_BYTES"
echo "notes_indexed=$NOTES"
echo "sections_indexed=$SECTIONS"
echo "embeddings_before=$EMBEDDINGS_BEFORE"
echo "embeddings_after=$EMBEDDINGS_AFTER"
echo "sample_search=$SEARCH_JSON"
echo "sample_embed=$EMBED_JSON"
echo "sample_vector=$VECTOR_JSON"
echo "repo_db_files=$REPO_DB_FILES"
