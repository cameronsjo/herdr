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
ALL_SOURCES = {
    path: path.read_text(encoding="utf-8")
    for path in sorted((PROJECT_ROOT / "src").rglob("*.rs"))
}
# Only files that name an env lock at all; masking every source is slow.
SOURCES = tuple(path for path, text in ALL_SOURCES.items() if "env_lock" in text)
SHARED_LOCK = "test_config_env_lock"
SHARED_LOCK_CALL = re.compile(r"\bcrate::config::test_config_env_lock\(\)")
ENV_LOCK_FN = re.compile(r"\bfn\s+(?P<name>\w*env_lock)\s*\(")
# A `static` declaration, not the word inside a `'static` lifetime.
STATIC_DECL = r"(?<!')\bstatic\s+(?:mut\s+)?"
# A body that builds a lock of its own is a private lock, even when it also
# calls the shared one.
PRIVATE_LOCK = re.compile(STATIC_DECL + r"[A-Za-z_]|\b(?:Mutex::new|OnceLock|LazyLock)\b")
# A lock declared outside the body. A wrapper that names one can call the
# shared lock and still hand back its own.
STATIC_LOCK = re.compile(
    STATIC_DECL + r"(?P<name>[A-Za-z_]\w*)\s*:[^=;]*\b(?:Mutex|RwLock|OnceLock|LazyLock)\b"
)


def static_lock_names(source: str) -> frozenset[str]:
    return frozenset(match.group("name") for match in STATIC_LOCK.finditer(source))


# Every static lock in src/, so a wrapper returning `&super::LOCK` is caught too.
STATIC_LOCK_NAMES = frozenset().union(*map(static_lock_names, ALL_SOURCES.values()))


def env_lock_violations(
    source: str, static_locks: frozenset[str] | None = None
) -> list[tuple[int, str]]:
    """(line, name) of each `*env_lock()` that is not the shared lock.

    `static_locks` names the static locks a wrapper must not touch; it
    defaults to the ones declared in `source`.
    """
    code = mask_comments_and_literals(source)
    if static_locks is None:
        static_locks = static_lock_names(code)
    names_a_static_lock = (
        re.compile(r"\b(?:" + "|".join(map(re.escape, sorted(static_locks))) + r")\b")
        if static_locks
        else None
    )
    violations: list[tuple[int, str]] = []
    for match in ENV_LOCK_FN.finditer(code):
        name = match.group("name")
        if name == SHARED_LOCK:
            continue
        body = function_body(code, match.start())
        if (
            SHARED_LOCK_CALL.search(body)
            and not PRIVATE_LOCK.search(body)
            and not (names_a_static_lock and names_a_static_lock.search(body))
        ):
            continue
        violations.append((code.count("\n", 0, match.start()) + 1, name))
    return violations


class EnvLockArchitectureTests(unittest.TestCase):
    def test_every_env_lock_returns_the_shared_lock(self) -> None:
        self.assertTrue(SOURCES, "No Rust source names an env lock")
        violations = [
            f"{path.relative_to(PROJECT_ROOT)}:{line}: {name}()"
            for path in SOURCES
            for line, name in env_lock_violations(ALL_SOURCES[path], STATIC_LOCK_NAMES)
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
            for match in ENV_LOCK_FN.finditer(mask_comments_and_literals(ALL_SOURCES[path]))
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
            "shared call, then a file-level lock returned": (
                "static LOCK: Mutex<()> = Mutex::new(());\n"
                "fn env_lock() -> &'static Mutex<()> {\n"
                "    let _ = crate::config::test_config_env_lock();\n"
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

    def test_scanner_rejects_a_static_lock_from_another_file(self) -> None:
        source = (
            "fn env_lock() -> &'static Mutex<()> {\n"
            "    let _shared = crate::config::test_config_env_lock();\n"
            "    super::ENV_LOCK.get_or_init(Default::default)\n"
            "}\n"
        )
        self.assertEqual(env_lock_violations(source), [])
        self.assertEqual(len(env_lock_violations(source, frozenset({"ENV_LOCK"}))), 1)

    def test_static_lock_names_skip_lifetimes(self) -> None:
        source = (
            "static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();\n"
            "static mut COUNTS: RwLock<u32> = RwLock::new(0);\n"
            "static NAMES: &[&str] = &[];\n"
            "fn f() -> &'static Mutex<()> { todo!() }\n"
        )
        self.assertEqual(static_lock_names(source), {"ENV_LOCK", "COUNTS"})

    def test_scanner_accepts_the_shared_lock(self) -> None:
        cases = (
            "fn env_lock() -> &'static std::sync::Mutex<()> {\n"
            "    crate::config::test_config_env_lock()\n"
            "}\n",
            "fn env_lock() -> &'static Mutex<()> {\n"
            "    let lock: &'static Mutex<()> = crate::config::test_config_env_lock();\n"
            "    lock\n"
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
