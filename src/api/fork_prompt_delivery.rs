//! Fork: decide whether `agent.prompt` text reached the agent's input
//! (cameronsjo/herdr#102).
//!
//! Writing the bytes into the pane only proves herdr sent them. A busy agent
//! can hold a prompt without any visible change for a while, and a caller that
//! takes that silence for a lost prompt resends it, so the agent gets two
//! copies. After the write, this watches the agent's status and the bottom of
//! its screen for a short window and reports what it saw.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::api::schema::{
    AgentInfo, AgentStatus, Method, PaneReadParams, PaneReadResult, PromptDelivery, ReadFormat,
    ReadIntent, ReadSource, Request,
};
use crate::api::server::{dispatch_to_app_with_timeout, APP_RESPONSE_TIMEOUT};
use crate::api::ApiRequestSender;

/// How long to look for evidence after the write completes.
pub(super) const PROMPT_DELIVERY_WINDOW: Duration = Duration::from_millis(1_500);
const PROMPT_DELIVERY_POLL: Duration = Duration::from_millis(100);
/// Characters of the prompt's first line to look for on screen. Short enough
/// to fit an input box row, long enough not to match unrelated text.
const ECHO_NEEDLE_CHARS: usize = 32;
/// A shorter first line ("y", "ok") matches too much incidental text to count.
const ECHO_NEEDLE_MIN_CHARS: usize = 4;

/// The agent's state just before the prompt was written.
pub(super) struct DeliveryBaseline {
    pane_id: String,
    busy: bool,
    state_change_seq: u64,
    needle: Option<String>,
    echoes: usize,
}

impl DeliveryBaseline {
    pub(super) fn capture(
        request_id: &str,
        before: &AgentInfo,
        text: &str,
        api_tx: &ApiRequestSender,
    ) -> Self {
        let needle = echo_needle(text);
        let echoes = match &needle {
            Some(needle) => read_detection_text(request_id, &before.pane_id, api_tx)
                .map(|screen| count_echoes(&screen, needle))
                .unwrap_or(0),
            None => 0,
        };
        Self {
            pane_id: before.pane_id.clone(),
            busy: before.agent_status == AgentStatus::Working,
            state_change_seq: before.state_change_seq,
            needle,
            echoes,
        }
    }

    /// Polls until there is evidence or `deadline` passes.
    pub(super) fn observe(
        &self,
        request_id: &str,
        target: &str,
        api_tx: &ApiRequestSender,
        running: &Arc<AtomicBool>,
        deadline: Instant,
    ) -> PromptDelivery {
        loop {
            let went_working = !self.busy
                && agent_get(request_id, target, api_tx).is_some_and(|agent| {
                    agent.state_change_seq != self.state_change_seq
                        && matches!(
                            agent.agent_status,
                            AgentStatus::Working | AgentStatus::Blocked
                        )
                });
            let echoed = !went_working
                && self.needle.as_deref().is_some_and(|needle| {
                    read_detection_text(request_id, &self.pane_id, api_tx)
                        .is_some_and(|screen| count_echoes(&screen, needle) > self.echoes)
                });
            if let Some(delivery) = classify(self.busy, went_working, echoed) {
                return delivery;
            }
            let now = Instant::now();
            if now >= deadline || !running.load(Ordering::Relaxed) {
                return PromptDelivery::Unknown;
            }
            std::thread::sleep(PROMPT_DELIVERY_POLL.min(deadline - now));
        }
    }
}

/// `None` means no evidence yet.
fn classify(busy: bool, went_working: bool, echoed: bool) -> Option<PromptDelivery> {
    match (busy, went_working || echoed) {
        (_, false) => None,
        (false, true) => Some(PromptDelivery::Typed),
        (true, true) => Some(PromptDelivery::Queued),
    }
}

/// The start of the prompt's first non-blank line, whitespace collapsed.
fn echo_needle(text: &str) -> Option<String> {
    let line = text
        .lines()
        .map(collapse_whitespace)
        .find(|line| !line.is_empty())?;
    let needle: String = line.chars().take(ECHO_NEEDLE_CHARS).collect();
    (needle.chars().count() >= ECHO_NEEDLE_MIN_CHARS).then_some(needle)
}

