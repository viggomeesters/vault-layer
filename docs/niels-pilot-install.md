# Niels pilot install and doctor

VaultLayer can be piloted over an existing Obsidian/Markdown vault without writing to that vault. The pilot flow below checks prerequisites, builds the local CLI, and verifies that runtime state and model cache are outside both the repository and the source vault.

## Install prerequisites

```bash
git clone https://github.com/viggomeesters/vault-layer.git
cd vault-layer
python3 -m pip install fastembed==0.7.3
```

`fastembed` is used only for local ONNX inference. No SaaS embedding token or hosted vector service is required.

## Run doctor

```bash
scripts/pilot_doctor.sh /path/to/niels-vault --state-dir ~/.local/share/vault-layer-niels
```

The script runs:

1. Python `fastembed` import preflight.
2. `cargo build --release -p vault-layer`.
3. `target/release/vault-layer doctor /path/to/niels-vault --state-dir <state-dir>`.

The doctor checks:

- vault path exists and is readable as a directory;
- source vault remains read-only by convention;
- state directory is outside the source vault;
- state/cache are outside the repository;
- local backend is selected and remote Turso/libSQL sync is not accidentally configured;
- sqlite-vec native smoke works;
- Python FastEmbed can generate a 384-dimensional `fastembed-mini-lm` vector;
- model cache is writable and outside the source vault.

Expected success tail:

```text
sqlite_vec_available=true
fastembed_available=true
fastembed_dimensions=384
doctor_status=ok
```

## Safe failure examples

If remote sync variables are present, doctor fails instead of uploading private vault text:

```text
ERROR remote backend configured; unset TURSO_DATABASE_URL/TURSO_AUTH_TOKEN or set VAULT_LAYER_BACKEND=sqlite for a local pilot
doctor_status=failed
```

If the runtime directory is inside the vault, doctor fails before indexing:

```text
ERROR runtime_state_or_cache_inside_vault=true
doctor_status=failed
```

## Cleanup

The pilot creates runtime data only under the chosen state directory and the FastEmbed cache, by default:

```text
~/.local/share/vault-layer-niels
~/.local/share/vault-layer/models/fastembed
```

Remove pilot runtime data without touching the vault:

```bash
rm -rf ~/.local/share/vault-layer-niels
```

Remove the shared local model cache only if no other VaultLayer pilot uses it:

```bash
rm -rf ~/.local/share/vault-layer/models/fastembed
```
