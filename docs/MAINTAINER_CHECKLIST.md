# Maintainer Checklist

Before pushing or releasing:

1. Run `make check`.
2. Confirm no private vault content is included.
3. Confirm no generated DB/index/embedding files are tracked.
4. Confirm examples are synthetic.
5. Confirm README/docs match CLI behavior.
6. Confirm GitHub About/topics are current.
7. For releases: update `CHANGELOG.md`, tag, build artifact if needed, create GitHub release.

No default CI: local `make check` is the repo-complete gate until CI is explicitly requested.