/// Rows holding the needle. Counting, not finding, lets a resend of text that
/// is already on screen still be told apart from no change.
fn count_echoes(screen: &str, needle: &str) -> usize {
    screen
        .lines()
        .map(|line| collapse_whitespace(line).matches(needle).count())
        .sum()
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn agent_get(request_id: &str, target: &str, api_tx: &ApiRequestSender) -> Option<AgentInfo> {
    let response = dispatch_to_app_with_timeout(
        Request {
            id: format!("{request_id}:delivery-agent"),
            method: Method::AgentGet(crate::api::schema::AgentTarget {
                target: target.to_string(),
            }),
        },
        api_tx,
        Some(APP_RESPONSE_TIMEOUT),
    );
    let value: serde_json::Value = serde_json::from_str(&response).ok()?;
    serde_json::from_value(value.get("result")?.get("agent")?.clone()).ok()
}

fn read_detection_text(
    request_id: &str,
    pane_id: &str,
    api_tx: &ApiRequestSender,
) -> Option<String> {
    let response = dispatch_to_app_with_timeout(
        Request {
            id: format!("{request_id}:delivery-read"),
            method: Method::PaneRead(PaneReadParams {
                pane_id: pane_id.to_string(),
                source: ReadSource::Detection,
                lines: None,
                format: ReadFormat::Text,
                strip_ansi: true,
                intent: ReadIntent::Passive,
            }),
        },
        api_tx,
        Some(APP_RESPONSE_TIMEOUT),
    );
    let value: serde_json::Value = serde_json::from_str(&response).ok()?;
    let read: PaneReadResult =
        serde_json::from_value(value.get("result")?.get("read")?.clone()).ok()?;
    Some(read.text)
}

/// Adds `delivered` to a successful `agent_prompted` response and leaves any
/// other response untouched.
pub(super) fn with_delivery(response: String, delivery: PromptDelivery) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&response) else {
        return response;
    };
    let Some(result) = value
        .get_mut("result")
        .and_then(serde_json::Value::as_object_mut)
        .filter(|result| result.get("type").and_then(|t| t.as_str()) == Some("agent_prompted"))
    else {
        return response;
    };
    let Ok(delivered) = serde_json::to_value(delivery) else {
        return response;
    };
    result.insert("delivered".into(), delivered);
    serde_json::to_string(&value).unwrap_or(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needle_is_the_start_of_the_first_non_blank_line() {
        assert_eq!(
            echo_needle("\n  Review   this diff\nsecond line").as_deref(),
            Some("Review this diff")
        );
        let long = "a".repeat(80);
        assert_eq!(echo_needle(&long).map(|n| n.len()), Some(ECHO_NEEDLE_CHARS));
    }

    #[test]
    fn short_or_blank_prompts_have_no_needle() {
        assert_eq!(echo_needle("ok"), None);
        assert_eq!(echo_needle(" \n\t\n"), None);
    }

    #[test]
    fn needle_respects_char_boundaries() {
        let text = "é".repeat(40);
        assert_eq!(
            echo_needle(&text).map(|n| n.chars().count()),
            Some(ECHO_NEEDLE_CHARS)
        );
    }

    #[test]
    fn echoes_count_rows_with_collapsed_whitespace() {
        let screen = "│ > Review  this diff │\nold: Review this diff\nunrelated";
        assert_eq!(count_echoes(screen, "Review this diff"), 2);
        assert_eq!(count_echoes(screen, "Fix the build"), 0);
    }

    #[test]
    fn classify_maps_evidence_to_delivery() {
        assert_eq!(classify(false, false, false), None);
        assert_eq!(classify(true, false, false), None);
        assert_eq!(classify(false, true, false), Some(PromptDelivery::Typed));
        assert_eq!(classify(false, false, true), Some(PromptDelivery::Typed));
        assert_eq!(classify(true, false, true), Some(PromptDelivery::Queued));
    }

    #[test]
    fn with_delivery_only_touches_agent_prompted_results() {
        let prompted = r#"{"id":"1","result":{"type":"agent_prompted","agent":{}}}"#;
        let value: serde_json::Value =
            serde_json::from_str(&with_delivery(prompted.into(), PromptDelivery::Queued)).unwrap();
        assert_eq!(value["result"]["delivered"], "queued");

        let error = r#"{"id":"1","error":{"code":"timeout","message":"x"}}"#;
        assert_eq!(with_delivery(error.into(), PromptDelivery::Typed), error);
    }

    #[test]
    fn unknown_future_values_deserialize_as_unknown() {
        let delivery: PromptDelivery = serde_json::from_str(r#""landed_on_mars""#).unwrap();
        assert_eq!(delivery, PromptDelivery::Unknown);
    }
}
