use std::path::PathBuf;

use crate::api::schema::{
    EventData, EventEnvelope, EventKind, ResponseResult, WorkspaceCloseParams,
    WorkspaceCreateParams, WorkspaceMergeParams, WorkspaceMoveBlockParams, WorkspaceMoveParams,
    WorkspaceRenameParams, WorkspaceReportMetadataParams, WorkspaceTarget,
};
use crate::app::App;

use super::super::api_helpers::{normalize_metadata_source, normalize_metadata_ttl};
use super::responses::{encode_error, encode_success};
use crate::label::sanitize_label;

impl App {
    pub(super) fn handle_workspace_list(&mut self, id: String) -> String {
        encode_success(
            id,
            ResponseResult::WorkspaceList {
                workspaces: self.workspace_list_info(),
            },
        )
    }

    pub(super) fn handle_workspace_get(&mut self, id: String, target: WorkspaceTarget) -> String {
        let Some(index) = self.parse_workspace_id(&target.workspace_id) else {
            return workspace_not_found(id, &target.workspace_id);
        };
        let Some(_) = self.state.workspaces.get(index) else {
            return workspace_not_found(id, &target.workspace_id);
        };

        encode_success(
            id,
            ResponseResult::WorkspaceInfo {
                workspace: self.workspace_info(index),
            },
        )
    }

    pub(super) fn handle_workspace_create(
        &mut self,
        id: String,
        params: WorkspaceCreateParams,
    ) -> String {
        let source_workspace_index = if params.cwd.is_some() {
            None
        } else {
            match params.source_workspace_id.as_deref() {
                Some(workspace_id) => match self
                    .parse_workspace_id(workspace_id)
                    .filter(|index| self.state.workspaces.get(*index).is_some())
                {
                    Some(index) => Some(index),
                    None => return workspace_not_found(id, workspace_id),
                },
                None => self.workspace_creation_source(),
            }
        };
        let cwd = params.cwd.map(PathBuf::from).unwrap_or_else(|| {
            source_workspace_index.map_or_else(
                || self.resolve_new_terminal_cwd(None),
                |index| self.resolved_new_workspace_cwd_from(index),
            )
        });
        let extra_env = match super::env::normalize_launch_env(params.env) {
            Ok(env) => env,
            Err((code, message)) => return encode_error(id, &code, message),
        };
        match self.create_workspace_with_launch_env(cwd, params.focus, extra_env) {
            Ok(index) => {
                if let Some(label) = params.label {
                    if let Some(workspace) = self.state.workspaces.get_mut(index) {
                        workspace.set_custom_name(label);
                        crate::logging::workspace_renamed(&workspace.id);
                    }
                }
                self.emit_workspace_open_events(index);
                encode_success(
                    id,
                    self.workspace_created_result(index)
                        .expect("new workspace should produce a complete create response"),
                )
            }
            Err(err) => encode_error(id, "workspace_create_failed", err.to_string()),
        }
    }

    pub(super) fn handle_workspace_focus(&mut self, id: String, target: WorkspaceTarget) -> String {
        let Some(index) = self.parse_workspace_id(&target.workspace_id) else {
            return workspace_not_found(id, &target.workspace_id);
        };
        if self.state.workspaces.get(index).is_none() {
            return workspace_not_found(id, &target.workspace_id);
        }
        self.state.switch_workspace(index);

        encode_success(
            id,
            ResponseResult::WorkspaceInfo {
                workspace: self.workspace_info(index),
            },
        )
    }

    pub(super) fn handle_workspace_rename(
        &mut self,
        id: String,
        params: WorkspaceRenameParams,
    ) -> String {
        let Some(index) = self.parse_workspace_id(&params.workspace_id) else {
            return workspace_not_found(id, &params.workspace_id);
        };
        let Some(ws) = self.state.workspaces.get_mut(index) else {
            return workspace_not_found(id, &params.workspace_id);
        };
        // Sanitize once and reuse, so the event carries what was stored.
        let label = sanitize_label(params.label.clone());
        ws.set_custom_name(label.clone());
        crate::logging::workspace_renamed(&ws.id);
        self.schedule_session_save();
        self.emit_event(EventEnvelope {
            event: EventKind::WorkspaceRenamed,
            data: EventData::WorkspaceRenamed {
                workspace_id: self.public_workspace_id(index),
                label,
            },
        });

        encode_success(
            id,
            ResponseResult::WorkspaceInfo {
                workspace: self.workspace_info(index),
            },
        )
    }

    pub(super) fn handle_workspace_move(
        &mut self,
        id: String,
        params: WorkspaceMoveParams,
    ) -> String {
        let Some(index) = self.parse_workspace_id(&params.workspace_id) else {
            return workspace_not_found(id, &params.workspace_id);
        };
        if self.state.workspaces.get(index).is_none() {
            return workspace_not_found(id, &params.workspace_id);
        }
        if params.insert_index > self.state.workspaces.len() {
            return encode_error(
                id,
                "workspace_move_failed",
                format!("insert_index {} is out of bounds", params.insert_index),
            );
        }

        let workspace_id = self.public_workspace_id(index);
        let insert_index = params.insert_index;
        let moved = self.state.move_workspace(index, insert_index);
        let workspaces = self.workspace_list_info();
        if moved {
            self.emit_event(EventEnvelope {
                event: EventKind::WorkspaceMoved,
                data: EventData::WorkspaceMoved {
                    workspace_id,
                    insert_index,
                    workspaces: workspaces.clone(),
                },
            });
        }

        encode_success(id, ResponseResult::WorkspaceList { workspaces })
    }

