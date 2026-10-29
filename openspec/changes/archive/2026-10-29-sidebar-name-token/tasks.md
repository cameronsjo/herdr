## 1. Token

- [x] 1.1 Parse and serialize `name` as an agent sidebar token
- [x] 1.2 Resolve it from the agent's herdr name only, omitting it when unnamed

## 2. Verification

- [x] 2.1 Tests: config parses `name` apart from `$name`; a named agent shows its name beside a reported display name, an unnamed one drops the token
- [x] 2.2 Pass `scripts/openspec.sh validate --all --strict`

## 3. Docs and ledger

- [x] 3.1 Document the token in `configuration.mdx`
- [x] 3.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
