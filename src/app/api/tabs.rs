use std::path::PathBuf;

use crate::api::schema::{
    EventData, EventEnvelope, EventKind, ResponseResult, TabCreateParams, TabListParams,
    TabMoveDestination, TabMoveParams, TabMoveReason, TabMoveResult, TabRenameParams, TabTarget,
};
use crate::app::{App, Mode};

use super::responses::{encode_error, encode_success};
use crate::label::sanitize_label;

impl App {
    pub(super) fn handle_tab_list(&mut self, id: String, params: TabListParams) -> String {
        let tabs = if let Some(workspace_id) = params.workspace_id {
            let Some(ws_idx) = self.parse_workspace_id(&workspace_id) else {
                return workspace_not_found(id, &workspace_id);
            };
            let Some(_) = self.state.workspaces.get(ws_idx) else {
                return workspace_not_found(id, &workspace_id);
            };
            self.tab_list_info(ws_idx)
        } else {
            let mut tabs = Vec::new();
            for (ws_idx, ws) in self.state.workspaces.iter().enumerate() {
                for tab_idx in 0..ws.tabs.len() {
                    if let Some(tab) = self.tab_info(ws_idx, tab_idx) {
                        tabs.push(tab);
                    }
                }
            }
            tabs
        };

        encode_success(id, ResponseResult::TabList { tabs })
    }

    pub(super) fn handle_tab_get(&mut self, id: String, target: TabTarget) -> String {
        let Some((ws_idx, tab_idx)) = self.parse_tab_id(&target.tab_id) else {
            return tab_not_found(id, &target.tab_id);
        };
        let Some(tab) = self.tab_info(ws_idx, tab_idx) else {
            return tab_not_found(id, &target.tab_id);
        };

        encode_success(id, ResponseResult::TabInfo { tab })
    }

    pub(super) fn handle_tab_create(&mut self, id: String, params: TabCreateParams) -> String {
        let TabCreateParams {
            workspace_id,
            cwd,
            focus,
            label,
            env,
        } = params;
        let ws_idx = if let Some(workspace_id) = workspace_id {
            let Some(ws_idx) = self.parse_workspace_id(&workspace_id) else {
                return workspace_not_found(id, &workspace_id);
            };
            ws_idx
        } else if let Some(active) = self.state.active {
            active
        } else {
            return encode_error(id, "workspace_not_found", "no active workspace");
        };
        let cwd = cwd.map(PathBuf::from).unwrap_or_else(|| {
            self.resolve_new_terminal_cwd(self.focused_pane_cwd_in_workspace(ws_idx))
        });
        let (rows, cols) = self.state.estimate_pane_size();
        let default_shell = self.state.default_shell.clone();
        let scrollback_limit_bytes = self.state.pane_scrollback_limit_bytes;
        let host_terminal_theme = self.state.host_terminal_theme;
        let host_terminal_appearance = self.state.host_terminal_appearance;
        let extra_env = match super::env::normalize_launch_env(env) {
            Ok(env) => env,
            Err((code, message)) => return encode_error(id, &code, message),
        };
        let result = self
            .state
            .workspaces
            .get_mut(ws_idx)
            .ok_or_else(|| std::io::Error::other("workspace disappeared"))
            .and_then(|ws| {
                ws.create_tab(
                    rows,
                    cols,
                    cwd,
                    scrollback_limit_bytes,
                    host_terminal_theme,
                    host_terminal_appearance,
                    crate::pane::PaneShellConfig::new(&default_shell, self.state.shell_mode),
                    extra_env,
                )
            });
        match result {
            Ok((tab_idx, terminal, runtime)) => {
                self.terminal_runtimes.insert(terminal.id.clone(), runtime);
                self.state.terminals.insert(terminal.id.clone(), terminal);
                self.state.remove_alias_shadowed_by_new_pane(
                    self.state.workspaces[ws_idx].tabs[tab_idx].root_pane,
                );
                if let Some(label) = label {
                    let workspace_id = self.state.workspaces[ws_idx].id.clone();
                    let tab_id = self.public_tab_id(ws_idx, tab_idx).unwrap_or_else(|| {
                        crate::workspace::public_tab_id_for_number(&workspace_id, tab_idx + 1)
                    });
                    if let Some(tab) = self
                        .state
                        .workspaces
                        .get_mut(ws_idx)
                        .and_then(|ws| ws.tabs.get_mut(tab_idx))
                    {
                        tab.set_custom_name(label);
                        crate::logging::tab_renamed(&workspace_id, &tab_id);
                    }
                }
                if focus {
                    self.state.switch_workspace_tab(ws_idx, tab_idx);
                    self.state.mode = Mode::Terminal;
                }
                self.schedule_session_save();
                self.emit_tab_created_events(ws_idx, tab_idx);
                encode_success(
                    id,
                    self.tab_created_result(ws_idx, tab_idx)
                        .expect("new tab should produce a complete create response"),
                )
            }
            Err(err) => encode_error(id, "tab_create_failed", err.to_string()),
        }
    }