    pub(super) fn handle_workspace_move_block(
        &mut self,
        id: String,
        params: WorkspaceMoveBlockParams,
    ) -> String {
        if params.workspace_ids.is_empty() {
            return encode_error(
                id,
                "workspace_move_block_failed",
                "workspace_ids must not be empty",
            );
        }

        let mut workspace_ids = Vec::with_capacity(params.workspace_ids.len());
        let mut seen_ids = std::collections::HashSet::new();
        for requested_id in &params.workspace_ids {
            let Some(index) = self.parse_workspace_id(requested_id) else {
                return workspace_not_found(id, requested_id);
            };
            let Some(workspace) = self.state.workspaces.get(index) else {
                return workspace_not_found(id, requested_id);
            };
            if !seen_ids.insert(workspace.id.clone()) {
                return encode_error(
                    id,
                    "workspace_move_block_failed",
                    format!("workspace {requested_id} appears more than once"),
                );
            }
            workspace_ids.push(workspace.id.clone());
        }

        let before_workspace_id = match params.before_workspace_id {
            Some(requested_id) => {
                let Some(index) = self.parse_workspace_id(&requested_id) else {
                    return workspace_not_found(id, &requested_id);
                };
                let Some(workspace) = self.state.workspaces.get(index) else {
                    return workspace_not_found(id, &requested_id);
                };
                if seen_ids.contains(&workspace.id) {
                    return encode_error(
                        id,
                        "workspace_move_block_failed",
                        "before_workspace_id must not be part of workspace_ids",
                    );
                }
                Some(workspace.id.clone())
            }
            None => None,
        };

        let moved = self
            .state
            .move_workspace_block(&workspace_ids, before_workspace_id.as_deref());
        let workspaces = self.workspace_list_info();
        if moved {
            self.emit_event(EventEnvelope {
                event: EventKind::WorkspaceReordered,
                data: EventData::WorkspaceReordered {
                    workspace_ids,
                    before_workspace_id,
                    workspaces: workspaces.clone(),
                },
            });
        }

        encode_success(id, ResponseResult::WorkspaceList { workspaces })
    }

    pub(super) fn handle_workspace_report_metadata(
        &mut self,
        id: String,
        params: WorkspaceReportMetadataParams,
    ) -> String {
        let Some(index) = self.parse_workspace_id(&params.workspace_id) else {
            return workspace_not_found(id, &params.workspace_id);
        };
        let source = match normalize_metadata_source(params.source) {
            Ok(source) => source,
            Err(message) => return encode_error(id, "invalid_metadata_source", message),
        };
        let ttl = match normalize_metadata_ttl(params.ttl_ms) {
            Ok(ttl) => ttl,
            Err(message) => return encode_error(id, "invalid_metadata_ttl", message),
        };
        let tokens = match super::super::api_helpers::normalize_metadata_tokens(params.tokens) {
            Ok(tokens) => tokens,
            Err(message) => return encode_error(id, "invalid_metadata_token", message),
        };
        let Some(workspace) = self.state.workspaces.get_mut(index) else {
            return workspace_not_found(id, &params.workspace_id);
        };
        if !crate::metadata_tokens::sequence_is_fresh(
            &workspace.metadata_token_sequences,
            &source,
            params.seq,
        ) {
            return encode_success(id, ResponseResult::Ok {});
        }
        if workspace.metadata_tokens.key_count_after_patch(&tokens)
            > super::super::api_helpers::MAX_METADATA_TOKEN_KEYS_PER_RESOURCE
        {
            return encode_error(
                id,
                "metadata_token_limit",
                format!(
                    "workspace metadata may contain at most {} tokens",
                    super::super::api_helpers::MAX_METADATA_TOKEN_KEYS_PER_RESOURCE
                ),
            );
        }
        match crate::metadata_tokens::accept_sequence(
            &mut workspace.metadata_token_sequences,
            &source,
            params.seq,
        ) {
            Ok(true) => {}
            Ok(false) => return encode_success(id, ResponseResult::Ok {}),
            Err(()) => {
                return encode_error(
                    id,
                    "metadata_sequence_source_limit",
                    format!(
                        "workspace metadata may track at most {} sequenced sources",
                        crate::metadata_tokens::MAX_SEQUENCE_SOURCES
                    ),
                );
            }
        }
        let changed = workspace
            .metadata_tokens
            .patch(tokens, ttl, std::time::Instant::now());
        if changed {
            self.sync_agent_metadata_deadline();
            self.emit_workspace_token_updated(index);
        }
        encode_success(id, ResponseResult::Ok {})
    }

