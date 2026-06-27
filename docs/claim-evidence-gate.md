# Claim evidence gate

This gate defines when VaultLayer is allowed to make a strong Niels-facing claim about being self-contained and faster on his vault.

## Unsafe source claim

Do **not** say this as a default repo/product claim:

> Dit is al een bewezen self-contained product dat gegarandeerd performance winst geeft op jouw vault.

It is too broad because installability and performance depend on the target machine, filesystem, vault shape, first-run model download, and query workload.

## Claim states

### Blocked claim

Use this until target-vault evidence exists:

> VaultLayer is een pilot-ready lokale MVP. Hij kan read-only over je vault draaien, bouwt state/cache buiten je vault, en heeft een benchmarkpad om performancewinst op jouw vault te meten.

Allowed when:

- package/doctor has not yet been run on the target machine; or
- Niels' real vault path, machine context, or representative queries are missing; or
- benchmark evidence is partial, failed, or inconclusive.

### Narrowed validated claim

Use this only after the gate passes on a named machine/vault/query set:

> In de gemeten Niels-pilot op `<machine/filesystem>`, met `<vault-size>` en `<queries>`, was VaultLayer read-only te installeren en leverde het voor deze queries een meetbare retrieval-winst tegenover de baseline. De winst geldt voor deze gemeten setup, niet als algemene garantie.

Allowed when all required self-contained and performance evidence below is recorded.

### Strong product claim

A broad statement such as “bewezen self-contained product” is allowed only after repeated target installs/runs prove the same result across more than one environment and the non-vendored runtime dependency story is resolved or explicitly accepted.

A broad statement such as “gegarandeerd performance winst op jouw vault” remains disallowed. Performance can be measured and reported; it should not be guaranteed outside the measured workload.

## Required self-contained evidence

Before saying the product/repo is self-contained enough for a pilot, record:

- `target/pilot-package.tar.gz` created by `scripts/package_pilot.sh`;
- matching `target/pilot-package.tar.gz.sha256`;
- package unpacked outside the source repository;
- unpacked `bin/vault-layer --help` works;
- doctor runs from the unpacked package with `VAULT_LAYER_FASTEMBED_HELPER` pointing at the unpacked helper;
- `doctor_status=ok`;
- runtime state path is outside the source vault and outside the repo;
- FastEmbed model cache path is outside the source vault and outside the repo;
- remote sync variables are absent or explicitly disabled for the pilot;
- package excludes private vault data, generated DB/index files, embeddings, caches, and raw benchmark JSON;
- cleanup commands remove state/cache without touching the source vault.

Boundary: Python `fastembed` is currently a non-vendored runtime dependency. The package can be pilot-self-contained only if that is stated clearly; it is not a single binary/offline installer unless the dependency and model cache are already present.

## Required performance evidence

Before saying VaultLayer produced a performance win on Niels' vault, record:

- real vault path/location class, without committing private path details if sensitive;
- machine/OS/filesystem context;
- vault size: Markdown file count, indexed note count, section count, DB size;
- 2–3 representative Niels queries;
- baseline filesystem/search timing per query;
- VaultLayer index timing;
- VaultLayer FTS/search timing per query;
- FastEmbed embed timing, separated from search timing;
- vector search timing per query;
- first-run model download/setup cost separated from steady-state runs;
- provenance samples for search and vector results: path, heading/chunk id, content hash, modified timestamp, bounded excerpt;
- cleanup proof that generated state/cache can be removed without touching the vault.

## Thresholds

A target-vault performance claim passes only when:

- doctor succeeds without unsafe remote sync;
- at least 2 representative queries complete successfully;
- steady-state VaultLayer search or vector retrieval is at least **2× faster** than the relevant baseline for the same query, or saves at least **1 second** on a query where baseline is slow enough that the difference is user-visible;
- returned results preserve provenance and are useful enough for the intended workflow;
- index/embed setup cost is reported separately and is acceptable for the use case;
- no private vault content or generated runtime artifacts are committed.

A partial pass can say “measured pilot result” but must list which threshold failed or remained untested.

## Kill criteria

Stop or narrow the claim when any are true:

- state/cache is inside the source vault;
- source vault is modified;
- remote sync/upload is configured unintentionally;
- package only works from inside the repository or with undocumented local paths;
- benchmark cannot finish with progress visibility;
- no material latency or usefulness gain appears for representative queries;
- first-run setup/model download cost dominates the claimed benefit and is not acceptable;
- evidence requires committing private vault content, raw samples, DBs, embeddings, caches, or benchmark JSON.

## Drift check

Docs and public copy must not contain unscoped guarantee language unless a target-vault evidence report explicitly passes this gate.

Use grep-style checks for at least:

```bash
! grep -R "gegarandeerd performance winst\|guaranteed performance win" README.md docs --exclude=claim-evidence-gate.md
! grep -R "bewezen self-contained product" README.md docs --exclude=claim-evidence-gate.md
```

If those phrases appear, they must be quoted as disallowed wording or placed inside this gate with explicit caveats.
