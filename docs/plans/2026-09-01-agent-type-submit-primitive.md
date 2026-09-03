# Agent Type-Submit Primitive

## Goal

Add a generic Herdr input primitive that submits literal UI text as typed terminal input, for harness commands that are not model-invocable prompts. The motivating case is `/compact`: callers need Herdr to type the command into the current agent UI and press Enter, rather than sending it through `agent prompt` as a normal user prompt.

## Chosen approach

Herdr will add `agent type-submit <target> <text>` as a generic agent-control command. It will resolve the target the same way `agent prompt` and `agent send-keys` do, verify that the target still hosts a recognized foreground agent, encode the text as key/text input suitable for interactive UI command entry, append Enter, and send the bytes as one operation.

The command stays generic: it does not know about `/compact`, `/journal`, or named recipes. Personal recipe sequencing belongs in forgectl.

## Requirements

- `herdr agent type-submit <target> <text>` MUST send literal UI text followed by Enter to the resolved live agent.
- The command MUST reject unknown, stale, or not-ready agent targets using the same target-safety rules as existing `agent send-keys`.
- The command MUST validate the whole text before writing any bytes, so unsupported input cannot partially type.
- The command SHOULD preserve bracketed-paste safety for text where appropriate, but MUST behave like interactive command entry for harness commands such as `/compact`.
- The command MUST not introduce any recipe-specific behavior in Herdr core.
- Documentation MUST describe when to use `agent prompt`, `agent send-keys`, and `agent type-submit`.

## Alternatives declined

- Use `agent prompt` for `/compact`: declined because `/compact` is not a model-invocable prompt in some harnesses, so this does not reliably exercise the UI command path.
- Add `herdr recipe afk`: declined because Herdr should expose reusable terminal/runtime primitives, while personal named routines belong in forgectl.
- Require callers to spell every character with `agent send-keys`: declined because the user-facing API would be too awkward and would duplicate text-to-key validation at every caller.

## Implementation checklist

- [x] Add the API schema and CLI dispatch for `agent type-submit`.
- [x] Add an app handler that resolves agent targets and sends the encoded text plus Enter.
- [x] Add behavior tests for success and pre-write validation/refusal paths.
- [x] Update Herdr docs for the new primitive.
- [x] Run focused tests, then the agreed broader validation.

## Validation notes

- `PATH="$HOME/.cargo/bin:$PATH" cargo fmt` completed successfully.
- `lsp_diagnostics` over the edited Rust files reported zero primary LSP diagnostics.
- `PATH="$HOME/.cargo/bin:$PATH" SDKROOT="$(xcrun --show-sdk-path)" ZIG="$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig" cargo test agent_type_submit --no-fail-fast` was blocked before tests ran by the vendored libghostty-vt Zig build: undefined macOS symbols including `__availability_version_check`, `_abort`, and `_arc4random_buf`.
- A minimal Zig build reproduced the same failure under full Xcode 26.5 and passed with `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, so the blocker is local Zig/Xcode SDK selection rather than this feature's code.
- `PATH="$HOME/.cargo/bin:$PATH" SDKROOT="$(xcrun --show-sdk-path)" DEVELOPER_DIR=/Library/Developer/CommandLineTools ZIG="$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig" cargo test agent_type_submit --no-fail-fast` completed successfully: 2 matching tests passed.
- `PATH="$HOME/.cargo/bin:$PATH" DEVELOPER_DIR=/Library/Developer/CommandLineTools ZIG="$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig" mise x just aqua:nextest-rs/nextest/cargo-nextest -- just check` reached the suite and failed in `tests/live_handoff.rs`; the same `live_handoff_keeps_unmanaged_agent_name_bound_to_saved_session` failure reproduced on base `master` at `6ef60c32`, so it is not caused by this branch.
- `PATH="$HOME/.cargo/bin:$PATH" DEVELOPER_DIR=/Library/Developer/CommandLineTools ZIG="$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig" HERDR_UPDATE_API_SCHEMA=1 mise x just aqua:nextest-rs/nextest/cargo-nextest -- just test-one generated_protocol_schema_artifact_is_current` regenerated the stale API schema artifact and passed.
- `PATH="$HOME/.cargo/bin:$PATH" DEVELOPER_DIR=/Library/Developer/CommandLineTools ZIG="$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig" mise x just aqua:nextest-rs/nextest/cargo-nextest -- just ci 'all() - binary(live_handoff)'` completed successfully.
- `PATH="$HOME/.cargo/bin:$PATH" DEVELOPER_DIR=/Library/Developer/CommandLineTools ZIG="$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig" mise x just aqua:nextest-rs/nextest/cargo-nextest -- just windows-lint` completed successfully as part of the remaining-checks bundle.
- `export PATH="$HOME/.cargo/bin:$PATH" DEVELOPER_DIR=/Library/Developer/CommandLineTools ZIG="$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig" GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=tag.gpgSign GIT_CONFIG_VALUE_0=false && python3 -m unittest scripts.test_agent_detection_manifest_check scripts.test_changelog scripts.test_config_reference_check scripts.test_docs_translation_parity scripts.test_hermes_integration_asset scripts.test_package_windows_conpty scripts.test_preview scripts.test_unix_installer scripts.test_vendor_libghostty_vt scripts.test_vendor_portable_pty` completed successfully: 98 tests passed.
