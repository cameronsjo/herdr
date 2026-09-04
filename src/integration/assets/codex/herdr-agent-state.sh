#!/bin/sh
# installed by herdr
# managed by herdr; reinstalling or updating the integration overwrites this file.
# add custom hooks beside this file instead of editing it.
# HERDR_INTEGRATION_ID=codex
# HERDR_INTEGRATION_VERSION=9

set -eu

action="${1:-}"
hook_input_file="$(mktemp "${TMPDIR:-/tmp}/herdr-codex-hook.XXXXXX")" || exit 0
trap 'rm -f "$hook_input_file"' EXIT HUP INT TERM
cat >"$hook_input_file" 2>/dev/null || true

case "$action" in
  session|working|blocked|idle|metadata) ;;
  *) exit 0 ;;
esac

[ "${HERDR_ENV:-}" = "1" ] || exit 0
[ -n "${HERDR_SOCKET_PATH:-}" ] || exit 0
[ -n "${HERDR_PANE_ID:-}" ] || exit 0
command -v python3 >/dev/null 2>&1 || exit 0

HERDR_ACTION="$action" HERDR_HOOK_INPUT_FILE="$hook_input_file" python3 - <<'PY'
import json
import os
import queue
import random
import re
import shutil
import socket
import subprocess
import threading
import time

SOURCE = "herdr:codex"
AGENT = "codex"
EXPECTED_EVENTS = {
    "session": {"SessionStart"},
    "working": {"UserPromptSubmit", "PreToolUse", "PostToolUse"},
    "blocked": {"PermissionRequest"},
    "idle": {"Stop"},
    "metadata": {"SessionStart", "UserPromptSubmit", "Stop"},
}


def send_request(method, params):
    request = {
        "id": f"{SOURCE}:{int(time.time() * 1000)}:{random.randrange(1_000_000):06d}",
        "method": method,
        "params": params,
    }
    try:
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.settimeout(0.5)
        client.connect(socket_path)
        client.sendall((json.dumps(request) + "\n").encode())
        try:
            client.recv(4096)
        except Exception:
            pass
        client.close()
    except Exception:
        pass


def read_responses(process, responses):
    for line in process.stdout:
        try:
            responses.put(json.loads(line))
        except Exception:
            continue


def read_response(responses, response_id, deadline):
    while time.monotonic() < deadline:
        remaining = max(0.0, deadline - time.monotonic())
        try:
            message = responses.get(timeout=remaining)
        except queue.Empty:
            return None
        if message.get("id") == response_id:
            return message
    return None


def read_thread_title(thread_id):
    codex_bin = os.environ.get("HERDR_CODEX_BIN_PATH") or shutil.which("codex")
    if not codex_bin:
        return None
    process = None
    try:
        process = subprocess.Popen(
            [codex_bin, "app-server", "--stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
        )
        responses = queue.Queue()
        threading.Thread(
            target=read_responses, args=(process, responses), daemon=True
        ).start()
        deadline = time.monotonic() + 2.5
        initialize = {
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {"name": "herdr", "title": "Herdr", "version": "0.9"}
            },
        }
        process.stdin.write(json.dumps(initialize) + "\n")
        process.stdin.flush()
        if read_response(responses, 1, deadline) is None:
            return None
        process.stdin.write(json.dumps({"method": "initialized", "params": {}}) + "\n")
        process.stdin.write(
            json.dumps(
                {
                    "id": 2,
                    "method": "thread/read",
                    "params": {"threadId": thread_id, "includeTurns": False},
                }
            )
            + "\n"
        )
        process.stdin.flush()
        response = read_response(responses, 2, deadline)
        if not response:
            return None
        thread = (response.get("result") or {}).get("thread") or {}
        return thread.get("name") or thread.get("preview")
    except Exception:
        return None
    finally:
        if process is not None:
            try:
                process.terminate()
                process.wait(timeout=0.2)
            except Exception:
                try:
                    process.kill()
                except Exception:
                    pass


def normalize_title(value):
    if not isinstance(value, str):
        return None
    value = re.sub(r"^\s*(?:\[Image #\d+\]\s*)+", "", value)
    value = " ".join(value.split())
    if not value:
        return None
    if len(value) > 80:
        value = value[:77].rstrip() + "..."
    return value


action = os.environ.get("HERDR_ACTION", "")
pane_id = os.environ.get("HERDR_PANE_ID")
socket_path = os.environ.get("HERDR_SOCKET_PATH")
hook_input_file = os.environ.get("HERDR_HOOK_INPUT_FILE")
if not pane_id or not socket_path or action not in EXPECTED_EVENTS:
    raise SystemExit(0)

try:
    with open(hook_input_file, encoding="utf-8") as handle:
        hook_input = json.load(handle)
except Exception:
    raise SystemExit(0)

hook_event_name = str(hook_input.get("hook_event_name") or "")
if hook_event_name not in EXPECTED_EVENTS[action]:
    raise SystemExit(0)

session_id = hook_input.get("session_id")
transcript_path = hook_input.get("transcript_path")
if not isinstance(session_id, str) or not session_id:
    raise SystemExit(0)
if not isinstance(transcript_path, str) or not transcript_path.strip():
    raise SystemExit(0)
inherited_session_id = os.environ.get("CODEX_THREAD_ID")
if inherited_session_id and inherited_session_id != session_id:
    raise SystemExit(0)

report_seq = time.time_ns()
common_params = {
    "pane_id": pane_id,
    "source": SOURCE,
    "agent": AGENT,
    "seq": report_seq,
    "agent_session_id": session_id,
}

if action == "session":
    session_start_source = hook_input.get("source")
    if isinstance(session_start_source, str) and session_start_source:
        common_params["session_start_source"] = session_start_source
    send_request("pane.report_agent_session", common_params)
elif action == "metadata":
    title = normalize_title(read_thread_title(session_id))
    if title is None and hook_event_name == "UserPromptSubmit":
        title = normalize_title(hook_input.get("prompt"))
    if title is not None:
        send_request(
            "pane.report_metadata",
            {
                "pane_id": pane_id,
                "source": SOURCE,
                "agent": AGENT,
                "seq": report_seq,
                "title": title,
            },
        )
else:
    common_params["state"] = action
    send_request("pane.report_agent", common_params)
PY
