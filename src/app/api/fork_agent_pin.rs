//! Fork-only handler for `agent.pin` and `agent.unpin`. See `crate::fork_agent_pin`.

use crate::api::schema::{AgentTarget, ResponseResult};
use crate::app::terminal_targets::TerminalTargetError;
use crate::app::App;

use super::responses::{encode_error_body, encode_success};

impl App {
    /// Pins or unpins the target agent. Pinning an agent that is already
    /// pinned keeps its place, and unpinning one that is not pinned is a no-op;
    /// both report the agent as it stands.
    ///
    /// Unpin also takes a pane id whose agent is no longer detected, so a pin
    /// left behind by detection uncertainty can always be removed; it clears
    /// the pin and still reports `agent_not_found`.
    pub(super) fn handle_agent_pin(
        &mut self,
        id: String,
        target: AgentTarget,
        pin: bool,
    ) -> String {
        let resolved = match self.resolve_agent_target(&target.target) {
            Ok(resolved) => resolved,
            Err(err) => {
                if !pin {
                    self.clear_pin_on_pane(&target.target);
                }
                return encode_error_body(id, self.agent_target_error_body(err));
            }
        };
        let next = crate::fork_agent_pin::next_pin_order(
            self.state
                .terminals
                .values()
                .filter_map(|terminal| terminal.pin_order),
        );
        let terminal = self
            .state
            .workspaces
            .get(resolved.ws_idx)
            .and_then(|ws| ws.pane_state(resolved.pane_id))
            .map(|pane| pane.attached_terminal_id.clone())
            .and_then(|terminal_id| self.state.terminals.get_mut(&terminal_id));
        let Some(terminal) = terminal else {
            let err = TerminalTargetError::NotFound {
                target: target.target,
            };
            return encode_error_body(id, self.agent_target_error_body(err));
        };
        let changed = match (pin, terminal.pin_order) {
            (true, None) => {
                terminal.pin_order = Some(next);
                true
            }
            (false, Some(_)) => {
                terminal.pin_order = None;
                true
            }
            _ => false,
        };
        if changed {
            // The pin is in the session snapshot, so a change is saved.
            self.state.mark_session_dirty();
            self.emit_pane_updated(resolved.ws_idx, resolved.pane_id);
        }
        match self.agent_info(resolved.ws_idx, resolved.pane_id) {
            Some(agent) => encode_success(id, ResponseResult::AgentInfo { agent }),
            None => encode_error_body(
                id,
                self.agent_target_error_body(TerminalTargetError::NotFound {
                    target: target.target,
                }),
            ),
        }
    }

    fn clear_pin_on_pane(&mut self, target: &str) {
        let Some((ws_idx, pane_id)) = self.parse_current_public_pane_id(target) else {
            return;
        };
        let terminal_id = self
            .state
            .workspaces
            .get(ws_idx)
            .and_then(|ws| ws.pane_state(pane_id))
            .map(|pane| pane.attached_terminal_id.clone());
        let Some(terminal) = terminal_id.and_then(|id| self.state.terminals.get_mut(&id)) else {
            return;
        };
        if terminal.pin_order.take().is_some() {
            self.state.mark_session_dirty();
            self.emit_pane_updated(ws_idx, pane_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::schema::{AgentTarget, ErrorResponse, ResponseResult, SuccessResponse};
    use crate::app::{App, Mode};
    use crate::config::Config;
    use crate::detect::{Agent, AgentState};
    use crate::workspace::Workspace;

    /// Two workspaces, each with one pane running an agent.
    fn app_with_two_agents() -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        for terminal in app.state.terminals.values_mut() {
            terminal.set_detected_state(Some(Agent::Pi), AgentState::Idle);
        }
        app
    }

    fn target(app: &App, ws_idx: usize) -> AgentTarget {
        let pane_id = app.state.workspaces[ws_idx].tabs[0].root_pane;
        AgentTarget {
            target: app.public_pane_id(ws_idx, pane_id).unwrap(),
        }
    }

    fn pin_order(app: &mut App, ws_idx: usize, pin: bool) -> Option<u64> {
        let target = target(app, ws_idx);
        let response = app.handle_agent_pin("req".into(), target, pin);
        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::AgentInfo { agent } = success.result else {
            panic!("expected agent info, got {response}");
        };
        agent.pin_order
    }

    #[test]
    fn a_pin_change_marks_the_session_for_saving_and_a_no_op_does_not() {
        let mut app = app_with_two_agents();
        app.state.session_dirty = false;
        assert_eq!(pin_order(&mut app, 0, true), Some(1));
        assert!(app.state.session_dirty);

        app.state.session_dirty = false;
        assert_eq!(pin_order(&mut app, 0, true), Some(1));
        assert!(!app.state.session_dirty, "re-pinning changes nothing");

        assert_eq!(pin_order(&mut app, 0, false), None);
        assert!(app.state.session_dirty);
    }

    #[test]
    fn pins_keep_the_order_they_were_made_in() {
        let mut app = app_with_two_agents();
        assert_eq!(pin_order(&mut app, 1, true), Some(1));
        assert_eq!(pin_order(&mut app, 0, true), Some(2));
        // Pinning again keeps the agent's place.
        assert_eq!(pin_order(&mut app, 1, true), Some(1));

        assert_eq!(pin_order(&mut app, 1, false), None);
        // Unpinning an agent that is not pinned reports it as it stands.
        assert_eq!(pin_order(&mut app, 1, false), None);
        // A new pin goes after the agents still pinned.
        assert_eq!(pin_order(&mut app, 1, true), Some(3));
        assert_eq!(
            app.collect_agent_infos()
                .iter()
                .filter_map(|agent| agent.pin_order)
                .collect::<std::collections::BTreeSet<_>>(),
            [2, 3].into(),
        );
    }

    #[test]
    fn unpin_clears_a_pin_left_on_a_pane_whose_agent_is_gone() {
        let mut app = app_with_two_agents();
        assert_eq!(pin_order(&mut app, 0, true), Some(1));
        let target = target(&app, 0);
        let pane_id = app.state.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.state.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_detected_state(None, AgentState::Unknown);

        let response = app.handle_agent_pin("req".into(), target, false);
        let error: ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(error.error.code, "agent_not_found");
        assert_eq!(app.state.terminals[&terminal_id].pin_order, None);
    }

    #[test]
    fn a_respawned_terminal_drops_its_pin() {
        let mut app = app_with_two_agents();
        assert_eq!(pin_order(&mut app, 0, true), Some(1));
        for terminal in app.state.terminals.values_mut() {
            terminal.clear_agent_runtime_identity_after_respawn();
        }
        assert!(app.state.terminals.values().all(|t| t.pin_order.is_none()));
    }

    #[test]
    fn a_pane_without_an_agent_cannot_be_pinned() {
        let mut app = app_with_two_agents();
        let target = target(&app, 0);
        for terminal in app.state.terminals.values_mut() {
            terminal.set_detected_state(None, AgentState::Unknown);
        }
        let response = app.handle_agent_pin("req".into(), target, true);
        let error: ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(error.error.code, "agent_not_found");
        assert!(app.state.terminals.values().all(|t| t.pin_order.is_none()));
    }
}
