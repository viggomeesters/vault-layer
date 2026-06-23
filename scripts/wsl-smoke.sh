#!/usr/bin/env bash
set -euo pipefail

VAULT_PATH="${1:-/mnt/c/Users/Viggo/Syncthing/vault}"
LIMIT="${2:-20}"
STATE_DIR="${VAULT_LAYER_STATE_DIR:-$HOME/.local/share/vault-layer}"

if [[ ! -d "$VAULT_PATH" ]]; then
  echo "vault_missing=$VAULT_PATH" >&2
  exit 2
fi

cargo run -p vault-layer -- index "$VAULT_PATH" --state-dir "$STATE_DIR" --limit "$LIMIT"
DB_PATH=$(find "$STATE_DIR" -maxdepth 2 -name vault-layer.db -type f -printf '%T@ %p
' | sort -nr | head -1 | cut -d' ' -f2-)

if [[ -z "$DB_PATH" ]]; then
  echo "db_missing_under=$STATE_DIR" >&2
  exit 3
fi

case "$DB_PATH" in
  "$PWD"/*|"$VAULT_PATH"/*)
    echo "unsafe_db_path=$DB_PATH" >&2
    exit 4
    ;;
esac

NOTES=$(sqlite3 "$DB_PATH" 'select count(*) from notes;')
CHUNKS=$(sqlite3 "$DB_PATH" 'select count(*) from sections;')

echo "vault_path=$VAULT_PATH"
echo "state_dir=$STATE_DIR"
echo "db_path=$DB_PATH"
echo "notes_indexed=$NOTES"
echo "chunks_indexed=$CHUNKS"
echo "repo_db_files=$(git ls-files | grep -E '\.(db|sqlite|sqlite3|libsql|turso)$' | wc -l)"
