mod tokens;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
};

pub(crate) use self::tokens::{
    agent_rows as sidebar_agent_rows, space_rows as sidebar_space_rows, AgentTokenContext,
    ResolvedToken, ResolvedTokenKind, SpaceTokenContext,
};
use super::text::{display_width, truncate_end};
use crate::app::state::Palette;
use crate::app::AppState;
use crate::config::SidebarTokenAlignment;
use crate::detect::AgentState;
use crate::terminal::TerminalRuntimeRegistry;

pub(crate) struct AgentPanelEntry {
    pub ws_idx: usize,
    pub tab_idx: usize,
    pub pane_id: crate::layout::PaneId,
    pub agent_kind_label: Option<String>,
    pub state: AgentState,
    pub seen: bool,
    pub last_agent_state_change_seq: Option<u64>,
    pub tokens: std::collections::HashMap<String, String>,
}

fn sidebar_section_heights(total_height: u16, split_ratio: f32) -> (u16, u16) {
    if total_height == 0 {
        return (0, 0);
    }
    if total_height < 6 {
        let workspace_height = total_height.div_ceil(2);
        return (
            workspace_height,
            total_height.saturating_sub(workspace_height),
        );
    }

    let workspace_height = ((total_height as f32) * split_ratio.clamp(0.1, 0.9)).round() as u16;
    let workspace_height = workspace_height.clamp(3, total_height.saturating_sub(3));
    (
        workspace_height,
        total_height.saturating_sub(workspace_height),
    )
}

pub(crate) fn expanded_sidebar_sections(area: Rect, split_ratio: f32) -> (Rect, Rect) {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.is_empty() {
        return (Rect::default(), Rect::default());
    }

    let (workspace_height, detail_height) = sidebar_section_heights(content.height, split_ratio);
    (
        Rect::new(content.x, content.y, content.width, workspace_height),
        Rect::new(
            content.x,
            content.y + workspace_height,
            content.width,
            detail_height,
        ),
    )
}

pub(crate) fn sidebar_section_divider_rect(area: Rect, split_ratio: f32) -> Rect {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.width == 0 || content.height < 6 {
        return Rect::default();
    }

    let (workspace_height, _) = sidebar_section_heights(content.height, split_ratio);
    Rect::new(content.x, content.y + workspace_height, content.width, 1)
}

pub(crate) fn agent_panel_entries_from(
    app: &AppState,
    _terminal_runtimes: &TerminalRuntimeRegistry,
) -> Vec<AgentPanelEntry> {
    let mut entries = app
        .workspaces
        .iter()
        .enumerate()
        .flat_map(|(ws_idx, workspace)| {
            workspace
                .pane_details(&app.terminals)
                .into_iter()
                .map(move |detail| AgentPanelEntry {
                    ws_idx,
                    tab_idx: detail.tab_idx,
                    pane_id: detail.pane_id,
                    agent_kind_label: detail.agent_kind_label,
                    state: detail.state,
                    seen: detail.seen,
                    last_agent_state_change_seq: detail.last_agent_state_change_seq,
                    tokens: detail.tokens,
                })
        })
        .collect();
    crate::app::agent_view::apply_agent_view(app, &mut entries);
    entries
}

