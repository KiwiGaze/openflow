//! Usage counters for the local Insights view.
//!
//! The session counters here are aggregates only — counts, sums, per-mode and
//! per-app tallies — never transcripts or audio. They live in RAM for the
//! running session, are never written to disk and never transmitted; quitting
//! resets them. That is the whole privacy stance: a number like "1,200 words"
//! cannot reconstruct a single sentence, so it is safe to show without keeping
//! any of what was said. (All-time totals and streaks are opt-in and persisted
//! by `db.rs`; they are assembled into the snapshot in `commands.rs`.)

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{Duration, NaiveDate};
use serde::Serialize;

#[derive(Default)]
struct Counters {
    total_words: u64,
    total_record_ms: u64,
    dictations: u64,
    polished: u64,
    /// Dictionary replacement occurrences applied this session.
    dict_fixes: u64,
    by_mode: HashMap<String, u64>,
    /// Frontmost-app display name → words dictated into it this session. Only
    /// real app names land here (a detection failure contributes no key).
    by_app: HashMap<String, u64>,
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

/// One app's word total for the "where it goes" breakdown. The name is an
/// application name, never any dictated content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppWords {
    pub name: String,
    pub words: u64,
}

/// Which source the per-app breakdown was computed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PerAppScope {
    /// All-time, from the persisted history table.
    AllTime,
    /// This session only, from the in-RAM tally.
    Session,
}

/// All-time usage summary derived from `insights_daily` (opt-in persistence).
/// Present only when `app_stats_enabled`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllTimeStats {
    pub words: u64,
    pub dictations: u64,
    /// Percent of all-time dictations that used the LLM.
    pub ai_percent: u32,
    pub fixes: u64,
    /// All-time speaking pace (words ÷ minutes spoken); 0 with no duration.
    pub words_per_minute: u32,
}

/// Consecutive-day dictation streaks, in days. Present only when
/// `app_stats_enabled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Streak {
    pub current: u32,
    pub longest: u32,
}

/// A read-only snapshot for the UI. Mirrored by `Insights` in
/// `packages/core/src/types.ts`. The session fields are filled by
/// `Stats::snapshot`; the persistence-backed fields (`per_app`/`per_app_scope`
/// default to the session tally, `all_time`/`streak` to `None`) are overwritten
/// in `commands.rs` when their opt-in flags are on.
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
    /// Dictionary replacements applied this session.
    pub dictionary_fixes: u64,
    /// Per-app word totals, highest first (up to 6).
    pub per_app: Vec<AppWords>,
    pub per_app_scope: PerAppScope,
    /// All-time totals, or `None` unless `app_stats_enabled`.
    pub all_time: Option<AllTimeStats>,
    /// Dictation streaks, or `None` unless `app_stats_enabled`.
    pub streak: Option<Streak>,
}

/// How many of the per-app breakdown rows to show.
pub const PER_APP_LIMIT: usize = 6;

impl Stats {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Counters::default()),
        }
    }

    /// Records one completed dictation. `record_ms` is the speech duration (the
    /// recorded audio length, not pipeline time), so the pace reflects how fast
    /// the user actually spoke. `app_name` is the frontmost app's display name
    /// (None on a detection failure); `fixes` is the dictionary replacements
    /// applied to this dictation.
    pub fn record_dictation(
        &self,
        words: u64,
        record_ms: u64,
        mode_id: &str,
        polished: bool,
        app_name: Option<&str>,
        fixes: u64,
    ) {
        let mut c = self.inner.lock().expect("stats poisoned");
        c.total_words += words;
        c.total_record_ms += record_ms;
        c.dictations += 1;
        c.dict_fixes += fixes;
        if polished {
            c.polished += 1;
        }
        *c.by_mode.entry(mode_id.to_string()).or_insert(0) += 1;
        if let Some(name) = app_name {
            *c.by_app.entry(name.to_string()).or_insert(0) += words;
        }
    }

    pub fn snapshot(&self) -> Insights {
        let c = self.inner.lock().expect("stats poisoned");
        let words_per_minute = pace_wpm(c.total_words, c.total_record_ms);
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

        let mut per_app: Vec<AppWords> = c
            .by_app
            .iter()
            .map(|(name, &words)| AppWords {
                name: name.clone(),
                words,
            })
            .collect();
        per_app.sort_by(|a, b| b.words.cmp(&a.words).then_with(|| a.name.cmp(&b.name)));
        per_app.truncate(PER_APP_LIMIT);

        Insights {
            total_words: c.total_words,
            dictations: c.dictations,
            words_per_minute,
            polished_percent,
            top_modes,
            dictionary_fixes: c.dict_fixes,
            per_app,
            // Defaults the assembler overrides when the history table is the
            // richer source (history_enabled).
            per_app_scope: PerAppScope::Session,
            all_time: None,
            streak: None,
        }
    }
}

/// Speaking pace in words per minute, guarding against a zero duration.
pub fn pace_wpm(words: u64, record_ms: u64) -> u32 {
    if record_ms == 0 {
        return 0;
    }
    (words as f64 / (record_ms as f64 / 60_000.0)).round() as u32
}

