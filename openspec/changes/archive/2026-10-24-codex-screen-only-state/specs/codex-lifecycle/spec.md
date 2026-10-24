## REMOVED Requirements

### Requirement: The Codex hook reports lifecycle state
**Reason**: Screen detection, which reads Codex's terminal title first, reports the same states without the misrouting a shared Codex daemon causes, and the hooks cost a conflict on every upstream sync.
**Migration**: Codex state now follows upstream's screen-only detection. Reinstalling the integration removes the old state hooks.

### Requirement: Codex hook reports are honored
**Reason**: `herdr:codex` is a reserved native-state source again, as upstream.
**Migration**: None. Session reports still reach the pane.

### Requirement: Screen detection corrects stale Codex reports
**Reason**: With no Codex state reports, screen detection is the only Codex state source and needs no correction rule.
**Migration**: None.

## MODIFIED Requirements

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