pub(crate) fn resolved_token_spans(
    resolved: &[ResolvedToken],
    state_icon: (&str, Style),
    state_text_style: Style,
    workspace_style: Style,
    secondary_style: Style,
    custom_style: Style,
    palette: &Palette,
    max_width: usize,
) -> Vec<Span<'static>> {
    let fixed_widths = resolved
        .iter()
        .map(|token| match &token.kind {
            ResolvedTokenKind::StateIcon => display_width(state_icon.0),
            ResolvedTokenKind::GitStatus { ahead, behind } => {
                usize::from(*ahead > 0) * display_width(&format!("↑{ahead}"))
                    + usize::from(*behind > 0) * display_width(&format!("↓{behind}"))
                    + usize::from(*ahead > 0 && *behind > 0)
            }
            _ => 0,
        })
        .collect::<Vec<_>>();
    let flexible_widths = resolved
        .iter()
        .map(|token| match &token.kind {
            ResolvedTokenKind::StateText(text)
            | ResolvedTokenKind::Workspace(text)
            | ResolvedTokenKind::Tab(text)
            | ResolvedTokenKind::Pane(text)
            | ResolvedTokenKind::Agent(text)
            | ResolvedTokenKind::TerminalTitle(text)
            | ResolvedTokenKind::Branch(text)
            | ResolvedTokenKind::Custom(text) => display_width(text),
            _ => 0,
        })
        .collect::<Vec<_>>();
    let minimum_width = |active: &[bool]| {
        let indices = active
            .iter()
            .enumerate()
            .filter_map(|(index, active)| active.then_some(index))
            .collect::<Vec<_>>();
        let content = indices
            .iter()
            .map(|index| fixed_widths[*index] + usize::from(flexible_widths[*index] > 0))
            .sum::<usize>();
        let separators = indices
            .windows(2)
            .map(|pair| display_width(tokens::separator(&resolved[pair[0]], &resolved[pair[1]])))
            .sum::<usize>();
        content + separators
    };
    let mut active = resolved.iter().map(|_| true).collect::<Vec<_>>();
    if minimum_width(&active) > max_width {
        for (index, width) in flexible_widths.iter().enumerate() {
            if *width > 0 {
                active[index] = false;
            }
        }
        for index in (0..resolved.len()).rev() {
            if flexible_widths[index] == 0 {
                continue;
            }
            active[index] = true;
            if minimum_width(&active) > max_width {
                active[index] = false;
            }
        }
    }
    let visible_indices = active
        .iter()
        .enumerate()
        .filter_map(|(index, active)| active.then_some(index))
        .collect::<Vec<_>>();
    let separator_width = visible_indices
        .windows(2)
        .map(|pair| display_width(tokens::separator(&resolved[pair[0]], &resolved[pair[1]])))
        .sum::<usize>();
    let fixed_width = visible_indices
        .iter()
        .map(|index| fixed_widths[*index])
        .sum::<usize>();
    let mut budgets = flexible_widths
        .iter()
        .enumerate()
        .map(|(index, width)| usize::from(active[index] && *width > 0))
        .collect::<Vec<_>>();
    let minimum = budgets.iter().sum::<usize>();
    let mut remaining = max_width
        .saturating_sub(separator_width + fixed_width)
        .saturating_sub(minimum);
    // A token marked `align = "right"` opens a trailing group: it and every
    // visible token after it are funded to their full width first, so the group
    // can be pushed against the right edge below without truncating mid-group.
    let right_start = visible_indices
        .iter()
        .position(|index| resolved[*index].style.align == Some(SidebarTokenAlignment::Right));
    if let Some(right_start) = right_start {
        for index in &visible_indices[right_start..] {
            let budget = &mut budgets[*index];
            let width = flexible_widths[*index];
            let growth = width.saturating_sub(*budget).min(remaining);
            *budget += growth;
            remaining -= growth;
        }
    }
    while remaining > 0 {
        let mut grew = false;
        for (budget, width) in budgets.iter_mut().zip(&flexible_widths) {
            if *budget > 0 && *budget < *width {
                *budget += 1;
                remaining -= 1;
                grew = true;
                if remaining == 0 {
                    break;
                }
            }
        }
        if !grew {
            break;
        }
    }

    let right_width = right_start.map(|right_start| {
        let indices = &visible_indices[right_start..];
        let content = indices
            .iter()
            .map(|index| fixed_widths[*index] + budgets[*index])
            .sum::<usize>();
        let separators = indices
            .windows(2)
            .map(|pair| display_width(tokens::separator(&resolved[pair[0]], &resolved[pair[1]])))
            .sum::<usize>();
        content + separators
    });
    let mut rendered_width = 0;
    let mut spans = Vec::new();
    for (position, index) in visible_indices.iter().copied().enumerate() {
        let token = &resolved[index];
        if right_start == Some(position) {
            let separator = (position > 0)
                .then(|| tokens::separator(&resolved[visible_indices[position - 1]], token));
            let separator_width = separator.map_or(0, display_width);
            let padding = max_width
                .saturating_sub(rendered_width + separator_width + right_width.unwrap_or(0));
            if padding > 0 {
                spans.push(Span::raw(" ".repeat(padding)));
                rendered_width += padding;
            }
        }
        if position > 0 {
            let previous = &resolved[visible_indices[position - 1]];
            let separator = tokens::separator(previous, token);
            rendered_width += display_width(separator);
            spans.push(Span::styled(
                separator,
                Style::default()
                    .fg(palette.overlay0)
                    .add_modifier(Modifier::DIM),
            ));
        }
        rendered_width += fixed_widths[index] + budgets[index];
        match &token.kind {
            ResolvedTokenKind::StateIcon => spans.push(Span::styled(
                state_icon.0.to_string(),
                apply_token_style(state_icon.1, token.style),
            )),
            ResolvedTokenKind::StateText(text) => spans.push(Span::styled(
                truncate_end(text, budgets[index]),
                apply_token_style(state_text_style, token.style),
            )),
            ResolvedTokenKind::Workspace(text) => spans.push(Span::styled(
                truncate_end(text, budgets[index]),
                apply_token_style(workspace_style, token.style),
            )),
            ResolvedTokenKind::Tab(text)
            | ResolvedTokenKind::Pane(text)
            | ResolvedTokenKind::Agent(text)
            | ResolvedTokenKind::Branch(text) => spans.push(Span::styled(
                truncate_end(text, budgets[index]),
                apply_token_style(secondary_style, token.style),
            )),
            ResolvedTokenKind::GitStatus { ahead, behind } => {
                if *ahead > 0 {
                    spans.push(Span::styled(
                        format!("↑{ahead}"),
                        apply_token_style(Style::default().fg(palette.green), token.style),
                    ));
                }
                if *ahead > 0 && *behind > 0 {
                    spans.push(Span::styled(
                        " ",
                        apply_token_style(Style::default(), token.style),
                    ));
                }
                if *behind > 0 {
                    spans.push(Span::styled(
                        format!("↓{behind}"),
                        apply_token_style(Style::default().fg(palette.red), token.style),
                    ));
                }
            }
            ResolvedTokenKind::TerminalTitle(text) | ResolvedTokenKind::Custom(text) => {
                spans.push(Span::styled(
                    truncate_end(text, budgets[index]),
                    apply_token_style(custom_style, token.style),
                ));
            }
        }
    }
    spans
}