/// Current and longest consecutive-day dictation streaks from sorted, distinct
/// `YYYY-MM-DD` day strings and today's local day string.
///
/// - **Current** = the run of consecutive days ending today, OR ending
///   yesterday when today has no dictation yet (a grace day so the streak does
///   not read as broken mid-day before the first dictation). No qualifying run
///   ending today/yesterday → 0.
/// - **Longest** = the longest consecutive run anywhere in the history.
///
/// Unparseable day strings are skipped defensively; well-formed data never hits
/// that path. Input need not be pre-deduplicated.
pub fn streaks(days: &[String], today: &str) -> Streak {
    let mut parsed: Vec<NaiveDate> = days
        .iter()
        .filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .collect();
    parsed.sort_unstable();
    parsed.dedup();
    if parsed.is_empty() {
        return Streak {
            current: 0,
            longest: 0,
        };
    }

    // Longest run anywhere: walk ascending, extending while each day is exactly
    // one after the previous.
    let mut longest = 1u32;
    let mut run = 1u32;
    for pair in parsed.windows(2) {
        if pair[1] - pair[0] == Duration::days(1) {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 1;
        }
    }

    // Current run: only counts if the most recent day is today or yesterday
    // (the grace day). Then walk backwards while days stay consecutive.
    let current = match NaiveDate::parse_from_str(today, "%Y-%m-%d").ok() {
        Some(today_date) => {
            let last = *parsed.last().expect("non-empty checked above");
            let gap = today_date - last;
            if gap == Duration::days(0) || gap == Duration::days(1) {
                let mut count = 1u32;
                for pair in parsed.windows(2).rev() {
                    if pair[1] - pair[0] == Duration::days(1) {
                        count += 1;
                    } else {
                        break;
                    }
                }
                count
            } else {
                0
            }
        }
        None => 0,
    };

    Streak { current, longest }
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
        assert_eq!(s.dictionary_fixes, 0);
        assert!(s.per_app.is_empty());
        assert_eq!(s.per_app_scope, PerAppScope::Session);
        assert!(s.all_time.is_none());
        assert!(s.streak.is_none());
    }

    #[test]
    fn aggregates_pace_polished_share_and_top_modes() {
        let stats = Stats::new();
        stats.record_dictation(100, 60_000, "standard", true, Some("Mail"), 2);
        stats.record_dictation(20, 0, "email", false, Some("Mail"), 0);
        stats.record_dictation(5, 0, "standard", false, None, 1);
        let s = stats.snapshot();
        assert_eq!(s.total_words, 125);
        assert_eq!(s.dictations, 3);
        assert_eq!(s.words_per_minute, 125); // 125 words / 60 s of speech
        assert_eq!(s.polished_percent, 33); // 1 of 3
        assert_eq!(s.top_modes[0].mode_id, "standard");
        assert_eq!(s.top_modes[0].count, 2);
        // Dictionary fixes sum across dictations (2 + 0 + 1).
        assert_eq!(s.dictionary_fixes, 3);
        // Per-app words: Mail got 100 + 20; the app-less dictation contributes none.
        assert_eq!(
            s.per_app,
            vec![AppWords {
                name: "Mail".into(),
                words: 120
            }]
        );
    }

    fn days(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn streaks_empty_is_zero() {
        let s = streaks(&[], "2026-06-13");
        assert_eq!(s.current, 0);
        assert_eq!(s.longest, 0);
    }

    #[test]
    fn streaks_single_today() {
        let s = streaks(&days(&["2026-06-13"]), "2026-06-13");
        assert_eq!(s.current, 1);
        assert_eq!(s.longest, 1);
    }

    #[test]
    fn streaks_run_ending_today() {
        let s = streaks(
            &days(&["2026-06-11", "2026-06-12", "2026-06-13"]),
            "2026-06-13",
        );
        assert_eq!(s.current, 3);
        assert_eq!(s.longest, 3);
    }

    #[test]
    fn streaks_grace_when_today_missing_but_yesterday_present() {
        // Today has no dictation yet; a run ending yesterday still counts.
        let s = streaks(&days(&["2026-06-11", "2026-06-12"]), "2026-06-13");
        assert_eq!(s.current, 2);
        assert_eq!(s.longest, 2);
    }

    #[test]
    fn streaks_gap_resets_current() {
        // Most recent day is two days before today → no current streak, but the
        // earlier consecutive run is still the longest.
        let s = streaks(
            &days(&["2026-06-09", "2026-06-10", "2026-06-11"]),
            "2026-06-13",
        );
        assert_eq!(s.current, 0);
        assert_eq!(s.longest, 3);
    }

    #[test]
    fn streaks_longest_exceeds_current() {
        // A long early run, a gap, then a short run reaching today.
        let s = streaks(
            &days(&[
                "2026-06-01",
                "2026-06-02",
                "2026-06-03",
                "2026-06-04",
                "2026-06-12",
                "2026-06-13",
            ]),
            "2026-06-13",
        );
        assert_eq!(s.current, 2);
        assert_eq!(s.longest, 4);
    }
}
