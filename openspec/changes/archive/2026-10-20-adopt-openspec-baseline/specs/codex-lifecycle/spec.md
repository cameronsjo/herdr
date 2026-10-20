## Purpose
The fork's Codex integration reports Codex turn state and a pane title through hooks while screen detection stays active to correct stale reports, a hybrid kept deliberately against upstream's screen-only Codex state.

## ADDED Requirements

### Requirement: The Codex hook reports lifecycle state
`herdr integration install codex` SHALL install a hook script in the Codex config directory (`CODEX_HOME`, default `~/.codex`) and register it in `hooks.json`: `UserPromptSubmit`, `PreToolUse` and `PostToolUse` report `working`, `PermissionRequest` reports `blocked`, `Stop` and `Interrupt` report `idle`, and `SessionStart` reports the session. State MUST be reported through `pane.report_agent` as source `herdr:codex`, agent `codex`.

#### Scenario: Turn lifecycle
- **WHEN** Codex runs in a herdr pane with the integration installed and the user submits a prompt, then Codex asks for a permission, then the turn stops
- **THEN** the pane's agent state goes `working`, then `blocked`, then `idle`

### Requirement: Reinstalling the Codex hook replaces its entries
Reinstalling the Codex integration MUST replace herdr's earlier `hooks.json` entries rather than duplicating them.

#### Scenario: Reinstall
- **WHEN** `herdr integration install codex` is run twice
- **THEN** `hooks.json` holds one herdr entry per event and action

### Requirement: Codex hook reports are honored
The server MUST accept `herdr:codex` state reports for the `codex` agent; `herdr:codex` MUST NOT be treated as a reserved native-state source whose reports are discarded. A report whose session id differs from the pane's current Codex session MUST be ignored.

#### Scenario: Hook report reaches the pane
- **WHEN** the Codex hook reports `working` for its pane's current session
- **THEN** the pane's agent state becomes `working`

#### Scenario: Report from another session
- **WHEN** a Codex hook report carries a session id different from the one the pane already holds
- **THEN** the report is ignored and the pane's state does not change

### Requirement: Screen detection corrects stale Codex reports
Because Codex runs hooks concurrently and emits no approval-resolved hook, screen detection SHALL stay active alongside the hook. A visible working signal that is not older than the hook report MUST override a Codex `idle` or `blocked` report and show `working`, and a visible blocker MUST override a working report and show `blocked`.

#### Scenario: Stale Stop during a running turn
- **WHEN** the Codex hook last reported `idle` but the screen now shows Codex working
- **THEN** the pane's state is `working`

#### Scenario: Stale working during an approval
- **WHEN** the Codex hook last reported `working` but the screen shows an approval prompt
- **THEN** the pane's state is `blocked`

### Requirement: The hook ignores events outside its contract
The hook script MUST exit silently, sending nothing, when it runs outside a herdr pane (`HERDR_ENV`, `HERDR_SOCKET_PATH` or `HERDR_PANE_ID` unset), when the payload's `hook_event_name` does not belong to the action it was installed for, when the payload has no `session_id`, when a `SessionStart` payload has no transcript path, or when `CODEX_THREAD_ID` is set to a different session (a nested Codex). `Stop` and `Interrupt` MUST still report without a transcript path.

#### Scenario: Nested Codex session
- **WHEN** a hook fires with a `session_id` different from the inherited `CODEX_THREAD_ID`
- **THEN** the hook sends no report

#### Scenario: Interrupt without a transcript
- **WHEN** an `Interrupt` payload omits `transcript_path`
- **THEN** the hook still reports `idle`

### Requirement: The Codex hook reports a pane title
On `SessionStart`, `UserPromptSubmit` and `Stop`, the hook SHALL report a title through `pane.report_metadata`, read from the thread's name or preview over Codex's app-server protocol, falling back on `UserPromptSubmit` to the submitted prompt. The title MUST drop leading `[Image #N]` markers, collapse whitespace, and cut anything over 80 characters to 77 plus `...`. It MUST NOT change Codex's thread id, and nothing is sent when no title can be derived.

#### Scenario: Thread renamed in Codex
- **WHEN** the Codex thread was renamed with `/rename fix login` and a turn stops
- **THEN** the pane's title becomes `fix login`

#### Scenario: Long prompt fallback
- **WHEN** the app-server is unavailable and the user submits a 200-character prompt
- **THEN** the reported title is the prompt's first 77 characters followed by `...`

### Requirement: Codex integration version 11
The installed Codex hook script SHALL carry `HERDR_INTEGRATION_VERSION=11`, and an installed Codex integration with a lower or missing version MUST be reported as outdated by `herdr integration status` so it is reinstalled. The version MUST be above both sides' version 10, since upstream's session-only script also carries 10 and would otherwise never be replaced.

#### Scenario: Upstream-installed script
- **WHEN** the Codex hook on disk carries `HERDR_INTEGRATION_VERSION=10`
- **THEN** `herdr integration status` reports the Codex integration as outdated
