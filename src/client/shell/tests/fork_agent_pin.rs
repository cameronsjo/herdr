//! Fork: pinned agents lead the agents panel (cameronsjo/herdr#96).

use super::agents_worktrees_notifications::{
    agent_headers, grouped_agents_snapshot, grouped_config, token_agents_snapshot, token_config,
};
use super::*;

/// Pins each pane in `pins`, in that order.
fn pinned(mut snapshot: ClientShellSnapshot, pins: &[&str]) -> ClientShellSnapshot {
    for (order, pane_id) in pins.iter().enumerate() {
        let agent = snapshot
            .agents
            .iter_mut()
            .find(|agent| agent.pane_id == *pane_id)
            .expect("pinned pane is an agent");
        agent.tokens.push((
            crate::fork_agent_pin::PIN_TOKEN.into(),
            (order + 1).to_string(),
        ));
    }
    snapshot
}

fn drawn(config: &Config, snapshot: ClientShellSnapshot) -> (ClientShellState, FrameData) {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(config));
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 40).expect("agents sidebar").frame;
    (state, frame)
}

fn order(state: &ClientShellState) -> Vec<&str> {
    state
        .hits
        .agents
        .iter()
        .map(|(_, pane_id)| pane_id.as_str())
        .collect()
}

#[test]
fn pinned_agents_lead_in_the_order_they_were_pinned() {
    let (state, _) = drawn(
        &Config::default(),
        pinned(grouped_agents_snapshot(), &["pane_4", "pane_2"]),
    );
    assert_eq!(order(&state), vec!["pane_4", "pane_2", "pane_1", "pane_3"]);
}

#[test]
fn pinned_agents_get_their_own_header_when_grouped() {
    let (state, frame) = drawn(
        &grouped_config(),
        pinned(grouped_agents_snapshot(), &["pane_3"]),
    );
    assert_eq!(order(&state), vec!["pane_3", "pane_1", "pane_2", "pane_4"]);
    assert_eq!(
        agent_headers(&state, &frame),
        vec![
            ("pane_3".to_owned(), " pinned".to_owned()),
            ("pane_1".to_owned(), " alpha".to_owned()),
            ("pane_4".to_owned(), " beta".to_owned()),
        ],
    );
}

#[test]
fn pinned_agents_lead_a_token_grouped_panel() {
    let (state, frame) = drawn(
        &token_config(),
        pinned(token_agents_snapshot(), &["pane_1"]),
    );
    assert_eq!(order(&state)[0], "pane_1");
    assert_eq!(
        agent_headers(&state, &frame)
            .into_iter()
            .map(|(_, header)| header)
            .collect::<Vec<_>>()
            .first()
            .map(String::as_str),
        Some(" pinned"),
    );
}

#[test]
fn pinned_agents_lead_the_priority_order_whatever_their_status() {
    let mut snapshot = pinned(grouped_agents_snapshot(), &["pane_4"]);
    snapshot.agents[0].agent_status = AgentStatus::Blocked;
    let mut config = grouped_config();
    config.ui.agent_panel_sort = crate::config::AgentPanelSortConfig::Priority;
    let (state, _) = drawn(&config, snapshot);
    assert_eq!(order(&state)[..2], ["pane_4", "pane_1"]);
}

#[test]
fn blocked_first_never_moves_an_agent_above_the_pins() {
    let mut snapshot = pinned(grouped_agents_snapshot(), &["pane_3", "pane_4"]);
    for agent in &mut snapshot.agents {
        if matches!(agent.pane_id.as_str(), "pane_2" | "pane_4") {
            agent.agent_status = AgentStatus::Blocked;
        }
    }
    let mut config = Config::default();
    config.ui.sidebar.agents.blocked_first = true;
    let (state, _) = drawn(&config, snapshot);
    assert_eq!(order(&state), vec!["pane_3", "pane_4", "pane_2", "pane_1"]);
}

#[test]
fn an_active_view_keeps_its_filter_and_pins_lead_what_it_shows() {
    let mut snapshot = pinned(grouped_agents_snapshot(), &["pane_4", "pane_2"]);
    snapshot.agent_view_label = Some("review".into());
    snapshot.agent_order = vec!["pane_1".into(), "pane_3".into(), "pane_4".into()];
    let (state, _) = drawn(&Config::default(), snapshot);
    assert_eq!(order(&state), vec!["pane_4", "pane_1", "pane_3"]);
}

#[test]
fn agent_navigation_follows_the_pinned_order() {
    let (mut state, _) = drawn(
        &Config::default(),
        pinned(grouped_agents_snapshot(), &["pane_3"]),
    );
    let mut next = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::FocusAgent(0)),
        &mut next,
    );
    assert!(matches!(
        &next.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                crate::api::schema::Method::PaneFocus(target) if target.pane_id == "pane_3"
            )
    ));
}
