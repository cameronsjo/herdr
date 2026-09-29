//! Fork-only registry of agents that run outside any herdr pane.
//!
//! The agents panel lists herdr panes, so a Claude session in cmux, a plain
//! terminal, or an editor never appears there. A process closes that gap by
//! calling `agent.register` for itself and repeating the call as a heartbeat
//! before its TTL runs out; a record that misses its heartbeat expires.
//!
//! This is shared runtime state, so it lives on the server. Clients receive it
//! as the optional `fork.registered-agents.v1` endpoint control, which older
//! clients ignore, so the stable snapshot codec is untouched.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::api::schema::{AgentRegisterParams, AgentStatus, RegisteredAgentInfo};

pub(crate) const DEFAULT_TTL_MS: u64 = 60_000;
pub(crate) const MIN_TTL_MS: u64 = 1_000;
pub(crate) const MAX_TTL_MS: u64 = 86_400_000;
pub(crate) const MAX_ENTRIES: usize = 256;
/// Budget for the whole registry as JSON. The endpoint control carries all of
/// it in one frame, so it must stay well under `MAX_FRAME_SIZE`.
const MAX_ENCODED_BYTES: usize = 512 * 1024;
const MAX_ID_BYTES: usize = 128;
const MAX_AGENT_BYTES: usize = 64;
const MAX_CWD_BYTES: usize = 4096;
const MAX_TOKENS: usize = 32;
const MAX_TOKEN_KEY_BYTES: usize = 64;
const MAX_TOKEN_VALUE_BYTES: usize = 256;

/// Optional endpoint control carrying the full registry to client shells.
pub(crate) const ENDPOINT_KIND: &str = "fork.registered-agents.v1";

/// Prefix of the synthetic pane ids the client gives registered-agent rows.
/// No real pane id starts with it, so a row with it is never focusable.
pub(crate) const PANE_ID_PREFIX: &str = "registered:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisterError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl RegisterError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_params",
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    info: RegisteredAgentInfo,
    expires_at: Instant,
    encoded_len: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RegisteredAgents {
    entries: BTreeMap<(String, String), Entry>,
    /// Bumped whenever the visible set changes, so clients are sent the
    /// registry only when it differs from what they last received.
    revision: u64,
}

impl RegisteredAgents {
    /// Inserts or refreshes a record. A heartbeat that changes nothing but the
    /// deadline leaves `revision` alone, so it costs clients nothing.
    pub(crate) fn register(
        &mut self,
        params: AgentRegisterParams,
        now: Instant,
    ) -> Result<RegisteredAgentInfo, RegisterError> {
        let info = validate(params)?;
        let key = (info.source.clone(), info.name.clone());
        if !self.entries.contains_key(&key) && self.entries.len() >= MAX_ENTRIES {
            return Err(RegisterError {
                code: "registered_agent_limit",
                message: format!("at most {MAX_ENTRIES} agents can be registered"),
            });
        }
        let encoded_len = serde_json::to_vec(&info).map_or(usize::MAX, |bytes| bytes.len());
        let others = self
            .entries
            .iter()
            .filter(|(existing, _)| **existing != key)
            .map(|(_, entry)| entry.encoded_len)
            .sum::<usize>();
        if others.saturating_add(encoded_len) > MAX_ENCODED_BYTES {
            return Err(RegisterError {
                code: "registered_agent_limit",
                message: format!("registered agents may total at most {MAX_ENCODED_BYTES} bytes"),
            });
        }
        let expires_at = now + Duration::from_millis(info.ttl_ms);
        let changed = self
            .entries
            .get(&key)
            .is_none_or(|entry| entry.info != info);
        self.entries.insert(
            key,
            Entry {
                info: info.clone(),
                expires_at,
                encoded_len,
            },
        );
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
        Ok(info)
    }

    pub(crate) fn unregister(&mut self, source: &str, name: &str) -> bool {
        let removed = self
            .entries
            .remove(&(source.to_owned(), name.to_owned()))
            .is_some();
        if removed {
            self.revision = self.revision.wrapping_add(1);
        }
        removed
    }

