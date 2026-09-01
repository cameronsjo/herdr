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

- [ ] Add the API schema and CLI dispatch for `agent type-submit`.
- [ ] Add an app handler that resolves agent targets and sends the encoded text plus Enter.
- [ ] Add behavior tests for success and pre-write validation/refusal paths.
- [ ] Update Herdr docs for the new primitive.
- [ ] Run focused tests, then the agreed broader validation.
