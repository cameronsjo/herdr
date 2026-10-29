//! Fork-only handler for `agent.pin` and `agent.unpin`. See `crate::fork_agent_pin`.

use crate::api::schema::{AgentTarget, ResponseResult};
use crate::app::terminal_targets::TerminalTargetError;
use crate::app::App;

use super::responses::{encode_error_body, encode_success};

impl App {
    /// Pins or unpins the target agent. Pinning an agent that is already
    /// pinned keeps its place, and unpinning one that is not pinned is a no-op;
    /// both report the agent as it stands.
    pub(super) fn handle_agent_pin(
        &mut self,
        id: String,
        target: AgentTarget,
        pin: bool,
    ) -> String {
        let resolved = match self.resolve_agent_target(&target.target) {
            Ok(resolved) => resolved,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
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
