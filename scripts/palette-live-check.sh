#!/usr/bin/env bash
# Live check for the palette direction chooser and destructive confirm.
#
# Runs the freshly built debug binary against its own dev server, with a
# throwaway config that marks one plugin action destructive so the confirm has
# something to fire on. Needs a real terminal: run it from a terminal window,
# not from an agent's shell.
set -uo pipefail

WORKTREE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$WORKTREE/target/debug/herdr"

if [ ! -t 0 ] || ! : < /dev/tty 2>/dev/null; then
  echo "This needs a controlling terminal. Run it from a terminal window." >&2
  exit 1
fi

if [ ! -x "$BIN" ]; then
  echo "Build it first:" >&2
  echo "  DEVELOPER_DIR=/Library/Developer/CommandLineTools \\" >&2
  echo "  ZIG=\"\$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig\" \\" >&2
  echo "  cargo build --bin herdr" >&2
  exit 1
fi

CONFIG_DIR="$(mktemp -d /tmp/herdr-palette-check.XXXXXX)"
trap 'rm -rf "$CONFIG_DIR"' EXIT
cat > "$CONFIG_DIR/config.toml" <<'EOF'
# Mark a third-party action destructive without touching its manifest. Replace
# the entry with a plugin you actually have installed.
[palette]
destructive_actions = ["collie:uninstall"]
EOF

cat <<EOF

Config: $CONFIG_DIR/config.toml
Binary: $BIN

What to check once herdr is up (open the palette with your prefix key then '/'):

  1. Type 'move'.
     Expect three rows ending in '...': move tab, move workspace, swap pane.
     Expect NO directional leaf rows, and NO plugin row for a mid-word 'remove'.
  2. Press Enter on 'swap pane...'.
     Expect a chooser with four buttons. Press Esc.
     Expect the palette back with 'move' still typed.
  3. Type 'move tab l'.
     Expect 'move tab left' ranked first.
  4. Look at the right column of every visible row.
     Expect a key, a 'matched: <keyword>' reason, or a dash. Never a blank.
  5. Type 'uninstall' with a plugin whose action the config above names.
     Expect a '[destructive]' tag on the row, and Enter to ask before running,
     with cancel selected. Press Enter again: expect it to cancel, not run.

Screenshot steps 1 and 5.

EOF

exec env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH \
  XDG_CONFIG_HOME="$CONFIG_DIR" "$BIN" "$@"