    pub(super) fn handle_tab_focus(&mut self, id: String, target: TabTarget) -> String {
        let Some((ws_idx, tab_idx)) = self.parse_tab_id(&target.tab_id) else {
            return tab_not_found(id, &target.tab_id);
        };
        self.state.switch_workspace_tab(ws_idx, tab_idx);
        let tab = self.tab_info(ws_idx, tab_idx).unwrap();

        encode_success(id, ResponseResult::TabInfo { tab })
    }

    pub(super) fn handle_tab_rename(&mut self, id: String, params: TabRenameParams) -> String {
        let Some((ws_idx, tab_idx)) = self.parse_tab_id(&params.tab_id) else {
            return tab_not_found(id, &params.tab_id);
        };
        let workspace_id = self.state.workspaces[ws_idx].id.clone();
        let tab_id = self.public_tab_id(ws_idx, tab_idx).unwrap_or_else(|| {
            crate::workspace::public_tab_id_for_number(&workspace_id, tab_idx + 1)
        });
        let Some(tab) = self
            .state
            .workspaces
            .get_mut(ws_idx)
            .and_then(|ws| ws.tabs.get_mut(tab_idx))
        else {
            return tab_not_found(id, &params.tab_id);
        };
        // Sanitize once and reuse: the event must carry what was actually
        // stored, or subscribers receive the unfiltered string.
        let label = sanitize_label(params.label.clone());
        tab.set_custom_name(label.clone());
        crate::logging::tab_renamed(&workspace_id, &tab_id);
        if self.state.active == Some(ws_idx) {
            // Reflow the tab bar so the new label width takes effect immediately.
            // The tab bar renders into cached hit areas; without this refresh the
            // old geometry lingers until the next refresh (e.g. a tab switch),
            // leaving the visible label stale. Mirrors handle_tab_move.
            self.state.refresh_tab_bar_view();
        }
        self.schedule_session_save();
        self.emit_event(EventEnvelope {
            event: EventKind::TabRenamed,
            data: EventData::TabRenamed {
                tab_id: self.public_tab_id(ws_idx, tab_idx).unwrap(),
                workspace_id: self.public_workspace_id(ws_idx),
                label,
            },
        });
        let tab = self.tab_info(ws_idx, tab_idx).unwrap();

        encode_success(id, ResponseResult::TabInfo { tab })
    }

    pub(super) fn handle_tab_move(&mut self, id: String, params: TabMoveParams) -> String {
        let Some((ws_idx, tab_idx)) = self.parse_tab_id(&params.tab_id) else {
            return tab_not_found(id, &params.tab_id);
        };
        if self.state.workspaces.get(ws_idx).is_none() {
            return tab_not_found(id, &params.tab_id);
        }

        let Some(destination) = params.resolved_destination() else {
            return encode_error(
                id,
                "tab_move_failed",
                "one of insert_index or destination is required",
            );
        };

        match destination {
            TabMoveDestination::Index { insert_index } => {
                self.tab_move_within_workspace(id, ws_idx, tab_idx, insert_index)
            }
            TabMoveDestination::Workspace {
                workspace_id,
                insert_index,
            } => {
                // `parse_workspace_id` falls back to positional parsing and does
                // NOT bounds-check, so a bare numeric id yields an index past the
                // end. Re-check before anything indexes it, matching
                // `handle_workspace_rename` and `handle_workspace_move`.
                let target_ws_idx = match self.parse_workspace_id(&workspace_id) {
                    Some(idx) if idx < self.state.workspaces.len() => idx,
                    _ => {
                        return encode_error(
                            id,
                            "workspace_not_found",
                            format!("workspace {workspace_id} not found"),
                        );
                    }
                };
                self.tab_move_to_workspace(id, ws_idx, tab_idx, target_ws_idx, insert_index)
            }
            TabMoveDestination::NewWorkspace { label } => {
                self.tab_move_to_new_workspace(id, ws_idx, tab_idx, label)
            }
        }
    }

