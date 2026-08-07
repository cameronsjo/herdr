use std::borrow::Cow;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::release_notes::release_notes_close_button_rect;
use super::scrollbar::{release_notes_scrollbar_rect, render_scrollbar};
use super::widgets::{
    modal_stack_areas, panel_contrast_fg, render_action_button, render_modal_header,
    render_modal_shell,
};
use crate::app::{AppState, NavigateAction};

#[derive(Debug, Clone)]
pub(super) struct HelpEntry {
    pub key: String,
    pub label: Cow<'static, str>,
    pub action: Option<NavigateAction>,
    /// Extra search terms the command palette matches this entry against
    /// (e.g. "split right" for the "split vertical" action) — a surfaced-
    /// elsewhere synonym, not a full alias system.
    pub keywords: &'static [&'static str],
}

pub(super) type HelpGroup = (&'static str, Vec<HelpEntry>);

fn help_entry(key: impl Into<String>, label: &'static str) -> HelpEntry {
    HelpEntry {
        key: key.into(),
        label: Cow::Borrowed(label),
        action: None,
        keywords: &[],
    }
}

fn help_action(key: impl Into<String>, label: &'static str, action: NavigateAction) -> HelpEntry {
    HelpEntry {
        key: key.into(),
        label: Cow::Borrowed(label),
        action: Some(action),
        keywords: &[],
    }
}

fn help_action_kw(
    key: impl Into<String>,
    label: &'static str,
    action: NavigateAction,
    keywords: &'static [&'static str],
) -> HelpEntry {
    HelpEntry {
        key: key.into(),
        label: Cow::Borrowed(label),
        action: Some(action),
        keywords,
    }
}

fn keybind_label(bindings: &crate::config::ActionKeybinds) -> String {
    bindings.label().unwrap_or_else(|| "unset".to_string())
}

fn indexed_label(bindings: &[crate::config::IndexedKeybind]) -> String {
    if bindings.is_empty() {
        return "unset".to_string();
    }

    let mut parts = Vec::new();
    let mut index = 0;
    while index < bindings.len() {
        if let Some(prefix) = indexed_range_prefix(&bindings[index..]) {
            parts.push(format!("{prefix}1..9"));
            index += 9;
        } else {
            parts.push(bindings[index].label.clone());
            index += 1;
        }
    }

    parts.join(" / ")
}

fn indexed_range_prefix(bindings: &[crate::config::IndexedKeybind]) -> Option<&str> {
    let run = bindings.get(..9)?;
    let prefix = run[0].label.strip_suffix('1')?;
    for (offset, binding) in run.iter().enumerate() {
        let digit = char::from(b'1' + offset as u8);
        if binding.label.strip_suffix(digit) != Some(prefix) {
            return None;
        }
    }
    Some(prefix)
}

