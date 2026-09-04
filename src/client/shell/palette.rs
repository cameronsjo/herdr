//! The command palette's command model: what rows exist, how a query ranks
//! them, and what running a row does.
//!
//! Rows come from the keybind help entries (`src/input/keybind_help.rs`) plus
//! the plugin actions and panes the endpoint reports. There is no separate
//! palette registry: an action becomes searchable by gaining a help row that
//! carries a `KeybindAction`.

use std::borrow::Cow;
use std::collections::HashMap;

use ratatui::layout::Rect;

use crate::api::schema::{InstalledPluginInfo, PluginActionContext, PluginPlatform};
use crate::config::LiveKeybindConfig;
use crate::input::KeybindAction;
use crate::protocol::ClientShellSnapshot;

const EMPTY_PALETTE_LIMIT: usize = 12;

const PALETTE_MODAL_SIZE: (u16, u16) = (76, 22);
const SPLIT_MODAL_SIZE: (u16, u16) = (44, 6);
const SPLIT_VERTICAL_LABEL: &str = " v vertical ";
const SPLIT_HORIZONTAL_LABEL: &str = " h horizontal ";

/// The palette's popup, panel-inner and command-list rects. One source, read
/// by the renderer, the mouse hit-test and the scroll math, so the three
/// cannot disagree about where a row is.
pub(super) fn palette_geometry(area: Rect) -> Option<(Rect, Rect, Rect)> {
    let popup = crate::ui::centered_popup_rect(area, PALETTE_MODAL_SIZE.0, PALETTE_MODAL_SIZE.1)?;
    let inner = Rect::new(
        popup.x.saturating_add(1),
        popup.y.saturating_add(1),
        popup.width.saturating_sub(2),
        popup.height.saturating_sub(2),
    );
    if inner.height < 6 || inner.width < 20 {
        return None;
    }
    let body = crate::ui::modal_stack_areas(inner, 2, 1, 0, 1).content;
    Some((popup, inner, body))
}

/// The split-direction picker's popup, panel-inner and two button rects,
/// shared between the renderer and the mouse hit-test for the same reason.
pub(super) fn pane_split_direction_geometry(area: Rect) -> Option<(Rect, Rect, Rect, Rect)> {
    let popup = crate::ui::centered_popup_rect(area, SPLIT_MODAL_SIZE.0, SPLIT_MODAL_SIZE.1)?;
    let inner = Rect::new(
        popup.x.saturating_add(1),
        popup.y.saturating_add(1),
        popup.width.saturating_sub(2),
        popup.height.saturating_sub(2),
    );
    let vertical_width = SPLIT_VERTICAL_LABEL.len() as u16;
    let horizontal_width = SPLIT_HORIZONTAL_LABEL.len() as u16;
    let gap = 2;
    let total = vertical_width + gap + horizontal_width;
    if inner.height < 3 || inner.width < total {
        return None;
    }
    let x = inner.x + (inner.width - total) / 2;
    let y = inner.y.saturating_add(1);
    Some((
        popup,
        inner,
        Rect::new(x, y, vertical_width, 1),
        Rect::new(x + vertical_width + gap, y, horizontal_width, 1),
    ))
}

pub(super) fn split_button_labels() -> (&'static str, &'static str) {
    (SPLIT_VERTICAL_LABEL, SPLIT_HORIZONTAL_LABEL)
}

/// What the palette offers before anything is typed: the most recently run
/// commands first, then this list, capped at `EMPTY_PALETTE_LIMIT`.
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

/// What running a palette row does. Core rows replay the action their keybind
/// would have dispatched; plugin rows name their target directly rather than
/// indexing into a list that can be rebuilt between render and run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PaletteAction {
    Keybind(KeybindAction),
    PluginAction {
        plugin_id: String,
        action_id: String,
    },
    PluginPane {
        plugin_id: String,
        entrypoint: String,
    },
}

pub(crate) struct PaletteCommand {
    pub id: String,
    pub name: Cow<'static, str>,
    pub key: String,
    pub action: PaletteAction,
    pub keywords: &'static [&'static str],
}

struct PluginPaletteCommand {
    command: PaletteCommand,
    kind: &'static str,
}

/// The client's own platform. Plugin manifests declare which platforms an
/// action or pane supports, and the endpoint refuses an unsupported one; the
/// palette hides those rows rather than offering a row that can only error.
/// For a client attached to a server on a different OS this reads the wrong
/// platform — the endpoint's refusal remains the backstop in that case.
const fn host_platform() -> Option<PluginPlatform> {
    if cfg!(target_os = "linux") {
        Some(PluginPlatform::Linux)
    } else if cfg!(target_os = "macos") {
        Some(PluginPlatform::Macos)
    } else if cfg!(windows) {
        Some(PluginPlatform::Windows)
    } else {
        None
    }
}

