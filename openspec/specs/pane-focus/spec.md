# pane-focus Specification

## Purpose
Pane focus covers how the fork's CLI focuses a pane: by direction within a tab, as upstream, or by id from anywhere.

## Requirements

### Requirement: pane focus jumps to a pane by id
`herdr pane focus <PANE_ID>` SHALL send `pane.focus` for that pane, which switches to the pane's workspace and tab and focuses it. `herdr pane focus --direction <DIR>` SHALL keep sending `pane.focus_direction`. The command MUST refuse to run with neither a pane id nor `--direction`, and MUST refuse a pane id combined with `--direction`, `--pane` or `--current`.

#### Scenario: Pane in another workspace
- **WHEN** the operator is in workspace `w1` and runs `herdr pane focus w2:p3`
- **THEN** `pane.focus` is sent for `w2:p3`, and workspace `w2`, the pane's tab and the pane become focused

#### Scenario: Direction
- **WHEN** `herdr pane focus --direction up` is run
- **THEN** `pane.focus_direction` is sent with direction `up`

#### Scenario: Neither given
- **WHEN** `herdr pane focus` is run with no arguments
- **THEN** the command fails with a usage error and sends nothing