pub(super) fn keybind_help_groups(app: &AppState) -> Vec<HelpGroup> {
    let kb = &app.keybinds;
    let mut groups = Vec::new();

    groups.push((
        "global",
        vec![
            help_entry(
                crate::config::format_key_combo((app.prefix_code, app.prefix_mods)),
                "prefix mode",
            ),
            help_entry(keybind_label(&kb.help), "keybinds"),
            help_action(
                keybind_label(&kb.settings),
                "settings",
                NavigateAction::Settings,
            ),
            help_action(keybind_label(&kb.detach), "detach", NavigateAction::Detach),
            help_action(
                keybind_label(&kb.reload_config),
                "reload config",
                NavigateAction::ReloadConfig,
            ),
            help_action(
                keybind_label(&kb.open_notification_target),
                "open notification target",
                NavigateAction::OpenNotificationTarget,
            ),
        ],
    ));

    groups.push((
        "navigation",
        vec![
            help_entry("esc", "back"),
            help_entry(
                format!(
                    "{} / {}",
                    keybind_label(&kb.navigate.workspace_up),
                    keybind_label(&kb.navigate.workspace_down)
                ),
                "workspace list",
            ),
            help_entry(
                format!(
                    "{} / {} / {} / {} / left / right",
                    keybind_label(&kb.navigate.pane_left),
                    keybind_label(&kb.navigate.pane_down),
                    keybind_label(&kb.navigate.pane_up),
                    keybind_label(&kb.navigate.pane_right)
                ),
                "move focus",
            ),
            help_entry("tab / shift+tab", "cycle pane"),
            help_entry("enter", "open workspace"),
            help_entry("1..9", "switch workspace"),
        ],
    ));

    let workspace_tab = vec![
        help_action(
            keybind_label(&kb.workspace_picker),
            "workspace navigation",
            NavigateAction::WorkspacePicker,
        ),
        help_action(
            keybind_label(&kb.command_palette),
            "command palette",
            NavigateAction::OpenCommandPalette,
        ),
        help_action(
            keybind_label(&kb.goto),
            "session navigator",
            NavigateAction::OpenNavigator,
        ),
        help_action(
            keybind_label(&kb.new_workspace),
            "new workspace",
            NavigateAction::NewWorkspace,
        ),
        help_action(
            keybind_label(&kb.new_worktree),
            "new worktree",
            NavigateAction::NewWorktree,
        ),
        help_action(
            keybind_label(&kb.open_worktree),
            "open worktree",
            NavigateAction::OpenWorktree,
        ),
        help_action(
            keybind_label(&kb.remove_worktree),
            "delete worktree checkout",
            NavigateAction::RemoveWorktree,
        ),
        help_action(
            keybind_label(&kb.rename_workspace),
            "rename workspace",
            NavigateAction::RenameWorkspace,
        ),
        help_action(
            keybind_label(&kb.close_workspace),
            "close workspace",
            NavigateAction::CloseWorkspace,
        ),
        help_action(
            keybind_label(&kb.previous_workspace),
            "previous workspace",
            NavigateAction::PreviousWorkspace,
        ),
        help_action(
            keybind_label(&kb.next_workspace),
            "next workspace",
            NavigateAction::NextWorkspace,
        ),
        help_entry(indexed_label(&kb.switch_workspace), "switch workspace 1-9"),
        help_action(
            keybind_label(&kb.previous_agent),
            "previous agent",
            NavigateAction::PreviousAgent,
        ),
        help_action(
            keybind_label(&kb.next_agent),
            "next agent",
            NavigateAction::NextAgent,
        ),
        help_entry(indexed_label(&kb.focus_agent), "focus agent 1-9"),
        help_action(
            keybind_label(&kb.new_tab),
            "new tab",
            NavigateAction::NewTab,
        ),
        help_action(
            keybind_label(&kb.rename_tab),
            "rename tab",
            NavigateAction::RenameTab,
        ),
        help_action(
            keybind_label(&kb.previous_tab),
            "previous tab",
            NavigateAction::PreviousTab,
        ),
        help_action(
            keybind_label(&kb.next_tab),
            "next tab",
            NavigateAction::NextTab,
        ),
        help_entry(indexed_label(&kb.switch_tab), "switch tab 1-9"),
        help_action(
            keybind_label(&kb.close_tab),
            "close tab",
            NavigateAction::CloseTab,
        ),
    ];
    groups.push(("workspaces / tabs", workspace_tab));

    let panes = vec![
        help_action_kw(
            keybind_label(&kb.split_vertical),
            "split vertical",
            NavigateAction::SplitVertical,
            &["new pane", "split right", "split left"],
        ),
        help_action_kw(
            keybind_label(&kb.split_horizontal),
            "split horizontal",
            NavigateAction::SplitHorizontal,
            &["new pane", "split down", "split up"],
        ),
        help_action(
            keybind_label(&kb.close_pane),
            "close pane",
            NavigateAction::ClosePane,
        ),
        help_action(
            keybind_label(&kb.rename_pane),
            "rename pane",
            NavigateAction::RenamePane,
        ),
        help_action("", "move pane to space", NavigateAction::MovePaneToSpace),
        help_action(
            "",
            "move pane to new space",
            NavigateAction::MovePaneToNewSpace,
        ),
        help_action("", "move pane to new tab", NavigateAction::MovePaneToNewTab),
        help_entry(keybind_label(&kb.edit_scrollback), "edit scrollback"),
        help_entry(keybind_label(&kb.copy_mode), "copy mode"),
        help_action(keybind_label(&kb.zoom), "zoom pane", NavigateAction::Zoom),
        help_action(
            keybind_label(&kb.resize_mode),
            "resize mode",
            NavigateAction::EnterResizeMode,
        ),
        help_action(
            keybind_label(&kb.toggle_sidebar),
            "toggle sidebar",
            NavigateAction::ToggleSidebar,
        ),
        help_action(
            keybind_label(&kb.focus_pane_left),
            "focus pane left",
            NavigateAction::FocusPaneLeft,
        ),
        help_action(
            keybind_label(&kb.focus_pane_down),
            "focus pane down",
            NavigateAction::FocusPaneDown,
        ),
        help_action(
            keybind_label(&kb.focus_pane_up),
            "focus pane up",
            NavigateAction::FocusPaneUp,
        ),
        help_action(
            keybind_label(&kb.focus_pane_right),
            "focus pane right",
            NavigateAction::FocusPaneRight,
        ),
        help_action(
            keybind_label(&kb.cycle_pane_next),
            "cycle pane next",
            NavigateAction::CyclePaneNext,
        ),
        help_action(
            keybind_label(&kb.cycle_pane_previous),
            "cycle pane previous",
            NavigateAction::CyclePanePrevious,
        ),
        help_action(
            keybind_label(&kb.last_pane),
            "last pane",
            NavigateAction::LastPane,
        ),
    ];
    groups.push(("panes", panes));

    if !kb.custom_commands.is_empty() {
        groups.push((
            "custom",
            kb.custom_commands
                .iter()
                .map(|binding| HelpEntry {
                    key: binding.label.clone(),
                    label: binding
                        .description
                        .clone()
                        .map(Cow::Owned)
                        .unwrap_or(Cow::Borrowed("custom command")),
                    action: None,
                    keywords: &[],
                })
                .collect(),
        ));
    }

    groups
}