    fn tab_move_within_workspace(
        &mut self,
        id: String,
        ws_idx: usize,
        tab_idx: usize,
        insert_index: usize,
    ) -> String {
        let Some(ws) = self.state.workspaces.get(ws_idx) else {
            return encode_error(id, "tab_move_failed", "workspace is unavailable");
        };
        if insert_index > ws.tabs.len() {
            return encode_error(
                id,
                "tab_move_failed",
                format!("insert_index {insert_index} is out of bounds"),
            );
        }

        let tab_id = self
            .public_tab_id(ws_idx, tab_idx)
            .unwrap_or_else(|| crate::workspace::public_tab_id_for_number(&ws.id, tab_idx + 1));
        let workspace_id = self.public_workspace_id(ws_idx);
        let moved = self
            .state
            .workspaces
            .get_mut(ws_idx)
            .is_some_and(|ws| ws.move_tab(tab_idx, insert_index));
        let tabs = self.tab_list_info(ws_idx);
        if moved {
            self.schedule_session_save();
            if self.state.active == Some(ws_idx) {
                self.state.tab_scroll_follow_active = true;
                self.state.refresh_tab_bar_view();
            }
            self.emit_event(EventEnvelope {
                event: EventKind::TabMoved,
                data: EventData::TabMoved {
                    tab_id: tab_id.clone(),
                    workspace_id: workspace_id.clone(),
                    insert_index,
                    tabs: tabs.clone(),
                },
            });
        }

        encode_success(
            id,
            ResponseResult::TabMove {
                move_result: TabMoveResult {
                    changed: moved,
                    // A no-op reorder is the position already being correct,
                    // which needs no explaining.
                    reason: None,
                    tab_id,
                    workspace_id,
                },
                tabs,
            },
        )
    }

    fn tab_move_to_workspace(
        &mut self,
        id: String,
        ws_idx: usize,
        tab_idx: usize,
        target_ws_idx: usize,
        insert_index: Option<usize>,
    ) -> String {
        let previous_tab_id = match self.public_tab_id(ws_idx, tab_idx) {
            Some(tab_id) => tab_id,
            None => return encode_error(id, "tab_move_failed", "source tab is unavailable"),
        };
        let source_workspace_id = self.public_workspace_id(ws_idx);

        if ws_idx == target_ws_idx {
            return self.encode_unchanged_tab_move(
                id,
                TabMoveReason::SameWorkspace,
                previous_tab_id,
                source_workspace_id,
                ws_idx,
            );
        }
        // A workspace derefs through its active tab, so it must keep one.
        // Refusing beats silently closing the workspace out from under the user.
        let source_tabs_len = match self.state.workspaces.get(ws_idx) {
            Some(ws) => ws.tabs.len(),
            None => return encode_error(id, "tab_move_failed", "source workspace is unavailable"),
        };
        if source_tabs_len <= 1 {
            return self.encode_unchanged_tab_move(
                id,
                TabMoveReason::LastTabInWorkspace,
                previous_tab_id,
                source_workspace_id,
                ws_idx,
            );
        }

        // Resolve the destination FULLY before detaching anything. Between the
        // take and the insert the tab belongs to no workspace, so a failure in
        // that window loses it and its live panes — `pane.move` carries a
        // recovery context for the same reason; validating first removes the
        // need for one.
        let Some(target_tabs_len) = self
            .state
            .workspaces
            .get(target_ws_idx)
            .map(|ws| ws.tabs.len())
        else {
            return encode_error(id, "workspace_not_found", "destination workspace not found");
        };
        let insert_index = insert_index.unwrap_or(target_tabs_len);
        if insert_index > target_tabs_len {
            return encode_error(
                id,
                "tab_move_failed",
                format!("insert_index {insert_index} is out of bounds"),
            );
        }

        // Snapshot before the take: it unregisters these panes from the source.
        let previous_pane_ids = self.public_pane_ids_in_tab(ws_idx, tab_idx);
        let Some(taken) = self
            .state
            .workspaces
            .get_mut(ws_idx)
            .and_then(|ws| ws.take_tab_for_move(tab_idx))
        else {
            return encode_error(id, "tab_move_failed", "source tab could not be moved");
        };
        // Old public pane ids keep resolving until something reuses the number.
        self.alias_moved_pane_ids(previous_pane_ids);

        let target_tab_idx =
            self.state.workspaces[target_ws_idx].insert_moved_tab(taken, insert_index);

        self.finish_cross_workspace_tab_move(
            id,
            ws_idx,
            target_ws_idx,
            target_tab_idx,
            previous_tab_id,
            source_workspace_id,
        )
    }

