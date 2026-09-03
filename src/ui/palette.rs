use std::{borrow::Cow, collections::HashMap};

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use tracing::{debug, trace};

use super::release_notes::release_notes_close_button_rect;
use super::scrollbar::{release_notes_scrollbar_rect, render_scrollbar};
use super::widgets::{
    modal_stack_areas, panel_contrast_fg, render_action_button, render_modal_header,
    render_modal_shell,
};
use crate::app::{AppState, NavigateAction};

const EMPTY_PALETTE_LIMIT: usize = 12;
const FEATURED_COMMAND_IDS: [&str; EMPTY_PALETTE_LIMIT] = [
    "core:new-workspace",
    "core:new-worktree",
    "core:open-worktree",
    "core:new-tab",
    "core:split-vertical",
    "core:split-horizontal",
    "core:move-pane-to-space",
    "core:move-pane-to-new-tab",
    "core:move-tab-to-space",
    "core:zoom-pane",
    "core:toggle-sidebar",
    "core:settings",
];

pub(crate) struct PaletteCommand {
    pub id: String,
    pub name: Cow<'static, str>,
    pub key: String,
    pub action: NavigateAction,
    pub keywords: &'static [&'static str],
}

struct PluginPaletteCommand {
    command: PaletteCommand,
    kind: &'static str,
}

pub(crate) fn palette_commands(app: &AppState) -> Vec<PaletteCommand> {
    debug!("Building palette commands from keybind help groups");
    let mut commands: Vec<PaletteCommand> = super::keybind_help::keybind_help_groups(app)
        .into_iter()
        .flat_map(|(_, entries)| entries)
        .filter_map(|entry| {
            let action: NavigateAction = entry.action?;
            Some(PaletteCommand {
                id: action.palette_id()?.to_string(),
                name: entry.label,
                key: entry.key,
                action,
                keywords: entry.keywords,
            })
        })
        .collect();
    trace!(
        keybind_count = commands.len(),
        "Loaded keybind help entries"
    );

    let plugin_actions = crate::app::palette_plugin_actions(app);
    let plugin_action_count = plugin_actions.len();
    let mut plugin_commands: Vec<PluginPaletteCommand> = plugin_actions
        .into_iter()
        .enumerate()
        .map(|(index, action)| {
            let display_name: String = plugin_display_name(app, &action.plugin_id);
            PluginPaletteCommand {
                command: PaletteCommand {
                    id: format!("plugin-action:{}", action.qualified_id()),
                    name: Cow::Owned(plugin_command_name(&display_name, &action.title)),
                    key: String::new(),
                    action: NavigateAction::InvokePluginAction(index),
                    keywords: &[],
                },
                kind: "action",
            }
        })
        .collect();
    trace!(plugin_action_count, "Added plugin actions");

    let plugin_panes = crate::app::palette_plugin_panes(app);
    let plugin_pane_count = plugin_panes.len();
    plugin_commands.extend(plugin_panes.into_iter().enumerate().map(
        |(index, (plugin_id, pane))| {
            let display_name: String = plugin_display_name(app, &plugin_id);
            PluginPaletteCommand {
                command: PaletteCommand {
                    id: format!("plugin-pane:{plugin_id}.{}", pane.id),
                    name: Cow::Owned(plugin_command_name(&display_name, &pane.title)),
                    key: String::new(),
                    action: NavigateAction::OpenPluginPane(index),
                    keywords: &[],
                },
                kind: "pane",
            }
        },
    ));
    trace!(plugin_pane_count, "Added plugin panes");

    disambiguate_plugin_labels(&mut plugin_commands);
    commands.extend(
        plugin_commands
            .into_iter()
            .map(|plugin_command| plugin_command.command),
    );

    debug!(
        total_commands = commands.len(),
        "Successfully built palette commands"
    );
    commands
}

fn plugin_display_name(app: &AppState, plugin_id: &str) -> String {
    app.installed_plugins
        .get(plugin_id)
        .map(|plugin| plugin.name.clone())
        .unwrap_or_else(|| plugin_id.to_string())
}

