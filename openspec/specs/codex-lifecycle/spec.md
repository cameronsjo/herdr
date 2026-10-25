# codex-lifecycle Specification

## Purpose
The fork's Codex integration reports the Codex session and a pane title through hooks. Codex state comes from screen detection alone, as upstream.

## Requirements

### Requirement: Reinstalling the Codex hook replaces its entries
Reinstalling the Codex integration MUST replace herdr's earlier `hooks.json` entries rather than duplicating them, and MUST remove the `working`, `blocked` and `idle` entries an older fork integration installed, leaving other commands on those events in place.

#### Scenario: Reinstall
- **WHEN** `herdr integration install codex` is run twice
- **THEN** `hooks.json` holds one herdr entry per event and action

#### Scenario: Upgrade from the hybrid integration
- **WHEN** `hooks.json` holds the fork's v11 `working`, `blocked` and `idle` entries and a user command on `UserPromptSubmit`, and the integration is reinstalled
- **THEN** no `PreToolUse`, `PermissionRequest`, `PostToolUse` or `Interrupt` entry remains, and the user command is kept

### Requirement: The hook ignores events outside its contract
The hook script SHALL accept only the `session` and `metadata` actions and MUST exit without reporting for any other action, when it runs outside a herdr pane (`HERDR_ENV` is not `1`, or `HERDR_PANE_ID` or, for the shell script, `HERDR_SOCKET_PATH` is empty), when the payload's `hook_event_name` does not match its installed action, when the payload has no `session_id`, when a `SessionStart` payload has no transcript path, or when `CODEX_THREAD_ID` is set to a different session (a nested Codex).

#### Scenario: Nested Codex session
- **WHEN** a hook fires with a `session_id` different from the inherited `CODEX_THREAD_ID`
- **THEN** the hook sends no report

#### Scenario: Leftover state hook
- **WHEN** a hook from an older integration runs with the `working` action on `UserPromptSubmit`
- **THEN** the hook sends no report

### Requirement: The Codex hook reports a pane title
On `SessionStart`, `UserPromptSubmit` and `Stop`, the hook SHALL report a title through `pane.report_metadata`, read from the thread's name or preview over Codex's app-server protocol, or on `UserPromptSubmit` the submitted prompt. The title MUST drop leading `[Image #N]` markers, collapse whitespace, and cut anything over 80 characters to its first 77 with trailing whitespace stripped, plus `...`. It MUST NOT change Codex's thread id, and nothing is sent when no title can be derived.

#### Scenario: Thread renamed in Codex
- **WHEN** the Codex thread was renamed with `/rename fix login` and a turn stops
- **THEN** the pane's title becomes `fix login`

#### Scenario: Long prompt fallback
- **WHEN** the app-server is unavailable and the user submits a 200-character prompt
- **THEN** the reported title is the prompt's first 77 characters, trailing whitespace stripped, followed by `...`

### Requirement: Codex integration version 11
The installed Codex hook script SHALL carry `HERDR_INTEGRATION_VERSION=11`, and an installed Codex integration with a lower or missing version MUST be reported as outdated by `herdr integration status` so it is reinstalled. The version MUST be above both sides' version 10, since upstream's session-only script also carries 10 and would otherwise never be replaced.

#### Scenario: Upstream-installed script
- **WHEN** the Codex hook on disk carries `HERDR_INTEGRATION_VERSION=10`
- **THEN** `herdr integration status` reports the Codex integration as outdated
