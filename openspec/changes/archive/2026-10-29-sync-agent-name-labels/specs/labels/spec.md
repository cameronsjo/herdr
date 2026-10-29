## ADDED Requirements

### Requirement: Tab label that spells the agent name follows a rename
When `agent.rename` renames the tab's only live named agent, and the tab's label equals that agent's previous name, the tab's label SHALL become the new name and `tab_renamed` SHALL be emitted. An auto-named tab, a tab whose label differs from the previous name, and a tab holding two or more named live agents MUST keep their labels. `tab.rename` MUST NOT change any agent name, and clearing an agent name MUST NOT change any label.

#### Scenario: Agent renamed under a tab labeled with its name
- **WHEN** a tab labeled `cos-cadence` holds one pane, named `cos-cadence`, and `agent.rename` renames it to `cos-forge`
- **THEN** the tab's label is `cos-forge`

#### Scenario: Named agent beside an unnamed pane
- **WHEN** a tab labeled `cos-cadence` holds the live agent named `cos-cadence` and a shell pane with no agent name, and the agent is renamed to `cos-forge`
- **THEN** the tab's label is `cos-forge`

#### Scenario: Label of the user's own
- **WHEN** a tab labeled `backend` holds one agent named `reviewer` and the agent is renamed to `planner`
- **THEN** the tab's label stays `backend`

#### Scenario: Tab renamed
- **WHEN** a tab labeled `reviewer` holds the agent named `reviewer` and `tab.rename` sets the label `planner`
- **THEN** the agent keeps the name `reviewer`

### Requirement: Pane label that spells the agent name follows a rename
`agent.rename` SHALL replace a pane label equal to the agent's previous name with the new name, and MUST leave any other pane label unchanged.

#### Scenario: Pane label kept from a cold restore
- **WHEN** a pane labeled `reviewer` hosts the agent named `reviewer` and the agent is renamed to `planner`
- **THEN** the pane's label is `planner`

### Requirement: A live named agent titles its pane
While a pane's agent name routes to a live agent, its border title SHALL be its pane label when it has one, else its agent name, ahead of any reported title. Once no live agent backs the name, the border MUST fall back to upstream's order: reported title, pane label, then agent kind when enabled.

#### Scenario: Named agent with a session title
- **WHEN** a pane hosts a live agent named `reviewer` whose hooks report the title `Brief task execution`
- **THEN** the pane's border reads `reviewer`

#### Scenario: Agent has left
- **WHEN** the agent named `reviewer`, in a pane with no label, has exited and the pane is a shell again
- **THEN** the pane's border no longer reads `reviewer`
