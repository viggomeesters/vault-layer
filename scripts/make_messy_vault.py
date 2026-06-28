#!/usr/bin/env python3
"""Generate a deterministic messy fake Obsidian-style vault for preflight testing."""
from __future__ import annotations

import argparse
import shutil
from pathlib import Path


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out_dir", help="Output directory for the fake vault")
    parser.add_argument("--force", action="store_true", help="Remove an existing output directory first")
    args = parser.parse_args()

    out = Path(args.out_dir).expanduser().resolve()
    if out.exists():
        if not args.force:
            raise SystemExit(f"output exists; pass --force to replace: {out}")
        shutil.rmtree(out)
    out.mkdir(parents=True)

    notes: dict[str, str] = {
        "00 Inbox/empty note.md": "",
        "00 Inbox/Quick Capture.md": "---\ntype: capture\ntags: [inbox, messy-test]\n---\n# Quick Capture\n\nLoose thought with [[Project Alpha]] and #inbox.\n",
        "Projects/Alpha/Project Alpha.md": "---\nstatus: active\nowner: Niels\n---\n# Project Alpha\n\n## Context\nAlpha links to [[Duplicate]] and [[Møøse Research]].\n\n## Tasks\n- [ ] Chase ambiguous requirement\n- [x] Preserve provenance\n",
        "Projects/Beta/Duplicate.md": "# Duplicate\n\nSame filename in a different folder. Mentions performance baseline and vector search.\n",
        "Archive/2024/Duplicate.md": "# Duplicate\n\nArchived duplicate filename. Should keep path provenance distinct.\n",
        "Research/Møøse Research.md": "# Møøse Research 🫎\n\nUnicode title, emoji, and Dutch text: efficiënt zoeken in rommelige kennis.\n\nTags: #research #unicode\n",
        "Research/spaces and [brackets].md": "# Spaces and [brackets]\n\nFilename has spaces and brackets. Link to [[Quick Capture]].\n",
        "Daily/2026-06-28.md": "# Daily\n\nA daily note with repeated headings.\n\n## Notes\nFirst heading.\n\n## Notes\nSecond heading with same text should still get unique section ids.\n",
        "Deep/Nested/Folder/Very Deep Note.md": "# Very Deep Note\n\nDeep nesting plus a long-ish section.\n\n" + "Lorem ipsum vault layer retrieval benchmark provenance. " * 80 + "\n",
        "Weird/control-ish.md": "# Control-ish\n\nContains tabs\tand pipes | and quotes ' \" but no secrets.\n",
    }
    for rel, text in notes.items():
        write(out / rel, text)

    # Non-markdown and runtime-ish directories that scanners should ignore or skip.
    write(out / "Attachments/image.png", "not actually an image; fixture only\n")
    write(out / "Exports/report.csv", "a,b\n1,2\n")
    write(out / ".obsidian/workspace.json", "{\"ignored\": true}\n")
    write(out / ".trash/deleted.md", "# Deleted\nScanner should ignore hidden trash-like dirs.\n")
    write(out / "node_modules/package/readme.md", "# Dependency Readme\nShould be ignored by scanner.\n")
    write(out / "vault-layer.db", "not a real db; should not be treated as markdown\n")

    md_files = sorted(p for p in out.rglob("*.md") if ".obsidian" not in p.parts and "node_modules" not in p.parts and ".trash" not in p.parts)
    print(f"messy_vault={out}")
    print(f"markdown_notes={len(md_files)}")
    print("features=nested,weird-filenames,unicode,duplicates,empty,frontmatter,wikilinks,tags,long,non-md,hidden-runtime-dirs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
