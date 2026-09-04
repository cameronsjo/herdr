use std::fmt;

use crate::api::schema::{ErrorBody, ErrorResponse};

#[derive(Debug)]
pub(super) struct ProtocolMismatchReported;

impl fmt::Display for ProtocolMismatchReported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("protocol mismatch was already reported")
    }
}

impl std::error::Error for ProtocolMismatchReported {}

pub(super) fn mismatch_response(
    request_id: &str,
    server_protocol: u32,
    restart_guidance: &str,
) -> Option<ErrorResponse> {
    let client_protocol = crate::protocol::PROTOCOL_VERSION;
    if client_protocol == server_protocol {
        return None;
    }

    // Only one side carrying the fork offset is ambiguous, and the ambiguity is
    // the point: a number below the offset is either an upstream build or a fork
    // build from before the offset existed. So this augments the usual advice
    // rather than replacing it — restarting a stale server is still the fix for
    // the common case, and an operator facing the other case needs to be told
    // that no amount of restarting will help.
    let cross_distribution = crate::protocol::is_fork_protocol(client_protocol)
        != crate::protocol::is_fork_protocol(server_protocol);
    let distribution_note = if cross_distribution {
        let (offset_side, plain_side) = if crate::protocol::is_fork_protocol(client_protocol) {
            ("client", "server")
        } else {
            ("server", "client")
        };
        format!(
            " If restarting does not help, these may be different herdr distributions rather than \
             different versions: the {offset_side} carries this fork's protocol offset and the \
             {plain_side} does not, which also describes a {plain_side} built before the offset \
             existed. Run both from the same distribution."
        )
    } else {
        String::new()
    };

    let message = if client_protocol > server_protocol {
        format!(
            "client protocol {client_protocol} is newer than server protocol {server_protocol}; restart the Herdr server before using this command. {restart_guidance}{distribution_note}"
        )
    } else {
        format!(
            "client protocol {client_protocol} is older than server protocol {server_protocol}; upgrade the Herdr client before using this command{distribution_note}"
        )
    };

    Some(ErrorResponse {
        id: request_id.to_string(),
        error: ErrorBody {
            code: "protocol_mismatch".into(),
            message,
        },
    })
}

pub(super) fn reported_error() -> std::io::Error {
    std::io::Error::other(ProtocolMismatchReported)
}

pub(super) fn was_reported(err: &std::io::Error) -> bool {
    err.get_ref()
        .and_then(|source| source.downcast_ref::<ProtocolMismatchReported>())
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_protocol_has_no_error() {
        assert!(mismatch_response("req", crate::protocol::PROTOCOL_VERSION, "restart").is_none());
    }

    #[test]
    fn older_server_error_preserves_request_id_and_guidance() {
        let response = mismatch_response(
            "cli:agent:wait",
            crate::protocol::PROTOCOL_VERSION - 1,
            "Run the session stop command, then restart.",
        )
        .unwrap();

        assert_eq!(response.id, "cli:agent:wait");
        assert_eq!(response.error.code, "protocol_mismatch");
        assert!(response.error.message.contains(&format!(
            "client protocol {}",
            crate::protocol::PROTOCOL_VERSION
        )));
        assert!(response.error.message.contains(&format!(
            "server protocol {}",
            crate::protocol::PROTOCOL_VERSION - 1
        )));
        assert!(response.error.message.contains("restart"));
    }

    #[test]
    fn newer_server_error_tells_user_to_upgrade_client() {
        let response = mismatch_response(
            "cli:pane:list",
            crate::protocol::PROTOCOL_VERSION + 1,
            "unused restart guidance",
        )
        .unwrap();

        assert!(response
            .error
            .message
            .contains("older than server protocol"));
        assert!(response.error.message.contains("upgrade the Herdr client"));
        assert!(!response.error.message.contains("unused restart guidance"));
    }

    #[test]
    fn reported_error_is_recognizable_without_string_matching() {
        assert!(was_reported(&reported_error()));
        assert!(!was_reported(&std::io::Error::other(
            "protocol mismatch was already reported"
        )));
    }
}
