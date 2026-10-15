use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::common::AgentStatus;
use crate::cli::projection::{cli_params, Field, Input};
use crate::cli::workspace_bindings as bindings;

pub(crate) fn workspace_option(value_name: &'static str) -> Input<Option<String>> {
    Input::value("workspace", value_name, |slot, value| {
        *slot = Some(crate::cli::normalize_workspace_id(value));
        Ok(())
    })
}

pub(crate) fn tab_id() -> Input<String> {
    Input::positional("tab_id", 0, |slot, value| {
        *slot = crate::cli::normalize_tab_id(value);
        Ok(())
    })
    .value_name("TAB_ID")
}

cli_params! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct TabCreateParams => TabCreateProjection {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String> => Field::new(None).input(workspace_option("WORKSPACE_ID").order(0)),
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cwd: Option<String> => Field::new(None).input(bindings::optional_text("cwd", "PATH").path().order(1)),
        #[serde(default)]
        pub focus: bool => Field::new(false)
            .input(bindings::boolean("focus", true).order(4))
            .input(bindings::boolean("no-focus", false).order(5)),
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub label: Option<String> => Field::new(None).input(bindings::optional_text("label", "TEXT").order(2)),
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub env: HashMap<String, String> => Field::new(HashMap::new()).input(bindings::environment().order(3)),
    }
}

cli_params! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
    pub struct TabListParams => TabListProjection {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String> => Field::new(None).input(workspace_option("WORKSPACE_ID")),
    }
}

cli_params! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct TabRenameParams => TabRenameProjection {
        pub tab_id: String => Field::new(String::new()).input(tab_id()),
        pub label: String => Field::new(String::new()).input(bindings::label()),
    }
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

/// Explicit destination moves are separately advertised from legacy tab reordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TabMoveToDestinationParams {
    pub tab_id: String,
    pub destination: TabMoveDestination,
}

impl TabMoveParams {
    /// Resolves the two accepted request shapes into one destination.
    ///
    /// `destination` wins when present; a bare `insert_index` is the legacy
    /// in-workspace reorder. Supplying neither is an error rather than a
    /// default: `insert_index` used to be required, so a request missing both
    /// was rejected at deserialization, and silently reordering it to the front
    /// would turn a previously-refused request into a state mutation.
    pub fn resolved_destination(&self) -> Option<TabMoveDestination> {
        if let Some(destination) = self.destination.clone() {
            return Some(destination);
        }
        self.insert_index
            .map(|insert_index| TabMoveDestination::Index { insert_index })
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
    /// The tab is its workspace's last, and the workspace owns linked worktree
    /// workspaces, so closing it would orphan them. Any other workspace closes
    /// when its last tab moves out.
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