fn plugin_command_name(plugin_name: &str, title: &str) -> String {
    let repeated_prefix: bool = title
        .get(..plugin_name.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(plugin_name));
    let remainder: &str = title.get(plugin_name.len()..).unwrap_or_default();
    if repeated_prefix
        && (remainder.is_empty()
            || remainder.starts_with(':')
            || remainder.starts_with(" —")
            || remainder.starts_with(" -"))
    {
        title.to_string()
    } else {
        format!("{plugin_name} — {title}")
    }
}

fn disambiguate_plugin_labels(plugin_commands: &mut [PluginPaletteCommand]) {
    let mut label_counts: HashMap<String, usize> = HashMap::new();
    for plugin_command in plugin_commands.iter() {
        *label_counts
            .entry(plugin_command.command.name.to_string())
            .or_default() += 1;
    }
    for plugin_command in plugin_commands {
        if label_counts
            .get(plugin_command.command.name.as_ref())
            .is_some_and(|count| *count > 1)
        {
            plugin_command.command.name = Cow::Owned(format!(
                "{} ({})",
                plugin_command.command.name, plugin_command.kind
            ));
        }
    }
}

/// Ranking by match quality rather than list order keeps a query that exactly
/// names one command from being answered by a longer command containing it.
/// `MAX_NAME_RANK` is the worst (highest) rank this function returns —
/// `command_match_rank` derives its keyword-tier offset from it so the two
/// stay coupled structurally instead of by two files agreeing on a number.
const MAX_NAME_RANK: u8 = 3;

fn match_rank(name: &str, query: &str) -> Option<u8> {
    let name = name.to_lowercase();
    if name == query {
        Some(0)
    } else if name.starts_with(query) {
        Some(1)
    } else if name.split_whitespace().any(|word| word.starts_with(query)) {
        Some(2)
    } else if name.contains(query) {
        Some(MAX_NAME_RANK)
    } else {
        None
    }
}

/// A keyword match (e.g. "split right" finding the "split vertical" command)
/// always ranks below every name match, so a command whose own name answers
/// the query is never outranked by a synonym on a different command.
fn command_match_rank(command: &PaletteCommand, query: &str) -> Option<u8> {
    if let Some(rank) = match_rank(&command.name, query) {
        return Some(rank);
    }
    command
        .keywords
        .iter()
        .filter_map(|keyword| match_rank(keyword, query))
        .min()
        .map(|rank| rank + MAX_NAME_RANK + 1)
}

fn compact_palette_commands(
    commands: Vec<PaletteCommand>,
    recent_command_ids: &[String],
) -> Vec<PaletteCommand> {
    let mut commands_by_id: HashMap<String, PaletteCommand> = commands
        .into_iter()
        .map(|command| (command.id.clone(), command))
        .collect();
    recent_command_ids
        .iter()
        .map(String::as_str)
        .chain(FEATURED_COMMAND_IDS)
        .filter_map(|command_id| commands_by_id.remove(command_id))
        .take(EMPTY_PALETTE_LIMIT)
        .collect()
}

