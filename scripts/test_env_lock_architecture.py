"""Every test env lock is the crate's one shared lock.

Fork-only (cameronsjo/herdr#74, #36). Tests that change process-wide
environment variables must hold `crate::config::test_config_env_lock()`.
Upstream gives each module its own private `env_lock()`. Per-module locks do
not exclude each other, so a single-process `cargo test` run races. The fork
routes every one through the shared lock, and this test fails when a sync
brings in an `env_lock()` that does not.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

from scripts.test_ui_hot_path_architecture import function_body, mask_comments_and_literals


PROJECT_ROOT = Path(__file__).resolve().parent.parent
# Only files that name an env lock at all; masking every source is slow.
SOURCES = tuple(
    path
    for path in sorted((PROJECT_ROOT / "src").rglob("*.rs"))
    if "env_lock" in path.read_text(encoding="utf-8")
)
SHARED_LOCK = "test_config_env_lock"
SHARED_LOCK_CALL = re.compile(r"\bcrate::config::test_config_env_lock\(\)")
ENV_LOCK_FN = re.compile(r"\bfn\s+(?P<name>\w*env_lock)\s*\(")
# A body that builds a lock of its own is a private lock, even when it also
# calls the shared one.
PRIVATE_LOCK = re.compile(r"\b(?:static|Mutex::new|OnceLock|LazyLock)\b")


def env_lock_violations(source: str) -> list[tuple[int, str]]:
    """(line, name) of each `*env_lock()` that is not the shared lock."""
    code = mask_comments_and_literals(source)
    violations: list[tuple[int, str]] = []
    for match in ENV_LOCK_FN.finditer(code):
        name = match.group("name")
        if name == SHARED_LOCK:
            continue
        body = function_body(code, match.start())
        if SHARED_LOCK_CALL.search(body) and not PRIVATE_LOCK.search(body):
            continue
        violations.append((code.count("\n", 0, match.start()) + 1, name))
    return violations


class EnvLockArchitectureTests(unittest.TestCase):
    def test_every_env_lock_returns_the_shared_lock(self) -> None:
        self.assertTrue(SOURCES, "No Rust source names an env lock")
        violations = [
            f"{path.relative_to(PROJECT_ROOT)}:{line}: {name}()"
            for path in SOURCES
            for line, name in env_lock_violations(path.read_text(encoding="utf-8"))
        ]

        self.assertEqual(
            violations,
            [],
            "Each test env lock must return crate::config::test_config_env_lock():\n"
            + "\n".join(violations),
        )

    def test_the_shared_lock_and_its_callers_are_still_found(self) -> None:
        config = (PROJECT_ROOT / "src" / "config.rs").read_text(encoding="utf-8")
        self.assertRegex(config, r"\bpub\(crate\) fn test_config_env_lock\s*\(")
        found = {
            match.group("name")
            for path in SOURCES
            for match in ENV_LOCK_FN.finditer(
                mask_comments_and_literals(path.read_text(encoding="utf-8"))
            )
        }
        # If these vanish, the scan has stopped seeing the functions it guards.
        self.assertTrue({"env_lock", "integration_env_lock"} <= found, found)

    def test_scanner_rejects_a_private_lock(self) -> None:
        cases = {
            "private static": (
                "fn env_lock() -> &'static Mutex<()> {\n"
                "    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();\n"
                "    LOCK.get_or_init(|| Mutex::new(()))\n"
                "}\n"
            ),
            "shared call plus a private lock": (
                "fn env_lock() -> &'static Mutex<()> {\n"
                "    let _ = crate::config::test_config_env_lock();\n"
                "    static LOCK: Mutex<()> = Mutex::new(());\n"
                "    &LOCK\n"
                "}\n"
            ),
            "same name in another module": (
                "fn remote_env_lock() -> &'static Mutex<()> {\n"
                "    crate::other::test_config_env_lock()\n"
                "}\n"
            ),
            "call only in a comment": (
                "fn env_lock() -> &'static Mutex<()> {\n"
                "    // crate::config::test_config_env_lock()\n"
                "    &LOCK\n"
                "}\n"
            ),
        }
        for label, source in cases.items():
            with self.subTest(label):
                self.assertEqual(len(env_lock_violations(source)), 1)

    def test_scanner_accepts_the_shared_lock(self) -> None:
        cases = (
            "fn env_lock() -> &'static std::sync::Mutex<()> {\n"
            "    crate::config::test_config_env_lock()\n"
            "}\n",
            "pub(crate) fn integration_env_lock() -> IntegrationEnvLock {\n"
            "    let guard = crate::config::test_config_env_lock().lock().unwrap();\n"
            "    IntegrationEnvLock { _guard: guard }\n"
            "}\n",
        )
        for source in cases:
            with self.subTest(source=source):
                self.assertEqual(env_lock_violations(source), [])


if __name__ == "__main__":
    unittest.main()
