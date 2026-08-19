use crate::api::schema::{ErrorBody, ErrorResponse, ResponseResult, SuccessResponse};

pub(super) fn encode_success(id: String, result: ResponseResult) -> String {
    serde_json::to_string(&SuccessResponse { id, result }).unwrap()
}

pub(super) fn encode_error(id: String, code: &str, message: impl Into<String>) -> String {
    encode_error_body(
        id,
        ErrorBody {
            code: code.into(),
            message: message.into(),
        },
    )
}

pub(super) fn encode_error_body(id: String, error: ErrorBody) -> String {
    serde_json::to_string(&ErrorResponse { id, error }).unwrap()
}

/// Strips control characters from a user-supplied workspace or tab label.
///
/// Labels are rendered directly into the sidebar and tab bar, so an embedded
/// ANSI escape or newline corrupts the surrounding frame. Only control
/// characters go — multibyte text is legitimate (workspaces are routinely named
/// in CJK), so this must never be an ASCII filter.
pub(super) fn sanitize_label(label: String) -> String {
    label.chars().filter(|ch| !ch.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use super::sanitize_label;

    #[test]
    fn sanitize_label_strips_controls_and_keeps_multibyte_text() {
        assert_eq!(
            sanitize_label("stat\u{1b}[31mus\nboard".into()),
            "stat[31musboard"
        );
        assert_eq!(
            sanitize_label("提交 herdr 的反馈".into()),
            "提交 herdr 的反馈"
        );
        assert_eq!(sanitize_label("plain".into()), "plain");
    }
}
