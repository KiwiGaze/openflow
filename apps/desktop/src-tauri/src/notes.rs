//! Scratchpad notes: the opt-in writing surface backed by the shared SQLite
//! store (`db.rs`). Text only — like history, **never audio**. The note body is
//! stored as the minimal HTML the Scratchpad toolbar produces (`<p>`, `<b>`,
//! `<i>`, `<u>`, `<code>`, `<ul>`/`<ol>`/`<li>`, `<br>`); paste is forced to
//! plain text in the webview so stored markup can only ever be our own tags.
//!
//! This module holds the IPC-facing shapes (serialized camelCase; the TS mirror
//! is `packages/core/src/types.ts`) plus the two pure conversions the transform
//! path needs: HTML → plain text (for the LLM input and list previews) and
//! plain text → minimal HTML (the LLM returns text; we re-wrap it as paragraphs).

use serde::Serialize;

/// Per-note version cap. The newest this many are kept; older ones drop on add.
pub const NOTE_VERSION_CAP: i64 = 50;

/// Hard limit on a note body, in bytes. A larger update is rejected (never
/// silently truncated — that would lose the user's words).
pub const MAX_NOTE_CONTENT_BYTES: usize = 1_000_000;

/// Characters of preview text shown in the note list (after tags are stripped).
const PREVIEW_CHARS: usize = 120;

/// A full note for the editor. Non-deleted only; the body is stored HTML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    /// Unix epoch milliseconds.
    pub created_at: i64,
    pub updated_at: i64,
    pub pinned: bool,
}

/// A note row for the list: enough to render without loading the full body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    /// First ~120 characters of the body with tags stripped.
    pub preview: String,
    pub updated_at: i64,
    pub pinned: bool,
}

/// One immutable snapshot of a note's body, taken before a destructive edit
/// (a transform or a restore) so the user can always walk back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVersion {
    pub id: String,
    pub note_id: String,
    pub content: String,
    /// Why the snapshot exists: "created", "transform", or "restore".
    pub source: String,
    /// The transform applied when `source` is "transform"; null otherwise (and
    /// for a Polish transform, which carries no settings transform id).
    pub transform_id: Option<String>,
    pub created_at: i64,
}

/// Plain text from the stored note HTML. A paragraph or `<div>` boundary
/// becomes a blank-line separator (`\n\n`); a `<br>` or a list item starts a new
/// line (`\n`); the stronger break wins where they meet, so adjacent paragraphs
/// give one blank line, never a pile. Named entities we emit are decoded back.
/// Deliberately small: the stored markup is only ever the toolbar's own tags,
/// not arbitrary HTML.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '<' => {
                // Read the tag name to decide whether (and how hard) it breaks.
                let mut name = String::new();
                let mut closed = false;
                for tag_ch in chars.by_ref() {
                    if tag_ch == '>' {
                        closed = true;
                        break;
                    }
                    name.push(tag_ch);
                }
                if !closed {
                    // A lone '<' with no '>' — treat it as literal text.
                    out.push('<');
                    out.push_str(&name);
                    continue;
                }
                if let Some(weight) = tag_break_weight(&name) {
                    push_break(&mut out, weight);
                }
            }
            '&' => {
                // Decode just the entities our text_to_html / browsers produce.
                let mut entity = String::new();
                let mut terminated = false;
                while let Some(&peek) = chars.peek() {
                    if peek == ';' {
                        chars.next();
                        terminated = true;
                        break;
                    }
                    if entity.len() >= 6 || !peek.is_ascii_alphanumeric() {
                        break;
                    }
                    entity.push(peek);
                    chars.next();
                }
                match (terminated, entity.as_str()) {
                    (true, "amp") => out.push('&'),
                    (true, "lt") => out.push('<'),
                    (true, "gt") => out.push('>'),
                    (true, "quot") => out.push('"'),
                    (true, "nbsp") => out.push(' '),
                    // Unknown or unterminated — keep the literal characters.
                    _ => {
                        out.push('&');
                        out.push_str(&entity);
                        if terminated {
                            out.push(';');
                        }
                    }
                }
            }
            _ => out.push(ch),
        }
    }
    trim_breaks(&out)
}

/// A single-line preview for the note list: the stripped body, whitespace
/// collapsed, truncated on a char boundary to `PREVIEW_CHARS`.
pub fn preview(html: &str) -> String {
    let plain = strip_tags(html);
    let collapsed = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= PREVIEW_CHARS {
        collapsed
    } else {
        let truncated: String = collapsed.chars().take(PREVIEW_CHARS).collect();
        format!("{truncated}…")
    }
}

