//! Session-only usage counters for the local Insights view.
//!
//! Aggregates only — counts, sums, and per-mode tallies — never transcripts or
//! audio. Lives in RAM for the running session, is never written to disk and
//! never transmitted; quitting resets it. That is the whole privacy stance: a
//! number like "1,200 words" cannot reconstruct a single sentence, so it is
//! safe to show without keeping any of what was said.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

#[derive(Default)]
struct Counters {
    total_words: u64,
    total_record_ms: u64,
    dictations: u64,
    polished: u64,
    by_mode: HashMap<String, u64>,
}

pub struct Stats {
    inner: Mutex<Counters>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeCount {
    pub mode_id: String,
    pub count: u64,
}

/// A read-only snapshot for the UI. Mirrored by `Insights` in
/// `packages/core/src/types.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Insights {
    pub total_words: u64,
    pub dictations: u64,
    /// Average speaking pace this session; 0 until some speech is recorded.
    pub words_per_minute: u32,
    /// Percent of dictations that went through the LLM (vs rules cleanup).
    pub polished_percent: u32,
    /// Most-used modes, highest first (up to 3).
    pub top_modes: Vec<ModeCount>,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Counters::default()),
        }
    }

    /// Records one completed dictation. `record_ms` is the speech duration (the
    /// recorded audio length, not pipeline time), so the pace reflects how fast
    /// the user actually spoke.
    pub fn record_dictation(&self, words: u64, record_ms: u64, mode_id: &str, polished: bool) {
        let mut c = self.inner.lock().expect("stats poisoned");
        c.total_words += words;
        c.total_record_ms += record_ms;
        c.dictations += 1;
        if polished {
            c.polished += 1;
        }
        *c.by_mode.entry(mode_id.to_string()).or_insert(0) += 1;
    }

    pub fn snapshot(&self) -> Insights {
        let c = self.inner.lock().expect("stats poisoned");
        let words_per_minute = if c.total_record_ms > 0 {
            (c.total_words as f64 / (c.total_record_ms as f64 / 60_000.0)).round() as u32
        } else {
            0
        };
        let polished_percent = if c.dictations > 0 {
            (c.polished as f64 / c.dictations as f64 * 100.0).round() as u32
        } else {
            0
        };
        let mut top_modes: Vec<ModeCount> = c
            .by_mode
            .iter()
            .map(|(mode_id, &count)| ModeCount {
                mode_id: mode_id.clone(),
                count,
            })
            .collect();
        // Highest count first; ties broken by id for a stable order.
        top_modes.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.mode_id.cmp(&b.mode_id))
        });
        top_modes.truncate(3);
        Insights {
            total_words: c.total_words,
            dictations: c.dictations,
            words_per_minute,
            polished_percent,
            top_modes,
        }
    }
}

/// Counts whitespace-separated words. Good enough for a usage tally; it does not
/// try to be linguistically precise.
pub fn word_count(text: &str) -> u64 {
    text.split_whitespace().count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words_by_whitespace() {
        assert_eq!(word_count("  hello   world \n again "), 3);
        assert_eq!(word_count(""), 0);
    }

    #[test]
    fn empty_snapshot_is_all_zero() {
        let s = Stats::new().snapshot();
        assert_eq!(s.total_words, 0);
        assert_eq!(s.words_per_minute, 0);
        assert_eq!(s.polished_percent, 0);
        assert!(s.top_modes.is_empty());
    }

    #[test]
    fn aggregates_pace_polished_share_and_top_modes() {
        let stats = Stats::new();
        stats.record_dictation(100, 60_000, "standard", true);
        stats.record_dictation(20, 0, "email", false);
        stats.record_dictation(5, 0, "standard", false);
        let s = stats.snapshot();
        assert_eq!(s.total_words, 125);
        assert_eq!(s.dictations, 3);
        assert_eq!(s.words_per_minute, 125); // 125 words / 60 s of speech
        assert_eq!(s.polished_percent, 33); // 1 of 3
        assert_eq!(s.top_modes[0].mode_id, "standard");
        assert_eq!(s.top_modes[0].count, 2);
    }
}
