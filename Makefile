.PHONY: check

check:
	@echo "VaultLayer scaffold check"
	@test -f README.md
	@test -f AGENTS.md
	@test -f docs/architecture.md
	@! git ls-files | grep -E '\.(db|sqlite|sqlite3|libsql|turso)$$'
	@git diff --check
