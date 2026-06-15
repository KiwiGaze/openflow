//! Pure helpers for the lifetime Insights view.
//!
//! Insights are aggregates only — counts, sums, and dates — never transcripts
//! or audio. They are derived in `commands.rs` from the `insights_daily` table
//! (`db.rs`), which is written unconditionally on every dictation: a number
//! like "1,200 words" cannot reconstruct a single sentence, so it is safe to
//! keep without retaining any of what was said. This module holds only the
//! stateless math those derivations need.

use chrono::{Duration, Local, NaiveDate};

/// The read-only lifetime snapshot for the UI, derived from `insights_daily`.
/// Mirrored by `Insights` in `packages/core/src/types.ts`. Counts and dates
/// only — never words or audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Insights {
    pub words: u64,
    pub dictations: u64,
    /// Lifetime speaking pace (words ÷ minutes spoken); 0 with no duration.
    pub words_per_minute: u32,
    /// Current consecutive-day dictation streak, in days.
    pub streak: u32,
}

/// Speaking pace in words per minute, guarding against a zero duration.
pub fn pace_wpm(words: u64, record_ms: u64) -> u32 {
    if record_ms == 0 {
        return 0;
    }
    (words as f64 / (record_ms as f64 / 60_000.0)).round() as u32
}

/// The current consecutive-day dictation streak, in days, from sorted, distinct
/// `YYYY-MM-DD` day strings and today's local day string.
///
/// The run of consecutive days ends today, OR ends yesterday when today has no
/// dictation yet (a grace day so the streak does not read as broken mid-day
/// before the first dictation). No qualifying run ending today/yesterday → 0.
///
/// Unparseable day strings are skipped defensively; well-formed data never hits
/// that path. Input need not be pre-deduplicated.
pub fn streaks(days: &[String], today: &str) -> u32 {
    let mut parsed: Vec<NaiveDate> = days
        .iter()
        .filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .collect();
    parsed.sort_unstable();
    parsed.dedup();
    if parsed.is_empty() {
        return 0;
    }

    // Only counts if the most recent day is today or yesterday (the grace day).
    // Then walk backwards while days stay consecutive.
    let Some(today_date) = NaiveDate::parse_from_str(today, "%Y-%m-%d").ok() else {
        return 0;
    };
    let last = *parsed.last().expect("non-empty checked above");
    let gap = today_date - last;
    if gap != Duration::days(0) && gap != Duration::days(1) {
        return 0;
    }
    let mut count = 1u32;
    for pair in parsed.windows(2).rev() {
        if pair[1] - pair[0] == Duration::days(1) {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// The user's LOCAL calendar day as `YYYY-MM-DD` — the `insights_daily` bucket
/// key and the streak calculator's "today". Local (not UTC) so a dictation
/// counts toward the day on the user's clock.
pub fn local_day() -> String {
    Local::now().format("%Y-%m-%d").to_string()
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
    fn pace_guards_zero_duration() {
        assert_eq!(pace_wpm(100, 0), 0);
        assert_eq!(pace_wpm(120, 60_000), 120);
    }

    fn days(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn streaks_empty_is_zero() {
        assert_eq!(streaks(&[], "2026-06-13"), 0);
    }

    #[test]
    fn streaks_single_today() {
        assert_eq!(streaks(&days(&["2026-06-13"]), "2026-06-13"), 1);
    }

    #[test]
    fn streaks_cross_month_boundary() {
        // Jan 31 → Feb 1 is one calendar day apart; string math would miss it.
        assert_eq!(
            streaks(&days(&["2026-01-31", "2026-02-01"]), "2026-02-01"),
            2
        );
    }

    #[test]
    fn streaks_run_ending_today() {
        assert_eq!(
            streaks(
                &days(&["2026-06-11", "2026-06-12", "2026-06-13"]),
                "2026-06-13"
            ),
            3
        );
    }

    #[test]
    fn streaks_grace_when_today_missing_but_yesterday_present() {
        // Today has no dictation yet; a run ending yesterday still counts.
        assert_eq!(
            streaks(&days(&["2026-06-11", "2026-06-12"]), "2026-06-13"),
            2
        );
    }

    #[test]
    fn streaks_gap_resets_current() {
        // Most recent day is two days before today → no current streak.
        assert_eq!(
            streaks(
                &days(&["2026-06-09", "2026-06-10", "2026-06-11"]),
                "2026-06-13"
            ),
            0
        );
    }

    #[test]
    fn streaks_counts_only_the_run_reaching_today() {
        // A long early run, a gap, then a short run reaching today: only the
        // current run counts now that the longest streak is no longer surfaced.
        assert_eq!(
            streaks(
                &days(&[
                    "2026-06-01",
                    "2026-06-02",
                    "2026-06-03",
                    "2026-06-04",
                    "2026-06-12",
                    "2026-06-13",
                ]),
                "2026-06-13",
            ),
            2
        );
    }
}
