//! Fork-only `herdr agent task` command: sets the one-line task an agent's
//! sidebar entry shows under its name.
//!
//! The task is the pane metadata token `task`, so it rides the existing token
//! path (`$task` in sidebar rows, `tokens.task` in `agent get`). Pane-less
//! registered agents set it with `agent register --token task=...` instead.

use std::collections::HashMap;

use clap::{Arg, ArgAction, Command};

use crate::api::schema::{AgentTarget, Method, PaneReportMetadataParams, Request};

pub(super) const TASK_USAGE: &str =
    "herdr agent task <TEXT> [--target TARGET]\n       herdr agent task --clear [--target TARGET]";
use crate::config::AGENT_TASK_TOKEN as TASK_TOKEN;
/// Metadata source for every task report, so one source owns the token's
/// report sequence no matter which process sets it.
const TASK_SOURCE: &str = "herdr:task";

/// The clap shape of this command, for `herdr agent --help` and completions.
/// Parsing stays with `parse_task_args`.
pub(super) fn command() -> Command {
    Command::new("task")
        .about("Set or clear the one-line task an agent's sidebar entry shows")
        .arg(Arg::new("text").value_name("TEXT"))
        .arg(
            Arg::new("target")
                .long("target")
                .value_name("TARGET")
                .action(ArgAction::Set),
        )
        .arg(Arg::new("clear").long("clear").action(ArgAction::SetTrue))
}

#[derive(Debug, PartialEq, Eq)]
enum TaskTarget {
    /// The calling pane, from `HERDR_PANE_ID`.
    OwnPane(String),
    /// An agent name or pane id given with `--target`, resolved through `agent.get`.
    Agent(String),
}

#[derive(Debug, PartialEq, Eq)]
struct TaskArgs {
    target: TaskTarget,
    /// `None` clears the task.
    text: Option<String>,
}

fn parse_task_args(args: &[String], own_pane: Option<String>) -> Result<TaskArgs, String> {
    let mut text = None;
    let mut target = None;
    let mut clear = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--clear" => clear = true,
            "--target" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("missing value for --target".into());
                };
                target = Some(value.clone());
                index += 1;
            }
            flag if flag.starts_with("--") => return Err(format!("unknown option: {flag}")),
            value if text.is_none() => text = Some(value.to_owned()),
            _ => return Err("expected one TEXT argument; quote a task with spaces".into()),
        }
        index += 1;
    }
    let text = match (clear, text) {
        (true, Some(_)) => return Err("--clear takes no TEXT".into()),
        (true, None) => None,
        (false, Some(text)) if !text.trim().is_empty() => Some(text),
        (false, _) => return Err("missing TEXT (use --clear to remove the task)".into()),
    };
    let target = match (target, own_pane.filter(|pane| !pane.is_empty())) {
        (Some(target), _) => TaskTarget::Agent(target),
        (None, Some(pane)) => TaskTarget::OwnPane(pane),
        (None, None) => {
            return Err("not inside a herdr pane; pass --target TARGET".into());
        }
    };
    Ok(TaskArgs { target, text })
}

fn task_params(pane_id: String, text: Option<String>) -> PaneReportMetadataParams {
    PaneReportMetadataParams {
        pane_id,
        source: TASK_SOURCE.into(),
        agent: None,
        applies_to_source: None,
        title: None,
        display_agent: None,
        state_labels: HashMap::new(),
        tokens: HashMap::from([(TASK_TOKEN.to_owned(), text)]),
        clear_title: false,
        clear_display_agent: false,
        clear_state_labels: false,
        seq: None,
        ttl_ms: None,
    }
}

pub(super) fn agent_task(args: &[String]) -> std::io::Result<i32> {
    let own_pane = std::env::var(crate::integration::HERDR_PANE_ID_ENV_VAR).ok();
    let parsed = match parse_task_args(args, own_pane) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}\nusage: {TASK_USAGE}");
            return Ok(2);
        }
    };
    let pane_id = match parsed.target {
        TaskTarget::OwnPane(pane_id) => pane_id,
        TaskTarget::Agent(target) => {
            let response = super::send_request(&Request {
                id: "cli:agent:task".into(),
                method: Method::AgentGet(AgentTarget { target }),
            })?;
            match response
                .pointer("/result/agent/pane_id")
                .and_then(serde_json::Value::as_str)
            {
                Some(pane_id) => pane_id.to_owned(),
                None => return super::print_response(&response),
            }
        }
    };
    super::send_ok_request(Method::PaneReportMetadata(task_params(
        pane_id,
        parsed.text,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn task_defaults_to_the_calling_pane() {
        assert_eq!(
            parse_task_args(&args(&["fixing homelab#1093"]), Some("p_1_2".into())),
            Ok(TaskArgs {
                target: TaskTarget::OwnPane("p_1_2".into()),
                text: Some("fixing homelab#1093".into()),
            })
        );
    }

    #[test]
    fn task_target_flag_wins_over_the_calling_pane() {
        assert_eq!(
            parse_task_args(
                &args(&["--target", "worker-1", "review"]),
                Some("p_1_2".into())
            ),
            Ok(TaskArgs {
                target: TaskTarget::Agent("worker-1".into()),
                text: Some("review".into()),
            })
        );
    }

    #[test]
    fn task_clear_sends_no_text() {
        assert_eq!(
            parse_task_args(&args(&["--clear"]), Some("p_1_2".into())),
            Ok(TaskArgs {
                target: TaskTarget::OwnPane("p_1_2".into()),
                text: None,
            })
        );
    }

    #[test]
    fn task_rejects_bad_shapes() {
        let pane = || Some("p_1_2".to_owned());
        for (values, own_pane) in [
            (vec![], pane()),
            (vec!["  "], pane()),
            (vec!["--clear", "text"], pane()),
            (vec!["one", "two"], pane()),
            (vec!["--target"], pane()),
            (vec!["--bogus", "text"], pane()),
            (vec!["text"], None),
            (vec!["text"], Some(String::new())),
        ] {
            assert!(
                parse_task_args(&args(&values), own_pane.clone()).is_err(),
                "{values:?} {own_pane:?}"
            );
        }
    }

    #[test]
    fn task_params_patch_only_the_task_token() {
        let json = serde_json::to_value(Request {
            id: "cli:request".into(),
            method: Method::PaneReportMetadata(task_params("p_1_2".into(), Some("review".into()))),
        })
        .unwrap();
        assert_eq!(json["method"], "pane.report_metadata");
        assert_eq!(json["params"]["source"], "herdr:task");
        assert_eq!(
            json["params"]["tokens"],
            serde_json::json!({"task": "review"})
        );
        let cleared = serde_json::to_value(task_params("p_1_2".into(), None)).unwrap();
        assert_eq!(cleared["tokens"], serde_json::json!({"task": null}));
    }
}
