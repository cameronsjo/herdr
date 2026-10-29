//! Fork: extra process names for built-in agent kinds (cameronsjo/herdr#110).
//!
//! `[advanced] agent_process_names` maps a kind to more executable names, so a
//! wrapper installed under its own name (say `my-claude` running Claude Code)
//! is identified as that kind and gets its screen rules. The names only widen
//! process identification; `agent start --kind` and other label parsing stay
//! on the built-in names.

use std::collections::{BTreeMap, HashMap};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock, RwLock,
};

use super::Agent;

static EXTRA_NAMES: OnceLock<RwLock<HashMap<String, Agent>>> = OnceLock::new();
// The kernel's TASK_COMM_LEN less its terminating NUL.
const LINUX_COMM_MAX_BYTES: usize = 15;

// Lets the common case (nothing configured) skip the lock on every lookup.
static HAS_EXTRA_NAMES: AtomicBool = AtomicBool::new(false);

// Every `App` sets the process-wide names from its config, so tests that run
// in parallel threads use this per-thread map instead of the global one.
#[cfg(test)]
thread_local! {
    static TEST_NAMES: std::cell::RefCell<HashMap<String, Agent>> =
        std::cell::RefCell::new(HashMap::new());
}

/// Validates the configured names. Returns the usable name-to-kind map and one
/// diagnostic per entry that was dropped.
pub(crate) fn extra_process_names(
    config: &BTreeMap<String, Vec<String>>,
) -> (HashMap<String, Agent>, Vec<String>) {
    let mut names = HashMap::new();
    let mut diagnostics = Vec::new();
    for (kind, entries) in config {
        let Some(agent) = super::parse_canonical_agent_label(&kind.to_lowercase()) else {
            diagnostics.push(format!(
                "unknown agent kind `{kind}` in [advanced] agent_process_names; ignoring its names"
            ));
            continue;
        };
        for entry in entries {
            let name = super::normalized_agent_lookup_name(entry);
            if name.is_empty() || name.starts_with('-') || name.contains(['/', '\\']) {
                diagnostics.push(format!(
                    "invalid process name `{entry}` for `{kind}` in [advanced] agent_process_names; use a bare executable name"
                ));
                continue;
            }
            if is_shell_or_launcher(&name) {
                diagnostics.push(format!(
                    "process name `{entry}` in [advanced] agent_process_names is a shell, runtime or launcher; ignoring it for `{kind}`"
                ));
                continue;
            }
            match super::parse_agent_label(&name) {
                Some(builtin) if builtin == agent => continue,
                Some(builtin) => {
                    diagnostics.push(format!(
                        "process name `{entry}` in [advanced] agent_process_names already names `{}`; ignoring it for `{kind}`",
                        super::agent_label(builtin)
                    ));
                    continue;
                }
                None => {}
            }
            match names.get(&name) {
                Some(&existing) if existing != agent => diagnostics.push(format!(
                    "process name `{entry}` in [advanced] agent_process_names is listed under `{}` and `{kind}`; keeping `{}`",
                    super::agent_label(existing),
                    super::agent_label(existing)
                )),
                Some(_) => {}
                None => {
                    // Linux reports a process name cut to 15 bytes, so a longer
                    // configured name also matches its cut form.
                    let cut = name
                        .get(..LINUX_COMM_MAX_BYTES)
                        .filter(|cut| cut.len() < name.len())
                        .filter(|cut| super::parse_agent_label(cut).is_none());
                    if let Some(cut) = cut {
                        names.entry(cut.to_string()).or_insert(agent);
                    }
                    names.insert(name, agent);
                }
            }
        }
    }
    (names, diagnostics)
}

/// Names that also run things other than an agent. Claiming one would make a
/// pane's shell, or a dev server, count as the agent, so name-routed input
/// such as `agent prompt` would be typed into it.
fn is_shell_or_launcher(name: &str) -> bool {
    super::is_generic_runtime_or_shell(name)
        || matches!(
            name,
            "dash"
                | "ksh"
                | "mksh"
                | "csh"
                | "tcsh"
                | "nu"
                | "xonsh"
                | "elvish"
                | "screen"
                | "deno"
                | "npx"
                | "npm"
                | "pnpm"
                | "yarn"
                | "uv"
                | "uvx"
                | "pipx"
                | "bunx"
                | "env"
                | "sudo"
                | "doas"
                | "ssh"
                | "login"
                | "su"
        )
}

/// Replaces the extra names used by process identification. Invalid entries
/// are dropped; [`extra_process_names`] reports them.
pub(crate) fn set_extra_process_names(config: &BTreeMap<String, Vec<String>>) {
    let (names, _) = extra_process_names(config);
    let has_names = !names.is_empty();
    let lock = EXTRA_NAMES.get_or_init(|| RwLock::new(HashMap::new()));
    match lock.write() {
        Ok(mut current) => *current = names,
        Err(poisoned) => *poisoned.into_inner() = names,
    }
    HAS_EXTRA_NAMES.store(has_names, Ordering::Release);
}

