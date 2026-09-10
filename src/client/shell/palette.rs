//! The command palette's command model: what rows exist, how a query ranks
//! them, and what running a row does.
//!
//! Rows come from the keybind help entries (`src/input/keybind_help.rs`) plus
//! the plugin actions and panes the endpoint reports. There is no separate
//! palette registry: an action becomes searchable by gaining a help row that
//! carries a `KeybindAction`.

mod input;

use std::borrow::Cow;
use std::collections::HashMap;

use ratatui::layout::Rect;

use crate::api::schema::{InstalledPluginInfo, PluginActionContext, PluginPlatform};
use crate::config::LiveKeybindConfig;
use crate::input::KeybindAction;
use crate::protocol::ClientShellSnapshot;

const EMPTY_PALETTE_LIMIT: usize = 12;

const PALETTE_MODAL_SIZE: (u16, u16) = (76, 22);
const CHOOSER_MIN_MODAL_WIDTH: u16 = 44;
const CHOOSER_MODAL_HEIGHT: u16 = 6;
/// Borders, title and footer — what a stacked chooser costs on top of one row
/// per choice.
const CHOOSER_MODAL_CHROME_HEIGHT: u16 = 5;
const CHOOSER_BUTTON_GAP: u16 = 2;
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

/// The chooser's popup, panel-inner and one rect per button, shared between
/// the renderer and the mouse hit-test for the same reason.
///
/// Buttons sit on one row while that row fits, and stack into a column when it
/// does not. Stacking rather than refusing matters: a family row whose chooser
/// returns nothing is a dead end, because collapsing the leaf rows took away
/// the only other way to reach them.
pub(super) fn chooser_geometry(area: Rect, labels: &[&str]) -> Option<(Rect, Rect, Vec<Rect>)> {
    if labels.is_empty() {
        return None;
    }
    let widths: Vec<u16> = labels
        .iter()
        .map(|label| super::render::display_width(label))
        .collect();
    let count = labels.len() as u16;
    let row_width: u16 = widths
        .iter()
        .copied()
        .sum::<u16>()
        .saturating_add(CHOOSER_BUTTON_GAP.saturating_mul(count.saturating_sub(1)));
    let widest = widths.iter().copied().max().unwrap_or(0);

    let row_popup_width = CHOOSER_MIN_MODAL_WIDTH.max(row_width.saturating_add(4));
    if let Some(popup) = crate::ui::centered_popup_rect(area, row_popup_width, CHOOSER_MODAL_HEIGHT)
    {
        let inner = chooser_inner(popup);
        if inner.height >= 3 && inner.width >= row_width {
            let mut x = inner.x + (inner.width - row_width) / 2;
            let y = inner.y.saturating_add(1);
            let mut buttons = Vec::with_capacity(labels.len());
            for width in &widths {
                buttons.push(Rect::new(x, y, *width, 1));
                x = x.saturating_add(*width).saturating_add(CHOOSER_BUTTON_GAP);
            }
            return Some((popup, inner, buttons));
        }
    }

    let stacked_popup_width = CHOOSER_MIN_MODAL_WIDTH.max(widest.saturating_add(4));
    let stacked_popup_height = count.saturating_add(CHOOSER_MODAL_CHROME_HEIGHT);
    let popup = crate::ui::centered_popup_rect(area, stacked_popup_width, stacked_popup_height)?;
    let inner = chooser_inner(popup);
    if inner.width < widest || inner.height < count.saturating_add(2) {
        return None;
    }
    let x = inner.x + (inner.width - widest) / 2;
    let buttons = (0..count)
        .map(|row| Rect::new(x, inner.y.saturating_add(1 + row), widest, 1))
        .collect();
    Some((popup, inner, buttons))
}

fn chooser_inner(popup: Rect) -> Rect {
    Rect::new(
        popup.x.saturating_add(1),
        popup.y.saturating_add(1),
        popup.width.saturating_sub(2),
        popup.height.saturating_sub(2),
    )
}

/// The two labels the pane-split chooser offers, in the order its outcomes are
/// built — vertical first.
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
    /// Asks which direction before running one of the family's leaf actions.
    Chooser(DirectionFamily),
}

/// A set of actions that differ only by direction. Each is one palette row
/// that asks, rather than four that wall the list — nine directional rows
/// answering `move` was what the palette looked like before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectionFamily {
    MoveTab,
    MoveWorkspace,
    SwapPane,
}

/// The direction words a query can name to reach a family's leaf rows. One
/// character is enough, so `move tab l` reaches `move tab left`.
const DIRECTION_WORDS: [&str; 4] = ["left", "right", "up", "down"];

