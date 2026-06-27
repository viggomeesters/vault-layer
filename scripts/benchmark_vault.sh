#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/benchmark_vault.sh /path/to/obsidian-vault [--state-dir PATH] [--query TEXT] [--limit N]

Runs a read-only VaultLayer pilot benchmark:
- baseline filesystem scan/grep-style query count;
- VaultLayer index timing and DB size;
- FTS search timing with provenance sample;
- FastEmbed embedding timing;
- vector-search timing with provenance sample.

The source vault is not modified. Runtime DB/cache/report files are written under
--state-dir and the configured FastEmbed cache, both expected outside the vault.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || $# -lt 1 ]]; then
  usage
  exit 0
fi

vault_path="$1"
shift
state_dir="${VAULT_LAYER_STATE_DIR:-$HOME/.local/share/vault-layer-benchmark}"
query="${VAULT_LAYER_BENCHMARK_QUERY:-agent context}"
limit="${VAULT_LAYER_BENCHMARK_LIMIT:-5000}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --state-dir)
      state_dir="${2:?--state-dir requires a path}"
      shift 2
      ;;
    --state-dir=*)
      state_dir="${1#--state-dir=}"
      shift
      ;;
    --query)
      query="${2:?--query requires text}"
      shift 2
      ;;
    --query=*)
      query="${1#--query=}"
      shift
      ;;
    --limit)
      limit="${2:?--limit requires a number}"
      shift 2
      ;;
    --limit=*)
      limit="${1#--limit=}"
      shift
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"
mkdir -p "$state_dir"
report="$state_dir/benchmark-report.md"
index_log="$state_dir/index.out"
search_json="$state_dir/search.json"
vector_json="$state_dir/vector-search.json"
embed_log="$state_dir/embed.out"

time_cmd=(/usr/bin/time -f 'elapsed=%E maxrss_kb=%M')
if [[ ! -x /usr/bin/time ]]; then
  time_cmd=(time)
fi

python3 - <<'PY' "$state_dir" "$vault_path"
from pathlib import Path
import sys
state = Path(sys.argv[1]).expanduser().resolve()
vault = Path(sys.argv[2]).expanduser().resolve()
if state == vault or vault in state.parents:
    raise SystemExit(f"state dir must be outside the vault: {state}")
PY

cargo build --release -p vault-layer >/dev/null
bin="./target/release/vault-layer"
helper="$repo_root/scripts/fastembed_embed.py"

started="$(date --iso-8601=seconds)"
read -r md_files baseline_matches baseline_elapsed_ms < <(python3 - <<'PY' "$vault_path" "$query"
from pathlib import Path
import sys, time
vault = Path(sys.argv[1])
terms = [term.casefold() for term in sys.argv[2].split() if term]
start = time.perf_counter()
files = 0
matches = 0
for path in vault.rglob('*.md'):
    if any(part.startswith('.') for part in path.relative_to(vault).parts):
        continue
    files += 1
    try:
        text = path.read_text(encoding='utf-8', errors='ignore').casefold()
    except OSError:
        continue
    if all(term in text for term in terms):
        matches += 1
elapsed_ms = int((time.perf_counter() - start) * 1000)
print(files, matches, elapsed_ms)
PY
)

"${time_cmd[@]}" "$bin" index "$vault_path" --state-dir "$state_dir" --limit "$limit" >"$index_log" 2>"$state_dir/index.time"
db_path="$(awk -F= '/^db_path=/{print $2}' "$index_log" | tail -1)"
if [[ -z "$db_path" || ! -f "$db_path" ]]; then
  echo "index did not produce db_path" >&2
  cat "$index_log" >&2
  exit 1
fi

db_size_bytes="$(stat -c '%s' "$db_path")"
read -r notes sections fts embeddings_before < <(python3 - <<'PY' "$db_path"
import sqlite3, sys
conn = sqlite3.connect(sys.argv[1])
def scalar(sql):
    return conn.execute(sql).fetchone()[0]
print(scalar('select count(*) from notes'), scalar('select count(*) from sections'), scalar('select count(*) from sections_fts'), scalar('select count(*) from embeddings'))
PY
)

"${time_cmd[@]}" "$bin" search "$query" --db "$db_path" --json >"$search_json" 2>"$state_dir/search.time"
VAULT_LAYER_FASTEMBED_HELPER="${VAULT_LAYER_FASTEMBED_HELPER:-$helper}" "${time_cmd[@]}" "$bin" embed --db "$db_path" --model fastembed-mini-lm >"$embed_log" 2>"$state_dir/embed.time"
read -r embeddings_after model_dims < <(python3 - <<'PY' "$db_path"
import sqlite3, sys
conn = sqlite3.connect(sys.argv[1])
rows = conn.execute("select model, dimensions, count(*) c from embeddings group by model, dimensions order by model").fetchall()
total = sum(row[2] for row in rows)
models = ';'.join(f"{model}:{dimensions}:{count}" for model, dimensions, count in rows)
print(total, models)
PY
)
VAULT_LAYER_FASTEMBED_HELPER="${VAULT_LAYER_FASTEMBED_HELPER:-$helper}" "${time_cmd[@]}" "$bin" vector-search "$query" --db "$db_path" --model fastembed-mini-lm --json >"$vector_json" 2>"$state_dir/vector.time"

cat >"$report" <<REPORT
# VaultLayer pilot benchmark

started=$started
vault_path=$vault_path
state_dir=$state_dir
query=$query
limit=$limit
runtime_outside_vault=true

## Baseline filesystem scan

markdown_files=$md_files
baseline_query_matches=$baseline_matches
baseline_elapsed_ms=$baseline_elapsed_ms

## VaultLayer index

$(cat "$index_log")
index_time=$(tr '\n' ' ' < "$state_dir/index.time")
db_size_bytes=$db_size_bytes
notes=$notes
sections=$sections
fts_rows=$fts
embeddings_before=$embeddings_before

## VaultLayer FTS search

search_time=$(tr '\n' ' ' < "$state_dir/search.time")
search_sample_file=$search_json

## VaultLayer FastEmbed

embed_time=$(tr '\n' ' ' < "$state_dir/embed.time")
embeddings_after=$embeddings_after
embedding_models=$model_dims

## VaultLayer vector search

vector_time=$(tr '\n' ' ' < "$state_dir/vector.time")
vector_sample_file=$vector_json

## Privacy boundary

The benchmark report stores counts, timings, DB/cache paths, and pointers to bounded VaultLayer JSON samples. It does not copy source vault files into the repo. Runtime artifacts live under the benchmark state dir.
REPORT

cat "$report"