    fn tab_move_to_new_workspace(
        &mut self,
        id: String,
        ws_idx: usize,
        tab_idx: usize,
        label: Option<String>,
    ) -> String {
        let previous_tab_id = match self.public_tab_id(ws_idx, tab_idx) {
            Some(tab_id) => tab_id,
            None => return encode_error(id, "tab_move_failed", "source tab is unavailable"),
        };
        let source_workspace_id = self.public_workspace_id(ws_idx);
        let source_tabs_len = match self.state.workspaces.get(ws_idx) {
            Some(ws) => ws.tabs.len(),
            None => return encode_error(id, "tab_move_failed", "source workspace is unavailable"),
        };
        if source_tabs_len <= 1 {
            return self.encode_unchanged_tab_move(
                id,
                TabMoveReason::LastTabInWorkspace,
                previous_tab_id,
                source_workspace_id,
                ws_idx,
            );
        }

        // Inherit the source workspace's identity cwd: the tab's panes are
        // already running there, so anything else mislabels the new workspace.
        let Some(identity_cwd) = self
            .state
            .workspaces
            .get(ws_idx)
            .map(|ws| ws.identity_cwd.clone())
        else {
            return encode_error(id, "tab_move_failed", "source workspace is unavailable");
        };
        let previous_pane_ids = self.public_pane_ids_in_tab(ws_idx, tab_idx);
        let Some(taken) = self
            .state
            .workspaces
            .get_mut(ws_idx)
            .and_then(|ws| ws.take_tab_for_move(tab_idx))
        else {
            return encode_error(id, "tab_move_failed", "source tab could not be moved");
        };
        self.alias_moved_pane_ids(previous_pane_ids);

        let workspace = crate::workspace::Workspace::from_existing_tab(label, identity_cwd, taken);
        self.state.workspaces.push(workspace);
        let target_ws_idx = self.state.workspaces.len() - 1;

        self.finish_cross_workspace_tab_move(
            id,
            ws_idx,
            target_ws_idx,
            0,
            previous_tab_id,
            source_workspace_id,
        )
    }

    /// Snapshots the public ids of every pane in a tab, before a move
    /// unregisters them from the source workspace's id space.
    fn public_pane_ids_in_tab(
        &self,
        ws_idx: usize,
        tab_idx: usize,
    ) -> Vec<(String, crate::layout::PaneId)> {
        let Some(tab) = self
            .state
            .workspaces
            .get(ws_idx)
            .and_then(|ws| ws.tabs.get(tab_idx))
        else {
            return Vec::new();
        };
        tab.layout
            .pane_ids()
            .into_iter()
            .filter_map(|pane_id| {
                self.public_pane_id(ws_idx, pane_id)
                    .map(|public| (public, pane_id))
            })
            .collect()
    }

    /// Keeps the moved panes' old public ids resolving after the workspace
    /// change, matching what `pane.move` does for a single pane.
    fn alias_moved_pane_ids(&mut self, previous: Vec<(String, crate::layout::PaneId)>) {
        for (previous_public_id, pane_id) in previous {
            self.state
                .record_moved_pane_alias(previous_public_id, pane_id);
        }
    }

    fn finish_cross_workspace_tab_move(
        &mut self,
        id: String,
        source_ws_idx: usize,
        target_ws_idx: usize,
        target_tab_idx: usize,
        previous_tab_id: String,
        source_workspace_id: String,
    ) -> String {
        let Some(tab_id) = self.public_tab_id(target_ws_idx, target_tab_idx) else {
            return encode_error(id, "tab_move_failed", "moved tab is unavailable");
        };
        let workspace_id = self.public_workspace_id(target_ws_idx);
        let tabs = self.tab_list_info(target_ws_idx);
        let source_tabs = self.tab_list_info(source_ws_idx);

        self.state.mark_session_dirty();
        self.schedule_session_save();
        if self.state.active == Some(source_ws_idx) || self.state.active == Some(target_ws_idx) {
            self.state.tab_scroll_follow_active = true;
            self.state.refresh_tab_bar_view();
        }

        self.emit_event(EventEnvelope {
            event: EventKind::TabMovedAcrossWorkspaces,
            data: EventData::TabMovedAcrossWorkspaces {
                tab_id: tab_id.clone(),
                previous_tab_id,
                workspace_id: workspace_id.clone(),
                source_workspace_id,
                insert_index: target_tab_idx,
                tabs: tabs.clone(),
                source_tabs,
            },
        });

        encode_success(
            id,
            ResponseResult::TabMove {
                move_result: TabMoveResult {
                    changed: true,
                    reason: None,
                    tab_id,
                    workspace_id,
                },
                tabs,
            },
        )
    }

    fn encode_unchanged_tab_move(
        &mut self,
        id: String,
        reason: TabMoveReason,
        tab_id: String,
        workspace_id: String,
        ws_idx: usize,
    ) -> String {
        let tabs = self.tab_list_info(ws_idx);
        encode_success(
            id,
            ResponseResult::TabMove {
                move_result: TabMoveResult {
                    changed: false,
                    reason: Some(reason),
                    tab_id,
                    workspace_id,
                },
                tabs,
            },
        )
    }

