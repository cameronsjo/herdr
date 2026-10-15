use clap::Command;

use crate::api::schema::tabs::{
    tab_id, TabCreateProjection, TabListProjection, TabRenameProjection,
};
use crate::api::schema::{Method, Request, TabMoveDestination, TabMoveParams, TabTarget};

use super::super::projection::{self, cli_projection, Field, Input, Leaf, ParseError};

cli_projection! {
    pub(crate) TabTargetProjection for TabTarget {
        tab_id => Field::new(String::new()).input(tab_id()),
    }
}

/// The options `tab move` accepts, before they resolve to one destination.
pub(crate) struct TabMoveArgs {
    tab_id: String,
    workspace_id: Option<String>,
    insert_index: Option<usize>,
    new_workspace: bool,
    label: Option<String>,
}

fn insert_index() -> Input<Option<usize>> {
    Input::value("index", "N", |slot, value| {
        *slot = Some(
            value
                .parse::<usize>()
                .map_err(|_| ParseError::Usage("--index must be a non-negative integer".into()))?,
        );
        Ok(())
    })
}

cli_projection! {
    pub(crate) TabMoveProjection for TabMoveArgs {
        tab_id => Field::new(String::new()).input(tab_id()),
        workspace_id => Field::new(None)
            .input(projection::optional_text("workspace", "WORKSPACE_ID")),
        insert_index => Field::new(None).input(insert_index()),
        new_workspace => Field::new(false).input(projection::boolean("new-workspace", true)),
        label => Field::new(None).input(projection::optional_text("label", "TEXT")),
    }
}

const TAB_MOVE_USAGE: &str = "usage: herdr tab move <tab_id> --index N\n       herdr tab move <tab_id> --workspace <workspace_id> [--index N]\n       herdr tab move <tab_id> --new-workspace [--label TEXT]";

/// The three destinations are mutually exclusive; say so rather than silently
/// preferring one, which would move the tab somewhere unasked.
fn tab_move_method(args: TabMoveArgs) -> Result<Method, ParseError> {
    let usage = || ParseError::Usage(TAB_MOVE_USAGE.into());
    let label_misplaced = || ParseError::Usage("--label applies only to --new-workspace".into());
    let destination = if args.new_workspace {
        if args.workspace_id.is_some() || args.insert_index.is_some() {
            return Err(usage());
        }
        TabMoveDestination::NewWorkspace { label: args.label }
    } else if let Some(workspace_id) = args.workspace_id {
        if args.label.is_some() {
            return Err(label_misplaced());
        }
        TabMoveDestination::Workspace {
            workspace_id,
            insert_index: args.insert_index,
        }
    } else {
        if args.label.is_some() {
            return Err(label_misplaced());
        }
        let Some(insert_index) = args.insert_index else {
            return Err(usage());
        };
        TabMoveDestination::Index { insert_index }
    };
    Ok(Method::tab_move(TabMoveParams {
        tab_id: args.tab_id,
        insert_index: None,
        destination: Some(destination),
    }))
}

const LEAVES: [Leaf; 7] = [
    Leaf {
        name: "list",
        about: "List tabs",
        id: "cli:tab:list",
        arguments: TabListProjection::arguments,
        method: |matches| TabListProjection::from_matches(matches).map(Method::TabList),
    },
    Leaf {
        name: "create",
        about: "Create a tab",
        id: "cli:tab:create",
        arguments: TabCreateProjection::arguments,
        method: |matches| TabCreateProjection::from_matches(matches).map(Method::TabCreate),
    },
    Leaf {
        name: "get",
        about: "Show a tab",
        id: "cli:tab:get",
        arguments: TabTargetProjection::arguments,
        method: |matches| TabTargetProjection::from_matches(matches).map(Method::TabGet),
    },
    Leaf {
        name: "focus",
        about: "Focus a tab",
        id: "cli:tab:focus",
        arguments: TabTargetProjection::arguments,
        method: |matches| TabTargetProjection::from_matches(matches).map(Method::TabFocus),
    },
    Leaf {
        name: "rename",
        about: "Rename a tab",
        id: "cli:tab:rename",
        arguments: TabRenameProjection::arguments,
        method: |matches| TabRenameProjection::from_matches(matches).map(Method::TabRename),
    },
    Leaf {
        name: "move",
        about: "Move a tab within its workspace, to another workspace, or to a new one",
        id: "cli:tab:move",
        arguments: TabMoveProjection::arguments,
        method: |matches| TabMoveProjection::from_matches(matches).and_then(tab_move_method),
    },
    Leaf {
        name: "close",
        about: "Close a tab",
        id: "cli:tab:close",
        arguments: TabTargetProjection::arguments,
        method: |matches| TabTargetProjection::from_matches(matches).map(Method::TabClose),
    },
];

