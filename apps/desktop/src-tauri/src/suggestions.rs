//! Session-only dictionary suggestions.
//!
//! Watches dictation for distinctive terms the user is likely to want preserved
//! — product/proper names whisper writes with internal capitals (TanStack,
//! DeepSeek, OAuth) — and offers them as one-click dictionary additions.
//!
//! Privacy: this keeps an in-RAM tally of *candidate tokens only* — never whole
//! transcripts — bounded in size and erased on quit. Nothing is written to disk
//! or transmitted; a suggestion is computed entirely from local state, the same
//! principle the rest of the app follows. Accepting one writes an ordinary
//! dictionary entry; declining leaves no trace.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use serde::Serialize;

use crate::settings::DictionaryEntry;

/// Cap on distinct candidates held at once, so a marathon session can't grow the
/// table without bound. Far above what any real session surfaces.
const MAX_CANDIDATES: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionarySuggestion {
    pub term: String,
    pub count: u32,
}

#[derive(Default)]
pub struct Suggestions {
    counts: Mutex<HashMap<String, u32>>,
    dismissed: Mutex<HashSet<String>>,
}

impl Suggestions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Tallies candidate terms found in a dictation transcript.
    pub fn observe(&self, text: &str) {
        let mut counts = self.counts.lock().expect("suggestions poisoned");
        for raw in text.split_whitespace() {
            let token = strip_edges(raw);
            if is_candidate(token) {
                if counts.len() >= MAX_CANDIDATES && !counts.contains_key(token) {
                    continue;
                }
                *counts.entry(token.to_string()).or_insert(0) += 1;
            }
        }
    }

    /// The most-seen candidates not already in the dictionary or dismissed,
    /// highest count first.
    pub fn top(&self, dictionary: &[DictionaryEntry], limit: usize) -> Vec<DictionarySuggestion> {
        let counts = self.counts.lock().expect("suggestions poisoned");
        let dismissed = self.dismissed.lock().expect("suggestions poisoned");
        let known: HashSet<String> = dictionary
            .iter()
            .flat_map(|e| [e.from.to_lowercase(), e.to.to_lowercase()])
            .collect();
        let mut items: Vec<DictionarySuggestion> = counts
            .iter()
            .filter(|(term, _)| {
                !dismissed.contains(term.as_str()) && !known.contains(&term.to_lowercase())
            })
            .map(|(term, &count)| DictionarySuggestion {
                term: term.clone(),
                count,
            })
            .collect();
        // Highest count first; ties broken alphabetically for a stable order.
        items.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.term.cmp(&b.term)));
        items.truncate(limit);
        items
    }

    /// Suppresses a term for the rest of the session.
    pub fn dismiss(&self, term: &str) {
        self.dismissed
            .lock()
            .expect("suggestions poisoned")
            .insert(term.to_string());
    }
}

/// Trims leading/trailing non-alphanumerics ("OAuth," → "OAuth"), keeping
/// internal punctuation intact.
fn strip_edges(token: &str) -> &str {
    token.trim_matches(|c: char| !c.is_alphanumeric())
}

/// A candidate is a multi-character token with an *internal* capital that is not
/// all-caps — i.e. CamelCase/mixed product names, not acronyms (API) or
/// ordinary Capitalized words (Monday, names).
fn is_candidate(token: &str) -> bool {
    if token.chars().take(2).count() < 2 {
        return false;
    }
    if !token.chars().any(|c| c.is_alphabetic()) {
        return false;
    }
    let has_internal_upper = token
        .chars()
        .enumerate()
        .any(|(i, c)| i > 0 && c.is_uppercase());
    let all_upper = token
        .chars()
        .filter(|c| c.is_alphabetic())
        .all(|c| c.is_uppercase());
    has_internal_upper && !all_upper
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
    fn flags_internal_caps_but_not_acronyms_or_plain_words() {
        assert!(is_candidate("TanStack"));
        assert!(is_candidate("DeepSeek"));
        assert!(is_candidate("OAuth"));
        assert!(is_candidate("iOS"));
        assert!(!is_candidate("API")); // all-caps acronym
        assert!(!is_candidate("Monday")); // capital only at the start
        assert!(!is_candidate("hello")); // no caps
        assert!(!is_candidate("a")); // too short
    }

    #[test]
    fn observe_tallies_and_top_orders_by_count() {
        let s = Suggestions::new();
        s.observe("we use TanStack and DeepSeek today.");
        s.observe("TanStack again, plus OAuth.");
        let top = s.top(&[], 10);
        assert_eq!(top[0].term, "TanStack");
        assert_eq!(top[0].count, 2);
        assert_eq!(top.len(), 3);
    }

    #[test]
    fn top_excludes_known_dictionary_terms_and_dismissed() {
        let s = Suggestions::new();
        s.observe("TanStack and DeepSeek and OAuth.");
        s.dismiss("OAuth");
        let dict = vec![entry("tan stack", "TanStack")];
        let top = s.top(&dict, 10);
        let terms: Vec<&str> = top.iter().map(|t| t.term.as_str()).collect();
        assert_eq!(terms, vec!["DeepSeek"]); // TanStack known, OAuth dismissed
    }
}