    pub(super) fn handle_tab_close(&mut self, id: String, target: TabTarget) -> String {
        let Some((ws_idx, tab_idx)) = self.parse_tab_id(&target.tab_id) else {
            return tab_not_found(id, &target.tab_id);
        };
        let Some(tab_id) = self.public_tab_id(ws_idx, tab_idx) else {
            return tab_not_found(id, &target.tab_id);
        };
        let workspace_id = self.public_workspace_id(ws_idx);
        let Some(ws) = self.state.workspaces.get(ws_idx) else {
            return tab_not_found(id, &target.tab_id);
        };
        let closes_workspace = ws.tabs.len() <= 1;
        let terminal_ids = self.state.terminal_ids_for_tab(ws_idx, tab_idx);
        let pane_ids = ws
            .tabs
            .get(tab_idx)
            .map(|tab| tab.layout.pane_ids())
            .unwrap_or_default();

        if closes_workspace {
            if self.state.confirm_implicit_worktree_group_close(ws_idx) {
                return encode_error(
                    id,
                    "confirmation_required",
                    "closing this tab would close a worktree group",
                );
            }
            let workspace = self.workspace_info(ws_idx);
            self.state.selected = ws_idx;
            self.state.close_selected_workspace();
            self.state.remove_plugin_pane_records(pane_ids);
            self.shutdown_detached_terminal_runtimes();
            self.emit_event(EventEnvelope {
                event: EventKind::TabClosed,
                data: EventData::TabClosed {
                    tab_id,
                    workspace_id: workspace_id.clone(),
                },
            });
            self.emit_event(EventEnvelope {
                event: EventKind::WorkspaceClosed,
                data: EventData::WorkspaceClosed {
                    workspace_id,
                    workspace: Some(workspace),
                },
            });
            return encode_success(id, ResponseResult::Ok {});
        }

        let Some(ws) = self.state.workspaces.get_mut(ws_idx) else {
            return tab_not_found(id, &target.tab_id);
        };
        if !ws.close_tab(tab_idx) {
            return encode_error(
                id,
                "tab_close_failed",
                format!("tab {} could not be closed", target.tab_id),
            );
        }
        self.state.remove_plugin_pane_records(pane_ids);
        self.state.remove_unattached_terminal_ids(terminal_ids);
        self.shutdown_detached_terminal_runtimes();
        self.schedule_session_save();
        self.emit_event(EventEnvelope {
            event: EventKind::TabClosed,
            data: EventData::TabClosed {
                tab_id,
                workspace_id,
            },
        });

        encode_success(id, ResponseResult::Ok {})
    }

