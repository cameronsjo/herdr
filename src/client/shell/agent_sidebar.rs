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

pub(super) struct AgentRow {
    pub(super) pane_id: String,
    pub(super) status: crate::api::schema::AgentStatus,
    pub(super) focused: bool,
    pub(super) rows: Vec<Vec<crate::ui::ResolvedToken>>,
    /// Machine scope plus workspace id. Two endpoints can advertise the same
    /// workspace id, so the scope is what keeps their runs from merging into
    /// one header and losing the gap between them. The agent sidebar shows one
    /// machine at a time and leaves the scope `None`.
    ///
    /// The scope is the endpoint's index, not its label: labels are display
    /// names and two connections can share one, which would put two machines
    /// back in the same run. An index is also `Copy`, so a per-frame row costs
    /// no allocation for it.
    group_key: (Option<usize>, AgentGroupKey<String>),
    header: Option<String>,
    grouped: bool,
}

/// The run an agent belongs to under `group_by`.
///
/// A token value and a workspace id live in separate variants, so a project
/// named like a workspace id never merges with that workspace's run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum AgentGroupKey<S> {
    Workspace(S),
    Token(S),
}

impl AgentGroupKey<&str> {
    fn to_owned_key(self) -> AgentGroupKey<String> {
        match self {
            Self::Workspace(id) => AgentGroupKey::Workspace(id.to_owned()),
            Self::Token(value) => AgentGroupKey::Token(value.to_owned()),
        }
    }
}

/// The group key for one agent. Token mode keys by the pane's token value,
/// then by the same token on the agent's workspace, and falls back to the
/// workspace itself, so an agent with neither keeps the grouping it had under
/// `group_by = "workspace"`.
///
/// The workspace token is the durable one: a plugin that stops refreshing a
/// pane token lets it expire, while a workspace token set once stays. The
/// workspace is looked up only when the pane has no value, so a tokened pane
/// costs no scan.
pub(super) fn agent_group_key<'a>(
    agent: &'a ClientShellAgent,
    workspaces: &'a [ClientShellWorkspace],
    group_by: &crate::config::AgentGroupBy,
) -> AgentGroupKey<&'a str> {
    let crate::config::AgentGroupBy::Token(name) = group_by else {
        return AgentGroupKey::Workspace(agent.workspace_id.as_str());
    };
    let token_value = |tokens: &'a [(String, String)]| {
        tokens
            .iter()
            .find(|(key, value)| key == name && !value.is_empty())
            .map(|(_, value)| value.as_str())
    };
    token_value(&agent.tokens)
        .or_else(|| {
            workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == agent.workspace_id)
                .and_then(|workspace| token_value(&workspace.tokens))
        })
        .map_or(
            AgentGroupKey::Workspace(agent.workspace_id.as_str()),
            AgentGroupKey::Token,
        )
}

/// The header text for a run: the token value, or the workspace label.
pub(super) fn group_header<'a>(
    key: AgentGroupKey<&'a str>,
    workspace: &'a ClientShellWorkspace,
) -> &'a str {
    match key {
        AgentGroupKey::Token(value) => value,
        AgentGroupKey::Workspace(_) => workspace.label.as_str(),
    }
}

/// Moves every item up behind the first item with the same key, keeping the
/// relative order inside each key and the order of first appearances.
///
/// Token groups span workspaces, so their members are not adjacent in space
/// order; this makes each key one run. It runs in the ordering functions, not
/// the renderer, so the hit-test, scrolling, and agent navigation all see the
/// order that gets drawn.
pub(super) fn gather_runs<T, K: Eq + std::hash::Hash>(
    items: Vec<T>,
    key: impl Fn(&T) -> K,
) -> Vec<T> {
    let mut first_seen = HashMap::<K, usize>::new();
    let mut indexed = items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let run = *first_seen.entry(key(&item)).or_insert(index);
            (run, index, item)
        })
        .collect::<Vec<_>>();
    indexed.sort_by_key(|(run, index, _)| (*run, *index));
    indexed.into_iter().map(|(_, _, item)| item).collect()
}

impl AgentRow {
    pub(super) fn row_lines(&self) -> usize {
        self.rows
            .len()
            .max(1)
            .saturating_add(usize::from(self.header_rows()))
    }

    pub(super) fn gap_after(&self, next: &Self, gap: u16) -> u16 {
        if self.grouped && next.grouped && self.group_key == next.group_key {
            0
        } else {
            gap
        }
    }

    fn header_rows(&self) -> u16 {
        u16::from(self.header.is_some())
    }
}