pub(crate) fn command() -> Command {
    Command::new("tab")
        .about("Manage tabs over the socket API")
        .subcommand_required(true)
        .subcommands(LEAVES.iter().map(Leaf::command))
}

pub(crate) fn request(args: &[String]) -> Result<Request, ParseError> {
    projection::group_request(command(), &LEAVES, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Request, ParseError> {
        request(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
    }

    #[test]
    fn tab_projection_preserves_requests_defaults_and_maps() {
        for (args, expected) in [
            (
                vec!["list"],
                serde_json::json!({"id":"cli:tab:list","method":"tab.list","params":{}}),
            ),
            (
                vec!["create"],
                serde_json::json!({"id":"cli:tab:create","method":"tab.create","params":{"focus":false}}),
            ),
            (
                vec![
                    "create",
                    "--workspace",
                    "2",
                    "--cwd",
                    "relative",
                    "--label",
                    "first",
                    "--label",
                    "last",
                    "--focus",
                    "--no-focus",
                    "--env",
                    "K=first",
                    "--env",
                    "K=",
                    "--env",
                    "V=a=b",
                ],
                serde_json::json!({"id":"cli:tab:create","method":"tab.create","params":{"workspace_id":"2","cwd":"relative","label":"last","focus":false,"env":{"K":"","V":"a=b"}}}),
            ),
            (
                vec!["rename", "2", "two", "--words", "here"],
                serde_json::json!({"id":"cli:tab:rename","method":"tab.rename","params":{"tab_id":"2","label":"two --words here"}}),
            ),
            (
                vec!["get", "2"],
                serde_json::json!({"id":"cli:tab:get","method":"tab.get","params":{"tab_id":"2"}}),
            ),
            (
                vec!["focus", "2"],
                serde_json::json!({"id":"cli:tab:focus","method":"tab.focus","params":{"tab_id":"2"}}),
            ),
            (
                vec!["close", "tab-2"],
                serde_json::json!({"id":"cli:tab:close","method":"tab.close","params":{"tab_id":"tab-2"}}),
            ),
        ] {
            assert_eq!(
                serde_json::to_value(parse(&args).unwrap()).unwrap(),
                expected,
                "{args:?}"
            );
        }
        assert!(
            matches!(parse(&["create", "--env", "=value"]), Err(ParseError::Usage(message)) if message == "env key must not be empty")
        );
    }

    #[test]
    fn tab_projection_clap_grammar_and_ordered_focus() {
        let value = serde_json::to_value(
            parse(&[
                "create",
                "--focus",
                "--no-focus",
                "--focus",
                "--cwd=relative",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            value["params"],
            serde_json::json!({"cwd":"relative", "focus":true})
        );
        assert!(
            matches!(parse(&["get", "2", "--help"]), Err(ParseError::Clap(error)) if error.exit_code() == 0)
        );
        for args in [
            vec!["get"],
            vec!["get", "2", "extra"],
            vec!["create", "--cwd"],
        ] {
            assert!(matches!(parse(&args), Err(ParseError::Clap(error)) if error.exit_code() == 2));
        }
        command().debug_assert();
    }
}