    fn tab_list_info(&self, ws_idx: usize) -> Vec<crate::api::schema::TabInfo> {
        self.state
            .workspaces
            .get(ws_idx)
            .map(|ws| {
                (0..ws.tabs.len())
                    .filter_map(|idx| self.tab_info(ws_idx, idx))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn workspace_not_found(id: String, workspace_id: &str) -> String {
    encode_error(
        id,
        "workspace_not_found",
        format!("workspace {workspace_id} not found"),
    )
}

fn tab_not_found(id: String, tab_id: &str) -> String {
    encode_error(id, "tab_not_found", format!("tab {tab_id} not found"))
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{exiting_test_command, shutdown_test_runtimes};
    use super::*;
    use crate::{
        api::schema::SuccessResponse,
        config::{Config, ShellModeConfig},
        workspace::Workspace,
    };

    #[test]
    fn api_tab_close_last_tab_closes_workspace_and_emits_both_events() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub.clone());
        app.state.workspaces = vec![Workspace::test_new("tabs")];
        app.state.active = Some(0);
        app.state.selected = 0;
        let tab_id = app.public_tab_id(0, 0).unwrap();
        let workspace_id = app.public_workspace_id(0);

        let response = app.handle_tab_close(
            "req".into(),
            TabTarget {
                tab_id: tab_id.clone(),
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(success.result, ResponseResult::Ok {});
        assert!(app.state.workspaces.is_empty());
        assert!(app.state.active.is_none());
        let events = event_hub.events_after(0);
        assert_eq!(
            events
                .iter()
                .map(|(_, event)| event.event)
                .collect::<Vec<_>>(),
            [EventKind::TabClosed, EventKind::WorkspaceClosed]
        );
        assert!(matches!(
            &events[0].1.data,
            EventData::TabClosed {
                tab_id: closed_tab_id,
                workspace_id: closed_workspace_id,
            } if closed_tab_id == &tab_id && closed_workspace_id == &workspace_id
        ));
        assert!(matches!(
            &events[1].1.data,
            EventData::WorkspaceClosed {
                workspace_id: closed_workspace_id,
                workspace: Some(workspace),
            } if closed_workspace_id == &workspace_id
                && workspace.workspace_id == workspace_id
        ));
    }

    #[test]
    fn api_tab_move_reorders_tabs_in_target_workspace() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub.clone());
        let mut workspace = Workspace::test_new("tabs");
        workspace.test_add_tab(Some("two"));
        workspace.test_add_tab(Some("three"));
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        let moved_root = app.state.workspaces[0].tabs[0].root_pane;
        let moved_id = app.public_tab_id(0, 0).unwrap();

        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id: moved_id.clone(),
                insert_index: Some(3),
                destination: None,
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::TabMove { move_result, tabs } = success.result else {
            panic!("expected tab move result");
        };
        assert!(move_result.changed);
        assert_eq!(move_result.reason, None);
        assert_eq!(app.state.workspaces[0].tabs[2].root_pane, moved_root);
        assert_eq!(tabs[2].tab_id, app.public_tab_id(0, 2).unwrap());
        let events = event_hub.events_after(0);
        assert!(events.iter().any(|(_, event)| {
            matches!(
                &event.data,
                EventData::TabMoved {
                    tab_id,
                    workspace_id,
                    insert_index: 3,
                    tabs,
                } if tab_id == &moved_id
                    && workspace_id == &app.public_workspace_id(0)
                    && tabs[2].tab_id == moved_id
            )
        }));
    }

    /// Two workspaces, the source holding an extra tab so the move is legal.
    fn seed_two_workspaces(app: &mut App) {
        let mut source = Workspace::test_new("source");
        source.test_add_tab(Some("movable"));
        let target = Workspace::test_new("target");
        app.state.workspaces = vec![source, target];
        app.state.active = Some(0);
        app.state.selected = 0;
    }

    #[test]
    fn api_tab_move_to_workspace_reissues_identity_and_keeps_panes() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub.clone());
        seed_two_workspaces(&mut app);

        let moved_root = app.state.workspaces[0].tabs[1].root_pane;
        let pane_count = app.state.workspaces[0].tabs[1].layout.pane_count();
        let previous_tab_id = app.public_tab_id(0, 1).unwrap();
        let target_workspace_id = app.public_workspace_id(1);

        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id: previous_tab_id.clone(),
                insert_index: None,
                destination: Some(TabMoveDestination::Workspace {
                    workspace_id: target_workspace_id.clone(),
                    insert_index: None,
                }),
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::TabMove { move_result, .. } = success.result else {
            panic!("expected tab move result");
        };
        assert!(move_result.changed);
        assert_eq!(move_result.workspace_id, target_workspace_id);

        // The tab left the source and arrived whole.
        assert_eq!(app.state.workspaces[0].tabs.len(), 1);
        assert_eq!(app.state.workspaces[1].tabs.len(), 2);
        let arrived = app.state.workspaces[1].tabs.last().unwrap();
        assert_eq!(arrived.root_pane, moved_root);
        assert_eq!(arrived.layout.pane_count(), pane_count);

        // Its public id is reissued in the target's id space, and resolves.
        assert_ne!(move_result.tab_id, previous_tab_id);
        assert_eq!(app.parse_tab_id(&move_result.tab_id), Some((1, 1)));

        app.state.workspaces[0].assert_invariants_for_test();
        app.state.workspaces[1].assert_invariants_for_test();
    }

    #[test]
    fn api_tab_move_reissues_tab_number_when_target_already_uses_it() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        seed_two_workspaces(&mut app);
        // Give the target a tab carrying the same public number as the mover,
        // which is the collision that silently resolved the wrong tab before.
        let colliding_number = app.state.workspaces[0].tabs[1].number;
        app.state.workspaces[1].tabs[0].number = colliding_number;

