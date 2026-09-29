//! Fork-only agents-panel rows for agents registered outside any pane.
//!
//! The endpoint sends its registry as the optional `fork.registered-agents.v1`
//! control. These rows reuse the panel's own row builder by describing the
//! registered agents as a small snapshot of their own, under one synthetic
//! "elsewhere" space, so token formats, grouping and status styling match the
//! pane rows exactly. The rows carry `PANE_ID_PREFIX` ids and are never
//! focusable.

use crate::api::schema::{AgentStatus, RegisteredAgentInfo};
use crate::fork_registered_agents::PANE_ID_PREFIX;
use crate::protocol::{ClientShellAgent, ClientShellSnapshot, ClientShellWorkspace};

use super::agent_sidebar::{agent_rows, AgentRow};
use super::ClientShellConfig;

const SPACE_LABEL: &str = "elsewhere";

pub(super) fn is_registered_row(pane_id: &str) -> bool {
    pane_id.starts_with(PANE_ID_PREFIX)
}

/// Rows for `registered`, drawn after the pane rows. An active agent view
/// filters panes only, so registered agents are hidden while one is set.
pub(super) fn registered_agent_rows(
    snapshot: &ClientShellSnapshot,
    registered: &[RegisteredAgentInfo],
    config: &ClientShellConfig,
) -> Vec<AgentRow> {
    if registered.is_empty() || snapshot.agent_view_label.is_some() {
        return Vec::new();
    }
    agent_rows(&registered_snapshot(registered), config, None)
}

fn registered_snapshot(registered: &[RegisteredAgentInfo]) -> ClientShellSnapshot {
    ClientShellSnapshot {
        boot_id: String::new(),
        revision: 0,
        config_diagnostic: None,
        product_announcement: None,
        update_available: None,
        update_install_command: String::new(),
        server_keybindings_toml: None,
        latest_release_notes_available: false,
        integration_updates_available: false,
        worktree_directory: String::new(),
        release_notes: None,
        focused_workspace_id: None,
        focused_tab_id: None,
        focused_pane_id: None,
        tab_bar_right: Vec::new(),
        tab_bar_right_separator: String::new(),
        agent_view_label: None,
        agent_order: Vec::new(),
        workspaces: vec![ClientShellWorkspace {
            workspace_id: PANE_ID_PREFIX.to_owned(),
            active_tab_id: String::new(),
            new_workspace_cwd: String::new(),
            number: 0,
            label: SPACE_LABEL.to_owned(),
            custom_label: true,
            branch: None,
            git_ahead_behind: None,
            tokens: Vec::new(),
            worktree: None,
            focused: false,
            agent_status: AgentStatus::Unknown,
        }],
        tabs: Vec::new(),
        panes: Vec::new(),
        agents: registered.iter().map(registered_agent).collect(),
        commands: Vec::new(),
    }
}

fn registered_agent(info: &RegisteredAgentInfo) -> ClientShellAgent {
    // The cwd's last component stands in for the pane label, the way a pane
    // row names its checkout.
    let title = info.cwd.as_deref().and_then(|cwd| {
        std::path::Path::new(cwd)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
    });
    ClientShellAgent {
        pane_id: format!("{PANE_ID_PREFIX}{}/{}", info.source, info.name),
        workspace_id: PANE_ID_PREFIX.to_owned(),
        tab_id: String::new(),
        name: Some(info.name.clone()),
        display_agent: None,
        agent: info.agent.clone(),
        title,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: info.status,
        state_change_seq: 0,
        state_labels: Vec::new(),
        tokens: info
            .tokens
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        focused: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(name: &str, cwd: Option<&str>) -> RegisteredAgentInfo {
        RegisteredAgentInfo {
            source: "test".into(),
            name: name.into(),
            agent: Some("claude".into()),
            status: AgentStatus::Blocked,
            cwd: cwd.map(str::to_owned),
            tokens: Default::default(),
            ttl_ms: 60_000,
        }
    }

    #[test]
    fn registered_agents_become_unfocusable_rows_in_one_space() {
        let snapshot =
            registered_snapshot(&[info("one", Some("/work/cadence")), info("two", None)]);
        assert_eq!(snapshot.workspaces.len(), 1);
        assert_eq!(snapshot.workspaces[0].label, SPACE_LABEL);
        assert_eq!(snapshot.agents[0].title.as_deref(), Some("cadence"));
        assert_eq!(snapshot.agents[1].title, None);
        assert!(snapshot
            .agents
            .iter()
            .all(|agent| is_registered_row(&agent.pane_id) && !agent.focused));
        assert_eq!(snapshot.agents[0].agent_status, AgentStatus::Blocked);
    }

    fn render_panel(
        registered: &[RegisteredAgentInfo],
        agent_view_label: Option<&str>,
    ) -> (String, super::super::ShellHitMap) {
        let mut snapshot = registered_snapshot(&[]);
        snapshot.agent_view_label = agent_view_label.map(str::to_owned);
        let config = ClientShellConfig::from_config(&crate::config::Config::default());
        let area = ratatui::layout::Rect::new(0, 0, 40, 12);
        let mut buffer = ratatui::buffer::Buffer::empty(area);
        let mut hits = super::super::ShellHitMap::default();
        let mut scroll = 0;
        super::super::agent_sidebar::render_agent_panel(
            &mut buffer,
            area,
            &snapshot,
            registered,
            &config,
            &mut scroll,
            &mut hits,
        );
        let text = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        (text, hits)
    }

    #[test]
    fn panel_draws_registered_agents_without_click_targets() {
        let (text, hits) = render_panel(&[info("remote-one", Some("/work/cadence"))], None);
        assert!(text.contains("remote-one"), "{text}");
        assert!(hits.agents.is_empty());
    }

    #[test]
    fn endpoint_control_round_trips_and_ignores_bad_payloads() {
        let mut registry = crate::fork_registered_agents::RegisteredAgents::default();
        registry
            .register(
                crate::api::schema::AgentRegisterParams {
                    source: "test".into(),
                    name: "remote-one".into(),
                    agent: None,
                    status: None,
                    cwd: None,
                    tokens: Default::default(),
                    ttl_ms: None,
                },
                std::time::Instant::now(),
            )
            .unwrap();
        let Some(crate::protocol::ServerMessage::EndpointControl { kind, data }) =
            crate::fork_registered_agents::client_update(&registry, &mut None)
        else {
            panic!("expected a registered-agents control");
        };
        let crate::client::endpoint::EndpointControlMessage::RegisteredAgents(agents) =
            crate::client::endpoint::decode_endpoint_control(&kind, &data).unwrap()
        else {
            panic!("expected decoded registered agents");
        };
        assert_eq!(agents, registry.list());
        // A payload that no longer parses clears the list instead of keeping
        // rows the server can no longer vouch for.
        let crate::client::endpoint::EndpointControlMessage::RegisteredAgents(agents) =
            crate::client::endpoint::decode_endpoint_control(&kind, "not json").unwrap()
        else {
            panic!("expected decoded registered agents");
        };
        assert!(agents.is_empty());
    }

    #[test]
    fn active_agent_view_hides_registered_agents() {
        let (text, _) = render_panel(&[info("remote-one", None)], Some("focus"));
        assert!(!text.contains("remote-one"), "{text}");
    }
}
