//! Fork: environment variables for the process `agent.start` launches
//! (cameronsjo/herdr#103).
//!
//! Values come from the caller and may be credentials, so nothing here (and
//! nothing that consumes it) may print a value: `Debug` lists keys only, and
//! every error names at most a key.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

/// The key grammar, quoted verbatim in error messages.
pub const AGENT_ENV_KEY_PATTERN: &str = "^[A-Za-z_][A-Za-z0-9_]*$";

/// Environment variables set for the agent process only, keyed by name.
///
/// Serialized as a JSON object of string values. An empty map is the default
/// and is omitted on the wire, so requests without `env` are unchanged.
// `Deserialize` is hand-written (below) so a wrongly typed `env` or value is
// refused without serde quoting the offending value back to the caller.
#[derive(Clone, Default, PartialEq, Eq, Serialize, schemars::JsonSchema)]
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
            if !valid_agent_env_value(value) {
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
                write!(
                    f,
                    "agent env value for {key} contains control, separator or format characters"
                )
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
    if !valid_agent_env_value(value) {
        return Err(AgentEnvError::InvalidValue {
            key: key.to_owned(),
        });
    }
    Ok((key, value))
}

/// Refuses control characters (Cc, which includes U+0085), the line and
/// paragraph separators U+2028/U+2029, and format characters (Cf: bidi
/// overrides, zero-width), which could make the command echoed in the pane
/// read differently from what runs.
pub fn valid_agent_env_value(value: &str) -> bool {
    !value.chars().any(|ch| {
        ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}') || crate::label::is_format_char(ch)
    })
}

const ENV_TYPE_ERROR: &str = "agent env must be an object of string values";

fn redacted_type_error<E: de::Error>() -> E {
    E::custom(ENV_TYPE_ERROR)
}

/// Overrides every scalar `visit_*` that serde would otherwise report with
/// the value quoted (`invalid type: string "..."`). Both visitors use
/// `deserialize_any`, because serde_json reports a type mismatch on
/// `deserialize_map`/`deserialize_string` itself, value included, without
/// asking the visitor.
macro_rules! refuse_scalars {
    () => {
        fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
            Err(redacted_type_error())
        }
        fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
            Err(redacted_type_error())
        }
        fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
            Err(redacted_type_error())
        }
        fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
            Err(redacted_type_error())
        }
        fn visit_bytes<E: de::Error>(self, _: &[u8]) -> Result<Self::Value, E> {
            Err(redacted_type_error())
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Err(redacted_type_error())
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Err(redacted_type_error())
        }
        fn visit_seq<A: de::SeqAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
            Err(redacted_type_error())
        }
    };
}

struct EnvValue(String);

impl<'de> Deserialize<'de> for EnvValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = EnvValue;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a string")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<EnvValue, E> {
                Ok(EnvValue(value.to_owned()))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<EnvValue, E> {
                Ok(EnvValue(value))
            }
            fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<EnvValue, A::Error> {
                Err(redacted_type_error())
            }
            refuse_scalars!();
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}

impl<'de> Deserialize<'de> for AgentEnv {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EnvVisitor;
        impl<'de> Visitor<'de> for EnvVisitor {
            type Value = AgentEnv;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(ENV_TYPE_ERROR)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<AgentEnv, A::Error> {
                let mut env = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let EnvValue(value) = map.next_value()?;
                    env.insert(key, value);
                }
                Ok(AgentEnv(env))
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<AgentEnv, E> {
                Err(redacted_type_error())
            }
            refuse_scalars!();
        }
        deserializer.deserialize_any(EnvVisitor)
    }
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
    fn values_refuse_separators_and_format_characters() {
        for bad in [
            "a\u{2028}b",
            "a\u{2029}b",
            "a\u{0085}b",
            "a\u{202E}b",
            "a\u{200B}b",
            "a\u{2066}b",
            "a\u{FEFF}b",
            "a\rb",
            "a\u{7F}b",
        ] {
            let pair = format!("K={bad}");
            assert_eq!(
                parse_agent_env_pair(&pair),
                Err(AgentEnvError::InvalidValue { key: "K".into() }),
                "{bad:?}"
            );
            assert!(AgentEnv::from([("K", bad)]).validate().is_err(), "{bad:?}");
        }
        assert!(valid_agent_env_value(
            "plain ’quoted‘ $x `y` !z %p ^c ;# \\"
        ));
    }

    #[test]
    fn mistyped_env_is_refused_without_echoing_it() {
        use crate::api::schema::Request;

        for env in [
            serde_json::json!(SECRET),
            serde_json::json!({"K": 731_947_113_u64}),
            serde_json::json!({"K": -731_947_113_i64}),
            serde_json::json!({"K": true}),
            serde_json::json!({"K": [SECRET]}),
            serde_json::json!({"K": {"nested": SECRET}}),
            serde_json::json!([SECRET]),
            serde_json::json!(42),
        ] {
            let request = serde_json::json!({
                "id": "start",
                "method": "agent.start",
                "params": {"name": "w", "kind": "claude", "pane_id": "1", "env": env},
            })
            .to_string();
            let error = serde_json::from_str::<Request>(&request).unwrap_err();
            let message = error.to_string();
            assert!(!message.contains(SECRET), "{message}");
            assert!(!message.contains("731947113"), "{message}");
        }
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
