#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package_pilot.sh [--out-dir PATH]

Build a local VaultLayer pilot package under target/pilot-package by default.
The package contains the release binary, helper scripts, and pilot docs. It does
not vendor private vault data, generated DBs, model caches, or Python packages.
USAGE
}

out_dir="target/pilot-package"
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --out-dir)
      out_dir="${2:?--out-dir requires a path}"
      shift 2
      ;;
    --out-dir=*)
      out_dir="${1#--out-dir=}"
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

cargo build --release -p vault-layer
python3 - <<'PY'
try:
    import fastembed  # noqa: F401
except Exception as error:
    raise SystemExit(
        "Python fastembed is missing. Install with: python3 -m pip install fastembed==0.7.3\n"
        f"Import error: {error}"
    )
PY

rm -rf "$out_dir"
mkdir -p "$out_dir/bin" "$out_dir/scripts" "$out_dir/docs"
cp target/release/vault-layer "$out_dir/bin/vault-layer"
cp scripts/fastembed_embed.py scripts/pilot_doctor.sh scripts/benchmark_vault.sh "$out_dir/scripts/"
cp README.md LICENSE CHANGELOG.md "$out_dir/"
cp docs/PACKAGE.md docs/claim-evidence-gate.md docs/niels-pilot-install.md docs/niels-pilot-benchmark.md docs/local-embedding-adapter.md docs/full-vault-progress-resume.md "$out_dir/docs/"

cat >"$out_dir/README-PILOT.md" <<'README'
# VaultLayer pilot package

This package is a local pilot artifact. It is not a full production installer.

## Prerequisite

Install Python FastEmbed in the Python environment you will use for the pilot:

```bash
python3 -m pip install fastembed==0.7.3
```

## Doctor

```bash
VAULT_LAYER_FASTEMBED_HELPER="$PWD/scripts/fastembed_embed.py" \
  ./bin/vault-layer doctor /path/to/vault --state-dir ~/.local/share/vault-layer-pilot
```

## Benchmark

From a cloned repo, prefer `scripts/benchmark_vault.sh`. From this package, use
the binary plus docs to run the same index/search/embed/vector commands manually.

## Cleanup

Remove the state dir selected for the pilot. Remove the shared FastEmbed model
cache only if no other VaultLayer pilot uses it:

```bash
rm -rf ~/.local/share/vault-layer-pilot
rm -rf ~/.local/share/vault-layer/models/fastembed
```
README

VAULT_LAYER_FASTEMBED_HELPER="$out_dir/scripts/fastembed_embed.py" "$out_dir/bin/vault-layer" --help >/dev/null

tarball="${out_dir%/}.tar.gz"
tar -C "$(dirname "$out_dir")" -czf "$tarball" "$(basename "$out_dir")"
sha256sum "$tarball" > "$tarball.sha256"

printf 'package_dir=%s\n' "$out_dir"
printf 'package_tarball=%s\n' "$tarball"
printf 'package_sha256=%s\n' "$tarball.sha256"
