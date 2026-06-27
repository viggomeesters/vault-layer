# Package and Install

VaultLayer is currently a Rust workspace with a CLI package named `vault-layer`.

## Build

```bash
cargo build --release -p vault-layer
```

Binary:

```text
target/release/vault-layer
```

## Verify

```bash
make check
./target/release/vault-layer --help
```

## Pilot package

Build a local pilot artifact:

```bash
scripts/package_pilot.sh
```

Verify the package from a cloned repo:

```bash
scripts/package_smoke.sh /path/to/synthetic-or-test-vault
```

This builds the package, verifies the tarball checksum, unpacks it outside the repository, runs `bin/vault-layer --help`, runs `doctor` with `VAULT_LAYER_FASTEMBED_HELPER` pointing at the unpacked helper, and fails if DBs/caches/raw benchmark artifacts are bundled.

Default outputs:

```text
target/pilot-package/
target/pilot-package.tar.gz
target/pilot-package.tar.gz.sha256
```

The package contains:

- release binary: `bin/vault-layer`;
- local FastEmbed helper: `scripts/fastembed_embed.py`;
- pilot helper scripts and docs;
- README/LICENSE/CHANGELOG.

The package deliberately does **not** vendor private vault data, generated DBs, embeddings, model caches, or Python packages.

## Optional Python runtime

Real local embeddings require Python `fastembed` in the runtime environment:

```bash
python3 -m pip install fastembed==0.7.3
```

First use may download the ONNX model into:

```text
~/.local/share/vault-layer/models/fastembed/
```

A cached model path can be reused offline. Override with:

```bash
VAULT_LAYER_FASTEMBED_CACHE_DIR=/path/outside/repo-and-vault
VAULT_LAYER_FASTEMBED_PYTHON=/path/to/python-with-fastembed
VAULT_LAYER_FASTEMBED_HELPER=/path/to/fastembed_embed.py
```

## Runtime state

Do not install or run VaultLayer in a way that writes indexes into the repository or source vault. Use `VAULT_LAYER_STATE_DIR`, `--state-dir`, or the default user state directory.

## Cleanup / uninstall pilot data

Remove the chosen state dir:

```bash
rm -rf ~/.local/share/vault-layer-pilot
```

Remove the shared model cache only if no other VaultLayer pilot uses it:

```bash
rm -rf ~/.local/share/vault-layer/models/fastembed
```
