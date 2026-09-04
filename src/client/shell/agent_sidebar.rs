use std::collections::HashMap;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use super::*;
use crate::protocol::ClientShellAgent;

struct AgentRow<'a> {
    pane_id: &'a str,
    workspace_id: &'a str,
    /// Workspace label this entry draws above its own rows, set only when the
    /// entry starts a workspace run while grouping is on.
    header: Option<&'a str>,
    status: crate::api::schema::AgentStatus,
    focused: bool,
    rows: Vec<Vec<crate::ui::ResolvedToken>>,
}

impl AgentRow<'_> {
    fn header_rows(&self) -> u16 {
        u16::from(self.header.is_some())
    }
}

/// The one definition of an entry's height in the panel body.
///
/// The scroll-metrics pass and the render loop both go through this, so the
/// layout the scrollbar and the hit-test rects report cannot drift from the
/// rows actually drawn. Headers make the two genuinely different arithmetic,
/// which is why they share a function rather than agreeing by coincidence.
fn agent_entry_height_from_rows(rows_len: usize, header_rows: u16, body_height: u16) -> u16 {
    (rows_len
        .max(1)
        .saturating_add(usize::from(header_rows))
        .min(u16::MAX as usize) as u16)
        .min(body_height)
}

/// Whether the agent panel draws one workspace header per contiguous run.
///
/// One gate drives both the headers and the row layout. `agent_panel_sort` is a
/// one-click toggle, so gating the header on the live sort while gating the
/// layout on config would leave a `priority` user with no workspace shown at
/// all.
///
/// A header must never label agents from another workspace, so the order about
/// to be rendered has to keep each workspace in one run. The endpoint's
/// snapshot carries an active agent view's resulting order but not the sort
/// clause that produced it, so the order itself is what gets checked: a
/// filter-only view keeps space order and stays grouped, while a view that
/// interleaves workspaces turns grouping off.
fn agent_grouping_is_effective(
    entries: &[(&ClientShellAgent, &ClientShellWorkspace)],
    config: &ClientShellConfig,
) -> bool {
    config.agents.group_by == crate::config::AgentGroupBy::Workspace
        && config.agent_panel_sort == crate::config::AgentPanelSortConfig::Spaces
        && workspaces_are_contiguous(entries)
}

/// Whether every workspace occupies exactly one run of the ordered entries.
///
/// The inner scan runs only at a run boundary, so the cost is bounded by the
/// number of runs — at most the workspace count — rather than the entry count
/// squared, and it allocates nothing inside this per-frame path.
fn workspaces_are_contiguous(entries: &[(&ClientShellAgent, &ClientShellWorkspace)]) -> bool {
    entries.iter().enumerate().all(|(index, (agent, _))| {
        let Some(previous) = index.checked_sub(1) else {
            return true;
        };
        entries[previous].0.workspace_id == agent.workspace_id
            || !entries[..previous]
                .iter()
                .any(|(earlier, _)| earlier.workspace_id == agent.workspace_id)
    })
}

pub(super) fn ordered_agent_pane_ids(
    snapshot: &ClientShellSnapshot,
    sort: crate::config::AgentPanelSortConfig,
) -> Vec<String> {
    if snapshot.agent_view_label.is_some() {
        return snapshot
            .agent_order
            .iter()
            .filter(|pane_id| {
                snapshot
                    .agents
                    .iter()
                    .any(|agent| agent.pane_id == pane_id.as_str())
            })
            .cloned()
            .collect();
    }
    let mut agents = snapshot.agents.iter().collect::<Vec<_>>();
    if sort == crate::config::AgentPanelSortConfig::Priority {
        agents.sort_by_key(|agent| {
            (
                std::cmp::Reverse(status_priority(agent.agent_status)),
                std::cmp::Reverse(agent.state_change_seq),
            )
        });
    }
    agents
        .into_iter()
        .map(|agent| agent.pane_id.clone())
        .collect()
}

