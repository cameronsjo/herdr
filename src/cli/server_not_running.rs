use std::fmt;
use std::path::Path;

use crate::api::schema::{ErrorBody, ErrorResponse};

/// Marker error signalling a dead API socket. Carries the `ErrorResponse` that
/// should be printed at the edge that finally surfaces the error, so callers
/// that recover (e.g. plugin offline fallback) print nothing. Mirrors
/// `ProtocolMismatchReported`, except printing is deferred because several CLI
/// commands recover from a dead server instead of reporting it.
#[derive(Debug)]
pub(super) struct ServerNotRunningReported {
    pub(super) response: ErrorResponse,
}

impl fmt::Display for ServerNotRunningReported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to the carried message so pre-existing paths that
        // stringify transport errors still show the actionable text.
        f.write_str(&self.response.error.message)
    }
}

impl std::error::Error for ServerNotRunningReported {}

/// Builds the friendly `server_not_running` ErrorResponse shown when no
/// server is listening on the resolved API socket.
pub(super) fn response(request_id: &str, socket_path: &Path) -> ErrorResponse {
    let attach_command = startup_command(socket_path);
    ErrorResponse {
        id: request_id.to_string(),
        error: ErrorBody {
            code: "server_not_running".into(),
            message: format!(
                "no herdr server is running at {}; run `{attach_command}` to start or attach it",
                socket_path.display()
            ),
        },
    }
}

/// Builds the `server_api_not_accepting` ErrorResponse shown when the API
/// socket exists and refuses connections while a herdr server is still
/// answering on the paired client socket. Running `herdr` here would delete
/// this stale-looking-but-live socket file and start a second server
/// (`crate::ipc::prepare_socket_path`), so the message steers away from that
/// remedy instead of repeating it.
pub(super) fn not_accepting_response(request_id: &str, socket_path: &Path) -> ErrorResponse {
    ErrorResponse {
        id: request_id.to_string(),
        error: ErrorBody {
            code: "server_api_not_accepting".into(),
            message: format!(
                "the herdr server at {} is not accepting api connections, but a herdr server is still running; do not run `herdr` here (it would delete this socket and start a second server) — check `herdr status server` and restart the server deliberately if it is stuck",
                socket_path.display()
            ),
        },
    }
}

/// Builds the `server_api_not_responding` ErrorResponse shown when the API
/// socket accepted the connection but the server never replied to the
/// handshake ping within the probe timeout.
pub(super) fn not_responding_response(
    request_id: &str,
    socket_path: &Path,
    timeout: std::time::Duration,
) -> ErrorResponse {
    ErrorResponse {
        id: request_id.to_string(),
        error: ErrorBody {
            code: "server_api_not_responding".into(),
            message: format!(
                "connected to the herdr server at {} but got no handshake reply after waiting {}s",
                socket_path.display(),
                timeout.as_secs()
            ),
        },
    }
}

fn startup_command(socket_path: &Path) -> String {
    let session_socket =
        crate::session::api_socket_path_for(crate::session::active_name().as_deref());
    if socket_path == session_socket {
        crate::session::local_attach_command()
    } else {
        // A socket override wins over an inherited HERDR_SESSION. Keep the
        // command in the current environment so it starts the overridden
        // target instead of directing the user to an unrelated session.
        "herdr".to_string()
    }
}

/// Wraps the response in the recognizable marker WITHOUT printing. The caller
/// that ultimately surfaces the error prints the carried response (see
/// `reported_response`); recovering callers simply drop it.
pub(super) fn reported_error(response: ErrorResponse) -> std::io::Error {
    std::io::Error::other(ServerNotRunningReported { response })
}

pub(super) fn was_reported(err: &std::io::Error) -> bool {
    err.get_ref()
        .and_then(|source| source.downcast_ref::<ServerNotRunningReported>())
        .is_some()
}

/// Returns the `ErrorResponse` carried by a `server_not_running` marker, if any,
/// so the surfacing edge can print it exactly once.
pub(super) fn reported_response(err: &std::io::Error) -> Option<&ErrorResponse> {
    err.get_ref()
        .and_then(|source| source.downcast_ref::<ServerNotRunningReported>())
        .map(|reported| &reported.response)
}
