use super::*;

use crate::input::{KeybindAction, TerminalKey};

fn shell() -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.set_endpoint_methods(Some(
        crate::server::client_commands::supported_client_shell_method_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    ));
    state.compose(106, 24).expect("composed frame");
    state
}

fn press(state: &mut ClientShellState, key: KeyCode) -> ClientShellInput {
    press_with(state, key, KeyModifiers::empty())
}

fn press_with(
    state: &mut ClientShellState,
    key: KeyCode,
    modifiers: KeyModifiers,
) -> ClientShellInput {
    state.handle_raw_events(vec![RawInputEvent::Key(TerminalKey::new(key, modifiers))])
}

fn open_palette(state: &mut ClientShellState) -> ClientShellInput {
    let outcome = state.handle_raw_events(vec![RawInputEvent::Key(TerminalKey::new(
        KeyCode::Char('/'),
        KeyModifiers::empty(),
    ))]);
    state.compose(106, 24).expect("composed frame");
    outcome
}

fn enter_prefix(state: &mut ClientShellState) {
    press_with(state, KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert_eq!(state.mode, ClientShellMode::Prefix);
}

fn click(state: &mut ClientShellState, column: u16, row: u16) -> ClientShellInput {
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })])
}

fn endpoint_methods(outcome: &ClientShellInput) -> Vec<crate::api::schema::Method> {
    outcome
        .actions
        .iter()
        .filter_map(|action| match action {
            ClientShellAction::Endpoint { request, .. } => Some(request.method.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_configured_binding_opens_the_palette_and_asks_for_the_plugin_registry() {
    let mut state = shell();
    enter_prefix(&mut state);
    let opened = open_palette(&mut state);

    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::Palette(_))
    ));
    assert!(
        endpoint_methods(&opened)
            .iter()
            .any(|method| matches!(method, crate::api::schema::Method::PluginList(_))),
        "opening the palette should ask the endpoint for plugins"
    );
}

#[test]
fn typing_filters_and_enter_runs_the_highlighted_command() {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);

    for character in "zoom pane".chars() {
        press(&mut state, KeyCode::Char(character));
    }
    let names: Vec<String> = state
        .filtered_palette_commands()
        .into_iter()
        .map(|row| row.command.name.into_owned())
        .collect();
    assert_eq!(names.first().map(String::as_str), Some("zoom pane"));

    let ran = press(&mut state, KeyCode::Enter);
    assert!(
        state.overlay.is_none(),
        "running a command closes the palette"
    );
    assert!(
        endpoint_methods(&ran)
            .iter()
            .any(|method| matches!(method, crate::api::schema::Method::PaneZoom(_))),
        "the palette should dispatch the same request the keybind would"
    );
    assert_eq!(
        state.recent_command_ids.first().map(String::as_str),
        Some("core:zoom-pane"),
        "a run command is remembered"
    );
}

#[test]
fn a_remembered_command_leads_the_next_empty_palette() {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);
    for character in "last pane".chars() {
        press(&mut state, KeyCode::Char(character));
    }
    press(&mut state, KeyCode::Enter);

    enter_prefix(&mut state);
    open_palette(&mut state);
    let ids: Vec<String> = state
        .filtered_palette_commands()
        .into_iter()
        .map(|row| row.command.id)
        .collect();
    assert_eq!(ids.first().map(String::as_str), Some("core:last-pane"));
}

#[test]
fn a_query_matching_nothing_leaves_the_palette_open_on_enter() {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);
    for character in "zzzznotacommand".chars() {
        press(&mut state, KeyCode::Char(character));
    }
    assert!(state.filtered_palette_commands().is_empty());

    let ran = press(&mut state, KeyCode::Enter);
    assert!(
        matches!(state.overlay, Some(ClientShellOverlay::Palette(_))),
        "an empty result must not read as a dismissal"
    );
    assert!(endpoint_methods(&ran).is_empty());
}

#[test]
fn escape_closes_the_palette_without_running_anything() {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);
    let closed = press(&mut state, KeyCode::Esc);
    assert!(state.overlay.is_none());
    assert!(endpoint_methods(&closed).is_empty());
}

#[test]
fn clicking_a_palette_row_runs_it_and_clicking_outside_closes_the_palette() {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);
    let (rect, index) = state.hits.palette_rows[0];
    let expected = state
        .filtered_palette_commands()
        .into_iter()
        .nth(index)
        .expect("a first row")
        .command
        .id;

    let ran = click(&mut state, rect.x + 2, rect.y);
    assert!(state.overlay.is_none());
    assert!(!endpoint_methods(&ran).is_empty() || !ran.actions.is_empty());
    assert_eq!(state.recent_command_ids.first(), Some(&expected));

    enter_prefix(&mut state);
    open_palette(&mut state);
    click(&mut state, 0, 0);
    assert!(
        state.overlay.is_none(),
        "a click outside closes the palette"
    );
}

