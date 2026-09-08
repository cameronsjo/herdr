use super::*;

pub(super) fn print_grouped_agent_profiles() {
    for (label, build) in [
        ("background", workspaces as fn(usize) -> Vec<Workspace>),
        ("active", active_panes),
    ] {
        for grouped in [false, true] {
            let mut config = Config::default();
            if grouped {
                config.ui.sidebar.agents.group_by = crate::config::AgentGroupBy::Workspace;
            }
            let rows = [1, 15].map(|count| {
                let mut pipeline = RenderPipeline::with_config(build(count), &config);
                pipeline.app.state.ensure_test_terminals();
                for terminal in pipeline.app.state.terminals.values_mut() {
                    terminal.detected_agent = Some(crate::detect::Agent::Pi);
                }
                pipeline
                    .client
                    .set_snapshot(Box::new(super::super::client_shell::snapshot(
                        &pipeline.app,
                        "bench-boot",
                        1,
                        None,
                        None,
                    )));
                (count, profile_pipeline(pipeline))
            });
            println!("fork agent grouping {label}: populated agents, grouped={grouped}");
            print_stage("client shell composition", &rows, |stats| stats.client);
        }
    }
}