/// Whether the agent panel draws one header per contiguous run.
///
/// One gate drives both the headers and the row layout. `agent_panel_sort` is a
/// one-click toggle, so gating the header on the live sort while gating the
/// layout on config would leave a `priority` user with no group shown at all.
///
/// A header must never label agents from another group, so the order about to
/// be rendered has to keep each group in one run. The endpoint's snapshot
/// carries an active agent view's resulting order but not the sort clause that
/// produced it, so the order itself is what gets checked: a filter-only view
/// keeps space order and stays grouped, while a view that interleaves
/// workspaces turns workspace grouping off. Token grouping gathers its runs in
/// `ordered_agent_pane_ids`, so the order is contiguous by construction and the
/// scan is skipped: with many one-agent groups it is quadratic per frame.
fn agent_grouping_is_effective(
    entries: &[(&ClientShellAgent, &ClientShellWorkspace)],
    workspaces: &[ClientShellWorkspace],
    config: &ClientShellConfig,
) -> bool {
    config.agents.group_by.is_grouped()
        && config.agent_panel_sort == crate::config::AgentPanelSortConfig::Spaces
        && (gathers_group_runs(&config.agents.group_by, config.agent_panel_sort)
            || runs_are_contiguous(entries, |(agent, _)| {
                agent_group_key(agent, workspaces, &config.agents.group_by)
            }))
}

/// Whether `ordered_agent_pane_ids` gathers each group into one run.
///
/// Only token grouping gathers. Workspace grouping keeps the order it had
/// before token grouping existed, and turns off when that order interleaves.
pub(super) fn gathers_group_runs(
    group_by: &crate::config::AgentGroupBy,
    sort: crate::config::AgentPanelSortConfig,
) -> bool {
    matches!(group_by, crate::config::AgentGroupBy::Token(_))
        && sort == crate::config::AgentPanelSortConfig::Spaces
}

/// Whether every key occupies exactly one run of `items`, under `key`.
///
/// The inner scan runs only at a run boundary, so the cost is bounded by the
/// number of runs — at most the distinct key count — rather than the item count
/// squared, and it allocates nothing inside this per-frame path.
///
/// Takes an accessor rather than a slice of keys: the endpoint list groups the
/// same way but keys each run by machine as well as workspace, and materializing
/// either caller's keys into a `Vec` would put an allocation on every frame.
pub(super) fn runs_are_contiguous<T, K: PartialEq>(items: &[T], key: impl Fn(&T) -> K) -> bool {
    items.iter().enumerate().all(|(index, item)| {
        let Some(previous) = index.checked_sub(1) else {
            return true;
        };
        let current = key(item);
        key(&items[previous]) == current
            || !items[..previous]
                .iter()
                .any(|earlier| key(earlier) == current)
    })
}

/// Moves blocked items to the front of each run of equal `key`, keeping every
/// other order. A run is a stretch of adjacent items with one key, so this
/// never splits or merges runs, and a grouped panel stays grouped.
pub(super) fn blocked_first_within_runs<T, K: PartialEq>(
    items: Vec<T>,
    key: impl Fn(&T) -> K,
    blocked: impl Fn(&T) -> bool,
) -> Vec<T> {
    let mut run = 0usize;
    let mut previous: Option<K> = None;
    let mut indexed = items
        .into_iter()
        .map(|item| {
            let current = key(&item);
            if previous
                .as_ref()
                .is_some_and(|previous| *previous != current)
            {
                run += 1;
            }
            previous = Some(current);
            (run, !blocked(&item), item)
        })
        .collect::<Vec<_>>();
    // `sort_by_key` is stable, so the order inside each part is unchanged.
    indexed.sort_by_key(|(run, not_blocked, _)| (*run, *not_blocked));
    indexed.into_iter().map(|(_, _, item)| item).collect()
}

/// Applies `blocked_first` to an order already gathered for display: blocked
/// agents lead each group while the panel draws groups, and the whole list
/// when it does not. Workspace grouping turns off when the order interleaves
/// workspaces, so its runs are grouped only when they are contiguous; sorting
/// within interleaved fragments would leave a blocked agent where it was.
///
/// `group_key` names an item's group; `whole_key` names the list it belongs to
/// when groups are off (one list locally, one per machine in the endpoint list).
pub(super) fn blocked_first_order<T, K: PartialEq>(
    items: Vec<T>,
    group_by: &crate::config::AgentGroupBy,
    sort: crate::config::AgentPanelSortConfig,
    group_key: impl Fn(&T) -> K,
    whole_key: impl Fn(&T) -> K,
    blocked: impl Fn(&T) -> bool,
) -> Vec<T> {
    let grouped = group_by.is_grouped()
        && (gathers_group_runs(group_by, sort) || runs_are_contiguous(&items, &group_key));
    if grouped {
        blocked_first_within_runs(items, group_key, blocked)
    } else {
        blocked_first_within_runs(items, whole_key, blocked)
    }
}

