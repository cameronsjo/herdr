# labels Specification

## Purpose
Labels covers the fork's single sanitizing rule for every workspace, tab, pane and worktree label herdr stores or draws, including labels derived from paths, and the grammar an agent name must satisfy.

## Requirements

### Requirement: One sanitizer for every stored label
Every workspace, tab and pane label setter, whether reached through the API, the CLI, worktree open, layout apply or session restore, SHALL strip Unicode control characters (Cc, such as ESC, newline and BEL) and format characters (Cf, such as bidi overrides, zero-width space and joiner, and the byte-order mark), and MUST keep at most 128 characters. Multibyte text such as CJK MUST pass unchanged.

#### Scenario: Escape sequence in a workspace label
- **WHEN** `workspace.rename` is called with the label `stat\u001b[31mus`
- **THEN** the stored label is `stat[31mus`

#### Scenario: Bidi override in a tab label
- **WHEN** a tab is renamed to `safe\u202egnp.exe`
- **THEN** the stored label is `safegnp.exe`

#### Scenario: CJK label
- **WHEN** a workspace is renamed to `作業`
- **THEN** the stored label is `作業`

#### Scenario: Very long label
- **WHEN** a pane label of 500 characters is set
- **THEN** the stored label is its first 128 characters

### Requirement: Path-derived workspace labels are sanitized
A workspace label derived from a directory SHALL pass through the same sanitizer, because a checked-out repository names its own directory. For a non-repository directory the label is the sanitized final path component, falling back to the sanitized full path when that component sanitizes to empty. For a repository the label is the sanitized repository directory name, and a worktree group's repository name falls back to `repo` when it sanitizes to empty.

#### Scenario: Directory name with control characters
- **WHEN** a workspace opens on a directory named `proj\u0007ect` with no custom label
- **THEN** the workspace's label is `project`

#### Scenario: Name made only of stripped characters
- **WHEN** a workspace opens on a non-repository directory whose final component is only a zero-width space
- **THEN** the label is the sanitized full path rather than an empty string

### Requirement: Branch names are sanitized when drawn
A workspace's git branch label SHALL be sanitized with the same rule before it is drawn, and a branch that sanitizes to blank MUST show no branch label.

#### Scenario: Branch with a format character
- **WHEN** the checked-out branch name contains a right-to-left override
- **THEN** the drawn branch label omits it

### Requirement: Agent view labels drop format characters
The label of an agent view, which is drawn beside a click target in the agents panel header, MUST drop format characters as well as control characters.

#### Scenario: View label with a zero-width joiner
- **WHEN** an agent view is set with the label `my\u200dview`
- **THEN** the view's label is `myview`

### Requirement: Agent name grammar
An agent name SHALL be a lowercase ASCII letter followed by lowercase ASCII letters, digits, `-` or `_`, at most 32 bytes in total. The check MUST run on the raw input before any sanitizing, and a name that fails it MUST NOT be stored as a routing key by any path, including session restore.

#### Scenario: Valid names
- **WHEN** an agent is started with the name `reviewer-2` or `a_b`
- **THEN** the name is accepted

#### Scenario: Invalid names
- **WHEN** an agent name is `Reviewer`, ` reviewer`, `reviewer one`, `2nd`, `rev\u200biewer` or 33 bytes long
- **THEN** it is refused and no pane carries it
