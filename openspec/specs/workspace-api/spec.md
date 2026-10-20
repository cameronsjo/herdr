# workspace-api Specification

## Purpose
The fork's workspace and tab API adds a reuse-aware `workspace.open`, a gated `workspace.merge`, cross-workspace `tab.move` destinations, and hardening of workspace ids and the API accept loop, so socket callers can reorganize spaces without silent duplicates, data loss, or a server crash.

## Requirements

### Requirement: workspace.open focuses the space already on a path
The `workspace.open` method (CLI `herdr workspace open [--cwd PATH] [--label TEXT] [--focus|--no-focus]`) SHALL resolve the requested path, or the `follow` cwd of `source_workspace_id`, and when a workspace is already open there return it with `already_open: true` instead of creating a second one. When no workspace is open there it SHALL create one and return `already_open: false`.

#### Scenario: A space already exists on the path
- **WHEN** a workspace is open on `/repo` and `workspace.open` is called with `cwd: "/repo"` and `focus: true`
- **THEN** the response is `workspace_opened` with that workspace and `already_open: true`
- **AND** no new workspace is created and that workspace becomes focused

#### Scenario: No space on the path
- **WHEN** no workspace is open on `/other` and `workspace.open` is called with `cwd: "/other"` and `label: "docs"`
- **THEN** a workspace is created on `/other` with the label `docs` and the response carries `already_open: false`

### Requirement: workspace.open reuse rules
`workspace.open` MUST match a workspace only by exact identity path after canonicalization, never by repository membership. When several workspaces match, the focused one MUST win over the lowest index. Reuse MUST NOT rename the reused workspace: `label` applies only to a workspace the call creates.

#### Scenario: Subdirectory of an open repository
- **WHEN** a workspace is open on `/repo` and `workspace.open` is called with `cwd: "/repo/sub"`
- **THEN** a new workspace is created on `/repo/sub`

#### Scenario: Two spaces on the path
- **WHEN** two workspaces are open on `/repo`, the second is focused, and `workspace.open` is called for `/repo`
- **THEN** the second workspace is returned

#### Scenario: Label on reuse
- **WHEN** `workspace.open` reuses the workspace on `/repo` and passes `label: "new"`
- **THEN** the reused workspace keeps its existing label

### Requirement: workspace.create keeps creating unconditionally
`workspace.create` MUST keep creating a new workspace even when one is already open on the same path. The new-space action of a client shell SHALL send `workspace.open` when no label was typed and the server advertises `workspace.open`, and SHALL fall back to `workspace.create` when the server does not advertise it or when the user typed a label, which is a deliberate second space.

#### Scenario: Unnamed new space against a fork server
- **WHEN** the user creates a new space without typing a name and the server advertises `workspace.open`
- **THEN** the client sends `workspace.open`

#### Scenario: Older server without workspace.open
- **WHEN** the user creates a new space without typing a name and the server does not advertise `workspace.open`
- **THEN** the client sends `workspace.create`

#### Scenario: Named new space
- **WHEN** the user types a name in the new-space prompt
- **THEN** the client sends `workspace.create` with that label

### Requirement: Two spaces on one path stay independent
A worktree group SHALL consist of the first non-linked workspace on a repository key plus the linked worktree workspaces on that key. A second non-linked workspace on the same checkout MUST NOT be a member of that group, so a group close or group merge on the parent MUST leave it open, and closing the duplicate MUST close only the duplicate. An implicit (non-group) close of a parent that owns linked worktrees MUST still be refused with `workspace_group_close_required`.

