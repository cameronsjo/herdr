use super::render::put_text;
use super::*;

pub(super) fn render_collapsed(
    buffer: &mut Buffer,
    area: Rect,
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) {
    let rows = agent_rows(endpoints, active_endpoint_id, config);
    for (index, row) in rows.into_iter().take(area.height as usize).enumerate() {
        let rect = Rect::new(area.x, area.y + index as u16, area.width, 1);
        if row.agent.focused {
            buffer.set_style(rect, Style::default().bg(config.palette.active_row_bg));
        }
        let initial = row.machine_label.chars().next().unwrap_or('?');
        put_text(
            buffer,
            rect.x,
            rect.y,
            rect.width,
            &format!(
                "{initial}{}",
                status_icon(row.agent.status, config.status_indicators)
            ),
            Style::default()
                .fg(if row.stale {
                    config.palette.overlay0
                } else {
                    status_color(row.agent.status, &config.palette)
                })
                .add_modifier(if row.stale {
                    Modifier::DIM
                } else {
                    Modifier::empty()
                }),
        );
        hits.endpoint_agents
            .push((rect, row.endpoint_id, row.agent.pane_id));
    }
}

pub(super) fn render_expanded(
    buffer: &mut Buffer,
    area: Rect,
    agent_view_label: Option<&str>,
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    hits: &mut ShellHitMap,
) {
    if !super::agent_sidebar::render_agent_panel_header(
        buffer,
        area,
        agent_view_label,
        config,
        hits,
    ) {
        return;
    }
    let rows = agent_rows(endpoints, active_endpoint_id, config);
    super::agent_sidebar::render_agent_list(
        buffer,
        area,
        &rows,
        agent_view_label.map(|_| " no matching agents"),
        config,
        agent_scroll,
        hits,
        |row| row.agent.row_lines(),
        |row, next| {
            if row.endpoint_id == next.endpoint_id {
                row.agent.gap_after(&next.agent, config.agents.row_gap)
            } else {
                config.agents.row_gap
            }
        },
        |buffer, rect, row, hits| {
            super::agent_sidebar::render_agent_row(buffer, rect, &row.agent, config);
            if row.stale {
                buffer.set_style(
                    rect,
                    Style::default()
                        .fg(config.palette.overlay0)
                        .add_modifier(Modifier::DIM),
                );
            }
            hits.endpoint_agents
                .push((rect, row.endpoint_id.clone(), row.agent.pane_id.clone()));
        },
    );
}

struct EndpointAgentRow {
    endpoint_id: ClientEndpointId,
    machine_label: String,
    stale: bool,
    agent: super::agent_sidebar::AgentRow,
}

/// The run a row belongs to: its machine and its group (workspace or token).
///
/// Keyed by machine as well as workspace, because two endpoints can advertise
/// the same workspace id and must still read as separate runs. The endpoint
/// index rather than its label: labels are display names and two connections
/// can share one.
fn run_key<'a>(
    row: &super::aggregate_navigation::AggregateAgentRow<'a>,
    group_by: &crate::config::AgentGroupBy,
) -> (usize, super::agent_sidebar::AgentGroupKey<&'a str>) {
    (
        row.endpoint.endpoint_index,
        super::agent_sidebar::agent_group_key(
            row.agent,
            &row.endpoint.snapshot.workspaces,
            group_by,
        ),
    )
}

fn agent_rows(
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    config: &ClientShellConfig,
) -> Vec<EndpointAgentRow> {
    // Rows are built in the order they will be drawn, not per endpoint: a
    // header belongs to the first row of a run, and only the aggregated order
    // says where the runs actually start.
    let ordered = super::aggregate_navigation::aggregate_agent_rows(
        endpoints,
        active_endpoint_id,
        config.agent_panel_sort,
        &config.agents.group_by,
    );
    let group_by = &config.agents.group_by;
    let grouped = group_by.is_grouped()
        && config.agent_panel_sort == crate::config::AgentPanelSortConfig::Spaces
        && super::agent_sidebar::runs_are_contiguous(&ordered, |row| run_key(row, group_by));

    ordered
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let snapshot = row.endpoint.snapshot;
            let key = run_key(row, group_by);
            let header = (grouped
                && index
                    .checked_sub(1)
                    .is_none_or(|previous| run_key(&ordered[previous], group_by) != key))
            .then(|| match key.1 {
                super::agent_sidebar::AgentGroupKey::Token(value) => Some(value),
                super::agent_sidebar::AgentGroupKey::Workspace(workspace_id) => snapshot
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.workspace_id == workspace_id)
                    .map(|workspace| workspace.label.as_str()),
            })
            .flatten();
            let mut agent = super::agent_sidebar::agent_row(
                snapshot,
                &row.agent.pane_id,
                config,
                Some(row.endpoint.label),
                super::agent_sidebar::AgentRowGrouping {
                    grouped,
                    header,
                    scope: Some(row.endpoint.endpoint_index),
                },
            )?;
            agent.focused &= row.endpoint.endpoint_id == active_endpoint_id;
            Some(EndpointAgentRow {
                endpoint_id: row.endpoint.endpoint_id.clone(),
                machine_label: row.endpoint.label.to_owned(),
                stale: row.endpoint.stale(),
                agent,
            })
        })
        .collect()
}
