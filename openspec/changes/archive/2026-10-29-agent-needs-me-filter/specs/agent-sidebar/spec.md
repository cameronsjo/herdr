## ADDED Requirements

### Requirement: Needs me shows only blocked agents
While the "needs me" filter is on, the Agent panel SHALL list only agents whose status is `blocked`, from every workspace and, in the multi-machine list, every machine. An empty result MUST say ` no agents need you`; the mobile list is titled `agents · needs me`, and the collapsed sidebar marks its divider with `!`. The filter is client state: it sends nothing, survives a config reload, and is not saved.

#### Scenario: One agent is blocked
- **WHEN** the filter is on and one of three agents is blocked
- **THEN** only the blocked agent is listed

#### Scenario: Nothing is blocked
- **WHEN** the filter is on and no agent is blocked
- **THEN** the panel shows ` no agents need you`

### Requirement: Turning needs me on and off
Clicking the panel header's order toggle SHALL cycle `grouped`, `priority`, `needs me`, then `grouped` again; while the filter is on the header reads `needs me` and the order underneath is `priority`. The palette row `toggle needs me` and the optional `keys.toggle_needs_me` binding, unset by default, MUST flip the filter.

#### Scenario: Cycle into needs me
- **WHEN** the header reads `priority` and the operator clicks the toggle
- **THEN** the header reads `needs me` and the filter is on

#### Scenario: Palette toggle
- **WHEN** the operator runs `toggle needs me` from the palette twice
- **THEN** the filter turns on and then off, and no endpoint method is sent