#### Scenario: Group close leaves the duplicate open
- **WHEN** workspaces `parent`, a linked worktree of it, and `duplicate` (a second space on the parent's checkout) are open, and `workspace.close` is called on `parent` with `close_group: true`
- **THEN** `parent` and its linked worktree close and `duplicate` stays open

#### Scenario: Closing the duplicate
- **WHEN** `workspace.close` is called on `duplicate` without `close_group`
- **THEN** only `duplicate` closes

### Requirement: workspace.merge moves every tab and closes the source
`workspace.merge` (CLI `herdr workspace merge <SOURCE_WORKSPACE_ID> <TARGET_WORKSPACE_ID> [--group]`) SHALL move every tab of the source workspace into the target, in order, and then close the emptied source. A source equal to the target, or a target inside the source's worktree group, MUST be refused with `workspace_merge_failed`; an unknown id MUST return `workspace_not_found`. Merging two background workspaces MUST NOT move the operator's view.

#### Scenario: Plain merge
- **WHEN** `workspace.merge` is called with a one-workspace source holding two tabs and a different target
- **THEN** the target holds its own tabs followed by the source's two tabs and the source workspace no longer exists

#### Scenario: Merge into itself
- **WHEN** `workspace.merge` names the same workspace as source and target
- **THEN** the response is the error `workspace_merge_failed`

### Requirement: Group-close intent gate on merge
When the source workspace owns linked worktree workspaces, `workspace.merge` MUST refuse with `workspace_group_merge_required` unless `merge_group: true` (CLI `--group`) is given, the same gate `workspace.close` applies, because any process in a pane can reach the socket. With group intent, every member of the source group SHALL merge into the target and close.

#### Scenario: Group source without intent
- **WHEN** `workspace.merge` is called on a source that owns a linked worktree, without `merge_group`
- **THEN** the response is the error `workspace_group_merge_required` and nothing moves

#### Scenario: Group source with intent
- **WHEN** the same call is made with `merge_group: true`
- **THEN** every group member's tabs land in the target and every member closes

### Requirement: Merge does not report moved tabs as closed
For each moved tab, `workspace.merge` SHALL emit `tab.moved_across_workspaces` carrying the new tab id, the previous tab id and the source workspace id, and only afterwards one `workspace.closed` event per closed source. Each of those `workspace.closed` events MUST carry `workspace: null`, never a snapshot describing tabs and panes that moved to the target and are still alive.

#### Scenario: Merge events
- **WHEN** a source with one tab is merged into a target
- **THEN** a `tab.moved_across_workspaces` event precedes exactly one `workspace.closed` event for the source
- **AND** that `workspace.closed` event has no workspace snapshot

### Requirement: Tab move takes an explicit destination
`tab.move` SHALL accept a `destination` of type `index` (`insert_index`), `workspace` (`workspace_id`, optional `insert_index`, default the end) or `new_workspace` (optional `label`); `tab.move_to_destination` SHALL accept the same with `destination` required. A bare `insert_index` MUST stay the legacy in-workspace reorder, and a request with neither MUST be refused with `tab_move_failed`. A cross-workspace move SHALL reissue the tab id and keep moved panes' old ids resolving.

#### Scenario: Neither destination nor index
- **WHEN** `tab.move` is called with only `tab_id`
- **THEN** the response is the error `tab_move_failed` and no tab moves

#### Scenario: Move to another workspace
- **WHEN** a tab of workspace A is moved with destination `workspace` B and no `insert_index`
- **THEN** the tab becomes the last tab of B with a new tab id

#### Scenario: Move to the same workspace
- **WHEN** a tab is moved with destination `workspace` naming its own workspace
- **THEN** the result is `changed: false` with `reason: same_workspace`

### Requirement: The tab move CLI takes exactly one destination
`herdr tab move <tab_id>` MUST take exactly one of `--index N`, `--workspace ID [--index N]` or `--new-workspace [--label TEXT]`, and MUST refuse mixed or missing destinations, and a `--label` without `--new-workspace`, with a usage error and no request sent.

#### Scenario: Mixed CLI destinations
- **WHEN** `herdr tab move t1 --new-workspace --workspace w2` is run
- **THEN** the CLI prints the `tab move` usage and sends no request

#### Scenario: Label without a new workspace
- **WHEN** `herdr tab move t1 --index 0 --label x` is run
- **THEN** the CLI reports that `--label` applies only to `--new-workspace`

### Requirement: Moving a space's only tab closes the space
Moving the last tab of a workspace to another workspace or to a new workspace SHALL move it and close the emptied source, emitting `tab.moved_across_workspaces` followed by `workspace.closed` with no snapshot. A view that was on the source MUST follow the tab without recording a focus change. When the source owns linked worktree workspaces, the move MUST instead return `changed: false` with `reason: last_tab_in_workspace`, since closing it would orphan them.

#### Scenario: Only tab moves to another space
- **WHEN** workspace A has one tab and it is moved with destination `workspace` B
- **THEN** the tab is in B, workspace A no longer exists, and the view shows the moved tab

#### Scenario: Only tab of a worktree group owner
- **WHEN** the only tab of a workspace that owns a linked worktree is moved to another workspace
- **THEN** the result is `changed: false`, `reason: last_tab_in_workspace`, and nothing moves

### Requirement: Restored workspace ids are validated
When a session restores, a stored workspace id SHALL be kept only when it is canonical (`w` plus an encoded public number from 1 to 4294967295) and not already used by an earlier restored workspace. A malformed or duplicate id MUST be replaced with a freshly generated canonical id, logged with the stored id escaped, the new id, the index, the label and the reason, and every canonical restored id MUST be reserved so no later workspace is handed the same id.

#### Scenario: Duplicate stored id
- **WHEN** a session file holds two workspaces whose stored id is `w5`
- **THEN** the first restores as `w5` and the second restores with a different canonical id

#### Scenario: Malformed stored id
- **WHEN** a session file holds a workspace whose stored id is `not-an-id`
- **THEN** that workspace restores with a generated canonical id and a warning names the reason `malformed`

### Requirement: An out-of-range workspace index never panics the server
Converting an internal workspace index that is out of range into a public workspace id SHALL log an error naming the caller and yield an empty id that matches no workspace, rather than panicking the server.

#### Scenario: Stale index in a release build
- **WHEN** a code path asks for the public id of a workspace index past the end of the list in a release build
- **THEN** an error is logged with the caller's location and the server keeps running

### Requirement: The API accept loop survives failures
The API socket listener SHALL keep accepting after an `accept` error, backing off from 10 ms doubling to 1 s and resetting on the next accepted connection. When the OS refuses a thread for one connection, only that connection MUST be dropped. Failures MUST be logged on the first of a streak and every 50th, a streak that hid failures SHALL end with a recovery line, and the listener MUST warn if it stops accepting while the server runs.

#### Scenario: Transient accept errors
- **WHEN** `accept` fails repeatedly with a resource-exhaustion error and then succeeds
- **THEN** the listener retries with growing delays capped at 1 s, serves the next connection, and logs that it is accepting again

#### Scenario: Connection thread cannot start
- **WHEN** spawning the thread for one API connection fails
- **THEN** that connection is dropped with a warning and later connections are still served
