//! Fork-only: pin an agent to the top of the agents panel (cameronsjo/herdr#96).
//!
//! In an orchestrator setup the operator returns to one agent all day, and the
//! panel's status and recency orders keep moving it. `agent.pin` marks the
//! agent's terminal with a pin order; the panel lists pinned agents first, in
//! the order they were pinned, whatever their status.
//!
//! The pin is a server fact, carried to client shells inside the agent's
//! `tokens` under a reserved key. Pane metadata token keys are limited to
//! letters, digits, `_` and `-`, so no reporter can set a key with a colon, and
//! a client that does not know the key ignores it like any other token. The
//! stable snapshot codec is untouched.

/// Reserved agent token whose value is the agent's pin order.
pub(crate) const PIN_TOKEN: &str = "herdr:pinned";

/// The pin order a client shell agent carries, if it is pinned.
pub(crate) fn pin_order(tokens: &[(String, String)]) -> Option<u64> {
    tokens
        .iter()
        .find(|(key, _)| key == PIN_TOKEN)
        .and_then(|(_, value)| value.parse().ok())
}

/// The order a newly pinned agent takes: after every agent already pinned.
pub(crate) fn next_pin_order(pinned: impl IntoIterator<Item = u64>) -> u64 {
    pinned
        .into_iter()
        .max()
        .map_or(1, |order| order.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_order_reads_only_the_reserved_token() {
        let tokens = vec![
            ("pinned".to_owned(), "7".to_owned()),
            (PIN_TOKEN.to_owned(), "3".to_owned()),
        ];
        assert_eq!(pin_order(&tokens), Some(3));
        assert_eq!(pin_order(&tokens[..1]), None);
        assert_eq!(pin_order(&[(PIN_TOKEN.to_owned(), "yes".to_owned())]), None);
    }

    #[test]
    fn next_pin_order_follows_the_latest_pin() {
        assert_eq!(next_pin_order([]), 1);
        assert_eq!(next_pin_order([4, 2]), 5);
        assert_eq!(next_pin_order([u64::MAX]), u64::MAX);
    }
}
