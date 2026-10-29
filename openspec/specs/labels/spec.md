# labels Specification

## Purpose
Labels covers the fork's single sanitizing rule for every workspace, tab, pane and worktree label herdr stores or draws, including labels derived from paths, and where the fork enforces upstream's agent-name grammar.

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
A workspace label derived from a directory SHALL pass through the same sanitizer. For a non-repository directory the label is `~` for the home directory itself, otherwise the sanitized final path component, falling back to the sanitized full path when that component sanitizes to empty. For a repository the label is the sanitized repository directory name, and a worktree group's repository name falls back to `repo` when it sanitizes to empty.

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

### Requirement: Agent names are checked at the setter and on restore
The agent-name grammar is upstream's `valid_agent_name`, which upstream applies in `agent.start` and `agent.rename`. The fork SHALL also apply it inside the terminal's agent-name setter, on the raw input before any sanitizing, so a rejected name clears the routing key whatever path set it. Session restore MUST drop a stored name the grammar refuses, and when two panes restore the same name only the first MUST keep it.

#### Scenario: Format character is not laundered
- **WHEN** the setter receives `rev\u200biewer`
- **THEN** the pane carries no agent name, rather than `reviewer`

#### Scenario: Invalid stored name on restore
- **WHEN** a session file stores the agent name `Reviewer` for a pane
- **THEN** the pane restores with no agent name

#### Scenario: Duplicate stored name on restore
- **WHEN** a session file stores the agent name `reviewer` for two panes that both restore their agent
- **THEN** the first pane restored keeps `reviewer` and the second restores with no agent name

### Requirement: Tab label follows its only agent's name
When a tab holds exactly one pane with an agent name, the tab SHALL be linked to that name while it is auto-named or its label equals the agent's name. `agent.start` and `agent.rename` MUST set a linked tab's label to the new name and emit `tab_renamed`. A tab whose label differs from the agent's name, or that holds two or more named agents, MUST keep its label. Clearing an agent name MUST NOT change any label.

#### Scenario: Agent renamed under a tab labeled with its name
- **WHEN** a tab labeled `cos-cadence` holds one pane, named `cos-cadence`, and `agent.rename` renames it to `cos-forge`
- **THEN** the tab's label is `cos-forge`

#### Scenario: Label of the user's own
- **WHEN** a tab labeled `backend` holds one agent named `reviewer` and the agent is renamed to `planner`
- **THEN** the tab's label stays `backend`

### Requirement: Renaming a linked tab renames its agent
`tab.rename` on a linked tab SHALL also rename its agent when the raw new label passes the agent-name grammar and no other pane holds that name. Otherwise it MUST rename only the tab and still succeed.

#### Scenario: Tab renamed over its only agent
- **WHEN** a tab labeled `reviewer` holds the only agent named `reviewer` and `tab.rename` sets the label `planner`
- **THEN** the agent is named `planner`

#### Scenario: Tab label that is no agent name
- **WHEN** the same tab is renamed to `Code Review`
- **THEN** the tab's label is `Code Review` and the agent keeps the name `reviewer`

### Requirement: Pane label that spells the agent name follows a rename
`agent.rename` SHALL replace a pane label equal to the agent's previous name with the new name, and MUST leave any other pane label unchanged.

#### Scenario: Pane label kept from a cold restore
- **WHEN** a pane labeled `reviewer` hosts the agent named `reviewer` and the agent is renamed to `planner`
- **THEN** the pane's label is `planner`

### Requirement: Agent name titles the pane
A pane's border title SHALL be its agent name when it has one, ahead of the agent's reported title, the pane label and the agent kind.

#### Scenario: Named agent with a session title
- **WHEN** a pane hosts an agent named `reviewer` whose hooks report the title `Brief task execution`
- **THEN** the pane's border reads `reviewer`