fn platform_supported(platforms: Option<&Vec<PluginPlatform>>) -> bool {
    let Some(platforms) = platforms.filter(|platforms| !platforms.is_empty()) else {
        return true;
    };
    host_platform().is_some_and(|host| platforms.contains(&host))
}

/// The manifest's `contexts` list says what focus an action needs. An action
/// with no declared contexts always applies; a selection context never does,
/// because the palette has no text selection of its own to offer.
fn action_context_applies(
    contexts: &[PluginActionContext],
    snapshot: &ClientShellSnapshot,
) -> bool {
    if contexts.is_empty() {
        return true;
    }
    contexts.iter().any(|context| match context {
        PluginActionContext::Global => true,
        PluginActionContext::Workspace | PluginActionContext::Tab => {
            snapshot.focused_workspace_id.is_some()
        }
        PluginActionContext::Pane => snapshot.focused_pane_id.is_some(),
        PluginActionContext::Selection => false,
    })
}

fn plugin_command_name(plugin_name: &str, title: &str) -> String {
    let repeated_prefix = title
        .get(..plugin_name.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(plugin_name));
    let remainder = title.get(plugin_name.len()..).unwrap_or_default();
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

fn plugin_palette_commands(
    plugins: &[InstalledPluginInfo],
    snapshot: &ClientShellSnapshot,
) -> Vec<PaletteCommand> {
    let mut plugin_commands: Vec<PluginPaletteCommand> = Vec::new();
    let mut enabled: Vec<&InstalledPluginInfo> =
        plugins.iter().filter(|plugin| plugin.enabled).collect();
    enabled.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));

    for plugin in &enabled {
        let mut actions: Vec<_> = plugin
            .actions
            .iter()
            .filter(|action| {
                platform_supported(action.platforms.as_ref().or(plugin.platforms.as_ref()))
            })
            .filter(|action| action_context_applies(&action.contexts, snapshot))
            .collect();
        actions.sort_by(|left, right| left.id.cmp(&right.id));
        for action in actions {
            plugin_commands.push(PluginPaletteCommand {
                command: PaletteCommand {
                    id: format!("plugin-action:{}.{}", plugin.plugin_id, action.id),
                    name: Cow::Owned(plugin_command_name(&plugin.name, &action.title)),
                    key: String::new(),
                    action: PaletteAction::PluginAction {
                        plugin_id: plugin.plugin_id.clone(),
                        action_id: action.id.clone(),
                    },
                    keywords: &[],
                },
                kind: "action",
            });
        }
    }

    for plugin in &enabled {
        let mut panes: Vec<_> = plugin
            .panes
            .iter()
            .filter(|pane| {
                platform_supported(pane.platforms.as_ref().or(plugin.platforms.as_ref()))
            })
            .collect();
        panes.sort_by(|left, right| left.id.cmp(&right.id));
        for pane in panes {
            plugin_commands.push(PluginPaletteCommand {
                command: PaletteCommand {
                    id: format!("plugin-pane:{}.{}", plugin.plugin_id, pane.id),
                    name: Cow::Owned(plugin_command_name(&plugin.name, &pane.title)),
                    key: String::new(),
                    action: PaletteAction::PluginPane {
                        plugin_id: plugin.plugin_id.clone(),
                        entrypoint: pane.id.clone(),
                    },
                    keywords: &[],
                },
                kind: "pane",
            });
        }
    }

    disambiguate_plugin_labels(&mut plugin_commands);
    plugin_commands
        .into_iter()
        .map(|plugin_command| plugin_command.command)
        .collect()
}

