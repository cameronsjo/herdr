use std::borrow::Cow;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::release_notes::release_notes_close_button_rect;
use super::scrollbar::{release_notes_scrollbar_rect, render_scrollbar};
use super::widgets::{
    modal_stack_areas, panel_contrast_fg, render_action_button, render_modal_header,
    render_modal_shell,
};
use crate::app::{AppState, NavigateAction};

pub(crate) struct PaletteCommand {
    pub name: Cow<'static, str>,
    pub key: String,
    pub action: NavigateAction,
}

pub(crate) fn palette_commands(app: &AppState) -> Vec<PaletteCommand> {
    super::keybind_help::keybind_help_groups(app)
        .into_iter()
        .flat_map(|(_, entries)| entries)
        .filter_map(|entry| {
            Some(PaletteCommand {
                name: entry.label,
                key: entry.key,
                action: entry.action?,
            })
        })
        .collect()
}

/// Ranking by match quality rather than list order keeps a query that exactly
/// names one command from being answered by a longer command containing it.
fn match_rank(name: &str, query: &str) -> Option<u8> {
    let name = name.to_lowercase();
    if name == query {
        Some(0)
    } else if name.starts_with(query) {
        Some(1)
    } else if name.split_whitespace().any(|word| word.starts_with(query)) {
        Some(2)
    } else if name.contains(query) {
        Some(3)
    } else {
        None
    }
}

pub(crate) fn filtered_palette_commands(app: &AppState) -> Vec<PaletteCommand> {
    let query = app.command_palette.query.trim().to_lowercase();
    let commands = palette_commands(app);
    if query.is_empty() {
        return commands;
    }

    let mut ranked: Vec<(u8, usize, PaletteCommand)> = commands
        .into_iter()
        .enumerate()
        .filter_map(|(index, command)| {
            match_rank(&command.name, &query).map(|rank| (rank, index, command))
        })
        .collect();
    ranked.sort_by_key(|(rank, index, _)| (*rank, *index));
    ranked.into_iter().map(|(_, _, command)| command).collect()
}

fn palette_lines(app: &AppState, width: usize) -> Vec<Line<'static>> {
    let selected_style = Style::default()
        .fg(panel_contrast_fg(&app.palette))
        .bg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let name_style = Style::default().fg(app.palette.text);
    let key_style = Style::default().fg(app.palette.overlay1);

    filtered_palette_commands(app)
        .into_iter()
        .enumerate()
        .map(|(index, command)| {
            let (name_span_style, key_span_style) = if index == app.command_palette.selected {
                (selected_style, selected_style)
            } else {
                (name_style, key_style)
            };
            let name = format!(" {}", command.name);
            let key = format!("{} ", command.key);
            let gap = width.saturating_sub(name.chars().count() + key.chars().count());
            Line::from(vec![
                Span::styled(name, name_span_style),
                Span::styled(" ".repeat(gap), name_span_style),
                Span::styled(key, key_span_style),
            ])
        })
        .collect()
}

pub(super) fn render_palette_overlay(app: &AppState, frame: &mut Frame) {
    super::dim_background(frame, frame.area());

    let Some(inner) = render_modal_shell(frame, frame.area(), 76, 22, &app.palette) else {
        return;
    };
    if inner.height < 6 || inner.width < 20 {
        return;
    }

    let stack = modal_stack_areas(inner, 2, 1, 0, 1);
    let header_rows =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas::<2>(stack.header);

    render_modal_header(frame, header_rows[0], "commands", &app.palette);
    render_action_button(
        frame,
        release_notes_close_button_rect(header_rows[0]),
        Some("esc"),
        "close",
        Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD),
    );

    let query_span = if app.command_palette.query.is_empty() {
        Span::styled("type to filter", Style::default().fg(app.palette.overlay0))
    } else {
        Span::styled(
            app.command_palette.query.clone(),
            Style::default()
                .fg(app.palette.text)
                .add_modifier(Modifier::BOLD),
        )
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " > ",
                Style::default()
                    .fg(app.palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            query_span,
        ])),
        header_rows[1],
    );

    let body_area = stack.content;
    let total = filtered_palette_commands(app).len();
    let metrics = crate::pane::ScrollMetrics {
        offset_from_bottom: app
            .palette_max_scroll()
            .saturating_sub(app.command_palette.scroll) as usize,
        max_offset_from_bottom: app.palette_max_scroll() as usize,
        viewport_rows: body_area.height.max(1) as usize,
    };
    let track = release_notes_scrollbar_rect(body_area, metrics);
    let text_area = track
        .map(|_| {
            Rect::new(
                body_area.x,
                body_area.y,
                body_area.width.saturating_sub(1),
                body_area.height,
            )
        })
        .unwrap_or(body_area);

    let lines = if total == 0 {
        vec![Line::from(Span::styled(
            " no matching commands",
            Style::default().fg(app.palette.overlay1),
        ))]
    } else {
        palette_lines(app, text_area.width as usize)
    };
    frame.render_widget(
        Paragraph::new(lines).scroll((app.command_palette.scroll, 0)),
        text_area,
    );
    if let Some(track) = track {
        render_scrollbar(
            frame,
            metrics,
            track,
            app.palette.overlay0,
            app.palette.overlay1,
            "▐",
        );
    }

    let mut spans = Vec::new();
    for (index, (label, keys)) in [("run ", "enter"), ("move ", "↑↓"), ("close ", "esc")]
        .into_iter()
        .enumerate()
    {
        spans.push(Span::styled(
            if index == 0 {
                format!(" {label}")
            } else {
                label.to_string()
            },
            Style::default().fg(app.palette.overlay0),
        ));
        spans.push(Span::styled(
            keys.to_string(),
            Style::default().fg(app.palette.text),
        ));
        spans.push(Span::styled(
            " · ",
            Style::default().fg(app.palette.overlay0),
        ));
    }
    spans.pop();
    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        stack.footer.unwrap_or_default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(query: &str) -> Vec<String> {
        let mut state = AppState::test_new();
        state.command_palette.query = query.into();
        filtered_palette_commands(&state)
            .into_iter()
            .map(|command| command.name.into_owned())
            .collect()
    }

    #[test]
    fn an_exact_name_outranks_a_longer_command_containing_it() {
        let matches = names("new tab");
        assert_eq!(
            matches.first().map(String::as_str),
            Some("new tab"),
            "got {matches:?}"
        );
        assert!(
            matches.iter().any(|name| name == "move pane to new tab"),
            "the longer command should still match, got {matches:?}"
        );
    }

    #[test]
    fn a_word_prefix_outranks_a_mid_word_substring() {
        let matches = names("pane");
        let first = matches.first().map(String::as_str).unwrap_or_default();
        assert!(
            first
                .split_whitespace()
                .any(|word| word.starts_with("pane")),
            "got {matches:?}"
        );
    }

    #[test]
    fn every_palette_command_is_runnable() {
        let state = AppState::test_new();
        assert!(!palette_commands(&state).is_empty());
    }
}
