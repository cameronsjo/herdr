//! Fork-only API types for agents that run outside any herdr pane.
//!
//! A registered agent is a record a process keeps alive by re-registering
//! before its TTL runs out, so a Claude session in cmux or a plain terminal can
//! appear in the agents panel without a pane. See `crate::fork_registered_agents`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::common::AgentStatus;

/// `agent.register` params. Registering an existing `(source, name)` again is
/// the heartbeat: it replaces the record and restarts its TTL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AgentRegisterParams {
    /// Namespace of the reporter, such as `cadence.herdr-bridge`.
    pub source: String,
    /// Unique within `source`.
    pub name: String,
    /// Agent kind, such as `claude`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Defaults to `unknown`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Free-form labels, drawn by the agents panel's token formats.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<String, String>,
    /// Milliseconds until the record expires without another register.
    /// Defaults to 60000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
}

/// `agent.unregister` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AgentUnregisterParams {
    pub source: String,
    pub name: String,
}

/// One registered agent, as `agent.register` and `agent.registered` return it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RegisteredAgentInfo {
    pub source: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    pub status: AgentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<String, String>,
    pub ttl_ms: u64,
}
