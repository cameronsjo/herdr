use super::*;

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
