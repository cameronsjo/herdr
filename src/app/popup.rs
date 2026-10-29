use std::path::PathBuf;

use crate::app::{App, Mode};
use crate::layout::PaneId;
use crate::pane::PaneLaunchEnv;
use crate::popup_size::{resolve_popup_geometry, PopupSize};
use crate::terminal::{TerminalId, TerminalRuntime, TerminalState};

/// A peek is meant for answering a prompt, so it takes most of the tab.
const PEEK_SIZE: PopupSize = PopupSize::Percent(80);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeekOpenError {
    PaneNotFound,
    /// The pane is already on screen in the active tab.
    PaneVisible,
    /// A popup that is not a peek is open.
    PopupOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PopupGeometry {
    pub width: Option<PopupSize>,
    pub height: Option<PopupSize>,
}

impl App {
    pub(crate) fn close_popup_pane(&mut self) -> bool {
        let Some(popup) = self.state.popup_pane.take() else {
            return false;
        };
        // A peek only borrowed the view: the terminal stays with its tiled pane,
        // and the next tab geometry pass gives it back its layout size.
        if popup.peek.is_none() {
            self.state
                .direct_attach_resize_locks
                .remove(&popup.terminal_id);
            self.state.terminals.remove(&popup.terminal_id);
            self.shutdown_terminal_runtime(popup.terminal_id);
        }
        self.state.mode = if self.state.active.is_some() {
            Mode::Terminal
        } else {
            Mode::Navigate
        };
        self.render_dirty.request_generic();
        self.render_notify.notify_one();
        true
    }

    /// Show a tiled pane from another tab in the popup overlay without moving
    /// it. The pane keeps its place in its own tab; only the view is borrowed.
    /// A peek that is already open is replaced; any other popup blocks it.
    pub(crate) fn open_peek_popup(
        &mut self,
        ws_idx: usize,
        pane_id: PaneId,
    ) -> Result<(), PeekOpenError> {
        if self
            .state
            .popup_pane
            .as_ref()
            .is_some_and(|popup| popup.peek.is_none())
        {
            return Err(PeekOpenError::PopupOpen);
        }
        let workspace = self
            .state
            .workspaces
            .get(ws_idx)
            .ok_or(PeekOpenError::PaneNotFound)?;
        let tab_idx = workspace
            .find_tab_index_for_pane(pane_id)
            .ok_or(PeekOpenError::PaneNotFound)?;
        let terminal_id = workspace
            .terminal_id(pane_id)
            .cloned()
            .ok_or(PeekOpenError::PaneNotFound)?;
        if self.state.active == Some(ws_idx) && workspace.active_tab_index() == tab_idx {
            return Err(PeekOpenError::PaneVisible);
        }
        if self.terminal_runtimes.get(&terminal_id).is_none() {
            return Err(PeekOpenError::PaneNotFound);
        }
        // The public id is always shown: a caller can set any label, so the
        // label alone cannot tell the operator which pane gets their keys.
        let public_id = self
            .public_pane_id(ws_idx, pane_id)
            .unwrap_or_else(|| "pane".to_owned());
        let label = match self
            .pane_info(ws_idx, pane_id)
            .and_then(|info| info.label.or(info.display_agent).or(info.agent))
        {
            Some(name) => format!("{name} ({public_id})"),
            None => public_id,
        };
        self.state.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id,
            terminal_id,
            width: Some(PEEK_SIZE),
            height: Some(PEEK_SIZE),
            peek: Some(crate::app::state::PopupPeek { label }),
        });
        self.state.mode = Mode::Terminal;
        self.render_dirty.request_generic();
        self.render_notify.notify_one();
        Ok(())
    }

    /// Close the popup only when it is a peek, leaving other popups alone.
    pub(crate) fn close_peek_popup(&mut self) -> bool {
        if self
            .state
            .popup_pane
            .as_ref()
            .is_some_and(|popup| popup.peek.is_some())
        {
            self.close_popup_pane()
        } else {
            false
        }
    }

    pub(crate) fn spawn_popup_shell_command(
        &mut self,
        command: &str,
        cwd: Option<PathBuf>,
        extra_env: Vec<(String, String)>,
        geometry: PopupGeometry,
    ) -> std::io::Result<()> {
        self.spawn_popup_command(
            cwd,
            extra_env,
            geometry,
            |pane_id, rows, cols, cwd, launch_env, app| {
                TerminalRuntime::spawn_shell_command(
                    pane_id,
                    rows,
                    cols,
                    cwd,
                    command,
                    launch_env,
                    crate::pane::AgentDetection::Disabled,
                    app.state.pane_scrollback_limit_bytes,
                    app.state.host_terminal_theme,
                    app.pane_host_terminal_appearance(),
                    app.event_tx.clone(),
                    app.render_notify.clone(),
                    app.render_dirty.clone(),
                )
                .map(|runtime| (runtime, None))
            },
        )
    }

    pub(crate) fn spawn_popup_argv_command(
        &mut self,
        argv: &[String],
        cwd: Option<PathBuf>,
        extra_env: Vec<(String, String)>,
        geometry: PopupGeometry,
    ) -> std::io::Result<()> {
        self.spawn_popup_command(
            cwd,
            extra_env,
            geometry,
            |pane_id, rows, cols, cwd, launch_env, app| {
                TerminalRuntime::spawn_argv_command(
                    pane_id,
                    rows,
                    cols,
                    cwd,
                    argv,
                    launch_env,
                    crate::pane::AgentDetection::Disabled,
                    app.state.pane_scrollback_limit_bytes,
                    app.state.host_terminal_theme,
                    app.pane_host_terminal_appearance(),
                    app.event_tx.clone(),
                    app.render_notify.clone(),
                    app.render_dirty.clone(),
                )
                .map(|runtime| (runtime, Some(argv.to_vec())))
            },
        )
    }

    fn spawn_popup_command<F>(
        &mut self,
        cwd: Option<PathBuf>,
        extra_env: Vec<(String, String)>,
        geometry: PopupGeometry,
        spawn: F,
    ) -> std::io::Result<()>
    where
        F: FnOnce(
            PaneId,
            u16,
            u16,
            PathBuf,
            &PaneLaunchEnv,
            &mut App,
        ) -> std::io::Result<(TerminalRuntime, Option<Vec<String>>)>,
    {
        if self.state.popup_pane.is_some() {
            return Err(std::io::Error::other("popup already open"));
        }
        let Some(ws_idx) = self.state.active else {
            return Err(std::io::Error::other("no active workspace"));
        };
        let ws = self
            .state
            .workspaces
            .get(ws_idx)
            .ok_or_else(|| std::io::Error::other("active workspace disappeared"))?;
        let active_tab = ws
            .active_tab()
            .ok_or_else(|| std::io::Error::other("active tab disappeared"))?;
        let focused_pane = ws
            .focused_pane_id()
            .ok_or_else(|| std::io::Error::other("active tab has no focused pane"))?;
        let cwd = cwd.or_else(|| {
            active_tab.cwd_for_pane(focused_pane, &self.state.terminals, &self.terminal_runtimes)
        });
        let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| "/".into()));
        let pane_id = PaneId::alloc();
        let terminal_id = TerminalId::alloc();
        let launch_env = PaneLaunchEnv::from_extra(extra_env).without_pane_identity();
        let terminal_area = if self.state.view.terminal_area.width >= 4
            && self.state.view.terminal_area.height >= 4
        {
            self.state.view.terminal_area
        } else {
            let (cols, rows) = self.state.headless_size;
            ratatui::layout::Rect::new(0, 0, cols, rows)
        };
        let Some(resolved_geometry) =
            resolve_popup_geometry(geometry.width, geometry.height, terminal_area)
        else {
            return Err(std::io::Error::other("terminal area too small for popup"));
        };
        let rows = resolved_geometry.inner.height;
        let cols = resolved_geometry.inner.width;
        let (runtime, launch_argv) = spawn(pane_id, rows, cols, cwd.clone(), &launch_env, self)?;
        let terminal = match launch_argv {
            Some(argv) => TerminalState::new(terminal_id.clone(), cwd).with_launch_argv(argv),
            None => TerminalState::new(terminal_id.clone(), cwd),
        };
        self.terminal_runtimes.insert(terminal_id.clone(), runtime);
        self.state.terminals.insert(terminal_id.clone(), terminal);
        self.state.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id,
            terminal_id,
            width: geometry.width,
            height: geometry.height,
            peek: None,
        });
        self.state.mode = Mode::Terminal;
        Ok(())
    }
}

