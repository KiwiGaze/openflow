//! Deterministic text processing: transcript cleanup, rules-based polishing,
//! and personal-dictionary replacements. Everything here is pure and fast —
//! the optional LLM pass lives in `llm.rs`.

use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};

use crate::settings::DictionaryEntry;

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

/// English vocal fillers. Word-bounded so words like "umbrella" are safe.
/// An optional leading comma is consumed so ", uh, the plan" collapses to
/// a single separator instead of leaving ", , the plan" behind.
static FILLERS: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?:[,;]\s*)?\b(?:um+|uh+|uhm|erm|hmm+|mhm|mmm+)\b[,.;]?")
        .case_insensitive(true)
        .build()
        .expect("static regex")
});

/// Spoken layout commands. Only fires when the phrase is punctuated like a
/// standalone command ("… done. New line. Next …"), so sentences such as
/// "a new line of products" are left alone.
static NEW_PARAGRAPH: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?:^|[,.!?;:])\s*new paragraph\s*(?:[,.!?;:]|$)")
        .case_insensitive(true)
        .build()
        .expect("static regex")
});

static NEW_LINE: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?:^|[,.!?;:])\s*new line\s*(?:[,.!?;:]|$)")
        .case_insensitive(true)
        .build()
        .expect("static regex")
});

static LEFTOVER_LEADING_PUNCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[ ,;.]+").expect("static regex"));

/// Strips transcriber artifacts and normalizes whitespace. Applied to every
/// transcript regardless of mode.
pub fn clean_transcript(raw: &str) -> String {
    let text = BRACKET_TAG.replace_all(raw, " ");
    let text = PAREN_TAG.replace_all(&text, " ");
    let text = MUSIC_NOTES.replace_all(&text, " ");
    normalize_whitespace(&text)
}

/// Rules-based polish for when no LLM is configured (or the mode opts out of
/// one): filler removal, spoken layout commands, sentence capitalization.
/// English-centric by design; the LLM path handles other languages better.
pub fn apply_rules_cleanup(text: &str) -> String {
    let text = NEW_PARAGRAPH.replace_all(text, ".\n\n");
    let text = NEW_LINE.replace_all(&text, ".\n");
    let text = FILLERS.replace_all(&text, " ");
    let text = normalize_whitespace(&text);
    let text = LEFTOVER_LEADING_PUNCT.replace(&text, "");
    capitalize_sentences(&text)
}

/// Applies personal-dictionary replacements with whole-word, case-insensitive
/// matching. Longer phrases win over shorter ones.
pub fn apply_dictionary(text: &str, entries: &[DictionaryEntry]) -> String {
    let mut sorted: Vec<&DictionaryEntry> = entries
        .iter()
        .filter(|e| !e.from.trim().is_empty())
        .collect();
    sorted.sort_by_key(|e| std::cmp::Reverse(e.from.len()));

    let mut result = text.to_string();
    for entry in sorted {
        let from = entry.from.trim();
        // `\b` only works against word characters; an entry like "c++" ends
        // on a symbol, so the boundary is applied per edge.
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
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
        let pattern = format!("{lead}{}{tail}", regex::escape(from));
        let Ok(re) = RegexBuilder::new(&pattern).case_insensitive(true).build() else {
            continue;
        };
        result = re
            .replace_all(&result, regex::NoExpand(entry.to.as_str()))
            .into_owned();
    }
    result
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

fn capitalize_sentences(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut capitalize_next = true;
    for ch in text.chars() {
        if capitalize_next && ch.is_alphabetic() {
            out.extend(ch.to_uppercase());
            capitalize_next = false;
            continue;
        }
        match ch {
            '.' | '!' | '?' | '\n' => capitalize_next = true,
            c if c.is_alphanumeric() => capitalize_next = false,
            _ => {}
        }
        out.push(ch);
    }
    out
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

    #[test]
    fn removes_fillers() {
        assert_eq!(
            apply_rules_cleanup("um, so this is, uh, the plan"),
            "So this is the plan"
        );
        assert_eq!(
            apply_rules_cleanup("the umbrella is here"),
            "The umbrella is here"
        );
        assert_eq!(apply_rules_cleanup("Hmm, let me think"), "Let me think");
    }

    #[test]
    fn spoken_commands_require_punctuation_boundaries() {
        assert_eq!(
            apply_rules_cleanup("first item. new line. second item"),
            "First item.\nSecond item"
        );
        assert_eq!(
            apply_rules_cleanup("intro done. new paragraph. the body starts"),
            "Intro done.\n\nThe body starts"
        );
        // No surrounding punctuation → treated as normal prose.
        assert_eq!(
            apply_rules_cleanup("we launched a new line of products"),
            "We launched a new line of products"
        );
    }

    #[test]
    fn capitalizes_sentence_starts() {
        assert_eq!(
            apply_rules_cleanup("hello there. how are you? i am fine"),
            "Hello there. How are you? I am fine"
        );
    }

    #[test]
    fn dictionary_replaces_whole_words_case_insensitively() {
        let entries = vec![entry("open flow", "OpenFlow"), entry("tory", "Tauri")];
        assert_eq!(
            apply_dictionary("Open Flow uses tory under the hood", &entries),
            "OpenFlow uses Tauri under the hood"
        );
        // "history" contains "tory" but must not match.
        assert_eq!(apply_dictionary("history class", &entries), "history class");
    }

    #[test]
    fn dictionary_prefers_longest_match() {
        let entries = vec![entry("flow", "Flow?"), entry("open flow", "OpenFlow")];
        assert_eq!(
            apply_dictionary("try open flow now", &entries),
            "try OpenFlow now"
        );
    }

    #[test]
    fn dictionary_replacement_is_literal_not_regex() {
        let entries = vec![entry("cash", "$$$"), entry("c++", "C++")];
        assert_eq!(apply_dictionary("send cash", &entries), "send $$$");
        assert_eq!(
            apply_dictionary("i like c++ a lot", &entries),
            "i like C++ a lot"
        );
    }

    #[test]
    fn full_literal_path_keeps_content_untouched_except_artifacts() {
        let raw = "so um the code is `let x = 1;` you know";
        // Literal mode: clean_transcript + dictionary only — fillers stay.
        assert_eq!(
            clean_transcript(raw),
            "so um the code is `let x = 1;` you know"
        );
    }
}