/// The kind a configured extra name belongs to, matched on the executable's
/// basename the same way as the built-in names.
pub(crate) fn lookup_extra_process_name(process_name: &str) -> Option<Agent> {
    #[cfg(test)]
    if let Some(agent) = TEST_NAMES.with(|names| {
        let name = super::normalized_agent_lookup_name(process_name);
        names.borrow().get(super::path_basename(&name)).copied()
    }) {
        return Some(agent);
    }
    if !HAS_EXTRA_NAMES.load(Ordering::Acquire) {
        return None;
    }
    let name = super::normalized_agent_lookup_name(process_name);
    let name = super::path_basename(&name);
    let lock = EXTRA_NAMES.get()?;
    let names = match lock.read() {
        Ok(names) => names,
        Err(poisoned) => poisoned.into_inner(),
    };
    names.get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(entries: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        entries
            .iter()
            .map(|(kind, names)| {
                (
                    (*kind).to_string(),
                    names.iter().map(|name| (*name).to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn extra_process_names_maps_names_to_their_kind() {
        let (names, diagnostics) =
            extra_process_names(&config(&[("claude", &["My-Claude", "cc-wrap.exe"])]));
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(names.get("my-claude"), Some(&Agent::Claude));
        assert_eq!(names.get("cc-wrap"), Some(&Agent::Claude));
    }

    #[test]
    fn extra_process_names_reports_and_drops_bad_entries() {
        let (names, diagnostics) = extra_process_names(&config(&[
            (
                "claude",
                &["codex", "", "bin/cc", "-zsh", "claude-code", "shared"],
            ),
            ("codex", &["shared"]),
            ("not-a-kind", &["whatever"]),
        ]));
        assert_eq!(names.len(), 1, "{names:?}");
        assert_eq!(names.get("shared"), Some(&Agent::Claude));
        let joined = diagnostics.join("\n");
        assert!(joined.contains("`codex` in [advanced] agent_process_names already names `codex`"));
        assert!(joined.contains("invalid process name `` for `claude`"));
        assert!(joined.contains("invalid process name `bin/cc` for `claude`"));
        assert!(joined.contains("invalid process name `-zsh` for `claude`"));
        assert!(joined.contains("listed under `claude` and `codex`; keeping `claude`"));
        assert!(joined.contains("unknown agent kind `not-a-kind`"));
        assert!(
            !joined.contains("claude-code"),
            "a built-in alias of the same kind is accepted silently"
        );
        assert_eq!(diagnostics.len(), 6, "{diagnostics:?}");
    }

    #[test]
    fn extra_process_names_match_the_linux_cut_name_and_any_kind_case() {
        let (names, diagnostics) =
            extra_process_names(&config(&[("Claude", &["claude-wrapper-v2"])]));
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(names.get("claude-wrapper-v2"), Some(&Agent::Claude));
        assert_eq!(names.get("claude-wrapper-"), Some(&Agent::Claude));
    }

    #[test]
    fn extra_process_names_match_a_wrapper_run_by_an_interpreter() {
        let name = "herdr-test-wrapper";
        let (names, _) = extra_process_names(&config(&[("claude", &[name])]));
        TEST_NAMES.with(|current| *current.borrow_mut() = names);
        let job = crate::platform::ForegroundJob {
            process_group_id: 41,
            processes: vec![crate::platform::ForegroundProcess {
                pid: 41,
                name: "bash".into(),
                argv0: Some("bash".into()),
                argv: Some(vec!["bash".into(), format!("/opt/bin/{name}")]),
                cmdline: Some(format!("bash /opt/bin/{name}")),
            }],
        };
        let identified = crate::detect::identify_agent_in_job(&job).map(|(agent, _)| agent);
        TEST_NAMES.with(|current| current.borrow_mut().clear());
        assert_eq!(identified, Some(Agent::Claude));
    }

    #[test]
    fn extra_process_names_refuses_shells_runtimes_and_launchers() {
        let (names, diagnostics) = extra_process_names(&config(&[(
            "claude",
            &[
                "zsh",
                "node",
                "python3",
                "pwsh.exe",
                "sudo",
                "nu",
                "my-claude",
            ],
        )]));
        assert_eq!(names.len(), 1, "{names:?}");
        assert_eq!(names.get("my-claude"), Some(&Agent::Claude));
        assert_eq!(diagnostics.len(), 6, "{diagnostics:?}");
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.contains("is a shell, runtime or launcher")));
    }

    #[test]
    fn extra_process_names_identify_the_configured_kind() {
        let name = "herdr-test-extra-process-name";
        assert_eq!(crate::detect::identify_agent(name), None);
        let (names, _) = extra_process_names(&config(&[("claude", &[name])]));
        TEST_NAMES.with(|current| *current.borrow_mut() = names);
        let identified =
            crate::detect::identify_agent(&format!("/opt/bin/{}", name.to_uppercase()));
        let builtin = crate::detect::parse_agent_label(name);
        TEST_NAMES.with(|current| current.borrow_mut().clear());
        assert_eq!(identified, Some(Agent::Claude));
        assert_eq!(builtin, None, "label parsing stays on built-in names");
        assert_eq!(crate::detect::identify_agent(name), None);
    }

    #[test]
    fn agent_process_names_config_parses_and_reports_unknown_kinds() {
        let config: crate::config::Config = toml::from_str(
            "[advanced]\nagent_process_names = { claude = [\"my-claude\"], nope = [\"x\"] }\n",
        )
        .unwrap();
        assert_eq!(
            config.advanced.agent_process_names.get("claude"),
            Some(&vec!["my-claude".to_string()])
        );
        let diagnostics = config.collect_diagnostics();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("unknown agent kind `nope`")),
            "{diagnostics:?}"
        );
    }
}
