#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/pilot_doctor.sh /path/to/obsidian-vault [--state-dir PATH]

Builds the VaultLayer release binary and runs a local read-only pilot doctor.
It never writes to the source vault. Runtime state and FastEmbed model cache
must live outside both the repository and the vault.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || $# -lt 1 ]]; then
  usage
  exit 0
fi

vault_path="$1"
shift
state_dir="${VAULT_LAYER_STATE_DIR:-$HOME/.local/share/vault-layer}"
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
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"

case "$(python3 - <<'PY' "$state_dir" "$vault_path"
from pathlib import Path
import sys
state = Path(sys.argv[1]).expanduser().resolve()
vault = Path(sys.argv[2]).expanduser().resolve()
print('inside' if state == vault or vault in state.parents else 'outside')
PY
)" in
  inside)
    echo "state dir must be outside the vault: $state_dir" >&2
    exit 1
    ;;
esac

python3 - <<'PY'
try:
    import fastembed  # noqa: F401
except Exception as error:
    raise SystemExit(
        "Python fastembed is missing. Install locally with: python3 -m pip install fastembed==0.7.3\n"
        f"Import error: {error}"
    )
PY

cargo build --release -p vault-layer
VAULT_LAYER_FASTEMBED_HELPER="${VAULT_LAYER_FASTEMBED_HELPER:-$repo_root/scripts/fastembed_embed.py}" \
  ./target/release/vault-layer doctor "$vault_path" --state-dir "$state_dir"
