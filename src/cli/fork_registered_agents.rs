//! Fork-only `herdr agent register|unregister|registered` commands.

use std::collections::BTreeMap;

use crate::api::schema::{
    AgentRegisterParams, AgentUnregisterParams, EmptyParams, Method, Request,
};

pub(super) const REGISTER_USAGE: &str = "herdr agent register <name> [--source ID] [--agent LABEL] [--status STATUS] [--cwd PATH] [--token KEY=VALUE]... [--ttl-ms N]";
pub(super) const UNREGISTER_USAGE: &str = "herdr agent unregister <name> [--source ID]";
pub(super) const REGISTERED_USAGE: &str = "herdr agent registered";
const DEFAULT_SOURCE: &str = "cli";

pub(super) fn agent_register(args: &[String]) -> std::io::Result<i32> {
    let Some(name) = args.first().filter(|name| !name.starts_with("--")) else {
        eprintln!("usage: {REGISTER_USAGE}");
        return Ok(2);
    };
    let mut params = AgentRegisterParams {
        source: DEFAULT_SOURCE.into(),
        name: name.clone(),
        agent: None,
        status: None,
        cwd: None,
        tokens: BTreeMap::new(),
        ttl_ms: None,
    };
    let mut index = 1;
    while index < args.len() {
        let flag = args[index].as_str();
        let Some(value) = args.get(index + 1) else {
            eprintln!("missing value for {flag}");
            return Ok(2);
        };
        match flag {
            "--source" => params.source = value.clone(),
            "--agent" => params.agent = Some(value.clone()),
            "--status" => match super::parse_agent_status(value) {
                Ok(status) => params.status = Some(status),
                Err(err) => {
                    eprintln!("{err}");
                    return Ok(2);
                }
            },
            "--cwd" => params.cwd = Some(value.clone()),
            "--token" => {
                let Some((key, token)) = value.split_once('=') else {
                    eprintln!("--token expects KEY=VALUE");
                    return Ok(2);
                };
                params.tokens.insert(key.to_owned(), token.to_owned());
            }
            "--ttl-ms" => match super::parse_u64_flag("--ttl-ms", value) {
                Ok(ttl_ms) => params.ttl_ms = Some(ttl_ms),
                Err(err) => {
                    eprintln!("{err}");
                    return Ok(2);
                }
            },
            _ => {
                eprintln!("unknown option: {flag}\nusage: {REGISTER_USAGE}");
                return Ok(2);
            }
        }
        index += 2;
    }
    super::print_response(&super::send_request(&Request {
        id: "cli:agent:register".into(),
        method: Method::AgentRegister(params),
    })?)
}

pub(super) fn agent_unregister(args: &[String]) -> std::io::Result<i32> {
    let (name, source) = match args {
        [name] => (name, DEFAULT_SOURCE.to_owned()),
        [name, flag, source] if flag == "--source" => (name, source.clone()),
        _ => {
            eprintln!("usage: {UNREGISTER_USAGE}");
            return Ok(2);
        }
    };
    super::print_response(&super::send_request(&Request {
        id: "cli:agent:unregister".into(),
        method: Method::AgentUnregister(AgentUnregisterParams {
            source,
            name: name.clone(),
        }),
    })?)
}

pub(super) fn agent_registered(args: &[String]) -> std::io::Result<i32> {
    if !args.is_empty() {
        eprintln!("usage: {REGISTERED_USAGE}");
        return Ok(2);
    }
    super::print_response(&super::send_request(&Request {
        id: "cli:agent:registered".into(),
        method: Method::AgentRegistered(EmptyParams::default()),
    })?)
}