    /// Drops every record whose deadline has passed. Returns whether any went.
    pub(crate) fn expire(&mut self, now: Instant) -> bool {
        let before = self.entries.len();
        self.entries.retain(|_, entry| entry.expires_at > now);
        let expired = self.entries.len() != before;
        if expired {
            self.revision = self.revision.wrapping_add(1);
        }
        expired
    }

    pub(crate) fn next_expiry(&self) -> Option<Instant> {
        self.entries.values().map(|entry| entry.expires_at).min()
    }

    pub(crate) fn list(&self) -> Vec<RegisteredAgentInfo> {
        self.entries
            .values()
            .map(|entry| entry.info.clone())
            .collect()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
}

fn validate(params: AgentRegisterParams) -> Result<RegisteredAgentInfo, RegisterError> {
    let AgentRegisterParams {
        source,
        name,
        agent,
        status,
        cwd,
        tokens,
        ttl_ms,
    } = params;
    check_text("source", &source, MAX_ID_BYTES, false)?;
    // Clients key rows as `registered:<source>/<name>`, so a slash in the
    // source would let two records share a row id.
    if source.contains('/') {
        return Err(RegisterError::invalid("source must not contain '/'"));
    }
    check_text("name", &name, MAX_ID_BYTES, false)?;
    if let Some(agent) = &agent {
        check_text("agent", agent, MAX_AGENT_BYTES, false)?;
    }
    if let Some(cwd) = &cwd {
        check_text("cwd", cwd, MAX_CWD_BYTES, false)?;
    }
    if tokens.len() > MAX_TOKENS {
        return Err(RegisterError::invalid(format!(
            "at most {MAX_TOKENS} tokens are allowed"
        )));
    }
    for (key, value) in &tokens {
        check_text("token key", key, MAX_TOKEN_KEY_BYTES, false)?;
        check_text("token value", value, MAX_TOKEN_VALUE_BYTES, true)?;
    }
    let ttl_ms = ttl_ms.unwrap_or(DEFAULT_TTL_MS);
    if !(MIN_TTL_MS..=MAX_TTL_MS).contains(&ttl_ms) {
        return Err(RegisterError::invalid(format!(
            "ttl_ms must be between {MIN_TTL_MS} and {MAX_TTL_MS}"
        )));
    }
    Ok(RegisteredAgentInfo {
        source,
        name,
        agent,
        status: status.unwrap_or(AgentStatus::Unknown),
        cwd,
        tokens,
        ttl_ms,
    })
}

fn check_text(
    field: &str,
    value: &str,
    max_bytes: usize,
    allow_empty: bool,
) -> Result<(), RegisterError> {
    if !allow_empty && value.trim().is_empty() {
        return Err(RegisterError::invalid(format!("{field} must not be empty")));
    }
    if value.len() > max_bytes {
        return Err(RegisterError::invalid(format!(
            "{field} must be at most {max_bytes} bytes"
        )));
    }
    // Format characters (bidi overrides, zero-width) are refused with control
    // characters: these values are drawn in the panel beside real agents.
    if value
        .chars()
        .any(|ch| ch.is_control() || crate::label::is_format_char(ch))
    {
        return Err(RegisterError::invalid(format!(
            "{field} must not contain control or format characters"
        )));
    }
    Ok(())
}

/// Payload of the `fork.registered-agents.v1` endpoint control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EndpointRegisteredAgents {
    pub(crate) agents: Vec<RegisteredAgentInfo>,
}

/// The control to send a client shell that last saw `sent_revision`, or
/// `None` when it is already current. Records the new revision as sent.
pub(crate) fn client_update(
    registry: &RegisteredAgents,
    sent_revision: &mut Option<u64>,
) -> Option<crate::protocol::ServerMessage> {
    if *sent_revision == Some(registry.revision()) {
        return None;
    }
    // A client that has never seen a registered agent needs no empty list, so
    // a server with none sends nothing and old message sequences stay intact.
    if sent_revision.is_none() && registry.entries.is_empty() {
        *sent_revision = Some(registry.revision());
        return None;
    }
    let data = match serde_json::to_string(&EndpointRegisteredAgents {
        agents: registry.list(),
    }) {
        Ok(data) => data,
        Err(err) => {
            tracing::warn!(err = %err, "failed to encode registered agents");
            return None;
        }
    };
    *sent_revision = Some(registry.revision());
    Some(crate::protocol::ServerMessage::EndpointControl {
        kind: ENDPOINT_KIND.into(),
        data,
    })
}