    /// Relocates every tab of the source workspace into the target, then closes
    /// the emptied source.
    ///
    /// The gate is `workspace.close`'s, unchanged: a source that owns linked
    /// worktree workspaces refuses without explicit group intent. The socket is
    /// reachable by every process running inside a pane — `HERDR_SOCKET_PATH` is
    /// in their environment — so a merge without this gate would be a one-call
    /// way around a control `workspace.close` enforces. A TUI confirmation is a
    /// second layer over this one, never a substitute for it.
    ///
    /// With group intent the whole worktree group merges: every member's tabs
    /// land in the target and every member closes, so merge never destroys a
    /// tab the way a group close does.
    pub(super) fn handle_workspace_merge(
        &mut self,
        id: String,
        params: WorkspaceMergeParams,
    ) -> String {
        // `parse_workspace_id` falls back to positional parsing without bounds
        // checking, so a bare numeric id yields an index past the end.
        let Some(source_index) = self
            .parse_workspace_id(&params.source_workspace_id)
            .filter(|index| *index < self.state.workspaces.len())
        else {
            return workspace_not_found(id, &params.source_workspace_id);
        };
        let Some(target_index) = self
            .parse_workspace_id(&params.target_workspace_id)
            .filter(|index| *index < self.state.workspaces.len())
        else {
            return workspace_not_found(id, &params.target_workspace_id);
        };
        if source_index == target_index {
            return encode_error(
                id,
                "workspace_merge_failed",
                "source and target must be different workspaces",
            );
        }

        let merge_indices = self.state.workspace_close_indices(source_index);
        if merge_indices.len() >= 2 && !params.merge_group {
            return encode_error(
                id,
                "workspace_group_merge_required",
                "workspace has linked worktree workspaces; use --group (merge_group=true in the API) to merge the group",
            );
        }
        if merge_indices.contains(&target_index) {
            return encode_error(
                id,
                "workspace_merge_failed",
                "target workspace belongs to the source's worktree group",
            );
        }

        // Snapshot every closing workspace before its tabs leave: a drained
        // workspace derefs through an active tab it no longer has.
        let closed_workspaces = merge_indices
            .iter()
            .map(|index| {
                (
                    self.public_workspace_id(*index),
                    self.workspace_info(*index),
                )
            })
            .collect::<Vec<_>>();
        // Follow the operator's own view by id, not by index. `workspace.close`
        // sets `selected` to the closing workspace, which would yank the view
        // to the target on every merge; restoring it keeps a merge of two
        // background workspaces invisible to the operator.
        let selected_workspace_id = self
            .state
            .workspaces
            .get(self.state.selected)
            .map(|workspace| workspace.id.clone());
        let active_workspace_id = self
            .state
            .active
            .and_then(|index| self.state.workspaces.get(index))
            .map(|workspace| workspace.id.clone());
        let target_workspace_key = self.state.workspaces[target_index].id.clone();

        let mut moved_tabs = Vec::new();
        for source_index in merge_indices.iter().copied() {
            let source_workspace_id = self.public_workspace_id(source_index);
            let tab_count = self.state.workspaces[source_index].tabs.len();
            let previous_tab_ids = (0..tab_count)
                .map(|tab_idx| {
                    self.public_tab_id(source_index, tab_idx)
                        .unwrap_or_else(|| {
                            crate::workspace::public_tab_id_for_number(
                                &self.state.workspaces[source_index].id,
                                tab_idx + 1,
                            )
                        })
                })
                .collect::<Vec<_>>();
            // Snapshot before the take: it unregisters these panes from the
            // source's per-workspace public id space.
            let previous_pane_ids = (0..tab_count)
                .flat_map(|tab_idx| self.public_pane_ids_in_tab(source_index, tab_idx))
                .collect::<Vec<_>>();

            let taken = self.state.workspaces[source_index].take_all_tabs_for_move();
            self.alias_moved_pane_ids(previous_pane_ids);
            for (tab, previous_tab_id) in taken.into_iter().zip(previous_tab_ids) {
                let insert_index = self.state.workspaces[target_index].tabs.len();
                let target_tab_idx =
                    self.state.workspaces[target_index].insert_moved_tab(tab, insert_index);
                moved_tabs.push((previous_tab_id, source_workspace_id.clone(), target_tab_idx));
            }
        }

        self.state.mark_session_dirty();
        for index in merge_indices.iter().rev() {
            if let Some(workspace) = self.state.workspaces.get(*index) {
                crate::logging::workspace_closed(&workspace.id);
            }
            self.state.workspaces.remove(*index);
        }
        let target_index = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.id == target_workspace_key)
            .unwrap_or(0);
        self.state.selected = selected_workspace_id
            .and_then(|id| {
                self.state
                    .workspaces
                    .iter()
                    .position(|workspace| workspace.id == id)
            })
            .unwrap_or(target_index);
        if let Some(id) = active_workspace_id {
            self.state.active = Some(
                self.state
                    .workspaces
                    .iter()
                    .position(|workspace| workspace.id == id)
                    .unwrap_or(target_index),
            );
        }
        self.schedule_session_save();

