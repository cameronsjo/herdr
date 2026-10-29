## ADDED Requirements

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