impl DirectionFamily {
    /// Derived from the leaf action rather than declared alongside it: one
    /// source, so a family and its leaves cannot drift apart.
    fn of(action: KeybindAction) -> Option<Self> {
        match action {
            KeybindAction::MoveTabPrevious | KeybindAction::MoveTabNext => Some(Self::MoveTab),
            KeybindAction::MoveWorkspacePrevious | KeybindAction::MoveWorkspaceNext => {
                Some(Self::MoveWorkspace)
            }
            KeybindAction::SwapPaneLeft
            | KeybindAction::SwapPaneDown
            | KeybindAction::SwapPaneUp
            | KeybindAction::SwapPaneRight => Some(Self::SwapPane),
            _ => None,
        }
    }

    /// The row label, which ends in an ellipsis for the same reason `merge
    /// workspace into...` does: it asks you something next.
    fn row_name(&self) -> &'static str {
        match self {
            Self::MoveTab => "move tab...",
            Self::MoveWorkspace => "move workspace...",
            Self::SwapPane => "swap pane...",
        }
    }

    /// The chooser's title — the row name without the ellipsis, since the
    /// chooser is the question the ellipsis promised.
    pub(super) fn chooser_title(&self) -> &'static str {
        self.row_name().trim_end_matches('.')
    }

    fn keywords(&self) -> &'static [&'static str] {
        match self {
            Self::MoveTab => &["reorder tab", "tab left", "tab right"],
            Self::MoveWorkspace => &["reorder workspace", "workspace up", "workspace down"],
            Self::SwapPane => &["move pane", "reorder pane"],
        }
    }

    fn id(&self) -> &'static str {
        match self {
            Self::MoveTab => "core:move-tab-family",
            Self::MoveWorkspace => "core:move-workspace-family",
            Self::SwapPane => "core:swap-pane-family",
        }
    }

    fn leaves(&self) -> &'static [(&'static str, KeybindAction)] {
        match self {
            Self::MoveTab => &[
                (" left ", KeybindAction::MoveTabPrevious),
                (" right ", KeybindAction::MoveTabNext),
            ],
            Self::MoveWorkspace => &[
                (" up ", KeybindAction::MoveWorkspacePrevious),
                (" down ", KeybindAction::MoveWorkspaceNext),
            ],
            Self::SwapPane => &[
                (" left ", KeybindAction::SwapPaneLeft),
                (" down ", KeybindAction::SwapPaneDown),
                (" up ", KeybindAction::SwapPaneUp),
                (" right ", KeybindAction::SwapPaneRight),
            ],
        }
    }

    /// One chooser button per leaf, each running exactly what the leaf row
    /// would have run — including recording that leaf in palette history, so
    /// reaching an action through the chooser and through its own row leave
    /// the same trace.
    pub(super) fn choices(&self) -> Vec<super::state::ChooserChoice> {
        self.leaves()
            .iter()
            .filter_map(|(label, action)| {
                Some(super::state::ChooserChoice {
                    label,
                    outcome: super::state::ChooserOutcome::Palette {
                        action: PaletteAction::Keybind(*action),
                        command_id: action.palette_id()?.to_string(),
                    },
                })
            })
            .collect()
    }

    fn all() -> [Self; 3] {
        [Self::MoveTab, Self::MoveWorkspace, Self::SwapPane]
    }
}

pub(crate) struct PaletteCommand {
    pub id: String,
    pub name: Cow<'static, str>,
    pub key: String,
    pub action: PaletteAction,
    pub keywords: &'static [&'static str],
    /// Set on a leaf row of a direction family. Such a row is hidden unless
    /// the query names a direction, because its family row answers for it.
    pub family: Option<DirectionFamily>,
    /// Running this removes or replaces something. The row carries a tag and
    /// Enter asks before it runs.
    pub destructive: bool,
}

/// The suffix a destructive row wears in the palette list.
pub(super) const DESTRUCTIVE_TAG: &str = " [destructive]";

/// A command that matched, with why it matched. The keyword is what the
/// renderer shows when a row has no key of its own — a hit with no visible
/// reason reads as the palette guessing.
pub(crate) struct PaletteRow {
    pub command: PaletteCommand,
    pub matched_keyword: Option<&'static str>,
}

struct PluginPaletteCommand {
    command: PaletteCommand,
    kind: &'static str,
}

/// What the endpoint reported on the palette's `plugin.list` call. Empty
/// until the response arrives, and empty forever against an endpoint that
/// does not support the method — the palette's core rows do not depend on it.
#[derive(Debug, Default)]
pub(crate) struct PalettePlugins {
    pub installed: Vec<InstalledPluginInfo>,
    /// The operator's own `[palette] destructive_actions` list, carried
    /// alongside the endpoint's report because a row is destructive when
    /// either source says so — and the manifest is the side we do not
    /// control.
    pub destructive_actions: Vec<String>,
    /// The platform the answering server runs on. `None` against a server too
    /// old to report it, which means "do not filter" — see
    /// [`platform_supported`].
    pub host_platform: Option<PluginPlatform>,
}

