## Purpose
The agent skill is the guidance file agents read to drive herdr. The fork splits it into a `SKILL.md` plus progressive-disclosure references, bundles all of them into the binary, and lets the binary write the whole directory.

## ADDED Requirements

### Requirement: The skill is a directory with references
The agent skill SHALL be `skills/herdr/SKILL.md` plus reference files under `skills/herdr/references/` (today `agent-lifecycle.md`, `cli-semantics.md` and `reading-output.md`), with `SKILL.md` pointing to the references for detail. Every file under `references/` MUST be bundled into the binary at build time, so the emitted skill always matches the running release.

#### Scenario: A reference file is added
- **WHEN** a new `.md` file is added under `skills/herdr/references/` without bundling it
- **THEN** the build's skill-bundling test fails

### Requirement: herdr --skill prints the skill
`herdr --skill` SHALL print the bundled `SKILL.md` to stdout and exit with status 0, without starting or contacting a server.

#### Scenario: Print the skill
- **WHEN** `herdr --skill` is run
- **THEN** the contents of `SKILL.md` are printed and the process exits 0

### Requirement: herdr --skill --install writes the directory
`herdr --skill --install <dir>` (or `--install=<dir>`) SHALL create `<dir>/references/`, write `SKILL.md` and every bundled reference file, and print `installed herdr skill to <dir>`. Installing over an existing directory MUST remove any `references/*.md` file this build does not carry and MUST NOT touch any other file in `<dir>`.

#### Scenario: Fresh install
- **WHEN** `herdr --skill --install ~/.agents/skills/herdr` is run on a missing directory
- **THEN** the directory holds `SKILL.md` and `references/` with every bundled reference

#### Scenario: Stale reference pruned
- **WHEN** `<dir>/references/old.md` exists from an earlier release and `<dir>/notes.txt` exists, and the install runs again
- **THEN** `old.md` is removed and `notes.txt` is left in place

### Requirement: --install needs a directory operand
`--install` with no operand, an empty operand, or an operand starting with `-` (in either spelling) MUST fail with `error: --install requires a directory` and exit status 2. A failure to write the directory MUST print the error and exit with status 1.

#### Scenario: Missing operand
- **WHEN** `herdr --skill --install` is run with nothing after it
- **THEN** the process prints `error: --install requires a directory` and exits 2

#### Scenario: Flag-shaped operand
- **WHEN** `herdr --skill --install=-x` is run
- **THEN** the process exits 2 without writing anything