fn apply_token_style(mut style: Style, patch: crate::config::SidebarTokenStyle) -> Style {
    if let Some(foreground) = patch.fg {
        style = style.fg(foreground.ratatui());
    }
    if let Some(bold) = patch.bold {
        style = if bold {
            style.add_modifier(Modifier::BOLD)
        } else {
            style.remove_modifier(Modifier::BOLD)
        };
    }
    if let Some(dim) = patch.dim {
        style = if dim {
            style.add_modifier(Modifier::DIM)
        } else {
            style.remove_modifier(Modifier::DIM)
        };
    }
    style
}

#[cfg(test)]
mod sidebar_alignment_tests {
    use super::*;
    use crate::config::SidebarTokenStyle;

    fn render(resolved: &[ResolvedToken], max_width: usize) -> String {
        let palette = Palette::catppuccin();
        resolved_token_spans(
            resolved,
            ("○", Style::default()),
            Style::default(),
            Style::default(),
            Style::default(),
            Style::default(),
            &palette,
            max_width,
        )
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>()
    }

    fn right_aligned(kind: ResolvedTokenKind) -> ResolvedToken {
        ResolvedToken {
            kind,
            style: SidebarTokenStyle {
                align: Some(SidebarTokenAlignment::Right),
                ..Default::default()
            },
        }
    }

    #[test]
    fn right_aligned_token_stays_at_the_row_edge() {
        // The row fits with room to spare, so only the padding can push the
        // state text to the edge — a truncating row would end with it anyway.
        let rendered = render(
            &[
                ResolvedToken::unstyled(ResolvedTokenKind::TerminalTitle("short".into())),
                right_aligned(ResolvedTokenKind::StateText("idle".into())),
            ],
            20,
        );

        assert!(rendered.starts_with("short"), "rendered: {rendered:?}");
        assert!(rendered.ends_with("idle"), "rendered: {rendered:?}");
        assert_eq!(display_width(&rendered), 20);
    }

    #[test]
    fn the_right_group_runs_to_the_end_of_the_row() {
        // `align` opens a trailing group: the state text and everything after
        // it travel together against the right edge, separators included.
        let rendered = render(
            &[
                ResolvedToken::unstyled(ResolvedTokenKind::Agent("claude".into())),
                right_aligned(ResolvedTokenKind::StateText("idle".into())),
                ResolvedToken::unstyled(ResolvedTokenKind::Tab("2".into())),
            ],
            24,
        );

        assert!(rendered.starts_with("claude"), "rendered: {rendered:?}");
        assert!(rendered.ends_with("idle · 2"), "rendered: {rendered:?}");
        assert_eq!(display_width(&rendered), 24);
    }

    #[test]
    fn an_unaligned_row_is_left_packed_with_no_padding() {
        let rendered = render(
            &[
                ResolvedToken::unstyled(ResolvedTokenKind::Agent("claude".into())),
                ResolvedToken::unstyled(ResolvedTokenKind::StateText("idle".into())),
            ],
            24,
        );

        assert_eq!(rendered, "claude · idle");
    }
}
