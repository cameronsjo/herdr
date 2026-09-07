# Sync upstream into the fork

Codex will merge upstream `herdrdev/herdr` master at `792b2baa` into the fork based on `c8e54868`, preserve fork behavior, validate, and open a PR against `cameronsjo/herdr:master`. Cameron approved this approach and the merge subject `chore: sync upstream master into the fork`.

Use an isolated worktree. Preserve the fork README, palette, grouped sidebar and trailing tokens, workspace merge/move operations, integration behavior, and fork-only distribution gates. No upstream writes or release actions. Merge rather than reset/rebase so existing fork history survives.

This is release-risk integration across client state, input projection, and API behavior. Existing palette, sidebar, workspace merge, endpoint contract and identity tests are characterization coverage. Review both conflicts and clean merges that cross these features; use a bounded agent review before delivery.

- [x] Fetch both remotes, verify fork account and preview conflicts.
- [ ] Merge and resolve conflicts, inspect behavior overlap.
- [ ] Review correctness and simplification; fix findings.
- [ ] Run `just check` and applicable scaling validation.
- [ ] Commit and push sync branch, open fork PR, inspect checks.

Validation results and deviations will be recorded below.
