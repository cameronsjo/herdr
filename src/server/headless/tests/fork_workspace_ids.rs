//! Fork regression tests: an out-of-range positional workspace id (`N`, `w_N`)
//! is a `workspace_not_found` error on every request path, never a panic.
//!
//! Stock herdr panics the whole server on these ids. The fork bounds
//! `parse_workspace_id` (refs #77); these tests pin the request paths that
//! reached a bare index through it and had no test of their own.

use super::*;

const OUT_OF_RANGE: &[&str] = &[
    "3",
    "w_3",
    "999999",
    "w_999999",
    "18446744073709551615",
    "w_18446744073709551615",
];

fn two_workspace_server() -> HeadlessServer {
    let mut server = test_headless_server();
    server.app.state.workspaces = vec![
        crate::workspace::Workspace::test_new("first"),
        crate::workspace::Workspace::test_new("second"),
    ];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    server.app.state.mode = crate::app::Mode::Terminal;
    server
}

fn bad_id_requests(workspace_id: &str, pane_id: &str) -> Vec<crate::api::schema::Method> {
    use crate::api::schema::{
        Method, PaneMoveDestination, PaneMoveParams, WorkspaceTarget, WorktreeListParams,
        WorktreeOpenParams,
    };
    vec![
        Method::WorkspaceFocus(WorkspaceTarget {
            workspace_id: workspace_id.into(),
        }),
        Method::WorktreeList(WorktreeListParams {
            workspace_id: Some(workspace_id.into()),
            cwd: None,
            trust_repository: false,
        }),
        Method::WorktreeOpen(WorktreeOpenParams {
            workspace_id: Some(workspace_id.into()),
            cwd: None,
            path: None,
            branch: Some("probe".into()),
            label: None,
            focus: false,
            trust_repository: false,
        }),
        Method::PaneMove(PaneMoveParams {
            pane_id: pane_id.into(),
            destination: PaneMoveDestination::NewTab {
                workspace_id: Some(workspace_id.into()),
                label: None,
            },
            focus: false,
        }),
    ]
}

fn send(
    server: &mut HeadlessServer,
    client_id: Option<u64>,
    method: crate::api::schema::Method,
) -> serde_json::Value {
    let (respond_to, response) = std::sync::mpsc::channel();
    let message = crate::api::ApiRequestMessage {
        request: crate::api::schema::Request {
            id: "bad-workspace-id".into(),
            method,
        },
        respond_to,
        response_write_complete: None,
    };
    match client_id {
        Some(client_id) => {
            server.handle_client_shell_api_request(client_id, message);
        }
        None => {
            server.handle_api_request_with_shutdown_check(message);
        }
    }
    let raw = response
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("a response");
    serde_json::from_str(&raw).expect("a JSON response")
}

#[tokio::test]
async fn out_of_range_positional_workspace_ids_are_not_found_on_every_path() {
    let mut server = two_workspace_server();
    let pane_id = server
        .app
        .public_pane_id(0, server.app.state.workspaces[0].tabs[0].root_pane)
        .expect("first pane id");
    let first_tab = server.app.public_tab_id(0, 0).expect("first tab id");
    let (control, _render) = connect_test_shell(&mut server, 81, 100, 30);
    control.recv().expect("first snapshot");
    assert!(server.focus_shell_client_on_tab(81, &first_tab));

    for &workspace_id in OUT_OF_RANGE {
        for client_id in [None, Some(81)] {
            for method in bad_id_requests(workspace_id, &pane_id) {
                let label = format!("{method:?} via {client_id:?}");
                let response = send(&mut server, client_id, method);
                assert_eq!(
                    response
                        .pointer("/error/code")
                        .and_then(|code| code.as_str()),
                    Some("workspace_not_found"),
                    "{label}: {response}"
                );
            }
        }
    }

    assert_eq!(server.app.state.workspaces.len(), 2);
    assert_eq!(
        server.app.state.active,
        Some(0),
        "a rejected focus moved the app"
    );
    for workspace in &server.app.state.workspaces {
        assert_eq!(workspace.tabs.len(), 1, "a rejected request added a tab");
    }
    let location = server.clients[&81].shell_location.as_ref().unwrap();
    assert_eq!(location.focused_tab_id(), Some(first_tab.as_str()));
    server.app.state.assert_invariants_for_test();
    shutdown_test_runtimes(&mut server);
}

/// The positive control: the bound must not reject the in-range positional
/// ids it exists to keep.
#[tokio::test]
async fn in_range_positional_workspace_ids_still_focus_on_every_path() {
    use crate::api::schema::{Method, WorkspaceTarget};

    let mut server = two_workspace_server();
    let tabs = [
        server.app.public_tab_id(0, 0).expect("first tab id"),
        server.app.public_tab_id(1, 0).expect("second tab id"),
    ];
    let full_ids = [
        server.app.public_workspace_id(0),
        server.app.public_workspace_id(1),
    ];
    let (control, _render) = connect_test_shell(&mut server, 82, 100, 30);
    control.recv().expect("first snapshot");
    let focused = |server: &HeadlessServer| {
        server.clients[&82]
            .shell_location
            .as_ref()
            .and_then(|location| location.focused_tab_id().map(str::to_owned))
    };

    for (workspace_id, expected) in [("2", 1), ("w_1", 0), ("w_2", 1), ("1", 0)] {
        for client_id in [None, Some(82)] {
            let label = format!("{workspace_id} via {client_id:?}");
            // Start every request on the other workspace, by full id over the
            // public path, so a pass needs this request to move the client.
            let reset = send(
                &mut server,
                None,
                Method::WorkspaceFocus(WorkspaceTarget {
                    workspace_id: full_ids[1 - expected].clone(),
                }),
            );
            assert!(reset.get("error").is_none(), "{label} reset: {reset}");
            assert_eq!(
                focused(&server).as_deref(),
                Some(tabs[1 - expected].as_str())
            );
            assert_eq!(server.app.state.active, Some(1 - expected), "{label} reset");

            let response = send(
                &mut server,
                client_id,
                Method::WorkspaceFocus(WorkspaceTarget {
                    workspace_id: workspace_id.into(),
                }),
            );
            assert!(response.get("error").is_none(), "{label}: {response}");
            assert_eq!(
                focused(&server).as_deref(),
                Some(tabs[expected].as_str()),
                "{label}"
            );
            if client_id.is_none() {
                assert_eq!(server.app.state.active, Some(expected), "{label}");
            }
        }
    }

    server.app.state.assert_invariants_for_test();
    shutdown_test_runtimes(&mut server);
}