pub(super) fn render_agent_panel(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    hits: &mut ShellHitMap,
) {
    if area.height == 0 {
        return;
    }
    put_text(
        buffer,
        area.x,
        area.y,
        area.width,
        &"─".repeat(area.width as usize),
        Style::default().fg(config.palette.surface_dim),
    );
    if area.height < 2 {
        return;
    }
    put_text(
        buffer,
        area.x,
        area.y + 1,
        area.width,
        " agents",
        Style::default()
            .fg(config.palette.overlay0)
            .add_modifier(Modifier::BOLD),
    );
    let sort_label =
        snapshot
            .agent_view_label
            .as_deref()
            .unwrap_or(match config.agent_panel_sort {
                crate::config::AgentPanelSortConfig::Spaces => "grouped",
                crate::config::AgentPanelSortConfig::Priority => "priority",
            });
    let sort_width = display_width(sort_label).min(area.width as usize) as u16;
    let sort_rect = Rect::new(
        area.right().saturating_sub(sort_width),
        area.y + 1,
        sort_width,
        1,
    );
    hits.agent_sort_toggle = if config.mouse_capture && snapshot.agent_view_label.is_none() {
        sort_rect
    } else {
        Rect::default()
    };
    put_text(
        buffer,
        sort_rect.x,
        sort_rect.y,
        sort_rect.width,
        sort_label,
        Style::default()
            .fg(if snapshot.agent_view_label.is_some() {
                config.palette.accent
            } else {
                config.palette.overlay0
            })
            .add_modifier(Modifier::BOLD),
    );

    let (rows, grouped) = agent_rows(snapshot, config);
    let body = Rect::new(
        area.x,
        area.y.saturating_add(3),
        area.width,
        area.height.saturating_sub(3),
    );
    hits.agent_body = body;
    if body.is_empty() || rows.is_empty() {
        *agent_scroll = 0;
        if !body.is_empty() && snapshot.agent_view_label.is_some() {
            put_text(
                buffer,
                body.x,
                body.y,
                body.width,
                " no matching agents",
                Style::default()
                    .fg(config.palette.overlay0)
                    .add_modifier(Modifier::DIM),
            );
        }
        return;
    }

    let row_heights = rows
        .iter()
        .map(|row| agent_entry_height_from_rows(row.rows.len(), row.header_rows(), body.height))
        .collect::<Vec<_>>();
    let gaps = rows
        .iter()
        .enumerate()
        .map(|(index, row)| match rows.get(index + 1) {
            None => 0,
            // Grouping packs a workspace run under its shared header, so the
            // gap separates runs rather than individual agents.
            Some(next) if grouped && next.workspace_id == row.workspace_id => 0,
            Some(_) => config.agents.row_gap,
        })
        .collect::<Vec<_>>();
    let metrics =
        super::scroll::list_scroll_metrics(&row_heights, &gaps, body.height, *agent_scroll);
    hits.agent_max_scroll = metrics.max_offset_from_bottom;
    hits.agent_scroll_metrics = Some(metrics);
    *agent_scroll = metrics
        .max_offset_from_bottom
        .saturating_sub(metrics.offset_from_bottom);
    let show_scrollbar = metrics.max_offset_from_bottom > 0 && body.width > 1;
    let content_width = body.width.saturating_sub(u16::from(show_scrollbar));
    let mut y = body.y;
    for (index, row) in rows.iter().enumerate().skip(*agent_scroll) {
        let height = row_heights[index];
        if y.saturating_add(height) > body.bottom() {
            break;
        }
        // The header belongs to the entry that draws it, so a click anywhere in
        // this rect — header row included — focuses the run's first agent.
        let rect = Rect::new(body.x, y, content_width, height);
        hits.agents.push((rect, row.pane_id.to_string()));
        render_agent_row(buffer, rect, row, grouped, config);
        y = y.saturating_add(height).saturating_add(gaps[index]);
    }

    if show_scrollbar {
        let track = Rect::new(body.right().saturating_sub(1), body.y, 1, body.height);
        hits.agent_scrollbar = track;
        super::scroll::render_list_scrollbar(buffer, track, metrics, &config.palette);
    }
}

fn agent_rows<'a>(
    snapshot: &'a ClientShellSnapshot,
    config: &ClientShellConfig,
) -> (Vec<AgentRow<'a>>, bool) {
    let entries = ordered_agent_pane_ids(snapshot, config.agent_panel_sort)
        .into_iter()
        .filter_map(|pane_id| {
            let agent = snapshot
                .agents
                .iter()
                .find(|agent| agent.pane_id == pane_id)?;
            let workspace = snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == agent.workspace_id)?;
            Some((agent, workspace))
        })
        .collect::<Vec<_>>();
    let grouped = agent_grouping_is_effective(&entries, config);
    let rows = entries
        .iter()
        .enumerate()
        .map(|(index, (agent, workspace))| {
            let tab = snapshot.tabs.iter().find(|tab| tab.tab_id == agent.tab_id);
            let pane = snapshot
                .panes
                .iter()
                .find(|pane| pane.pane_id == agent.pane_id);
            let tab_count = snapshot
                .tabs
                .iter()
                .filter(|candidate| candidate.workspace_id == agent.workspace_id)
                .count();
            let tab_label = tab
                .filter(|tab| tab_count > 1 || tab.custom_label)
                .map(|tab| tab.label.as_str());
            let agent_label = agent
                .display_agent
                .as_deref()
                .or(agent.name.as_deref())
                .or(agent.agent.as_deref())
                .or(agent.title.as_deref());
            let labels = agent
                .state_labels
                .iter()
                .cloned()
                .collect::<HashMap<_, _>>();
            let tokens = agent.tokens.iter().cloned().collect::<HashMap<_, _>>();
            let state_text = labels
                .get(status_text(agent.agent_status))
                .map(String::as_str)
                .unwrap_or_else(|| sidebar_status_text(agent.agent_status));
            let canonical_agent = agent
                .agent
                .as_deref()
                .and_then(crate::detect::parse_agent_label);
            let rows = crate::ui::sidebar_agent_rows(
                &config.agents,
                crate::ui::AgentTokenContext {
                    workspace: &workspace.label,
                    tab: tab_label,
                    pane: agent
                        .title
                        .as_deref()
                        .or_else(|| pane.and_then(|pane| pane.label.as_deref())),
                    agent_label,
                    terminal_title: agent.terminal_title.as_deref(),
                    terminal_title_stripped: agent.terminal_title_stripped.as_deref(),
                    canonical_agent,
                    tokens: &tokens,
                },
                state_text,
                grouped,
            );
            // The run's first entry draws the header for everyone behind it, so
            // no entry of its own is inserted and every position-indexed
            // consumer — the hit-test, the scroll offset, the scrollbar
            // metrics — keeps counting agents.
            let header = (grouped
                && index
                    .checked_sub(1)
                    .is_none_or(|previous| entries[previous].0.workspace_id != agent.workspace_id))
            .then_some(workspace.label.as_str());
            AgentRow {
                pane_id: agent.pane_id.as_str(),
                workspace_id: agent.workspace_id.as_str(),
                header,
                status: agent.agent_status,
                focused: agent.focused,
                rows,
            }
        })
        .collect();
    (rows, grouped)
}

