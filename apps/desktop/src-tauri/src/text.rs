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

/// Single-pass, whole-word, case-insensitive replacement of many phrases at
/// once. Shared by the dictionary and snippet expanders. Two properties matter:
///
/// 1. **Longest wins.** Phrases compete longest-first, so "open flow" beats a
///    bare "flow" at the same span.
/// 2. **No cascading.** Matches are found in the original text only and each is
///    replaced verbatim; a replacement is never re-scanned, so one expansion
///    can never trigger another (e.g. a "cal" snippet can't fire inside the URL
///    that a "my cal" snippet just produced).
///
/// `pairs` is `(from, to)`; each `from` must be trimmed and non-empty.
fn replace_phrases(text: &str, pairs: &[(&str, &str)]) -> String {
    let mut ordered: Vec<&(&str, &str)> = pairs.iter().filter(|(f, _)| !f.is_empty()).collect();
    if ordered.is_empty() {
        return text.to_string();
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
        return text.to_string();
    };

    let lookup: HashMap<String, &str> = ordered
        .iter()
        .map(|(from, to)| (from.to_lowercase(), *to))
        .collect();
    re.replace_all(text, |caps: &regex::Captures| {
        let matched = caps.get(0).map_or("", |m| m.as_str());
        // Verbatim: return the replacement as an owned string so `$`-bearing
        // expansions (e.g. "$$$") are never treated as capture references.
        lookup
            .get(&matched.to_lowercase())
            .copied()
            .unwrap_or(matched)
            .to_string()
    })
    .into_owned()
}

/// Applies personal-dictionary replacements with whole-word, case-insensitive
/// matching. Longer phrases win over shorter ones.
pub fn apply_dictionary(text: &str, entries: &[DictionaryEntry]) -> String {
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
    replace_phrases(text, &pairs)
}

/// Casing styles for Code-mode identifier conversion.
#[derive(Clone, Copy)]
enum CaseStyle {
    Camel,
    Pascal,
    Snake,
    ScreamingSnake,
    Kebab,
}

/// Code mode: turns a spoken phrase into a single source-code identifier.
///
/// The whole utterance becomes one identifier — the utterance boundary is the
/// delimiter, which sidesteps the "where does the name end" ambiguity that a
/// mid-sentence command grammar would hit. An optional leading style keyword
/// overrides the default of camelCase:
///
/// - "get user by id"            → `getUserById`
/// - "snake case user id"        → `user_id`
/// - "constant max retries"      → `MAX_RETRIES`
/// - "pascal case user service"  → `UserService`
/// - "kebab case feature flag"   → `feature-flag`
pub fn apply_code_identifier(text: &str) -> String {
    let lower = text.trim().to_lowercase();
    let (style, rest) = parse_case_prefix(&lower);
    // Identifiers are alphanumeric: strip any spoken/transcribed punctuation
    // ("get user, by id" or "user's id" must not become "getUser,ById"). Words
    // that reduce to nothing are dropped.
    let words: Vec<String> = rest
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    join_identifier(style, &words)
}

/// Strips a leading style keyword ("snake case", "constant", …) if present,
/// returning the chosen style and the remaining words. Defaults to camelCase.
fn parse_case_prefix(lower: &str) -> (CaseStyle, &str) {
    // Longest phrases first so "screaming snake case" wins over "snake case".
    const PREFIXES: &[(&str, CaseStyle)] = &[
        ("screaming snake case", CaseStyle::ScreamingSnake),
        ("constant case", CaseStyle::ScreamingSnake),
        ("snake case", CaseStyle::Snake),
        ("pascal case", CaseStyle::Pascal),
        ("camel case", CaseStyle::Camel),
        ("kebab case", CaseStyle::Kebab),
        ("constant", CaseStyle::ScreamingSnake),
        ("pascal", CaseStyle::Pascal),
        ("camel", CaseStyle::Camel),
        ("kebab", CaseStyle::Kebab),
    ];
    for (keyword, style) in PREFIXES {
        if let Some(rest) = lower.strip_prefix(keyword) {
            // Require a word boundary so "constants" doesn't match "constant".
            if rest.is_empty() || rest.starts_with(' ') {
                return (*style, rest.trim_start());
            }
        }
    }
    (CaseStyle::Camel, lower)
}

fn join_identifier(style: CaseStyle, words: &[String]) -> String {
    if words.is_empty() {
        return String::new();
    }
    match style {
        CaseStyle::Snake => words.join("_"),
        CaseStyle::Kebab => words.join("-"),
        CaseStyle::ScreamingSnake => words
            .iter()
            .map(|w| w.to_uppercase())
            .collect::<Vec<_>>()
            .join("_"),
        CaseStyle::Pascal => words.iter().map(|w| capitalize(w)).collect::<String>(),
        CaseStyle::Camel => words
            .iter()
            .enumerate()
            .map(|(i, w)| if i == 0 { w.clone() } else { capitalize(w) })
            .collect::<String>(),
    }
}

/// Uppercases the first character of an already-lowercased word.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
        None => String::new(),
    }
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
        let entries = vec![entry("open flow", "Velata"), entry("tory", "Tauri")];
        assert_eq!(
            apply_dictionary("Open Flow uses tory under the hood", &entries),
            "Velata uses Tauri under the hood"
        );
        // "history" contains "tory" but must not match.
        assert_eq!(apply_dictionary("history class", &entries), "history class");
    }

    #[test]
    fn dictionary_prefers_longest_match() {
        let entries = vec![entry("flow", "Flow?"), entry("open flow", "Velata")];
        assert_eq!(
            apply_dictionary("try open flow now", &entries),
            "try Velata now"
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
    fn code_identifier_defaults_to_camel_case() {
        assert_eq!(apply_code_identifier("get user by id"), "getUserById");
        assert_eq!(apply_code_identifier("user service"), "userService");
        // Trailing punctuation from cleanup is ignored.
        assert_eq!(apply_code_identifier("Get User By Id."), "getUserById");
    }

    #[test]
    fn code_identifier_honors_leading_style_keyword() {
        assert_eq!(apply_code_identifier("snake case user id"), "user_id");
        assert_eq!(apply_code_identifier("constant max retries"), "MAX_RETRIES");
        assert_eq!(
            apply_code_identifier("pascal case user service"),
            "UserService"
        );
        assert_eq!(
            apply_code_identifier("kebab case feature flag"),
            "feature-flag"
        );
        assert_eq!(
            apply_code_identifier("screaming snake case max size"),
            "MAX_SIZE"
        );
    }

    #[test]
    fn code_identifier_keyword_needs_a_word_boundary() {
        // "constants" must not be read as the "constant" style keyword.
        assert_eq!(apply_code_identifier("constants table"), "constantsTable");
    }

    #[test]
    fn code_identifier_handles_empty_after_keyword() {
        assert_eq!(apply_code_identifier("snake case"), "");
        assert_eq!(apply_code_identifier("   "), "");
    }

    #[test]
    fn code_identifier_strips_transcribed_punctuation() {
        // Commas/apostrophes from the transcriber must not leak into the name.
        assert_eq!(apply_code_identifier("get user, by id"), "getUserById");
        assert_eq!(apply_code_identifier("user's id"), "usersId");
        assert_eq!(
            apply_code_identifier("snake case first. second"),
            "first_second"
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
