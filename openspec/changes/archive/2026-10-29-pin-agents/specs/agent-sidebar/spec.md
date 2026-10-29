## ADDED Requirements

### Requirement: Pinned agents lead the panel
The agents panel SHALL list pinned agents before every other agent, in ascending `pin_order`, in `spaces` and `priority` order and under an active agent view (among the agents the view shows). While grouping is in effect the pinned agents MUST form one run under a `pinned` header. `blocked_first` MUST NOT move an agent above or within the pinned run. Agent navigation follows the same order.

#### Scenario: Pinned worker under priority order
- **WHEN** the panel is in `priority` order, agent A is blocked and agent B is pinned
- **THEN** B is drawn first and A second

#### Scenario: Grouped panel
- **WHEN** `group_by = "workspace"` and an agent in workspace `beta` is pinned
- **THEN** the panel shows a `pinned` header with that agent, then the `alpha` and `beta` runs

### Requirement: Pins in the multi-machine list
In the multi-machine agent list, space order SHALL keep each machine's pinned agents at the top of that machine's run, and an order that mixes machines (priority, or a sorting view) SHALL put every pinned agent first, by machine and then `pin_order`.

#### Scenario: Two machines in space order
- **WHEN** machine 2 has a pinned agent and the list is in `spaces` order
- **THEN** that agent leads machine 2's run and machine 1's run is drawn above it unchanged
