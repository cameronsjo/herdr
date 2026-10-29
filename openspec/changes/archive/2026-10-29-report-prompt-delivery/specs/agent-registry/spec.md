## ADDED Requirements

### Requirement: Prompt results report delivery
A successful `agent.prompt` result SHALL carry `delivered`, set from what herdr observes for up to 1.5 seconds after the prompt is written, capped by any `--wait` timeout. It MUST be `typed` when the agent was not working before the write and then starts working, blocks, or shows the prompt; `queued` when it was working before the write and shows the prompt; and `unknown` otherwise. A client MUST read an unrecognized value as `unknown`.

#### Scenario: Prompt to an idle agent
- **WHEN** `herdr agent prompt main "Review this diff"` targets an idle agent that starts working
- **THEN** the result's `delivered` is `typed`

#### Scenario: Prompt to a working agent
- **WHEN** a prompt is sent to an agent that is already working and its text appears in the agent's input area
- **THEN** the result's `delivered` is `queued`

#### Scenario: No evidence
- **WHEN** a prompt is sent to a working agent that collapses it into a paste placeholder
- **THEN** the result's `delivered` is `unknown` after at most 1.5 seconds

### Requirement: A prompt is shown when its opening text appears again
The agent SHALL count as showing the prompt when the first 32 characters of the prompt's first non-blank line, whitespace collapsed, appear on more rows of the pane's detection region than they did before the write. A first line shorter than 4 characters MUST never count as shown.

#### Scenario: Resending text already on screen
- **WHEN** the prompt's opening text is already on one row before the write and on two rows after it
- **THEN** the agent counts as showing the prompt

#### Scenario: Very short prompt
- **WHEN** the prompt is `ok`
- **THEN** only a status change can count as evidence