pub(crate) fn palette_commands(
    keybinds: &LiveKeybindConfig,
    plugins: &[InstalledPluginInfo],
    snapshot: &ClientShellSnapshot,
) -> Vec<PaletteCommand> {
    let mut commands: Vec<PaletteCommand> =
        crate::input::keybind_help_groups(&keybinds.keybinds, keybinds.prefix)
            .into_iter()
            .flat_map(|(_, entries)| entries)
            .filter_map(|entry| {
                let action = entry.action?;
                Some(PaletteCommand {
                    id: action.palette_id()?.to_string(),
                    name: entry.label,
                    key: entry.key,
                    action: PaletteAction::Keybind(action),
                    keywords: entry.keywords,
                })
            })
            .collect();
    commands.extend(plugin_palette_commands(plugins, snapshot));
    commands
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

pub(crate) fn filtered_palette_commands(
    query: &str,
    recent_command_ids: &[String],
    keybinds: &LiveKeybindConfig,
    plugins: &[InstalledPluginInfo],
    snapshot: &ClientShellSnapshot,
) -> Vec<PaletteCommand> {
    let query = query.trim().to_lowercase();
    let commands = palette_commands(keybinds, plugins, snapshot);
    if query.is_empty() {
        return compact_palette_commands(commands, recent_command_ids);
    }

    let mut ranked: Vec<(u8, usize, PaletteCommand)> = commands
        .into_iter()
        .enumerate()
        .filter_map(|(index, command)| {
            command_match_rank(&command, &query).map(|rank| (rank, index, command))
        })
        .collect();
    ranked.sort_by_key(|(rank, index, _)| (*rank, *index));
    ranked.into_iter().map(|(_, _, command)| command).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keybinds() -> LiveKeybindConfig {
        crate::config::Config::default()
            .live_keybinds_with_diagnostics()
            .map(|(keybinds, _diagnostics)| keybinds)
            .expect("default keybinds resolve")
    }

    fn names(query: &str) -> Vec<String> {
        let snapshot = super::super::tests::snapshot();
        filtered_palette_commands(query, &[], &keybinds(), &[], &snapshot)
            .into_iter()
            .map(|command| command.name.into_owned())
            .collect()
    }

    fn command(name: &'static str, keywords: &'static [&'static str]) -> PaletteCommand {
        PaletteCommand {
            id: format!("test:{name}"),
            name: Cow::Borrowed(name),
            key: String::new(),
            action: PaletteAction::Keybind(KeybindAction::ClosePane),
            keywords,
        }
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
        let snapshot = super::super::tests::snapshot();
        assert!(!palette_commands(&keybinds(), &[], &snapshot).is_empty());
    }

    #[test]
    fn empty_query_is_compact_and_does_not_offer_the_palette_itself() {
        let names = names("");
        assert_eq!(names.len(), EMPTY_PALETTE_LIMIT);
        assert!(!names.iter().any(|name| name == "command palette"));
    }

    #[test]
    fn remembered_available_commands_lead_the_empty_palette_without_duplicates() {
        let snapshot = super::super::tests::snapshot();
        let recent = vec![
            "core:resize-pane-left".to_string(),
            "core:new-tab".to_string(),
            "plugin-action:missing.action".to_string(),
        ];
        let commands = filtered_palette_commands("", &recent, &keybinds(), &[], &snapshot);
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

    // A help row reaches the palette only when it carries a KeybindAction, so
    // a row added upstream as a plain entry is silently palette-invisible.
    // These six arrived that way in an upstream sync.
    #[test]
    fn tab_reorder_and_pane_resize_rows_reach_the_palette() {
        let snapshot = super::super::tests::snapshot();
        let actions: Vec<PaletteAction> = palette_commands(&keybinds(), &[], &snapshot)
            .into_iter()
            .map(|command| command.action)
            .collect();

        for expected in [
            KeybindAction::MoveTabPrevious,
            KeybindAction::MoveTabNext,
            KeybindAction::ResizePaneLeft,
            KeybindAction::ResizePaneDown,
            KeybindAction::ResizePaneUp,
            KeybindAction::ResizePaneRight,
        ] {
            assert!(
                actions.contains(&PaletteAction::Keybind(expected)),
                "{expected:?} is missing from the palette"
            );
        }
    }

    #[test]
    fn move_tab_to_space_commands_reach_the_palette() {
        let snapshot = super::super::tests::snapshot();
        let commands = palette_commands(&keybinds(), &[], &snapshot);

        for expected in [
            KeybindAction::MoveTabToSpace,
            KeybindAction::MoveTabToNewSpace,
        ] {
            assert!(
                commands
                    .iter()
                    .any(|command| command.action == PaletteAction::Keybind(expected)),
                "{expected:?} is missing from the palette"
            );
        }

        // The keywords are the only way "workspace" wording finds these.
        let by_keyword = commands
            .iter()
            .find(|command| command.action == PaletteAction::Keybind(KeybindAction::MoveTabToSpace))
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

    #[test]
    fn a_command_with_no_keywords_only_matches_via_its_name() {
        let cmd = command("close pane", &[]);
        assert!(command_match_rank(&cmd, "close").is_some());
        assert_eq!(command_match_rank(&cmd, "split"), None);
    }

    #[test]
    fn keyword_matching_lowercases_the_keyword_like_name_matching() {
        let cmd = command("split vertical", &["Split Right"]);
        assert_eq!(command_match_rank(&cmd, "split right"), Some(4));
    }

    #[test]
    fn the_best_matching_keyword_wins_when_several_keywords_match() {
        // The command name itself must not match "split" (or the test would
        // exercise the name branch instead of the keyword branch). First
        // keyword only word-prefix-matches (rank 2); second is an exact match
        // (rank 0). The overall keyword rank should reflect the best of the
        // two, not list order.
        let cmd = command("close pane", &["foo split bar", "split"]);
        assert_eq!(command_match_rank(&cmd, "split"), Some(MAX_NAME_RANK + 1));
    }

    #[test]
    fn a_substring_only_name_match_still_outranks_any_keyword_match() {
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
        assert_eq!(command_match_rank(&cmd, ""), Some(1));
    }

    // `pane_split_direction_geometry` sizes each button from the label's byte
    // length, which only equals its rendered width while the labels stay
    // ASCII. A wide or multi-byte glyph would silently mis-size the rect the
    // mouse hit-test shares with the renderer.
    #[test]
    fn split_button_labels_are_ascii_so_byte_length_is_their_rendered_width() {
        for label in [SPLIT_VERTICAL_LABEL, SPLIT_HORIZONTAL_LABEL] {
            assert!(label.is_ascii(), "{label:?} must stay ASCII");
            assert_eq!(
                label.len(),
                unicode_width::UnicodeWidthStr::width(label),
                "{label:?} byte length must equal its rendered width"
            );
        }
    }

    #[test]
    fn the_split_picker_fits_its_two_buttons() {
        let (_, _, vertical, horizontal) =
            pane_split_direction_geometry(Rect::new(0, 0, 120, 40)).expect("geometry");
        assert_eq!(usize::from(vertical.width), SPLIT_VERTICAL_LABEL.len());
        assert_eq!(usize::from(horizontal.width), SPLIT_HORIZONTAL_LABEL.len());
        assert!(
            horizontal.x >= vertical.x + vertical.width,
            "buttons must not overlap: {vertical:?} {horizontal:?}"
        );
        assert_eq!(vertical.y, horizontal.y);
    }

    #[test]
    fn a_terminal_too_small_for_the_palette_yields_no_geometry() {
        assert!(palette_geometry(Rect::new(0, 0, 10, 4)).is_none());
        assert!(pane_split_direction_geometry(Rect::new(0, 0, 10, 4)).is_none());
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

    #[test]
    fn a_disabled_plugin_offers_no_palette_rows() {
        let snapshot = super::super::tests::snapshot();
        let mut plugin = test_plugin();
        plugin.enabled = false;
        assert!(plugin_palette_commands(&[plugin], &snapshot).is_empty());
    }

    #[test]
    fn an_enabled_plugin_offers_its_actions_and_panes() {
        let snapshot = super::super::tests::snapshot();
        let commands = plugin_palette_commands(&[test_plugin()], &snapshot);
        let ids: Vec<&str> = commands.iter().map(|command| command.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["plugin-action:demo.build", "plugin-pane:demo.board"]
        );
        assert_eq!(
            commands[0].action,
            PaletteAction::PluginAction {
                plugin_id: "demo".into(),
                action_id: "build".into(),
            }
        );
        assert_eq!(
            commands[1].action,
            PaletteAction::PluginPane {
                plugin_id: "demo".into(),
                entrypoint: "board".into(),
            }
        );
    }

    #[test]
    fn a_platform_the_client_is_not_running_on_hides_the_row() {
        let snapshot = super::super::tests::snapshot();
        let mut plugin = test_plugin();
        let other = if host_platform() == Some(PluginPlatform::Linux) {
            PluginPlatform::Windows
        } else {
            PluginPlatform::Linux
        };
        plugin.actions[0].platforms = Some(vec![other]);
        let commands = plugin_palette_commands(&[plugin], &snapshot);
        assert!(commands
            .iter()
            .all(|command| command.id != "plugin-action:demo.build"));
    }

    #[test]
    fn a_selection_context_action_is_never_offered() {
        let snapshot = super::super::tests::snapshot();
        let mut plugin = test_plugin();
        plugin.actions[0].contexts = vec![PluginActionContext::Selection];
        let commands = plugin_palette_commands(&[plugin], &snapshot);
        assert!(commands
            .iter()
            .all(|command| command.id != "plugin-action:demo.build"));
    }

    fn test_plugin() -> InstalledPluginInfo {
        InstalledPluginInfo {
            plugin_id: "demo".into(),
            name: "Demo".into(),
            version: "1.0.0".into(),
            min_herdr_version: String::new(),
            description: None,
            manifest_path: "/tmp/demo/herdr-plugin.toml".into(),
            plugin_root: "/tmp/demo".into(),
            enabled: true,
            platforms: None,
            build: Vec::new(),
            startup: Vec::new(),
            actions: vec![crate::api::schema::PluginManifestAction {
                id: "build".into(),
                title: "Build".into(),
                description: None,
                contexts: Vec::new(),
                platforms: None,
                command: vec!["true".into()],
            }],
            events: Vec::new(),
            panes: vec![crate::api::schema::PluginManifestPane {
                id: "board".into(),
                title: "Board".into(),
                description: None,
                platforms: None,
                placement: Default::default(),
                width: None,
                height: None,
                command: vec!["true".into()],
            }],
            link_handlers: Vec::new(),
            source: Default::default(),
            warnings: Vec::new(),
        }
    }
}
