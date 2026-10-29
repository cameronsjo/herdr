//! Fork: decide whether `agent.prompt` text reached the agent's input
//! (cameronsjo/herdr#102).
//!
//! Writing the bytes into the pane only proves herdr sent them. A busy agent
//! can hold a prompt without any visible change for a while, and a caller that
//! takes that silence for a lost prompt resends it, so the agent gets two
//! copies. After the write, this watches the agent's status and the bottom of
//! its screen for a short window and reports what it saw.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::api::schema::{
    AgentInfo, AgentStatus, EventData, Method, PaneReadParams, PaneReadResult, PromptDelivery,
    ReadFormat, ReadIntent, ReadSource, Request,
};
use crate::api::server::{
    dispatch_to_app_with_caller_timeout, should_stop_connection, APP_RESPONSE_TIMEOUT,
};
use crate::api::{ApiRequestSender, EventHub};
use crate::ipc::LocalStream;

/// How long to look for evidence after the write completes.
pub(super) const PROMPT_DELIVERY_WINDOW: Duration = Duration::from_millis(1_500);
const PROMPT_DELIVERY_POLL: Duration = Duration::from_millis(100);
/// Characters of the prompt's first line to look for on screen. Short enough
/// to fit most input box rows, long enough not to match unrelated text; a
/// pane too narrow for it wraps the text and reads `unknown`.
const ECHO_NEEDLE_CHARS: usize = 32;
/// A shorter first line ("y", "ok") matches too much incidental text to count.
const ECHO_NEEDLE_MIN_CHARS: usize = 4;

/// The agent's state just before the prompt was written.
pub(super) struct DeliveryBaseline {
    pane_id: String,
    busy: bool,
    needle: Option<String>,
    echoes: usize,
    event_sequence: u64,
}

impl DeliveryBaseline {
    /// Call immediately before dispatching the prompt: status changes after
    /// the event sequence read here count as evidence.
    pub(super) fn capture(
        request_id: &str,
        before: &AgentInfo,
        text: &str,
        api_tx: &ApiRequestSender,
        event_hub: &EventHub,
        deadline: Option<Instant>,
    ) -> Self {
        let needle = echo_needle(text);
        let echoes = match &needle {
            Some(needle) => read_detection_text(request_id, &before.pane_id, api_tx, deadline)
                .map(|screen| count_echoes(&screen, needle))
                .unwrap_or(0),
            None => 0,
        };
        Self {
            pane_id: before.pane_id.clone(),
            busy: before.agent_status == AgentStatus::Working,
            needle,
            echoes,
            event_sequence: event_hub.current_sequence(),
        }
    }

    /// Polls until there is evidence, `deadline` passes, or the caller goes
    /// away.
    pub(super) fn observe(
        &self,
        request_id: &str,
        api_tx: &ApiRequestSender,
        event_hub: &EventHub,
        stream: &mut LocalStream,
        running: &Arc<AtomicBool>,
        deadline: Instant,
    ) -> PromptDelivery {
        let mut statuses = StatusEvidence::default();
        let mut last_sequence = self.event_sequence;
        loop {
            // Replaying events, not polling status, catches a turn that starts
            // and ends between two polls.
            for (sequence, event) in event_hub.events_after(last_sequence) {
                last_sequence = sequence;
                if let EventData::PaneAgentStatusChanged {
                    pane_id,
                    agent_status,
                    ..
                } = event.data
                {
                    if pane_id == self.pane_id {
                        statuses.record(agent_status);
                    }
                }
            }
            let started_turn = statuses.started_turn(self.busy);
            let echoed = !started_turn
                && self.needle.as_deref().is_some_and(|needle| {
                    read_detection_text(request_id, &self.pane_id, api_tx, Some(deadline))
                        .is_some_and(|screen| count_echoes(&screen, needle) > self.echoes)
                });
            if let Some(delivery) = classify(self.busy, started_turn, echoed) {
                return delivery;
            }
            let now = Instant::now();
            if now >= deadline || should_stop_connection(stream, running).unwrap_or(true) {
                return PromptDelivery::Unknown;
            }
            std::thread::sleep(PROMPT_DELIVERY_POLL.min(deadline - now));
        }
    }
}

/// Status changes seen on the agent's pane since the write.
#[derive(Default)]
struct StatusEvidence {
    settled: bool,
    active_after_settled: bool,
    active: bool,
}

impl StatusEvidence {
    fn record(&mut self, status: AgentStatus) {
        match status {
            AgentStatus::Working | AgentStatus::Blocked => {
                self.active = true;
                self.active_after_settled |= self.settled;
            }
            AgentStatus::Idle | AgentStatus::Done => self.settled = true,
            AgentStatus::Unknown => {}
        }
    }

    /// A non-working agent started a turn, or a working one finished its turn
    /// and started another.
    fn started_turn(&self, busy: bool) -> bool {
        if busy {
            self.active_after_settled
        } else {
            self.active
        }
    }
}

/// `None` means no evidence yet.
fn classify(busy: bool, started_turn: bool, echoed: bool) -> Option<PromptDelivery> {
    match (busy, started_turn || echoed) {
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
        .filter(|line| collapse_whitespace(line).contains(needle))
        .count()
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Sends one app request, bounded by `deadline` as well as the usual app
/// timeout so a stalled app loop cannot hold the caller past its own timeout.
fn dispatch(
    request: Request,
    api_tx: &ApiRequestSender,
    deadline: Option<Instant>,
) -> Option<String> {
    let timeout = match deadline {
        Some(deadline) => {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            remaining.min(APP_RESPONSE_TIMEOUT)
        }
        None => APP_RESPONSE_TIMEOUT,
    };
    Some(dispatch_to_app_with_caller_timeout(
        request,
        api_tx,
        Some(timeout),
    ))
}

fn read_detection_text(
    request_id: &str,
    pane_id: &str,
    api_tx: &ApiRequestSender,
    deadline: Option<Instant>,
) -> Option<String> {
    let response = dispatch(
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
        deadline,
    )?;
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
        assert_eq!(
            count_echoes("Review this diff Review this diff", "Review this diff"),
            1
        );
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
    fn a_settled_turn_is_not_evidence_for_an_idle_agent() {
        let mut idle = StatusEvidence::default();
        idle.record(AgentStatus::Done);
        idle.record(AgentStatus::Idle);
        assert!(!idle.started_turn(false));
        idle.record(AgentStatus::Working);
        assert!(idle.started_turn(false));
    }

    #[test]
    fn a_busy_agent_needs_a_new_turn_after_settling() {
        let mut busy = StatusEvidence::default();
        busy.record(AgentStatus::Working);
        assert!(!busy.started_turn(true));
        busy.record(AgentStatus::Idle);
        assert!(!busy.started_turn(true));
        busy.record(AgentStatus::Working);
        assert!(busy.started_turn(true));
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
