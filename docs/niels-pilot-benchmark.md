# Niels pilot benchmark

Use this after `scripts/pilot_doctor.sh` passes. The benchmark turns the pilot claim into measured evidence for a specific vault.

```bash
scripts/benchmark_vault.sh /path/to/niels-vault \
  --state-dir ~/.local/share/vault-layer-niels-benchmark \
  --query "agent context" \
  --limit 5000
```

The script is read-only for the source vault. It writes runtime artifacts under the selected state directory and uses the configured FastEmbed cache outside the vault.

## What it measures

- Markdown file count in the source vault.
- Baseline filesystem scan/grep-style query count and elapsed milliseconds.
- VaultLayer index time, note/section/FTS counts, DB path, and DB size.
- VaultLayer FTS search latency and a provenance-preserving JSON sample file.
- FastEmbed embedding time and model/dimension counts.
- Vector-search latency and a provenance-preserving JSON sample file.

## Report output

The report is written to:

```text
<state-dir>/benchmark-report.md
```

Sample fields:

```text
baseline_elapsed_ms=1234
index_time=elapsed=0:12.16 maxrss_kb=9164
search_time=elapsed=0:00.04 maxrss_kb=12000
embed_time=elapsed=2:33.73 maxrss_kb=1963516
vector_time=elapsed=0:02.26 maxrss_kb=221908
```

## Privacy boundary

The report stores counts, timings, runtime paths, and pointers to bounded VaultLayer JSON samples. It does not commit source vault files, generated DBs, embeddings, or model caches.

The JSON sample files may contain bounded retrieval excerpts because that is the product output being evaluated. Do not commit those generated state-dir files.

## Success criteria for the pilot

A useful Niels pilot should answer:

1. Does the doctor pass without remote tokens and without runtime state inside the vault?
2. How long does baseline filesystem scan/search take for the chosen query?
3. How long do VaultLayer FTS and vector searches take after indexing?
4. Do returned samples include provenance fields: path, chunk id, heading/excerpt, content hash, modified timestamp?
5. Is the index/embed cost acceptable for the expected update cadence?
