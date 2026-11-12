//! Counters that outlive the process (`XMAGE_GATEWAY_TOTALS`, a JSON file on the data volume): shadow comparisons,
//! declined calls and Rust-hosted games. Their gates are a week long (docs/RustMigration.md, phase 2;
//! docs/rust/GameHost.md, step E) and every deploy restarts the gateway, so `/metrics` alone can't show a week.
//!
//! At start the file's totals are loaded back into the counters, so `/metrics` keeps counting from them. Every 30
//! seconds, and when the gateway stops, the counters are written back, with what changed since the last write added
//! to the day's bucket (UTC). `GET /totals` gives the file with the last seven days summed.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// days kept in `days`
const KEEP_DAYS: usize = 90;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Counts {
    /// shadow comparisons by method: [same, different]
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub shadow: BTreeMap<String, (u64, u64)>,
    /// calls Rust declined, by method
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub declined: BTreeMap<String, u64>,
    #[serde(default)]
    pub requests_shadow: u64,
    #[serde(default)]
    pub host_games_started: u64,
    /// hosted games ended, by how (`host::end_kind`)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub host_games_ended: BTreeMap<String, u64>,
}

impl Counts {
    /// what `self` has more than `earlier` (counters only grow; a smaller value counts as 0)
    pub fn since(&self, earlier: &Counts) -> Counts {
        let mut shadow = BTreeMap::new();
        for (method, (same, different)) in &self.shadow {
            let (s0, d0) = earlier.shadow.get(method).copied().unwrap_or_default();
            let delta = (same.saturating_sub(s0), different.saturating_sub(d0));
            if delta != (0, 0) {
                shadow.insert(method.clone(), delta);
            }
        }
        Counts {
            shadow,
            declined: minus(&self.declined, &earlier.declined),
            requests_shadow: self.requests_shadow.saturating_sub(earlier.requests_shadow),
            host_games_started: self.host_games_started.saturating_sub(earlier.host_games_started),
            host_games_ended: minus(&self.host_games_ended, &earlier.host_games_ended),
        }
    }

    pub fn add(&mut self, more: &Counts) {
        for (method, (same, different)) in &more.shadow {
            let counts = self.shadow.entry(method.clone()).or_default();
            counts.0 += same;
            counts.1 += different;
        }
        for (method, n) in &more.declined {
            *self.declined.entry(method.clone()).or_default() += n;
        }
        self.requests_shadow += more.requests_shadow;
        self.host_games_started += more.host_games_started;
        for (how, n) in &more.host_games_ended {
            *self.host_games_ended.entry(how.clone()).or_default() += n;
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Counts::default()
    }
}

fn minus(now: &BTreeMap<String, u64>, earlier: &BTreeMap<String, u64>) -> BTreeMap<String, u64> {
    now.iter()
        .map(|(k, v)| (k.clone(), v.saturating_sub(earlier.get(k).copied().unwrap_or(0))))
        .filter(|(_, v)| *v > 0)
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Totals {
    /// when counting started (unix seconds)
    pub since: u64,
    /// last written (unix seconds)
    #[serde(default)]
    pub saved: u64,
    pub total: Counts,
    /// what each day (UTC, `YYYY-MM-DD`) added, the last 90 days
    #[serde(default)]
    pub days: BTreeMap<String, Counts>,
}

impl Totals {
    pub fn load(path: &Path) -> Totals {
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(totals) => totals,
                Err(e) => {
                    tracing::warn!("{}: can't read the totals ({e}); counting from zero", path.display());
                    Totals::fresh(now())
                }
            },
            Err(_) => Totals::fresh(now()),
        }
    }

    pub fn fresh(at: u64) -> Totals {
        Totals { since: at, saved: at, ..Totals::default() }
    }