/// Whether the server would run this action or pane, given the platform it
/// reported on `plugin.list`.
///
/// The deciding platform is the *server's*, never the client's: `herdr
/// --remote` puts the two on different machines, and the commands run where
/// the server is. Filtering on the client's own OS is wrong in both
/// directions, and only one of them is recoverable — too permissive and the
/// server's own check refuses the invoke with an error the operator sees, but
/// too restrictive and the row is hidden, the invoke never sent, and a
/// runnable action has no other way to be reached.
///
/// `host_platform` is `None` against a server too old to report it. That case
/// must not filter, for the same reason: unknown resolves toward the
/// recoverable direction.
fn platform_supported(
    platforms: Option<&Vec<PluginPlatform>>,
    host_platform: Option<PluginPlatform>,
) -> bool {
    let Some(platforms) = platforms.filter(|platforms| !platforms.is_empty()) else {
        return true;
    };
    let Some(host) = host_platform else {
        return true;
    };
    platforms.contains(&host)
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

/// Builds the palette row label for a plugin action or pane.
///
/// Both inputs come from a plugin manifest, which the server accepts after only
/// a non-empty trim, and this label is drawn straight to the host terminal. A
/// pane process can install a manifest through `plugin.link`, so the strings are
/// untrusted; under `herdr --remote` they also cross machines. Filtering here
/// covers both callers — actions and panes — because this is the one place the
/// two strings become a drawn label.
fn plugin_command_name(plugin_name: &str, title: &str) -> String {
    let plugin_name = &crate::label::sanitize_label(plugin_name);
    let title = &crate::label::sanitize_label(title);
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

/// Whether the operator's override list names this action. Matched on the
/// exact `"<plugin_id>:<action_id>"` pair rather than either half, so marking
/// one action never silently marks a sibling.
fn action_is_marked_destructive(
    destructive_actions: &[String],
    plugin_id: &str,
    action_id: &str,
) -> bool {
    destructive_actions
        .iter()
        .any(|marked| match marked.split_once(':') {
            Some((marked_plugin, marked_action)) => {
                marked_plugin == plugin_id && marked_action == action_id
            }
            None => false,
        })
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
    plugins: &PalettePlugins,
    snapshot: &ClientShellSnapshot,
) -> Vec<PaletteCommand> {
    let host_platform = plugins.host_platform;
    let mut plugin_commands: Vec<PluginPaletteCommand> = Vec::new();
    let mut enabled: Vec<&InstalledPluginInfo> = plugins
        .installed
        .iter()
        .filter(|plugin| plugin.enabled)
        .collect();
    enabled.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));

    for plugin in &enabled {
        let mut actions: Vec<_> = plugin
            .actions
            .iter()
            .filter(|action| {
                platform_supported(
                    action.platforms.as_ref().or(plugin.platforms.as_ref()),
                    host_platform,
                )
            })
            .filter(|action| action_context_applies(&action.contexts, snapshot))
            .collect();
        actions.sort_by(|left, right| left.id.cmp(&right.id));
        for action in actions {
            let destructive = action.destructive
                || action_is_marked_destructive(
                    &plugins.destructive_actions,
                    &plugin.plugin_id,
                    &action.id,
                );
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
                    family: None,
                    destructive,
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
                platform_supported(
                    pane.platforms.as_ref().or(plugin.platforms.as_ref()),
                    host_platform,
                )
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
                    family: None,
                    // Opening a pane shows something; it removes nothing.
                    destructive: false,
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
    plugins: &PalettePlugins,
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
                    family: DirectionFamily::of(action),
                    // Core actions are confirmed where they need it — closing
                    // a pane already has its own dialog.
                    destructive: false,
                })
            })
            .collect();
    commands.extend(DirectionFamily::all().into_iter().map(|family| {
        PaletteCommand {
            id: family.id().to_string(),
            name: Cow::Borrowed(family.row_name()),
            // A family row is a question, not a binding — its leaves keep
            // whatever keys they were bound to.
            key: String::new(),
            action: PaletteAction::Chooser(family),
            keywords: family.keywords(),
            family: None,
            destructive: false,
        }
    }));
    commands.extend(plugin_palette_commands(plugins, snapshot));
    commands
}

/// Ranking by match quality rather than list order keeps a query that exactly
/// names one command from being answered by a longer command containing it.
/// `MAX_NAME_RANK` is the worst (highest) rank this function returns —
/// `command_match_rank` derives its keyword-tier offset from it so the two
/// stay coupled structurally instead of by two files agreeing on a number.
const MAX_NAME_RANK: u8 = 2;

/// Matching stops at a word boundary. A query landing mid-word is deliberately
/// not a match: it is how `remove` inside "uninstall web bridge (remove
/// service)" answered a query for `move`, putting a destructive plugin action
/// in front of an operator who was reordering tabs.
///
/// The boundary is any non-alphanumeric character, not whitespace alone. That
/// keeps a multi-word query working mid-name (`new tab` still finds "move pane
/// to new tab") and keeps a word reachable through punctuation (`remove` still
/// finds "(remove service)"), while `move` inside `remove` stays out.
fn match_rank(name: &str, query: &str) -> Option<u8> {
    let name = name.to_lowercase();
    if name == query {
        Some(0)
    } else if name.starts_with(query) {
        Some(1)
    } else if word_starts(&name).any(|offset| name[offset..].starts_with(query)) {
        Some(MAX_NAME_RANK)
    } else {
        None
    }
}