fn render_agent_row(
    buffer: &mut Buffer,
    rect: Rect,
    row: &AgentRow<'_>,
    grouped: bool,
    config: &ClientShellConfig,
) {
    let palette = &config.palette;
    // The clamp in `agent_entry_height_from_rows` can leave room for the header
    // but not the agent rows it labels; the agent row wins that tie.
    let header_rows = if rect.height > row.header_rows() {
        row.header_rows()
    } else {
        0
    };
    let row_style = if row.focused {
        Style::default().bg(palette.active_row_bg)
    } else {
        Style::default()
    };
    let name_style = if row.focused {
        Style::default()
            .fg(palette.text)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(palette.subtext0)
            .add_modifier(Modifier::BOLD)
    };
    let status_style = Style::default()
        .fg(status_color(row.status, palette))
        .add_modifier(if row.focused {
            Modifier::empty()
        } else {
            Modifier::DIM
        });
    let secondary = Style::default()
        .fg(palette.overlay0)
        .add_modifier(Modifier::DIM);
    let icon = (
        status_icon(row.status, config.status_indicators),
        Style::default().fg(status_color(row.status, palette)),
    );
    let rows = if row.rows.is_empty() {
        vec![vec![crate::ui::ResolvedToken {
            kind: crate::ui::ResolvedTokenKind::StateIcon,
            style: Default::default(),
        }]]
    } else {
        row.rows.clone()
    };
    if let (1, Some(label)) = (header_rows, row.header) {
        // The header labels the whole run, so it never carries the active-row
        // highlight — even though the entry drawing it may be the focused pane.
        // Doing so would mark two rows for one focused agent, and only ever for
        // the run's first agent, since a later agent in the run draws no header
        // of its own. The hit-test still routes a click here to that first
        // agent; that is unchanged.
        Paragraph::new(Line::from(vec![
            ratatui::text::Span::raw(" "),
            ratatui::text::Span::styled(
                crate::ui::truncate_end(label, rect.width.saturating_sub(1) as usize),
                Style::default()
                    .fg(palette.subtext0)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .render(Rect::new(rect.x, rect.y, rect.width, 1), buffer);
    }

    let agent_rows = rect.height.saturating_sub(header_rows);
    for (index, tokens) in rows.iter().take(agent_rows as usize).enumerate() {
        // Prefix and width budget key off the visual row, not `index`. While
        // grouping, every agent row sits under a workspace header — including
        // the rows of later entries in the same run, which draw no header of
        // their own — so they all take the indented prefix.
        let visual_row = index as u16 + header_rows;
        let indent = if grouped || visual_row > 0 { 3 } else { 1 };
        let mut spans = vec![ratatui::text::Span::raw(" ".repeat(indent))];
        spans.extend(crate::ui::resolved_token_spans(
            tokens,
            icon,
            status_style,
            name_style,
            secondary,
            secondary,
            palette,
            rect.width.saturating_sub(indent as u16) as usize,
        ));
        Paragraph::new(Line::from(spans)).style(row_style).render(
            Rect::new(rect.x, rect.y + visual_row, rect.width, 1),
            buffer,
        );
    }
}

fn put_text(buffer: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    for (offset, character) in text.chars().take(width as usize).enumerate() {
        if let Some(cell) = buffer.cell_mut((x + offset as u16, y)) {
            cell.set_char(character).set_style(style);
        }
    }
}

fn display_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

fn sidebar_status_text(status: crate::api::schema::AgentStatus) -> &'static str {
    use crate::api::schema::AgentStatus;
    match status {
        AgentStatus::Blocked => "blocked",
        AgentStatus::Done => "done",
        AgentStatus::Working => "working",
        AgentStatus::Idle | AgentStatus::Unknown => "idle",
    }
}
