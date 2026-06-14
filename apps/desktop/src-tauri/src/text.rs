//! Deterministic text processing: transcript cleanup, rules-based polishing,
//! and personal-dictionary replacements. Everything here is pure and fast —
//! the optional LLM pass lives in `llm.rs`.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};

use crate::settings::{DictionaryEntry, Snippet};

/// Bracketed all-caps tags whisper.cpp emits for non-speech audio,
/// e.g. `[BLANK_AUDIO]`, `[MUSIC]`, `[ Silence ]`.
static BRACKET_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[[ _A-Za-z]{1,30}\]").expect("static regex"));

/// Parenthesized non-speech annotations from a known list, e.g. `(laughs)`.
static PAREN_TAG: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(
        r"\((?:laughs|laughter|laughing|music|applause|noise|silence|coughs|coughing|sighs|inaudible|unintelligible)\)",
    )
    .case_insensitive(true)
    .build()
    .expect("static regex")
});

static MUSIC_NOTES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[♪♫]+").expect("static regex"));

static MULTI_SPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[ \t]{2,}").expect("static regex"));

static SPACE_BEFORE_PUNCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r" +([,.!?;:])").expect("static regex"));

/// Strips transcriber artifacts and normalizes whitespace. Applied to every
/// transcript before insertion (and before any prompt transform).
pub fn clean_transcript(raw: &str) -> String {
    let text = BRACKET_TAG.replace_all(raw, " ");
    let text = PAREN_TAG.replace_all(&text, " ");
    let text = MUSIC_NOTES.replace_all(&text, " ");
    normalize_whitespace(&text)
}

/// Single-pass, whole-word, case-insensitive replacement of many phrases at
/// once. Shared by the dictionary and snippet expanders. Returns the rewritten
/// text and the number of replacements applied. Two properties matter:
///
/// 1. **Longest wins.** Phrases compete longest-first, so "open flow" beats a
///    bare "flow" at the same span.
/// 2. **No cascading.** Matches are found in the original text only and each is
///    replaced verbatim; a replacement is never re-scanned, so one expansion
///    can never trigger another (e.g. a "cal" snippet can't fire inside the URL
///    that a "my cal" snippet just produced).
///
/// `pairs` is `(from, to)`; each `from` must be trimmed and non-empty.
fn replace_phrases(text: &str, pairs: &[(&str, &str)]) -> (String, usize) {
    let mut ordered: Vec<&(&str, &str)> = pairs.iter().filter(|(f, _)| !f.is_empty()).collect();
    if ordered.is_empty() {
        return (text.to_string(), 0);
    }
    // Longest first: with leftmost matching plus word boundaries, the longer of
    // two competing phrases is the one that matches its span.
    ordered.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));

    // `\b` only works against word characters; a phrase like "c++" ends on a
    // symbol, so the boundary is applied per edge of each alternative.
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let alternatives: Vec<String> = ordered
        .iter()
        .map(|(from, _)| {
            let lead = if from.chars().next().is_some_and(is_word) {
                r"\b"
            } else {
                ""
            };
            let tail = if from.chars().last().is_some_and(is_word) {
                r"\b"
            } else {
                ""
            };
            format!("{lead}{}{tail}", regex::escape(from))
        })
        .collect();
    let Ok(re) = RegexBuilder::new(&alternatives.join("|"))
        .case_insensitive(true)
        .build()
    else {
        return (text.to_string(), 0);
    };

    let lookup: HashMap<String, &str> = ordered
        .iter()
        .map(|(from, to)| (from.to_lowercase(), *to))
        .collect();
    let mut count = 0usize;
    let replaced = re
        .replace_all(text, |caps: &regex::Captures| {
            let matched = caps.get(0).map_or("", |m| m.as_str());
            count += 1;
            // Verbatim: return the replacement as an owned string so `$`-bearing
            // expansions (e.g. "$$$") are never treated as capture references.
            lookup
                .get(&matched.to_lowercase())
                .copied()
                .unwrap_or(matched)
                .to_string()
        })
        .into_owned();
    (replaced, count)
}

/// Applies personal-dictionary replacements with whole-word, case-insensitive
/// matching. Longer phrases win over shorter ones. Returns the rewritten text
/// and how many replacements were applied (the Insights "dictionary fixes"
/// counter), counting occurrences replaced, not entries that matched.
pub fn apply_dictionary(text: &str, entries: &[DictionaryEntry]) -> (String, usize) {
    let pairs: Vec<(&str, &str)> = entries
        .iter()
        .map(|e| (e.from.trim(), e.to.as_str()))
        .collect();
    replace_phrases(text, &pairs)
}

/// Normalizes a string for whole-utterance comparison: trims surrounding
/// whitespace and trailing sentence punctuation, then lowercases.
fn utterance_key(text: &str) -> String {
    text.trim()
        .trim_end_matches(['.', '!', '?'])
        .trim()
        .to_lowercase()
}

/// Expands snippet triggers into their (possibly multi-line) replacements,
/// inserted verbatim. A `whole_utterance` snippet fires only when the trigger
/// is the entire text and replaces all of it; the rest expand inline,
/// whole-word and case-insensitively, longest trigger first. Runs after the
/// dictionary so word fixes happen before phrase expansion.
pub fn apply_snippets(text: &str, snippets: &[Snippet]) -> String {
    // A whole-utterance snippet swallows the whole dictation when it matches,
    // so it is checked first and short-circuits. Trailing sentence punctuation
    // is ignored so cleanup adding a period ("my email" → "My email.") still
    // counts as "spoken alone".
    let spoken = utterance_key(text);
    for snippet in snippets {
        if snippet.whole_utterance {
            let trigger = snippet.trigger.trim();
            if !trigger.is_empty() && spoken == utterance_key(trigger) {
                return snippet.expansion.clone();
            }
        }
    }

    let pairs: Vec<(&str, &str)> = snippets
        .iter()
        .filter(|s| !s.whole_utterance)
        .map(|s| (s.trigger.trim(), s.expansion.as_str()))
        .collect();
    // Snippet expansion does not feed the dictionary-fixes counter.
    replace_phrases(text, &pairs).0
}

