use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    config::{ActionKeybinds, IndexedKeybind, Keybinds},
    input::{KeybindAction, TerminalKey},
};

/// One row of the keybind help — and, when it carries an `action`, one command
/// palette row too. The palette has no registry of its own: a new action
/// becomes searchable by gaining a row here.
#[derive(Debug, Clone)]
pub(crate) struct KeybindHelpEntry {
    pub key: String,
    pub label: Cow<'static, str>,
    pub action: Option<KeybindAction>,
    /// Extra search terms the command palette matches this entry against
    /// (e.g. "split right" for the "split vertical" action) — a surfaced-
    /// elsewhere synonym, not a full alias system. Hand-authored per entry;
    /// nothing keeps siblings sharing a synonym in sync.
    pub keywords: &'static [&'static str],
}

pub(crate) type KeybindHelpGroup = (&'static str, Vec<KeybindHelpEntry>);

pub(crate) fn keybind_help_text_char(key: &TerminalKey) -> Option<char> {
    if !key.modifiers.difference(KeyModifiers::SHIFT).is_empty() {
        return None;
    }
    if let Some(character) = key.shifted_codepoint.and_then(char::from_u32) {
        return Some(character);
    }
    let KeyCode::Char(character) = key.code else {
        return None;
    };
    Some(character)
}

fn entry(key: impl Into<String>, label: &'static str) -> KeybindHelpEntry {
    KeybindHelpEntry {
        key: key.into(),
        label: Cow::Borrowed(label),
        action: None,
        keywords: &[],
    }
}

fn action_entry(
    key: impl Into<String>,
    label: &'static str,
    action: KeybindAction,
) -> KeybindHelpEntry {
    KeybindHelpEntry {
        key: key.into(),
        label: Cow::Borrowed(label),
        action: Some(action),
        keywords: &[],
    }
}

fn action_entry_kw(
    key: impl Into<String>,
    label: &'static str,
    action: KeybindAction,
    keywords: &'static [&'static str],
) -> KeybindHelpEntry {
    KeybindHelpEntry {
        key: key.into(),
        label: Cow::Borrowed(label),
        action: Some(action),
        keywords,
    }
}

fn binding_label(bindings: &ActionKeybinds) -> String {
    bindings.label().unwrap_or_else(|| "unset".to_owned())
}