#[cfg(test)]
impl App {
    pub(crate) fn install_test_popup_runtime(
        &mut self,
        runtime: TerminalRuntime,
    ) -> (PaneId, TerminalId) {
        let pane_id = PaneId::alloc();
        let terminal_id = TerminalId::alloc();
        self.terminal_runtimes.insert(terminal_id.clone(), runtime);
        self.state.terminals.insert(
            terminal_id.clone(),
            TerminalState::new(terminal_id.clone(), PathBuf::from("/popup")),
        );
        self.state.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id,
            terminal_id: terminal_id.clone(),
            width: None,
            height: None,
            peek: None,
        });
        (pane_id, terminal_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_popup() -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &crate::config::Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("popup")];
        app.state.active = Some(0);
        app.state.selected = 0;
        let terminal_id = TerminalId::alloc();
        app.state.terminals.insert(
            terminal_id.clone(),
            TerminalState::new(terminal_id.clone(), PathBuf::from("/popup")),
        );
        app.state.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id: PaneId::alloc(),
            terminal_id,
            width: None,
            height: None,
            peek: None,
        });
        app
    }

    #[test]
    fn close_popup_uses_terminal_mode_with_active_workspace() {
        let mut app = app_with_popup();
        app.state.mode = Mode::Navigate;

        assert!(app.close_popup_pane());

        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn close_popup_uses_navigate_mode_without_active_workspace() {
        let mut app = app_with_popup();
        app.state.workspaces.clear();
        app.state.active = None;
        app.state.mode = Mode::Navigate;

        assert!(app.close_popup_pane());

        assert_eq!(app.state.mode, Mode::Navigate);
    }

    #[test]
    fn close_popup_clears_direct_attach_resize_lock() {
        let mut app = app_with_popup();
        let terminal_id = app.state.popup_pane.as_ref().unwrap().terminal_id.clone();
        app.state
            .direct_attach_resize_locks
            .insert(terminal_id.clone());

        assert!(app.close_popup_pane());

        assert!(!app.state.direct_attach_resize_locks.contains(&terminal_id));
    }

    #[test]
    fn popup_survives_background_workspace_removal() {
        let mut app = app_with_popup();
        app.state.workspaces.clear();
        app.state.active = None;

        app.state.assert_invariants_for_test();

        assert!(app.state.popup_pane.is_some());
    }

    #[test]
    fn popup_close_api_closes_only_active_popup() {
        let mut app = app_with_popup();
        let close = || crate::api::schema::Request {
            id: "close-popup".into(),
            method: crate::api::schema::Method::PopupClose(
                crate::api::schema::EmptyParams::default(),
            ),
        };

        let response = app.handle_api_request(close());
        let response: crate::api::schema::SuccessResponse =
            serde_json::from_str(&response).unwrap();
        assert_eq!(response.result, crate::api::schema::ResponseResult::Ok {});

        let response = app.handle_api_request(close());
        let response: crate::api::schema::ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(response.error.code, "popup_not_open");
    }

    /// One workspace with two tabs, each with a runtime-backed pane. Tab 0 is
    /// active, so tab 1's pane is the one a peek can borrow.
    fn app_with_peekable_pane() -> (App, String, PaneId) {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &crate::config::Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let mut workspace = crate::workspace::Workspace::test_new("peek");
        workspace.test_add_tab(Some("worker"));
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        for tab in 0..2 {
            let pane_id = app.state.workspaces[0].tabs[tab].root_pane;
            let terminal_id = app.state.workspaces[0]
                .terminal_id(pane_id)
                .unwrap()
                .clone();
            app.terminal_runtimes
                .insert(terminal_id, TerminalRuntime::test_with_channel(40, 10).0);
        }
        let worker = app.state.workspaces[0].tabs[1].root_pane;
        let public = app.public_pane_id(0, worker).unwrap();
        (app, public, worker)
    }

    fn request(method: crate::api::schema::Method) -> crate::api::schema::Request {
        crate::api::schema::Request {
            id: "peek".into(),
            method,
        }
    }

    fn peek(pane_id: &str) -> crate::api::schema::Request {
        request(crate::api::schema::Method::PanePeek(
            crate::api::schema::PaneTarget {
                pane_id: pane_id.to_owned(),
            },
        ))
    }

    fn peek_close() -> crate::api::schema::Request {
        request(crate::api::schema::Method::PanePeekClose(
            crate::api::schema::EmptyParams::default(),
        ))
    }

    fn error_code(response: &str) -> String {
        serde_json::from_str::<crate::api::schema::ErrorResponse>(response)
            .unwrap()
            .error
            .code
    }

    #[tokio::test]
    async fn peek_borrows_a_pane_without_moving_it() {
        let (mut app, public, worker) = app_with_peekable_pane();
        let terminal_id = app.state.workspaces[0].terminal_id(worker).unwrap().clone();

        let response = app.handle_api_request(peek(&public));
        serde_json::from_str::<crate::api::schema::SuccessResponse>(&response).expect(&response);

        let popup = app.state.popup_pane.as_ref().unwrap();
        assert_eq!(popup.pane_id, worker);
        assert_eq!(popup.terminal_id, terminal_id);
        assert!(popup.peek.is_some());
        assert_eq!(
            app.state.workspaces[0].find_tab_index_for_pane(worker),
            Some(1)
        );
        assert!(app.state.tab_resize_locked(&terminal_id));
        app.state.assert_invariants_for_test();

        let response = app.handle_api_request(peek_close());
        serde_json::from_str::<crate::api::schema::SuccessResponse>(&response).unwrap();

        assert!(app.state.popup_pane.is_none());
        assert!(app.state.terminals.contains_key(&terminal_id));
        assert!(app.terminal_runtimes.get(&terminal_id).is_some());
        assert!(!app.state.tab_resize_locked(&terminal_id));
        app.state.assert_invariants_for_test();
        assert_eq!(
            error_code(&app.handle_api_request(peek_close())),
            "peek_not_open"
        );
    }

    #[tokio::test]
    async fn peek_refuses_a_pane_already_on_screen() {
        let (mut app, _, _) = app_with_peekable_pane();
        let visible = app.state.workspaces[0].tabs[0].root_pane;
        let public = app.public_pane_id(0, visible).unwrap();

        assert_eq!(
            error_code(&app.handle_api_request(peek(&public))),
            "pane_visible"
        );
        assert_eq!(
            error_code(&app.handle_api_request(peek("w9:p9"))),
            "pane_not_found"
        );
        assert!(app.state.popup_pane.is_none());
    }

    #[tokio::test]
    async fn peek_replaces_a_peek_but_not_another_popup() {
        let (mut app, public, worker) = app_with_peekable_pane();
        app.handle_api_request(peek(&public));
        app.handle_api_request(peek(&public));
        assert_eq!(app.state.popup_pane.as_ref().unwrap().pane_id, worker);

        app.close_popup_pane();
        let (popup_pane, _) =
            app.install_test_popup_runtime(TerminalRuntime::test_with_channel(10, 4).0);
        assert_eq!(
            error_code(&app.handle_api_request(peek(&public))),
            "popup_open"
        );
        assert_eq!(
            error_code(&app.handle_api_request(peek_close())),
            "peek_not_open"
        );
        assert_eq!(app.state.popup_pane.as_ref().unwrap().pane_id, popup_pane);
    }

    #[tokio::test]
    async fn peek_closes_when_its_pane_terminal_shuts_down() {
        let (mut app, public, worker) = app_with_peekable_pane();
        app.handle_api_request(peek(&public));
        let terminal_id = app.state.workspaces[0].terminal_id(worker).unwrap().clone();

        app.shutdown_terminal_runtime(terminal_id);

        assert!(app.state.popup_pane.is_none());
    }

    #[tokio::test]
    async fn peeked_pane_exit_closes_the_peek_and_still_removes_the_pane() {
        let (mut app, public, worker) = app_with_peekable_pane();
        app.handle_api_request(peek(&public));

        app.handle_internal_event(crate::events::AppEvent::PaneDied {
            pane_id: worker,
            exit_reason: crate::platform::ChildExitReason::Exited,
        });

        assert!(app.state.popup_pane.is_none());
        assert!(
            app.find_pane(worker).is_none(),
            "the pane's own exit handling ran"
        );
    }

    #[tokio::test]
    async fn peek_title_always_names_the_pane_id() {
        let (mut app, public, _) = app_with_peekable_pane();
        app.handle_api_request(peek(&public));

        let label = &app
            .state
            .popup_pane
            .as_ref()
            .unwrap()
            .peek
            .as_ref()
            .unwrap()
            .label;
        assert!(label.contains(&public), "{label}");
    }
}