// The button sits inside the popup, so it satisfies neither the row hit nor
// the click-outside test. Before it was wired, a click on it was swallowed and
// the palette just sat there — a drawn affordance that did nothing.
#[test]
fn clicking_the_rendered_esc_close_button_closes_the_palette() {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);
    let close = state.hits.overlay_cancel;
    assert!(
        !close.is_empty(),
        "the renderer must publish the close button rect"
    );
    assert!(
        super::super::contains(state.hits.palette_popup, (close.x, close.y)),
        "the button is inside the popup, which is what makes this reachable \
         only through overlay_cancel"
    );

    let closed = click(&mut state, close.x + 1, close.y);
    assert!(state.overlay.is_none());
    assert!(endpoint_methods(&closed).is_empty(), "closing runs nothing");
}

#[test]
fn move_pane_to_space_arms_the_navigator_and_a_workspace_row_moves_the_pane() {
    let mut state = shell();
    state.open_navigator_overlay_for_move(Some("pane_1".into()), None);
    state.compose(106, 24).expect("composed frame");
    let Some(ClientShellOverlay::Navigator(navigator)) = state.overlay.as_ref() else {
        panic!("navigator should be open");
    };
    assert!(navigator.move_armed());

    let rows = render::client_navigator_rows(
        &state.endpoints,
        &state.active_endpoint_id,
        match state.overlay.as_ref() {
            Some(ClientShellOverlay::Navigator(navigator)) => navigator,
            _ => unreachable!(),
        },
    );
    assert!(
        matches!(rows[0].target, ClientNavigatorTarget::NewWorkspace),
        "an armed navigator offers a new-space destination"
    );
    let workspace_row = rows
        .iter()
        .position(|row| matches!(row.target, ClientNavigatorTarget::Workspace { .. }))
        .expect("a workspace row");
    if let Some(ClientShellOverlay::Navigator(navigator)) = state.overlay.as_mut() {
        navigator.selected = Some(rows[workspace_row].target.clone());
    }

    let mut outcome = ClientShellInput::default();
    state.accept_navigator_selection(&mut outcome);
    assert!(state.overlay.is_none());
    assert!(
        endpoint_methods(&outcome).iter().any(|method| matches!(
            method,
            crate::api::schema::Method::PaneMove(params)
                if matches!(
                    params.destination,
                    crate::api::schema::PaneMoveDestination::NewTab { .. }
                )
        )),
        "a workspace destination gives the pane a new tab there"
    );
}

#[test]
fn a_pane_row_destination_asks_which_way_the_pane_splits() {
    let mut state = shell();
    state.open_navigator_overlay_for_move(Some("pane_1".into()), None);
    state.compose(106, 24).expect("composed frame");
    let rows = render::client_navigator_rows(
        &state.endpoints,
        &state.active_endpoint_id,
        match state.overlay.as_ref() {
            Some(ClientShellOverlay::Navigator(navigator)) => navigator,
            _ => unreachable!(),
        },
    );
    let pane_row = rows
        .iter()
        .position(|row| matches!(row.target, ClientNavigatorTarget::Pane { .. }))
        .expect("a pane row");
    if let Some(ClientShellOverlay::Navigator(navigator)) = state.overlay.as_mut() {
        navigator.selected = Some(rows[pane_row].target.clone());
    }

    let mut outcome = ClientShellInput::default();
    state.accept_navigator_selection(&mut outcome);
    assert!(
        matches!(state.overlay, Some(ClientShellOverlay::Chooser(_))),
        "a tab destination is a split, so the direction is asked rather than guessed"
    );
    assert!(endpoint_methods(&outcome).is_empty(), "nothing moves yet");

    state.compose(106, 24).expect("composed frame");
    let confirmed = press(&mut state, KeyCode::Char('h'));
    assert!(state.overlay.is_none());
    assert!(
        endpoint_methods(&confirmed).iter().any(|method| matches!(
            method,
            crate::api::schema::Method::PaneMove(params)
                if matches!(
                    &params.destination,
                    crate::api::schema::PaneMoveDestination::Tab { split, .. }
                        if *split == crate::api::schema::SplitDirection::Down
                )
        )),
        "h picks a horizontal split"
    );
}

