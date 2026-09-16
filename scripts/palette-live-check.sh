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

build_hint() {
  echo "Build it first:" >&2
  echo "  DEVELOPER_DIR=/Library/Developer/CommandLineTools \\" >&2
  echo "  ZIG=\"\$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig\" \\" >&2
  echo "  cargo build --bin herdr" >&2
}

if [ ! -x "$BIN" ]; then
  build_hint
  exit 1
fi

# An existing binary is not a current one. A binary built before the palette
# change draws the OLD palette — directional leaf rows, "unset" in the key
# column, the untrimmed footer — which reads as a broken feature rather than a
# stale build. Testing -x alone burned a whole session on 2026-09-15.
for src in \
  "$WORKTREE/src/client/shell/palette.rs" \
  "$WORKTREE/src/client/shell/overlays.rs" \
  "$WORKTREE/src/client/shell/palette/input.rs"
do
  if [ ! -f "$src" ]; then
    echo "Expected source file is missing: $src" >&2
    echo "This check cannot tell whether the binary is current, so it refuses" >&2
    echo "rather than pass. Update the path list in this script." >&2
    exit 1
  fi
  if [ "$src" -nt "$BIN" ]; then
    echo "Stale binary: $src is newer than $BIN." >&2
    echo "Every step below would show pre-change behavior." >&2
    build_hint
    exit 1
  fi
done

CONFIG_DIR="$(mktemp -d /tmp/herdr-palette-check.XXXXXX)"
trap 'rm -rf "$CONFIG_DIR"' EXIT
# A debug build reads $XDG_CONFIG_HOME/herdr-dev, not .../herdr — see
# app_dir_name() in src/config/io.rs. Writing the override one level up leaves
# it unread, and step 5 then shows no tag whether the feature works or not.
APP_CONFIG_DIR="$CONFIG_DIR/herdr-dev"
mkdir -p "$APP_CONFIG_DIR"
cat > "$APP_CONFIG_DIR/config.toml" <<'EOF'
# Mark a third-party action destructive without touching its manifest. The entry
# is "<plugin_id>:<action_id>", compared as an exact string by
# action_is_marked_destructive() in src/client/shell/palette.rs — there is no
# normalization, so a near-miss silently matches nothing.
#
# The plugin id is NOT the display name shown in the palette row. Collie renders
# as "Collie" but its id is "herdr.collie". Read the id from `herdr plugin list`
# and replace this entry with a plugin you actually have installed.
[palette]
destructive_actions = ["herdr.collie:uninstall"]

# This check is normally run from a pane inside a running herdr, which the
# nesting guard refuses by default (src/main.rs, should_block_nested). The
# opt-in lives in this throwaway config, so it never reaches the real one.
[experimental]
allow_nested = true
EOF

# Seed the throwaway config with the operator's real plugin registry. The debug
# binary reads its registry from $XDG_CONFIG_HOME/herdr-dev/plugins.json —
# registry_path() is config_dir().join("plugins.json"), and app_dir_name() is
# herdr-dev under debug_assertions. A throwaway config dir has none, so without
# this the run sees zero plugins and step 5 has no row to tag at all, whether or
# not the feature works. Measured 2026-09-15: unseeded prints "No plugins
# installed"; seeded prints all five. The registry stores absolute plugin_root
# paths, so a copy is enough and the real config is only ever read.
REAL_REGISTRY="$HOME/.config/herdr/plugins.json"
if [ ! -f "$REAL_REGISTRY" ]; then
  echo "No plugin registry at $REAL_REGISTRY." >&2
  echo "Step 5 needs an installed plugin to mark destructive, and this script" >&2
  echo "cannot conjure one. Install a plugin, or delete step 5 from the list" >&2
  echo "below and run steps 1-4 only." >&2
  exit 1
fi
command cp "$REAL_REGISTRY" "$APP_CONFIG_DIR/plugins.json"

# Ask the binary where it will actually look, rather than trusting the path
# above. A silently unread override makes step 5 show no tag whether the
# feature works or not, which is the one outcome this script must not produce.
RESOLVED="$(XDG_CONFIG_HOME="$CONFIG_DIR" "$BIN" --help 2>/dev/null | command sed -n 's/^Config: //p')"
if [ "$RESOLVED" != "$APP_CONFIG_DIR/config.toml" ]; then
  echo "This script wrote the override to $APP_CONFIG_DIR/config.toml," >&2
  echo "but the binary reads ${RESOLVED:-<unknown>}. Step 5 would test nothing." >&2
  echo "Fix the path in this script to match app_dir_name() in src/config/io.rs." >&2
  exit 1
fi

# A correctly-read override naming a plugin this run cannot see is the same dead
# end as an unread one: step 5 shows no tag either way. Ask the binary under test
# what it sees, rather than checking a directory — the operator's real plugin
# directory is not what this run reads, so an on-disk check answers a different
# question and passes while step 5 still shows nothing.
OVERRIDE_PLUGIN_ID="$(command sed -n 's/^destructive_actions = \["\([^:]*\):.*/\1/p' "$APP_CONFIG_DIR/config.toml")"
if [ -z "$OVERRIDE_PLUGIN_ID" ]; then
  echo "Could not read a plugin id from this script's override." >&2
  echo "Expected a line of the form:" >&2
  echo "  destructive_actions = [\"<plugin_id>:<action_id>\"]" >&2
  exit 1
fi

# No pipe into grep -q: under pipefail a SIGPIPE'd producer can turn a found
# match into a non-zero status, which would report "not installed" for an
# installed plugin.
INSTALLED_PLUGINS="$(env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH \
  XDG_CONFIG_HOME="$CONFIG_DIR" "$BIN" plugin list 2>/dev/null)"
case "$INSTALLED_PLUGINS" in
  *"- $OVERRIDE_PLUGIN_ID "*) ;;
  *)
    echo "This run's herdr has no plugin with the id '$OVERRIDE_PLUGIN_ID'." >&2
    echo "Step 5 would show no [destructive] tag whether the feature works or not." >&2
    echo "Run 'herdr plugin list' for the real ids — note the id is not the" >&2
    echo "display name shown in the palette row — and correct the override above." >&2
    exit 1
    ;;
esac

cat <<EOF

Config: $APP_CONFIG_DIR/config.toml
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