    /// Takes the counters as they are now: what grew since `self.total` goes to the day of `at`
    pub fn update(&mut self, counts: Counts, at: u64) {
        let delta = counts.since(&self.total);
        if !delta.is_empty() {
            self.days.entry(day(at)).or_default().add(&delta);
        }
        while self.days.len() > KEEP_DAYS {
            let first = self.days.keys().next().cloned().expect("not empty");
            self.days.remove(&first);
        }
        self.total = counts;
        self.saved = at;
    }

    /// the last `n` days up to `at`'s, summed
    pub fn last_days(&self, n: u64, at: u64) -> Counts {
        let first = day(at.saturating_sub((n.saturating_sub(1)) * 86_400));
        let mut sum = Counts::default();
        for (_, counts) in self.days.range(first..) {
            sum.add(counts);
        }
        sum
    }

    /// Writes the file (a temporary file, then a rename, so a crash leaves the old one)
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let tmp = PathBuf::from(format!("{}.tmp", path.display()));
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)?;
        std::fs::rename(&tmp, path)
    }

    /// `GET /totals`: the file, plus `last7` (the last seven days, today's included)
    pub fn report(&self, at: u64) -> serde_json::Value {
        let mut value = serde_json::to_value(self).unwrap_or_default();
        value["last7"] = serde_json::to_value(self.last_days(7, at)).unwrap_or_default();
        value
    }
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// `YYYY-MM-DD` (UTC) of a unix time
pub fn day(secs: u64) -> String {
    // days since 1970-01-01 to a civil date (Howard Hinnant's algorithm)
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(same: u64, different: u64, started: u64, errors: u64) -> Counts {
        let mut c = Counts { requests_shadow: same + different, host_games_started: started, ..Counts::default() };
        c.shadow.insert("deckValidate".into(), (same, different));
        if errors > 0 {
            c.host_games_ended.insert("error".into(), errors);
        }
        c
    }

    #[test]
    fn days_are_utc_dates() {
        assert_eq!(day(0), "1970-01-01");
        assert_eq!(day(951_782_400), "2000-02-29");
        assert_eq!(day(1_794_441_599), "2026-11-11");
        assert_eq!(day(1_794_441_600), "2026-11-12");
    }

    #[test]
    fn growth_goes_to_its_day_and_survives_a_restart() {
        let dir = std::env::temp_dir().join(format!("playmat-totals-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("totals.json");
        let monday = 1_794_000_000;
        let mut totals = Totals::fresh(monday);
        totals.update(counts(3, 0, 2, 0), monday);
        totals.update(counts(5, 1, 2, 0), monday + 60);
        totals.save(&path).unwrap();

        // a restart: the counters come back from the file and keep growing the next day
        let mut again = Totals::load(&path);
        assert_eq!(again.total, counts(5, 1, 2, 0));
        again.update(counts(9, 1, 4, 1), monday + 86_400);
        assert_eq!(again.days.len(), 2);
        assert_eq!(again.days[&day(monday)], counts(5, 1, 2, 0));
        assert_eq!(again.days[&day(monday + 86_400)], counts(4, 0, 2, 1));
        assert_eq!(again.last_days(1, monday + 86_400), counts(4, 0, 2, 1));
        assert_eq!(again.last_days(7, monday + 86_400), counts(9, 1, 4, 1));
        assert_eq!(again.report(monday + 86_400)["last7"]["host_games_ended"]["error"], 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_counter_that_went_back_adds_nothing() {
        let mut totals = Totals::fresh(0);
        totals.update(counts(5, 0, 0, 0), 10);
        totals.update(counts(2, 0, 0, 0), 20);
        assert_eq!(totals.days[&day(0)].shadow["deckValidate"], (5, 0));
    }

    #[test]
    fn old_days_are_dropped() {
        let mut totals = Totals::fresh(0);
        for i in 0..100u64 {
            totals.update(counts(i + 1, 0, 0, 0), i * 86_400);
        }
        assert_eq!(totals.days.len(), KEEP_DAYS);
        assert_eq!(totals.days.keys().next().unwrap(), &day(10 * 86_400));
    }
}
