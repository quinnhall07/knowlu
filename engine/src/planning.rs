//! Port of `engine/planning.py` — the daily effort budget, session slice, approvals ceiling and
//! recurring commitments.

use std::path::Path;

use jiff::civil::{Date, Weekday};
use serde_yaml_ng::{Mapping, Value};

pub const DAY_KEYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// Python's `date.weekday()` is Monday-zero and indexes `DAY_KEYS` directly. Matching on the enum
/// rather than an offset method keeps this independent of jiff's numbering conventions.
pub fn day_key(date: Date) -> &'static str {
    match date.weekday() {
        Weekday::Monday => "mon",
        Weekday::Tuesday => "tue",
        Weekday::Wednesday => "wed",
        Weekday::Thursday => "thu",
        Weekday::Friday => "fri",
        Weekday::Saturday => "sat",
        Weekday::Sunday => "sun",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recurring {
    pub name: String,
    pub hours: f64,
    pub days: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanningConfig {
    pub daily_effort_budget: f64,
    pub slice_hours: f64,
    pub daily_approval_budget: i64,
    pub recurring: Vec<Recurring>,
}

impl Default for PlanningConfig {
    fn default() -> Self {
        PlanningConfig {
            daily_effort_budget: 4.0,
            slice_hours: 2.0,
            daily_approval_budget: 15,
            recurring: Vec::new(),
        }
    }
}

impl PlanningConfig {
    pub fn recurring_for(&self, day: Date) -> Vec<&Recurring> {
        let key = day_key(day);
        self.recurring
            .iter()
            .filter(|r| r.days.iter().any(|d| d == key))
            .collect()
    }
}

use crate::yaml::{get, opt_f64 as as_f64};

/// A missing file yields the defaults — never an error. Callers rely on this: a vault without a
/// `config/planning.yaml` still ranks.
pub fn load_planning(path: &Path) -> PlanningConfig {
    if !path.exists() {
        return PlanningConfig::default();
    }
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return PlanningConfig::default(),
    };
    let data: Mapping = match serde_yaml_ng::from_str::<Value>(&text) {
        Ok(Value::Mapping(m)) => m,
        _ => Mapping::new(),
    };

    let mut recurring = Vec::new();
    if let Some(Value::Sequence(items)) = get(&data, "recurring") {
        for item in items {
            let Value::Mapping(m) = item else { continue };
            let name = match get(m, "name") {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                _ => continue,
            };
            let hours = as_f64(get(m, "hours"), f64::NAN);
            if hours.is_nan() {
                continue;
            }
            // `item.get("days") or DAY_KEYS[:5]` — note the `or`: an EMPTY list is falsy in
            // Python and falls back to Monday-Friday, it does not mean "no days".
            let days: Vec<String> = match get(m, "days") {
                Some(Value::Sequence(ds)) if !ds.is_empty() => ds
                    .iter()
                    .filter_map(|d| match d {
                        Value::String(s) => Some(s.to_lowercase()),
                        Value::Number(n) => Some(n.to_string()),
                        _ => None,
                    })
                    .collect(),
                _ => DAY_KEYS[..5].iter().map(|s| s.to_string()).collect(),
            };
            recurring.push(Recurring { name, hours, days });
        }
    }

    PlanningConfig {
        daily_effort_budget: as_f64(get(&data, "daily_effort_budget"), 4.0),
        slice_hours: as_f64(get(&data, "slice_hours"), 2.0),
        daily_approval_budget: as_f64(get(&data, "daily_approval_budget"), 15.0).max(0.0) as i64,
        recurring,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let cfg = load_planning(Path::new("does/not/exist.yaml"));
        assert_eq!(cfg, PlanningConfig::default());
    }

    #[test]
    fn fixture_planning_parses() {
        let cfg = load_planning(Path::new("tests/fixtures/vault-s1/config/planning.yaml"));
        assert_eq!(cfg.daily_effort_budget, 4.0);
        assert_eq!(cfg.slice_hours, 2.0);
        assert_eq!(cfg.daily_approval_budget, 15);
        assert!(cfg.recurring.is_empty());
    }

    #[test]
    fn recurring_entries_parse_and_filter_by_day() {
        let dir = std::env::temp_dir().join(format!("qo-planning-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("planning.yaml");
        std::fs::write(
            &path,
            "daily_effort_budget: 3.5\nslice_hours: 1.0\nrecurring:\n  - name: German practice\n    hours: 0.75\n    days: [MON, wed]\n",
        )
        .unwrap();

        let cfg = load_planning(&path);
        assert_eq!(cfg.daily_effort_budget, 3.5);
        assert_eq!(cfg.recurring.len(), 1);
        assert_eq!(cfg.recurring[0].days, vec!["mon", "wed"]);

        // 2026-08-31 is a Monday, 2026-09-01 a Tuesday.
        assert_eq!(cfg.recurring_for(Date::constant(2026, 8, 31)).len(), 1);
        assert_eq!(cfg.recurring_for(Date::constant(2026, 9, 1)).len(), 0);
    }

    #[test]
    fn an_empty_days_list_falls_back_to_monday_friday() {
        // Python's `item.get("days") or DAY_KEYS[:5]`: [] is falsy, so it is NOT "no days".
        let dir = std::env::temp_dir().join(format!("qo-planning-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("planning.yaml");
        std::fs::write(&path, "recurring:\n  - name: x\n    hours: 1.0\n    days: []\n").unwrap();
        let cfg = load_planning(&path);
        assert_eq!(cfg.recurring[0].days, vec!["mon", "tue", "wed", "thu", "fri"]);
    }

    #[test]
    fn day_key_is_monday_zero() {
        assert_eq!(day_key(Date::constant(2026, 8, 31)), "mon");
        assert_eq!(day_key(Date::constant(2026, 8, 28)), "fri");
        assert_eq!(day_key(Date::constant(2026, 8, 30)), "sun");
    }
}