#[test]
fn an_armed_tab_move_lands_the_whole_tab_without_asking_for_a_direction() {
    let mut state = shell();
    state.open_navigator_overlay_for_move(None, Some("tab_1".into()));
    state.compose(106, 24).expect("composed frame");
    if let Some(ClientShellOverlay::Navigator(navigator)) = state.overlay.as_mut() {
        navigator.selected = Some(ClientNavigatorTarget::NewWorkspace);
    }

    let mut outcome = ClientShellInput::default();
    state.accept_navigator_selection(&mut outcome);
    assert!(state.overlay.is_none());
    assert!(
        endpoint_methods(&outcome).iter().any(|method| matches!(
            method,
            crate::api::schema::Method::TabMoveToDestination(params)
                if matches!(
                    params.destination,
                    crate::api::schema::TabMoveDestination::NewWorkspace { .. }
                )
        )),
        "the new-space row moves the tab into a workspace made for it"
    );
}

/// A chooser with more than two choices, carrying palette outcomes rather
/// than pane splits — the shape every direction family uses. Cycling must
/// wrap in both directions and enter must run the choice the cycle landed on,
/// not the one that happened to be first.
#[test]
fn a_four_way_chooser_cycles_and_runs_the_selected_outcome() {
    let mut state = shell();
    let choices = vec![
        chooser_choice(" left ", KeybindAction::SwapPaneLeft, "core:swap-pane-left"),
        chooser_choice(" down ", KeybindAction::SwapPaneDown, "core:swap-pane-down"),
        chooser_choice(" up ", KeybindAction::SwapPaneUp, "core:swap-pane-up"),
        chooser_choice(
            " right ",
            KeybindAction::SwapPaneRight,
            "core:swap-pane-right",
        ),
    ];
    state.open_chooser_overlay("swap pane".into(), choices, None);
    state.compose(106, 24).expect("composed frame");

    press(&mut state, KeyCode::Right);
    press(&mut state, KeyCode::Right);
    assert_eq!(chooser_selection(&state), Some(2), "right advances");
    press(&mut state, KeyCode::Left);
    assert_eq!(chooser_selection(&state), Some(1), "left goes back");
    press(&mut state, KeyCode::Left);
    press(&mut state, KeyCode::Left);
    assert_eq!(chooser_selection(&state), Some(3), "left wraps past zero");

    let ran = press(&mut state, KeyCode::Enter);
    assert!(state.overlay.is_none(), "running closes the chooser");
    assert!(
        endpoint_methods(&ran).iter().any(|method| matches!(
            method,
            crate::api::schema::Method::PaneSwap(params)
                if params.direction == Some(crate::api::schema::PaneDirection::Right)
        )),
        "enter runs the choice the cycle landed on"
    );
    assert_eq!(
        state.recent_command_ids.first().map(String::as_str),
        Some("core:swap-pane-right"),
        "a palette outcome records the leaf row it ran"
    );
}

/// A chooser opened from the palette owes the operator their query back when
/// they change their mind — otherwise cancelling costs them the whole search.
#[test]
fn cancel_from_a_palette_spawned_chooser_restores_the_query() {
    let mut state = shell();
    let choices = vec![chooser_choice(
        " left ",
        KeybindAction::SwapPaneLeft,
        "core:swap-pane-left",
    )];
    state.open_chooser_overlay(
        "swap pane".into(),
        choices,
        Some(PaletteReturn {
            query: "swap".into(),
            selected: 2,
        }),
    );
    state.compose(106, 24).expect("composed frame");

    press(&mut state, KeyCode::Esc);
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::Palette(palette)) => {
            assert_eq!(palette.query, "swap", "the query comes back");
            assert_eq!(palette.selected, 2, "and the row they were on");
        }
        other => panic!("esc should reopen the palette, got {other:?}"),
    }
}

/// Four buttons do not fit on one row of a narrow terminal. The chooser must
/// stack them rather than return nothing, because a family row that opens an empty
/// overlay is a dead end with no other way to reach its leaves.
#[test]
fn a_chooser_too_wide_for_one_row_stacks_its_buttons() {
    let labels = [" left ", " down ", " up ", " right "];
    let (_, _, wide) =
        super::super::palette::chooser_geometry(Rect::new(0, 0, 120, 40), "swap pane", &labels)
            .expect("wide");
    assert!(
        wide.windows(2).all(|pair| pair[0].y == pair[1].y),
        "a wide terminal keeps one row: {wide:?}"
    );

    let (_, _, narrow) =
        super::super::palette::chooser_geometry(Rect::new(0, 0, 34, 24), "swap pane", &labels)
            .expect("narrow");
    assert_eq!(narrow.len(), labels.len(), "every choice keeps a rect");
    assert!(
        narrow.windows(2).all(|pair| pair[1].y > pair[0].y),
        "a narrow terminal stacks: {narrow:?}"
    );
    assert!(
        narrow
            .windows(2)
            .all(|pair| pair[0].x == pair[1].x && pair[0].width == pair[1].width),
        "stacked buttons share a column: {narrow:?}"
    );
}

