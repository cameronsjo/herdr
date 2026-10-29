## ADDED Requirements

### Requirement: send-text can send a bracketed paste
`pane.send_text` SHALL accept an optional boolean `paste`, set by `herdr pane send-text --paste` before the pane id. When `paste` is `true` and the pane has enabled bracketed-paste mode, the text MUST be written wrapped in `ESC [200~` and `ESC [201~`; when the pane has not enabled it, the text MUST be written raw. When `paste` is absent or `false`, the text MUST be written raw, and a request serialized without it MUST NOT carry the field.

#### Scenario: Pane accepts paste
- **WHEN** a pane has enabled bracketed paste and `herdr pane send-text --paste <pane> "one two"` is run
- **THEN** the pane receives `ESC [200~one two ESC [201~`

#### Scenario: Pane does not accept paste
- **WHEN** a pane has not enabled bracketed paste and `pane.send_text` is called with `paste: true`
- **THEN** the pane receives the text raw

#### Scenario: Flag after the text
- **WHEN** `herdr pane send-text <pane> hello --paste` is run
- **THEN** the text sent is `hello --paste` and `paste` is not set

### Requirement: A paste refuses ESC
A `pane.send_text` request with `paste: true` whose text contains ESC MUST be refused with `invalid_paste_text` and nothing written, so the text cannot end the paste early. Without `paste`, escapes MUST still pass through raw.

#### Scenario: Paste text carries the end marker
- **WHEN** `pane.send_text` is called with `paste: true` and text `a ESC [201~ b`
- **THEN** the response is the error `invalid_paste_text` and the pane receives nothing
