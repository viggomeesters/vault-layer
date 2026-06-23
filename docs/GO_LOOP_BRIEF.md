# Go Loop Brief

VaultLayer go-loop/campaign work should use external Agent Workflow state, not repo-local `.go-workflow`.

## Campaign protocol

- Execute one public-safe slice at a time.
- Stop on private-data risk, generated artifact drift, failing `make check`, or ambiguous vault writeback behavior.
- Do not create `.go-workflow/` in this repo.
- Keep durable orchestration state outside the repo.
