//! Fork-only handlers for `agent.register`, `agent.unregister`, and
//! `agent.registered`. The registry itself is `crate::fork_registered_agents`.

use std::time::Instant;

use crate::api::schema::{AgentRegisterParams, AgentUnregisterParams, ResponseResult};
use crate::app::App;

use super::responses::{encode_error, encode_success};

impl App {
    pub(super) fn handle_agent_register(
        &mut self,
        id: String,
        params: AgentRegisterParams,
    ) -> String {
        let revision = self.state.registered_agents.revision();
        match self
            .state
            .registered_agents
            .register(params, Instant::now())
        {
            Ok(registered_agent) => {
                self.registered_agents_changed(revision);
                encode_success(id, ResponseResult::RegisteredAgent { registered_agent })
            }
            Err(err) => encode_error(id, err.code, err.message),
        }
    }

    pub(super) fn handle_agent_unregister(
        &mut self,
        id: String,
        params: AgentUnregisterParams,
    ) -> String {
        let revision = self.state.registered_agents.revision();
        let removed = self
            .state
            .registered_agents
            .unregister(&params.source, &params.name);
        self.registered_agents_changed(revision);
        encode_success(id, ResponseResult::RegisteredAgentRemoved { removed })
    }

    pub(super) fn handle_agent_registered(&mut self, id: String) -> String {
        self.expire_registered_agents(Instant::now());
        encode_success(
            id,
            ResponseResult::RegisteredAgentList {
                registered_agents: self.state.registered_agents.list(),
            },
        )
    }

    pub(crate) fn expire_registered_agents(&mut self, now: Instant) {
        let revision = self.state.registered_agents.revision();
        self.state.registered_agents.expire(now);
        self.registered_agents_changed(revision);
    }

    /// Re-arms the expiry timer, and redraws clients when the visible set moved.
    fn registered_agents_changed(&mut self, previous_revision: u64) {
        self.sync_agent_metadata_deadline();
        if self.state.registered_agents.revision() != previous_revision {
            self.render_dirty.request_generic();
            self.render_notify.notify_one();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::schema::{
        AgentRegisterParams, AgentStatus, AgentUnregisterParams, Method, Request,
    };

    fn app_for_api_tests() -> crate::app::App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        crate::app::App::new(
            &crate::config::Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        )
    }

    fn register(name: &str, status: AgentStatus) -> Request {
        Request {
            id: format!("register:{name}"),
            method: Method::AgentRegister(AgentRegisterParams {
                source: "test".into(),
                name: name.into(),
                agent: Some("claude".into()),
                status: Some(status),
                cwd: Some("/work/cadence".into()),
                tokens: Default::default(),
                ttl_ms: None,
            }),
        }
    }

    fn call(app: &mut crate::app::App, request: Request) -> serde_json::Value {
        serde_json::from_str(&app.handle_api_request(request)).unwrap()
    }

    #[test]
    fn register_list_and_unregister_round_trip() {
        let mut app = app_for_api_tests();

        let registered = call(&mut app, register("one", AgentStatus::Working));
        assert_eq!(registered["result"]["type"], "registered_agent");
        assert_eq!(
            registered["result"]["registered_agent"]["status"],
            "working"
        );
        assert_eq!(registered["result"]["registered_agent"]["ttl_ms"], 60_000);
        assert!(app.agent_metadata_deadline.is_some());

        let listed = call(
            &mut app,
            Request {
                id: "list".into(),
                method: Method::AgentRegistered(Default::default()),
            },
        );
        assert_eq!(listed["result"]["type"], "registered_agent_list");
        assert_eq!(listed["result"]["registered_agents"][0]["name"], "one");

        let removed = call(
            &mut app,
            Request {
                id: "unregister".into(),
                method: Method::AgentUnregister(AgentUnregisterParams {
                    source: "test".into(),
                    name: "one".into(),
                }),
            },
        );
        assert_eq!(removed["result"]["removed"], true);
        assert!(app.state.registered_agents.list().is_empty());
    }

    #[test]
    fn invalid_register_is_an_error() {
        let mut app = app_for_api_tests();
        let mut request = register("one", AgentStatus::Idle);
        if let Method::AgentRegister(params) = &mut request.method {
            params.ttl_ms = Some(0);
        }
        let response = call(&mut app, request);
        assert_eq!(response["error"]["code"], "invalid_params");
    }

    #[test]
    fn expiry_runs_with_the_metadata_timer() {
        let mut app = app_for_api_tests();
        call(&mut app, register("one", AgentStatus::Idle));
        let deadline = app.agent_metadata_deadline.unwrap();
        app.expire_metadata_at(deadline, deadline);
        assert!(app.state.registered_agents.list().is_empty());
        assert_eq!(app.agent_metadata_deadline, None);
    }
}
