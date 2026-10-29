//! Fork-only `--parent NAME` on `herdr agent start`: records which agent
//! launched which, as the pane metadata token `parent`.
//!
//! The started agent's pane gets `parent=NAME`. The parent's own pane gets
//! `parent=NAME` too unless it already has one, so `group_by = { token =
//! "parent" }` gathers an orchestrator and its workers into one group without
//! any new server state. The token rides the existing metadata path, so the
//! API and protocol are unchanged.

use std::collections::HashMap;

use crate::api::schema::{AgentTarget, Method, PaneReportMetadataParams, Request};

/// Token key that names the agent that launched this one.
pub(crate) const PARENT_TOKEN: &str = "parent";
const PARENT_SOURCE: &str = "herdr:parent";

/// Splits `--parent NAME` or `--parent=NAME` out of `agent start` arguments.
/// Only arguments before `--` are options; everything after is the agent's.
/// This relies on no `start` option taking a hyphen-leading value; one that
/// does (`allow_hyphen_values`) needs this revisited.
pub(super) fn take_parent(args: &[String]) -> Result<(Vec<String>, Option<String>), String> {
    let mut rest = Vec::with_capacity(args.len());
    let mut parent = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--" {
            rest.push(arg.clone());
            rest.extend(iter.by_ref().cloned());
            break;
        }
        let value = if arg == "--parent" {
            Some(
                iter.next()
                    .cloned()
                    .ok_or_else(|| "missing value for --parent".to_owned())?,
            )
        } else {
            arg.strip_prefix("--parent=").map(str::to_owned)
        };
        match value {
            Some(_) if parent.is_some() => return Err("--parent given more than once".into()),
            Some(value) if !crate::label::valid_agent_name(&value) => {
                return Err(format!(
                    "invalid --parent name: {value} (expected [a-z][a-z0-9_-]{{0,31}})"
                ));
            }
            Some(value) => parent = Some(value),
            None => rest.push(arg.clone()),
        }
    }
    Ok((rest, parent))
}

/// The parent agent, resolved before anything is typed into the new pane.
pub(super) struct ParentLink {
    name: String,
    pane_id: String,
    /// The parent already carries a `parent` token (it has a parent of its
    /// own, or an earlier start set it), which is left alone.
    has_token: bool,
}

/// Looks the parent up with `agent.get`. `Ok(Err(response))` is the error to
/// print when it is not a live agent.
pub(super) fn resolve_parent(name: &str) -> std::io::Result<Result<ParentLink, serde_json::Value>> {
    let response = super::send_request(&Request {
        id: "cli:agent:start".into(),
        method: Method::AgentGet(AgentTarget {
            target: name.to_owned(),
        }),
    })?;
    let agent = &response["result"]["agent"];
    let Some(pane_id) = agent["pane_id"].as_str() else {
        return Ok(Err(response));
    };
    Ok(Ok(ParentLink {
        name: name.to_owned(),
        pane_id: pane_id.to_owned(),
        has_token: agent["tokens"]
            .get(PARENT_TOKEN)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty()),
    }))
}

fn parent_params(pane_id: &str, parent: &str) -> PaneReportMetadataParams {
    PaneReportMetadataParams {
        pane_id: pane_id.to_owned(),
        source: PARENT_SOURCE.into(),
        agent: None,
        applies_to_source: None,
        title: None,
        display_agent: None,
        state_labels: HashMap::new(),
        tokens: HashMap::from([(PARENT_TOKEN.to_owned(), Some(parent.to_owned()))]),
        clear_title: false,
        clear_display_agent: false,
        clear_state_labels: false,
        seq: None,
        ttl_ms: None,
    }
}

/// The reports `record_parent` sends: the child first, then the parent's own
/// pane when it has no `parent` token yet.
fn parent_reports(link: &ParentLink, child_pane_id: &str) -> Vec<PaneReportMetadataParams> {
    let mut reports = vec![parent_params(child_pane_id, &link.name)];
    if !link.has_token {
        reports.push(parent_params(&link.pane_id, &link.name));
    }
    reports
}

/// Records the link once the child is running. Returns the first error
/// response, if any.
pub(super) fn record_parent(
    link: &ParentLink,
    child_pane_id: &str,
) -> std::io::Result<Option<serde_json::Value>> {
    for params in parent_reports(link, child_pane_id) {
        let response = super::send_request(&Request {
            id: "cli:agent:start".into(),
            method: Method::PaneReportMetadata(params),
        })?;
        if response.get("error").is_some() {
            return Ok(Some(response));
        }
    }
    Ok(None)
}

/// The error object to attach to a successful start when recording the
/// parent failed, whether the server refused a report or the socket did.
pub(super) fn parent_record_failure(
    recorded: std::io::Result<Option<serde_json::Value>>,
) -> Option<serde_json::Value> {
    match recorded {
        Ok(None) => None,
        Ok(Some(response)) => Some(response["error"].clone()),
        Err(err) => Some(serde_json::json!({
            "code": "parent_record_transport_failed",
            "message": err.to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parent_is_taken_out_of_start_options_only() {
        assert_eq!(
            take_parent(&args(&[
                "start", "w1", "--parent", "orch", "--kind", "pi", "--", "--parent", "x"
            ])),
            Ok((
                args(&["start", "w1", "--kind", "pi", "--", "--parent", "x"]),
                Some("orch".into())
            ))
        );
        assert_eq!(
            take_parent(&args(&["start", "w1", "--parent=orch"])),
            Ok((args(&["start", "w1"]), Some("orch".into())))
        );
        assert_eq!(
            take_parent(&args(&["start", "w1", "--kind", "pi"])),
            Ok((args(&["start", "w1", "--kind", "pi"]), None))
        );
    }

    #[test]
    fn parent_rejects_bad_values() {
        for values in [
            vec!["start", "w1", "--parent"],
            vec!["start", "w1", "--parent", "Orch"],
            vec!["start", "w1", "--parent="],
            vec!["start", "w1", "--parent", "a", "--parent", "b"],
        ] {
            assert!(take_parent(&args(&values)).is_err(), "{values:?}");
        }
    }

    #[test]
    fn parent_reports_tag_the_child_and_an_untagged_parent() {
        let link = |has_token| ParentLink {
            name: "orch".into(),
            pane_id: "p_1_1".into(),
            has_token,
        };
        let panes = |link: &ParentLink| {
            parent_reports(link, "p_1_2")
                .into_iter()
                .map(|params| {
                    assert_eq!(params.source, "herdr:parent");
                    assert_eq!(
                        params.tokens,
                        HashMap::from([("parent".to_owned(), Some("orch".to_owned()))])
                    );
                    params.pane_id
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(panes(&link(false)), vec!["p_1_2", "p_1_1"]);
        assert_eq!(panes(&link(true)), vec!["p_1_2"]);
    }

    #[test]
    fn parent_record_failure_keeps_the_error_without_failing_the_start() {
        assert_eq!(parent_record_failure(Ok(None)), None);
        let refused = serde_json::json!({
            "id": "cli:agent:start",
            "error": {"code": "pane_not_found", "message": "pane p_1_1 not found"},
        });
        assert_eq!(
            parent_record_failure(Ok(Some(refused))),
            Some(serde_json::json!({"code": "pane_not_found", "message": "pane p_1_1 not found"}))
        );
        let dropped = parent_record_failure(Err(std::io::Error::other("socket closed")))
            .expect("a transport error is reported");
        assert_eq!(dropped["code"], "parent_record_transport_failed");
        assert_eq!(dropped["message"], "socket closed");
    }
}
