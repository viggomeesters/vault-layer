#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package_smoke.sh /path/to/test-vault [--work-dir PATH]

Builds the pilot package, unpacks the tarball outside the repository, and proves
that the unpacked artifact can run --help and doctor with an explicit FastEmbed
helper path. The smoke also fails if private/runtime artifacts are bundled.
USAGE
}

if [[ $# -lt 1 ]]; then
  usage >&2
  exit 2
fi

vault_path="$1"
shift
work_dir="/tmp/vault-layer-package-smoke"
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --work-dir)
      work_dir="${2:?--work-dir requires a path}"
      shift 2
      ;;
    --work-dir=*)
      work_dir="${1#--work-dir=}"
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

if [[ ! -d "$vault_path" ]]; then
  echo "test vault does not exist: $vault_path" >&2
  exit 1
fi

rm -rf target/pilot-package target/pilot-package.tar.gz target/pilot-package.tar.gz.sha256
scripts/package_pilot.sh >/tmp/vault-layer-package-build.out

tarball="$repo_root/target/pilot-package.tar.gz"
sha_file="$repo_root/target/pilot-package.tar.gz.sha256"
test -f "$tarball"
test -f "$sha_file"
sha256sum -c "$sha_file"

rm -rf "$work_dir"
mkdir -p "$work_dir/unpack" "$work_dir/state" "$work_dir/cache"
tar -C "$work_dir/unpack" -xzf "$tarball"
package_dir="$work_dir/unpack/pilot-package"

# Prove the artifact is usable from outside the repo.
case "$package_dir" in
  "$repo_root"/*)
    echo "package unpacked inside repo unexpectedly: $package_dir" >&2
    exit 1
    ;;
esac

test -x "$package_dir/bin/vault-layer"
test -f "$package_dir/scripts/fastembed_embed.py"
VAULT_LAYER_FASTEMBED_HELPER="$package_dir/scripts/fastembed_embed.py" \
  "$package_dir/bin/vault-layer" --help >/dev/null

VAULT_LAYER_FASTEMBED_HELPER="$package_dir/scripts/fastembed_embed.py" \
VAULT_LAYER_FASTEMBED_CACHE_DIR="$work_dir/cache" \
  "$package_dir/bin/vault-layer" doctor "$vault_path" --state-dir "$work_dir/state" \
  | tee "$work_dir/doctor.out"
grep -q 'doctor_status=ok' "$work_dir/doctor.out"

# Bundled artifact must not contain source vault data, generated DBs, embeddings,
# model caches, or raw benchmark JSON.
if find "$package_dir" -type f \
  \( -name '*.db' -o -name '*.sqlite' -o -name '*.sqlite3' -o -name '*.libsql' -o -name '*.duckdb' -o -name '*.parquet' -o -name '*.arrow' -o -name 'search.json' -o -name 'vector-search.json' -o -path '*/models/fastembed/*' \) \
  | grep -q .; then
  echo "package contains forbidden runtime/private artifacts" >&2
  find "$package_dir" -type f \
    \( -name '*.db' -o -name '*.sqlite' -o -name '*.sqlite3' -o -name '*.libsql' -o -name '*.duckdb' -o -name '*.parquet' -o -name '*.arrow' -o -name 'search.json' -o -name 'vector-search.json' -o -path '*/models/fastembed/*' \) >&2
  exit 1
fi

printf 'package_smoke=ok\n'
printf 'package_dir=%s\n' "$package_dir"
printf 'doctor_output=%s\n' "$work_dir/doctor.out"
printf 'state_dir=%s\n' "$work_dir/state"
printf 'cache_dir=%s\n' "$work_dir/cache"
