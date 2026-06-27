#!/usr/bin/env python3
"""Generate local FastEmbed vectors for VaultLayer.

Input is a JSON file with:
  {"texts": ["..."], "kind": "passage"|"query", "cache_dir": "..."}

Output is JSON Lines:
  {"model": "fastembed:sentence-transformers/all-MiniLM-L6-v2", "dimensions": 384, "cache_dir": "..."}
  [0.1, ...]
  [0.2, ...]

The script never calls a SaaS embedding API. FastEmbed downloads the ONNX model on
first use into the explicit cache directory, then runs locally/offline from that
cache.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

MODEL_NAME = "sentence-transformers/all-MiniLM-L6-v2"
PUBLIC_MODEL_ID = f"fastembed:{MODEL_NAME}"
EXPECTED_DIMENSIONS = 384


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: fastembed_embed.py <input-json>")

    input_path = Path(sys.argv[1])
    try:
        payload = json.loads(input_path.read_text(encoding="utf-8"))
    except Exception as error:  # noqa: BLE001 - CLI boundary
        fail(f"read input json: {error}")

    texts = payload.get("texts")
    if not isinstance(texts, list) or not all(isinstance(text, str) for text in texts):
        fail("input json must contain texts: string[]")
    kind = payload.get("kind", "passage")
    if kind not in {"passage", "query"}:
        fail("kind must be passage or query")

    cache_dir = Path(payload.get("cache_dir") or os.environ.get("VAULT_LAYER_FASTEMBED_CACHE_DIR") or Path.home() / ".local/share/vault-layer/models/fastembed")
    cache_dir.mkdir(parents=True, exist_ok=True)

    try:
        from fastembed import TextEmbedding
    except Exception as error:  # noqa: BLE001 - optional runtime dependency
        fail(
            "fastembed Python package is not installed; install with "
            "`python3 -m pip install fastembed==0.7.3` or set VAULT_LAYER_FASTEMBED_PYTHON to an environment that has it. "
            f"Import error: {error}"
        )

    prefixed = [f"{kind}: {text}" for text in texts]
    model = TextEmbedding(model_name=MODEL_NAME, cache_dir=str(cache_dir))
    vectors = list(model.embed(prefixed))

    print(
        json.dumps(
            {
                "model": PUBLIC_MODEL_ID,
                "dimensions": EXPECTED_DIMENSIONS,
                "cache_dir": str(cache_dir),
                "texts": len(texts),
            },
            separators=(",", ":"),
        )
    )
    for vector in vectors:
        values = [float(value) for value in vector.tolist()]
        if len(values) != EXPECTED_DIMENSIONS:
            fail(f"unexpected embedding dimensions: {len(values)}")
        print(json.dumps(values, separators=(",", ":")))


if __name__ == "__main__":
    main()