fn indexed_label(bindings: &[IndexedKeybind]) -> String {
    if bindings.is_empty() {
        return "unset".to_owned();
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

fn indexed_range_prefix(bindings: &[IndexedKeybind]) -> Option<&str> {
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

pub(crate) fn keybind_help_groups(
    keybinds: &Keybinds,
    prefix: (crossterm::event::KeyCode, crossterm::event::KeyModifiers),
) -> Vec<KeybindHelpGroup> {
    let mut groups = vec![
        (
            "global",
            vec![
                entry(crate::config::format_key_combo(prefix), "prefix mode"),
                entry(binding_label(&keybinds.help), "keybinds"),
                action_entry(
                    binding_label(&keybinds.settings),
                    "settings",
                    KeybindAction::Settings,
                ),
                action_entry(
                    binding_label(&keybinds.detach),
                    "detach",
                    KeybindAction::Detach,
                ),
                action_entry(
                    binding_label(&keybinds.reload_config),
                    "reload config",
                    KeybindAction::ReloadConfig,
                ),
                action_entry(
                    binding_label(&keybinds.open_notification_target),
                    "open notification target",
                    KeybindAction::OpenNotificationTarget,
                ),
            ],
        ),
        (
            "navigation",
            vec![
                entry("esc", "back"),
                entry(
                    format!(
                        "{} / {}",
                        binding_label(&keybinds.navigate.workspace_up),
                        binding_label(&keybinds.navigate.workspace_down)
                    ),
                    "workspace list",
                ),
                entry(
                    format!(
                        "{} / {} / {} / {} / left / right",
                        binding_label(&keybinds.navigate.pane_left),
                        binding_label(&keybinds.navigate.pane_down),
                        binding_label(&keybinds.navigate.pane_up),
                        binding_label(&keybinds.navigate.pane_right)
                    ),
                    "move focus",
                ),
                entry("tab / shift+tab", "cycle pane"),
                entry("enter", "open workspace"),
                entry("1..9", "switch workspace"),
            ],
        ),
        (
            "workspaces / tabs",
            vec![
                action_entry(
                    binding_label(&keybinds.workspace_picker),
                    "workspace navigation",
                    KeybindAction::WorkspacePicker,
                ),
                action_entry(
                    binding_label(&keybinds.command_palette),
                    "command palette",
                    KeybindAction::OpenCommandPalette,
                ),
                action_entry(
                    binding_label(&keybinds.goto),
                    "session navigator",
                    KeybindAction::OpenNavigator,
                ),
                action_entry(
                    binding_label(&keybinds.new_workspace),
                    "new workspace",
                    KeybindAction::NewWorkspace,
                ),
                action_entry(
                    binding_label(&keybinds.new_worktree),
                    "new worktree",
                    KeybindAction::NewWorktree,
                ),
                action_entry(
                    binding_label(&keybinds.open_worktree),
                    "open worktree",
                    KeybindAction::OpenWorktree,
                ),
                action_entry(
                    binding_label(&keybinds.remove_worktree),
                    "delete worktree checkout",
                    KeybindAction::RemoveWorktree,
                ),
                action_entry(
                    binding_label(&keybinds.rename_workspace),
                    "rename workspace",
                    KeybindAction::RenameWorkspace,
                ),
                action_entry(
                    binding_label(&keybinds.close_workspace),
                    "close workspace",
                    KeybindAction::CloseWorkspace,
                ),
                action_entry_kw(
                    binding_label(&keybinds.merge_workspace),
                    "merge workspace into...",
                    KeybindAction::MergeWorkspace,
                    &[
                        "combine workspaces",
                        "merge space",
                        "move all tabs to another space",
                    ],
                ),
                action_entry(
                    binding_label(&keybinds.previous_workspace),
                    "previous workspace",
                    KeybindAction::PreviousWorkspace,
                ),
                action_entry(
                    binding_label(&keybinds.next_workspace),
                    "next workspace",
                    KeybindAction::NextWorkspace,
                ),
                action_entry_kw(
                    binding_label(&keybinds.move_workspace_previous),
                    "move workspace up",
                    KeybindAction::MoveWorkspacePrevious,
                    &["reorder workspace", "move workspace left", "move space up"],
                ),
                action_entry_kw(
                    binding_label(&keybinds.move_workspace_next),
                    "move workspace down",
                    KeybindAction::MoveWorkspaceNext,
                    &[
                        "reorder workspace",
                        "move workspace right",
                        "move space down",
                    ],
                ),
                entry(
                    indexed_label(&keybinds.switch_workspace),
                    "switch workspace 1-9",
                ),
                action_entry(
                    binding_label(&keybinds.previous_agent),
                    "previous agent",
                    KeybindAction::PreviousAgent,
                ),
                action_entry(
                    binding_label(&keybinds.next_agent),
                    "next agent",
                    KeybindAction::NextAgent,
                ),
                entry(indexed_label(&keybinds.focus_agent), "focus agent 1-9"),
                action_entry(
                    binding_label(&keybinds.new_tab),
                    "new tab",
                    KeybindAction::NewTab,
                ),
                action_entry(
                    binding_label(&keybinds.rename_tab),
                    "rename tab",
                    KeybindAction::RenameTab,
                ),
                action_entry(
                    binding_label(&keybinds.previous_tab),
                    "previous tab",
                    KeybindAction::PreviousTab,
                ),
                action_entry(
                    binding_label(&keybinds.next_tab),
                    "next tab",
                    KeybindAction::NextTab,
                ),
                action_entry(
                    binding_label(&keybinds.move_tab_previous),
                    "move tab left",
                    KeybindAction::MoveTabPrevious,
                ),
                action_entry(
                    binding_label(&keybinds.move_tab_next),
                    "move tab right",
                    KeybindAction::MoveTabNext,
                ),
                entry(indexed_label(&keybinds.switch_tab), "switch tab 1-9"),
                action_entry_kw(
                    binding_label(&keybinds.move_tab_to_space),
                    "move tab to space",
                    KeybindAction::MoveTabToSpace,
                    &["send tab to workspace", "move tab to another space"],
                ),
                action_entry_kw(
                    binding_label(&keybinds.move_tab_to_new_space),
                    "move tab to new space",
                    KeybindAction::MoveTabToNewSpace,
                    &["split tab into workspace", "new space from tab"],
                ),
                action_entry(
                    binding_label(&keybinds.close_tab),
                    "close tab",
                    KeybindAction::CloseTab,
                ),
            ],
        ),
        (
            "panes",
            vec![
                action_entry_kw(
                    binding_label(&keybinds.split_vertical),
                    "split vertical",
                    KeybindAction::SplitVertical,
                    &["new pane", "split right", "split left"],
                ),
                action_entry_kw(
                    binding_label(&keybinds.split_horizontal),
                    "split horizontal",
                    KeybindAction::SplitHorizontal,
                    &["new pane", "split down", "split up"],
                ),
                action_entry(
                    binding_label(&keybinds.close_pane),
                    "close pane",
                    KeybindAction::ClosePane,
                ),
                action_entry(
                    binding_label(&keybinds.rename_pane),
                    "rename pane",
                    KeybindAction::RenamePane,
                ),
                action_entry_kw(
                    binding_label(&keybinds.move_pane_to_space),
                    "move pane to space",
                    KeybindAction::MovePaneToSpace,
                    &["send pane to workspace", "move pane to another space"],
                ),
                action_entry_kw(
                    binding_label(&keybinds.move_pane_to_new_space),
                    "move pane to new space",
                    KeybindAction::MovePaneToNewSpace,
                    &["split pane into workspace", "new space from pane"],
                ),
                action_entry_kw(
                    binding_label(&keybinds.move_pane_to_new_tab),
                    "move pane to new tab",
                    KeybindAction::MovePaneToNewTab,
                    &["send pane to new tab"],
                ),
                entry(binding_label(&keybinds.edit_scrollback), "edit scrollback"),
                entry(binding_label(&keybinds.copy_mode), "copy mode"),
                action_entry(
                    binding_label(&keybinds.zoom),
                    "zoom pane",
                    KeybindAction::Zoom,
                ),
                action_entry(
                    binding_label(&keybinds.resize_mode),
                    "resize mode",
                    KeybindAction::EnterResizeMode,
                ),
                action_entry(
                    binding_label(&keybinds.resize_pane_left),
                    "resize pane left",
                    KeybindAction::ResizePaneLeft,
                ),
                action_entry(
                    binding_label(&keybinds.resize_pane_down),
                    "resize pane down",
                    KeybindAction::ResizePaneDown,
                ),
                action_entry(
                    binding_label(&keybinds.resize_pane_up),
                    "resize pane up",
                    KeybindAction::ResizePaneUp,
                ),
                action_entry(
                    binding_label(&keybinds.resize_pane_right),
                    "resize pane right",
                    KeybindAction::ResizePaneRight,
                ),
                action_entry(
                    binding_label(&keybinds.toggle_sidebar),
                    "toggle sidebar",
                    KeybindAction::ToggleSidebar,
                ),
                action_entry(
                    binding_label(&keybinds.focus_pane_left),
                    "focus pane left",
                    KeybindAction::FocusPaneLeft,
                ),
                action_entry(
                    binding_label(&keybinds.focus_pane_down),
                    "focus pane down",
                    KeybindAction::FocusPaneDown,
                ),
                action_entry(
                    binding_label(&keybinds.focus_pane_up),
                    "focus pane up",
                    KeybindAction::FocusPaneUp,
                ),
                action_entry(
                    binding_label(&keybinds.focus_pane_right),
                    "focus pane right",
                    KeybindAction::FocusPaneRight,
                ),
                action_entry_kw(
                    binding_label(&keybinds.swap_pane_left),
                    "swap pane left",
                    KeybindAction::SwapPaneLeft,
                    &["move pane left", "swap pane with left"],
                ),
                action_entry_kw(
                    binding_label(&keybinds.swap_pane_down),
                    "swap pane down",
                    KeybindAction::SwapPaneDown,
                    &["move pane down", "swap pane with below"],
                ),
                action_entry_kw(
                    binding_label(&keybinds.swap_pane_up),
                    "swap pane up",
                    KeybindAction::SwapPaneUp,
                    &["move pane up", "swap pane with above"],
                ),
                action_entry_kw(
                    binding_label(&keybinds.swap_pane_right),
                    "swap pane right",
                    KeybindAction::SwapPaneRight,
                    &["move pane right", "swap pane with right"],
                ),
                action_entry(
                    binding_label(&keybinds.cycle_pane_next),
                    "cycle pane next",
                    KeybindAction::CyclePaneNext,
                ),
                action_entry(
                    binding_label(&keybinds.cycle_pane_previous),
                    "cycle pane previous",
                    KeybindAction::CyclePanePrevious,
                ),
                action_entry(
                    binding_label(&keybinds.last_pane),
                    "last pane",
                    KeybindAction::LastPane,
                ),
            ],
        ),
    ];

    if !keybinds.custom_commands.is_empty() {
        groups.push((
            "custom",
            keybinds
                .custom_commands
                .iter()
                .map(|binding| KeybindHelpEntry {
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

pub(crate) fn filter_keybind_help_groups(
    groups: Vec<KeybindHelpGroup>,
    query: &str,
) -> Vec<KeybindHelpGroup> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<KeybindHelpGroup> {
        vec![
            (
                "workspaces / tabs",
                vec![entry("w", "workspace navigation"), entry("c", "new tab")],
            ),
            (
                "panes",
                vec![entry("v", "split vertical"), entry("x", "close pane")],
            ),
        ]
    }

    #[test]
    fn filter_matches_labels_and_shortcuts_case_insensitively() {
        let filtered = filter_keybind_help_groups(groups(), "WoRk");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[0].label, "workspace navigation");

        let filtered = filter_keybind_help_groups(groups(), "x");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[0].label, "close pane");
        assert!(filter_keybind_help_groups(groups(), "panes").is_empty());
    }

    #[test]
    fn filter_also_matches_a_palette_keyword() {
        let entries = vec![action_entry_kw(
            "v",
            "split vertical",
            KeybindAction::SplitVertical,
            &["split right"],
        )];
        let filtered = filter_keybind_help_groups(vec![("panes", entries)], "split right");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[0].label, "split vertical");
    }

    #[test]
    fn plain_and_action_entries_carry_no_keywords_by_default() {
        assert!(entry("k", "some entry").keywords.is_empty());
        assert!(entry("k", "some entry").action.is_none());
        assert!(action_entry("k", "some action", KeybindAction::ClosePane)
            .keywords
            .is_empty());
    }

    #[test]
    fn every_help_action_has_a_stable_palette_id_except_the_palette_itself() {
        let keybinds = Keybinds::default();
        for entry in keybind_help_groups(&keybinds, (KeyCode::Char(' '), KeyModifiers::CONTROL))
            .into_iter()
            .flat_map(|(_, entries)| entries)
        {
            let Some(action) = entry.action else {
                continue;
            };
            assert!(
                action == KeybindAction::OpenCommandPalette || action.palette_id().is_some(),
                "help action {action:?} needs a stable palette id"
            );
        }
    }
}
