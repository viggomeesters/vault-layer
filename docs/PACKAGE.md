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

## Runtime state

Do not install or run VaultLayer in a way that writes indexes into the repository. Use `VAULT_LAYER_STATE_DIR` or the default user state directory.