/// The whole point of collapsing the leaf rows: the family row asks, and
/// backing out of the question returns the operator to the search they typed.
#[test]
fn enter_on_a_family_row_opens_the_chooser_with_a_return() {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);
    for character in "swap pane".chars() {
        press(&mut state, KeyCode::Char(character));
    }
    let names: Vec<String> = state
        .filtered_palette_commands()
        .into_iter()
        .map(|row| row.command.name.into_owned())
        .collect();
    assert_eq!(
        names.first().map(String::as_str),
        Some("swap pane..."),
        "got {names:?}"
    );

    let opened = press(&mut state, KeyCode::Enter);
    assert!(
        endpoint_methods(&opened).is_empty(),
        "opening the question runs nothing"
    );
    assert!(
        state.recent_command_ids.is_empty(),
        "a family row is a question, not a command to remember"
    );
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::Chooser(chooser)) => {
            assert_eq!(chooser.title, "swap pane");
            assert_eq!(chooser.choices.len(), 4, "one button per direction");
            assert!(chooser.return_to.is_some(), "esc must have somewhere to go");
        }
        other => panic!("enter should open the chooser, got {other:?}"),
    }

    state.compose(106, 24).expect("composed frame");
    let ran = press(&mut state, KeyCode::Enter);
    assert!(
        endpoint_methods(&ran)
            .iter()
            .any(|method| matches!(method, crate::api::schema::Method::PaneSwap(_))),
        "the chosen direction runs the leaf action"
    );
    assert_eq!(
        state.recent_command_ids.first().map(String::as_str),
        Some("core:swap-pane-left"),
        "history records the leaf that actually ran"
    );
}

/// A palette holding one destructive plugin action, reached by a query that
/// names it.
fn shell_with_a_destructive_plugin_row() -> ClientShellState {
    shell_with_a_destructive_plugin_row_titled("Uninstall web bridge (remove service)")
}

/// The same palette, with the plugin choosing the action title. The title is
/// manifest text, so a test that varies it is testing untrusted input.
fn shell_with_a_destructive_plugin_row_titled(title: &str) -> ClientShellState {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);
    let action = crate::api::schema::PluginManifestAction {
        id: "uninstall".into(),
        title: title.into(),
        description: None,
        contexts: Vec::new(),
        platforms: None,
        destructive: true,
        command: vec!["true".into()],
    };
    let plugin = crate::api::schema::InstalledPluginInfo {
        plugin_id: "collie".into(),
        name: "Collie".into(),
        version: "1.0.0".into(),
        min_herdr_version: String::new(),
        description: None,
        manifest_path: "/tmp/collie/herdr-plugin.toml".into(),
        plugin_root: "/tmp/collie".into(),
        enabled: true,
        platforms: None,
        build: Vec::new(),
        startup: Vec::new(),
        actions: vec![action],
        events: Vec::new(),
        panes: Vec::new(),
        link_handlers: Vec::new(),
        source: Default::default(),
        warnings: Vec::new(),
    };
    assert!(state.receive_palette_plugins(vec![plugin], None));
    for character in "uninstall".chars() {
        press(&mut state, KeyCode::Char(character));
    }
    state.compose(106, 24).expect("composed frame");
    state
}

#[test]
fn enter_on_a_destructive_row_confirms_before_running() {
    let mut state = shell_with_a_destructive_plugin_row();
    let names: Vec<String> = state
        .filtered_palette_commands()
        .into_iter()
        .map(|row| row.command.name.into_owned())
        .collect();
    assert_eq!(
        names.first().map(String::as_str),
        Some("Collie — Uninstall web bridge (remove service)"),
        "got {names:?}"
    );

    let asked = press(&mut state, KeyCode::Enter);
    assert!(
        endpoint_methods(&asked)
            .iter()
            .all(|method| !matches!(method, crate::api::schema::Method::PluginActionInvoke(_))),
        "nothing runs before the confirm is answered"
    );
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::Chooser(chooser)) => {
            assert_eq!(
                chooser.title, "Collie — Uninstall web bridge (remove service)",
                "the confirm names the whole row, not a generic warning"
            );
            assert_eq!(chooser.choices.len(), 2);
        }
        other => panic!("enter should confirm, got {other:?}"),
    }
}

