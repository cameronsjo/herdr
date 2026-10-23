//! Fork: environment variables for the process `agent.start` launches
//! (cameronsjo/herdr#103).
//!
//! Values come from the caller and may be credentials, so nothing here (and
//! nothing that consumes it) may print a value: `Debug` lists keys only, and
//! every error names at most a key.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// The key grammar, quoted verbatim in error messages.
pub const AGENT_ENV_KEY_PATTERN: &str = "^[A-Za-z_][A-Za-z0-9_]*$";

/// Environment variables set for the agent process only, keyed by name.
///
/// Serialized as a JSON object of string values. An empty map is the default
/// and is omitted on the wire, so requests without `env` are unchanged.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct AgentEnv(pub BTreeMap<String, String>);

impl AgentEnv {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// Adds one CLI `KEY=VALUE` pair; a repeated key keeps the last value.
    pub fn insert_pair(&mut self, pair: &str) -> Result<(), AgentEnvError> {
        let (key, value) = parse_agent_env_pair(pair)?;
        self.0.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    /// Checks every entry against the key grammar and rejects control
    /// characters in values, which could not be typed safely into a shell.
    pub fn validate(&self) -> Result<(), AgentEnvError> {
        for (key, value) in self.iter() {
            if !valid_agent_env_key(key) {
                return Err(AgentEnvError::InvalidKey);
            }
            if value.chars().any(char::is_control) {
                return Err(AgentEnvError::InvalidValue {
                    key: key.to_owned(),
                });
            }
        }
        Ok(())
    }
}

/// Lists keys only, so a stray `{:?}` of a request can never leak a value.
impl fmt::Debug for AgentEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.0.keys().map(|key| (key, "<redacted>")))
            .finish()
    }
}

impl<const N: usize> From<[(&str, &str); N]> for AgentEnv {
    fn from(entries: [(&str, &str); N]) -> Self {
        Self(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
        )
    }
}

/// Why an env entry was refused. No variant carries a value, and an invalid
/// key is not repeated either, since a malformed key may be a misplaced value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEnvError {
    MissingEquals,
    InvalidKey,
    InvalidValue { key: String },
}

impl fmt::Display for AgentEnvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEquals => f.write_str("agent env entries must be KEY=VALUE"),
            Self::InvalidKey => write!(f, "agent env keys must match {AGENT_ENV_KEY_PATTERN}"),
            Self::InvalidValue { key } => {
                write!(f, "agent env value for {key} contains control characters")
            }
        }
    }
}

impl std::error::Error for AgentEnvError {}

/// `^[A-Za-z_][A-Za-z0-9_]*$`
pub fn valid_agent_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// Splits `KEY=VALUE` at the first `=`; the value may itself contain `=`.
pub fn parse_agent_env_pair(pair: &str) -> Result<(&str, &str), AgentEnvError> {
    let (key, value) = pair.split_once('=').ok_or(AgentEnvError::MissingEquals)?;
    if !valid_agent_env_key(key) {
        return Err(AgentEnvError::InvalidKey);
    }
    if value.chars().any(char::is_control) {
        return Err(AgentEnvError::InvalidValue {
            key: key.to_owned(),
        });
    }
    Ok((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "sk-secret-value-do-not-print";

    #[test]
    fn key_grammar_matches_the_documented_pattern() {
        for key in ["A", "_", "_A1", "CLAUDE_CONFIG_DIR", "lower_case", "x9"] {
            assert!(valid_agent_env_key(key), "{key} should be valid");
        }
        for key in ["", "1A", "A-B", "A.B", "A B", "É", "A=B", "$A", "-A"] {
            assert!(!valid_agent_env_key(key), "{key:?} should be invalid");
        }
    }

    #[test]
    fn pair_without_equals_is_rejected_without_echo() {
        let error = parse_agent_env_pair(SECRET).unwrap_err();
        assert_eq!(error, AgentEnvError::MissingEquals);
        assert!(!error.to_string().contains(SECRET));
        assert!(!format!("{error:?}").contains(SECRET));
    }

    #[test]
    fn pair_splits_at_first_equals_and_allows_empty_value() {
        assert_eq!(parse_agent_env_pair("A=b=c"), Ok(("A", "b=c")));
        assert_eq!(parse_agent_env_pair("A="), Ok(("A", "")));
    }

    #[test]
    fn invalid_key_error_repeats_neither_key_nor_value() {
        let pair = format!("{SECRET}={SECRET}");
        let error = parse_agent_env_pair(&pair).unwrap_err();
        assert_eq!(error, AgentEnvError::InvalidKey);
        assert!(!error.to_string().contains(SECRET));
    }

    #[test]
    fn control_character_value_names_only_the_key() {
        let pair = format!("TOKEN={SECRET}\n");
        let error = parse_agent_env_pair(&pair).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("TOKEN"));
        assert!(!message.contains(SECRET));
        assert!(!format!("{error:?}").contains(SECRET));
    }

    #[test]
    fn repeated_key_keeps_the_last_value() {
        let mut env = AgentEnv::default();
        env.insert_pair("A=1").unwrap();
        env.insert_pair("A=2").unwrap();
        assert_eq!(env, AgentEnv::from([("A", "2")]));
    }

    #[test]
    fn validate_rejects_bad_keys_and_values_from_the_api() {
        assert_eq!(
            AgentEnv::from([("1BAD", "x")]).validate(),
            Err(AgentEnvError::InvalidKey)
        );
        assert_eq!(
            AgentEnv::from([("OK", "a\u{7}b")]).validate(),
            Err(AgentEnvError::InvalidValue { key: "OK".into() })
        );
        assert_eq!(AgentEnv::from([("OK", "")]).validate(), Ok(()));
    }

    #[test]
    fn debug_redacts_values() {
        let env = AgentEnv::from([("TOKEN", SECRET)]);
        let debug = format!("{env:?}");
        assert!(debug.contains("TOKEN"));
        assert!(!debug.contains(SECRET));
    }

    #[test]
    fn agent_start_env_is_optional_on_the_wire() {
        use crate::api::schema::{Method, Request};

        let legacy = serde_json::json!({
            "id": "start",
            "method": "agent.start",
            "params": {"name": "w", "kind": "claude", "pane_id": "1"},
        });
        let request: Request = serde_json::from_value(legacy.clone()).unwrap();
        let Method::AgentStart(params) = &request.method else {
            panic!("agent.start");
        };
        assert!(params.env.is_empty());
        assert_eq!(serde_json::to_value(&request).unwrap(), legacy);

        let mut with_env = legacy;
        with_env["params"]["env"] = serde_json::json!({"CLAUDE_CONFIG_DIR": "/alt"});
        let request: Request = serde_json::from_value(with_env.clone()).unwrap();
        let Method::AgentStart(params) = &request.method else {
            panic!("agent.start");
        };
        assert_eq!(params.env, AgentEnv::from([("CLAUDE_CONFIG_DIR", "/alt")]));
        assert_eq!(serde_json::to_value(&request).unwrap(), with_env);
        assert!(!format!("{request:?}").contains("/alt"));
    }

    #[test]
    fn serializes_as_a_plain_object() {
        let env = AgentEnv::from([("A", "1")]);
        assert_eq!(
            serde_json::to_value(&env).unwrap(),
            serde_json::json!({"A": "1"})
        );
        let parsed: AgentEnv = serde_json::from_value(serde_json::json!({"B": "2"})).unwrap();
        assert_eq!(parsed, AgentEnv::from([("B", "2")]));
    }
}