fn normalize_whitespace(text: &str) -> String {
    let text = MULTI_SPACE.replace_all(text, " ");
    let text = SPACE_BEFORE_PUNCT.replace_all(&text, "$1");
    // Normalize around newlines without destroying intentional breaks.
    let lines: Vec<String> = text.split('\n').map(|l| l.trim().to_string()).collect();
    let mut joined = lines.join("\n");
    while joined.contains("\n\n\n") {
        joined = joined.replace("\n\n\n", "\n\n");
    }
    joined.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(from: &str, to: &str) -> DictionaryEntry {
        DictionaryEntry {
            from: from.into(),
            to: to.into(),
        }
    }

    #[test]
    fn strips_whisper_artifacts() {
        assert_eq!(clean_transcript(" [BLANK_AUDIO] "), "");
        assert_eq!(
            clean_transcript("hello [MUSIC] world (laughs) again ♪"),
            "hello world again"
        );
        assert_eq!(clean_transcript("[ Silence ]"), "");
    }

    #[test]
    fn normalizes_whitespace_and_punctuation_spacing() {
        assert_eq!(
            clean_transcript("hello   world , nice"),
            "hello world, nice"
        );
        assert_eq!(clean_transcript("  a\n\n\n\nb  "), "a\n\nb");
    }

    /// Test helper: the rewritten text only, dropping the replacement count.
    fn dict(text: &str, entries: &[DictionaryEntry]) -> String {
        apply_dictionary(text, entries).0
    }

    #[test]
    fn dictionary_replaces_whole_words_case_insensitively() {
        let entries = vec![entry("open flow", "Velata"), entry("tory", "Tauri")];
        assert_eq!(
            dict("Open Flow uses tory under the hood", &entries),
            "Velata uses Tauri under the hood"
        );
        // "history" contains "tory" but must not match.
        assert_eq!(dict("history class", &entries), "history class");
    }

    #[test]
    fn dictionary_prefers_longest_match() {
        let entries = vec![entry("flow", "Flow?"), entry("open flow", "Velata")];
        assert_eq!(dict("try open flow now", &entries), "try Velata now");
    }

    #[test]
    fn dictionary_replacement_is_literal_not_regex() {
        let entries = vec![entry("cash", "$$$"), entry("c++", "C++")];
        assert_eq!(dict("send cash", &entries), "send $$$");
        assert_eq!(dict("i like c++ a lot", &entries), "i like C++ a lot");
    }

    #[test]
    fn dictionary_counts_replacement_occurrences() {
        let entries = vec![entry("tory", "Tauri"), entry("open flow", "Velata")];
        // Two occurrences of "tory" plus one "open flow" → three replacements,
        // counting occurrences applied rather than the two entries that matched.
        let (text, count) = apply_dictionary("tory and tory in open flow", &entries);
        assert_eq!(text, "Tauri and Tauri in Velata");
        assert_eq!(count, 3);
        // No matches → zero, text unchanged.
        assert_eq!(
            apply_dictionary("nothing here", &entries),
            ("nothing here".into(), 0)
        );
        // Empty dictionary → zero.
        assert_eq!(apply_dictionary("anything", &[]), ("anything".into(), 0));
    }

    #[test]
    fn clean_transcript_keeps_content_untouched_except_artifacts() {
        let raw = "so um the code is `let x = 1;` you know";
        // Artifact stripping only — fillers and prose are left verbatim for the
        // optional prompt transform (or plain insertion) to handle.
        assert_eq!(
            clean_transcript(raw),
            "so um the code is `let x = 1;` you know"
        );
    }

    fn snip(trigger: &str, expansion: &str, whole_utterance: bool) -> Snippet {
        Snippet {
            trigger: trigger.into(),
            expansion: expansion.into(),
            whole_utterance,
        }
    }

    #[test]
    fn snippets_expand_inline_whole_word_case_insensitively() {
        let snippets = vec![snip("my email", "me@example.com", false)];
        assert_eq!(
            apply_snippets("send it to My Email please", &snippets),
            "send it to me@example.com please"
        );
        // Substring inside a larger word must not expand.
        assert_eq!(
            apply_snippets("my emails are full", &snippets),
            "my emails are full"
        );
    }

    #[test]
    fn snippets_support_multiline_expansion() {
        let snippets = vec![snip("sign off", "Best,\nYijiazhen", false)];
        assert_eq!(apply_snippets("sign off", &snippets), "Best,\nYijiazhen");
    }

    #[test]
    fn whole_utterance_snippet_only_fires_on_exact_match() {
        let snippets = vec![snip("my email", "me@example.com", true)];
        // Exact (trimmed, case-insensitive) match → replaces everything.
        assert_eq!(apply_snippets("  My Email  ", &snippets), "me@example.com");
        // Cleanup that capitalizes and adds a period still counts as alone.
        assert_eq!(apply_snippets("My email.", &snippets), "me@example.com");
        // Embedded in a sentence → left untouched.
        assert_eq!(
            apply_snippets("send it to my email", &snippets),
            "send it to my email"
        );
    }

    #[test]
    fn snippets_prefer_longest_trigger() {
        let snippets = vec![
            snip("cal", "calendar", false),
            snip("my cal", "https://cal.example.com/me", false),
        ];
        assert_eq!(
            apply_snippets("here is my cal link", &snippets),
            "here is https://cal.example.com/me link"
        );
    }
}