#[test]
fn a_destructive_confirm_defaults_to_cancel() {
    let mut state = shell_with_a_destructive_plugin_row();
    press(&mut state, KeyCode::Enter);
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::Chooser(chooser)) => {
            assert_eq!(chooser.selected, 0, "the default choice is the first");
            assert_eq!(
                chooser.choices.first().map(|choice| &choice.outcome),
                Some(&ChooserOutcome::Cancel),
                "and the first choice backs out"
            );
        }
        other => panic!("enter should confirm, got {other:?}"),
    }
}

#[test]
fn a_cancelled_destructive_row_is_not_remembered() {
    let mut state = shell_with_a_destructive_plugin_row();
    press(&mut state, KeyCode::Enter);
    state.compose(106, 24).expect("composed frame");

    // The reflex second enter takes the default, which is cancel.
    let cancelled = press(&mut state, KeyCode::Enter);
    assert!(
        endpoint_methods(&cancelled)
            .iter()
            .all(|method| !matches!(method, crate::api::schema::Method::PluginActionInvoke(_))),
        "cancel runs nothing"
    );
    assert!(
        !state
            .recent_command_ids
            .iter()
            .any(|id| id == "plugin-action:collie.uninstall"),
        "a cancelled row must not lead the next empty palette"
    );
    assert!(
        matches!(state.overlay, Some(ClientShellOverlay::Palette(_))),
        "cancel returns the operator to their search"
    );
}

#[test]
fn run_anyway_runs_the_destructive_action_and_remembers_it() {
    let mut state = shell_with_a_destructive_plugin_row();
    press(&mut state, KeyCode::Enter);
    state.compose(106, 24).expect("composed frame");
    press(&mut state, KeyCode::Right);

    let ran = press(&mut state, KeyCode::Enter);
    assert!(
        endpoint_methods(&ran).iter().any(|method| matches!(
            method,
            crate::api::schema::Method::PluginActionInvoke(params)
                if params.action_id == "uninstall"
        )),
        "run anyway invokes the action"
    );
    assert_eq!(
        state.recent_command_ids.first().map(String::as_str),
        Some("plugin-action:collie.uninstall"),
        "a run command is remembered like any other"
    );
}

/// Reads the drawn text of a palette row back off the composed buffer, so a
/// test can assert on what the operator actually sees rather than the model
/// behind it.
fn palette_row_text(state: &mut ClientShellState, command_id: &str) -> String {
    let index = state
        .filtered_palette_commands()
        .into_iter()
        .position(|row| row.command.id == command_id)
        .unwrap_or_else(|| panic!("command {command_id} should be in the filtered list"));
    let frame = state.compose(106, 24).expect("composed frame");
    let rect = state
        .hits
        .palette_rows
        .iter()
        .find(|(_, row_index)| *row_index == index)
        .map(|(rect, _)| *rect)
        .unwrap_or_else(|| {
            panic!("row {index} for {command_id} should be visible in the viewport")
        });
    let buffer = frame.to_ratatui_buffer().expect("buffer reconstructs");
    (rect.x..rect.right())
        .map(|x| buffer[(x, rect.y)].symbol().to_string())
        .collect()
}

// `merge workspace into...` is unbound by default — see
// `merge_workspace_command_reaches_the_palette` — so a query that hits it by
// name, with no keyword involved, is the case with neither a key nor a
// match reason to show. It must still read as a dash, not a hole.
#[test]
fn an_unbound_row_shows_a_dash_not_a_hole() {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);
    for character in "merge workspace into".chars() {
        press(&mut state, KeyCode::Char(character));
    }
    let row = state
        .filtered_palette_commands()
        .into_iter()
        .next()
        .expect("a matching row");
    assert!(
        row.command.key.is_none(),
        "merge workspace into... is unbound by default, got {:?}",
        row.command.key
    );
    assert!(
        row.matched_keyword.is_none(),
        "the query names the row directly, got keyword {:?}",
        row.matched_keyword
    );
    let command_id = row.command.id.clone();
    let text = palette_row_text(&mut state, &command_id);
    assert!(
        text.contains('—'),
        "an unbound, non-keyword row should show a dash, got {text:?}"
    );
    assert!(
        !text.contains("matched:"),
        "no keyword matched, so no match reason should print, got {text:?}"
    );
}

