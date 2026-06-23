.PHONY: check

check:
	@echo "VaultLayer scaffold check"
	@test -f README.md
	@test -f AGENTS.md
	@test -f docs/architecture.md
	@if cargo fmt --version >/dev/null 2>&1; then cargo fmt --all -- --check; else echo "cargo fmt unavailable; skipping format check"; fi
	@cargo clippy --workspace --all-targets -- -D warnings
	@cargo test --workspace
	@! git ls-files | grep -E '\.(db|sqlite|sqlite3|libsql|turso)$$'
	@git diff --check