        let target_workspace_id = self.public_workspace_id(target_index);
        let tabs = self.tab_list_info(target_index);
        for (previous_tab_id, source_workspace_id, target_tab_idx) in moved_tabs {
            let Some(tab_id) = self.public_tab_id(target_index, target_tab_idx) else {
                continue;
            };
            self.emit_event(EventEnvelope {
                event: EventKind::TabMovedAcrossWorkspaces,
                data: EventData::TabMovedAcrossWorkspaces {
                    tab_id,
                    previous_tab_id,
                    workspace_id: target_workspace_id.clone(),
                    source_workspace_id,
                    insert_index: target_tab_idx,
                    tabs: tabs.clone(),
                    // The source is gone, so it has no remaining tabs to report.
                    source_tabs: Vec::new(),
                },
            });
        }
        for (workspace_id, workspace) in closed_workspaces {
            self.emit_event(EventEnvelope {
                event: EventKind::WorkspaceClosed,
                data: EventData::WorkspaceClosed {
                    workspace_id,
                    workspace: Some(workspace),
                },
            });
        }

        encode_success(
            id,
            ResponseResult::WorkspaceInfo {
                workspace: self.workspace_info(target_index),
            },
        )
    }

    pub(super) fn handle_workspace_close(
        &mut self,
        id: String,
        params: WorkspaceCloseParams,
    ) -> String {
        let Some(index) = self.parse_workspace_id(&params.workspace_id) else {
            return workspace_not_found(id, &params.workspace_id);
        };
        if self.state.workspaces.get(index).is_none() {
            return workspace_not_found(id, &params.workspace_id);
        }
        let close_indices = self.state.workspace_close_indices(index);
        if close_indices.len() >= 2 && !params.close_group {
            return encode_error(
                id,
                "workspace_group_close_required",
                "workspace has linked worktree workspaces; use --group (close_group=true in the API) to close the group",
            );
        }
        let closed_workspaces = close_indices
            .iter()
            .map(|index| {
                (
                    self.public_workspace_id(*index),
                    self.workspace_info(*index),
                )
            })
            .collect::<Vec<_>>();
        self.state.selected = index;
        self.state.close_selected_workspace();
        self.shutdown_detached_terminal_runtimes();
        for (workspace_id, workspace) in closed_workspaces {
            self.emit_event(EventEnvelope {
                event: EventKind::WorkspaceClosed,
                data: EventData::WorkspaceClosed {
                    workspace_id,
                    workspace: Some(workspace),
                },
            });
        }

        encode_success(id, ResponseResult::Ok {})
    }

    fn workspace_list_info(&self) -> Vec<crate::api::schema::WorkspaceInfo> {
        self.state
            .workspaces
            .iter()
            .enumerate()
            .map(|(idx, _)| self.workspace_info(idx))
            .collect()
    }
}

