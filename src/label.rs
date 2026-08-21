//! Sanitizing for user-supplied workspace, tab, and pane labels.
//!
//! Labels are rendered straight into the sidebar, tab bar, and pane headers,
//! and ratatui writes grapheme symbols to the host terminal verbatim. Anything
//! a label carries, the terminal executes or draws.
//!
//! The filter lives here — and is applied inside the label *setters* rather
//! than at each API handler — because per-call-site sanitizing left four
//! handlers and the whole session-restore path unfiltered. A future handler
//! that reaches a setter cannot miss it.

/// Longest label kept, in `char`s.
///
/// The renderer truncates by *display width*, so a label that occupies four
/// columns can still carry unbounded codepoints (combining marks stack inside
/// a single grapheme) into storage, the session file, and every event echo.
/// This bounds what gets stored, not what gets drawn.
pub(crate) const MAX_LABEL_CHARS: usize = 128;

/// Strips control and format characters from a label and bounds its length.
///
/// Two Unicode categories go:
///
/// - `Cc` (control) — `ESC`, `\n`, `\r`, `BEL` and the C1 range. These corrupt
///   the surrounding frame.
/// - `Cf` (format) — bidi overrides and isolates (`U+202A`–`U+202E`,
///   `U+2066`–`U+2069`) and the zero-width joiner/space. These are zero-width,
///   so they survive width-based truncation and let one label visually reorder
///   adjacent text or render identically to another.
///
/// Characters that render blank but sit outside `Cc`/`Cf` — Hangul filler
/// (`U+3164`), blank braille (`U+2800`) — pass on purpose: this filter targets
/// control and format characters, and the 128-char cap bounds the rest.
///
/// Multibyte text is deliberately untouched: labels are routinely CJK, so this
/// must never become an ASCII filter.
pub(crate) fn sanitize_label(label: impl Into<String>) -> String {
    label
        .into()
        .chars()
        .filter(|ch| !ch.is_control() && !is_format_char(*ch))
        .take(MAX_LABEL_CHARS)
        .collect()
}

/// [`sanitize_label`] for render paths: borrows when the input is already
/// clean and within the cap, so the common case costs one scan and no
/// allocation. Pane-scaled render loops call this per row per frame.
pub(crate) fn sanitize_label_borrowed(label: &str) -> std::borrow::Cow<'_, str> {
    let mut chars = 0usize;
    let clean = label.chars().all(|ch| {
        chars += 1;
        chars <= MAX_LABEL_CHARS && !ch.is_control() && !is_format_char(ch)
    });
    if clean {
        std::borrow::Cow::Borrowed(label)
    } else {
        std::borrow::Cow::Owned(sanitize_label(label))
    }
}

/// Unicode general category `Cf`.
///
/// Spelled out rather than pulled from a table crate: the set is small, stable,
/// and adding a dependency for one predicate is not worth it.
fn is_format_char(ch: char) -> bool {
    matches!(ch,
        '\u{00AD}'                      // soft hyphen
        | '\u{0600}'..='\u{0605}'
        | '\u{061C}'                    // arabic letter mark
        | '\u{06DD}'
        | '\u{070F}'
        | '\u{0890}'..='\u{0891}'
        | '\u{08E2}'
        | '\u{180E}'
        | '\u{200B}'..='\u{200F}'       // zero-width space/joiner, LRM/RLM
        | '\u{202A}'..='\u{202E}'       // bidi embedding/override
        | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{206F}'       // bidi isolates
        | '\u{FEFF}'                    // zero-width no-break space
        | '\u{FFF9}'..='\u{FFFB}'
        | '\u{110BD}'
        | '\u{110CD}'
        | '\u{13430}'..='\u{1343F}'     // egyptian hieroglyph format controls
        | '\u{1BCA0}'..='\u{1BCA3}'     // shorthand format controls
        | '\u{1D173}'..='\u{1D17A}'
        | '\u{E0001}'
        | '\u{E0020}'..='\u{E007F}'     // tag characters
    )
}

#[cfg(test)]
mod tests {
    use super::{sanitize_label, sanitize_label_borrowed, MAX_LABEL_CHARS};

    #[test]
    fn strips_control_characters() {
        assert_eq!(sanitize_label("stat\u{1b}[31mus\nboard"), "stat[31musboard");
        assert_eq!(sanitize_label("bell\u{7}"), "bell");
    }

    #[test]
    fn strips_bidi_and_zero_width_format_characters() {
        // A right-to-left override can visually reverse adjacent tab-bar text.
        assert_eq!(sanitize_label("safe\u{202e}gnp.exe"), "safegnp.exe");
        // Zero-width characters let two workspaces render identically.
        assert_eq!(sanitize_label("de\u{200b}ploy"), "deploy");
        assert_eq!(sanitize_label("de\u{200d}ploy"), "deploy");
        assert_eq!(sanitize_label("\u{feff}lead"), "lead");
    }

    #[test]
    fn strips_format_characters_missing_from_the_original_table() {
        // One character from each range the original table missed.
        assert_eq!(sanitize_label("a\u{0890}b"), "ab");
        assert_eq!(sanitize_label("a\u{110CD}b"), "ab");
        assert_eq!(sanitize_label("a\u{13430}b"), "ab");
        assert_eq!(sanitize_label("a\u{1BCA0}b"), "ab");
    }

    #[test]
    fn borrowed_sanitizer_only_allocates_when_it_has_to() {
        assert!(matches!(
            sanitize_label_borrowed("feat/中文"),
            std::borrow::Cow::Borrowed("feat/中文")
        ));
        assert_eq!(sanitize_label_borrowed("a\u{202e}b").as_ref(), "ab");
        let long = "z".repeat(MAX_LABEL_CHARS + 1);
        assert_eq!(
            sanitize_label_borrowed(&long).chars().count(),
            MAX_LABEL_CHARS
        );
    }

    #[test]
    fn keeps_multibyte_text_intact() {
        // Labels are routinely CJK — this must never become an ASCII filter.
        assert_eq!(sanitize_label("提交 herdr 的反馈"), "提交 herdr 的反馈");
        assert_eq!(sanitize_label("café"), "café");
        assert_eq!(sanitize_label("emoji 🎛 ok"), "emoji 🎛 ok");
    }

    #[test]
    fn bounds_length_so_storage_cannot_be_flooded() {
        // Combining marks stack within one grapheme, so display-width
        // truncation alone leaves the stored label unbounded.
        let bomb: String = std::iter::once('a')
            .chain(std::iter::repeat_n('\u{0301}', 5_000))
            .collect();
        assert_eq!(sanitize_label(bomb).chars().count(), MAX_LABEL_CHARS);
    }

    #[test]
    fn leaves_ordinary_labels_alone() {
        assert_eq!(sanitize_label("main"), "main");
        assert_eq!(sanitize_label(""), "");
    }
}