pub(crate) fn filtered_palette_commands(app: &AppState) -> Vec<PaletteCommand> {
    let query: String = app.command_palette.query.trim().to_lowercase();
    let commands: Vec<PaletteCommand> = palette_commands(app);
    if query.is_empty() {
        let compact: Vec<PaletteCommand> =
            compact_palette_commands(commands, &app.command_palette.recent_command_ids);
        trace!(
            result_count = compact.len(),
            "Returning compact palette commands for an empty query"
        );
        return compact;
    }

    debug!(query = %query, available_commands = commands.len(), "Filtering palette commands");
    let mut ranked: Vec<(u8, usize, PaletteCommand)> = commands
        .into_iter()
        .enumerate()
        .filter_map(|(index, command)| {
            command_match_rank(&command, &query).map(|rank| (rank, index, command))
        })
        .collect();

    let matched_count: usize = ranked.len();
    debug!(query = %query, matched_commands = matched_count, "Command matching complete");

    ranked.sort_by_key(|(rank, index, _)| (*rank, *index));
    let result: Vec<PaletteCommand> = ranked.into_iter().map(|(_, _, command)| command).collect();

    trace!(query = %query, result_count = result.len(), "Returning filtered palette commands");
    result
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
        Span::styled(
            "recent first · type to search all",
            Style::default().fg(app.palette.overlay0),
        )
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

    #[test]
    fn empty_query_is_compact_and_does_not_offer_the_palette_itself() {
        let names = names("");
        assert_eq!(names.len(), EMPTY_PALETTE_LIMIT);
        assert!(!names.iter().any(|name| name == "command palette"));
    }

    #[test]
    fn remembered_available_commands_lead_the_empty_palette_without_duplicates() {
        let mut state = AppState::test_new();
        state.command_palette.recent_command_ids = vec![
            "core:resize-pane-left".to_string(),
            "core:new-tab".to_string(),
            "plugin-action:missing.action".to_string(),
        ];
        let commands = filtered_palette_commands(&state);
        let ids: Vec<&str> = commands.iter().map(|command| command.id.as_str()).collect();

        assert_eq!(ids.first().copied(), Some("core:resize-pane-left"));
        assert_eq!(ids.get(1).copied(), Some("core:new-tab"));
        assert_eq!(ids.iter().filter(|id| **id == "core:new-tab").count(), 1);
        assert_eq!(ids.len(), EMPTY_PALETTE_LIMIT);
    }

    #[test]
    fn typing_searches_commands_omitted_from_the_compact_palette() {
        let matches = names("resize pane left");
        assert_eq!(
            matches.first().map(String::as_str),
            Some("resize pane left")
        );
    }

    #[test]
    fn the_palette_self_action_is_not_searchable() {
        assert!(names("command palette").is_empty());
    }

    #[test]
    fn plugin_titles_do_not_repeat_an_existing_brand_prefix() {
        assert_eq!(
            plugin_command_name("Herdr Plus", "Herdr Plus: Projects"),
            "Herdr Plus: Projects"
        );
        assert_eq!(
            plugin_command_name("Browser", "Open localhost"),
            "Browser — Open localhost"
        );
    }

    #[test]
    fn identical_plugin_action_and_pane_labels_show_their_kind() {
        let mut commands = vec![
            PluginPaletteCommand {
                command: command("Herdr Plus: Projects", &[]),
                kind: "action",
            },
            PluginPaletteCommand {
                command: command("Herdr Plus: Projects", &[]),
                kind: "pane",
            },
        ];
        disambiguate_plugin_labels(&mut commands);

        assert_eq!(commands[0].command.name, "Herdr Plus: Projects (action)");
        assert_eq!(commands[1].command.name, "Herdr Plus: Projects (pane)");
    }

    // A help row reaches the palette only when it carries a NavigateAction, so
    // a row added upstream as a plain entry is silently palette-invisible. These
    // six arrived that way in an upstream sync.
    #[test]
    fn tab_reorder_and_pane_resize_rows_reach_the_palette() {
        let state = AppState::test_new();
        let actions: Vec<NavigateAction> = palette_commands(&state)
            .into_iter()
            .map(|command| command.action)
            .collect();

        for expected in [
            NavigateAction::MoveTabPrevious,
            NavigateAction::MoveTabNext,
            NavigateAction::ResizePaneLeft,
            NavigateAction::ResizePaneDown,
            NavigateAction::ResizePaneUp,
            NavigateAction::ResizePaneRight,
        ] {
            assert!(
                actions.contains(&expected),
                "{expected:?} is missing from the palette"
            );
        }
    }

    #[test]
    fn move_tab_to_space_commands_reach_the_palette() {
        let state = AppState::test_new();
        let commands = palette_commands(&state);
        let actions: Vec<NavigateAction> = commands.iter().map(|command| command.action).collect();

        for expected in [
            NavigateAction::MoveTabToSpace,
            NavigateAction::MoveTabToNewSpace,
        ] {
            assert!(
                actions.contains(&expected),
                "{expected:?} is missing from the palette"
            );
        }

        // The keywords are the only way "workspace" wording finds these.
        let by_keyword = commands
            .iter()
            .find(|command| command.action == NavigateAction::MoveTabToSpace)
            .expect("move tab to space command");
        assert!(
            by_keyword
                .keywords
                .iter()
                .any(|keyword| keyword.contains("workspace")),
            "move tab to space should be findable by workspace vocabulary"
        );
    }

    #[test]
    fn new_pane_matches_both_split_commands_via_keywords() {
        let matches = names("new pane");
        assert!(
            matches.iter().any(|name| name == "split vertical"),
            "got {matches:?}"
        );
        assert!(
            matches.iter().any(|name| name == "split horizontal"),
            "got {matches:?}"
        );
    }

    #[test]
    fn split_right_matches_split_vertical_via_keyword() {
        let matches = names("split right");
        assert_eq!(
            matches.first().map(String::as_str),
            Some("split vertical"),
            "got {matches:?}"
        );
    }

    #[test]
    fn split_down_matches_split_horizontal_via_keyword() {
        let matches = names("split down");
        assert_eq!(
            matches.first().map(String::as_str),
            Some("split horizontal"),
            "got {matches:?}"
        );
    }

    #[test]
    fn a_name_match_outranks_a_keyword_match_on_another_command() {
        // "split vertical" is itself a command name; make sure that direct
        // name match wins over any keyword-based hit from another entry.
        let matches = names("split vertical");
        assert_eq!(
            matches.first().map(String::as_str),
            Some("split vertical"),
            "got {matches:?}"
        );
    }

    fn command(name: &'static str, keywords: &'static [&'static str]) -> PaletteCommand {
        PaletteCommand {
            id: format!("test:{name}"),
            name: Cow::Borrowed(name),
            key: String::new(),
            action: NavigateAction::ClosePane,
            keywords,
        }
    }

    #[test]
    fn a_command_with_no_keywords_only_matches_via_its_name() {
        let cmd = command("close pane", &[]);
        assert!(command_match_rank(&cmd, "close").is_some());
        assert_eq!(command_match_rank(&cmd, "split"), None);
    }

    #[test]
    fn keyword_matching_lowercases_the_keyword_like_name_matching() {
        // `match_rank` lowercases `name` before comparing; keyword matching
        // reuses `match_rank`, so a mixed-case authored keyword should match
        // a lowercase query exactly the same way a mixed-case name would.
        let cmd = command("split vertical", &["Split Right"]);
        assert_eq!(command_match_rank(&cmd, "split right"), Some(4));
    }

    #[test]
    fn the_best_matching_keyword_wins_when_several_keywords_match() {
        // The command name itself must not match "split" (or the test would
        // exercise the name branch instead of the keyword branch). First
        // keyword only word-prefix-matches (rank 2); second is an exact
        // match (rank 0). The command's overall keyword rank should reflect
        // the best (lowest) of the two, not list order.
        let cmd = command("close pane", &["foo split bar", "split"]);
        assert_eq!(command_match_rank(&cmd, "split"), Some(MAX_NAME_RANK + 1));
    }

    #[test]
    fn a_substring_only_name_match_still_outranks_any_keyword_match() {
        // "xxsplitxx" contains "split" only as a mid-word substring (the
        // worst name-match tier, MAX_NAME_RANK); an exact keyword match on
        // a different command is a strictly worse (higher) rank, so the
        // name match must still sort first.
        let name_match = command("xxsplitxx", &[]);
        let keyword_match = command("close pane", &["split"]);
        let name_rank = command_match_rank(&name_match, "split").expect("name should match");
        let keyword_rank =
            command_match_rank(&keyword_match, "split").expect("keyword should match");
        assert_eq!(name_rank, MAX_NAME_RANK);
        assert!(
            name_rank < keyword_rank,
            "name_rank={name_rank} keyword_rank={keyword_rank}"
        );
    }

    #[test]
    fn empty_query_keyword_lookup_does_not_panic() {
        let cmd = command("split vertical", &["split right"]);
        // An empty query makes every name/keyword a "starts_with" match
        // (rank 1); this only guards against a panic/regression in the
        // keyword path when `filtered_palette_commands` short-circuits on
        // an empty query before ever calling `command_match_rank`.
        assert_eq!(command_match_rank(&cmd, ""), Some(1));
    }
}
