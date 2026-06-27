# Niels pilot runbook

This runbook is the safe path for testing VaultLayer over an existing Niels-style Obsidian/Markdown vault.

## Positioning

VaultLayer is currently a **pilot-ready local MVP**, not a fully self-contained production product.

What is proven:

- source vault is treated read-only;
- runtime DB, benchmark reports, and model cache live outside the source vault and repo;
- local SQLite/FTS retrieval works;
- sqlite-vec native smoke and selected-model vector refresh/search work;
- real local FastEmbed MiniLM embeddings work through optional Python `fastembed`;
- bounded 5000-note evidence exists.

What still needs target-vault proof:

- actual performance win on Niels' vault;
- acceptable first-run model download/setup time;
- full-vault behavior on Niels' filesystem and machine;
- whether returned samples are useful enough for his workflow.

## 1. Clone and check

```bash
git clone https://github.com/viggomeesters/vault-layer.git
cd vault-layer
make check
python3 -m pip install fastembed==0.7.3
```

## 2. Doctor

```bash
scripts/pilot_doctor.sh /path/to/niels-vault \
  --state-dir ~/.local/share/vault-layer-niels
```

Stop if doctor does not end with:

```text
doctor_status=ok
```

## 3. Benchmark

Pick one query Niels actually cares about. Then run:

```bash
scripts/benchmark_vault.sh /path/to/niels-vault \
  --state-dir ~/.local/share/vault-layer-niels-benchmark \
  --query "<real query>" \
  --limit 5000
```

The report is written to:

```text
~/.local/share/vault-layer-niels-benchmark/benchmark-report.md
```

## 4. Evidence to collect

Record these fields from the report:

- `markdown_files`
- `baseline_elapsed_ms`
- `notes_indexed`
- `sections`
- `index_time`
- `db_size_bytes`
- `search_time`
- `embed_time`
- `embeddings_after`
- `vector_time`
- `search_sample_file`
- `vector_sample_file`

Check that search/vector JSON samples preserve provenance:

- `chunk_id`
- `path`
- `heading_path`
- `excerpt`
- `content_hash`
- `modified_unix`
- `human_relevance_score`

## 5. Go / no-go criteria

Go when all are true:

- doctor passes without remote tokens;
- runtime state and model cache are outside the vault and repo;
- benchmark finishes without generated artifacts in Git;
- VaultLayer search/vector latency is materially better than repeated raw filesystem scanning for the chosen query;
- returned samples have useful provenance and no unacceptable private-data handling issue.

No-go / stop when any are true:

- doctor reports remote backend configured unintentionally;
- state/cache path is inside the source vault;
- index run is opaque or stalls without progress;
- benchmark report or JSON samples are accidentally staged/committed;
- performance is not better enough to justify index/embed cost for the intended workflow.

## 6. Cleanup

Remove benchmark/index runtime data:

```bash
rm -rf ~/.local/share/vault-layer-niels
rm -rf ~/.local/share/vault-layer-niels-benchmark
```

Remove the shared local model cache only if no other VaultLayer pilot uses it:

```bash
rm -rf ~/.local/share/vault-layer/models/fastembed
```
