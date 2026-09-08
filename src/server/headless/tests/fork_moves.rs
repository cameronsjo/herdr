use super::*;

#[tokio::test]
async fn structural_moves_reconcile_every_client_and_remove_stale_geometry_controllers() {
    use crate::api::schema::{
        Method, Request, TabMoveDestination, TabMoveParams, TabMoveToDestinationParams,
        WorkspaceMergeParams,
    };
    for public in [false, true] {
        for operation in 0..3 {
            let mut server = test_headless_server();
            let mut source = crate::workspace::Workspace::test_new("source");
            source.test_add_tab(Some("remaining"));
            let target = crate::workspace::Workspace::test_new("target");
            server.app.state.workspaces = vec![source, target];
            server.app.state.ensure_test_terminals();
            server.app.state.active = Some(0);
            server.app.state.selected = 0;
            server.app.state.mode = crate::app::Mode::Terminal;
            let old_tab = server.app.public_tab_id(0, 0).unwrap();
            let source_id = server.app.public_workspace_id(0);
            let target_id = server.app.public_workspace_id(1);
            let (first_control, _first_render) = connect_test_shell(&mut server, 71, 100, 30);
            let (second_control, _second_render) = connect_test_shell(&mut server, 72, 70, 20);
            first_control.recv().unwrap();
            second_control.recv().unwrap();
            assert!(server.focus_shell_client_on_tab(71, &old_tab));
            assert!(server.focus_shell_client_on_tab(72, &old_tab));
            server.claim_shell_tab_geometry(72, false);
            assert!(server.tab_geometry_controllers.contains_key(&old_tab));
            let destination = TabMoveDestination::Workspace {
                workspace_id: target_id.clone(),
                insert_index: None,
            };
            let method = match operation {
                0 => Method::TabMoveToDestination(TabMoveToDestinationParams {
                    tab_id: old_tab.clone(),
                    destination,
                }),
                1 => Method::TabMove(TabMoveParams {
                    tab_id: old_tab.clone(),
                    insert_index: None,
                    destination: Some(destination),
                }),
                _ => Method::WorkspaceMerge(WorkspaceMergeParams {
                    source_workspace_id: source_id,
                    target_workspace_id: target_id,
                    merge_group: false,
                }),
            };
            let (respond_to, response) = std::sync::mpsc::channel();
            let message = crate::api::ApiRequestMessage {
                request: Request {
                    id: "move".into(),
                    method,
                },
                respond_to,
                response_write_complete: None,
                stream_active: None,
            };
            if public {
                server.handle_api_request_with_shutdown_check(message);
            } else {
                server.handle_client_shell_api_request(71, message);
            }
            let result: crate::api::schema::SuccessResponse =
                serde_json::from_str(&response.recv().unwrap()).unwrap();
            assert!(matches!(
                result.result,
                crate::api::schema::ResponseResult::TabMove { .. }
                    | crate::api::schema::ResponseResult::WorkspaceInfo { .. }
            ));
            assert!(server.app.parse_tab_id(&old_tab).is_none());
            assert!(!server.tab_geometry_controllers.contains_key(&old_tab));
            for id in [71, 72] {
                let location = server.clients[&id].shell_location.as_ref().unwrap();
                let focused = location
                    .focused_tab_id()
                    .expect("a live fallback or relocated tab");
                assert!(
                    server.app.parse_tab_id(focused).is_some(),
                    "operation {operation}, public {public}: {focused}"
                );
                assert!(location
                    .active_tab_ids
                    .values()
                    .all(|tab| server.app.parse_tab_id(tab).is_some()));
            }
            server.app.state.assert_invariants_for_test();
        }
    }
}
