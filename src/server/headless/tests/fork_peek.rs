use super::*;
use crossterm::event::{KeyCode, KeyModifiers};

fn send_public(server: &mut HeadlessServer, method: api::schema::Method) -> String {
    let (respond_to, response) = std::sync::mpsc::channel();
    server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
        request: api::schema::Request {
            id: "peek".into(),
            method,
        },
        respond_to,
        response_write_complete: None,
    });
    response.recv().unwrap()
}

#[tokio::test]
async fn peek_shows_another_tab_pane_at_popup_size_then_gives_it_back() {
    let mut server = test_headless_server();
    let mut workspace = crate::workspace::Workspace::test_new("peek");
    let worker_tab = workspace.test_add_tab(Some("worker"));
    server.app.state.workspaces = vec![workspace];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    server.app.state.mode = crate::app::Mode::Terminal;
    for tab in [0, worker_tab] {
        let pane = server.app.state.workspaces[0].tabs[tab].root_pane;
        let terminal_id = server.app.state.workspaces[0]
            .terminal_id(pane)
            .unwrap()
            .clone();
        server.app.terminal_runtimes.insert(
            terminal_id,
            crate::terminal::TerminalRuntime::test_with_channel(80, 23).0,
        );
    }
    let worker = server.app.state.workspaces[0].tabs[worker_tab].root_pane;
    let worker_id = server.app.public_pane_id(0, worker).unwrap();
    let worker_terminal = server.app.state.workspaces[0]
        .terminal_id(worker)
        .unwrap()
        .clone();
    let worker_size = |server: &HeadlessServer| {
        server
            .app
            .terminal_runtimes
            .get(&worker_terminal)
            .unwrap()
            .current_size()
    };

    let (control, render) = connect_test_shell(&mut server, 41, 100, 30);
    control.recv().unwrap();
    server.claim_shell_tab_geometry(41, false);
    let layout_size = worker_size(&server);

    let response = send_public(
        &mut server,
        api::schema::Method::PanePeek(api::schema::PaneTarget {
            pane_id: worker_id.clone(),
        }),
    );
    serde_json::from_str::<api::schema::SuccessResponse>(&response).expect(&response);
    assert_eq!(
        server.popup_owner_tab_id,
        server.app.public_tab_id(0, 0),
        "a peek opens over the tab the operator is on"
    );
    let geometry = crate::popup_size::resolve_popup_geometry(
        Some(crate::popup_size::PopupSize::Percent(80)),
        Some(crate::popup_size::PopupSize::Percent(80)),
        Rect::new(0, 0, 100, 30),
    )
    .unwrap();
    let peek_size = worker_size(&server);
    assert_ne!(peek_size, layout_size);
    assert!(peek_size.0 <= geometry.inner.height && peek_size.1 <= geometry.inner.width);

    // The tab layout pass must not take the pane back while it is peeked.
    server.claim_shell_tab_geometry(41, false);
    server.reapply_controlled_shell_tab_geometry(false);
    crate::ui::resize_tab_surface(
        &server.app.state,
        &server.app.terminal_runtimes,
        0,
        worker_tab,
        Rect::new(0, 0, 100, 30),
        crate::kitty_graphics::HostCellSize::default(),
    );
    assert_eq!(worker_size(&server), peek_size);
    assert_eq!(
        server.app.state.workspaces[0].find_tab_index_for_pane(worker),
        Some(worker_tab)
    );

    while render.try_recv().is_ok() {}
    server.render_and_stream();
    let surface = recv_pane_surface(&render, "peek surface");
    let popup = surface.popup.expect("peek renders as a popup");
    assert_eq!(popup.terminal_id, worker_terminal.to_string());
    assert!(popup.title.starts_with("peek: "), "{}", popup.title);

    let response = send_public(
        &mut server,
        api::schema::Method::PanePeekClose(api::schema::EmptyParams::default()),
    );
    serde_json::from_str::<api::schema::SuccessResponse>(&response).expect(&response);
    assert!(server.app.state.popup_pane.is_none());
    assert_eq!(worker_size(&server), layout_size);
    assert!(server.app.terminal_runtimes.get(&worker_terminal).is_some());
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn moving_the_peeked_pane_under_its_overlay_closes_the_peek() {
    let mut server = test_headless_server();
    let mut workspace = crate::workspace::Workspace::test_new("peek");
    let worker_tab = workspace.test_add_tab(Some("worker"));
    workspace.test_add_tab(Some("third"));
    server.app.state.workspaces = vec![workspace];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    server.app.state.mode = crate::app::Mode::Terminal;
    for tab in 0..3 {
        let pane = server.app.state.workspaces[0].tabs[tab].root_pane;
        let terminal_id = server.app.state.workspaces[0]
            .terminal_id(pane)
            .unwrap()
            .clone();
        server.app.terminal_runtimes.insert(
            terminal_id,
            crate::terminal::TerminalRuntime::test_with_channel(80, 23).0,
        );
    }
    let worker = server.app.state.workspaces[0].tabs[worker_tab].root_pane;
    let worker_id = server.app.public_pane_id(0, worker).unwrap();
    let owner_tab = server.app.public_tab_id(0, 0).unwrap();
    let (control, _render) = connect_test_shell(&mut server, 43, 100, 30);
    control.recv().unwrap();
    server.claim_shell_tab_geometry(43, false);
    send_public(
        &mut server,
        api::schema::Method::PanePeek(api::schema::PaneTarget {
            pane_id: worker_id.clone(),
        }),
    );
    assert!(server.app.state.popup_pane.is_some());

    let response = send_public(
        &mut server,
        api::schema::Method::PaneMove(api::schema::PaneMoveParams {
            pane_id: worker_id,
            destination: api::schema::PaneMoveDestination::Tab {
                tab_id: owner_tab,
                target_pane_id: None,
                split: api::schema::SplitDirection::Right,
                ratio: None,
            },
            focus: false,
        }),
    );
    serde_json::from_str::<api::schema::SuccessResponse>(&response).expect(&response);

    assert!(server.app.state.popup_pane.is_none());
    server.app.state.assert_invariants_for_test();
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn prefix_key_closes_a_peek_and_never_reaches_the_pane() {
    let mut server = test_headless_server();
    let mut workspace = crate::workspace::Workspace::test_new("peek");
    let worker_tab = workspace.test_add_tab(Some("worker"));
    server.app.state.workspaces = vec![workspace];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    server.app.state.mode = crate::app::Mode::Terminal;
    let worker = server.app.state.workspaces[0].tabs[worker_tab].root_pane;
    let worker_terminal = server.app.state.workspaces[0]
        .terminal_id(worker)
        .unwrap()
        .clone();
    let (runtime, mut worker_input) = crate::terminal::TerminalRuntime::test_with_channel(80, 23);
    server
        .app
        .terminal_runtimes
        .insert(worker_terminal.clone(), runtime);
    let worker_id = server.app.public_pane_id(0, worker).unwrap();
    let (control, _render) = connect_test_shell(&mut server, 42, 100, 30);
    control.recv().unwrap();
    server.claim_shell_tab_geometry(42, false);
    let response = send_public(
        &mut server,
        api::schema::Method::PanePeek(api::schema::PaneTarget { pane_id: worker_id }),
    );
    serde_json::from_str::<api::schema::SuccessResponse>(&response).expect(&response);
    let key = |code, modifiers| {
        crate::protocol::ClientPaneInputEvent::from_terminal_key(crate::input::TerminalKey::new(
            code, modifiers,
        ))
        .unwrap()
    };

    assert!(
        !server.handle_server_event(ServerEvent::ClientShellPopupInput {
            client_id: 42,
            terminal_id: worker_terminal.to_string(),
            events: vec![crate::protocol::ClientPaneInputEvent::TextCommit(
                "y".into()
            )],
        })
    );
    assert_eq!(worker_input.try_recv().unwrap(), Bytes::from_static(b"y"));

    assert!(
        server.handle_server_event(ServerEvent::ClientShellPopupInput {
            client_id: 42,
            terminal_id: worker_terminal.to_string(),
            events: vec![
                key(KeyCode::Char('b'), KeyModifiers::CONTROL),
                crate::protocol::ClientPaneInputEvent::TextCommit("after".into()),
            ],
        })
    );
    assert!(server.app.state.popup_pane.is_none());
    assert!(
        worker_input.try_recv().is_err(),
        "the prefix and what follows stay out of the pane"
    );
    assert!(server.app.terminal_runtimes.get(&worker_terminal).is_some());
    shutdown_test_runtimes(&mut server);
}