// "combine" only reaches `merge workspace into...` through its keyword
// vocabulary (`merge_workspace_command_reaches_the_palette` pins that
// keyword) — so the row's right column has nothing but the match reason to
// show.
#[test]
fn a_keyword_only_row_shows_its_match_reason() {
    let mut state = shell();
    enter_prefix(&mut state);
    open_palette(&mut state);
    for character in "combine".chars() {
        press(&mut state, KeyCode::Char(character));
    }
    let row = state
        .filtered_palette_commands()
        .into_iter()
        .next()
        .expect("a matching row");
    assert_eq!(
        row.command.name, "merge workspace into...",
        "got {}",
        row.command.name
    );
    let keyword = row
        .matched_keyword
        .expect("combine should only match through a keyword");
    let command_id = row.command.id.clone();
    let text = palette_row_text(&mut state, &command_id);
    assert!(
        text.contains("matched:") && text.contains(keyword),
        "expected the match reason {keyword:?} in the row, got {text:?}"
    );
}

// A plugin action carries no keybind (plugin rows never set `key`) and this
// one is reached by a query that hits its own name — no keyword involved
// either — so it has nothing but the dash to show, same as a core row.
#[test]
fn a_plugin_row_with_no_key_and_no_keyword_shows_a_dash() {
    let mut state = shell_with_a_destructive_plugin_row();
    let row = state
        .filtered_palette_commands()
        .into_iter()
        .next()
        .expect("a matching row");
    assert!(
        row.command.key.is_none(),
        "plugin rows carry no keybind, got {:?}",
        row.command.key
    );
    assert!(
        row.matched_keyword.is_none(),
        "the query names the row directly, got keyword {:?}",
        row.matched_keyword
    );
    let command_id = row.command.id.clone();
    let text = palette_row_text(&mut state, &command_id);
    assert!(
        text.contains('—'),
        "a plugin row with neither a key nor a match reason should show a dash, got {text:?}"
    );
}

// The plugin picks the action title, and the tag is drawn after it. A title
// wide enough to fill the row left no width for the tag at all, so the one
// in-list warning disappeared on exactly the name a plugin would choose to
// hide it. The tag's width is reserved before the name is drawn.
#[test]
fn a_row_name_too_wide_for_the_row_cannot_crowd_out_the_destructive_tag() {
    let mut state = shell_with_a_destructive_plugin_row_titled(
        "Uninstall the entirely harmless and routinely reversible web bridge \
         companion service together with every one of its cached artifacts",
    );
    let row = state
        .filtered_palette_commands()
        .into_iter()
        .next()
        .expect("a matching row");
    assert!(row.command.destructive, "the fixture marks the row itself");
    let command_id = row.command.id.clone();
    let text = palette_row_text(&mut state, &command_id);
    assert!(
        text.contains("[destructive]"),
        "a long plugin-chosen name must not drop the tag, got {text:?}"
    );
}

/// An out-of-range index is a bug in a caller, not an operator action — but it
/// must not cost the operator their palette. Every other exit from
/// `run_chooser_choice` restores the palette; this one used to drop the overlay
/// and take the query with it.
#[test]
fn an_out_of_range_chooser_index_restores_the_palette_rather_than_closing_it() {
    let mut state = shell();
    let choices = vec![chooser_choice(
        " left ",
        KeybindAction::SwapPaneLeft,
        "core:swap-pane-left",
    )];
    state.open_chooser_overlay(
        "swap pane".into(),
        choices,
        Some(PaletteReturn {
            query: "swap".into(),
            selected: 1,
        }),
    );
    state.compose(106, 24).expect("composed frame");

    let mut outcome = ClientShellInput::default();
    state.run_chooser_choice(9, &mut outcome);
    // The reopened palette re-asks for the plugin registry, so the only
    // request allowed here is that list.
    assert!(
        endpoint_methods(&outcome)
            .iter()
            .all(|method| matches!(method, crate::api::schema::Method::PluginList(_))),
        "an index that names no button runs nothing"
    );
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::Palette(palette)) => {
            assert_eq!(palette.query, "swap", "the query survives");
            assert_eq!(palette.selected, 1);
        }
        other => panic!("the palette should come back, got {other:?}"),
    }
}

fn chooser_choice(
    label: &'static str,
    action: KeybindAction,
    command_id: &str,
) -> super::super::state::ChooserChoice {
    super::super::state::ChooserChoice {
        label,
        outcome: super::super::state::ChooserOutcome::Palette {
            action: super::super::palette::PaletteAction::Keybind(action),
            command_id: command_id.to_owned(),
        },
    }
}

fn chooser_selection(state: &ClientShellState) -> Option<usize> {
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::Chooser(chooser)) => Some(chooser.selected),
        _ => None,
    }
}

