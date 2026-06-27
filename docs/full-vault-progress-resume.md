# Full-vault progress and resume hardening

Date: 2026-06-27  
Task: `20260625-120500-vault-layer-full-index-progress-resume`

## Summary

VaultLayer index runs now emit progress during scan and write phases so long vault runs are no longer opaque foreground commands.

Progress is written to stderr as stable key/value lines:

```text
vault-layer progress phase=scan_files_found count=500
vault-layer progress phase=scan_notes_parsed count=500
vault-layer progress phase=write_notes count=500
```

For small runs, the first five counts are emitted. For larger runs, progress is emitted every 500 items.

## Existing-index skip

The SQLite index path can now avoid redoing the DB write when an existing DB already has the same note count as the requested scan and `--force` is not passed:

```text
vault-layer progress phase=write_skipped_existing count=<notes>
```

Use `--force` to rebuild the DB anyway:

```bash
vault-layer index /path/to/vault --state-dir ~/.local/share/vault-layer --force
```

This is intentionally conservative. It avoids repeated writes for unchanged-count smoke/pilot runs, but it is not yet a full content-hash incremental indexer.

## Verification evidence

Synthetic/test-vault smoke:

```bash
cargo run -q -p vault-layer -- index /mnt/c/Users/viggo/github/obsidian-test-vault \
  --state-dir /tmp/vault-layer-progress-smoke --limit 20
```

Observed progress included:

```text
vault-layer progress phase=scan_files_found count=1
vault-layer progress phase=scan_notes_parsed count=1
vault-layer progress phase=write_notes count=1
```

Second run over the same state dir emitted:

```text
vault-layer progress phase=write_skipped_existing count=13
```

## Remaining hardening

This closes the “silent 21-minute foreground command” failure mode enough for pilot diagnosis. A future production-grade indexer should still add content-hash-level incremental updates, resumable partial DB transactions, and more detailed per-directory timing.
