# Retrieval quality first pass

Date: 2026-06-25  
Task: `20260625-191734-vault-layer-retrieval-quality-first-pass`

## Why

The 5000-note bounded run confirmed SQLite + FTS5/index performance, but the deterministic JSON cosine vector fallback ranked low-information chunks such as `Status / Bezorgd` too highly. That meant storage was fine, but retrieval quality needed a first hardening pass before larger scale work.

## Change

`vector-search` now computes and reports:

- `cosine_score`: raw deterministic embedding similarity;
- `text_quality_score`: heuristic quality multiplier;
- `score`: `cosine_score * text_quality_score`.

The text-quality multiplier demotes:

- very short/status-only chunks;
- duplicate-ish tiny chunks with very few unique words;
- Excalidraw boilerplate warnings;
- URL/bookmark-only-ish chunks;
- email header/address-list boilerplate.

This does **not** make semantic retrieval production-grade. It makes the portable fallback less misleading while native sqlite-vec and real embeddings are still separate follow-up work.

## Test-vault evidence

```text
vault=/mnt/c/Users/viggo/github/obsidian-test-vault
notes_indexed=13
chunks_embedded=15
vector rows include text_quality_score and cosine_score
repo runtime DB under /tmp
```

Top result after quality pass had `text_quality_score=1.0` and retained provenance:

```text
fixtures/minimal-hidden-files/readme.md
heading=Minimal Hidden Files Fixtures
fields=chunk_id,path,heading_path,excerpt,score,cosine_score,text_quality_score,content_hash,modified_unix,human_relevance_score
```

## 5000-note bounded evidence

Input DB from bounded real-vault run:

```text
vault=/mnt/c/Users/Viggo/Syncthing/vault
limit=5000
notes=5000
sections=8304
embeddings=8304
```

Before the quality pass, `vector-search Context` surfaced low-information/boilerplate results including `Status / Bezorgd`, Excalidraw warnings, bookmarks, and email headers.

After the quality pass, the top 5 no longer included `Status / Bezorgd` or Excalidraw warning chunks:

```text
1. 10_notes/2015-07/20150707-1200-mbo-scalda-bedrijfsadministrateur.md
   heading=MBO Diploma Bedrijfsadministrateur Niveau 4
   score=0.927173 cosine=0.927173 quality=1.000
2. 10_notes/2016-11/20161125-0558-uw-jysk-factuur-voor-ordernr-4001731996.md
   heading=Uw order is verzonden vanuit ons distributiecentrum in Denemarken...
   score=0.924127 cosine=0.924127 quality=1.000
3. 10_notes/2010-01/20100101-1200-00-eczeem.md
   heading=SATURDAY, 03 FEBRUARY 2024, 11:29:04
   score=0.921444 cosine=0.921444 quality=1.000
```

## Remaining limitation

This is still a heuristic. The next real retrieval-quality layer is hybrid ranking:

1. FTS candidate set;
2. real/local embedding model;
3. sqlite-vec table search;
4. rerank using FTS score + vector score + human relevance + chunk quality + duplicate/content-hash dampening.