fn workspace_not_found(id: String, workspace_id: &str) -> String {
    encode_error(
        id,
        "workspace_not_found",
        format!("workspace {workspace_id} not found"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::schema::{ErrorResponse, SuccessResponse},
        config::Config,
        workspace::Workspace,
    };

    // `new_cwd = follow` must anchor on the focused pane for every creation
    // surface. Splits and tabs already do; a new workspace must follow the
    // focused pane too, not the source workspace's first-tab root pane.
    #[tokio::test]
    async fn workspace_create_follows_focused_pane_cwd_not_first_tab_root() {
        use super::super::test_support::{exiting_test_command, shutdown_test_runtimes};
        use crate::config::ShellModeConfig;

        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.default_shell = exiting_test_command().into();
        app.state.shell_mode = ShellModeConfig::NonLogin;
        app.state.workspaces = vec![Workspace::test_new("spaces")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();

        // Second tab becomes the focused pane, away from tab 1's root pane.
        let response = app.handle_tab_create(
            "tab".into(),
            crate::api::schema::TabCreateParams {
                workspace_id: None,
                cwd: None,
                focus: true,
                label: None,
                env: Default::default(),
            },
        );
        let _: SuccessResponse = serde_json::from_str(&response).unwrap();
        // Drop runtimes so cwd resolution deterministically uses cached state.
        shutdown_test_runtimes(&mut app);

        let focused_cwd = std::env::temp_dir().join(format!(
            "herdr-ws-follow-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&focused_cwd).unwrap();
        let ws = &app.state.workspaces[0];
        let root_cwd = ws.identity_cwd.clone();
        let focused_pane = ws.focused_pane_id().unwrap();
        assert_ne!(focused_pane, ws.tabs[0].root_pane);
        let terminal_id = ws.terminal_id(focused_pane).cloned().unwrap();
        app.state.terminals.get_mut(&terminal_id).unwrap().cwd = focused_cwd.clone();

        let response = app.handle_workspace_create(
            "req".into(),
            WorkspaceCreateParams {
                source_workspace_id: None,
                cwd: None,
                focus: false,
                label: None,
                env: Default::default(),
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert!(matches!(
            success.result,
            ResponseResult::WorkspaceCreated { .. }
        ));
        let created_cwd = &app.state.workspaces[1].identity_cwd;
        assert_eq!(
            crate::worktree::canonical_or_original(created_cwd),
            crate::worktree::canonical_or_original(&focused_cwd)
        );
        assert_ne!(
            crate::worktree::canonical_or_original(created_cwd),
            crate::worktree::canonical_or_original(&root_cwd)
        );
        shutdown_test_runtimes(&mut app);
        let _ = std::fs::remove_dir_all(&focused_cwd);
    }

    #[tokio::test]
    async fn workspace_create_uses_explicit_source_workspace() {
        use super::super::test_support::{exiting_test_command, shutdown_test_runtimes};
        use crate::config::ShellModeConfig;

        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.default_shell = exiting_test_command().into();
        app.state.shell_mode = ShellModeConfig::NonLogin;
        app.state.workspaces = vec![Workspace::test_new("first"), Workspace::test_new("source")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        shutdown_test_runtimes(&mut app);

        let source_cwd =
            std::env::temp_dir().join(format!("herdr-ws-explicit-source-{}", std::process::id()));
        std::fs::create_dir_all(&source_cwd).unwrap();
        let pane_id = app.state.workspaces[1].focused_pane_id().unwrap();
        let terminal_id = app.state.workspaces[1]
            .terminal_id(pane_id)
            .cloned()
            .unwrap();
        app.state.terminals.get_mut(&terminal_id).unwrap().cwd = source_cwd.clone();
        let source_workspace_id = app.public_workspace_id(1);

        let response = app.handle_workspace_create(
            "req".into(),
            WorkspaceCreateParams {
                source_workspace_id: Some(source_workspace_id),
                cwd: None,
                focus: false,
                label: None,
                env: Default::default(),
            },
        );
        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert!(matches!(
            success.result,
            ResponseResult::WorkspaceCreated { .. }
        ));
        assert_eq!(
            crate::worktree::canonical_or_original(&app.state.workspaces[2].identity_cwd),
            crate::worktree::canonical_or_original(&source_cwd)
        );

        let invalid = app.handle_workspace_create(
            "invalid".into(),
            WorkspaceCreateParams {
                source_workspace_id: Some("w_999".into()),
                cwd: None,
                focus: false,
                label: None,
                env: Default::default(),
            },
        );
        let error: ErrorResponse = serde_json::from_str(&invalid).unwrap();
        assert_eq!(error.error.code, "workspace_not_found");

        let captured = app.handle_workspace_create(
            "captured".into(),
            WorkspaceCreateParams {
                source_workspace_id: Some("w_999".into()),
                cwd: Some(source_cwd.display().to_string()),
                focus: false,
                label: None,
                env: Default::default(),
            },
        );
        let success: SuccessResponse = serde_json::from_str(&captured).unwrap();
        assert!(matches!(
            success.result,
            ResponseResult::WorkspaceCreated { .. }
        ));
        assert_eq!(
            crate::worktree::canonical_or_original(&app.state.workspaces[3].identity_cwd),
            crate::worktree::canonical_or_original(&source_cwd)
        );
        shutdown_test_runtimes(&mut app);
        let _ = std::fs::remove_dir_all(&source_cwd);
    }

    fn app_with_linked_worktree() -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = vec![Workspace::test_new("issue")];
        app.state.workspaces[0].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: "/repo/herdr-issue".into(),
            is_linked_worktree: true,
        });
        app
    }

    fn app_with_worktree_group() -> App {
        let mut app = app_with_linked_worktree();
        let mut parent = Workspace::test_new("parent");
        parent.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: "/repo/herdr".into(),
            is_linked_worktree: false,
        });
        app.state.workspaces.insert(0, parent);
        app.state.active = Some(1);
        app.state.selected = 1;
        app.state.mode = crate::app::Mode::Terminal;
        app
    }

    #[test]
    fn api_workspace_close_parent_group_requires_explicit_group_intent() {
        for confirm_close in [true, false] {
            let mut app = app_with_worktree_group();
            app.state.confirm_close = confirm_close;
            let parent_id = app.public_workspace_id(0);
            let workspace_ids = app
                .state
                .workspaces
                .iter()
                .map(|workspace| workspace.id.clone())
                .collect::<Vec<_>>();

            let request: crate::api::schema::Request = serde_json::from_value(serde_json::json!({
                "id": "req",
                "method": "workspace.close",
                "params": { "workspace_id": parent_id }
            }))
            .unwrap();
            let response = app.handle_api_request(request);

            let response: serde_json::Value = serde_json::from_str(&response).unwrap();
            assert_eq!(response["error"]["code"], "workspace_group_close_required");
            assert!(app.event_hub.events_after(0).is_empty());
            assert_eq!(app.state.mode, crate::app::Mode::Terminal);
            assert_eq!(app.state.active, Some(1));
            assert_eq!(app.state.selected, 1);
            assert_eq!(
                app.state
                    .workspaces
                    .iter()
                    .map(|workspace| workspace.id.clone())
                    .collect::<Vec<_>>(),
                workspace_ids
            );
        }
    }

    #[test]
    fn api_workspace_close_noncontiguous_group_preserves_adversarial_identity_state() {
        let mut app = app_with_worktree_group();
        let parent = app.state.workspaces.remove(0);
        let linked = app.state.workspaces.remove(0);
        app.state = crate::app::state::AppState::test_with_adversarial_identity_state();
        let survivor_id = app.state.workspaces[0].id.clone();
        app.state.workspaces.insert(0, parent);
        app.state.workspaces.push(linked);
        app.state.active = Some(1);
        app.state.selected = 1;
        app.state.mode = crate::app::Mode::Terminal;
        app.state.ensure_test_terminals();
        let closed_pane_ids = [0, 2].map(|index| app.state.workspaces[index].tabs[0].root_pane);
        let closed_terminal_ids = [0, 2].map(|index| {
            app.state
                .terminal_id_for_pane(index, app.state.workspaces[index].tabs[0].root_pane)
                .expect("closed workspace pane has a terminal")
        });
        for pane_id in closed_pane_ids {
            app.state.plugin_panes.insert(
                pane_id,
                crate::app::state::PluginPaneRecord {
                    plugin_id: "example.pane".into(),
                    entrypoint: "board".into(),
                },
            );
        }
        app.state.assert_invariants_for_test();

        let parent_id = app.public_workspace_id(0);
        let closed = [0, 2]
            .into_iter()
            .map(|index| (app.public_workspace_id(index), app.workspace_info(index)))
            .collect::<Vec<_>>();

        let response = app.handle_workspace_close(
            "req".into(),
            WorkspaceCloseParams {
                workspace_id: parent_id,
                close_group: true,
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(success.id, "req");
        assert_eq!(app.state.workspaces.len(), 1);
        assert_eq!(app.state.workspaces[0].id, survivor_id);
        for terminal_id in closed_terminal_ids {
            assert!(!app.state.terminals.contains_key(&terminal_id));
        }
        for pane_id in closed_pane_ids {
            assert!(!app.state.plugin_panes.contains_key(&pane_id));
        }
        assert!(app.state.terminal_runtime_shutdowns.is_empty());
        app.state.assert_invariants_for_test();
        let events = app.event_hub.events_after(0);
        assert_eq!(events.len(), closed.len());
        for ((_, event), (workspace_id, workspace)) in events.iter().zip(closed) {
            assert!(matches!(event.event, EventKind::WorkspaceClosed));
            assert!(matches!(
                &event.data,
                EventData::WorkspaceClosed {
                    workspace_id: closed_id,
                    workspace: Some(closed_workspace),
                } if closed_id == &workspace_id && closed_workspace == &workspace
            ));
        }
    }

    #[test]
    fn api_workspace_close_closes_linked_worktree_workspace_only() {
        let mut app = app_with_worktree_group();
        let linked_id = app.public_workspace_id(1);

        let response = app.handle_workspace_close(
            "req".into(),
            WorkspaceCloseParams {
                workspace_id: linked_id,
                close_group: true,
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(success.id, "req");
        assert_eq!(app.state.workspaces.len(), 1);
        assert_eq!(app.state.workspaces[0].display_name(), "parent");
    }

    #[test]
    fn api_workspace_close_event_includes_final_worktree_snapshot() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            event_hub.clone(),
        );
        app.state.workspaces = app_with_linked_worktree().state.workspaces;
        let workspace_id = app.state.workspaces[0].id.clone();

        let response = app.handle_workspace_close(
            "req".into(),
            WorkspaceCloseParams {
                workspace_id: workspace_id.clone(),
                close_group: false,
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(success.id, "req");
        let events = event_hub.events_after(0);
        assert!(events.iter().any(|(_, event)| {
            matches!(
                &event.data,
                EventData::WorkspaceClosed {
                    workspace_id: closed_id,
                    workspace: Some(workspace),
                } if closed_id == &workspace_id
                    && workspace
                        .worktree
                        .as_ref()
                        .is_some_and(|worktree| worktree.is_linked_worktree)
            )
        }));
    }

    fn merge_test_app(labels: &[&str]) -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = labels
            .iter()
            .map(|label| Workspace::test_new(label))
            .collect();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        app
    }

    fn merge_request(app: &mut App, source: usize, target: usize, merge_group: bool) -> String {
        let source_workspace_id = app.public_workspace_id(source);
        let target_workspace_id = app.public_workspace_id(target);
        app.handle_workspace_merge(
            "req".into(),
            WorkspaceMergeParams {
                source_workspace_id,
                target_workspace_id,
                merge_group,
            },
        )
    }

    /// The socket is reachable from inside every pane, so this gate — not the
    /// TUI confirmation — is what keeps merge from being a one-call way around
    /// the explicit intent `workspace.close` requires for a worktree group.
    #[test]
    fn api_workspace_merge_group_source_requires_explicit_group_intent() {
        let mut app = app_with_worktree_group();
        app.state.workspaces.push(Workspace::test_new("target"));
        app.state.ensure_test_terminals();
        let workspace_ids = app
            .state
            .workspaces
            .iter()
            .map(|workspace| workspace.id.clone())
            .collect::<Vec<_>>();

        let response = merge_request(&mut app, 0, 2, false);

        let error: ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(error.error.code, "workspace_group_merge_required");
        assert_eq!(
            app.state
                .workspaces
                .iter()
                .map(|workspace| workspace.id.clone())
                .collect::<Vec<_>>(),
            workspace_ids
        );
        assert!(app.event_hub.events_after(0).is_empty());
    }

    #[test]
    fn api_workspace_merge_group_proceeds_with_explicit_group_intent() {
        let mut app = app_with_worktree_group();
        app.state.workspaces.push(Workspace::test_new("target"));
        app.state.ensure_test_terminals();
        let target_id = app.state.workspaces[2].id.clone();

        let response = merge_request(&mut app, 0, 2, true);

        let _: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(app.state.workspaces.len(), 1);
        assert_eq!(app.state.workspaces[0].id, target_id);
        // Merge relocates tabs rather than destroying them: the target keeps
        // its own tab plus one from each merged group member.
        assert_eq!(app.state.workspaces[0].tabs.len(), 3);
        app.state.assert_invariants_for_test();
    }

    #[test]
    fn api_workspace_merge_moves_every_tab_in_order_and_closes_the_source() {
        let mut app = merge_test_app(&["source", "target"]);
        app.state.workspaces[0].test_add_tab(Some("two"));
        app.state.workspaces[0].test_add_tab(Some("three"));
        app.state.ensure_test_terminals();
        let source_id = app.state.workspaces[0].id.clone();
        let source_root_panes = app.state.workspaces[0]
            .tabs
            .iter()
            .map(|tab| tab.root_pane)
            .collect::<Vec<_>>();
        assert_eq!(source_root_panes.len(), 3);
        let target_root_pane = app.state.workspaces[1].tabs[0].root_pane;

        let response = merge_request(&mut app, 0, 1, false);

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::WorkspaceInfo { workspace } = success.result else {
            panic!("expected the merged target workspace");
        };
        assert_eq!(app.state.workspaces.len(), 1);
        assert!(
            !app.state
                .workspaces
                .iter()
                .any(|candidate| candidate.id == source_id),
            "merged source workspace should be gone"
        );
        assert_eq!(workspace.workspace_id, app.public_workspace_id(0));
        let merged_root_panes = app.state.workspaces[0]
            .tabs
            .iter()
            .map(|tab| tab.root_pane)
            .collect::<Vec<_>>();
        let mut expected = vec![target_root_pane];
        expected.extend(source_root_panes);
        assert_eq!(merged_root_panes, expected);
        app.state.assert_invariants_for_test();
    }

    #[test]
    fn api_workspace_merge_keeps_the_operators_selected_workspace() {
        let mut app = merge_test_app(&["source", "target", "watching"]);
        app.state.selected = 2;
        app.state.active = Some(2);
        let watched_id = app.state.workspaces[2].id.clone();

        let response = merge_request(&mut app, 0, 1, false);

        let _: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(app.state.workspaces[app.state.selected].id, watched_id);
        assert_eq!(
            app.state
                .active
                .map(|index| app.state.workspaces[index].id.clone()),
            Some(watched_id)
        );
        app.state.assert_invariants_for_test();
    }

    #[test]
    fn api_workspace_merge_refuses_a_workspace_into_itself() {
        let mut app = merge_test_app(&["only", "other"]);
        let workspace_id = app.public_workspace_id(0);

        let response = app.handle_workspace_merge(
            "req".into(),
            WorkspaceMergeParams {
                source_workspace_id: workspace_id.clone(),
                target_workspace_id: workspace_id,
                merge_group: false,
            },
        );

        let error: ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(error.error.code, "workspace_merge_failed");
        // Assert the reason, not just the refusal: every other guard here
        // returns the same code, so a code-only assertion passes even when the
        // self-merge check is gone.
        assert!(
            error.error.message.contains("different workspaces"),
            "unexpected refusal reason: {}",
            error.error.message
        );
        assert_eq!(app.state.workspaces.len(), 2);
        assert!(app.event_hub.events_after(0).is_empty());
    }

    #[test]
    fn api_workspace_merge_refuses_a_target_inside_the_source_group() {
        let mut app = app_with_worktree_group();
        app.state.ensure_test_terminals();

        let response = merge_request(&mut app, 0, 1, true);

        let error: ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(error.error.code, "workspace_merge_failed");
        assert_eq!(app.state.workspaces.len(), 2);
    }

    #[test]
    fn workspace_metadata_tokens_patch_clear_and_emit_snapshot() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            event_hub.clone(),
        );
        app.state.workspaces = vec![Workspace::test_new("one")];
        let workspace_id = app.public_workspace_id(0);

        for (tokens, expected) in [
            (
                std::collections::HashMap::from([
                    ("summary".into(), Some("reviewing auth".into())),
                    ("jj_status".into(), Some("2 changes".into())),
                ]),
                std::collections::HashMap::from([
                    ("summary".into(), "reviewing auth".into()),
                    ("jj_status".into(), "2 changes".into()),
                ]),
            ),
            (
                std::collections::HashMap::from([
                    ("summary".into(), Some("done".into())),
                    ("jj_status".into(), None),
                ]),
                std::collections::HashMap::from([("summary".into(), "done".into())]),
            ),
        ] {
            let response = app.handle_api_request(crate::api::schema::Request {
                id: "req".into(),
                method: crate::api::schema::Method::WorkspaceReportMetadata(
                    WorkspaceReportMetadataParams {
                        workspace_id: workspace_id.clone(),
                        source: "user:test".into(),
                        tokens,
                        seq: None,
                        ttl_ms: None,
                    },
                ),
            });
            let success: SuccessResponse = serde_json::from_str(&response).unwrap();
            assert_eq!(success.result, ResponseResult::Ok {});
            assert_eq!(app.workspace_info(0).tokens, expected);
        }

        assert!(event_hub.events_after(0).iter().any(|(_, event)| matches!(
            &event.data,
            EventData::WorkspaceMetadataUpdated { workspace }
                if workspace.tokens.get("summary").map(String::as_str) == Some("done")
                    && !workspace.tokens.contains_key("jj_status")
        )));
    }

    #[test]
    fn workspace_token_ttl_expires_through_runtime_and_emits_update() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            event_hub.clone(),
        );
        app.state.workspaces = vec![Workspace::test_new("one")];
        let workspace_id = app.public_workspace_id(0);
        let response = app.handle_workspace_report_metadata(
            "req".into(),
            WorkspaceReportMetadataParams {
                workspace_id,
                source: "user:test".into(),
                tokens: std::collections::HashMap::from([(
                    "summary".into(),
                    Some("temporary".into()),
                )]),
                seq: None,
                ttl_ms: Some(1),
            },
        );
        let _: SuccessResponse = serde_json::from_str(&response).unwrap();
        let deadline = app.agent_metadata_deadline.expect("token deadline");

        app.expire_metadata_at(deadline, deadline);

        assert!(app.workspace_info(0).tokens.is_empty());
        assert!(event_hub.events_after(0).iter().any(|(_, event)| matches!(
            &event.data,
            EventData::WorkspaceMetadataUpdated { workspace } if workspace.tokens.is_empty()
        )));
    }

    #[test]
    fn api_workspace_move_reorders_workspaces() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            event_hub.clone(),
        );
        app.state.workspaces = vec![
            Workspace::test_new("one"),
            Workspace::test_new("two"),
            Workspace::test_new("three"),
        ];
        app.state.active = Some(0);
        app.state.selected = 0;
        let moved_id = app.public_workspace_id(0);

        let response = app.handle_workspace_move(
            "req".into(),
            WorkspaceMoveParams {
                workspace_id: moved_id.clone(),
                insert_index: 3,
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::WorkspaceList { workspaces } = success.result else {
            panic!("expected workspace list");
        };
        assert_eq!(workspaces[2].workspace_id, moved_id);
        assert_eq!(app.state.workspaces[2].display_name(), "one");
        let events = event_hub.events_after(0);
        assert!(events.iter().any(|(_, event)| {
            matches!(
                &event.data,
                EventData::WorkspaceMoved {
                    workspace_id,
                    insert_index: 3,
                    workspaces,
                } if workspace_id == &moved_id
                    && workspaces[2].workspace_id == moved_id
            )
        }));
    }

    #[test]
    fn api_workspace_move_block_reorders_atomically() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            event_hub.clone(),
        );
        app.state.workspaces = vec![
            Workspace::test_new("child"),
            Workspace::test_new("normal"),
            Workspace::test_new("parent"),
            Workspace::test_new("tail"),
        ];
        let parent_id = app.public_workspace_id(2);
        let child_id = app.public_workspace_id(0);
        let tail_id = app.public_workspace_id(3);

        let response = app.handle_workspace_move_block(
            "req".into(),
            WorkspaceMoveBlockParams {
                workspace_ids: vec![parent_id.clone(), child_id.clone()],
                before_workspace_id: Some(tail_id.clone()),
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::WorkspaceList { workspaces } = success.result else {
            panic!("expected workspace list");
        };
        assert_eq!(
            app.state
                .workspaces
                .iter()
                .map(|workspace| workspace.display_name())
                .collect::<Vec<_>>(),
            ["normal", "parent", "child", "tail"]
        );
        assert_eq!(workspaces[1].workspace_id, parent_id);
        assert_eq!(workspaces[2].workspace_id, child_id);
        let events = event_hub.events_after(0);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].1.data,
            EventData::WorkspaceReordered {
                workspace_ids,
                before_workspace_id,
                workspaces,
            } if workspace_ids.first() == Some(&parent_id)
                && workspace_ids.get(1) == Some(&child_id)
                && workspace_ids.len() == 2
                && before_workspace_id.as_deref() == Some(tail_id.as_str())
                && workspaces[1].workspace_id == parent_id
        ));
    }

    #[test]
    fn api_workspace_move_noop_does_not_emit_event() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            event_hub.clone(),
        );
        app.state.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        let moved_id = app.public_workspace_id(0);

        let response = app.handle_workspace_move(
            "req".into(),
            WorkspaceMoveParams {
                workspace_id: moved_id.clone(),
                insert_index: 1,
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::WorkspaceList { workspaces } = success.result else {
            panic!("expected workspace list");
        };
        assert_eq!(workspaces[0].workspace_id, moved_id);
        assert!(event_hub.events_after(0).is_empty());
    }
}