/// Every byte offset in `name` that begins a word — index 0, and any character
/// whose predecessor is not alphanumeric.
fn word_starts(name: &str) -> impl Iterator<Item = usize> + '_ {
    let mut previous_was_alphanumeric = false;
    name.char_indices().filter_map(move |(offset, character)| {
        let boundary = !previous_was_alphanumeric;
        previous_was_alphanumeric = character.is_alphanumeric();
        boundary.then_some(offset)
    })
}

/// A keyword match (e.g. "split right" finding the "split vertical" command)
/// always ranks below every name match, so a command whose own name answers
/// the query is never outranked by a synonym on a different command. The
/// matched keyword comes back with the rank so the row can say why it is
/// there.
fn command_match_rank(command: &PaletteCommand, query: &str) -> Option<(u8, Option<&'static str>)> {
    if let Some(rank) = match_rank(&command.name, query) {
        return Some((rank, None));
    }
    command
        .keywords
        .iter()
        .filter_map(|keyword| match_rank(keyword, query).map(|rank| (rank, *keyword)))
        .min_by_key(|(rank, _)| *rank)
        .map(|(rank, keyword)| (rank + MAX_NAME_RANK + 1, Some(keyword)))
}

/// Whether the query asks for a specific direction, which is what releases a
/// family's leaf rows. Any whitespace token that prefixes a direction word
/// counts, so `move tab l` reaches `move tab left`.
fn query_names_a_direction(query: &str) -> bool {
    query.split_whitespace().any(|token| {
        DIRECTION_WORDS
            .iter()
            .any(|direction| direction.starts_with(token))
    })
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
    plugins: &PalettePlugins,
    snapshot: &ClientShellSnapshot,
) -> Vec<PaletteRow> {
    let query = query.trim().to_lowercase();
    let commands = palette_commands(keybinds, plugins, snapshot);
    if query.is_empty() {
        return compact_palette_commands(commands, recent_command_ids)
            .into_iter()
            .map(|command| PaletteRow {
                command,
                matched_keyword: None,
            })
            .collect();
    }

    let directional = query_names_a_direction(&query);
    let mut ranked: Vec<(u8, usize, PaletteRow)> = commands
        .into_iter()
        .filter(|command| directional || command.family.is_none())
        .enumerate()
        .filter_map(|(index, command)| {
            let (rank, matched_keyword) = command_match_rank(&command, &query)?;
            Some((
                rank,
                index,
                PaletteRow {
                    command,
                    matched_keyword,
                },
            ))
        })
        .collect();
    ranked.sort_by_key(|(rank, index, _)| (*rank, *index));
    ranked.into_iter().map(|(_, _, row)| row).collect()
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
        filtered_palette_commands(query, &[], &keybinds(), &no_plugins(), &snapshot)
            .into_iter()
            .map(|row| row.command.name.into_owned())
            .collect()
    }

    fn command(name: &'static str, keywords: &'static [&'static str]) -> PaletteCommand {
        PaletteCommand {
            id: format!("test:{name}"),
            name: Cow::Borrowed(name),
            key: String::new(),
            action: PaletteAction::Keybind(KeybindAction::ClosePane),
            keywords,
            family: None,
            destructive: false,
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

    // A mid-word hit is what put a destructive plugin action — "uninstall web
    // bridge (remove service)" — in front of a query for "move".
    #[test]
    fn a_mid_word_substring_does_not_match() {
        assert_eq!(
            command_match_rank(&command("remove service", &[]), "move"),
            None
        );
        assert_eq!(
            command_match_rank(&command("xxsplitxx", &[]), "split"),
            None
        );
        // The real row from the review, verbatim.
        assert_eq!(
            command_match_rank(
                &command("Collie — Uninstall web bridge (remove service)", &[]),
                "move"
            ),
            None
        );

        // A keyword can still carry a row whose own name never says "pane",
        // so this asserts on the name only where the name is what matched.
        let name_matched = command("close pane", &[]);
        assert_eq!(
            command_match_rank(&name_matched, "pane"),
            Some((MAX_NAME_RANK, None))
        );
    }

    // The boundary is punctuation-aware, not whitespace-only: three cases the
    // two rules disagree on, all of them real palette rows.
    #[test]
    fn a_word_boundary_match_survives_punctuation_and_multiple_words() {
        // A multi-word query mid-name still matches.
        assert_eq!(
            command_match_rank(&command("move pane to new tab", &[]), "new tab"),
            Some((MAX_NAME_RANK, None))
        );
        // A word reachable only through punctuation still matches.
        assert_eq!(
            command_match_rank(&command("uninstall bridge (remove service)", &[]), "remove"),
            Some((MAX_NAME_RANK, None))
        );
        // But its interior does not.
        assert_eq!(
            command_match_rank(&command("uninstall bridge (remove service)", &[]), "emove"),
            None
        );
    }

    #[test]
    fn every_palette_command_is_runnable() {
        let snapshot = super::super::tests::snapshot();
        assert!(!palette_commands(&keybinds(), &no_plugins(), &snapshot).is_empty());
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
        let commands =
            filtered_palette_commands("", &recent, &keybinds(), &no_plugins(), &snapshot);
        let ids: Vec<&str> = commands.iter().map(|row| row.command.id.as_str()).collect();

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

    // A help row reaches the palette only when it carries a KeybindAction and
    // the action carries a palette id, so an action missing either is silently
    // palette-invisible however bindable and dispatchable it is. Six rows
    // arrived that way in an upstream sync; the four pane swaps had a row but
    // no palette id, which this list would not have caught until it named them.
    #[test]
    fn reorder_and_resize_rows_reach_the_palette() {
        let snapshot = super::super::tests::snapshot();
        let actions: Vec<PaletteAction> = palette_commands(&keybinds(), &no_plugins(), &snapshot)
            .into_iter()
            .map(|command| command.action)
            .collect();

        for expected in [
            KeybindAction::MoveTabPrevious,
            KeybindAction::MoveTabNext,
            KeybindAction::MoveWorkspacePrevious,
            KeybindAction::MoveWorkspaceNext,
            KeybindAction::SwapPaneLeft,
            KeybindAction::SwapPaneDown,
            KeybindAction::SwapPaneUp,
            KeybindAction::SwapPaneRight,
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

        // The three family rows ride alongside their leaves: the leaves stay
        // runnable and searchable, the family row is what a directionless
        // query reaches.
        for family in DirectionFamily::all() {
            assert!(
                actions.contains(&PaletteAction::Chooser(family)),
                "{family:?} has no family row"
            );
        }
    }

    #[test]
    fn move_tab_to_space_commands_reach_the_palette() {
        let snapshot = super::super::tests::snapshot();
        let commands = palette_commands(&keybinds(), &no_plugins(), &snapshot);

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
    fn merge_workspace_command_reaches_the_palette() {
        let snapshot = super::super::tests::snapshot();
        let commands = palette_commands(&keybinds(), &no_plugins(), &snapshot);

        let merge = commands
            .iter()
            .find(|command| command.action == PaletteAction::Keybind(KeybindAction::MergeWorkspace))
            .expect("merge workspace is missing from the palette");
        // Unbound by default, so the palette is the only way to reach it.
        assert!(
            merge
                .keywords
                .iter()
                .any(|keyword| keyword.contains("combine")),
            "merge workspace should be findable by combine vocabulary"
        );
    }

    #[test]
    fn move_pane_left_still_reaches_the_swap_pane_left_leaf() {
        let matches = names("move pane left");
        assert!(
            matches.iter().any(|name| name == "swap pane left"),
            "naming a direction releases the leaf, got {matches:?}"
        );
    }

    #[test]
    fn reorder_workspace_ranks_the_workspace_family_row_first() {
        let matches = names("reorder workspace");
        assert_eq!(
            matches.first().map(String::as_str),
            Some("move workspace..."),
            "got {matches:?}"
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
        assert_eq!(
            command_match_rank(&cmd, "split right"),
            Some((MAX_NAME_RANK + 1, Some("Split Right")))
        );
    }

    #[test]
    fn a_keyword_only_match_reports_its_keyword() {
        let cmd = command("close pane", &["dismiss"]);
        assert_eq!(
            command_match_rank(&cmd, "dismiss"),
            Some((MAX_NAME_RANK + 1, Some("dismiss")))
        );
        assert_eq!(
            command_match_rank(&cmd, "close").map(|(_, keyword)| keyword),
            Some(None),
            "a name match has no keyword to report"
        );

        // And the reason survives the filter, which is the only path the
        // renderer sees it through.
        let snapshot = super::super::tests::snapshot();
        let rows = filtered_palette_commands("combine", &[], &keybinds(), &no_plugins(), &snapshot);
        let merge = rows
            .iter()
            .find(|row| row.command.name == "merge workspace into...")
            .expect("merge workspace matches 'combine' only by keyword");
        assert_eq!(
            merge
                .matched_keyword
                .map(|keyword| keyword.contains("combine")),
            Some(true),
            "got {:?}",
            merge.matched_keyword
        );
        let named = rows.iter().find(|row| row.command.name.contains("combine"));
        assert!(
            named.is_none_or(|row| row.matched_keyword.is_none()),
            "a row matched by its own name reports no keyword"
        );
    }

    #[test]
    fn the_best_matching_keyword_wins_when_several_keywords_match() {
        // The command name itself must not match "split" (or the test would
        // exercise the name branch instead of the keyword branch). First
        // keyword only word-prefix-matches; second is an exact match. The
        // overall keyword rank should reflect the best of the two, not list
        // order.
        let cmd = command("close pane", &["foo split bar", "split"]);
        assert_eq!(
            command_match_rank(&cmd, "split"),
            Some((MAX_NAME_RANK + 1, Some("split")))
        );
    }

    #[test]
    fn the_weakest_name_match_still_outranks_any_keyword_match() {
        // "zoom pane" matches "pane" only at its second word — the weakest
        // name tier — while "close tab" matches it by keyword alone.
        let name_match = command("zoom pane", &[]);
        let keyword_match = command("close tab", &["pane"]);
        let (name_rank, _) = command_match_rank(&name_match, "pane").expect("name should match");
        let (keyword_rank, _) =
            command_match_rank(&keyword_match, "pane").expect("keyword should match");
        assert_eq!(name_rank, MAX_NAME_RANK);
        assert!(
            name_rank < keyword_rank,
            "name_rank={name_rank} keyword_rank={keyword_rank}"
        );
    }

    #[test]
    fn empty_query_keyword_lookup_does_not_panic() {
        let cmd = command("split vertical", &["split right"]);
        assert_eq!(command_match_rank(&cmd, ""), Some((1, None)));
    }

    #[test]
    fn a_move_query_shows_one_row_per_family() {
        let matches = names("move");
        for family in ["move tab...", "move workspace..."] {
            assert_eq!(
                matches.iter().filter(|name| *name == family).count(),
                1,
                "exactly one {family} row, got {matches:?}"
            );
        }
        for leaf in [
            "move tab left",
            "move tab right",
            "move workspace up",
            "move workspace down",
            "swap pane left",
            "swap pane right",
        ] {
            assert!(
                !matches.iter().any(|name| name == leaf),
                "{leaf} should be behind its family row, got {matches:?}"
            );
        }
    }

    #[test]
    fn a_query_naming_a_direction_prefix_reaches_the_leaf_row() {
        let matches = names("move tab l");
        assert_eq!(
            matches.first().map(String::as_str),
            Some("move tab left"),
            "got {matches:?}"
        );
        // The full word works the same way; the prefix is a shortcut, not a
        // different path.
        assert_eq!(
            names("move tab left").first().map(String::as_str),
            Some("move tab left")
        );
    }

    #[test]
    fn move_pane_reaches_the_swap_family_row() {
        // The context menu and the sidebar say "move pane"; the API says
        // "swap". Without the keyword bridge the palette answers nothing to
        // the wording a user arrives with.
        let matches = names("move pane");
        assert!(
            matches.iter().any(|name| name == "swap pane..."),
            "got {matches:?}"
        );
    }

    #[test]
    fn a_family_row_offers_one_choice_per_leaf_carrying_that_leaf_s_command_id() {
        use super::super::state::ChooserOutcome;

        let choices = DirectionFamily::SwapPane.choices();
        let ids: Vec<&str> = choices
            .iter()
            .filter_map(|choice| match &choice.outcome {
                ChooserOutcome::Palette { command_id, .. } => Some(command_id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            ids,
            vec![
                "core:swap-pane-left",
                "core:swap-pane-down",
                "core:swap-pane-up",
                "core:swap-pane-right",
            ]
        );
        assert_eq!(DirectionFamily::SwapPane.chooser_title(), "swap pane");
    }

    #[test]
    fn every_family_leaf_reaches_the_palette_so_no_choice_is_dropped() {
        // `choices` drops a leaf whose action has no palette id, which would
        // silently make a direction unreachable now that the leaf rows are
        // hidden behind the family row.
        for family in DirectionFamily::all() {
            assert_eq!(
                family.choices().len(),
                family.leaves().len(),
                "{family:?} lost a leaf to a missing palette id"
            );
        }
    }

    // Chooser labels stay ASCII so a terminal cannot disagree with the
    // geometry about how wide a button is. The rect is shared with the mouse
    // hit-test, so a glyph that renders wider than measured moves the target
    // out from under the drawn button rather than failing anything.
    #[test]
    fn chooser_labels_are_ascii_so_every_terminal_renders_them_the_measured_width() {
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
    fn the_split_chooser_fits_its_two_buttons() {
        let (_, _, buttons) = chooser_geometry(
            Rect::new(0, 0, 120, 40),
            &[SPLIT_VERTICAL_LABEL, SPLIT_HORIZONTAL_LABEL],
        )
        .expect("geometry");
        let [vertical, horizontal] = buttons.as_slice() else {
            panic!("two buttons, got {buttons:?}");
        };
        assert_eq!(usize::from(vertical.width), SPLIT_VERTICAL_LABEL.len());
        assert_eq!(usize::from(horizontal.width), SPLIT_HORIZONTAL_LABEL.len());
        assert!(
            horizontal.x >= vertical.x + vertical.width,
            "buttons must not overlap: {vertical:?} {horizontal:?}"
        );
        assert_eq!(vertical.y, horizontal.y);
    }

    #[test]
    fn a_chooser_with_no_choices_has_no_geometry() {
        assert!(chooser_geometry(Rect::new(0, 0, 120, 40), &[]).is_none());
    }

    #[test]
    fn a_terminal_too_small_for_the_palette_yields_no_geometry() {
        assert!(palette_geometry(Rect::new(0, 0, 10, 4)).is_none());
        assert!(chooser_geometry(
            Rect::new(0, 0, 10, 4),
            &[SPLIT_VERTICAL_LABEL, SPLIT_HORIZONTAL_LABEL]
        )
        .is_none());
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
        assert!(plugin_palette_commands(&host_plugins(vec![plugin]), &snapshot).is_empty());
    }

    #[test]
    fn an_enabled_plugin_offers_its_actions_and_panes() {
        let snapshot = super::super::tests::snapshot();
        let commands = plugin_palette_commands(&host_plugins(vec![test_plugin()]), &snapshot);
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

    // The deciding platform is the server's. These three pin all of it: a
    // manifest matching the server is offered, one that does not is hidden,
    // and a server that never said stops the filter entirely — including when
    // the manifest names a platform the *client* happens to be running on,
    // which is the case a cfg!-based filter got wrong.
    #[test]
    fn a_manifest_platform_matching_the_server_is_offered() {
        let snapshot = super::super::tests::snapshot();
        let mut plugin = test_plugin();
        plugin.actions[0].platforms = Some(vec![PluginPlatform::Windows]);
        let plugins = PalettePlugins {
            installed: vec![plugin],
            host_platform: Some(PluginPlatform::Windows),
            destructive_actions: Vec::new(),
        };
        let commands = plugin_palette_commands(&plugins, &snapshot);
        assert!(commands
            .iter()
            .any(|command| command.id == "plugin-action:demo.build"));
    }

    #[test]
    fn a_manifest_platform_the_server_does_not_run_hides_the_row() {
        let snapshot = super::super::tests::snapshot();
        let mut plugin = test_plugin();
        plugin.actions[0].platforms = Some(vec![PluginPlatform::Windows]);
        let plugins = PalettePlugins {
            installed: vec![plugin],
            host_platform: Some(PluginPlatform::Linux),
            destructive_actions: Vec::new(),
        };
        let commands = plugin_palette_commands(&plugins, &snapshot);
        assert!(commands
            .iter()
            .all(|command| command.id != "plugin-action:demo.build"));
    }

    #[test]
    fn a_server_that_reports_no_platform_hides_nothing() {
        let snapshot = super::super::tests::snapshot();
        let mut plugin = test_plugin();
        // Every platform except the one this test binary runs on, so a
        // reintroduced cfg!-based filter would hide the row and fail here.
        plugin.actions[0].platforms = Some(
            vec![
                PluginPlatform::Linux,
                PluginPlatform::Macos,
                PluginPlatform::Windows,
            ]
            .into_iter()
            .filter(|platform| *platform != this_binarys_platform())
            .collect(),
        );
        let plugins = PalettePlugins {
            installed: vec![plugin],
            host_platform: None,
            destructive_actions: Vec::new(),
        };
        let commands = plugin_palette_commands(&plugins, &snapshot);
        assert!(
            commands
                .iter()
                .any(|command| command.id == "plugin-action:demo.build"),
            "an unreported server platform must not filter: hiding a runnable \
             action leaves no other way to reach it"
        );
    }

    fn this_binarys_platform() -> PluginPlatform {
        if cfg!(target_os = "linux") {
            PluginPlatform::Linux
        } else if cfg!(target_os = "macos") {
            PluginPlatform::Macos
        } else {
            PluginPlatform::Windows
        }
    }

    #[test]
    fn a_selection_context_action_is_never_offered() {
        let snapshot = super::super::tests::snapshot();
        let mut plugin = test_plugin();
        plugin.actions[0].contexts = vec![PluginActionContext::Selection];
        let commands = plugin_palette_commands(&host_plugins(vec![plugin]), &snapshot);
        assert!(commands
            .iter()
            .all(|command| command.id != "plugin-action:demo.build"));
    }

    fn no_plugins() -> PalettePlugins {
        PalettePlugins::default()
    }

    /// Plugins whose declared platforms always match, so a test that is not
    /// about platform filtering never trips it.
    fn host_plugins(installed: Vec<InstalledPluginInfo>) -> PalettePlugins {
        PalettePlugins {
            installed,
            host_platform: Some(this_binarys_platform()),
            destructive_actions: Vec::new(),
        }
    }

    /// The action `test_plugin` carries that removes something.
    fn uninstall_action() -> crate::api::schema::PluginManifestAction {
        crate::api::schema::PluginManifestAction {
            id: "uninstall".into(),
            title: "Uninstall web bridge (remove service)".into(),
            description: None,
            contexts: Vec::new(),
            platforms: None,
            destructive: false,
            command: vec!["true".into()],
        }
    }

    fn find_command<'a>(commands: &'a [PaletteCommand], id: &str) -> Option<&'a PaletteCommand> {
        commands.iter().find(|command| command.id == id)
    }

    #[test]
    fn a_destructive_manifest_action_carries_its_tag() {
        let snapshot = super::super::tests::snapshot();
        let mut plugin = test_plugin();
        let mut action = uninstall_action();
        action.destructive = true;
        plugin.actions.push(action);

        let commands = plugin_palette_commands(&host_plugins(vec![plugin]), &snapshot);
        let uninstall = find_command(&commands, "plugin-action:demo.uninstall")
            .expect("the uninstall row reaches the palette");
        assert!(uninstall.destructive, "the manifest said so");
        assert!(
            !find_command(&commands, "plugin-action:demo.build")
                .expect("build row")
                .destructive,
            "its sibling is untouched"
        );
    }

    #[test]
    fn a_config_override_marks_a_third_party_action_destructive() {
        let snapshot = super::super::tests::snapshot();
        let mut plugin = test_plugin();
        // The manifest does NOT set it — this is the third-party case the
        // override exists for.
        plugin.actions.push(uninstall_action());
        let plugins = PalettePlugins {
            installed: vec![plugin],
            host_platform: Some(this_binarys_platform()),
            destructive_actions: vec!["demo:uninstall".into()],
        };

        let commands = plugin_palette_commands(&plugins, &snapshot);
        assert!(
            find_command(&commands, "plugin-action:demo.uninstall")
                .expect("uninstall row")
                .destructive
        );
        assert!(
            !find_command(&commands, "plugin-action:demo.build")
                .expect("build row")
                .destructive,
            "marking one action must not mark its siblings"
        );
    }

    #[test]
    fn a_destructive_override_matches_the_whole_pair_not_either_half() {
        assert!(action_is_marked_destructive(
            &["demo:uninstall".to_string()],
            "demo",
            "uninstall"
        ));
        // Same action id under a different plugin.
        assert!(!action_is_marked_destructive(
            &["demo:uninstall".to_string()],
            "other",
            "uninstall"
        ));
        // Same plugin, different action.
        assert!(!action_is_marked_destructive(
            &["demo:uninstall".to_string()],
            "demo",
            "build"
        ));
        // A malformed entry marks nothing rather than everything.
        assert!(!action_is_marked_destructive(
            &["demo".to_string()],
            "demo",
            "uninstall"
        ));
    }

    #[test]
    fn an_old_manifest_without_the_field_parses_as_not_destructive() {
        let manifest: crate::api::schema::PluginManifestAction =
            serde_json::from_str(r#"{"id":"build","title":"Build","command":["true"]}"#)
                .expect("a manifest predating the field still parses");
        assert!(!manifest.destructive);
    }

    // Two plugins, because the sort is across plugins as well as within one:
    // a single-fixture test would pass on a comparator that only orders
    // actions inside their own plugin. Actions come before panes, plugin ids
    // ascending, then item ids ascending.
    #[test]
    fn rows_from_several_plugins_sort_deterministically() {
        let snapshot = super::super::tests::snapshot();
        let mut second = test_plugin();
        second.plugin_id = "alpha".into();
        second.name = "Alpha".into();
        second
            .actions
            .push(crate::api::schema::PluginManifestAction {
                destructive: false,
                id: "aaa-first".into(),
                title: "Aaa first".into(),
                description: None,
                contexts: Vec::new(),
                platforms: None,
                command: vec!["true".into()],
            });

        let commands =
            plugin_palette_commands(&host_plugins(vec![test_plugin(), second]), &snapshot);
        let ids: Vec<&str> = commands.iter().map(|command| command.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "plugin-action:alpha.aaa-first",
                "plugin-action:alpha.build",
                "plugin-action:demo.build",
                "plugin-pane:alpha.board",
                "plugin-pane:demo.board",
            ]
        );
    }

    #[test]
    fn the_sort_does_not_depend_on_the_order_the_endpoint_listed_plugins() {
        let snapshot = super::super::tests::snapshot();
        let mut second = test_plugin();
        second.plugin_id = "alpha".into();
        second.name = "Alpha".into();

        let forward = plugin_palette_commands(
            &host_plugins(vec![test_plugin(), second.clone()]),
            &snapshot,
        );
        let reversed =
            plugin_palette_commands(&host_plugins(vec![second, test_plugin()]), &snapshot);
        let ids = |commands: Vec<PaletteCommand>| {
            commands
                .into_iter()
                .map(|command| command.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(forward), ids(reversed));
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
                destructive: false,
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