/// Minimal HTML from plain text — the inverse mapping for transform output. The
/// LLM returns plain text; blank-line-separated blocks become `<p>` paragraphs
/// and single newlines inside a block become `<br>`. Text is HTML-escaped so the
/// result can be re-rendered via innerHTML safely. Empty input yields an empty
/// string (an empty note, not `<p></p>`).
pub fn text_to_html(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let blocks: Vec<&str> = normalized
        .split("\n\n")
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .collect();
    blocks
        .iter()
        .map(|block| {
            let lines: Vec<String> = block.lines().map(escape_html).collect();
            format!("<p>{}</p>", lines.join("<br>"))
        })
        .collect::<String>()
}

/// How hard a tag breaks the line in the plain-text projection: `Some(2)` for a
/// paragraph or `<div>` boundary (a blank-line separator), `Some(1)` for a
/// `<br>` or list item (a new line), `None` for inline tags (`b`/`i`/`u`/`code`)
/// and the `ul`/`ol` containers (their items carry the breaks).
fn tag_break_weight(raw: &str) -> Option<usize> {
    let name: String = raw
        .trim()
        .trim_start_matches('/')
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    match name.as_str() {
        "p" | "div" => Some(2),
        "br" | "li" => Some(1),
        _ => None,
    }
}

/// Ensures `out` ends with at least `weight` newlines (never more than 2), so a
/// stronger paragraph break is not downgraded by a following weaker one and
/// runs never pile up. A no-op while `out` is empty, so no leading blank lines.
fn push_break(out: &mut String, weight: usize) {
    if out.is_empty() {
        return;
    }
    // Trailing spaces before a break are noise; drop them first.
    while out.ends_with(' ') || out.ends_with('\t') {
        out.pop();
    }
    let existing = out.chars().rev().take_while(|&c| c == '\n').count();
    for _ in existing..weight.min(2) {
        out.push('\n');
    }
}

fn escape_html(line: &str) -> String {
    line.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Trims the result: drops the leading/trailing newlines the outermost block
/// tags leave behind, and any trailing whitespace on the final line. Internal
/// breaks are already bounded by `push_break`, so no run-collapsing is needed.
fn trim_breaks(text: &str) -> String {
    text.trim_matches(|c: char| c == '\n' || c == ' ' || c == '\t')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_inline_and_block_tags_to_plain_text() {
        assert_eq!(
            strip_tags("<p>Hello <b>bold</b> and <i>italic</i></p>"),
            "Hello bold and italic"
        );
        // Two paragraphs are separated by a blank line.
        assert_eq!(strip_tags("<p>First</p><p>Second</p>"), "First\n\nSecond");
        // An explicit break splits a line (single newline).
        assert_eq!(strip_tags("line one<br>line two"), "line one\nline two");
    }

    #[test]
    fn strips_nested_lists_one_item_per_line() {
        // List items break with a single newline; the surrounding paragraph
        // boundary does not inflate the gap between adjacent lists.
        let html = "<ul><li>alpha</li><li>beta</li></ul><ol><li>one</li><li>two</li></ol>";
        assert_eq!(strip_tags(html), "alpha\nbeta\none\ntwo");
    }

    #[test]
    fn decodes_known_entities_and_leaves_no_markup() {
        let plain = strip_tags("<p>a &amp; b &lt;tag&gt; &quot;q&quot;&nbsp;end</p>");
        assert_eq!(plain, "a & b <tag> \"q\" end");
        // No angle brackets from tags survive.
        assert!(!strip_tags("<p><code>x</code></p>").contains('<'));
    }

    #[test]
    fn preview_collapses_whitespace_and_truncates_on_char_boundary() {
        assert_eq!(preview("<p>hello   world</p>"), "hello world");
        let long = format!("<p>{}</p>", "a".repeat(200));
        let p = preview(&long);
        assert_eq!(p.chars().count(), PREVIEW_CHARS + 1); // 120 chars + the ellipsis
        assert!(p.ends_with('…'));
    }

    #[test]
    fn text_to_html_splits_paragraphs_and_escapes() {
        assert_eq!(text_to_html("one\n\ntwo"), "<p>one</p><p>two</p>");
        assert_eq!(text_to_html("a\nb"), "<p>a<br>b</p>");
        assert_eq!(text_to_html("a & b < c"), "<p>a &amp; b &lt; c</p>");
        assert_eq!(text_to_html("   "), "");
    }

    #[test]
    fn text_to_html_then_strip_round_trips_the_text() {
        // The plain text survives a round trip through the HTML mapping (modulo
        // the paragraph/line structure, which strip_tags reproduces as newlines).
        let text = "First paragraph.\n\nSecond paragraph\nwith a break.";
        let html = text_to_html(text);
        assert_eq!(strip_tags(&html), text);
    }
}
