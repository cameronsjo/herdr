## 1. Scaffold

- [x] 1.1 Run `openspec init --tools claude,codex` and commit the generated skills and commands
- [x] 1.2 Write the fork context and artifact rules in `openspec/config.yaml`
- [x] 1.3 Pin the CLI in `scripts/openspec/` and add `scripts/openspec.sh` and `scripts/openspec-localize.sh`

## 2. Baseline specs

- [x] 2.1 Write one spec per fork capability listed in the proposal, from the code first and `docs/fork/CHANGES.md` second
- [x] 2.2 Pass `scripts/openspec.sh validate --all --strict`
- [x] 2.3 Archive this change so the specs land in `openspec/specs/`

## 3. Process

- [x] 3.1 Add the fork-only `OpenSpec` workflow, with the pinned-CLI check on the generated skills
- [x] 3.2 Point the fork section of `AGENTS.md` at `openspec/` and freeze `docs/plans/`
- [x] 3.3 Add the `docs/fork/CHANGES.md` entry with its regression check
