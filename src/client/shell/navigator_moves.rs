//! Fork destination pickers reuse upstream navigator rows and endpoint identity.
use super::*;

pub(super) fn navigator_rows(
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    navigator: &ClientNavigatorOverlay,
) -> Vec<ClientNavigatorRow> {
    let picking = navigator.move_armed() || navigator.pending_workspace_merge.is_some();
    let mut rows =
        super::aggregate_navigation::navigator_rows(endpoints, active_endpoint_id, navigator);
    if !picking {
        return rows;
    }
    // Runtime moves are server-local. Identical IDs on another machine must
    // never become destinations for the active server's move request.
    let workspace_only =
        navigator.pending_tab_move.is_some() || navigator.pending_workspace_merge.is_some();
    rows.retain(|row| match &row.target {
        ClientNavigatorTarget::Workspace { endpoint_id, .. } => {
            endpoint_id == active_endpoint_id && !row.stale
        }
        ClientNavigatorTarget::Tab { endpoint_id, .. }
        | ClientNavigatorTarget::Pane { endpoint_id, .. } => {
            !workspace_only && endpoint_id == active_endpoint_id && !row.stale
        }
        _ => false,
    });
    if endpoints.len() > 1 {
        for row in &mut rows {
            row.depth = row.depth.saturating_sub(1);
        }
    }
    if navigator.move_armed() {
        rows.insert(
            0,
            ClientNavigatorRow {
                depth: 0,
                label: "new space".to_owned(),
                meta: "move into a space created for it".to_owned(),
                status: None,
                stale: false,
                current: false,
                target: ClientNavigatorTarget::NewWorkspace,
            },
        );
    }
    rows
}
