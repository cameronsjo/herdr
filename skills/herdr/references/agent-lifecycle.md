# Agent lifecycle

## States

| State | Means |
|---|---|
| `idle` | Ready for input, and its tab has been seen in the focused Herdr UI |
| `done` | The same underlying idle state, after unseen background work finished |
| `working` | A turn is in progress |
| `blocked` | Herdr recognized an approval or question UI |
| `unknown` | An agent is present but Herdr cannot classify it confidently — this does not prove completion |

Focusing the tab, or targeting the pane or agent with a focus command, marks it seen. CLI reads do not mark it seen.

## Starting an agent

An available shell pane must be at its interactive prompt, with the shell in the foreground and no foreground command, editor, or agent running.

```bash
herdr agent start reviewer --kind codex --pane <returned-pane-id>
```

Use the kind the user requested. Run `herdr agent` to inspect the installed kind list and options. Pass native agent arguments only after `--`:

```bash
herdr agent start reviewer --kind codex --pane <returned-pane-id> -- <agent-args...>
```

`agent start` returns only after Herdr detects the expected agent in the same pane and considers it ready for interactive input. It defaults to a 30-second startup timeout.

## Prompting

```bash
herdr agent prompt reviewer "Review the current diff and report only actionable findings." --wait --timeout 120000
```

`agent prompt` atomically submits text and encoded Enter while honoring the pane's live bracketed-paste mode. For normal agent work `--wait` is enough: it waits for the first settled `idle`, `done`, or `blocked` state. Do not repeat those defaults with `--until`.

### `agent_prompt_stalled`

A prompt sent from a non-working state must produce an observed lifecycle change within five seconds. Otherwise Herdr returns `agent_prompt_stalled` instead of waiting indefinitely.

This wait tracks lifecycle state, not an individual turn. If the agent is already working, completion of the active turn may satisfy it.

## Waiting for a specific state

Use `--until` only for a state-specific workflow, such as waiting for an already-running agent to request input:

```bash
herdr agent wait reviewer --until blocked --timeout 120000
```

Without `--until`, standalone `agent wait` uses the same settled-state defaults as `agent prompt --wait`.

## Interactive UI controls

Use logical keys, not raw bytes:

```bash
herdr agent send-keys reviewer esc
herdr agent send-keys reviewer ctrl+c
```

Herdr validates all keys before writing any bytes.

## After a wait

If a wait fails or returns `blocked`, inspect before deciding what input to send:

```bash
herdr agent get reviewer
herdr agent read reviewer --source recent-unwrapped --lines 120
```

Use the pane surface only when raw terminal control is intentional.