#[test]
fn the_chooser_captures_every_mouse_event_aimed_past_it() {
    let mut state = shell();
    state.open_pane_split_direction_overlay("pane_1".into(), "tab_1".into(), None);
    state.compose(106, 24).expect("composed frame");
    let scroll_before = state.workspace_scroll;

    // A right-click and a scroll outside the popup must not reach the sidebar
    // or open a pane context menu underneath.
    state.handle_raw_events(vec![
        RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 5,
            row: 5,
            modifiers: KeyModifiers::empty(),
        }),
        RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::empty(),
        }),
    ]);
    assert!(
        matches!(state.overlay, Some(ClientShellOverlay::Chooser(_))),
        "the picker stays open"
    );
    assert_eq!(state.workspace_scroll, scroll_before);

    // A left click outside the buttons cancels rather than moving the pane.
    let cancelled = click(&mut state, 0, 0);
    assert!(state.overlay.is_none());
    assert!(endpoint_methods(&cancelled).is_empty());
}

#[test]
fn the_chooser_buttons_are_hit_where_the_renderer_drew_them() {
    let mut state = shell();
    state.open_pane_split_direction_overlay("pane_1".into(), "tab_1".into(), None);
    state.compose(106, 24).expect("composed frame");
    let vertical = state
        .hits
        .chooser_buttons
        .first()
        .copied()
        .unwrap_or_default();
    assert!(!vertical.is_empty(), "the renderer publishes a button rect");

    let moved = click(&mut state, vertical.x + 1, vertical.y);
    assert!(state.overlay.is_none());
    assert!(
        endpoint_methods(&moved).iter().any(|method| matches!(
            method,
            crate::api::schema::Method::PaneMove(params)
                if matches!(
                    &params.destination,
                    crate::api::schema::PaneMoveDestination::Tab { split, .. }
                        if *split == crate::api::schema::SplitDirection::Right
                )
        )),
        "clicking vertical splits right"
    );
}

/// Two workspaces, so a merge has somewhere to land.
fn shell_with_second_workspace() -> ClientShellState {
    let mut snapshot = snapshot();
    let mut second = snapshot.workspaces[0].clone();
    second.workspace_id = "ws_2".into();
    second.number = 2;
    second.label = "second".into();
    second.focused = false;
    second.active_tab_id = "tab_2".into();
    snapshot.workspaces.push(second);
    let mut tab = snapshot.tabs[0].clone();
    tab.tab_id = "tab_2".into();
    tab.workspace_id = "ws_2".into();
    tab.number = 1;
    tab.focused = false;
    snapshot.tabs.push(tab);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state.set_endpoint_methods(Some(
        crate::server::client_commands::supported_client_shell_method_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    ));
    state.compose(106, 24).expect("composed frame");
    state
}

fn arm_merge_on_the_second_workspace(state: &mut ClientShellState) {
    state.open_navigator_overlay_for_merge("ws_1".into());
    state.compose(106, 24).expect("composed frame");
    let rows = render::client_navigator_rows(
        &state.endpoints,
        &state.active_endpoint_id,
        match state.overlay.as_ref() {
            Some(ClientShellOverlay::Navigator(navigator)) => navigator,
            _ => panic!("navigator should be open"),
        },
    );
    let target_row = rows
        .iter()
        .position(|row| matches!(&row.target, ClientNavigatorTarget::Workspace { workspace_id: id, .. } if id == "ws_2"))
        .expect("a row for the other workspace");
    if let Some(ClientShellOverlay::Navigator(navigator)) = state.overlay.as_mut() {
        navigator.selected = Some(rows[target_row].target.clone());
    }
}

#[test]
fn the_merge_action_arms_the_navigator_and_a_workspace_row_asks_first() {
    let mut state = shell_with_second_workspace();
    let mut outcome = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::MergeWorkspace),
        &mut outcome,
    );
    let Some(ClientShellOverlay::Navigator(navigator)) = state.overlay.as_ref() else {
        panic!("merge should open the navigator as a destination picker");
    };
    assert_eq!(navigator.pending_workspace_merge.as_deref(), Some("ws_1"));
    // Merging into a workspace created for the merge is a rename, not a merge,
    // so the destination-only new-space row stays off.
    assert!(!navigator.move_armed());

    arm_merge_on_the_second_workspace(&mut state);
    let mut accept = ClientShellInput::default();
    state.accept_navigator_selection(&mut accept);
    assert!(
        matches!(state.overlay, Some(ClientShellOverlay::ConfirmMerge(_))),
        "a merge destination is confirmed before anything moves"
    );
    assert!(
        endpoint_methods(&accept).is_empty(),
        "picking the destination must not merge on its own"
    );
}

