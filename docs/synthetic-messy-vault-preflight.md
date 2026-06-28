# Synthetic messy vault preflight

Date: 2026-06-28  
Task: `20260628-131628-vault-layer-add-and-run-a-synthetic-messy-vault-preflight-before-the`

## Purpose

This preflight tests VaultLayer on a deterministic fake messy Obsidian-style vault before running against Niels' private real vault.

It increases confidence in edge-case handling, package portability, doctor checks, and benchmark plumbing. It does **not** prove performance on Niels' real vault.

## Fixture

Generated with:

```bash
python3 scripts/make_messy_vault.py /tmp/vault-layer-messy-vault-preflight --force
```

Output:

```text
messy_vault=/tmp/vault-layer-messy-vault-preflight
markdown_notes=10
features=nested,weird-filenames,unicode,duplicates,empty,frontmatter,wikilinks,tags,long,non-md,hidden-runtime-dirs
```

Fixture coverage:

- nested folders;
- filenames with spaces and brackets;
- unicode filename/title and emoji content;
- duplicate filenames in different folders;
- empty note;
- YAML frontmatter;
- WikiLinks and tags;
- repeated headings;
- long-ish note body;
- non-Markdown files;
- hidden/runtime-ish directories such as `.obsidian`, `.trash`, and `node_modules`.

The generated fixture lives under `/tmp` and is not committed.

## Package smoke

Command:

```bash
scripts/package_smoke.sh /tmp/vault-layer-messy-vault-preflight \
  --work-dir /tmp/vault-layer-messy-package-smoke
```

Key output:

```text
target/pilot-package.tar.gz: OK
runtime_state_outside_vault=true
runtime_state_outside_repo=true
sqlite_vec_available=true
fastembed_available=true
fastembed_model=fastembed:sentence-transformers/all-MiniLM-L6-v2
fastembed_dimensions=384
doctor_status=ok
package_smoke=ok
package_dir=/tmp/vault-layer-messy-package-smoke/unpack/pilot-package
state_dir=/tmp/vault-layer-messy-package-smoke/state
cache_dir=/tmp/vault-layer-messy-package-smoke/cache
```

Result: pass.

## Benchmark smoke

Command:

```bash
scripts/benchmark_vault.sh /tmp/vault-layer-messy-vault-preflight \
  --state-dir /tmp/vault-layer-messy-benchmark \
  --query "performance baseline vector provenance" \
  --limit 500
```

Report path:

```text
/tmp/vault-layer-messy-benchmark/benchmark-report.md
```

Key output:

```text
runtime_outside_vault=true
markdown_files=11
baseline_query_matches=0
baseline_elapsed_ms=2
notes_indexed=10
sections=13
fts_rows=13
db_size_bytes=102400
search_time=elapsed=0:00.01 maxrss_kb=6400
embed_time=elapsed=0:02.43 maxrss_kb=249012
embeddings_after=13
embedding_models=fastembed:sentence-transformers/all-MiniLM-L6-v2:384:13
vector_time=elapsed=0:01.95 maxrss_kb=221488
```

The baseline `markdown_files=11` includes a deliberately generated `node_modules/.../readme.md`. VaultLayer indexed `notes_indexed=10` because scanner rules ignore hidden/runtime/dependency-style directories. That mismatch is expected for this fixture and confirms the ignore boundary.

Result: pass.

## Artifact boundary

Generated artifacts stayed outside the repository:

- fake vault: `/tmp/vault-layer-messy-vault-preflight`;
- package smoke state/cache: `/tmp/vault-layer-messy-package-smoke`;
- benchmark state/report/json/db: `/tmp/vault-layer-messy-benchmark`;
- package tarball under ignored `target/`.

No fake vault content, DB, sqlite/libsql/duckdb/parquet/arrow artifacts, raw benchmark JSON, model cache, or package tarball is committed.

## Claim impact

Allowed after this preflight:

> VaultLayer has passed a synthetic messy-vault preflight for package portability, doctor checks, indexing, embeddings, vector search, and benchmark plumbing.

Still not allowed:

> VaultLayer is guaranteed faster on Niels' vault.

The Niels target-vault benchmark remains blocked until the real vault path/access, machine context, and representative queries are available.
