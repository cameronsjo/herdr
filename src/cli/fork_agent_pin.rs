//! Fork-only `herdr agent pin|unpin` commands and `agent start --pin`.
//! See `crate::fork_agent_pin`.

use clap::{Arg, ArgAction, Command};

use crate::api::schema::{AgentTarget, Method, Request};

pub(super) const PIN_USAGE: &str = "herdr agent pin <TARGET>|--current";
pub(super) const UNPIN_USAGE: &str = "herdr agent unpin <TARGET>|--current";

/// The clap shape of these commands, for `herdr agent --help` and completions.
/// Parsing stays with the functions below.
pub(super) fn commands() -> [Command; 2] {
    let command = |name: &'static str, about: &'static str, usage: &'static str| {
        Command::new(name)
            .about(about)
            .override_usage(usage)
            .arg(Arg::new("target").value_name("TARGET"))
            .arg(
                Arg::new("current")
                    .long("current")
                    .action(ArgAction::SetTrue)
                    .help("Target the agent in this pane ($HERDR_PANE_ID)"),
            )
    };
    [
        command(
            "pin",
            "Pin an agent to the top of the agents panel",
            PIN_USAGE,
        ),
        command(
            "unpin",
            "Return a pinned agent to its usual place in the agents panel",
            UNPIN_USAGE,
        ),
    ]
}

/// The `--pin` flag on `herdr agent start`, for help and completions.
pub(super) fn start_pin_arg() -> Arg {
    Arg::new("pin")
        .long("pin")
        .action(ArgAction::SetTrue)
        .help("Pin the agent to the top of the agents panel once it is ready")
}

/// Removes `--pin` from `herdr agent start` arguments, returning whether it was
/// there. Arguments after `--` belong to the agent and are left alone.
pub(super) fn take_start_pin(args: &[String]) -> (Vec<String>, bool) {
    let mut pin = false;
    let mut passthrough = false;
    let kept = args
        .iter()
        .filter(|arg| {
            if passthrough {
                return true;
            }
            if arg.as_str() == "--" {
                passthrough = true;
                return true;
            }
            if arg.as_str() == "--pin" {
                pin = true;
                return false;
            }
            true
        })
        .cloned()
        .collect();
    (kept, pin)
}

/// Runs `herdr agent pin` (`pin = true`) or `herdr agent unpin`.
pub(super) fn agent_pin(args: &[String], pin: bool) -> std::io::Result<i32> {
    let usage = if pin { PIN_USAGE } else { UNPIN_USAGE };
    let target = match parse_target(args, super::target::caller_pane_id()) {
        Ok(target) => target,
        Err(message) => {
            eprintln!("{message}\nusage: {usage}");
            return Ok(2);
        }
    };
    super::print_response(&super::send_request(&pin_request(target, pin))?)
}

pub(super) fn pin_request(target: String, pin: bool) -> Request {
    let target = AgentTarget { target };
    if pin {
        Request {
            id: "cli:agent:pin".into(),
            method: Method::AgentPin(target),
        }
    } else {
        Request {
            id: "cli:agent:unpin".into(),
            method: Method::AgentUnpin(target),
        }
    }
}

fn parse_target(args: &[String], caller: Option<String>) -> Result<String, String> {
    match args {
        [flag] if flag == "--current" => {
            caller.ok_or_else(|| "--current requires HERDR_PANE_ID".to_owned())
        }
        [target] if !target.starts_with('-') => Ok(target.clone()),
        [] => Err("missing agent target".to_owned()),
        _ => Err("expected one agent target or --current".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn target_is_one_operand_or_the_current_pane() {
        assert_eq!(parse_target(&strings(&["orch"]), None), Ok("orch".into()));
        assert_eq!(
            parse_target(&strings(&["--current"]), Some("w1:p2".into())),
            Ok("w1:p2".into())
        );
        assert!(parse_target(&strings(&["--current"]), None).is_err());
        assert!(parse_target(&[], Some("w1:p2".into())).is_err());
        assert!(parse_target(&strings(&["a", "b"]), None).is_err());
        assert!(parse_target(&strings(&["--bogus"]), None).is_err());
    }

    #[test]
    fn start_pin_is_taken_only_before_the_agent_arguments() {
        let (kept, pin) = take_start_pin(&strings(&[
            "start", "orch", "--pin", "--kind", "claude", "--", "--pin",
        ]));
        assert!(pin);
        assert_eq!(
            kept,
            strings(&["start", "orch", "--kind", "claude", "--", "--pin"])
        );

        let (kept, pin) = take_start_pin(&strings(&["start", "orch", "--", "--pin"]));
        assert!(!pin);
        assert_eq!(kept, strings(&["start", "orch", "--", "--pin"]));
    }

    #[test]
    fn pin_and_unpin_send_their_own_methods() {
        assert!(matches!(
            pin_request("orch".into(), true).method,
            Method::AgentPin(AgentTarget { ref target }) if target == "orch"
        ));
        assert!(matches!(
            pin_request("orch".into(), false).method,
            Method::AgentUnpin(_)
        ));
    }
}