fn filter_keybind_help_groups(groups: Vec<HelpGroup>, query: &str) -> Vec<HelpGroup> {
    if query.is_empty() {
        return groups;
    }

    let query = query.to_lowercase();
    groups
        .into_iter()
        .filter_map(|(group, entries)| {
            let entries = entries
                .into_iter()
                .filter(|entry| {
                    entry.key.to_lowercase().contains(&query)
                        || entry.label.to_lowercase().contains(&query)
                        || entry
                            .keywords
                            .iter()
                            .any(|keyword| keyword.to_lowercase().contains(&query))
                })
                .collect::<Vec<_>>();
            (!entries.is_empty()).then_some((group, entries))
        })
        .collect()
}

pub(crate) fn keybind_help_lines(app: &AppState) -> Vec<(usize, Line<'static>)> {
    let heading_style = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(app.palette.mauve)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(app.palette.text);

    let groups = filter_keybind_help_groups(keybind_help_groups(app), &app.keybind_help.query);
    let key_width = groups
        .iter()
        .flat_map(|(_, entries)| entries.iter().map(|entry| entry.key.chars().count()))
        .max()
        .unwrap_or(8);

    let mut lines = Vec::new();

    if groups.is_empty() {
        let message = " no matching keybinds";
        return vec![(
            message.chars().count(),
            Line::from(Span::styled(
                message,
                Style::default().fg(app.palette.overlay1),
            )),
        )];
    }

    for (group, entries) in groups {
        lines.push((
            group.len() + 1,
            Line::from(vec![Span::styled(format!(" {group}"), heading_style)]),
        ));
        for entry in entries {
            let padded_key = format!(" {:<width$} ", entry.key, width = key_width);
            let width = padded_key.chars().count() + entry.label.chars().count();
            lines.push((
                width,
                Line::from(vec![
                    Span::styled(padded_key, key_style),
                    Span::styled(entry.label.into_owned(), label_style),
                ]),
            ));
        }
        lines.push((0, Line::raw("")));
    }

    lines
}

