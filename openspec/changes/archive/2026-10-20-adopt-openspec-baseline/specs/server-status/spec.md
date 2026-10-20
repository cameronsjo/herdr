## Purpose
Server status covers how the fork's CLI tells a dead server apart from a wedged one, how it explains a mismatch between upstream and fork builds, and how the internal `server live-handoff` command is exposed.

## ADDED Requirements

### Requirement: Three distinct server-unavailable errors
When a CLI command cannot reach the API socket it SHALL report one of three codes. `server_not_running` (no socket, or a stale one with no live server) MUST keep the remedy of running `herdr`. `server_api_not_accepting` MUST be reported when the API socket refuses connections while the paired client socket answers, and its message MUST say not to run `herdr` there. `server_api_not_responding` MUST be reported when the handshake ping gets no reply within 10 seconds.

#### Scenario: No server at all
- **WHEN** no herdr server is running and `herdr workspace list` is run
- **THEN** the error code is `server_not_running` and the message says to run `herdr`

#### Scenario: Wedged API listener
- **WHEN** the API socket refuses connections but the server still answers on its client socket
- **THEN** the error code is `server_api_not_accepting` and the message says not to run `herdr` here

#### Scenario: Silent handshake
- **WHEN** the API socket accepts a connection but never answers the handshake ping
- **THEN** after 10 seconds the error code is `server_api_not_responding`

### Requirement: The handshake probe is bounded
The protocol handshake probe SHALL use a 10-second timeout, longer than the server's own 5-second initial request timeout, so a merely slow server is not reported as not responding. The bound MUST hold on every platform, including Windows named pipes where socket timeouts are a no-op.

#### Scenario: Slow but healthy server
- **WHEN** the server takes 6 seconds to answer the handshake ping
- **THEN** the command proceeds and no `server_api_not_responding` error is reported

### Requirement: herdr status reports a server that is not accepting
`herdr status` and `herdr status server` SHALL report a server whose API socket refuses connections while its client socket answers as `status: running but not accepting api connections` in text output and `status: "not_accepting"` with `running: false` in JSON output, so scripts that read `running` keep treating it as unusable.

#### Scenario: JSON status of a wedged server
- **WHEN** `herdr status server --json` runs against a server whose API listener refuses connections
- **THEN** the output has `status` `not_accepting` and `running` `false`

### Requirement: Fork builds carry a protocol offset
The fork's wire protocol version SHALL be 1000 plus ten times upstream's protocol version plus the fork's own revision (1220 on upstream 22), so a fork build is told apart from an upstream build by number. When client and server protocols differ and exactly one carries the offset, the `protocol_mismatch` message MUST add that they may be different herdr distributions, or a fork build from before the offset existed.

#### Scenario: Fork client against an upstream server
- **WHEN** a fork CLI with protocol 1220 talks to an upstream server with protocol 22
- **THEN** the `protocol_mismatch` message says the two may be different herdr distributions

### Requirement: Cross-distribution live handoff warns
A live handoff SHALL log the source build's protocol and version on every accepted handoff, and MUST log a warning when exactly one of the source and receiving builds carries the fork protocol offset, saying state only the source build models may not carry over. It MUST NOT refuse the handoff on a protocol difference alone, since protocols differ on every in-place upgrade.

#### Scenario: Upstream server handed to a fork build
- **WHEN** a live handoff runs from an upstream build to a fork build
- **THEN** the handoff proceeds and a warning names both protocols and versions

### Requirement: live-handoff is listed in help but not completed
`herdr server --help` SHALL list the `live-handoff` subcommand and `herdr server live-handoff --help` MUST succeed. Shell completion scripts MUST NOT offer `live-handoff`, since it is an internal command driven by `herdr update`.

#### Scenario: Server help
- **WHEN** `herdr server --help` is run
- **THEN** the output lists `live-handoff`

#### Scenario: Completions
- **WHEN** completion scripts are generated for any supported shell
- **THEN** none of them contains `live-handoff`
