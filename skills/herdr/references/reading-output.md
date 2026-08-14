# Reading output

## Running a command in another pane

Create a sibling pane, preserve the caller's working directory, and keep user focus unchanged:

```bash
herdr pane split --current --direction right --cwd "$PWD" --no-focus
```

Read the new pane ID from `.result.pane.pane_id`, then run and inspect:

```bash
herdr pane run <returned-pane-id> "just test"
herdr pane wait-output <returned-pane-id> --match "test result" --timeout 120000
herdr pane read <returned-pane-id> --source recent-unwrapped --lines 120
```

`pane run` atomically sends command text and Enter. `pane wait-output` searches the selected snapshot immediately, so output that already exists can match. Use `--match <text>` for a literal substring or `--regex <pattern>` for a Rust regular expression. Omitting `--timeout` allows an indefinite wait.

## Read sources

| `--source` | Content |
|---|---|
| `visible` | The currently rendered viewport |
| `recent` | Recent rendered output, including soft wraps |
| `recent-unwrapped` | Recent output with soft wraps joined — prefer for logs and transcripts |
| `detection` | The plain-text bottom-buffer snapshot used for agent detection |

Use `--format ansi` when colors and terminal styling are evidence. Otherwise use text.

`recent` and `recent-unwrapped` return **nothing** on a pane whose output has not yet exceeded its viewport — a freshly split pane running one short command reads empty from both, while `visible` has the output. An empty read there is not a failed command. Reach for `visible` on a short-output pane, and keep `recent-unwrapped` for the logs and transcripts it is meant for.

## The alternate-screen ceiling

`--lines` asks Herdr for more rows from the pane's available screen and host scrollback.

If increasing it does not reveal more of a completed response, the pane is probably running the agent on the terminal's alternate screen. Rows that leave the alternate screen do not enter Herdr's host scrollback, so a larger line count cannot recover them.

## Fallback: write to a file

After that failed read, ask the agent to write its complete response as Markdown in a temporary directory and reply only with the file path, then read the file directly.

Use this only as a fallback. Do not request file output in the initial prompt.
