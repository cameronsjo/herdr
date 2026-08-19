use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::common::AgentStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TabCreateParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default)]
    pub focus: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct TabListParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TabRenameParams {
    pub tab_id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TabMoveParams {
    pub tab_id: String,
    /// Reorder position within the tab's own workspace. Kept for callers
    /// predating `destination`; ignored when `destination` is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insert_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<TabMoveDestination>,
}

impl TabMoveParams {
    /// Resolves the two accepted request shapes into one destination.
    ///
    /// `destination` wins when present; a bare `insert_index` is the legacy
    /// in-workspace reorder. Neither field is an error — it reorders to the
    /// front, matching what `insert_index: 0` always meant.
    pub fn resolved_destination(&self) -> TabMoveDestination {
        self.destination
            .clone()
            .unwrap_or(TabMoveDestination::Index {
                insert_index: self.insert_index.unwrap_or(0),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TabMoveDestination {
    /// Reorder within the tab's current workspace.
    Index { insert_index: usize },
    /// Move to another workspace, optionally at a given position.
    Workspace {
        workspace_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        insert_index: Option<usize>,
    },
    /// Move into a workspace created for it.
    NewWorkspace {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TabMoveReason {
    /// The destination workspace already owns this tab.
    SameWorkspace,
    /// The tab is its workspace's last, and a workspace must keep one.
    LastTabInWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TabMoveResult {
    pub changed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<TabMoveReason>,
    /// The tab's id after the move — reissued when it changed workspace.
    pub tab_id: String,
    pub workspace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TabInfo {
    pub tab_id: String,
    pub workspace_id: String,
    pub number: usize,
    pub label: String,
    pub focused: bool,
    pub pane_count: usize,
    pub agent_status: AgentStatus,
}
