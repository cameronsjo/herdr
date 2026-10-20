# command-palette Specification

## Purpose
The command palette lets the operator find and run any bindable herdr action, plugin action or plugin pane by typing part of its name, and is the keyboard route to actions that have no key bound.

## Requirements

### Requirement: Every bindable action is a palette row
The palette SHALL offer one row for every keybind help entry that carries an action with a palette id (`core:<name>`), plus one row per enabled plugin action and plugin pane. The palette's own open action MUST NOT be a row. Tab reorder (`move tab left`, `move tab right`), workspace reorder, pane swap, one-step pane resize (`resize pane left|down|up|right`), the space moves and `clear agent view` MUST all be runnable rows, whether or not a key is bound to them.

#### Scenario: Unbound resize from the palette
- **WHEN** no key is bound to `resize_pane_left` and the operator types `resize pane left` and presses Enter
- **THEN** the focused pane resizes one step left

#### Scenario: Palette does not list itself
- **WHEN** the operator types `command palette`
- **THEN** no row matches

### Requirement: Compact empty palette led by recent commands
With an empty query the palette SHALL show at most 12 rows: the most recently run commands first (newest on top, no duplicates, skipping any no longer available), then a fixed featured list of common actions (new workspace, new and open worktree, new tab, both splits, pane and tab moves to a space, zoom, toggle sidebar, settings). Typing any query MUST search every row, including rows the compact list omits.

#### Scenario: Recent command leads
- **WHEN** the operator last ran `resize pane left` from the palette and reopens it with no query
- **THEN** `resize pane left` is the first row and the list holds 12 rows

### Requirement: Recent command history persists
Running a palette row SHALL put its command id at the front of a history capped at 8 entries, moving an existing entry rather than duplicating it, and MUST save it to `palette-history.json` in herdr's state directory. A missing file reads as empty history, and a failed save is logged without stopping the command. A row that only opens a chooser, or a destructive row the operator cancels, MUST NOT be recorded; a chooser leaf is recorded under its own id.

#### Scenario: Cancelled destructive action
- **WHEN** the operator selects a destructive plugin row and chooses `cancel`
- **THEN** the history is unchanged

#### Scenario: Chooser leaf is recorded
- **WHEN** the operator runs `move tab...` and picks `right`
- **THEN** `core:move-tab-next` is recorded, not the family row

### Requirement: Ranking by word-boundary match, keywords last
A query (trimmed, case-insensitive) SHALL match a row's name as an exact match, then a prefix, then a match starting at a word boundary, where a boundary is the start of the name or any character after a non-alphanumeric one. A query landing mid-word MUST NOT match. When the name does not match, the row MAY match through its hand-authored keywords, and any keyword match MUST rank below every name match. Ties keep list order.

#### Scenario: Mid-word text is not a match
- **WHEN** the operator types `move` and a plugin action is named `Uninstall web bridge (remove service)`
- **THEN** that plugin row is not listed

#### Scenario: Exact name first
- **WHEN** the operator types `new tab`
- **THEN** `new tab` is the first row and `move pane to new tab` is still listed below it

### Requirement: Cross-vocabulary keywords
Rows SHALL carry synonym keywords so other surfaces' wording finds them: `split vertical` matches `split right`, `split left` and `new pane`; `split horizontal` matches `split down`, `split up` and `new pane`; `swap pane <dir>` matches `move pane <dir>`; `move tab to space` matches `send tab to workspace`; the space moves and `merge workspace into...` carry their own workspace-wording synonyms.

#### Scenario: Context-menu wording finds the palette action
- **WHEN** the operator types `split right`
- **THEN** the `split vertical` row is listed, below any row whose own name matches

### Requirement: Right column shows key, match reason, or dash
Each palette row's right column SHALL show the row's bound key when it has one; otherwise `matched: <keyword>` when it matched through a keyword; otherwise `—`. The palette MUST NOT show `unset` (the keybind help screen still does). The right column and any destructive tag are reserved before the name is drawn, so a long name is clipped rather than pushing them off the row.