pub(super) fn render_keybind_help_overlay(app: &AppState, frame: &mut Frame) {
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

    render_modal_header(frame, header_rows[0], "keybinds", &app.palette);
    render_action_button(
        frame,
        release_notes_close_button_rect(header_rows[0]),
        Some("esc"),
        if app.keybind_help.search_focused {
            "back"
        } else {
            "close"
        },
        Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD),
    );
    let search_line = if app.keybind_help.search_focused {
        Line::from(vec![
            Span::styled(
                " / ",
                Style::default()
                    .fg(app.palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                app.keybind_help.query.as_str(),
                Style::default()
                    .fg(app.palette.text)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(Span::styled(
            " press / to filter by command or shortcut",
            Style::default().fg(app.palette.overlay0),
        ))
    };
    frame.render_widget(Paragraph::new(search_line), header_rows[1]);

    let body_area = stack.content;
    let metrics = crate::pane::ScrollMetrics {
        offset_from_bottom: app
            .keybind_help_max_scroll()
            .saturating_sub(app.keybind_help.scroll) as usize,
        max_offset_from_bottom: app.keybind_help_max_scroll() as usize,
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

    let body = Paragraph::new(
        keybind_help_lines(app)
            .into_iter()
            .map(|(_, line)| line)
            .collect::<Vec<_>>(),
    )
    .wrap(Wrap { trim: false })
    .scroll((app.keybind_help.scroll, 0));
    frame.render_widget(body, text_area);
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

    let footer = if app.keybind_help.search_focused {
        Line::from(vec![
            Span::styled(" filter ", Style::default().fg(app.palette.overlay0)),
            Span::styled("type/backspace", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("clear ", Style::default().fg(app.palette.overlay0)),
            Span::styled("ctrl+u", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("scroll ", Style::default().fg(app.palette.overlay0)),
            Span::styled("↑↓/pgup/pgdn", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("back ", Style::default().fg(app.palette.overlay0)),
            Span::styled("esc", Style::default().fg(app.palette.text)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" search ", Style::default().fg(app.palette.overlay0)),
            Span::styled("/", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("scroll ", Style::default().fg(app.palette.overlay0)),
            Span::styled("j/k/↑↓/pgup/pgdn", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("close ", Style::default().fg(app.palette.overlay0)),
            Span::styled("esc/enter", Style::default().fg(app.palette.text)),
        ])
    };
    frame.render_widget(Paragraph::new(footer), stack.footer.unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<HelpGroup> {
        vec![
            (
                "workspaces / tabs",
                vec![
                    help_entry("w", "workspace navigation"),
                    help_entry("c", "new tab"),
                ],
            ),
            (
                "panes",
                vec![
                    help_entry("v", "split vertical"),
                    help_entry("x", "close pane"),
                ],
            ),
        ]
    }

    #[test]
    fn keybind_help_filter_matches_labels_case_insensitively() {
        let filtered = filter_keybind_help_groups(groups(), "WoRk");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "workspaces / tabs");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].label, "workspace navigation");
    }

    #[test]
    fn keybind_help_filter_matches_shortcuts_without_matching_group_headings() {
        let filtered = filter_keybind_help_groups(groups(), "x");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "panes");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].label, "close pane");

        assert!(filter_keybind_help_groups(groups(), "panes").is_empty());
    }

    #[test]
    fn help_entry_and_help_action_carry_no_keywords_by_default() {
        assert!(help_entry("k", "some entry").keywords.is_empty());
        assert!(help_action("k", "some action", NavigateAction::ClosePane)
            .keywords
            .is_empty());
    }

    #[test]
    fn help_action_kw_carries_the_given_keywords() {
        let entry = help_action_kw(
            "v",
            "split vertical",
            NavigateAction::SplitVertical,
            &["new pane", "split right", "split left"],
        );
        assert_eq!(entry.keywords, &["new pane", "split right", "split left"]);
        assert_eq!(entry.action, Some(NavigateAction::SplitVertical));
    }
}
