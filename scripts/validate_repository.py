#!/usr/bin/env python3
"""Repository-complete guard for VaultLayer."""
from __future__ import annotations

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
REQUIRED = [
    "README.md", "AGENTS.md", "LICENSE", "CHANGELOG.md", "CONTRIBUTORS.md",
    ".env.example", "SUPPORT.md", "CODE_OF_CONDUCT.md", "SECURITY.md", "NOTICE.md",
    ".editorconfig", ".github/pull_request_template.md", ".github/CODEOWNERS",
    ".github/ISSUE_TEMPLATE/config.yml", "docs/ARCHITECTURE.md", "docs/ROADMAP.md",
    "docs/REPO_COMPLETE.md", "docs/MAINTAINER_CHECKLIST.md", "docs/PACKAGE.md",
    "docs/FILL_LOOP.md", "docs/TASK_TEMPLATE.md", "docs/GO_LOOP_BRIEF.md",
    "docs/HERO_GUIDELINES.md", "assets/hero.svg",
]
FORBIDDEN_PATH_PATTERNS = [
    r"(^|/)\.go-workflow(/|$)", r"(^|/)tasks\.md$", r"(^|/)\.obsidian(/|$)",
    r"\.(db|sqlite|sqlite3|libsql|turso|parquet|arrow)$",
]
FORBIDDEN_TEXT = ["go_workflow", "repo-local go-workflow"]


def tracked_files() -> list[str]:
    result = subprocess.run(["git", "ls-files"], cwd=ROOT, text=True, capture_output=True, check=True)
    return result.stdout.splitlines()


def main() -> int:
    errors: list[str] = []
    files = tracked_files()
    file_set = set(files)
    for path in REQUIRED:
        if path not in file_set:
            errors.append(f"missing required file: {path}")
    for path in files:
        for pattern in FORBIDDEN_PATH_PATTERNS:
            if re.search(pattern, path):
                errors.append(f"forbidden tracked path: {path}")
    workflows = [path for path in files if path.startswith(".github/workflows/")]
    if workflows:
        errors.append("GitHub Actions workflows are opt-in only: " + ", ".join(workflows))
    for path in files:
        if not path.endswith((".md", ".py", ".toml", ".yml", ".yaml", ".rs", ".txt", "Makefile")):
            continue
        text = (ROOT / path).read_text(encoding="utf-8", errors="ignore")
        for forbidden in FORBIDDEN_TEXT:
            if forbidden in text and path not in {"docs/GO_LOOP_BRIEF.md", "scripts/validate_repository.py"}:
                errors.append(f"forbidden legacy workflow text in {path}: {forbidden}")
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print("repository guard ok")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