/// Decodes the `fork.registered-agents.v1` payload on the client.
///
/// A remote endpoint is not trusted to keep the server's rules, and a newer
/// one may send a status this client does not know. An unknown status reads
/// as `unknown`, a malformed record is dropped, the list is capped, and a
/// payload that does not parse at all clears the list rather than leaving
/// stale rows behind.
pub(crate) fn decode_endpoint_agents(data: &str) -> Vec<RegisteredAgentInfo> {
    let Ok(serde_json::Value::Object(mut payload)) = serde_json::from_str(data) else {
        return Vec::new();
    };
    let Some(serde_json::Value::Array(agents)) = payload.remove("agents") else {
        return Vec::new();
    };
    agents
        .into_iter()
        .filter_map(|mut agent| {
            if let Some(status) = agent.get_mut("status") {
                if serde_json::from_value::<AgentStatus>(status.clone()).is_err() {
                    *status = serde_json::Value::String("unknown".into());
                }
            }
            serde_json::from_value(agent).ok()
        })
        .take(MAX_ENTRIES)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(name: &str) -> AgentRegisterParams {
        AgentRegisterParams {
            source: "test".into(),
            name: name.into(),
            agent: Some("claude".into()),
            status: Some(AgentStatus::Working),
            cwd: None,
            tokens: BTreeMap::new(),
            ttl_ms: Some(5_000),
        }
    }

    #[test]
    fn register_lists_and_unregister_removes() {
        let mut registry = RegisteredAgents::default();
        let now = Instant::now();
        let info = registry.register(params("a"), now).unwrap();
        assert_eq!(info.status, AgentStatus::Working);
        assert_eq!(registry.list(), vec![info]);
        assert!(registry.unregister("test", "a"));
        assert!(!registry.unregister("test", "a"));
        assert!(registry.list().is_empty());
    }

    #[test]
    fn heartbeat_extends_the_deadline_without_a_new_revision() {
        let mut registry = RegisteredAgents::default();
        let now = Instant::now();
        registry.register(params("a"), now).unwrap();
        let revision = registry.revision();
        let later = now + Duration::from_millis(4_000);
        registry.register(params("a"), later).unwrap();
        assert_eq!(registry.revision(), revision);
        assert_eq!(
            registry.next_expiry(),
            Some(later + Duration::from_millis(5_000))
        );
        assert!(!registry.expire(now + Duration::from_millis(6_000)));
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn status_change_bumps_the_revision() {
        let mut registry = RegisteredAgents::default();
        let now = Instant::now();
        registry.register(params("a"), now).unwrap();
        let revision = registry.revision();
        let mut blocked = params("a");
        blocked.status = Some(AgentStatus::Blocked);
        registry.register(blocked, now).unwrap();
        assert_ne!(registry.revision(), revision);
    }

    #[test]
    fn records_expire_after_their_ttl() {
        let mut registry = RegisteredAgents::default();
        let now = Instant::now();
        registry.register(params("a"), now).unwrap();
        assert!(!registry.expire(now + Duration::from_millis(4_999)));
        assert!(registry.expire(now + Duration::from_millis(5_000)));
        assert!(registry.list().is_empty());
        assert_eq!(registry.next_expiry(), None);
    }

    #[test]
    fn invalid_params_are_rejected() {
        let mut registry = RegisteredAgents::default();
        let now = Instant::now();
        let mut empty = params(" ");
        empty.name = " ".into();
        assert_eq!(
            registry.register(empty, now).unwrap_err().code,
            "invalid_params"
        );
        let mut short = params("a");
        short.ttl_ms = Some(MIN_TTL_MS - 1);
        assert!(registry.register(short, now).is_err());
        let mut control = params("a");
        control.cwd = Some("/tmp/\u{1b}[2J".into());
        assert!(registry.register(control, now).is_err());
        let mut bidi = params("a");
        bidi.name = "evil\u{202e}gnp.exe".into();
        assert!(registry.register(bidi, now).is_err());
        assert!(registry.list().is_empty());
    }

    #[test]
    fn registry_size_is_budgeted() {
        let mut registry = RegisteredAgents::default();
        let now = Instant::now();
        let fat = |name: &str| {
            let mut params = params(name);
            params.cwd = Some("x".repeat(MAX_CWD_BYTES));
            params
        };
        let mut accepted = 0;
        while registry.register(fat(&accepted.to_string()), now).is_ok() {
            accepted += 1;
        }
        assert!(accepted < MAX_ENTRIES);
        assert_eq!(
            registry.register(fat("over"), now).unwrap_err().code,
            "registered_agent_limit"
        );
        // Refreshing an existing record does not count it twice.
        registry.register(fat("0"), now).unwrap();
        let encoded = serde_json::to_vec(&EndpointRegisteredAgents {
            agents: registry.list(),
        })
        .unwrap();
        assert!(encoded.len() < crate::protocol::MAX_FRAME_SIZE);
    }

    #[test]
    fn source_may_not_contain_a_slash() {
        let mut registry = RegisteredAgents::default();
        let mut slashed = params("c");
        slashed.source = "a/b".into();
        assert!(registry.register(slashed, Instant::now()).is_err());
    }

    #[test]
    fn registry_is_capped() {
        let mut registry = RegisteredAgents::default();
        let now = Instant::now();
        for index in 0..MAX_ENTRIES {
            registry.register(params(&index.to_string()), now).unwrap();
        }
        assert_eq!(
            registry.register(params("over"), now).unwrap_err().code,
            "registered_agent_limit"
        );
        registry.register(params("0"), now).unwrap();
    }

    #[test]
    fn client_update_sends_once_per_revision() {
        let mut registry = RegisteredAgents::default();
        let mut sent = None;
        assert!(client_update(&registry, &mut sent).is_none());
        assert!(client_update(&registry, &mut sent).is_none());
        registry.register(params("a"), Instant::now()).unwrap();
        let Some(crate::protocol::ServerMessage::EndpointControl { kind, data }) =
            client_update(&registry, &mut sent)
        else {
            panic!("expected a registered-agents control");
        };
        assert_eq!(kind, ENDPOINT_KIND);
        let decoded: EndpointRegisteredAgents = serde_json::from_str(&data).unwrap();
        assert_eq!(decoded.agents, registry.list());
        assert!(client_update(&registry, &mut sent).is_none());
        assert!(registry.unregister("test", "a"));
        let Some(crate::protocol::ServerMessage::EndpointControl { data, .. }) =
            client_update(&registry, &mut sent)
        else {
            panic!("a client that saw agents must hear the list emptied");
        };
        let decoded: EndpointRegisteredAgents = serde_json::from_str(&data).unwrap();
        assert!(decoded.agents.is_empty());
    }

    #[test]
    fn client_decode_tolerates_new_statuses_and_bad_records() {
        let data = r#"{"agents":[
            {"source":"s","name":"a","status":"thinking","ttl_ms":1000},
            {"source":"s","name":"b","status":"idle","ttl_ms":1000},
            {"name":"missing-source","status":"idle","ttl_ms":1000}
        ]}"#;
        let agents = decode_endpoint_agents(data);
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].status, AgentStatus::Unknown);
        assert_eq!(agents[1].status, AgentStatus::Idle);
        assert!(decode_endpoint_agents("not json").is_empty());
    }

    #[test]
    fn a_new_client_gets_the_current_list_when_agents_exist() {
        let mut registry = RegisteredAgents::default();
        registry.register(params("a"), Instant::now()).unwrap();
        assert!(client_update(&registry, &mut None).is_some());
    }
}
