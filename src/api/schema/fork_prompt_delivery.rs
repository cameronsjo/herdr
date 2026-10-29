//! Fork: whether `agent.prompt` text reached the agent's input
//! (cameronsjo/herdr#102).

use serde::{Deserialize, Serialize};

/// What herdr saw after writing a prompt into an agent's pane.
///
/// Herdr cannot look inside the agent, so this is screen and status evidence,
/// not an acknowledgement from the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PromptDelivery {
    /// The agent was not working, and it started working or its screen
    /// showed the prompt.
    Typed,
    /// The agent was already working and its screen showed the prompt, so the
    /// agent holds it for after the current turn.
    Queued,
    /// Neither showed up in time. The bytes were written; a resend may
    /// deliver the prompt twice.
    #[serde(other)]
    Unknown,
}