#### Scenario: Keyword hit with no key
- **WHEN** `split vertical` has no binding and the query is `split right`
- **THEN** its right column reads `matched: split right`

#### Scenario: Name hit with no key
- **WHEN** a row with no binding matches on its own name
- **THEN** its right column reads `—`

### Requirement: Direction families collapse into a chooser
Tab reorder, workspace reorder and pane swap SHALL each appear as one family row (`move tab...`, `move workspace...`, `swap pane...`) that opens a chooser of directions. Their per-direction leaf rows MUST be hidden unless a query token is a prefix of `left`, `right`, `up` or `down`. A chooser button runs exactly what its leaf row runs. Cancelling a chooser opened from the palette (Esc or `cancel`) SHALL reopen the palette with the same query and selected row.

#### Scenario: Directionless query
- **WHEN** the operator types `move tab`
- **THEN** `move tab...` is listed and `move tab left` is not

#### Scenario: Direction named
- **WHEN** the operator types `move tab l`
- **THEN** `move tab left` is listed

#### Scenario: Back out of a chooser
- **WHEN** the operator opens `swap pane...` from the query `swap` and presses Esc
- **THEN** the palette is open again with query `swap` and the same row selected

### Requirement: Split-direction chooser for pane moves into a tab
Moving a pane into an existing tab, from the move-pane picker's pane row or by dropping a dragged pane on a tab, SHALL open a chooser titled with the destination (`split beside <pane> in <space> / tab <tab>`, or `split into <space> / tab <tab>`) offering `vertical` and `horizontal`, with `v` and `h` as shortcuts. Nothing moves until a direction is chosen; the choice sends `pane.move` to that tab with a split to the right or down at ratio 0.5.

#### Scenario: Drop on a tab asks first
- **WHEN** the operator drags a pane onto another tab in the tab bar
- **THEN** the split chooser opens and no move is sent
- **AND** pressing `h` moves the pane into that tab split downward

### Requirement: Plugin rows follow the server's platform and focus
Plugin rows SHALL come from one `plugin.list` request per palette open, sent only to an endpoint that supports it, and list only enabled plugins. A row MUST be hidden only when its declared platforms exclude the platform the server reported; with no reported platform it is shown. An action declaring contexts shows only when one applies (`pane` needs a focused pane, `workspace` and `tab` a focused workspace, `selection` never).

#### Scenario: Endpoint without plugin.list
- **WHEN** the endpoint does not advertise `plugin.list`
- **THEN** the palette opens with its core rows and no notice is shown

### Requirement: Destructive plugin actions are tagged and confirmed
A plugin action SHALL be destructive when its manifest `[[actions]]` entry sets `destructive = true`, or when `[palette] destructive_actions` lists the exact string `"<plugin_id>:<action_id>"`. A destructive row MUST show a ` [destructive]` tag, and Enter on it MUST open a confirm chooser titled with the full row name, offering `cancel` (selected) then `run anyway`. Plugin panes and core actions are never marked; a manifest without the key reads as not destructive.

#### Scenario: Operator override
- **WHEN** config sets `[palette] destructive_actions = ["herdr.collie:uninstall"]` and the operator presses Enter on that row
- **THEN** the confirm chooser opens with `cancel` selected
- **AND** pressing Enter again runs nothing

#### Scenario: Id must match exactly
- **WHEN** the override lists `collie:uninstall` but the plugin id is `herdr.collie`
- **THEN** the row is not tagged

### Requirement: Plugin row labels are sanitized and unambiguous
Plugin row labels SHALL be sanitized before drawing and read `<plugin> — <title>`, unless the title already starts with the plugin name, in which case the title alone is used. When two plugin rows would share a label, each MUST be suffixed ` (action)` or ` (pane)`.

#### Scenario: Action and pane share a title
- **WHEN** a plugin has an action and a pane both titled `Logs`
- **THEN** the rows read `<plugin> — Logs (action)` and `<plugin> — Logs (pane)`