        let previous_tab_id = app.public_tab_id(0, 1).unwrap();
        let target_workspace_id = app.public_workspace_id(1);
        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id: previous_tab_id,
                insert_index: None,
                destination: Some(TabMoveDestination::Workspace {
                    workspace_id: target_workspace_id,
                    insert_index: None,
                }),
            },
        );
        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::TabMove { move_result, .. } = success.result else {
            panic!("expected tab move result");
        };
        assert!(move_result.changed);

        let numbers: Vec<usize> = app.state.workspaces[1]
            .tabs
            .iter()
            .map(|tab| tab.number)
            .collect();
        assert_eq!(
            numbers.len(),
            numbers
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            "moved tab must not reuse a public number already live in the target"
        );
        app.state.workspaces[1].assert_invariants_for_test();
    }

    #[test]
    fn api_tab_move_rejects_an_out_of_range_workspace_id_without_panicking() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        seed_two_workspaces(&mut app);
        let tab_id = app.public_tab_id(0, 1).unwrap();

        // `parse_workspace_id` falls back to positional parsing with no bounds
        // check, so a bare numeric id resolves to an index far past the end.
        // Unchecked, this indexed a Vec and panicked the whole server from one
        // socket request.
        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id,
                insert_index: None,
                destination: Some(TabMoveDestination::Workspace {
                    workspace_id: "999999".into(),
                    insert_index: None,
                }),
            },
        );

        assert!(
            response.contains("workspace_not_found"),
            "expected a workspace_not_found error, got: {response}"
        );
        // The source tab must still be attached — nothing may be detached before
        // the destination is known good.
        assert_eq!(app.state.workspaces[0].tabs.len(), 2);
        app.state.workspaces[0].assert_invariants_for_test();
    }

    #[test]
    fn api_tab_move_rejects_an_out_of_bounds_insert_index_without_detaching() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        seed_two_workspaces(&mut app);
        let tab_id = app.public_tab_id(0, 1).unwrap();
        let target_workspace_id = app.public_workspace_id(1);

        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id,
                insert_index: None,
                destination: Some(TabMoveDestination::Workspace {
                    workspace_id: target_workspace_id,
                    insert_index: Some(99),
                }),
            },
        );

        assert!(
            response.contains("out of bounds"),
            "expected an out-of-bounds error, got: {response}"
        );
        assert_eq!(app.state.workspaces[0].tabs.len(), 2);
        assert_eq!(app.state.workspaces[1].tabs.len(), 1);
        app.state.workspaces[0].assert_invariants_for_test();
        app.state.workspaces[1].assert_invariants_for_test();
    }

    #[test]
    fn api_tab_move_requires_a_destination_or_an_insert_index() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        seed_two_workspaces(&mut app);
        let tab_id = app.public_tab_id(0, 1).unwrap();
        let moved_root = app.state.workspaces[0].tabs[1].root_pane;

        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id,
                insert_index: None,
                destination: None,
            },
        );

        assert!(
            response.contains("tab_move_failed"),
            "expected an error, got: {response}"
        );
        // Previously this silently reordered the tab to the front.
        assert_eq!(app.state.workspaces[0].tabs[1].root_pane, moved_root);
    }

    #[test]
    fn a_moved_pane_keeps_only_its_most_recent_alias() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        seed_two_workspaces(&mut app);
        let pane = app.state.workspaces[0].tabs[0].root_pane;

        app.state.record_moved_pane_alias("old:p1".into(), pane);
        app.state.record_moved_pane_alias("older:p1".into(), pane);
        app.state.record_moved_pane_alias("newest:p1".into(), pane);

        let for_pane: Vec<&String> = app
            .state
            .public_pane_id_aliases
            .iter()
            .filter(|(_, alias)| **alias == pane)
            .map(|(key, _)| key)
            .collect();
        assert_eq!(
            for_pane,
            vec![&"newest:p1".to_string()],
            "repeated moves must not accumulate one alias each"
        );
    }

    #[test]
    fn a_live_pane_id_outranks_a_stale_alias_for_the_same_string() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        seed_two_workspaces(&mut app);

        // Take a real pane id, then forge an alias under that exact string
        // pointing at a pane in the *other* workspace — the shape a
        // cross-workspace move leaves behind once the number gets reused.
        let live_id = app
            .public_pane_id(0, app.state.workspaces[0].tabs[0].root_pane)
            .expect("workspace 0 root pane has a public id");
        let other_pane = app.state.workspaces[1].tabs[0].root_pane;
        app.state
            .public_pane_id_aliases
            .insert(live_id.clone(), other_pane);

        let (ws_idx, pane_id) = app.parse_pane_id(&live_id).expect("id should resolve");

        assert_eq!(
            (ws_idx, pane_id),
            (0, app.state.workspaces[0].tabs[0].root_pane),
            "a live public id must win over a stale alias wearing the same string"
        );
    }

    #[test]
    fn api_tab_move_refuses_the_last_tab_in_a_workspace() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        app.state.workspaces = vec![Workspace::test_new("source"), Workspace::test_new("target")];
        app.state.active = Some(0);

        let tab_id = app.public_tab_id(0, 0).unwrap();
        let target_workspace_id = app.public_workspace_id(1);
        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id,
                insert_index: None,
                destination: Some(TabMoveDestination::Workspace {
                    workspace_id: target_workspace_id,
                    insert_index: None,
                }),
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::TabMove { move_result, .. } = success.result else {
            panic!("expected tab move result");
        };
        assert!(!move_result.changed);
        assert_eq!(move_result.reason, Some(TabMoveReason::LastTabInWorkspace));
        assert_eq!(app.state.workspaces[0].tabs.len(), 1);
        assert_eq!(app.state.workspaces[1].tabs.len(), 1);
    }

    #[test]
    fn api_tab_move_to_same_workspace_is_unchanged() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        seed_two_workspaces(&mut app);
        let tab_id = app.public_tab_id(0, 1).unwrap();
        let source_workspace_id = app.public_workspace_id(0);

        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id,
                insert_index: None,
                destination: Some(TabMoveDestination::Workspace {
                    workspace_id: source_workspace_id,
                    insert_index: None,
                }),
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::TabMove { move_result, .. } = success.result else {
            panic!("expected tab move result");
        };
        assert!(!move_result.changed);
        assert_eq!(move_result.reason, Some(TabMoveReason::SameWorkspace));
        assert_eq!(app.state.workspaces[0].tabs.len(), 2);
    }

    #[test]
    fn api_tab_move_to_new_workspace_creates_one_and_emits_the_cross_event() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub.clone());
        let mut source = Workspace::test_new("source");
        source.test_add_tab(Some("movable"));
        app.state.workspaces = vec![source];
        app.state.active = Some(0);

        let moved_root = app.state.workspaces[0].tabs[1].root_pane;
        let previous_tab_id = app.public_tab_id(0, 1).unwrap();
        let source_workspace_id = app.public_workspace_id(0);

        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id: previous_tab_id.clone(),
                insert_index: None,
                destination: Some(TabMoveDestination::NewWorkspace {
                    label: Some("split off".into()),
                }),
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::TabMove { move_result, .. } = success.result else {
            panic!("expected tab move result");
        };
        assert!(move_result.changed);
        assert_eq!(app.state.workspaces.len(), 2);
        assert_eq!(app.state.workspaces[1].tabs.len(), 1);
        assert_eq!(app.state.workspaces[1].tabs[0].root_pane, moved_root);
        assert_eq!(
            app.state.workspaces[1].custom_name.as_deref(),
            Some("split off")
        );
        app.state.workspaces[0].assert_invariants_for_test();
        app.state.workspaces[1].assert_invariants_for_test();

        let events = event_hub.events_after(0);
        assert!(events.iter().any(|(_, event)| {
            matches!(
                &event.data,
                EventData::TabMovedAcrossWorkspaces {
                    previous_tab_id: prev,
                    source_workspace_id: src,
                    ..
                } if prev == &previous_tab_id && src == &source_workspace_id
            )
        }));
    }

    #[test]
    fn api_tab_move_without_destination_still_reorders() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        let mut workspace = Workspace::test_new("tabs");
        workspace.test_add_tab(Some("two"));
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);

        let moved_root = app.state.workspaces[0].tabs[0].root_pane;
        let tab_id = app.public_tab_id(0, 0).unwrap();
        let response = app.handle_tab_move(
            "req".into(),
            TabMoveParams {
                tab_id,
                insert_index: Some(2),
                destination: None,
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::TabMove { move_result, .. } = success.result else {
            panic!("expected tab move result");
        };
        assert!(move_result.changed);
        assert_eq!(app.state.workspaces[0].tabs[1].root_pane, moved_root);
    }

    #[test]
    fn api_tab_rename_reflows_active_tab_bar() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        let workspace = Workspace::test_new("tabs");
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.view.tab_bar_rect = ratatui::layout::Rect::new(0, 0, 60, 1);
        app.state.refresh_tab_bar_view();

        let tab_id = app.public_tab_id(0, 0).unwrap();
        let width_before = app.state.view.tab_hit_areas[0].width;

        app.handle_tab_rename(
            "req".into(),
            TabRenameParams {
                tab_id,
                label: "a much longer custom tab label".into(),
            },
        );

        let width_after = app.state.view.tab_hit_areas[0].width;
        assert!(
            width_after > width_before,
            "tab bar should reflow to the new label width immediately: \
             before={width_before}, after={width_after}"
        );
    }

    #[tokio::test]
    async fn tab_create_follows_cached_focused_pane_cwd_without_runtime() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        app.state.default_shell = exiting_test_command().into();
        app.state.shell_mode = ShellModeConfig::NonLogin;
        let workspace = Workspace::test_new("tabs");
        let focused_pane = workspace.tabs[0].root_pane;
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        let cached_cwd = std::env::temp_dir();
        let terminal_id = app.state.workspaces[0]
            .terminal_id(focused_pane)
            .cloned()
            .unwrap();
        app.state.terminals.get_mut(&terminal_id).unwrap().cwd = cached_cwd.clone();

        let response = app.handle_tab_create(
            "req".into(),
            TabCreateParams {
                workspace_id: None,
                cwd: None,
                focus: false,
                label: None,
                env: Default::default(),
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert!(matches!(success.result, ResponseResult::TabCreated { .. }));
        let created = &app.state.workspaces[0].tabs[1];
        let created_terminal_id = created.terminal_id(created.root_pane).unwrap();
        let created_cwd = &app.state.terminals.get(created_terminal_id).unwrap().cwd;
        assert_eq!(
            crate::worktree::canonical_or_original(created_cwd),
            crate::worktree::canonical_or_original(&cached_cwd)
        );
        shutdown_test_runtimes(&mut app);
    }
}