#[test]
fn picking_the_source_row_leaves_the_merge_armed() {
    let mut state = shell_with_second_workspace();
    state.open_navigator_overlay_for_merge("ws_1".into());
    state.compose(106, 24).expect("composed frame");
    let rows = render::client_navigator_rows(
        &state.endpoints,
        &state.active_endpoint_id,
        match state.overlay.as_ref() {
            Some(ClientShellOverlay::Navigator(navigator)) => navigator,
            _ => panic!("navigator should be open"),
        },
    );
    let own_row = rows
        .iter()
        .position(|row| matches!(&row.target, ClientNavigatorTarget::Workspace { workspace_id: id, .. } if id == "ws_1"))
        .expect("a row for the armed workspace");
    if let Some(ClientShellOverlay::Navigator(navigator)) = state.overlay.as_mut() {
        navigator.selected = Some(rows[own_row].target.clone());
    }

    let mut outcome = ClientShellInput::default();
    state.accept_navigator_selection(&mut outcome);
    assert!(
        matches!(state.overlay, Some(ClientShellOverlay::Navigator(_))),
        "a workspace cannot merge into itself, so the picker stays armed"
    );
    assert!(endpoint_methods(&outcome).is_empty());
}

#[test]
fn confirming_the_merge_sends_workspace_merge_for_the_picked_target() {
    let mut state = shell_with_second_workspace();
    arm_merge_on_the_second_workspace(&mut state);
    let mut accept = ClientShellInput::default();
    state.accept_navigator_selection(&mut accept);

    let confirmed = press(&mut state, KeyCode::Enter);
    assert!(state.overlay.is_none());
    assert!(
        endpoint_methods(&confirmed).iter().any(|method| matches!(
            method,
            crate::api::schema::Method::WorkspaceMerge(params)
                if params.source_workspace_id == "ws_1"
                    && params.target_workspace_id == "ws_2"
                    // No worktree group here, so the client asks for no group
                    // intent. The server refuses on its own if it disagrees.
                    && !params.merge_group
        )),
        "confirming merges the armed source into the picked target"
    );
}

#[test]
fn the_merge_confirmation_captures_every_mouse_event_aimed_past_it() {
    let mut state = shell_with_second_workspace();
    arm_merge_on_the_second_workspace(&mut state);
    let mut accept = ClientShellInput::default();
    state.accept_navigator_selection(&mut accept);
    state.compose(106, 24).expect("composed frame");
    let scroll_before = state.workspace_scroll;

    state.handle_raw_events(vec![
        RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 5,
            row: 5,
            modifiers: KeyModifiers::empty(),
        }),
        RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::empty(),
        }),
    ]);
    assert!(
        matches!(state.overlay, Some(ClientShellOverlay::ConfirmMerge(_))),
        "the confirmation stays open"
    );
    assert_eq!(state.workspace_scroll, scroll_before);

    let cancelled = click(&mut state, 0, 0);
    assert!(state.overlay.is_none());
    assert!(
        endpoint_methods(&cancelled).is_empty(),
        "cancelling merges nothing"
    );
}

#[test]
fn the_merge_confirm_button_is_hit_where_the_renderer_drew_it() {
    let mut state = shell_with_second_workspace();
    arm_merge_on_the_second_workspace(&mut state);
    let mut accept = ClientShellInput::default();
    state.accept_navigator_selection(&mut accept);
    state.compose(106, 24).expect("composed frame");
    let confirm = state.hits.overlay_primary;
    assert!(!confirm.is_empty(), "the renderer publishes a button rect");

    let merged = click(&mut state, confirm.x + 1, confirm.y);
    assert!(state.overlay.is_none());
    assert!(
        endpoint_methods(&merged).iter().any(|method| matches!(
            method,
            crate::api::schema::Method::WorkspaceMerge(params)
                if params.source_workspace_id == "ws_1"
                    && params.target_workspace_id == "ws_2"
        )),
        "the mouse path merges exactly what the key path does"
    );
}

#[test]
fn destination_moves_are_not_sent_to_a_server_that_only_supports_reordering() {
    let mut state = shell();
    state.set_endpoint_methods(Some(vec!["tab.move".into()]));
    let mut outcome = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::MoveTabToNewSpace),
        &mut outcome,
    );
    assert!(endpoint_methods(&outcome).is_empty());
    let notice = state
        .visible_endpoint_notice
        .as_ref()
        .expect("unsupported action notice");
    assert!(notice.body.contains("tab.move_to_destination"));
    assert!(
        state.supports_endpoint_method(&crate::api::schema::Method::TabMove(
            crate::api::schema::TabMoveParams {
                tab_id: "tab_1".into(),
                insert_index: Some(0),
                destination: None,
            }
        ))
    );
}