pub(super) fn ordered_agent_pane_ids(
    snapshot: &ClientShellSnapshot,
    sort: crate::config::AgentPanelSortConfig,
    agents_config: &crate::config::AgentsSidebarConfig,
) -> Vec<String> {
    let group_by = &agents_config.group_by;
    let mut agents = if snapshot.agent_view_label.is_some() {
        // An active view's order is the view's; only grouping and
        // blocked_first below adjust it.
        snapshot
            .agent_order
            .iter()
            .filter_map(|pane_id| {
                snapshot
                    .agents
                    .iter()
                    .find(|agent| agent.pane_id == pane_id.as_str())
            })
            .collect::<Vec<_>>()
    } else {
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
    };
    if gathers_group_runs(group_by, sort) {
        agents = gather_runs(agents, |agent| {
            agent_group_key(agent, &snapshot.workspaces, group_by)
        });
    }
    if agents_config.blocked_first && sort == crate::config::AgentPanelSortConfig::Spaces {
        agents = blocked_first_order(
            agents,
            group_by,
            sort,
            |agent| Some(agent_group_key(agent, &snapshot.workspaces, group_by)),
            |_| None,
            |agent| agent.agent_status == crate::api::schema::AgentStatus::Blocked,
        );
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
    if !render_agent_panel_header(
        buffer,
        area,
        snapshot.agent_view_label.as_deref(),
        config,
        hits,
    ) {
        return;
    }

    let rows = agent_rows(snapshot, config, None);
    render_agent_list(
        buffer,
        area,
        &rows,
        snapshot
            .agent_view_label
            .as_ref()
            .map(|_| " no matching agents"),
        config,
        agent_scroll,
        hits,
        AgentRow::row_lines,
        |row, next| row.gap_after(next, config.agents.row_gap),
        |buffer, rect, row, hits| {
            hits.agents.push((rect, row.pane_id.clone()));
            render_agent_row(buffer, rect, row, config);
        },
    );
}

pub(super) fn render_agent_panel_header(
    buffer: &mut Buffer,
    area: Rect,
    agent_view_label: Option<&str>,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) -> bool {
    if area.height == 0 {
        return false;
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
        return false;
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
    // An active view shows its label with a ✕ that clears it. The label is
    // truncated first so the ✕ stays visible next to " agents"; this runs once
    // per frame, not per pane.
    let sort_label: std::borrow::Cow<'_, str> = match agent_view_label {
        Some(label) => {
            let room = (area.width as usize).saturating_sub(display_width(" agents ✕ ") + 1);
            format!("{} ✕", crate::ui::truncate_end(label, room)).into()
        }
        None => match config.agent_panel_sort {
            crate::config::AgentPanelSortConfig::Spaces => "grouped",
            crate::config::AgentPanelSortConfig::Priority => "priority",
        }
        .into(),
    };
    let sort_width = display_width(&sort_label).min(area.width as usize) as u16;
    let sort_rect = Rect::new(
        area.right().saturating_sub(sort_width),
        area.y + 1,
        sort_width,
        1,
    );
    let (sort_toggle, view_clear) = match (config.mouse_capture, agent_view_label) {
        (false, _) => (Rect::default(), Rect::default()),
        (true, None) => (sort_rect, Rect::default()),
        (true, Some(_)) => (Rect::default(), sort_rect),
    };
    hits.agent_sort_toggle = sort_toggle;
    hits.agent_view_clear = view_clear;
    put_text(
        buffer,
        sort_rect.x,
        sort_rect.y,
        sort_rect.width,
        &sort_label,
        Style::default()
            .fg(if agent_view_label.is_some() {
                config.palette.accent
            } else {
                config.palette.overlay0
            })
            .add_modifier(Modifier::BOLD),
    );
    true
}

pub(super) fn render_agent_list<T>(
    buffer: &mut Buffer,
    area: Rect,
    rows: &[T],
    empty_message: Option<&str>,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    hits: &mut ShellHitMap,
    row_lines: impl Fn(&T) -> usize,
    gap_after: impl Fn(&T, &T) -> u16,
    mut render_row: impl FnMut(&mut Buffer, Rect, &T, &mut ShellHitMap),
) {
    let body = Rect::new(
        area.x,
        area.y.saturating_add(3),
        area.width,
        area.height.saturating_sub(3),
    );
    hits.agent_body = body;
    if body.is_empty() || rows.is_empty() {
        *agent_scroll = 0;
        if let Some(message) = empty_message.filter(|_| !body.is_empty()) {
            put_text(
                buffer,
                body.x,
                body.y,
                body.width,
                message,
                Style::default()
                    .fg(config.palette.overlay0)
                    .add_modifier(Modifier::DIM),
            );
        }
        return;
    }

    let row_heights = rows
        .iter()
        .map(|row| row_lines(row).max(1).min(usize::from(body.height)) as u16)
        .collect::<Vec<_>>();
    let gaps = rows
        .iter()
        .enumerate()
        .map(|(index, row)| match rows.get(index + 1) {
            None => 0,
            // Grouping packs a workspace run under its shared header, so the
            // gap separates runs rather than individual agents.
            Some(next) => gap_after(row, next),
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
        let height = row_heights[index].min(body.height);
        if y.saturating_add(height) > body.bottom() {
            break;
        }
        // The header belongs to the entry that draws it, so a click anywhere in
        // this rect — header row included — focuses the run's first agent.
        let rect = Rect::new(body.x, y, content_width, height);
        render_row(buffer, rect, row, hits);
        y = y.saturating_add(height).saturating_add(gaps[index]);
    }

    if show_scrollbar {
        let track = Rect::new(body.right().saturating_sub(1), body.y, 1, body.height);
        hits.agent_scrollbar = track;
        super::scroll::render_list_scrollbar(buffer, track, metrics, &config.palette);
    }
}

pub(super) fn agent_rows(
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    machine: Option<&str>,
) -> Vec<AgentRow> {
    let entries = ordered_agent_pane_ids(snapshot, config.agent_panel_sort, &config.agents)
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
    let grouped = agent_grouping_is_effective(&entries, &snapshot.workspaces, config);
    entries
        .iter()
        .enumerate()
        .filter_map(|(index, (agent, workspace))| {
            // The run's first entry draws the header for everyone behind it, so
            // no entry of its own is inserted and every position-indexed
            // consumer — the hit-test, the scroll offset, the scrollbar
            // metrics — keeps counting agents.
            let key = agent_group_key(agent, &snapshot.workspaces, &config.agents.group_by);
            let header = (grouped
                && index.checked_sub(1).is_none_or(|previous| {
                    agent_group_key(
                        entries[previous].0,
                        &snapshot.workspaces,
                        &config.agents.group_by,
                    ) != key
                }))
            .then(|| group_header(key, workspace));
            agent_row(
                snapshot,
                &agent.pane_id,
                config,
                machine,
                AgentRowGrouping {
                    grouped,
                    header,
                    scope: None,
                },
            )
        })
        .collect()
}

/// How one agent row joins the run above it.
///
/// `scope` names the machine the run belongs to; see `AgentRow::group_key`.
pub(super) struct AgentRowGrouping<'a> {
    pub(super) grouped: bool,
    pub(super) header: Option<&'a str>,
    pub(super) scope: Option<usize>,
}

pub(super) fn agent_row(
    snapshot: &ClientShellSnapshot,
    pane_id: &str,
    config: &ClientShellConfig,
    machine: Option<&str>,
    grouping: AgentRowGrouping<'_>,
) -> Option<AgentRow> {
    let AgentRowGrouping {
        grouped,
        header,
        scope,
    } = grouping;
    let agent = snapshot
        .agents
        .iter()
        .find(|agent| agent.pane_id == pane_id)?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|workspace| workspace.workspace_id == agent.workspace_id)?;
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
            machine,
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
    Some(AgentRow {
        pane_id: agent.pane_id.clone(),
        group_key: (
            scope,
            agent_group_key(agent, &snapshot.workspaces, &config.agents.group_by).to_owned_key(),
        ),
        header: header.map(str::to_owned),
        grouped,
        status: agent.agent_status,
        focused: agent.focused,
        rows,
    })
}

pub(super) fn render_agent_row(
    buffer: &mut Buffer,
    rect: Rect,
    row: &AgentRow,
    config: &ClientShellConfig,
) {
    let palette = &config.palette;
    // The clamp in `render_agent_list` can leave room for the header
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
    let status_style = Style::default().fg(status_color(row.status, palette));
    let secondary = Style::default().fg(palette.overlay0);
    let icon = (
        status_icon(row.status, config.status_indicators),
        Style::default().fg(status_color(row.status, palette)),
    );
    // A row with no resolved tokens still draws its status icon. Building the
    // fallback inside that branch lets the common path borrow the entry's rows
    // instead of copying every token, per agent, per frame.
    let fallback: Vec<Vec<crate::ui::ResolvedToken>>;
    let rows: &[Vec<crate::ui::ResolvedToken>] = if row.rows.is_empty() {
        fallback = vec![vec![crate::ui::ResolvedToken {
            kind: crate::ui::ResolvedTokenKind::StateIcon,
            style: Default::default(),
        }]];
        &fallback
    } else {
        &row.rows
    };
    if let (1, Some(label)) = (header_rows, row.header.as_deref()) {
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
        let indent = if row.grouped || visual_row > 0 { 3 } else { 1 };
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
